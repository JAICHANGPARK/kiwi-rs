use kiwi_rs::Kiwi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // If local assets are missing, init() tries to download them to cache.
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
