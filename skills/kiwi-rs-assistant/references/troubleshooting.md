# Troubleshooting & Runtime Capability Guide

Map observed errors or missing features to concrete fixes.

## Error-to-Fix Matrix

| Symptom or Error Text | Likely Cause | Actionable Fix |
|---|---|---|
| `failed to load library` | Dynamic library (`libkiwi.dylib` / `libkiwi.so` / `kiwi.dll`) not found | Set `KIWI_LIBRARY_PATH` explicitly, run `make install-kiwi`, or use `Kiwi::init()` for automatic cache bootstrap. |
| `Cannot open extract.mdl for WordDetector` | Model directory path is missing or points to the parent folder instead of the directory containing `.mdl` files | Point `KIWI_MODEL_PATH` (or `BuilderConfig::with_model_path`) to the leaf directory containing `extract.mdl` (e.g. `.../models/cong/base`). |
| `reading type 'Ds' failed` / iostream error | Version mismatch between `libkiwi` binary and model files | Ensure library and model come from the exact same Kiwi release tag (e.g. `v0.24.0`). |
| `symbol is not available in loaded Kiwi library` | Loaded `libkiwi` is an older version that does not export a newer optional symbol | Check the corresponding `supports_*()` method (see table below) or upgrade `libkiwi` to `v0.24.0+`. |
| `byte index X is not a char boundary` panic in user code | Slicing `&text[token.position..]` using `Token.position` as byte index | `Token.position` and `Token.length` are UTF-8 `char` indices (`text.chars()`). Convert via `text.char_indices()` or `Vec<char>`. |
| `bootstrap error` | Missing `curl` / `tar` (or `powershell` on Windows) or network failure | Install `curl` and `tar`, or pre-install assets using `make install-kiwi` and `Kiwi::new()`. |
| `Quantization is not supported for ArchType::neon` | Informational message from Kiwi C++ library on ARM64 when quantized model type is requested | Harmless (Kiwi automatically falls back to non-quantized model), or specify `KIWI_MODEL_CONG` explicitly. |

## Optional C API Capability Checks (`supports_*`)

Always gate optional C API features when the user's `libkiwi` version may be older than `v0.24.0`:

| Feature / Method Family | Capability Check Method | Fallback if `false` |
|---|---|---|
| `*_utf16*` APIs | `kiwi.supports_utf16_api()` | Use UTF-8 APIs (`tokenize`, `analyze`, `split_into_sents`) |
| `analyze_many_utf16_via_native` | `kiwi.supports_analyze_mw()` | Use `analyze_many_via_native` (UTF-8) or loop over `analyze_utf16` |
| `KiwiBuilder::from_stream` | `library.supports_builder_init_stream()` | Load models from filesystem via `library.builder(config)` |
| `add_user_word_with_def`, `add_alias_word_with_def` | `library.supports_builder_word_with_def()` | Use `add_user_word` and `add_alias_word` |
| `KiwiTypo::prepare`, `PreparedTypo`, `*_with_typo` | `library.supports_typo_prepare()` / `kiwi.supports_typo_prepare()` | Bake typo into `Kiwi` at build time with `builder.build_with_typo_and_default_options(&typo, opts)` |
| `space_via_native`, `glue_via_native`, `glue_with_options_via_native` | `kiwi.supports_native_space_and_glue()` | Use Rust-side `kiwi.space(...)` and `kiwi.glue_with_options(...)` |
