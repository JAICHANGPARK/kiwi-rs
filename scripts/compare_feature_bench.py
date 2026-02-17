#!/usr/bin/env python3
import argparse
import json
import os
import platform
import re
import statistics
import subprocess
import sys
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path


FEATURE_RE = re.compile(
    r"^feature=([^\s]+)\s+avg_ms=([0-9.]+)\s+calls_per_sec=([0-9.]+)\s+sink=([0-9]+)\s+iters=([0-9]+)$"
)
INIT_RE = re.compile(r"^init_ms=([0-9.]+)$")
ENGINE_RE = re.compile(r"^engine=([^\s]+)$")


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
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument(
        "--join-lm-search",
        type=str,
        default="true",
        help="Whether join uses LM search (true/false).",
    )
    parser.add_argument(
        "--python-bin",
        default=".venv-bench/bin/python",
        help="Python executable used for scripts/bench_features_kiwipiepy.py",
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
    if args.variant_pool <= 0:
        parser.error("--variant-pool must be >= 1")
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
        "git_head": git_head,
        "git_branch": git_branch,
        "git_dirty": git_dirty,
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


def fmt_med_range(values: list[float], digits: int = 2) -> str:
    return f"{median(values):.{digits}f} [{min(values):.{digits}f}-{max(values):.{digits}f}]"


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
                feature, {"avg_ms": [], "calls_per_sec": []}
            )
            feature_bucket["avg_ms"].append(sample.avg_ms)
            feature_bucket["calls_per_sec"].append(sample.calls_per_sec)
    return by_engine


def build_markdown(
    agg: dict[str, object],
    repeats: int,
    environment: dict[str, object],
    benchmark_config: dict[str, object],
    rust_engine: str = "kiwi-rs",
    py_engine: str = "kiwipiepy",
) -> str:
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

    lines = []
    lines.append(
        f"### Expanded Feature Benchmark Snapshot (median of {repeats} runs, min-max shown)"
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
    lines.append("")
    lines.append(
        "Throughput comparison (`calls_per_sec`, higher is better):"
    )
    lines.append("")
    lines.append(
        "| Feature | `kiwi-rs` | `kiwipiepy` | Relative (`kiwi-rs / kiwipiepy`) |"
    )
    lines.append("|---|---:|---:|---:|")

    for feature in common:
        rust_calls = rust_features[feature]["calls_per_sec"]
        py_calls = py_features[feature]["calls_per_sec"]
        rust_med = median(rust_calls)
        py_med = median(py_calls)
        ratio = rust_med / py_med if py_med > 0 else 0.0
        lines.append(
            f"| `{feature}` | {fmt_med_range(rust_calls)} | {fmt_med_range(py_calls)} | {ratio:.2f}x |"
        )

    lines.append("")
    lines.append("Startup (`init_ms`, lower is better):")
    lines.append("")
    lines.append("| Init path | `kiwi-rs` | `kiwipiepy` |")
    lines.append("|---|---:|---:|")
    lines.append(
        f"| `Kiwi::init()` / `Kiwi()` | {fmt_med_range(rust['init_ms'], 3)} ms | {fmt_med_range(py['init_ms'], 3)} ms |"
    )

    if rust_only:
        lines.append("")
        lines.append("Rust-only benchmark features:")
        lines.append("")
        lines.append("| Feature | `kiwi-rs` |")
        lines.append("|---|---:|")
        for feature in rust_only:
            lines.append(
                f"| `{feature}` | {fmt_med_range(rust_features[feature]['calls_per_sec'])} |"
            )

    if py_only:
        lines.append("")
        lines.append("Python-only benchmark features:")
        lines.append("")
        lines.append("| Feature | `kiwipiepy` |")
        lines.append("|---|---:|")
        for feature in py_only:
            lines.append(
                f"| `{feature}` | {fmt_med_range(py_features[feature]['calls_per_sec'])} |"
            )

    return "\n".join(lines)


def main() -> int:
    args = parse_args()
    cwd = Path(__file__).resolve().parents[1]
    environment = collect_environment(cwd, args.python_bin)
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
    }

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

    runs: list[RunSample] = []
    for index in range(args.repeats):
        print(f"[run {index + 1}/{args.repeats}] kiwi-rs")
        rust_output = run_command(rust_cmd, cwd)
        runs.append(parse_run_output(rust_output))
        print(f"[run {index + 1}/{args.repeats}] kiwipiepy")
        py_output = run_command(py_cmd, cwd)
        runs.append(parse_run_output(py_output))

    agg = aggregate(runs)
    markdown = build_markdown(agg, args.repeats, environment, benchmark_config)
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
            },
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

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
