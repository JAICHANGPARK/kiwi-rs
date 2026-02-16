use kiwi_rs::{AnalyzeOptions, BuilderConfig, KiwiLibrary, KIWI_TYPO_BASIC_TYPO_SET};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let library = KiwiLibrary::load_from_env_or_default()?;
    let builder = library.builder(BuilderConfig::default())?;

    let mut typo = library.default_typo_set(KIWI_TYPO_BASIC_TYPO_SET)?.copy()?;
    typo.add(&["ㅐ"], &["ㅔ"], 1.0, 0)?;
    typo.scale_cost(1.0)?;

    let kiwi =
        builder.build_with_typo_and_default_options(Some(&typo), AnalyzeOptions::default())?;
    let tokens = kiwi.tokenize("오타가 섞인 문장을 분석해봅니다.")?;

    for token in tokens {
        println!(
            "{}/{} typo_cost={} typo_form_id={}",
            token.form, token.tag, token.typo_cost, token.typo_form_id
        );
    }

    Ok(())
}
