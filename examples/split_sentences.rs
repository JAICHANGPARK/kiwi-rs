use kiwi_rs::{AnalyzeOptions, Kiwi, KIWI_MATCH_ALL_WITH_NORMALIZING};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;

    let text = "여러 문장으로 구성된 텍스트네. 이걸 분리해줘.";
    let options = AnalyzeOptions::default().with_match_options(KIWI_MATCH_ALL_WITH_NORMALIZING);

    let sentences = kiwi.split_into_sents_with_options(text, options, true, true)?;

    for (index, sentence) in sentences.iter().enumerate() {
        println!(
            "sentence #{index}: [{}..{}] {}",
            sentence.start, sentence.end, sentence.text
        );

        if let Some(tokens) = &sentence.tokens {
            for token in tokens {
                println!(
                    "  tok {}/{} @{}+{}",
                    token.form, token.tag, token.position, token.length
                );
            }
        }

        if let Some(subs) = &sentence.subs {
            for (sub_index, sub) in subs.iter().enumerate() {
                println!(
                    "  sub #{sub_index}: [{}..{}] {}",
                    sub.start, sub.end, sub.text
                );
            }
        }
    }

    Ok(())
}
