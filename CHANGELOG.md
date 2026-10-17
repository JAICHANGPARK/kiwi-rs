# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2026.10.17] - 2026-10-17

### Added
- Full Kiwi `v0.24.0` and upstream `HEAD` (`capi.h` `106/106` symbols) C API alignment and dynamic symbol loading:
  - `kiwi_builder_add_word_with_def` and `kiwi_builder_add_alias_word_with_def` (`KiwiBuilder::add_user_word_with_def`, `KiwiBuilder::add_alias_word_with_def`, `KiwiLibrary::supports_builder_word_with_def`, `Kiwi::supports_builder_word_with_def`).
  - `kiwi_space`, `kiwi_glue`, and `kiwi_free_string` (`Kiwi::space_via_native`, `Kiwi::glue_via_native`, `Kiwi::glue_with_options_via_native`, `KiwiLibrary::supports_native_space_and_glue`, `Kiwi::supports_native_space_and_glue`).
  - `KIWI_TYPO_DIALECT` (`6`) constant for dialect-aware typo preset sets.
  - `Kiwi::init_direct_with_dialects` supporting the 4-argument `kiwi_init(model_path, num_threads, options, enabled_dialects)` C signature.
- Runtime typo transformer (`PreparedTypo`) integration during analysis and tokenization:
  - Added `AnalyzeOptions::typo_threshold` (default `2.5`) and `.with_typo_threshold()` / `.with_typo_cost_threshold()` builder methods.
  - Added `Kiwi::default_prepared_typo` and `Kiwi::set_default_prepared_typo`, plus automatic `PreparedTypo` initialization in `KiwiBuilder::build_with_typo_and_default_options`.
  - Added `Kiwi::analyze_with_typo`, `Kiwi::analyze_with_blocklist_and_pretokenized_and_typo`, `Kiwi::tokenize_with_typo`, `Kiwi::tokenize_with_blocklist_and_pretokenized_and_typo`, and UTF-16 counterparts (`analyze_utf16_with_typo`, `analyze_utf16_with_blocklist_and_pretokenized_and_typo`, `tokenize_utf16_with_typo`, `tokenize_utf16_with_blocklist_and_pretokenized_and_typo`).
- Added `GlobalConfig` shortcut getters and setters on `Kiwi` for Kiwi 0.23+ OOV and unknown form parameters (`max_unk_form_size_followed_by_j_class`, `unk_form_score_scale` / `oov_rule_scale`, `unk_form_score_bias` / `oov_rule_bias`, `oov_chr_bias`, `oov_global_weight`, `oov_local_weight`, `oov_global_min_freq`).
- Added and expanded AI agent skills (`skills/kiwi-rs-assistant`, `skills/kiwi-rs-migration`) and workspace skill discovery manifest (`.agents/skills.json`).

### Changed
- Updated `FnKiwiInit` FFI signature to pass `enabled_dialects` (`KIWI_DIALECT_STANDARD` by default in `Kiwi::init_direct`) to match upstream `kiwi_init` in Kiwi `v0.24.0`.
- Updated Rust spacing heuristics (`should_insert_space_between` and `should_strip_gap`) to include `"하지"` in auxiliary verb (`VX`) spacing rules, matching upstream `src/Spacing.cpp`.
- Implemented `Send` and `Sync` for immutable `PreparedTypo` handle wrapper.

## [2026.8.26] - 2026-08-25

### Fixed
- Fixed C ABI layout mismatch in `KiwiGlobalConfigRaw` (`kiwi_config_t`) by aligning to Kiwi >= 0.23.0 52-byte layout with OOV parameters, preventing potential stack overflow and field offset shifts during `global_config` get/set operations.

### Added
- Added `PreparedTypo` RAII wrapper and `KiwiTypo::prepare` method for pre-compiled typo transformers (`kiwi_typo_prepare`, `kiwi_prepared_typo_close`).
- Added Kiwi 0.23 match option constants (`KIWI_MATCH_EMOJI`, `KIWI_MATCH_OOV_RULE_ONLY`, `KIWI_MATCH_OOV_CHR_MODEL`, `KIWI_MATCH_OOV_CHR_FREQ_MODEL`, `KIWI_MATCH_OOV_CHR_FREQ_BRANCH_MODEL`, `KIWI_MATCH_OOV_MASK`, `KIWI_MATCH_JOIN_PARTICLE_YO`, `KIWI_MATCH_USE_OLD_SPLITTER`).
- Updated `KIWI_MATCH_ALL` to include `KIWI_MATCH_EMOJI` matching Kiwi 0.23 official definition.
- Extended `GlobalConfig` with full OOV extraction parameters (`oov_chr_bias`, `oov_global_weight`, `oov_local_weight`, `oov_global_min_freq`, `max_unk_form_size_followed_by_j_class`).

## [2026.8.25] - 2026-08-25

### Fixed
- Fixed ABI mismatch with Kiwi >= 0.23.0 in `KiwiAnalyzeOption` by adding `typo_transformer` and `typo_threshold` fields, resolving segmentation faults during `analyze()` / `tokenize()` calls (#1, #2, #3).

### Added
- Added integration test coverage for `Kiwi::global_config` and `Kiwi::set_global_config`.

## [2026.7.24] - 2026-07-24

### Fixed
- Fixed null-termination in `rule_replacer_callback` FFI boundary to prevent memory buffer overflow during rule additions.
- Resolved Clippy warnings (`vec_box`) and MSRV 1.70 compatibility issue in `examples/bench_features.rs`.

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
