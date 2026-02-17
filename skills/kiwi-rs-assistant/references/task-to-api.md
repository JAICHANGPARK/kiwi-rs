# Task to API Map

Use this map before writing code. Prefer APIs already demonstrated in this repository.

## Quick Routing

| User intent | Primary API | Example | Validation command |
|---|---|---|---|
| 빠른 시작으로 토큰화하기 | `Kiwi::init`, `Kiwi::tokenize` | `examples/basic.rs` | `cargo run --example basic` |
| 후보 분석(top-n)과 옵션 조절 | `AnalyzeOptions`, `analyze_with_options` | `examples/analyze_options.rs` | `cargo run --example analyze_options` |
| 사용자 단어/정규식 규칙 반영 | `KiwiLibrary::builder`, `add_user_words`, `add_re_rule`, `build_with_default_options` | `examples/builder_custom_words.rs` | `cargo run --example builder_custom_words` |
| 오타 교정 기반 분석 | `default_typo_set`, `build_with_typo_and_default_options` | `examples/typo_build.rs` | `cargo run --example typo_build` |
| 특정 형태소 차단/구간 고정 | `new_morphset`, `new_pretokenized`, `tokenize_with_blocklist_and_pretokenized` | `examples/blocklist_and_pretokenized.rs` | `cargo run --example blocklist_and_pretokenized` |
| 문장 분리 + 토큰/하위문장 포함 | `split_into_sents_with_options` | `examples/split_sentences.rs` | `cargo run --example split_sentences` |
| UTF-16 경로 처리 | `supports_utf16_api`, `tokenize_utf16_with_options`, `analyze_utf16_with_options` | `examples/utf16_api.rs` | `cargo run --example utf16_api` |
| 네이티브 배치 분석 | `analyze_many_via_native`, `analyze_many_utf16_via_native` | `examples/native_batch.rs` | `cargo run --example native_batch` |
| 서브워드 토크나이저 | `open_sw_tokenizer`, `encode_with_offsets`, `decode` | `examples/sw_tokenizer.rs` | `cargo run --example sw_tokenizer -- /path/to/tokenizer.json` |
| 형태소 의미/유사도(CoNg) | `find_morphemes`, `morpheme`, `most_similar_morphemes`, `to_context_id` | `examples/morpheme_semantics.rs` | `cargo run --example morpheme_semantics` |

## Initialization Decision Rules

- Prefer `Kiwi::init()` for normal usage and quickest onboarding.
- Prefer `Kiwi::new()` when runtime paths are already injected by environment variables.
- Prefer `Kiwi::from_config(...)` when deployment requires explicit model/library paths in code.
- Prefer `KiwiLibrary` builder flow when the request includes lexicon/rule/typo customization.

## Accuracy Rules For Code Generation

- Keep snippets compile-ready with `Result<(), Box<dyn std::error::Error>>`.
- Use UTF-8 APIs by default.
- Use UTF-16 APIs only after a `supports_utf16_api()` check.
- If user asks for `kiwipiepy` features, verify parity document and avoid unsupported claims.
