#!/usr/bin/env python3
import argparse
import hashlib
import json
import os
import platform
import random
import re
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path


FEATURE_RE = re.compile(
    r"^feature=([^\s]+)\s+avg_ms=([0-9.]+)\s+calls_per_sec=([0-9.]+)\s+sink=([0-9]+)\s+iters=([0-9]+)$"
)
INIT_RE = re.compile(r"^init_ms=([0-9.]+)$")
ENGINE_RE = re.compile(r"^engine=([^\s]+)$")
RUST_ENGINE = "kiwi-rs"
PY_ENGINE = "kiwipiepy"


@dataclass
class FeatureSample:
    avg_ms: float
    calls_per_sec: float
    sink: int
    iters: int


@dataclass
class RunSample:
    engine: str
    init_ms: float
    features: dict[str, FeatureSample]
    feature_order: list[str] = field(default_factory=list)
    raw_output: str = ""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run kiwi-rs and kiwipiepy feature benchmarks repeatedly and print comparison tables."
    )
    parser.add_argument("--text", default="아버지가방에들어가신다.")
    parser.add_argument("--warmup", type=int, default=100)
    parser.add_argument("--iters", type=int, default=5000)
    parser.add_argument("--batch-size", type=int, default=256)
    parser.add_argument("--batch-iters", type=int, default=500)
    parser.add_argument(
        "--input-mode",
        choices=("repeated", "varied"),
        default="repeated",
        help="Use repeated identical input or varied rotating input pool.",
    )
    parser.add_argument(
        "--variant-pool",
        type=int,
        default=4096,
        help="Variant pool size for --input-mode varied.",
    )
    parser.add_argument(
        "--dataset-tsv",
        default="",
        help="Optional dataset TSV (`category<TAB>text`, comments with #).",
    )
    parser.add_argument(
        "--dataset-category",
        default="",
        help="Optional dataset category filter (requires --dataset-tsv).",
    )
    parser.add_argument(
        "--repeats",
        type=int,
        default=3,
        help="How many full benchmark rounds to execute.",
    )
    parser.add_argument(
        "--join-lm-search",
        type=str,
        default="true",
        help="Whether join uses LM search (true/false).",
    )
    parser.add_argument(
        "--engine-order",
        choices=("alternate", "rust-first", "python-first"),
        default="alternate",
        help="Execution order policy between engines.",
    )
    parser.add_argument(
        "--sleep-between-engines-ms",
        type=int,
        default=0,
        help="Sleep milliseconds between two engine runs in the same repeat.",
    )
    parser.add_argument(
        "--sleep-between-runs-ms",
        type=int,
        default=0,
        help="Sleep milliseconds between repeats.",
    )
    parser.add_argument(
        "--sink-warning-threshold",
        type=float,
        default=0.05,
        help="Allowed sink ratio deviation from 1.0 before warning (0.05 = 5%%).",
    )
    parser.add_argument(
        "--bootstrap-samples",
        type=int,
        default=2000,
        help="Bootstrap sample count for throughput ratio 95%% CI.",
    )
    parser.add_argument(
        "--equivalence-band",
        type=float,
        default=0.05,
        help="Practical-equivalence band around 1.0 ratio (0.05 = ±5%%).",
    )
    parser.add_argument(
        "--strict-sink-check",
        action="store_true",
        help="Fail with non-zero exit when sink parity warning appears.",
    )
    parser.add_argument(
        "--python-bin",
        default=".venv-bench/bin/python",
        help="Python executable used for scripts/bench_features_kiwipiepy.py",
    )
    parser.add_argument(
        "--python-model-path",
        default="",
        help="Optional model path passed to scripts/bench_features_kiwipiepy.py",
    )
    parser.add_argument(
        "--md-out",
        default="",
        help="Optional path to write markdown output.",
    )
    parser.add_argument(
        "--json-out",
        default="",
        help="Optional path to write raw benchmark json.",
    )
    args = parser.parse_args()
    raw_join_lm_search = str(args.join_lm_search).strip().lower()
    if raw_join_lm_search in {"1", "true", "yes", "on"}:
        args.join_lm_search = "true"
    elif raw_join_lm_search in {"0", "false", "no", "off"}:
        args.join_lm_search = "false"
    else:
        parser.error("--join-lm-search must be true/false")
    if args.warmup < 0:
        parser.error("--warmup must be >= 0")
    if args.iters <= 0:
        parser.error("--iters must be >= 1")
    if args.batch_size <= 0:
        parser.error("--batch-size must be >= 1")
    if args.batch_iters <= 0:
        parser.error("--batch-iters must be >= 1")
    if args.variant_pool <= 0:
        parser.error("--variant-pool must be >= 1")
    if args.dataset_category and not args.dataset_tsv:
        parser.error("--dataset-category requires --dataset-tsv")
    if args.repeats <= 0:
        parser.error("--repeats must be >= 1")
    if args.sleep_between_engines_ms < 0:
        parser.error("--sleep-between-engines-ms must be >= 0")
    if args.sleep_between_runs_ms < 0:
        parser.error("--sleep-between-runs-ms must be >= 0")
    if args.sink_warning_threshold < 0:
        parser.error("--sink-warning-threshold must be >= 0")
    if args.bootstrap_samples <= 0:
        parser.error("--bootstrap-samples must be >= 1")
    if args.equivalence_band < 0:
        parser.error("--equivalence-band must be >= 0")
    return args


