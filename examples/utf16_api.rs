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
