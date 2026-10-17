# Python `kiwipiepy` to Rust `kiwi-rs` Mapping

## API Translation Table

| Python (`kiwipiepy`) | Rust (`kiwi-rs`) | Migration Notes |
|---|---|---|
| `kiwi = Kiwi()` | `let kiwi = Kiwi::init()?;` | Or `Kiwi::new()?` when `KIWI_LIBRARY_PATH` & `KIWI_MODEL_PATH` are set |
| `kiwi = Kiwi(num_workers=4, model_type='cong', typos='basic', typo_cost_threshold=2.5)` | `KiwiLibrary::builder(BuilderConfig::default().with_num_threads(4).with_model_type(KIWI_MODEL_CONG))?.with_typo_cost_threshold(2.5).build_with_typo_and_default_options(&typo, AnalyzeOptions::default())?` | Builder configures threads, model type, and default typo transformer |
| `kiwi.add_user_word(word, tag, score)` | `builder.add_user_word(word, tag, score)?` | Must be called on `KiwiBuilder` before `.build()?` |
| `kiwi.add_user_word(word, tag, score, orig_word)` | `builder.add_alias_word(word, tag, score, orig_word)?` | Use `add_alias_word` or `add_alias_word_with_def` |
| `kiwi.add_pre_analyzed_word(form, analyzed, score)` | `builder.add_pre_analyzed_word(form, &tokens, score)?` | Pass `&[PreAnalyzedToken]` |
| `kiwi.tokenize(text, match_options=...)` | `kiwi.tokenize_with_options(text, AnalyzeOptions::default().with_match_options(...))?` | `Token.position` & `Token.length` are UTF-8 `char` indices in both Python and Rust |
| `kiwi.tokenize(text, blocklist=..., pretokenized=...)` | `kiwi.tokenize_with_blocklist_and_pretokenized(text, opts, Some(&morphset), Some(&pretokenized))?` | Use `kiwi.new_morphset()?` and `kiwi.new_pretokenized()?` |
| `kiwi.analyze(text, top_n=5)` | `kiwi.analyze_top_n(text, 5)?` | Returns `Vec<AnalysisCandidate>` with `.tokens` and `.probability` |
| `kiwi.split_into_sents(text, return_tokens=True, return_sub_sents=True)` | `kiwi.split_into_sents_with_options(text, AnalyzeOptions::default(), true, true)?` | Returns `Vec<Sentence>` with `.text`, `.start`, `.end`, `.tokens`, `.subs` |
| `kiwi.space(text, reset_whitespace=False)` | `kiwi.space(text, false)?` or `kiwi.space_via_native(text, false)?` | `space_via_native` uses C API `kiwi_space` (Kiwi >= 0.24) |
| `kiwi.glue(chunks, insert_new_lines=..., return_space_insertions=True)` | `kiwi.glue_with_options(&chunks, insert_new_lines, true)?` or `kiwi.glue_with_options_via_native(...)` | Returns `(String, Option<Vec<bool>>)` |
| `kiwi.join(morphs, lm_search=True)` | `kiwi.join(&morphs, true)?` | For repeated calls, use `kiwi.prepare_joiner(true)?` |

## Replacing `kiwipiepy.utils.Stopwords` in Rust

Since `Stopwords` is a Python-side filter in `kiwipiepy`, filter tokens with a `HashSet` in Rust:

```rust
use std::collections::HashSet;
use kiwi_rs::Kiwi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;
    let stopwords: HashSet<(&str, &str)> = [
        ("하", "VV"),
        ("있", "VX"),
        ("것", "NNB"),
    ]
    .into_iter()
    .collect();

    let tokens: Vec<_> = kiwi
        .tokenize("할 수 있는 것부터 시작하자.")?
        .into_iter()
        .filter(|t| !stopwords.contains(&(t.form.as_str(), t.tag.as_str())))
        .collect();

    println!("Filtered tokens: {}", tokens.len());
    Ok(())
}
```
