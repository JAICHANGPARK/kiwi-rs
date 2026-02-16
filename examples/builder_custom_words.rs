use kiwi_rs::{AnalyzeOptions, BuilderConfig, KiwiLibrary, UserWord};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let library = KiwiLibrary::load_from_env_or_default()?;
    let mut builder = library.builder(BuilderConfig::default())?;

    builder.add_user_words([
        UserWord::new("키위러스트", "NNP", 0.0),
        UserWord::new("형태소파이프라인", "NNG", 0.0),
    ])?;

    builder.add_re_rule("NNP", "Rust", "러스트", 0.0)?;

    let kiwi = builder.build_with_default_options(AnalyzeOptions::default())?;
    let tokens = kiwi.tokenize("Kiwi Rust 기반 형태소파이프라인을 테스트합니다.")?;

    for token in tokens {
        println!("{}/{}", token.form, token.tag);
    }

    Ok(())
}
