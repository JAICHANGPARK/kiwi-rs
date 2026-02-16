# kiwipiepy parity status for kiwi-rs

Snapshot date: 2026-02-16

Baseline references:

- `ref/kiwipiepy-main/kiwipiepy/_wrap.py`
- `ref/kiwipiepy-main/src/KiwiPy.cpp`
- `ref/Kiwi-main/include/kiwi/capi.h`

## Legend

- `Equivalent`: same practical capability is available in `kiwi-rs` (API shape may differ in Rust).
- `Partial`: related capability exists, but behavior/surface is not 1:1.
- `Unavailable`: not provided in current `kiwi-rs` (usually outside C API boundary).

## Progress by layer

1. C API coverage: `complete` (`101/101` symbols in `capi.h` are loaded).
2. High-level Rust API coverage: `strong` for core NLP workflows.
3. Full `kiwipiepy` parity: `partial`.

## Core APIs

| kiwipiepy API | status | kiwi-rs API / note |
|---|---|---|
| `Kiwi.add_user_word` | Partial | `KiwiBuilder::add_user_word` (builder-time API) |
| `Kiwi.add_pre_analyzed_word` | Partial | `KiwiBuilder::add_pre_analyzed_word` (builder-time API) |
| `Kiwi.load_user_dictionary` | Partial | `KiwiBuilder::load_user_dictionary` (builder-time API) |
| `Kiwi.add_rule` | Equivalent | `KiwiBuilder::add_rule` |
| `Kiwi.add_re_rule` | Partial | `KiwiBuilder::add_re_rule`; not Python `re.sub`-compatible in all edge cases |
| `Kiwi.add_re_word` | Partial | `Kiwi::add_re_word(pattern, tag)`; callback/user_value variant is not exposed |
| `Kiwi.clear_re_words` | Equivalent | `Kiwi::clear_re_words` |
| `Kiwi.analyze` | Partial | `Kiwi::analyze*`, `Kiwi::analyze_many_with_options`, `Kiwi::analyze_many_via_native` |
| `Kiwi.tokenize` | Partial | `Kiwi::tokenize*`, blocklist/pretokenized variants, batch variants; no `Stopwords` helper type |
| `Kiwi.split_into_sents` | Partial | `split_into_sents*`, `split_into_sents_with_options*`; Python namedtuple surface is different |

## Runtime config and metadata

| kiwipiepy API | status | kiwi-rs API / note |
|---|---|---|
| `Kiwi.global_config` | Equivalent | `Kiwi::global_config`, `Kiwi::set_global_config` |
| deprecated properties (`cutoff_threshold`, etc.) | Equivalent | method-style Rust API (`cutoff_threshold`, `set_cutoff_threshold`, ...) |
| `Kiwi.num_workers` | Equivalent | `Kiwi::num_workers` |
| `Kiwi.model_type` | Equivalent | `Kiwi::model_type` |
| `Kiwi.typo_cost_threshold` | Equivalent | `Kiwi::typo_cost_threshold` |
| `Kiwi.list_senses` | Equivalent | `Kiwi::list_senses` |
| `Kiwi.list_all_scripts` | Equivalent | `Kiwi::list_all_scripts` |
| `Kiwi.morpheme` | Partial | `Kiwi::morpheme(morph_id)` is ID-based |
| `Kiwi.tag_to_string` | Equivalent | `Kiwi::tag_to_string` |

## Join / spacing convenience APIs

| kiwipiepy API | status | kiwi-rs API / note |
|---|---|---|
| `Kiwi.join` | Partial | `Kiwi::join`; `return_positions`-style output is unavailable |
| `Kiwi.space` | Partial | `Kiwi::space`, `Kiwi::space_many`; Rust-side heuristics |
| `Kiwi.glue` | Partial | `Kiwi::glue`, `Kiwi::glue_with_options` |
| `Kiwi.template` | Unavailable | Python template layer, no matching C API |

## CoNg semantic APIs

| kiwipiepy API | status | kiwi-rs API / note |
|---|---|---|
| `Kiwi.most_similar_morphemes` | Partial | `Kiwi::most_similar_morphemes(morph_id, top_n)` |
| `Kiwi.most_similar_contexts` | Partial | `Kiwi::most_similar_contexts(context_id, top_n)` |
| `Kiwi.predict_next_morpheme` | Partial | `Kiwi::predict_next_morpheme*` (ID-based) |
| `Kiwi.morpheme_similarity` | Equivalent | `Kiwi::morpheme_similarity` |
| `Kiwi.context_similarity` | Equivalent | `Kiwi::context_similarity` |
| context id conversion | Equivalent | `Kiwi::to_context_id`, `Kiwi::from_context_id` |

## Dataset / training helpers

| kiwipiepy API | status | kiwi-rs API / note |
|---|---|---|
| `Kiwi.extract_words` | Equivalent | `KiwiBuilder::extract_words` |
| `Kiwi.extract_add_words` | Equivalent | `KiwiBuilder::extract_add_words` |
| `Kiwi.convert_hsdata` | Unavailable | C++ API utility, no C API endpoint |
| `Kiwi.make_hsdataset` | Unavailable | C++ API utility, no C API endpoint |
| `Kiwi.evaluate`, `Kiwi.predict_next` | Unavailable | not implemented in kiwipiepy runtime either (`NotImplementedError`) |

## Beyond `Kiwi` class (package-level)

| kiwipiepy surface | status in kiwi-rs | note |
|---|---|---|
| `TypoTransformer`, `TypoDefinition` presets | Partial | `KiwiTypo` exists, but Python class shape/presets are different |
| `HSDataset` | Unavailable | dataset class is Python/C++ layer |
| `NgramExtractor` | Unavailable | no C API endpoint |
| `KNLangModel` | Unavailable | no C API endpoint |
| `Template` / `Kiwi.template` | Unavailable | Python template layer |
| `utils.Stopwords` | Unavailable | not yet provided as Rust helper type |
| `extract_substrings` | Unavailable | C++ layer utility |

## Why some gaps remain

`kiwi-rs` intentionally binds through the public C API only. Some `kiwipiepy` features are implemented through direct C++ bindings in the Python extension layer, so they cannot be recreated 1:1 without adding a C++ bridge in `kiwi-rs`.
