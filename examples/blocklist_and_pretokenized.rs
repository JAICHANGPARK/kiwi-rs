use kiwi_rs::{AnalyzeOptions, Kiwi};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;

    let mut blocklist = kiwi.new_morphset()?;
    // Example: block a specific morpheme candidate.
    let _ = blocklist.add("하", Some("VV"))?;

    let mut pretokenized = kiwi.new_pretokenized()?;
    let text = "AI엔지니어링팀에서테스트중";

    // Force "AI" to be treated as a single NNP token at [0, 2).
    let span_id = pretokenized.add_span(0, 2)?;
    pretokenized.add_token_to_span(span_id, "AI", "NNP", 0, 2)?;

    let tokens = kiwi.tokenize_with_blocklist_and_pretokenized(
        text,
        AnalyzeOptions::default(),
        Some(&blocklist),
        Some(&pretokenized),
    )?;

    for token in tokens {
        println!(
            "{}/{} @{}+{}",
            token.form, token.tag, token.position, token.length
        );
    }

    Ok(())
}
