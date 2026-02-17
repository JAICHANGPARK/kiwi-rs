#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
from collections import Counter
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class TokenUnit:
    start: int
    end: int
    form: str
    tag: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Compare structural output parity between kiwi-rs and kiwipiepy on a text dataset."
        )
    )
    parser.add_argument(
        "--dataset-tsv",
        default="benchmarks/datasets/swe_textset_v2.tsv",
        help="Dataset TSV path (category<TAB>text).",
    )
    parser.add_argument(
        "--rust-jsonl",
        default="tmp/structural_parity/kiwi_rs_outputs.jsonl",
        help="Path for kiwi-rs NDJSON output.",
    )
    parser.add_argument(
        "--md-out",
        default="tmp/structural_parity/report.md",
        help="Markdown report output path.",
    )
    parser.add_argument(
        "--json-out",
        default="tmp/structural_parity/report.json",
        help="JSON report output path.",
    )
    parser.add_argument(
        "--skip-rust-run",
        action="store_true",
        help="Skip generating kiwi-rs outputs and reuse --rust-jsonl.",
    )
    return parser.parse_args()


def run_command(command: list[str], cwd: Path) -> None:
    subprocess.run(command, cwd=cwd, check=True)


def load_dataset_rows(path: Path) -> list[tuple[str, str]]:
    rows: list[tuple[str, str]] = []
    for line_no, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if "\t" not in line:
            raise ValueError(f"dataset line {line_no} does not contain tab separator")
        category, text = line.split("\t", 1)
        category = category.strip()
        text = text.strip()
        if not text:
            raise ValueError(f"dataset line {line_no} has empty text")
        rows.append((category, text))
    if not rows:
        raise ValueError("dataset produced zero rows")
    return rows


def load_rust_rows(path: Path) -> list[dict]:
    rows: list[dict] = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        raw = raw.strip()
        if not raw:
            continue
        rows.append(json.loads(raw))
    return rows


def to_token_units(tokens: list[dict]) -> list[TokenUnit]:
    out: list[TokenUnit] = []
    for token in tokens:
        start = int(token["start"])
        length = int(token["len"])
        out.append(
            TokenUnit(
                start=start,
                end=start + length,
                form=str(token["form"]),
                tag=str(token["tag"]),
            )
        )
    return out


def byte_to_char_index(text: str, byte_index: int) -> int:
    if byte_index <= 0:
        return 0
    encoded = text.encode("utf-8")
    if byte_index >= len(encoded):
        return len(text)
    return len(encoded[:byte_index].decode("utf-8", errors="ignore"))


def to_sent_bounds(items: list[list[int]], text: str) -> list[tuple[int, int]]:
    bounds = [(int(begin), int(end)) for begin, end in items]
    if not bounds:
        return bounds
    # Kiwi C API sentence boundaries are byte offsets in some paths.
    # Normalize to char offsets to align with kiwipiepy.
    if any(end > len(text) for _, end in bounds):
        return [
            (byte_to_char_index(text, begin), byte_to_char_index(text, end))
            for begin, end in bounds
        ]
    return bounds


