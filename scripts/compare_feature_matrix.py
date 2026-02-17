#!/usr/bin/env python3
import argparse
import json
import statistics
import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass
class Scenario:
    name: str
    text: str
    warmup: int
    iters: int
    batch_size: int
    batch_iters: int
    repeats: int
    join_lm_search: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run multiple compare_feature_bench scenarios and summarize Rust/Python ratios."
    )
    parser.add_argument(
        "--python-bin",
        default=".venv-bench/bin/python",
        help="Python executable for running compare_feature_bench.py",
    )
    parser.add_argument(
        "--out-dir",
        default="tmp/feature_matrix",
        help="Directory for per-scenario markdown/json and final matrix summary.",
    )
    return parser.parse_args()


def run_cmd(cmd: list[str], cwd: Path) -> None:
    completed = subprocess.run(cmd, cwd=cwd, text=True, capture_output=True, check=False)
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed ({completed.returncode}): {' '.join(cmd)}\n"
            f"{completed.stdout}\n{completed.stderr}"
        )
    print(completed.stdout, end="")


def median(values: list[float]) -> float:
    return float(statistics.median(values))


def load_json(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as fp:
        return json.load(fp)


def get_results(data: dict) -> dict:
    return data.get("results", data)


def scenario_markdown(name: str, data: dict) -> str:
    results = get_results(data)
    rust = results["kiwi-rs"]
    py = results["kiwipiepy"]
    common = sorted(set(rust["features"]).intersection(py["features"]))
    lines: list[str] = []
    lines.append(f"### {name}")
    lines.append("")
    lines.append("| Feature | kiwi-rs | kiwipiepy | Ratio |")
    lines.append("|---|---:|---:|---:|")
    for feature in common:
        r = median(rust["features"][feature]["calls_per_sec"])
        p = median(py["features"][feature]["calls_per_sec"])
        ratio = r / p if p else 0.0
        lines.append(f"| `{feature}` | {r:.2f} | {p:.2f} | {ratio:.2f}x |")
    lines.append("")
    return "\n".join(lines)


def overall_markdown(all_data: dict[str, dict]) -> str:
    ratios_by_feature: dict[str, list[float]] = {}
    for data in all_data.values():
        results = get_results(data)
        rust = results["kiwi-rs"]
        py = results["kiwipiepy"]
        for feature in sorted(set(rust["features"]).intersection(py["features"])):
            r = median(rust["features"][feature]["calls_per_sec"])
            p = median(py["features"][feature]["calls_per_sec"])
            ratios_by_feature.setdefault(feature, []).append((r / p) if p else 0.0)

    lines: list[str] = []
    lines.append("## Overall Feature Ratio Summary")
    lines.append("")
    lines.append("| Feature | Median Ratio | Min Ratio | Max Ratio | Rust Wins / Total |")
    lines.append("|---|---:|---:|---:|---:|")
    for feature in sorted(ratios_by_feature):
        values = ratios_by_feature[feature]
        wins = sum(1 for value in values if value >= 1.0)
        lines.append(
            f"| `{feature}` | {median(values):.2f}x | {min(values):.2f}x | {max(values):.2f}x | {wins}/{len(values)} |"
        )
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    args = parse_args()
    cwd = Path(__file__).resolve().parents[1]
    out_dir = (cwd / args.out_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    scenarios = [
        Scenario(
            name="short_lm_on",
            text="아버지가방에들어가신다.",
            warmup=50,
            iters=2000,
            batch_size=128,
            batch_iters=200,
            repeats=3,
            join_lm_search="true",
        ),
        Scenario(
            name="short_lm_off",
            text="아버지가방에들어가신다.",
            warmup=50,
            iters=2000,
            batch_size=128,
            batch_iters=200,
            repeats=3,
            join_lm_search="false",
        ),
        Scenario(
            name="long_sentence_lm_on",
            text="오늘은 날씨가 좋아서 한강공원을 천천히 걸었고, 집에 돌아와 따뜻한 차를 마셨다.",
            warmup=30,
            iters=1200,
            batch_size=96,
            batch_iters=150,
            repeats=2,
            join_lm_search="true",
        ),
        Scenario(
            name="mixed_text_lm_on",
            text="Rust와 Python 3.14를 같이 써도 성능 비교는 공정해야 한다.",
            warmup=40,
            iters=1500,
            batch_size=128,
            batch_iters=180,
            repeats=2,
            join_lm_search="true",
        ),
        Scenario(
            name="batch_heavy_lm_on",
            text="데이터 파이프라인 배치 처리 성능을 확인한다.",
            warmup=30,
            iters=1000,
            batch_size=512,
            batch_iters=120,
            repeats=2,
            join_lm_search="true",
        ),
    ]

    all_data: dict[str, dict] = {}
    sections: list[str] = []
    for scenario in scenarios:
        scenario_json = out_dir / f"{scenario.name}.json"
        scenario_md = out_dir / f"{scenario.name}.md"
        cmd = [
            args.python_bin,
            "scripts/compare_feature_bench.py",
            "--text",
            scenario.text,
            "--warmup",
            str(scenario.warmup),
            "--iters",
            str(scenario.iters),
            "--batch-size",
            str(scenario.batch_size),
            "--batch-iters",
            str(scenario.batch_iters),
            "--repeats",
            str(scenario.repeats),
            "--join-lm-search",
            scenario.join_lm_search,
            "--md-out",
            str(scenario_md),
            "--json-out",
            str(scenario_json),
        ]
        print(f"[scenario] {scenario.name}")
        run_cmd(cmd, cwd)
        data = load_json(scenario_json)
        all_data[scenario.name] = data
        sections.append(scenario_markdown(scenario.name, data))

    lines: list[str] = []
    lines.append("# Feature Matrix Benchmark")
    lines.append("")
    lines.append(overall_markdown(all_data))
    for section in sections:
        lines.append(section)
    summary = "\n".join(lines).strip() + "\n"

    summary_path = out_dir / "matrix_summary.md"
    summary_path.write_text(summary, encoding="utf-8")
    print(f"[written] {summary_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