def run_command(cmd: list[str], cwd: Path) -> str:
    completed = subprocess.run(
        cmd,
        cwd=cwd,
        capture_output=True,
        text=True,
        check=False,
    )
    output = completed.stdout + completed.stderr
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed ({completed.returncode}): {' '.join(cmd)}\n{output}"
        )
    return output


def run_command_optional(cmd: list[str], cwd: Path) -> str:
    completed = subprocess.run(
        cmd,
        cwd=cwd,
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        return ""
    return (completed.stdout + completed.stderr).strip()


def format_bytes_human(bytes_value: int | None) -> str:
    if bytes_value is None or bytes_value <= 0:
        return ""
    units = ["B", "KiB", "MiB", "GiB", "TiB"]
    value = float(bytes_value)
    unit_index = 0
    while value >= 1024.0 and unit_index < len(units) - 1:
        value /= 1024.0
        unit_index += 1
    return f"{value:.2f} {units[unit_index]} ({bytes_value} bytes)"


def collect_environment(cwd: Path, python_bin: str) -> dict[str, object]:
    system = platform.system()
    cpu_model = platform.processor() or ""
    physical_cores = ""
    logical_cores = str(os.cpu_count() or "")
    memory_bytes: int | None = None

    if system == "Darwin":
        cpu_model = run_command_optional(["sysctl", "-n", "machdep.cpu.brand_string"], cwd)
        physical_cores = run_command_optional(["sysctl", "-n", "hw.physicalcpu"], cwd)
        logical = run_command_optional(["sysctl", "-n", "hw.logicalcpu"], cwd)
        if logical:
            logical_cores = logical
        mem_raw = run_command_optional(["sysctl", "-n", "hw.memsize"], cwd)
        if mem_raw.isdigit():
            memory_bytes = int(mem_raw)
    elif system == "Linux":
        cpuinfo = Path("/proc/cpuinfo")
        if cpuinfo.exists():
            lines = cpuinfo.read_text(encoding="utf-8", errors="replace").splitlines()
            for line in lines:
                if line.startswith("model name"):
                    cpu_model = line.split(":", 1)[1].strip()
                    break

            core_pairs: set[tuple[str, str]] = set()
            physical_id = ""
            core_id = ""
            for line in lines + [""]:
                if not line.strip():
                    if physical_id or core_id:
                        core_pairs.add((physical_id, core_id))
                    physical_id = ""
                    core_id = ""
                    continue
                if line.startswith("physical id"):
                    physical_id = line.split(":", 1)[1].strip()
                elif line.startswith("core id"):
                    core_id = line.split(":", 1)[1].strip()
            if core_pairs:
                physical_cores = str(len(core_pairs))

        meminfo = Path("/proc/meminfo")
        if meminfo.exists():
            for line in meminfo.read_text(encoding="utf-8", errors="replace").splitlines():
                if line.startswith("MemTotal:"):
                    parts = line.split()
                    if len(parts) >= 2 and parts[1].isdigit():
                        memory_bytes = int(parts[1]) * 1024
                    break
    elif system == "Windows":
        try:
            import ctypes

            kernel32 = ctypes.windll.kernel32
            logical = kernel32.GetActiveProcessorCount(0xFFFF)
            if logical > 0:
                logical_cores = str(logical)
        except Exception:
            pass

    if not cpu_model:
        machine = platform.machine()
        if machine:
            cpu_model = f"{machine} (CPU brand unavailable in sandbox)"

    if memory_bytes is None:
        try:
            page_size = int(os.sysconf("SC_PAGE_SIZE"))
            page_count = int(os.sysconf("SC_PHYS_PAGES"))
            if page_size > 0 and page_count > 0:
                memory_bytes = page_size * page_count
        except Exception:
            pass

    rustc_version = run_command_optional(["rustc", "--version"], cwd)
    cargo_version = run_command_optional(["cargo", "--version"], cwd)
    python_bin_version = run_command_optional([python_bin, "--version"], cwd)
    kiwipiepy_version = run_command_optional(
        [
            python_bin,
            "-c",
            "import kiwipiepy,sys;sys.stdout.write(getattr(kiwipiepy,'__version__','unknown'))",
        ],
        cwd,
    )
    git_head = run_command_optional(["git", "rev-parse", "HEAD"], cwd)
    git_branch = run_command_optional(["git", "rev-parse", "--abbrev-ref", "HEAD"], cwd)
    git_dirty = bool(run_command_optional(["git", "status", "--porcelain"], cwd))

    now = datetime.now().astimezone()
    return {
        "timestamp_local": now.isoformat(timespec="seconds"),
        "os": f"{platform.system()} {platform.release()}",
        "platform": platform.platform(),
        "cpu_model": cpu_model,
        "physical_cores": physical_cores,
        "logical_cores": logical_cores,
        "memory": format_bytes_human(memory_bytes),
        "rustc": rustc_version,
        "cargo": cargo_version,
        "python_harness": sys.version.splitlines()[0],
        "python_bench_bin": python_bin,
        "python_bench_bin_version": python_bin_version,
        "kiwipiepy": kiwipiepy_version,
        "kiwi_library_path": os.environ.get("KIWI_LIBRARY_PATH", ""),
        "kiwi_model_path": os.environ.get("KIWI_MODEL_PATH", ""),
        "git_head": git_head,
        "git_branch": git_branch,
        "git_dirty": git_dirty,
    }


def load_dataset_rows(path: str) -> list[tuple[str, str]]:
    dataset_path = Path(path)
    if not dataset_path.exists():
        raise RuntimeError(f"dataset file not found: {path}")
    rows: list[tuple[str, str]] = []
    for line_no, raw in enumerate(
        dataset_path.read_text(encoding="utf-8", errors="replace").splitlines(),
        start=1,
    ):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if "\t" in line:
            category, text = line.split("\t", 1)
            category = category.strip() or "default"
            text = text.strip()
        else:
            category = "default"
            text = line
        if not text:
            raise RuntimeError(f"dataset line {line_no} has empty text")
        rows.append((category, text))
    if not rows:
        raise RuntimeError("dataset file has no usable rows")
    return rows


def filter_dataset_rows(
    rows: list[tuple[str, str]],
    category_filter: str,
) -> list[tuple[str, str]]:
    if not category_filter:
        return list(rows)
    wanted = category_filter.strip().lower()
    filtered = [(category, text) for category, text in rows if category.lower() == wanted]
    if not filtered:
        raise RuntimeError(
            f"dataset category '{category_filter}' produced zero rows"
        )
    return filtered


def dataset_sha256(path: str) -> str:
    digest = hashlib.sha256()
    with Path(path).open("rb") as fp:
        while True:
            chunk = fp.read(1024 * 1024)
            if not chunk:
                break
            digest.update(chunk)
    return digest.hexdigest()


def dataset_audit(rows: list[tuple[str, str]]) -> dict[str, object]:
    category_counts: dict[str, int] = {}
    lengths: list[int] = []
    unique_texts: set[str] = set()
    for category, text in rows:
        category_counts[category] = category_counts.get(category, 0) + 1
        lengths.append(len(text))
        unique_texts.add(text)
    sorted_categories = sorted(category_counts.items(), key=lambda item: item[0])
    category_text = ", ".join(f"{name}:{count}" for name, count in sorted_categories)
    return {
        "rows": len(rows),
        "unique_texts": len(unique_texts),
        "categories": len(category_counts),
        "category_counts": category_text,
        "char_len_min": min(lengths),
        "char_len_median": int(median([float(value) for value in lengths])),
        "char_len_max": max(lengths),
    }


def md_value(value: object) -> str:
    if value is None:
        return "-"
    text = str(value).strip()
    if not text:
        return "-"
    return text.replace("|", r"\|").replace("\n", " ")


def parse_run_output(output: str) -> RunSample:
    engine = ""
    init_ms = None
    features: dict[str, FeatureSample] = {}
    order: list[str] = []

    for raw_line in output.splitlines():
        line = raw_line.strip()
        if not line:
            continue

        engine_match = ENGINE_RE.match(line)
        if engine_match:
            engine = engine_match.group(1)
            continue

        init_match = INIT_RE.match(line)
        if init_match:
            init_ms = float(init_match.group(1))
            continue

        feature_match = FEATURE_RE.match(line)
        if feature_match:
            feature = feature_match.group(1)
            features[feature] = FeatureSample(
                avg_ms=float(feature_match.group(2)),
                calls_per_sec=float(feature_match.group(3)),
                sink=int(feature_match.group(4)),
                iters=int(feature_match.group(5)),
            )
            order.append(feature)

    if not engine:
        raise RuntimeError(f"failed to parse engine from output\n{output}")
    if init_ms is None:
        raise RuntimeError(f"failed to parse init_ms from output\n{output}")
    if not features:
        raise RuntimeError(f"failed to parse feature rows from output\n{output}")

    return RunSample(
        engine=engine,
        init_ms=init_ms,
        features=features,
        feature_order=order,
        raw_output=output,
    )


def median(values: list[float]) -> float:
    return float(statistics.median(values))


def percentile(values: list[float], q: float) -> float:
    if not values:
        return 0.0
    sorted_values = sorted(values)
    if len(sorted_values) == 1:
        return float(sorted_values[0])
    pos = (len(sorted_values) - 1) * q
    lower = int(pos)
    upper = min(lower + 1, len(sorted_values) - 1)
    weight = pos - lower
    return float(sorted_values[lower] * (1.0 - weight) + sorted_values[upper] * weight)


def cv_percent(values: list[float]) -> float:
    if len(values) <= 1:
        return 0.0
    mean_value = statistics.mean(values)
    if mean_value == 0:
        return 0.0
    return float(statistics.stdev(values) / mean_value * 100.0)


def bootstrap_ratio_ci(
    rust_values: list[float],
    py_values: list[float],
    samples: int,
) -> tuple[float, float, float, float]:
    if not rust_values or not py_values:
        return 0.0, 0.0, 0.0, 0.0

    rng = random.Random(42)
    n_rust = len(rust_values)
    n_py = len(py_values)
    ratios: list[float] = []
    for _ in range(samples):
        rust_sample = [rust_values[rng.randrange(n_rust)] for _ in range(n_rust)]
        py_sample = [py_values[rng.randrange(n_py)] for _ in range(n_py)]
        rust_med = median(rust_sample)
        py_med = median(py_sample)
        if py_med <= 0.0:
            continue
        ratios.append(rust_med / py_med)

    if not ratios:
        return 0.0, 0.0, 0.0, 0.0

    low = percentile(ratios, 0.025)
    med = percentile(ratios, 0.5)
    high = percentile(ratios, 0.975)
    prob_gt_one = sum(1 for value in ratios if value > 1.0) / len(ratios)
    return low, med, high, prob_gt_one


def ratio_decision(ci_low: float, ci_high: float, equivalence_band: float) -> str:
    lower_eq = 1.0 - equivalence_band
    upper_eq = 1.0 + equivalence_band
    if ci_low > upper_eq:
        return "kiwi-rs faster (robust)"
    if ci_high < lower_eq:
        return "kiwipiepy faster (robust)"
    if ci_low >= lower_eq and ci_high <= upper_eq:
        return "practically equivalent"
    if ci_low > 1.0:
        return "kiwi-rs likely faster"
    if ci_high < 1.0:
        return "kiwipiepy likely faster"
    return "inconclusive"


def fmt_med_range(values: list[float], digits: int = 2) -> str:
    return f"{median(values):.{digits}f} [{min(values):.{digits}f}-{max(values):.{digits}f}]"


def fmt_cv(values: list[float]) -> str:
    return f"{cv_percent(values):.2f}%"


def feature_sink_ratio(
    rust_values: list[int],
    py_values: list[int],
) -> float:
    rust_med = median([float(value) for value in rust_values])
    py_med = median([float(value) for value in py_values])
    if py_med == 0.0:
        return 1.0 if rust_med == 0.0 else float("inf")
    return rust_med / py_med


def aggregate(runs: list[RunSample]) -> dict[str, object]:
    by_engine: dict[str, dict[str, object]] = {}
    for run in runs:
        engine_bucket = by_engine.setdefault(
            run.engine,
            {
                "init_ms": [],
                "features": {},
                "feature_order": run.feature_order,
            },
        )
        engine_bucket["init_ms"].append(run.init_ms)
        for feature, sample in run.features.items():
            feature_bucket = engine_bucket["features"].setdefault(
                feature,
                {
                    "avg_ms": [],
                    "calls_per_sec": [],
                    "sink": [],
                    "iters": [],
                },
            )
            feature_bucket["avg_ms"].append(sample.avg_ms)
            feature_bucket["calls_per_sec"].append(sample.calls_per_sec)
            feature_bucket["sink"].append(sample.sink)
            feature_bucket["iters"].append(sample.iters)
    return by_engine


def build_markdown(
    agg: dict[str, object],
    repeats: int,
    environment: dict[str, object],
    benchmark_config: dict[str, object],
    sink_warning_threshold: float,
    bootstrap_samples: int,
    equivalence_band: float,
    rust_engine: str = RUST_ENGINE,
    py_engine: str = PY_ENGINE,
) -> tuple[str, list[str]]:
    if rust_engine not in agg or py_engine not in agg:
        raise RuntimeError(
            f"expected engines '{rust_engine}' and '{py_engine}', got: {', '.join(agg.keys())}"
        )

    rust = agg[rust_engine]
    py = agg[py_engine]
    rust_features = rust["features"]
    py_features = py["features"]

    rust_order = rust.get("feature_order", [])
    common = [feature for feature in rust_order if feature in py_features]
    rust_only = [feature for feature in rust_order if feature not in py_features]
    py_only = [feature for feature in py_features.keys() if feature not in rust_features]
    sink_warnings: list[str] = []
    decision_rows: list[tuple[str, float, float, float, float, str]] = []

    lines: list[str] = []
    lines.append(
        f"### Expanded Feature Benchmark Snapshot (median of {repeats} runs, min-max + p95/CV shown)"
    )
    lines.append("")
    lines.append("Benchmark environment:")
    lines.append("")
    lines.append("| Item | Value |")
    lines.append("|---|---|")
    lines.append(f"| Timestamp (local) | {md_value(environment.get('timestamp_local'))} |")
    lines.append(f"| OS | {md_value(environment.get('os'))} |")
    lines.append(f"| Platform | {md_value(environment.get('platform'))} |")
    lines.append(f"| CPU | {md_value(environment.get('cpu_model'))} |")
    lines.append(
        f"| Cores (physical/logical) | {md_value(environment.get('physical_cores'))}/{md_value(environment.get('logical_cores'))} |"
    )
    lines.append(f"| Memory | {md_value(environment.get('memory'))} |")
    lines.append(f"| rustc | {md_value(environment.get('rustc'))} |")
    lines.append(f"| cargo | {md_value(environment.get('cargo'))} |")
    lines.append(f"| Python (harness) | {md_value(environment.get('python_harness'))} |")
    lines.append(
        f"| Python (bench bin) | {md_value(environment.get('python_bench_bin_version'))} (`{md_value(environment.get('python_bench_bin'))}`) |"
    )
    lines.append(f"| kiwipiepy | {md_value(environment.get('kiwipiepy'))} |")
    lines.append(f"| KIWI_LIBRARY_PATH | {md_value(environment.get('kiwi_library_path'))} |")
    lines.append(f"| KIWI_MODEL_PATH | {md_value(environment.get('kiwi_model_path'))} |")
    lines.append(
        f"| Git | `{md_value(environment.get('git_head'))}` ({md_value(environment.get('git_branch'))}, dirty={md_value(environment.get('git_dirty'))}) |"
    )
    lines.append("")
    lines.append("Benchmark config:")
    lines.append("")
    lines.append("| Item | Value |")
    lines.append("|---|---|")
    for key, value in benchmark_config.items():
        lines.append(f"| {md_value(key)} | {md_value(value)} |")
    if benchmark_config.get("dataset_tsv"):
        lines.append("")
        lines.append("Dataset profile:")
        lines.append("")
        lines.append("| Item | Value |")
        lines.append("|---|---|")
        lines.append(f"| path | {md_value(benchmark_config.get('dataset_tsv'))} |")
        lines.append(
            f"| category filter | {md_value(benchmark_config.get('dataset_category'))} |"
        )
        lines.append(f"| sha256 | `{md_value(benchmark_config.get('dataset_sha256'))}` |")
        lines.append(f"| rows | {md_value(benchmark_config.get('dataset_rows'))} |")
        lines.append(
            f"| unique texts | {md_value(benchmark_config.get('dataset_unique_texts'))} |"
        )
        lines.append(
            f"| categories | {md_value(benchmark_config.get('dataset_categories'))} |"
        )
        lines.append(
            f"| category counts | {md_value(benchmark_config.get('dataset_category_counts'))} |"
        )
        lines.append(
            f"| text length (char) | min={md_value(benchmark_config.get('dataset_char_len_min'))}, median={md_value(benchmark_config.get('dataset_char_len_median'))}, max={md_value(benchmark_config.get('dataset_char_len_max'))} |"
        )
    lines.append("")
    lines.append("Throughput comparison (`calls_per_sec`, higher is better):")
    lines.append("")
    lines.append(
        "| Feature | `kiwi-rs` | `kiwipiepy` | Relative (`kiwi-rs / kiwipiepy`) | 95% CI (bootstrap) | P(`ratio > 1`) | `kiwi-rs` CV | `kiwipiepy` CV |"
    )
    lines.append("|---|---:|---:|---:|---:|---:|---:|---:|")

    for feature in common:
        rust_calls = rust_features[feature]["calls_per_sec"]
        py_calls = py_features[feature]["calls_per_sec"]
        rust_med = median(rust_calls)
        py_med = median(py_calls)
        ratio = rust_med / py_med if py_med > 0 else 0.0
        ci_low, _, ci_high, prob_gt_one = bootstrap_ratio_ci(
            rust_calls,
            py_calls,
            bootstrap_samples,
        )
        decision = ratio_decision(ci_low, ci_high, equivalence_band)
        decision_rows.append((feature, ratio, ci_low, ci_high, prob_gt_one, decision))
        lines.append(
            (
                f"| `{feature}` | {fmt_med_range(rust_calls)} | {fmt_med_range(py_calls)} "
                f"| {ratio:.2f}x | [{ci_low:.2f}, {ci_high:.2f}]x | {prob_gt_one:.3f} "
                f"| {fmt_cv(rust_calls)} | {fmt_cv(py_calls)} |"
            )
        )

    lines.append("")
    lines.append("Stability snapshot (`calls_per_sec` p95):")
    lines.append("")
    lines.append("| Feature | `kiwi-rs` p95 | `kiwipiepy` p95 |")
    lines.append("|---|---:|---:|")
    for feature in common:
        rust_calls = rust_features[feature]["calls_per_sec"]
        py_calls = py_features[feature]["calls_per_sec"]
        lines.append(
            f"| `{feature}` | {percentile(rust_calls, 0.95):.2f} | {percentile(py_calls, 0.95):.2f} |"
        )

    lines.append("")
    lines.append("Workload parity check (`sink`, should be near 1.0x):")
    lines.append("")
    lines.append(
        f"Warning threshold: ±{sink_warning_threshold * 100.0:.1f}% around 1.0x."
    )
    lines.append("")
    lines.append(
        "| Feature | `kiwi-rs` sink | `kiwipiepy` sink | Sink ratio (`kiwi-rs / kiwipiepy`) | Status |"
    )
    lines.append("|---|---:|---:|---:|---|")
    for feature in common:
        rust_sink = rust_features[feature]["sink"]
        py_sink = py_features[feature]["sink"]
        ratio = feature_sink_ratio(rust_sink, py_sink)
        status = "ok"
        if ratio == float("inf") or abs(ratio - 1.0) > sink_warning_threshold:
            status = "review"
            sink_warnings.append(feature)
        lines.append(
            f"| `{feature}` | {fmt_med_range([float(v) for v in rust_sink], 2)} | {fmt_med_range([float(v) for v in py_sink], 2)} | {ratio:.4f}x | {status} |"
        )
    if sink_warnings:
        warning_list = ", ".join(f"`{feature}`" for feature in sink_warnings)
        lines.append("")
        lines.append(f"Sink warning features: {warning_list}")

    lines.append("")
    lines.append("SWE-style decision table (throughput ratio hypothesis):")
    lines.append("")
    lines.append(
        f"Equivalence band: ±{equivalence_band * 100.0:.1f}% around 1.0x. Bootstrap samples: {bootstrap_samples}."
    )
    lines.append("")
    lines.append("| Feature | Ratio | 95% CI | P(`ratio > 1`) | Decision |")
    lines.append("|---|---:|---:|---:|---|")
    for feature, ratio, ci_low, ci_high, prob_gt_one, decision in decision_rows:
        lines.append(
            f"| `{feature}` | {ratio:.2f}x | [{ci_low:.2f}, {ci_high:.2f}]x | {prob_gt_one:.3f} | {decision} |"
        )

    lines.append("")
    lines.append("SWE defensibility scorecard:")
    lines.append("")
    lines.append("| Check | Status | Note |")
    lines.append("|---|---|---|")
    status_repeats = "pass" if repeats >= 5 else "warn"
    lines.append(
        f"| Run count (`repeats >= 5`) | {status_repeats} | current={repeats} |"
    )
    status_order = (
        "pass" if benchmark_config.get("engine_order") == "alternate" else "warn"
    )
    lines.append(
        f"| Order bias control (`engine_order=alternate`) | {status_order} | current={md_value(benchmark_config.get('engine_order'))} |"
    )
    status_sink = "pass" if not sink_warnings else "warn"
    lines.append(
        f"| Workload parity (`sink`) | {status_sink} | warnings={len(sink_warnings)} |"
    )
    status_bootstrap = "pass" if bootstrap_samples >= 1000 else "warn"
    lines.append(
        f"| CI robustness (`bootstrap_samples >= 1000`) | {status_bootstrap} | current={bootstrap_samples} |"
    )
    status_dirty = (
        "pass" if not bool(environment.get("git_dirty")) else "warn"
    )
    lines.append(
        f"| Clean git tree | {status_dirty} | dirty={md_value(environment.get('git_dirty'))} |"
    )

    lines.append("")
    lines.append("Startup (`init_ms`, lower is better):")
    lines.append("")
    lines.append("| Init path | `kiwi-rs` | `kiwipiepy` | `kiwi-rs` CV | `kiwipiepy` CV |")
    lines.append("|---|---:|---:|---:|---:|")
    lines.append(
        f"| `Kiwi::init()` / `Kiwi()` | {fmt_med_range(rust['init_ms'], 3)} ms | {fmt_med_range(py['init_ms'], 3)} ms | {fmt_cv(rust['init_ms'])} | {fmt_cv(py['init_ms'])} |"
    )

    if rust_only:
        lines.append("")
        lines.append("Rust-only benchmark features:")
        lines.append("")
        lines.append("| Feature | `kiwi-rs` | `kiwi-rs` CV |")
        lines.append("|---|---:|---:|")
        for feature in rust_only:
            lines.append(
                f"| `{feature}` | {fmt_med_range(rust_features[feature]['calls_per_sec'])} | {fmt_cv(rust_features[feature]['calls_per_sec'])} |"
            )

    if py_only:
        lines.append("")
        lines.append("Python-only benchmark features:")
        lines.append("")
        lines.append("| Feature | `kiwipiepy` | `kiwipiepy` CV |")
        lines.append("|---|---:|---:|")
        for feature in py_only:
            lines.append(
                f"| `{feature}` | {fmt_med_range(py_features[feature]['calls_per_sec'])} | {fmt_cv(py_features[feature]['calls_per_sec'])} |"
            )

    return "\n".join(lines), sink_warnings


def command_to_text(cmd: list[str]) -> str:
    return " ".join(cmd)


def run_order_for_repeat(engine_order: str, repeat_index: int) -> list[str]:
    if engine_order == "rust-first":
        return [RUST_ENGINE, PY_ENGINE]
    if engine_order == "python-first":
        return [PY_ENGINE, RUST_ENGINE]
    if repeat_index % 2 == 0:
        return [RUST_ENGINE, PY_ENGINE]
    return [PY_ENGINE, RUST_ENGINE]


def main() -> int:
    args = parse_args()
    cwd = Path(__file__).resolve().parents[1]
    environment = collect_environment(cwd, args.python_bin)
    dataset_rows_selected: list[tuple[str, str]] = []
    dataset_audit_info: dict[str, object] = {}
    dataset_hash = ""
    if args.dataset_tsv:
        dataset_rows_all = load_dataset_rows(args.dataset_tsv)
        dataset_rows_selected = filter_dataset_rows(dataset_rows_all, args.dataset_category)
        dataset_audit_info = dataset_audit(dataset_rows_selected)
        dataset_hash = dataset_sha256(args.dataset_tsv)

    rust_cmd = [
        "cargo",
        "run",
        "--release",
        "--example",
        "bench_features",
        "--",
        "--text",
        args.text,
        "--warmup",
        str(args.warmup),
        "--iters",
        str(args.iters),
        "--batch-size",
        str(args.batch_size),
        "--batch-iters",
        str(args.batch_iters),
        "--input-mode",
        args.input_mode,
        "--variant-pool",
        str(args.variant_pool),
        "--join-lm-search",
        args.join_lm_search,
    ]
    if args.dataset_tsv:
        rust_cmd.extend(["--dataset-tsv", args.dataset_tsv])
        if args.dataset_category:
            rust_cmd.extend(["--dataset-category", args.dataset_category])
    py_cmd = [
        args.python_bin,
        "scripts/bench_features_kiwipiepy.py",
        "--text",
        args.text,
        "--warmup",
        str(args.warmup),
        "--iters",
        str(args.iters),
        "--batch-size",
        str(args.batch_size),
        "--batch-iters",
        str(args.batch_iters),
        "--input-mode",
        args.input_mode,
        "--variant-pool",
        str(args.variant_pool),
        "--join-lm-search",
        args.join_lm_search,
    ]
    if args.dataset_tsv:
        py_cmd.extend(["--dataset-tsv", args.dataset_tsv])
        if args.dataset_category:
            py_cmd.extend(["--dataset-category", args.dataset_category])
    if args.python_model_path:
        py_cmd.extend(["--model-path", args.python_model_path])

    commands = {
        RUST_ENGINE: rust_cmd,
        PY_ENGINE: py_cmd,
    }
    run_schedule: list[dict[str, object]] = []
    benchmark_config = {
        "text": args.text,
        "warmup": args.warmup,
        "iters": args.iters,
        "batch_size": args.batch_size,
        "batch_iters": args.batch_iters,
        "input_mode": args.input_mode,
        "variant_pool": args.variant_pool,
        "repeats": args.repeats,
        "join_lm_search": args.join_lm_search,
        "engine_order": args.engine_order,
        "sleep_between_engines_ms": args.sleep_between_engines_ms,
        "sleep_between_runs_ms": args.sleep_between_runs_ms,
        "sink_warning_threshold_pct": args.sink_warning_threshold * 100.0,
        "bootstrap_samples": args.bootstrap_samples,
        "equivalence_band_pct": args.equivalence_band * 100.0,
        "python_model_path": args.python_model_path,
        "rust_cmd": command_to_text(rust_cmd),
        "python_cmd": command_to_text(py_cmd),
    }
    if args.dataset_tsv:
        benchmark_config.update(
            {
                "dataset_tsv": args.dataset_tsv,
                "dataset_category": args.dataset_category or "all",
                "dataset_sha256": dataset_hash,
                "dataset_rows": dataset_audit_info.get("rows", 0),
                "dataset_unique_texts": dataset_audit_info.get("unique_texts", 0),
                "dataset_categories": dataset_audit_info.get("categories", 0),
                "dataset_category_counts": dataset_audit_info.get("category_counts", ""),
                "dataset_char_len_min": dataset_audit_info.get("char_len_min", 0),
                "dataset_char_len_median": dataset_audit_info.get("char_len_median", 0),
                "dataset_char_len_max": dataset_audit_info.get("char_len_max", 0),
            }
        )

    runs: list[RunSample] = []
    run_records: list[dict[str, object]] = []
    for index in range(args.repeats):
        order = run_order_for_repeat(args.engine_order, index)
        run_schedule.append({"repeat": index + 1, "order": order})
        for engine_offset, engine_name in enumerate(order):
            print(f"[run {index + 1}/{args.repeats}] {engine_name}")
            output = run_command(commands[engine_name], cwd)
            sample = parse_run_output(output)
            if sample.engine != engine_name:
                raise RuntimeError(
                    f"engine output mismatch: expected={engine_name}, parsed={sample.engine}"
                )
            runs.append(sample)
            run_records.append(
                {
                    "repeat": index + 1,
                    "engine": sample.engine,
                    "init_ms": sample.init_ms,
                    "features": {
                        feature: {
                            "avg_ms": values.avg_ms,
                            "calls_per_sec": values.calls_per_sec,
                            "sink": values.sink,
                            "iters": values.iters,
                        }
                        for feature, values in sample.features.items()
                    },
                }
            )
            if engine_offset + 1 < len(order) and args.sleep_between_engines_ms > 0:
                time.sleep(args.sleep_between_engines_ms / 1000.0)
        if index + 1 < args.repeats and args.sleep_between_runs_ms > 0:
            time.sleep(args.sleep_between_runs_ms / 1000.0)

    agg = aggregate(runs)
    markdown, sink_warnings = build_markdown(
        agg,
        args.repeats,
        environment,
        benchmark_config,
        args.sink_warning_threshold,
        args.bootstrap_samples,
        args.equivalence_band,
    )
    print()
    print(markdown)

    if args.md_out:
        md_path = (cwd / args.md_out).resolve()
        md_path.parent.mkdir(parents=True, exist_ok=True)
        md_path.write_text(markdown + "\n", encoding="utf-8")
        print(f"\n[written] {md_path}")

    if args.json_out:
        json_path = (cwd / args.json_out).resolve()
        json_path.parent.mkdir(parents=True, exist_ok=True)
        serializable = {
            "metadata": {
                "environment": environment,
                "benchmark_config": benchmark_config,
                "run_schedule": run_schedule,
            },
            "raw_runs": run_records,
            "results": {
                engine: {
                    "init_ms": values["init_ms"],
                    "features": values["features"],
                    "feature_order": values.get("feature_order", []),
                }
                for engine, values in agg.items()
            },
        }
        json_path.write_text(
            json.dumps(serializable, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        print(f"[written] {json_path}")

    if args.strict_sink_check and sink_warnings:
        warning_list = ", ".join(sink_warnings)
        print(
            (
                "strict sink check failed: "
                f"{warning_list} exceeded threshold {args.sink_warning_threshold * 100.0:.1f}%"
            ),
            file=sys.stderr,
        )
        return 2

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