def build_markdown(
    dataset_path: Path,
    totals: dict,
    confusion: Counter[tuple[str, str]],
    category_stats: dict[str, dict[str, int]],
    examples: list[dict],
) -> str:
    lines: list[str] = []
    lines.append("# Structural Parity Report (kiwi-rs vs kiwipiepy)")
    lines.append("")
    lines.append(f"- dataset: `{dataset_path}`")
    lines.append(f"- rows: {totals['rows']}")
    lines.append("")
    lines.append("## Core Metrics")
    lines.append("")
    lines.append("| Metric | Value |")
    lines.append("|---|---:|")
    lines.append(
        f"| Exact token-sequence match rate | {totals['exact_token_seq_matches']}/{totals['rows']} ({totals['exact_token_seq_rate']:.2%}) |"
    )
    lines.append(
        f"| Exact sentence-boundary match rate | {totals['exact_sent_matches']}/{totals['rows']} ({totals['exact_sent_rate']:.2%}) |"
    )
    lines.append(
        f"| Token boundary precision | {totals['boundary_precision']:.4f} |"
    )
    lines.append(f"| Token boundary recall | {totals['boundary_recall']:.4f} |")
    lines.append(f"| Token boundary F1 | {totals['boundary_f1']:.4f} |")
    lines.append(
        f"| Token (span+form+tag) precision | {totals['token_precision']:.4f} |"
    )
    lines.append(f"| Token (span+form+tag) recall | {totals['token_recall']:.4f} |")
    lines.append(f"| Token (span+form+tag) F1 | {totals['token_f1']:.4f} |")
    lines.append(
        f"| POS agreement on shared spans | {totals['pos_shared_match']}/{totals['pos_shared_total']} ({totals['pos_shared_rate']:.2%}) |"
    )
    lines.append("")
    lines.append("## Error Taxonomy (row-level counts)")
    lines.append("")
    lines.append("| Type | Rows |")
    lines.append("|---|---:|")
    lines.append(
        f"| sentence-boundary mismatch | {totals['rows_with_sent_mismatch']} |"
    )
    lines.append(f"| token-boundary mismatch | {totals['rows_with_span_mismatch']} |")
    lines.append(f"| POS mismatch on shared span | {totals['rows_with_pos_mismatch']} |")
    lines.append("")
    lines.append("## Top POS Confusions (shared spans)")
    lines.append("")
    lines.append("| kiwi-rs tag | kiwipiepy tag | Count |")
    lines.append("|---|---|---:|")
    for (left, right), count in confusion.most_common(12):
        lines.append(f"| `{left}` | `{right}` | {count} |")
    if not confusion:
        lines.append("| - | - | 0 |")
    lines.append("")
    lines.append("## Category Breakdown (exact token-sequence match)")
    lines.append("")
    lines.append("| Category | Matches | Total | Rate |")
    lines.append("|---|---:|---:|---:|")
    for category in sorted(category_stats):
        matched = category_stats[category]["exact_token_seq_matches"]
        total = category_stats[category]["rows"]
        rate = matched / total if total else 0.0
        lines.append(f"| `{category}` | {matched} | {total} | {rate:.2%} |")
    lines.append("")
    lines.append("## Representative Mismatch Examples")
    lines.append("")
    if not examples:
        lines.append("- No mismatches detected.")
    for index, item in enumerate(examples, start=1):
        lines.append(f"{index}. category=`{item['category']}`, row={item['row_index']}")
        lines.append(f"   - text: {item['text']}")
        lines.append(f"   - sentence_match: {item['sentence_match']}")
        lines.append(f"   - span_match: {item['span_match']}")
        lines.append(f"   - pos_shared_match_rate: {item['pos_shared_match_rate']:.2%}")
        if item["pos_confusions"]:
            lines.append(f"   - pos_confusions: {item['pos_confusions']}")
    lines.append("")
    return "\n".join(lines)


def safe_ratio(numerator: float, denominator: float) -> float:
    if denominator == 0:
        return 0.0
    return numerator / denominator


