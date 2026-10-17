use kiwi_rs::*;

fn get_kiwi() -> Kiwi {
    // Shared builder/library logic
    KiwiLibrary::load_from_env_or_default()
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
    run_global_config();
    run_config_shortcuts();
    run_typo_prepare();
    run_builder_word_with_def();
    run_native_space_and_glue();
}

fn run_global_config() {
    println!("Starting run_global_config");
    let mut kiwi = get_kiwi();
    let config = kiwi.global_config().expect("Failed to get global config");
    println!("Global config: {:?}", config);
    kiwi.set_global_config(config)
        .expect("Failed to set global config");
}

fn run_config_shortcuts() {
    println!("Starting run_config_shortcuts");
    let mut kiwi = get_kiwi();
    let cutoff = kiwi.cutoff_threshold().expect("Failed to get cutoff");
    kiwi.set_cutoff_threshold(cutoff)
        .expect("Failed to set cutoff");

    let allomorph = kiwi
        .integrate_allomorph()
        .expect("Failed to get integrate_allomorph");
    kiwi.set_integrate_allomorph(allomorph)
        .expect("Failed to set integrate_allomorph");

    let penalty = kiwi.space_penalty().expect("Failed to get space_penalty");
    kiwi.set_space_penalty(penalty)
        .expect("Failed to set space_penalty");

    let tolerance = kiwi
        .space_tolerance()
        .expect("Failed to get space_tolerance");
    kiwi.set_space_tolerance(tolerance)
        .expect("Failed to set space_tolerance");

    let max_unk = kiwi
        .max_unk_form_size()
        .expect("Failed to get max_unk_form_size");
    kiwi.set_max_unk_form_size(max_unk)
        .expect("Failed to set max_unk_form_size");

    let typo_weight = kiwi
        .typo_cost_weight()
        .expect("Failed to get typo_cost_weight");
    kiwi.set_typo_cost_weight(typo_weight)
        .expect("Failed to set typo_cost_weight");

    let max_unk_j = kiwi
        .max_unk_form_size_followed_by_j_class()
        .expect("Failed to get max_unk_form_size_followed_by_j_class");
    kiwi.set_max_unk_form_size_followed_by_j_class(max_unk_j)
        .expect("Failed to set max_unk_form_size_followed_by_j_class");

    let oov_rule_scale = kiwi.oov_rule_scale().expect("Failed to get oov_rule_scale");
    kiwi.set_oov_rule_scale(oov_rule_scale)
        .expect("Failed to set oov_rule_scale");

    let oov_rule_bias = kiwi.oov_rule_bias().expect("Failed to get oov_rule_bias");
    kiwi.set_oov_rule_bias(oov_rule_bias)
        .expect("Failed to set oov_rule_bias");

    let oov_chr_bias = kiwi.oov_chr_bias().expect("Failed to get oov_chr_bias");
    kiwi.set_oov_chr_bias(oov_chr_bias)
        .expect("Failed to set oov_chr_bias");

    let oov_global_weight = kiwi
        .oov_global_weight()
        .expect("Failed to get oov_global_weight");
    kiwi.set_oov_global_weight(oov_global_weight)
        .expect("Failed to set oov_global_weight");

    let oov_local_weight = kiwi
        .oov_local_weight()
        .expect("Failed to get oov_local_weight");
    kiwi.set_oov_local_weight(oov_local_weight)
        .expect("Failed to set oov_local_weight");

    let oov_global_min_freq = kiwi
        .oov_global_min_freq()
        .expect("Failed to get oov_global_min_freq");
    kiwi.set_oov_global_min_freq(oov_global_min_freq)
        .expect("Failed to set oov_global_min_freq");
}

fn run_typo_prepare() {
    println!("Starting run_typo_prepare");
    let library = KiwiLibrary::load_from_env_or_default().expect("Failed to load Kiwi library");
    let typo = library.basic_typo().expect("Failed to create typo set");
    if !library.supports_typo_prepare() {
        println!("Kiwi library does not export kiwi_typo_prepare (pre-0.23), skipped gracefully.");
        return;
    }

    let prepared = typo.prepare().expect("Failed to prepare typo");
    let mut kiwi = get_kiwi();
    let options = AnalyzeOptions::default().with_typo_threshold(2.5);
    let candidates = kiwi
        .analyze_with_typo("외않되", options, Some(&prepared))
        .expect("Failed to analyze_with_typo");
    assert!(!candidates.is_empty());

    let tokens = kiwi
        .tokenize_with_typo("외않되", options, Some(&prepared))
        .expect("Failed to tokenize_with_typo");
    assert!(!tokens.is_empty());

    assert!(kiwi.default_prepared_typo().is_none());
    kiwi.set_default_prepared_typo(Some(prepared))
        .expect("Failed to set_default_prepared_typo");
    assert!(kiwi.default_prepared_typo().is_some());
    let tokens_default = kiwi
        .tokenize("외않되")
        .expect("Failed to tokenize with default typo");
    assert!(!tokens_default.is_empty());
    kiwi.set_default_prepared_typo(None)
        .expect("Failed to clear default_prepared_typo");
    assert!(kiwi.default_prepared_typo().is_none());
}

