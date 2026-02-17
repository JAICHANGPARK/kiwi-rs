#!/usr/bin/env python3
import argparse
import hashlib
import json
import re
import statistics
import subprocess
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run dataset-wide and category-stratified feature benchmarks."
    )
    parser.add_argument(
        "--python-bin",
        default=".venv-bench/bin/python",
        help="Python executable used to run compare_feature_bench.py",
    )
    parser.add_argument(
        "--dataset-tsv",
        required=True,
        help="Dataset TSV path (`category<TAB>text`).",
    )
    parser.add_argument("--text", default="아버지가방에들어가신다.")
    parser.add_argument("--warmup", type=int, default=100)
    parser.add_argument("--iters", type=int, default=5000)
    parser.add_argument("--batch-size", type=int, default=256)
    parser.add_argument("--batch-iters", type=int, default=500)
    parser.add_argument(
        "--input-mode",
        choices=("repeated", "varied"),
        default="varied",
    )
    parser.add_argument("--variant-pool", type=int, default=4096)
    parser.add_argument("--repeats", type=int, default=5)
    parser.add_argument("--join-lm-search", default="true")
    parser.add_argument(
        "--engine-order",
        choices=("alternate", "rust-first", "python-first"),
        default="alternate",
    )
    parser.add_argument("--sleep-between-engines-ms", type=int, default=100)
    parser.add_argument("--sleep-between-runs-ms", type=int, default=200)
    parser.add_argument("--sink-warning-threshold", type=float, default=0.05)
    parser.add_argument("--bootstrap-samples", type=int, default=2000)
    parser.add_argument("--equivalence-band", type=float, default=0.05)
    parser.add_argument(
        "--max-categories",
        type=int,
        default=0,
        help="Limit number of categories (0 means all).",
    )
    parser.add_argument(
        "--out-dir",
        default="tmp/feature_dataset_matrix",
        help="Output directory for markdown/json artifacts.",
    )
    return parser.parse_args()


def load_dataset_rows(path: Path) -> list[tuple[str, str]]:
    if not path.exists():
        raise RuntimeError(f"dataset file not found: {path}")
    rows: list[tuple[str, str]] = []
    for line_no, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
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
        raise RuntimeError("dataset has zero usable rows")
    return rows


def dataset_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as fp:
        while True:
            chunk = fp.read(1024 * 1024)
            if not chunk:
                break
            digest.update(chunk)
    return digest.hexdigest()


def median(values: list[float]) -> float:
    return float(statistics.median(values))


def run_cmd(cmd: list[str], cwd: Path) -> None:
    completed = subprocess.run(
        cmd,
        cwd=cwd,
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed ({completed.returncode}): {' '.join(cmd)}\n"
            f"{completed.stdout}\n{completed.stderr}"
        )


