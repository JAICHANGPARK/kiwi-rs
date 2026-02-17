# Benchmark Dataset Spec

## Purpose

This dataset is a reproducible text-set for runtime performance benchmarking (`kiwi-rs` vs `kiwipiepy`).
It is not an accuracy gold set and should not be used to claim linguistic quality improvements.

## Format

- File type: UTF-8 TSV
- Path (current): `benchmarks/datasets/swe_textset_v2.tsv`
- Line format: `category<TAB>text`
- Empty lines and `#` comments are ignored.
- If a line has no tab, category is interpreted as `default`.

## Reproducibility Rules

- Always publish dataset path + SHA-256 hash.
- Always publish category filter used (`all` or specific category).
- Keep benchmark commands with explicit flags in report metadata.
- Do not modify dataset rows without version bump (`*_v2.tsv`).

## Category Policy

The current v2 dataset includes mixed operational styles:

- `news`
- `colloquial`
- `typo_noisy`
- `code_mixed`
- `ecommerce`
- `finance`
- `tech`
- `longform`

## Suggested Use

- Overall benchmark: run with full dataset (`--dataset-tsv ...`).
- Stratified benchmark: run per category (`--dataset-category <name>`).
- Publish both overall and stratified results.