def main() -> int:
    args = parse_args()
    root = Path(__file__).resolve().parents[1]
    dataset_path = (root / args.dataset_tsv).resolve()
    rust_jsonl = (root / args.rust_jsonl).resolve()
    md_out = (root / args.md_out).resolve()
    json_out = (root / args.json_out).resolve()

    md_out.parent.mkdir(parents=True, exist_ok=True)
    json_out.parent.mkdir(parents=True, exist_ok=True)
    rust_jsonl.parent.mkdir(parents=True, exist_ok=True)

    if not args.skip_rust_run:
        run_command(
            [
                "cargo",
                "run",
                "--release",
                "--example",
                "dump_structural_outputs",
                "--",
                "--dataset-tsv",
                str(dataset_path),
                "--out",
                str(rust_jsonl),
            ],
            root,
        )

    rows = load_dataset_rows(dataset_path)
    rust_rows = load_rust_rows(rust_jsonl)
    if len(rows) != len(rust_rows):
        raise RuntimeError(
            f"row count mismatch: dataset={len(rows)} rust_jsonl={len(rust_rows)}"
        )

    from kiwipiepy import Kiwi, Match

    kiwi = Kiwi()

    totals = {
        "rows": len(rows),
        "exact_token_seq_matches": 0,
        "exact_sent_matches": 0,
        "rows_with_sent_mismatch": 0,
        "rows_with_span_mismatch": 0,
        "rows_with_pos_mismatch": 0,
    }
    confusion: Counter[tuple[str, str]] = Counter()
    category_stats: dict[str, dict[str, int]] = {}
    examples: list[dict] = []

    boundary_intersection = 0
    boundary_rust_total = 0
    boundary_py_total = 0

    token_intersection = 0
    token_rust_total = 0
    token_py_total = 0

    pos_shared_total = 0
    pos_shared_match = 0

    for i, (category, text) in enumerate(rows):
        rust = rust_rows[i]
        rust_tokens = to_token_units(rust["tokens"])
        rust_sents = to_sent_bounds(rust["sents"], text)

        py_tokens_obj = kiwi.tokenize(text, match_options=Match.ALL)
        py_tokens = [
            TokenUnit(
                start=int(token.start),
                end=int(token.end),
                form=str(token.form),
                tag=str(token.tag),
            )
            for token in py_tokens_obj
        ]
        py_sents_obj = kiwi.split_into_sents(text, return_tokens=True)
        py_sents = [(int(sent.start), int(sent.end)) for sent in py_sents_obj]

        rust_seq = [(t.form, t.tag, t.start, t.end) for t in rust_tokens]
        py_seq = [(t.form, t.tag, t.start, t.end) for t in py_tokens]
        exact_token_seq = rust_seq == py_seq
        exact_sent = rust_sents == py_sents
        if exact_token_seq:
            totals["exact_token_seq_matches"] += 1
        if exact_sent:
            totals["exact_sent_matches"] += 1
        if not exact_sent:
            totals["rows_with_sent_mismatch"] += 1

        cat = category_stats.setdefault(
            category, {"rows": 0, "exact_token_seq_matches": 0}
        )
        cat["rows"] += 1
        if exact_token_seq:
            cat["exact_token_seq_matches"] += 1

        rust_spans = {(t.start, t.end) for t in rust_tokens}
        py_spans = {(t.start, t.end) for t in py_tokens}
        shared_spans = rust_spans & py_spans
        if rust_spans != py_spans:
            totals["rows_with_span_mismatch"] += 1

        boundary_intersection += len(shared_spans)
        boundary_rust_total += len(rust_spans)
        boundary_py_total += len(py_spans)

        rust_token_set = {(t.start, t.end, t.form, t.tag) for t in rust_tokens}
        py_token_set = {(t.start, t.end, t.form, t.tag) for t in py_tokens}
        shared_token_set = rust_token_set & py_token_set

        token_intersection += len(shared_token_set)
        token_rust_total += len(rust_token_set)
        token_py_total += len(py_token_set)

        rust_by_span = {(t.start, t.end): t.tag for t in rust_tokens}
        py_by_span = {(t.start, t.end): t.tag for t in py_tokens}
        row_pos_shared_total = len(shared_spans)
        row_pos_shared_match = 0
        row_confusions: Counter[tuple[str, str]] = Counter()
        for span in sorted(shared_spans):
            left = rust_by_span[span]
            right = py_by_span[span]
            pos_shared_total += 1
            if left == right:
                pos_shared_match += 1
                row_pos_shared_match += 1
            else:
                confusion[(left, right)] += 1
                row_confusions[(left, right)] += 1

        if row_confusions:
            totals["rows_with_pos_mismatch"] += 1

        if (not exact_token_seq or not exact_sent or row_confusions) and len(examples) < 8:
            examples.append(
                {
                    "row_index": i,
                    "category": category,
                    "text": text,
                    "sentence_match": exact_sent,
                    "span_match": rust_spans == py_spans,
                    "pos_shared_match_rate": safe_ratio(
                        row_pos_shared_match, row_pos_shared_total
                    ),
                    "pos_confusions": [
                        {"rust": left, "py": right, "count": count}
                        for (left, right), count in row_confusions.most_common(4)
                    ],
                }
            )

    boundary_precision = safe_ratio(boundary_intersection, boundary_rust_total)
    boundary_recall = safe_ratio(boundary_intersection, boundary_py_total)
    boundary_f1 = safe_ratio(
        2 * boundary_precision * boundary_recall, boundary_precision + boundary_recall
    )

    token_precision = safe_ratio(token_intersection, token_rust_total)
    token_recall = safe_ratio(token_intersection, token_py_total)
    token_f1 = safe_ratio(2 * token_precision * token_recall, token_precision + token_recall)

    totals["exact_token_seq_rate"] = safe_ratio(
        totals["exact_token_seq_matches"], totals["rows"]
    )
    totals["exact_sent_rate"] = safe_ratio(totals["exact_sent_matches"], totals["rows"])
    totals["boundary_precision"] = boundary_precision
    totals["boundary_recall"] = boundary_recall
    totals["boundary_f1"] = boundary_f1
    totals["token_precision"] = token_precision
    totals["token_recall"] = token_recall
    totals["token_f1"] = token_f1
    totals["pos_shared_total"] = pos_shared_total
    totals["pos_shared_match"] = pos_shared_match
    totals["pos_shared_rate"] = safe_ratio(pos_shared_match, pos_shared_total)

    payload = {
        "dataset": str(dataset_path),
        "totals": totals,
        "confusion_top": [
            {"rust": left, "py": right, "count": count}
            for (left, right), count in confusion.most_common(32)
        ],
        "category_stats": category_stats,
        "examples": examples,
    }

    md_out.write_text(
        build_markdown(dataset_path, totals, confusion, category_stats, examples),
        encoding="utf-8",
    )
    json_out.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )

    print(f"wrote markdown: {md_out}")
    print(f"wrote json: {json_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
