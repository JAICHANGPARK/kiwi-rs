# Copy-Paste `kiwi-rs` Code Recipes

Use these self-contained, verified recipes when generating code for users.

## 1. Quick Start: Tokenize & Extract Character Slice Safely

Remember that `token.position` and `token.length` are UTF-8 **character** indices (`char`), not byte offsets.

```rust
use kiwi_rs::Kiwi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;
    let text = "아버지가방에들어가신다.";
    let chars: Vec<char> = text.chars().collect();

    for token in kiwi.tokenize(text)? {
        let surface: String = chars[token.position..token.position + token.length]
            .iter()
            .collect();
        println!(
            "form={} tag={} char_span={}..{} surface={}",
            token.form,
            token.tag,
            token.position,
            token.position + token.length,
            surface
        );
    }
    Ok(())
}
```

## 2. Custom Dictionary & Sense/Dialect Definition (`KiwiBuilder`)

```rust
use kiwi_rs::{BuilderConfig, KiwiLibrary, KIWI_DIALECT_STANDARD};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let library = KiwiLibrary::load_from_env_or_default()?;
    let mut builder = library.builder(BuilderConfig::default())?;

    // Standard user word
    builder.add_user_word("키위러스트", "NNP", 5.0)?;

    // Kiwi >= 0.24: User word & alias word with explicit sense_id and dialect
    if library.supports_builder_word_with_def() {
        builder.add_user_word_with_def("안티그래비티", "NNP", 1, KIWI_DIALECT_STANDARD, 6.0)?;
        builder.add_alias_word_with_def(
            "안티그래비티AI",
            "NNP",
            1,
            KIWI_DIALECT_STANDARD,
            5.0,
            "안티그래비티",
        )?;
    }

    let kiwi = builder.build()?;
    let tokens = kiwi.tokenize("안티그래비티AI와 키위러스트로 형태소 분석을 수행합니다.")?;
    for token in tokens {
        println!("{}/{} (sense={})", token.form, token.tag, token.sense_id);
    }
    Ok(())
}
```

## 3. Typo Correction: Build-Time & Runtime `PreparedTypo`

```rust
use kiwi_rs::{
    AnalyzeOptions, BuilderConfig, KiwiLibrary, KIWI_TYPO_BASIC_TYPO_SET_WITH_CONTINUAL,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let library = KiwiLibrary::load_from_env_or_default()?;
    let typo = library.default_typo_set(KIWI_TYPO_BASIC_TYPO_SET_WITH_CONTINUAL)?;

    // Option A: Prepare a runtime typo transformer and pass it per call or as default
    if library.supports_typo_prepare() {
        let prepared = typo.prepare()?;
        let kiwi = library.builder(BuilderConfig::default())?.build()?;
        let options = AnalyzeOptions::default().with_typo_threshold(3.0);

        let tokens = kiwi.tokenize_with_typo("외않되", options, Some(&prepared))?;
        for token in tokens {
            println!("{}/{} (typo_cost={})", token.form, token.tag, token.typo_cost);
        }
    } else {
        // Option B: Build-time typo baking
        let builder = library
            .builder(BuilderConfig::default())?
            .with_typo_cost_threshold(3.0);
        let kiwi =
            builder.build_with_typo_and_default_options(&typo, AnalyzeOptions::default())?;
        for token in kiwi.tokenize("외않되")? {
            println!("{}/{}", token.form, token.tag);
        }
    }
    Ok(())
}
```

## 4. Blocking Morphemes (`MorphemeSet`) & Forcing Spans (`Pretokenized`)

```rust
use kiwi_rs::{AnalyzeOptions, Kiwi, PreAnalyzedToken};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;
    let text = "새로운 AI모델을 배포했다.";

    let mut blocklist = kiwi.new_morphset()?;
    blocklist.add("새롭", Some("VA"))?;

    // Force character span [4, 8) ("AI모델") as a single NNP token
    let mut pretokenized = kiwi.new_pretokenized()?;
    let span_id = pretokenized.add_span(4, 8)?;
    pretokenized.add_token_to_span(span_id, &PreAnalyzedToken::new("AI모델", "NNP"))?;

    let tokens = kiwi.tokenize_with_blocklist_and_pretokenized(
        text,
        AnalyzeOptions::default(),
        Some(&blocklist),
        Some(&pretokenized),
    )?;
    for token in tokens {
        println!("{}/{} ({}..{})", token.form, token.tag, token.position, token.end());
    }
    Ok(())
}
```

## 5. Sentence Splitting, Spacing (`space`), Gluing (`glue`), and Joining (`join`)

```rust
use kiwi_rs::{ AnalyzeOptions, Kiwi };

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;

    // 1. Sentence splitting with sub-sentences and tokens
    let sents = kiwi.split_into_sents_with_options(
        "안녕하세요. 오늘 날씨가 참 좋네요!",
        AnalyzeOptions::default(),
        true,
        true,
    )?;
    for sent in &sents {
        println!("Sentence [{}..{}]: {}", sent.start, sent.end, sent.text);
    }

    // 2. Automatic spacing correction (native C API if >= 0.24, otherwise Rust fallback)
    let spaced = if kiwi.supports_native_space_and_glue() {
        kiwi.space_via_native("띄어쓰기가없는문장입니다.", true)?
    } else {
        kiwi.space("띄어쓰기가없는문장입니다.", true)?
    };
    println!("Spaced: {spaced}");

    // 3. Gluing text chunks with automatic space insertion decisions
    let chunks = ["안녕하세요", "반갑습니다"];
    let (glued, insertions) = if kiwi.supports_native_space_and_glue() {
        kiwi.glue_with_options_via_native(&chunks, None, true)?
    } else {
        kiwi.glue_with_options(&chunks, None, true)?
    };
    println!("Glued: {glued} (insertions={insertions:?})");

    // 4. Morpheme joining (with allomorph selection)
    let joined = kiwi.join(&[("먹", "VV"), ("어", "EC"), ("보", "VX"), ("았", "EP"), ("다", "EF")], true)?;
    println!("Joined: {joined}");

    Ok(())
}
```

## 6. Global Config & OOV Parameter Tuning

```rust
use kiwi_rs::{AnalyzeOptions, Kiwi, KIWI_MATCH_ALL_WITH_NORMALIZING, KIWI_MATCH_OOV_CHR_MODEL};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut kiwi = Kiwi::init()?;

    // Tune OOV scoring and space tolerance directly on Kiwi
    kiwi.set_space_tolerance(2)?;
    kiwi.set_oov_chr_bias(-1.5)?;
    kiwi.set_oov_global_weight(50.0)?;

    let opts = AnalyzeOptions::default()
        .with_match_options(KIWI_MATCH_ALL_WITH_NORMALIZING | KIWI_MATCH_OOV_CHR_MODEL);
    let tokens = kiwi.tokenize_with_options("신조어와 이모지😊를 분석합니다.", opts)?;
    for token in tokens {
        println!("{}/{}", token.form, token.tag);
    }
    Ok(())
}
```
