# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.4] - 2026-02-17

### Changed
- Refined benchmark readability in `README.md` and `README.ko.md` by:
  - splitting absolute-value charts by engine to avoid overlapping bars,
  - adding side-by-side numeric comparison tables for varied-input runs.
- Corrected license metadata/docs to `LGPL-2.1-or-later` to match `LICENSE`.

## [0.1.3] - 2026-02-17

### Changed
- Expanded benchmark documentation in `README.md` and `README.ko.md` with:
  - percentage delta columns for repeated/varied ratio snapshots,
  - absolute throughput bar charts (`calls/sec`),
  - absolute latency bar charts (`avg_ms`),
  - clearer metric interpretation guidance for end users.

## [0.1.2] - 2026-02-17

### Added
- Added benchmark examples: `bench_tokenize`, `bench_glue`, `bench_features`.
- Added Python benchmark helpers and comparison tools for Rust vs `kiwipiepy`:
  `bench_kiwipiepy.py`, `bench_features_kiwipiepy.py`,
  `compare_feature_bench.py`, and `compare_feature_matrix.py`.
- Added weekly feature benchmark workflow: `.github/workflows/feature-benchmark.yml`.

### Changed
- Improved runtime inference caching paths for tokenize/analyze/split/glue hot paths.
- Optimized `glue` with reduced allocation cost and pair-decision reuse cache.
- Added repeated/varied input benchmark modes for fair warm-cache vs near no-cache comparison.
- Updated benchmark documentation in `README.md` and `README.ko.md` with
  environment metadata and visual bar charts.

## [0.1.1] - 2026-02-16

### Added
- Added `kiwi-rs` to `crates.io`.

### Changed
- Changed the repository to `https://github.com/JAICHANGPARK/kiwi-rs`.

## [0.1.0] - 2026-02-16

### Added
- Initial `kiwi-rs` release.
- Core Kiwi C API loading and high-level Rust wrapper APIs.
- Model/bootstrap installer scripts and cache-based runtime initialization.
- Example programs and bilingual documentation (`README.md`, `README.ko.md`).
