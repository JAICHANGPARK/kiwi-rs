---
name: kiwi-rs-migration
description: >-
  Migrate or translate Python kiwipiepy code into idiomatic Rust using kiwi-rs. Use whenever the user shares kiwipiepy Python code, asks for Python-to-Rust Korean NLP porting, or asks how a kiwipiepy class/method maps to kiwi-rs.
---

# kiwipiepy to kiwi-rs Migration Guide

Use this skill to translate Python `kiwipiepy` code into accurate, high-performance Rust `kiwi-rs` code.

## Migration Workflow

1. **Separate Build-Time vs. Runtime Operations**
   - In Python `kiwipiepy`, dictionary mutations (`kiwi.add_user_word(...)`, `kiwi.add_pre_analyzed_word(...)`, `kiwi.load_user_dictionary(...)`, `kiwi.add_rule(...)`, `kiwi.add_re_rule(...)`) are called directly on the `Kiwi` object before analysis.
   - In Rust `kiwi-rs`, dictionary mutations belong to **`KiwiBuilder`** (`KiwiLibrary::load_from_env_or_default()?.builder(BuilderConfig::default())?`), after which `.build()?` produces an immutable `Kiwi` analyzer (except `add_re_word` / `clear_re_words` and `GlobalConfig` / `default_prepared_typo` which can be updated on `Kiwi`).

2. **Map Options & Constants**
   - `Match.*` in Python maps to `KIWI_MATCH_*` bitflags in Rust (`KIWI_MATCH_ALL`, `KIWI_MATCH_ALL_WITH_NORMALIZING`, `KIWI_MATCH_JOIN_NOUN_PREFIX`, `KIWI_MATCH_EMOJI`, `KIWI_MATCH_OOV_CHR_MODEL`, etc.).
   - `Dialect.*` in Python maps to `KIWI_DIALECT_*` bitflags in Rust (`KIWI_DIALECT_STANDARD`, `KIWI_DIALECT_ALL`, etc.).
   - `TypoTransformer` / `basic_typos` presets map to `KiwiLibrary::default_typo_set(KIWI_TYPO_*)` and `KiwiTypo::prepare()` (`PreparedTypo`).

3. **Handle Unsupported Python-Only Helpers Cleanly**
   - `kiwipiepy.utils.Stopwords`: Implement as a lightweight `HashSet<(&str, &str)>` or `MorphemeSet` filter in Rust.
   - `Kiwi.template`: Not part of the C API; replace with `Kiwi::join` or `Kiwi::glue`.
   - Consult [references/kiwipiepy-mapping.md](./references/kiwipiepy-mapping.md) for side-by-side Python -> Rust translations.

## Side-by-Side Reference

Read [references/kiwipiepy-mapping.md](./references/kiwipiepy-mapping.md) for exact 1:1 translation patterns.
