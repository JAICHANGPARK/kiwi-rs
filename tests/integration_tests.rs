use kiwi_rs::*;

fn get_kiwi() -> Kiwi {
    // Shared builder/library logic
    KiwiLibrary::load_default()
        .expect("Failed to load Kiwi library")
        .builder(BuilderConfig::default())
        .expect("Failed to create builder")
        .build()
        .expect("Failed to build Kiwi")
}

#[test]
fn test_all_sequential() {
    // Run tests sequentially to avoid SIGSEGV in underlying C++ library
    // which seems to have thread-unsafe global state during initialization/teardown.
    run_tokenize();
    run_analyze();
    run_split_into_sents();
    run_space();
    run_join();
    run_add_user_word();
}

// ... (existing functions)

fn run_add_user_word() {
    println!("Starting run_add_user_word");
    // Need a fresh builder for this, but KiwiLibrary::load_default handles singleton
    let library = KiwiLibrary::load_default().expect("Failed to load Kiwi library");
    let mut builder = library
        .builder(BuilderConfig::default())
        .expect("Failed to create builder");

    // "데브시스터즈" is not in default dictionary (probably).
    // Add it as NNP (Proper Noun)
    builder
        .add_user_word("데브시스터즈", "NNP", 10.0)
        .expect("Failed to add user word");

    let kiwi = builder.build().expect("Failed to build Kiwi");
    let text = "데브시스터즈에 입사했다.";
    let res = kiwi.analyze(text).expect("Failed to analyze");

    let top = &res[0];
    let has_user_word = top
        .tokens
        .iter()
        .any(|t| t.form == "데브시스터즈" && t.tag == "NNP");
    assert!(
        has_user_word,
        "Should detect user added word '데브시스터즈'"
    );
}

fn run_tokenize() {
    println!("Starting run_tokenize");
    let kiwi = get_kiwi();
    let text = "안녕하세요? 키위입니다.";
    let tokens = kiwi.tokenize(text).expect("Failed to tokenize");
    println!("Tokenized. Count: {}", tokens.len());

    assert!(!tokens.is_empty());
    assert_eq!(tokens[0].position, 0);
}

fn run_analyze() {
    println!("Starting run_analyze");
    let kiwi = get_kiwi();
    let text = "형태소 분석기";
    let res = kiwi.analyze(text).expect("Failed to analyze");

    assert!(!res.is_empty());
    let top = &res[0];
    assert!(top.probability > -100.0); // Log probability, so can be negative
    assert!(!top.tokens.is_empty());

    // Check if "형태소" is detected as NNG
    let has_noun = top
        .tokens
        .iter()
        .any(|t| t.form == "형태소" && t.tag == "NNG");
    assert!(has_noun, "Should detect '형태소' as NNG");
}

fn run_split_into_sents() {
    println!("Starting run_split_into_sents");
    let kiwi = get_kiwi();
    let text = "안녕하세요. 반가워요.";
    // split_into_sents returns ranges (SentenceBoundary)
    let sents = kiwi
        .split_into_sents(text, KIWI_MATCH_ALL)
        .expect("Failed to split sentences");

    assert_eq!(sents.len(), 2);
    // SentenceBoundary has begin, end
    let first = &sents[0];
    let first_text = &text[first.begin..first.end];
    assert_eq!(first_text, "안녕하세요.");

    let second = &sents[1];
    let second_text = &text[second.begin..second.end];
    assert_eq!(second_text, "반가워요.");
}

fn run_space() {
    println!("Starting run_space");
    let kiwi = get_kiwi();
    let text = "띄어쓰기가없는문장입니다";
    let spaced = kiwi.space(text, true).expect("Failed to space");

    assert!(spaced.contains("띄어쓰기"));
    assert!(spaced.contains(" "));
}

fn run_join() {
    println!("Starting run_join");
    let kiwi = get_kiwi();
    let text = "겨울눈";
    let res = kiwi.analyze(text).expect("Failed to analyze");
    let tokens = &res[0].tokens;

    // Prepare morphs for join: Vec<(&str, &str)>
    let morphs: Vec<(&str, &str)> = tokens
        .iter()
        .map(|t| (t.form.as_str(), t.tag.as_str()))
        .collect();

    let joined = kiwi.join(&morphs, true).expect("Failed to join");
    assert_eq!(joined, "겨울눈");
}