fn run_builder_word_with_def() {
    println!("Starting run_builder_word_with_def");
    let library = KiwiLibrary::load_from_env_or_default().expect("Failed to load Kiwi library");
    if !library.supports_builder_word_with_def() {
        println!(
            "Kiwi library does not export kiwi_builder_add_word_with_def (pre-0.24), skipped gracefully."
        );
        return;
    }

    let mut builder = library
        .builder(BuilderConfig::default())
        .expect("Failed to create builder");
    builder
        .add_user_word_with_def(
            "러스트바인딩테스트단어",
            "NNP",
            1,
            KIWI_DIALECT_STANDARD,
            5.0,
        )
        .expect("Failed to add_user_word_with_def");
    builder
        .add_alias_word_with_def(
            "러바테단",
            "NNP",
            1,
            KIWI_DIALECT_STANDARD,
            4.0,
            "러스트바인딩테스트단어",
        )
        .expect("Failed to add_alias_word_with_def");
    let kiwi = builder.build().expect("Failed to build Kiwi");
    let tokens = kiwi
        .tokenize("러스트바인딩테스트단어에 대해 알아보자.")
        .expect("Failed to tokenize");
    assert!(tokens
        .iter()
        .any(|t| t.form == "러스트바인딩테스트단어" && t.tag == "NNP"));
}

fn run_native_space_and_glue() {
    println!("Starting run_native_space_and_glue");
    let kiwi = get_kiwi();
    if !kiwi.supports_native_space_and_glue() {
        println!(
            "Kiwi library does not export kiwi_space / kiwi_glue (pre-0.24), skipped gracefully."
        );
        return;
    }

    let spaced = kiwi
        .space_via_native("띄어쓰기가없는문장입니다", true)
        .expect("Failed to space_via_native");
    assert!(spaced.contains("띄어쓰기"));
    assert!(spaced.contains(' '));

    let chunks = ["안녕하세요", "반갑습니다"];
    let (glued, insertions) = kiwi
        .glue_with_options_via_native(&chunks, None, true)
        .expect("Failed to glue_with_options_via_native");
    assert!(!glued.is_empty());
    assert_eq!(insertions.as_ref().map(Vec::len), Some(1));
}

// ... (existing functions)

fn run_add_user_word() {
    println!("Starting run_add_user_word");
    // Need a fresh builder for this, but KiwiLibrary::load_default handles singleton
    let library = KiwiLibrary::load_default().expect("Failed to load Kiwi library");
    let mut builder = library
        .builder(BuilderConfig::default())
        .expect("Failed to create builder");

    // Some words can already exist in packaged dictionaries depending on version.
    // Try a small set of unlikely custom nouns until one is accepted.
    let mut added_word = None;
    let mut last_error = None;
    for candidate in [
        "초희귀커버리지전용어",
        "키위러스트검증전용명사",
        "데브시스터즈",
    ] {
        match builder.add_user_word(candidate, "NNP", 10.0) {
            Ok(()) => {
                added_word = Some(candidate);
                break;
            }
            Err(err) => last_error = Some(err.to_string()),
        }
    }

    let added_word = added_word.unwrap_or_else(|| {
        panic!(
            "Failed to add any user word candidate; last error: {}",
            last_error.unwrap_or_else(|| "unknown error".to_string())
        )
    });

    let kiwi = builder.build().expect("Failed to build Kiwi");
    let text = format!("{added_word}에 입사했다.");
    let res = kiwi.analyze(&text).expect("Failed to analyze");

    let top = &res[0];
    let has_user_word = top
        .tokens
        .iter()
        .any(|t| t.form == added_word && t.tag == "NNP");
    assert!(
        has_user_word,
        "Should detect user added word '{added_word}'"
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
