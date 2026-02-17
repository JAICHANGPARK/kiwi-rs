# Implementing kiwi-rs for Korean Morphological Analysis in Rust
## A Practical kiwi-rs Guide: Implementation Patterns, Code Walkthroughs, Benchmarks, and Production Strategy

In Korean NLP systems, a morphological analyzer is often the single component that determines both quality and latency. In the Rust ecosystem, production-ready choices for Korean morphology bindings were still limited, so we implemented `kiwi-rs`. Most teams obsess over linguistic quality, but in production they get hurt by less visible factors: startup latency, batch throughput, cache hit/miss behavior, and call-boundary overhead.

This article uses `kiwi-rs` as the concrete foundation and covers the full path end-to-end:

- How to integrate Kiwi C API safely in Rust services
- Which coding patterns are practical in real products, and why
- How to design reproducible benchmark experiments
- How to interpret measured numbers from February 17, 2026
- What to monitor and tune in production

This write-up is intended for:

- Engineers running Rust-based search/retrieval/recommendation/agent pipelines
- Teams migrating Python NLP PoCs to Rust services
- DevRel and technical leads who need to explain performance numbers with rigor

---

## 1. Why `kiwi-rs`

`kiwi-rs` is a Rust binding around the official Kiwi C API (`include/kiwi/capi.h`). It is not just an FFI toy. It already exposes most high-level workflows a production system needs.

- Initialization: `Kiwi::init`, `Kiwi::new`, `Kiwi::from_config`
- Analysis/tokenization: `analyze*`, `tokenize*`
- Sentence split/join/spacing: `split_into_sents*`, `join*`, `space*`, `glue*`
- Batch paths: `analyze_many*`, `tokenize_many*`
- Constraints and deterministic spans: `MorphemeSet`, `Pretokenized`
- Advanced APIs: typo features, UTF-16 APIs, subword tokenizer, CoNg semantics

An important clarification: Kiwi itself was not missing. Multiple language/platform entry points already existed. The real gap was a production-oriented Rust binding.

