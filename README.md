# kiwi-rs

[한국어 README](README.ko.md) | [kiwipiepy parity (EN)](docs/kiwipiepy_parity.md) | [kiwipiepy parity (KO)](docs/kiwipiepy_parity.ko.md)

Rust bindings for Kiwi via the official C API (`include/kiwi/capi.h`).

## Current support status

As of February 16, 2026:

- C API symbol loading: complete (`101/101` symbols in `capi.h` are loaded)
- Core high-level usage: implemented (`init/new/from_config`, `analyze/tokenize/split/join`, `MorphemeSet`, `Pretokenized`, typo APIs, `SwTokenizer`, CoNg APIs)
- kiwipiepy full surface parity: partial (Python/C++-specific layers still missing)

## Installation

```toml
[dependencies]
kiwi-rs = "0.1"
```

## Runtime setup options

### Option 1: automatic bootstrap in code

`Kiwi::init()` tries local paths first, then downloads a matching release pair (library + model) into cache.

```rust
use kiwi_rs::Kiwi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;
    let tokens = kiwi.tokenize("아버지가방에들어가신다.")?;
    println!("{}", tokens.len());
    Ok(())
}
```

Environment variables used by bootstrap:

- `KIWI_RS_VERSION` (default: `latest`, e.g. `v0.22.2`)
- `KIWI_RS_CACHE_DIR` (default: OS cache directory)

External commands required by bootstrap:

- Common: `curl`, `tar`
- Windows zip extraction: `powershell` (`Expand-Archive`)

### Option 2: helper installer scripts

Linux/macOS:

```bash
cd kiwi-rs
make install-kiwi
```

Windows (PowerShell):

```powershell
cd kiwi-rs
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install_kiwi.ps1
```

Installer options:

- `KIWI_VERSION` / `-Version` (default: `latest`)
- `KIWI_PREFIX` / `-Prefix` (default: `$HOME/.local/kiwi` on Unix, `%LOCALAPPDATA%\\kiwi` on Windows)
- `KIWI_MODEL_VARIANT` / `-ModelVariant` (default: `base`)

## Manual path configuration

### Env-based (`Kiwi::new`)

- `KIWI_LIBRARY_PATH`: dynamic library path
- `KIWI_MODEL_PATH`: model directory path

### Config-based (`Kiwi::from_config`)

```rust
use kiwi_rs::{Kiwi, KiwiConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = KiwiConfig::default()
        .with_library_path("/path/to/libkiwi.dylib")
        .with_model_path("/path/to/models/cong/base")
        .add_user_word("러스트", "NNP", 0.0);

    let kiwi = Kiwi::from_config(config)?;
    let analyses = kiwi.analyze_top_n("형태소 분석 예시", 2)?;
    println!("{} candidates", analyses.len());
    Ok(())
}
```

## API overview

### Core

- Initialization: `Kiwi::init`, `Kiwi::new`, `Kiwi::from_config`, `Kiwi::init_direct`
- Analyze/tokenize: `analyze*`, `tokenize*`, `analyze_many*`, `tokenize_many*`
- Sentence split: `split_into_sents*`, `split_into_sents_with_options*`
- Join/spacing: `join*`, `space*`, `glue*`

### Advanced

- Builder: user words, alias words, pre-analyzed words, dictionary loading, regex rules, extract APIs
- Constraints: `MorphemeSet`, `Pretokenized`
- Typo: `KiwiTypo`, default typo sets, cost controls
- Subword: `SwTokenizer`
- CoNg: similarity/context/prediction/context-id conversion

### UTF-16 and optional API checks

- `Kiwi::supports_utf16_api`
- `Kiwi::supports_analyze_mw`
- `KiwiLibrary::supports_builder_init_stream`

## Examples

```bash
cd kiwi-rs
cargo run --example basic
cargo run --example analyze_options
cargo run --example builder_custom_words
cargo run --example typo_build
cargo run --example blocklist_and_pretokenized
cargo run --example split_sentences
cargo run --example utf16_api
cargo run --example native_batch
cargo run --example sw_tokenizer -- /path/to/tokenizer.json
cargo run --example morpheme_semantics
```

What each example is for:

| Example | What you learn | Key APIs | Notes |
|---|---|---|---|
| `basic` | End-to-end quick start (init + tokenize) | `Kiwi::init`, `Kiwi::tokenize` | Demonstrates cache bootstrap behavior when assets are missing. |
| `analyze_options` | How candidate analysis options change output | `AnalyzeOptions`, `Kiwi::analyze_with_options` | Shows `top_n`, `match_options`, and candidate probabilities. |
| `builder_custom_words` | Building a custom analyzer with user lexicon/rules | `KiwiLibrary::builder`, `add_user_words`, `add_re_rule` | Uses builder-time customization APIs. |
| `typo_build` | Enabling typo-aware analysis | `default_typo_set`, `build_with_typo_and_default_options` | Prints typo-related token metadata. |
| `blocklist_and_pretokenized` | Blocking specific morphemes and forcing token spans | `new_morphset`, `new_pretokenized`, `tokenize_with_blocklist_and_pretokenized` | Useful for domain constraints and deterministic spans. |
| `split_sentences` | Sentence segmentation with per-sentence token/sub-sentence structures | `split_into_sents_with_options` | Shows the `Sentence` return surface (`text/start/end/tokens/subs`). |
| `utf16_api` | UTF-16 analysis/tokenization/sentence split path | `supports_utf16_api`, `analyze_utf16*`, `tokenize_utf16*`, `split_into_sents_utf16*` | Includes runtime feature check for UTF-16 support. |
| `native_batch` | Native callback-based batch analysis route | `analyze_many_via_native`, `analyze_many_utf16_via_native` | Useful for higher-throughput multi-line processing. |
| `sw_tokenizer` | Subword tokenizer encode/decode flow | `open_sw_tokenizer`, `encode_with_offsets`, `decode` | Requires `tokenizer.json` path argument. |
| `morpheme_semantics` | Morpheme ID lookup and CoNg semantic utilities | `find_morphemes`, `morpheme`, `most_similar_morphemes`, `to_context_id` | Shows semantic APIs that operate on morpheme/context IDs. |

## kiwipiepy parity

Detailed matrix:

- English: `docs/kiwipiepy_parity.md`
- Korean: `docs/kiwipiepy_parity.ko.md`

In short, `kiwi-rs` already covers most C API-backed workflows, while Python/C++-specific layers (template/dataset/ngram utilities) remain out of scope for a pure C API binding.

## Common errors

- `failed to load library`
  - Library path is invalid or inaccessible. Set `KIWI_LIBRARY_PATH` explicitly or use `Kiwi::init()`.

- `Cannot open extract.mdl for WordDetector`
  - Model path is wrong. Point `KIWI_MODEL_PATH` (or config model path) to the directory containing model files.

- `reading type 'Ds' failed` (iostream-style errors)
  - Library/model version mismatch. Use matching assets from the same Kiwi release tag.

## Local quality checks

```bash
cd kiwi-rs
cargo fmt
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
cargo check --examples
cargo package --allow-dirty
```

## License

Same as Kiwi: LGPL v3.