def load_json(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as fp:
        return json.load(fp)


def sanitize(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", value).strip("_") or "category"


def feature_ratio(data: dict, feature: str) -> float:
    results = data["results"]
    rust = results["kiwi-rs"]["features"][feature]["calls_per_sec"]
    py = results["kiwipiepy"]["features"][feature]["calls_per_sec"]
    rust_med = median([float(value) for value in rust])
    py_med = median([float(value) for value in py])
    if py_med <= 0:
        return 0.0
    return rust_med / py_med


def common_features(data: dict) -> list[str]:
    results = data["results"]
    rust_features = set(results["kiwi-rs"]["features"].keys())
    py_features = set(results["kiwipiepy"]["features"].keys())
    return sorted(rust_features.intersection(py_features))


def build_summary_markdown(
    dataset_path: Path,
    dataset_hash: str,
    rows: list[tuple[str, str]],
    artifacts: dict[str, dict],
) -> str:
    category_counts: dict[str, int] = {}
    for category, _ in rows:
        category_counts[category] = category_counts.get(category, 0) + 1

    overall = artifacts["overall"]["json"]
    features = common_features(overall)
    lines: list[str] = []
    lines.append("# Dataset-Stratified Feature Benchmark")
    lines.append("")
    lines.append("## Dataset")
    lines.append("")
    lines.append(f"- path: `{dataset_path}`")
    lines.append(f"- sha256: `{dataset_hash}`")
    lines.append(f"- rows: {len(rows)}")
    lines.append(f"- categories: {len(category_counts)}")
    lines.append("")
    lines.append("| Category | Rows |")
    lines.append("|---|---:|")
    for category in sorted(category_counts):
        lines.append(f"| `{category}` | {category_counts[category]} |")
    lines.append("")

    lines.append("## Category Summary")
    lines.append("")
    lines.append("| Category | Median Ratio | Rust Wins / Total | Worst Feature | Best Feature | Report |")
    lines.append("|---|---:|---:|---|---|---|")
    for category in sorted(key for key in artifacts.keys() if key != "overall"):
        data = artifacts[category]["json"]
        ratios: list[tuple[str, float]] = [
            (feature, feature_ratio(data, feature)) for feature in common_features(data)
        ]
        if not ratios:
            continue
        ratio_values = [ratio for _, ratio in ratios]
        wins = sum(1 for ratio in ratio_values if ratio >= 1.0)
        worst_feature, worst_ratio = min(ratios, key=lambda item: item[1])
        best_feature, best_ratio = max(ratios, key=lambda item: item[1])
        md_path = artifacts[category]["md_path"]
        lines.append(
            f"| `{category}` | {median(ratio_values):.2f}x | {wins}/{len(ratios)} | `{worst_feature}` ({worst_ratio:.2f}x) | `{best_feature}` ({best_ratio:.2f}x) | `{md_path}` |"
        )
    lines.append("")

    lines.append("## Feature Spread Across Categories")
    lines.append("")
    lines.append("| Feature | Median Ratio | Min Ratio | Max Ratio | Rust Wins / Total Categories |")
    lines.append("|---|---:|---:|---:|---:|")
    category_keys = sorted(key for key in artifacts.keys() if key != "overall")
    for feature in features:
        values = [feature_ratio(artifacts[key]["json"], feature) for key in category_keys]
        wins = sum(1 for value in values if value >= 1.0)
        lines.append(
            f"| `{feature}` | {median(values):.2f}x | {min(values):.2f}x | {max(values):.2f}x | {wins}/{len(values)} |"
        )
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    args = parse_args()
    cwd = Path(__file__).resolve().parents[1]
    dataset_path = (cwd / args.dataset_tsv).resolve()
    rows = load_dataset_rows(dataset_path)
    dataset_hash = dataset_sha256(dataset_path)
    categories = sorted({category for category, _ in rows})
    if args.max_categories > 0:
        categories = categories[: args.max_categories]

    out_dir = (cwd / args.out_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    base_args = [
        args.python_bin,
        "scripts/compare_feature_bench.py",
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
        "--repeats",
        str(args.repeats),
        "--join-lm-search",
        args.join_lm_search,
        "--engine-order",
        args.engine_order,
        "--sleep-between-engines-ms",
        str(args.sleep_between_engines_ms),
        "--sleep-between-runs-ms",
        str(args.sleep_between_runs_ms),
        "--sink-warning-threshold",
        str(args.sink_warning_threshold),
        "--bootstrap-samples",
        str(args.bootstrap_samples),
        "--equivalence-band",
        str(args.equivalence_band),
        "--strict-sink-check",
        "--dataset-tsv",
        str(dataset_path),
    ]

    artifacts: dict[str, dict] = {}
    overall_md = out_dir / "overall.md"
    overall_json = out_dir / "overall.json"
    print("[scope] overall")
    run_cmd(
        base_args
        + [
            "--md-out",
            str(overall_md),
            "--json-out",
            str(overall_json),
        ],
        cwd,
    )
    artifacts["overall"] = {
        "md_path": str(overall_md.relative_to(cwd)),
        "json_path": str(overall_json.relative_to(cwd)),
        "json": load_json(overall_json),
    }

    for category in categories:
        safe = sanitize(category)
        scope_md = out_dir / f"category_{safe}.md"
        scope_json = out_dir / f"category_{safe}.json"
        print(f"[scope] {category}")
        run_cmd(
            base_args
            + [
                "--dataset-category",
                category,
                "--md-out",
                str(scope_md),
                "--json-out",
                str(scope_json),
            ],
            cwd,
        )
        artifacts[category] = {
            "md_path": str(scope_md.relative_to(cwd)),
            "json_path": str(scope_json.relative_to(cwd)),
            "json": load_json(scope_json),
        }

    summary = build_summary_markdown(dataset_path, dataset_hash, rows, artifacts)
    summary_path = out_dir / "matrix_summary.md"
    summary_path.write_text(summary + "\n", encoding="utf-8")
    print(f"[written] {summary_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