- C API: `include/kiwi/capi.h`
- Prebuilt binaries: [GitHub Releases](https://github.com/bab2min/Kiwi/releases) provides Windows/Linux/macOS/Android libraries and model assets
- C# wrapper: [kiwi-gui](https://github.com/bab2min/kiwi-gui) (used by the official GUI), plus the community wrapper `NetKiwi`
- Python3 wrapper: [kiwipiepy](https://github.com/bab2min/kiwipiepy)
- Java wrapper: `KiwiJava` for Java 1.8+ (`bindings/java`)
- Android library: NDK-based AAR (`kiwi-android-VERSION.aar`) from Releases, minimum Android API 21+ / ARM64, usage documented under `bindings/java`
- R wrapper: `elbird` (community contribution)
- Go wrapper: `kiwigo` (codingpot community)
- WebAssembly (JavaScript/TypeScript): `bindings/wasm` (contributed by RicBent)
- GUI application: Windows GUI available at [kiwi-gui](https://github.com/bab2min/kiwi-gui)

Why teams like this in practice:

1. It integrates cleanly into Rust services.
2. It gives room for Rust-side reuse/caching strategies.
3. Since the API boundary is explicit, environment validation (library/model compatibility) becomes operationally manageable.

---

## 2. Setup First Principle: Choose One Initialization Path

Before anything else, lock down a single initialization strategy. Mixing init modes in different environments destroys reproducibility.

### 2.1 `Kiwi::init()` path

Fastest way to get started:

- It tries local assets first.
- If missing, it bootstraps library/model assets into cache.

```rust
use kiwi_rs::Kiwi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // If local assets are missing, init() bootstraps from cache/download.
    let kiwi = Kiwi::init()?;

    let text = "아버지가방에들어가신다.";
    let tokens = kiwi.tokenize(text)?;

    for token in tokens {
        println!(
            "{} / {} (pos={}, len={}, word={}, sent={}, score={}, typo_cost={})",
            token.form,
            token.tag,
            token.position,
            token.length,
            token.word_position,
            token.sent_position,
            token.score,
            token.typo_cost
        );
    }

    Ok(())
}
```

Key notes:

- Treat `position`/`length` as UTF-8 character-index semantics, not byte offsets.
- Because bootstrap is part of init, track `init_ms` separately from steady-state throughput.

### 2.2 `Kiwi::new` / `Kiwi::from_config` path

In production, teams usually pin exact asset paths.

- `KIWI_LIBRARY_PATH`: dynamic library path
- `KIWI_MODEL_PATH`: model directory path

Or configure through code:

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

A practical default policy:

- Dev/demo: `Kiwi::init()`
- Production: `Kiwi::from_config` or `Kiwi::new` with pinned asset versions

---

## 3. Pattern 1: Handle N-best Analysis Candidates Correctly

Many teams push `analyze_top_n` results directly into ranking/UI. Common mistakes happen around candidate probability handling and token-boundary interpretation.

```rust
use kiwi_rs::{AnalyzeOptions, Kiwi, KIWI_MATCH_ALL_WITH_NORMALIZING};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;

    let options = AnalyzeOptions::default()
        .with_top_n(3)
        .with_match_options(KIWI_MATCH_ALL_WITH_NORMALIZING)
        .with_open_ending(false);

    let text = "형태소 분석 결과 후보를 여러 개 보고 싶습니다.";
    let candidates = kiwi.analyze_with_options(text, options)?;

    for (index, candidate) in candidates.iter().enumerate() {
        println!("candidate #{index} prob={}", candidate.probability);
        for token in &candidate.tokens {
            println!("  {}/{} @{}+{}", token.form, token.tag, token.position, token.length);
        }
    }

    Ok(())
}
```

Why this matters:

- `with_top_n(3)`: single 1-best outputs can be brittle on ambiguous strings.
- `with_match_options(...)`: normalization policy directly changes operational behavior.
- `with_open_ending(false)`: useful when you need tighter candidate space control.

Practical tip:

- Always log candidate probabilities in debugging and offline evaluation.
- If you have downstream models, passing N-best features is usually better than forcing early collapse.

---

## 4. Pattern 2: Domain Constraints with Blocklist + Forced Span Tokenization

In enterprise search, legal/medical processing, or product-name recognition, you often need deterministic token behavior. `MorphemeSet` + `Pretokenized` is a strong combination.

```rust
use kiwi_rs::{AnalyzeOptions, Kiwi};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;

    let mut blocklist = kiwi.new_morphset()?;
    // Block a specific morpheme candidate.
    let _ = blocklist.add("하", Some("VV"))?;

    let mut pretokenized = kiwi.new_pretokenized()?;
    let text = "AI엔지니어링팀에서테스트중";

    // Force [0, 2) to be a single token: AI/NNP.
    let span_id = pretokenized.add_span(0, 2)?;
    pretokenized.add_token_to_span(span_id, "AI", "NNP", 0, 2)?;

    let tokens = kiwi.tokenize_with_blocklist_and_pretokenized(
        text,
        AnalyzeOptions::default(),
        Some(&blocklist),
        Some(&pretokenized),
    )?;

    for token in tokens {
        println!("{}/{} @{}+{}", token.form, token.tag, token.position, token.length);
    }

    Ok(())
}
```

Interpretation:

- `MorphemeSet` removes unacceptable analyses first.
- `Pretokenized` pins must-preserve spans.
- Together, they reduce search space and improve domain consistency.

This is especially useful when:

- Product/version strings are being split incorrectly.
- Brand/domain entities are fragmented and hurt retrieval quality.
- Compliance requires preserving specific spans.

---

## 5. Pattern 3: Batch Processing + UTF-16 Runtime Safety

For high-volume workloads, batch paths are often more stable than per-line loops. For UTF-16 integrations, runtime capability checks are mandatory.

### 5.1 Native batch analysis

```rust
use kiwi_rs::{AnalyzeOptions, Kiwi};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;

    let lines = vec![
        "첫 번째 문장입니다.",
        "두 번째 문장도 형태소 분석합니다.",
        "세 번째 입력입니다.",
    ];

    let options = AnalyzeOptions::default().with_top_n(2);

    let batched = kiwi.analyze_many_via_native(&lines, options)?;
    for (index, candidates) in batched.iter().enumerate() {
        println!("line #{index}: {} candidates", candidates.len());
    }

    if kiwi.supports_analyze_mw() {
        let utf16_lines: Vec<Vec<u16>> = lines
            .iter()
            .map(|line| line.encode_utf16().collect())
            .collect();
        let batched_w = kiwi.analyze_many_utf16_via_native(utf16_lines, options)?;
        println!("utf16 batch count: {}", batched_w.len());
    }

    Ok(())
}
```

Important:

- Do not assume UTF-16 native batch support without `supports_analyze_mw()` checks.
- Batch metrics are sensitive to input diversity, batch size, and cache conditions.

### 5.2 UTF-16 capability checks

```rust
use kiwi_rs::{AnalyzeOptions, Kiwi};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;

    if !kiwi.supports_utf16_api() {
        println!("Loaded Kiwi library does not support UTF-16 API on this runtime.");
        return Ok(());
    }

    let text = "UTF16 경로도 동일하게 분석할 수 있습니다.";
    let utf16: Vec<u16> = text.encode_utf16().collect();

    let tokens = kiwi.tokenize_utf16_with_options(&utf16, AnalyzeOptions::default())?;
    println!("token count: {}", tokens.len());

    let candidates =
        kiwi.analyze_utf16_with_options(&utf16, AnalyzeOptions::default().with_top_n(2))?;
    println!("candidate count: {}", candidates.len());

    let sentences =
        kiwi.split_into_sents_utf16_with_options(&utf16, AnalyzeOptions::default(), true, true)?;
    println!("sentence count: {}", sentences.len());

    Ok(())
}
```

---

## 6. Benchmark Methodology: Experiment Design Before Numbers

The biggest benchmark mistakes in NLP systems are methodological:

- Mixing startup and steady-state into one number
- Measuring only repeated identical input (cache-inflated)
- Measuring only varied input and missing warm-cache reality

This repo explicitly separates two scenarios:

1. `input_mode=repeated`: warm-cache-like repeated processing
2. `input_mode=varied`: near no-cache rotating inputs

Reproducible commands:

```bash
cd kiwi-rs
mkdir -p tmp

# repeated
.venv-bench/bin/python scripts/compare_feature_bench.py \
  --text "아버지가방에들어가신다." \
  --warmup 100 --iters 5000 \
  --batch-size 256 --batch-iters 500 \
  --input-mode repeated --variant-pool 4096 \
  --repeats 1 \
  --md-out tmp/feature_bench_repeated.md \
  --json-out tmp/feature_bench_repeated.json

# varied
.venv-bench/bin/python scripts/compare_feature_bench.py \
  --text "아버지가방에들어가신다." \
  --warmup 100 --iters 5000 \
  --batch-size 256 --batch-iters 500 \
  --input-mode varied --variant-pool 8192 \
  --repeats 1 \
  --md-out tmp/feature_bench_varied.md \
  --json-out tmp/feature_bench_varied.json
```

---

## 7. Benchmark Results (Local Measurement Snapshot)

Source snapshots:

- `tmp/feature_bench_repeated.md` (timestamp: `<LOCAL_TIMESTAMP_REDACTED>`)
- `tmp/feature_bench_varied.md` (timestamp: `<LOCAL_TIMESTAMP_REDACTED>`)

Common environment summary:

- OS: `<HOST_OS_REDACTED>`
- rustc: 1.93.1 (2026-02-11)
- cargo: 1.93.1
- Python: 3.14.3
- kiwipiepy: 0.22.2
- text: `아버지가방에들어가신다.`
- warmup: 100, iters: 5000, batch_size: 256, batch_iters: 500
- Git HEAD: `<REPO_COMMIT_SHA>`

### 7.1 Repeated input (cache-favorable)

| Feature | kiwi-rs calls/sec | kiwipiepy calls/sec | Ratio (kiwi-rs / kiwipiepy) |
|---|---:|---:|---:|
| tokenize | 1,185,489.51 | 7,792.55 | 152.13x |
| analyze_top1 | 1,199,112.66 | 7,612.25 | 157.52x |
| split_into_sents | 28,908,752.41 | 3,802.38 | 7602.80x |
| split_into_sents_with_tokens | 250,558.01 | 4,872.41 | 51.42x |
| space | 357,757.20 | 4,768.69 | 75.02x |
| join | 2,402,355.08 | 675,759.32 | 3.56x |
| glue | 6,221,490.02 | 7,613.64 | 817.15x |
| analyze_many_native | 166.11 | 165.71 | 1.00x |
| tokenize_many_batch | 3,134.67 | 184.16 | 17.02x |
| space_many_batch | 161.79 | 160.39 | 1.01x |

Startup (`init_ms`, lower is better):

- `Kiwi::init()` / `Kiwi()`: kiwi-rs 1417.905 ms vs kiwipiepy 680.748 ms

### 7.2 Varied input (near no-cache)

| Feature | kiwi-rs calls/sec | kiwipiepy calls/sec | Ratio |
|---|---:|---:|---:|
| tokenize | 6,956.95 | 7,393.81 | 0.94x |
| analyze_top1 | 7,319.22 | 7,212.44 | 1.01x |
| split_into_sents | 5,104.73 | 4,399.49 | 1.16x |
| split_into_sents_with_tokens | 4,372.13 | 4,282.95 | 1.02x |
| space | 4,944.59 | 4,497.21 | 1.10x |
| glue | 5,692.86 | 4,965.80 | 1.15x |
| join | 2,927,258.22 | 669,983.08 | 4.37x |
| analyze_many_native | 158.62 | 192.74 | 0.82x |
| tokenize_many_batch | 151.12 | 190.38 | 0.79x |
| space_many_batch | 150.76 | 159.43 | 0.95x |

Startup (`init_ms`):

- kiwi-rs 1368.136 ms vs kiwipiepy 751.911 ms

---

## 8. Interpreting the Numbers in Operational Terms

### 8.1 Why repeated input can explode ratios

Repeated input reuses internal caches heavily. So these categories naturally spike:

- `tokenize`, `analyze_top1`
- `split_into_sents`, `split_into_sents_with_tokens`
- `glue`, `join`

These values should be interpreted as upper-bound behavior under cache-friendly conditions, not universal real-world speedup.

### 8.2 Why varied input compresses gaps

When each input differs, miss-path costs become visible. For user-generated or long-tail traffic, varied measurements are usually closer to production reality.

Representative varied ratios:

- `tokenize`: 0.94x
- `analyze_top1`: 1.01x
- `split_into_sents`: 1.16x
- `space`: 1.10x
- `glue`: 1.15x

So for many core single-call operations, ratios converge near parity with small feature-level differences.

### 8.3 Why `join` remains strong

`join` remains strong even in varied mode (4.37x), likely due to internal join-path optimizations and joiner reuse characteristics.

Rust-only support metrics reinforce this:

- `joiner_reuse`: 3,213,367.61 calls/sec (varied)
- `join_prepared`: 324,130.72 calls/sec (varied)

Operational takeaway:

- If your service repeatedly joins similar morpheme sequences, prepare/reuse strategies are worth it.

### 8.4 Batch-native caution

`analyze_many_native` and `tokenize_many_batch` can underperform in varied scenarios:

- `analyze_many_native`: 0.82x
- `tokenize_many_batch`: 0.79x

This region is heavily shaped by:

- `batch_size`
- input diversity (`variant_pool`)
- memory and boundary overhead
- runtime scheduling interactions

Never ship batch assumptions without replaying production-like traffic distributions.

---

## 9. Practical Performance Tuning Checklist

### 9.1 Separate measurement layers

- Startup: track `init_ms`
- Steady-state: track `avg_ms`, `calls_per_sec`
- Cache behavior: always report both repeated and varied modes

### 9.2 API selection strategy

- Single-call heavy APIs: `tokenize`, `analyze_with_options`
- Fixed-size high-volume flows: `*_many_*` + batch-size tuning
- Domain constraints: `MorphemeSet` + `Pretokenized`
- UTF-16 integrations: guard with `supports_utf16_api()` and `supports_analyze_mw()`

### 9.3 Data and cache policy

- Decide normalization policy explicitly if you want predictable cache behavior.
- High query reuse products (autocomplete, repeated intents) benefit more from warm-cache conditions.
- High-diversity user text should be capacity-planned from varied-mode numbers.

### 9.4 Failure prevention

Most recurring errors are environment mismatch issues:

- `failed to load library`
  - Fix library path (`KIWI_LIBRARY_PATH`) or use `Kiwi::init()` bootstrap
- `Cannot open extract.mdl for WordDetector`
  - Fix model directory (`KIWI_MODEL_PATH`)
- `reading type 'Ds' failed`
  - Library/model version mismatch

Production rule:

- Pin library and model assets to the same Kiwi release tag.
- Add a startup/tokenize smoke test in CI.

---

## 10. Release Gate Commands

```bash
cd kiwi-rs
cargo fmt
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
cargo check --examples
cargo package --allow-dirty
```

Recommended additions:

- Archive benchmark JSON snapshots for regression tracking.
- Compare at least one repeated and one varied sample in each PR cycle.
- Include exact command lines and measurement dates in docs.

---

## 11. Migration-Oriented Conclusion

Performance interpretation for `kiwi-rs` should not end at "X is Y times faster." A robust conclusion needs workload shape context.

- Repeated inputs amplify cache advantages.
- Varied inputs compress many gaps and may invert some batch paths.
- Startup (`init_ms`) must be tracked separately from steady-state.

A practical decision framework:

1. Decompose your traffic into repeated-like and varied-like workloads.
2. Benchmark with identical commands, text, and batch configs on a regular schedule.
3. Make feature-level decisions separately (single-call, batch, join-heavy, constrained spans).

When you do this consistently, your morphological analyzer becomes a measurable SLO component, not a black box.

---

## Appendix A. Quick Commands

```bash
# Core examples
cargo run --example basic
cargo run --example analyze_options
cargo run --example blocklist_and_pretokenized
cargo run --example native_batch
cargo run --example utf16_api

# Rust-side benchmarks
cargo run --release --example bench_tokenize -- --iters 1000 --warmup 100
cargo run --release --example bench_features -- --iters 5000 --warmup 100 --batch-size 256 --batch-iters 500

# Rust vs Python side-by-side
python3 scripts/bench_kiwipiepy.py --text "아버지가방에들어가신다." --warmup 100 --iters 5000
```

---

## Appendix B. Reference Files

- `README.md`
- `README.ko.md`
- `examples/basic.rs`
- `examples/analyze_options.rs`
- `examples/blocklist_and_pretokenized.rs`
- `examples/native_batch.rs`
- `examples/utf16_api.rs`
- `examples/bench_features.rs`
- `scripts/compare_feature_bench.py`
- `tmp/feature_bench_repeated.md`
- `tmp/feature_bench_varied.md`
