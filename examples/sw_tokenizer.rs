use kiwi_rs::Kiwi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;

    let tokenizer_path = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --example sw_tokenizer -- <tokenizer.json>")?;

    let tokenizer = kiwi.open_sw_tokenizer(tokenizer_path)?;

    let text = "형태소 분석기";
    let (token_ids, offsets) = tokenizer.encode_with_offsets(text)?;
    let decoded = tokenizer.decode(&token_ids)?;

    println!("ids={token_ids:?}");
    println!("offsets={offsets:?}");
    println!("decoded={decoded}");

    Ok(())
}
