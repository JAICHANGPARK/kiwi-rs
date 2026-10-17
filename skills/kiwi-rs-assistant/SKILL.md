---
name: kiwi-rs-assistant
description: >-
  Guide AI agents to write accurate, idiomatic Rust code using kiwi-rs for Korean morphological analysis, tokenization, sentence splitting, spacing/gluing, custom dictionaries, typo correction, CoNg semantics, and subword tokenization. Use whenever the user asks to build, customize, or debug Korean NLP features with kiwi-rs.
---

# kiwi-rs Assistant

Use this skill to produce runnable, minimal, and accurate `kiwi-rs` solutions aligned with Kiwi `v0.24.0` (`106/106` C API coverage).

## Workflow

1. **Identify Primary Intent**
   - Map the user's request to the smallest API family that solves the task:
     - Quick tokenization / morphological analysis: `Kiwi::tokenize*`, `Kiwi::analyze*`
     - Sentence boundary segmentation: `Kiwi::split_into_sents*`
     - Spacing & chunk gluing / morpheme joining: `Kiwi::space`, `Kiwi::space_via_native`, `Kiwi::glue*`, `Kiwi::glue_via_native`, `Kiwi::join*`
     - Custom dictionary / sense / dialect / regex rule builder: `KiwiBuilder` (`add_user_word`, `add_user_word_with_def`, `add_alias_word_with_def`, `add_pre_analyzed_word`, `add_re_rule`)
     - Typo correction (`KiwiTypo`, `PreparedTypo`, `KIWI_TYPO_DIALECT`, `analyze_with_typo`, `tokenize_with_typo`)
     - Constrained analysis (`MorphemeSet` blocklist, `Pretokenized` forced spans)
     - Global & OOV configuration (`GlobalConfig`, `oov_chr_bias`, `oov_global_weight`, `KIWI_MATCH_OOV_*`)
     - CoNg semantic similarity & context prediction (`find_morphemes`, `most_similar_morphemes`, `predict_next_morpheme`)
     - Subword tokenization (`SwTokenizer`)
   - Consult [references/task-to-api.md](./references/task-to-api.md) for API routing and [references/recipes.md](./references/recipes.md) for copy-pasteable code templates.

2. **Choose One Initialization Path Deliberately**
   - `Kiwi::init()`: Best default for quick starts; checks local paths first, then downloads matching library + model assets to cache (`KIWI_RS_VERSION`, `KIWI_RS_CACHE_DIR`).
   - `Kiwi::new()`: Use when `KIWI_LIBRARY_PATH` and `KIWI_MODEL_PATH` are already managed by environment variables.
   - `Kiwi::from_config(KiwiConfig::default()...)`: Use when explicit library/model paths and simple user words are configured in code.
   - `KiwiLibrary::load_from_env_or_default()?.builder(BuilderConfig::default())?`: Use when adding custom words with definitions (`add_user_word_with_def`), alias words (`add_alias_word_with_def`), pre-analyzed words, regex rules, word extraction, or build-time typo transformers (`build_with_typo_and_default_options`).
   - `Kiwi::init_direct_with_dialects(model_path, num_threads, build_options, enabled_dialects)`: Use when directly initializing with specific regional dialect flags (`KIWI_DIALECT_*`).

3. **Generate Runnable Rust Code**
   - Always return complete snippets (`fn main() -> Result<(), Box<dyn std::error::Error>>`).
   - Import only required symbols from `kiwi_rs`.
   - Use Korean sample sentences unless the user specifies otherwise.

4. **Include a Verification Step**
   - Provide at least one concrete verification command (`cargo run --example <name>` or `cargo run`).

## Critical Accuracy Guardrails

- **UTF-8 Offsets Are Character Indices**: `Token.position` and `Token.length` in UTF-8 APIs count `char`s (`str.chars()`), **not** byte offsets. Never slice `&text[token.position..]` directly with byte indexing on Korean text.
- **Runtime Feature Checks**: Gate optional C API symbols when targeting older `libkiwi` binaries:
  - `kiwi.supports_utf16_api()` before `*_utf16*` APIs.
  - `kiwi.supports_analyze_mw()` before `analyze_many_utf16_via_native`.
  - `library.supports_builder_word_with_def()` before `add_user_word_with_def` / `add_alias_word_with_def`.
  - `kiwi.supports_typo_prepare()` before `KiwiTypo::prepare` / `PreparedTypo` runtime APIs.
  - `kiwi.supports_native_space_and_glue()` before `space_via_native` / `glue_via_native` / `glue_with_options_via_native` (otherwise use `Kiwi::space` and `Kiwi::glue`).
- **PreparedTypo Reuse**: Compile `KiwiTypo` once via `typo.prepare()?` into a [`PreparedTypo`] (`Send + Sync`) and reuse it via `kiwi.set_default_prepared_typo(Some(prepared))` or `kiwi.tokenize_with_typo(text, options, Some(&prepared))`.

## Response Contract

Structure answers in this order:
1. **Init Path**: Selected initialization method and why it fits the user's environment.
2. **Code**: Complete, runnable Rust code snippet.
3. **Verify**: Command to run and verify output.
4. **Pitfalls**: 2–3 task-specific caveats (offsets, capability checks, or builder lifecycle).

## References

- [Task-to-API Routing Table](./references/task-to-api.md)
- [Copy-Paste Code Recipes](./references/recipes.md)
- [Troubleshooting & Error Resolution](./references/troubleshooting.md)
