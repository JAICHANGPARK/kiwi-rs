use kiwi_rs::Kiwi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;

    let morph_ids = kiwi.find_morphemes("사과", None, -1, 8)?;
    println!("found morph ids: {morph_ids:?}");

    if let Some(&first_id) = morph_ids.first() {
        let morph = kiwi.morpheme(first_id)?;
        println!(
            "first morph: id={} form={} tag={} sense={} dialect={}",
            morph.morph_id, morph.form, morph.tag, morph.sense_id, morph.dialect
        );

        let similar = kiwi.most_similar_morphemes(first_id, 5)?;
        println!("similar morph ids: {similar:?}");

        let context_id = kiwi.to_context_id(&[first_id])?;
        let restored = kiwi.from_context_id(context_id, 8)?;
        println!("context_id={context_id}, restored={restored:?}");
    }

    Ok(())
}
