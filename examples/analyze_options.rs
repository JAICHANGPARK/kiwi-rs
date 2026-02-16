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
            println!(
                "  {}/{} @{}+{}",
                token.form, token.tag, token.position, token.length
            );
        }
    }

    Ok(())
}
