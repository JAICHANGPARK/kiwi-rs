use kiwi_rs::{BuilderConfig, KiwiLibrary};

#[test]
fn test_add_rule_safety() {
    // This test attempts to reproduce a Use-After-Free in add_rule.
    // If the callback context is dropped but accessed later, this might segfault or print garbage.

    // Initialize library (might download if missing, but should be cached)
    // We assume the environment is set up or it can download.
    let library = KiwiLibrary::load_default().expect("Failed to load Kiwi library");
    let mut builder = library
        .builder(BuilderConfig::default())
        .expect("Failed to create builder");

    // We use a ref cell to track if callback is called.
    // Use an atomic to be safe in callbacks.
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();

    // add_rule adds a rule to transform text for a specific tag?
    // Or maybe it's for typo correction rules.
    // Documentation says: add_rule(tag, replacer, score).
    // Let's assume it runs during analysis.

    // We add a rule that should trigger for "NNG" (common noun).
    // "사람" is NNG.
    builder
        .add_rule(
            "NNG",
            move |text| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                // Returns the same text to avoid confusing the analyzer with weird replacements
                text.to_string()
            },
            1.0,
        )
        .unwrap();

    // Clobber the stack
    fn clobber() {
        let _data = [0xFFu8; 1024];
    }
    clobber();

    // Build the kiwi instance
    let kiwi = builder.build().expect("Failed to build Kiwi");

    // Analyze text that contains "NNG"
    // "사람" (Person) -> NNG
    let res = kiwi.analyze("사람").unwrap();
    println!("Analyze result: {:?}", res);

    if counter.load(Ordering::SeqCst) > 0 {
        println!("Callback was called!");
    } else {
        println!("Callback was NOT called.");
    }
}
