use crate::bootstrap::{extract_json_string_field, find_asset_url};
use crate::test_support::{with_env_var, with_env_vars};
use crate::{
    AnalyzeOptions, BuilderConfig, KiwiConfig, KIWI_DIALECT_STANDARD,
    KIWI_MATCH_ALL_WITH_NORMALIZING, KIWI_TYPO_BASIC_TYPO_SET, KIWI_TYPO_CONTINUAL_TYPO_SET,
    KIWI_TYPO_WITHOUT_TYPO,
};
use std::os::raw::c_int;
use std::path::PathBuf;

#[test]
fn analyze_options_default_is_reasonable() {
    let options = AnalyzeOptions::default();
    assert_eq!(options.top_n, 1);
    assert_eq!(options.match_options, KIWI_MATCH_ALL_WITH_NORMALIZING);
    assert!(!options.open_ending);
    assert_eq!(options.allowed_dialects, KIWI_DIALECT_STANDARD);
}

#[test]
fn analyze_options_validate_top_n() {
    let err = AnalyzeOptions::default().with_top_n(0).validated_top_n();
    assert!(err.is_err());
}

#[test]
fn analyze_options_builder_methods_update_fields() {
    let options = AnalyzeOptions::default()
        .with_top_n(3)
        .with_match_options(42)
        .with_open_ending(true)
        .with_allowed_dialects(7)
        .with_dialect_cost(1.25);

    assert_eq!(options.top_n, 3);
    assert_eq!(options.match_options, 42);
    assert!(options.open_ending);
    assert_eq!(options.allowed_dialects, 7);
    assert_eq!(options.dialect_cost, 1.25);
}

#[test]
fn analyze_options_validate_top_n_rejects_overflow() {
    let too_large = (c_int::MAX as usize).saturating_add(1);
    let err = AnalyzeOptions::default()
        .with_top_n(too_large)
        .validated_top_n();
    assert!(err.is_err());
}

#[test]
fn typo_constants_are_stable() {
    assert_eq!(KIWI_TYPO_WITHOUT_TYPO, 0);
    assert_eq!(KIWI_TYPO_BASIC_TYPO_SET, 1);
    assert_eq!(KIWI_TYPO_CONTINUAL_TYPO_SET, 2);
}

#[test]
fn kiwi_config_add_user_word() {
    let config = KiwiConfig::default().add_user_word("러스트", "NNP", 0.0);
    assert_eq!(config.user_words.len(), 1);
    assert_eq!(config.user_words[0].word, "러스트");
}

#[test]
fn builder_config_builder_methods_update_fields() {
    let config = BuilderConfig::default()
        .with_model_path("/tmp/kiwi-custom-model")
        .with_num_threads(8)
        .with_build_options(11)
        .with_enabled_dialects(5)
        .with_typo_cost_threshold(0.75);

    assert_eq!(
        config.model_path,
        Some(PathBuf::from("/tmp/kiwi-custom-model"))
    );
    assert_eq!(config.num_threads, 8);
    assert_eq!(config.build_options, 11);
    assert_eq!(config.enabled_dialects, 5);
    assert_eq!(config.typo_cost_threshold, 0.75);
}

#[test]
fn kiwi_config_builder_methods_update_fields() {
    let default_options = AnalyzeOptions::default().with_top_n(2);
    let builder = BuilderConfig::default().with_num_threads(2);
    let config = KiwiConfig::default()
        .with_library_path("/tmp/libkiwi-custom.so")
        .with_model_path("/tmp/model-path")
        .with_builder(builder.clone())
        .with_default_analyze_options(default_options)
        .add_user_word("테스트어", "NNP", 2.5);

    assert_eq!(
        config.library_path,
        Some(PathBuf::from("/tmp/libkiwi-custom.so"))
    );
    assert_eq!(config.builder.num_threads, builder.num_threads);
    assert_eq!(config.builder.model_path, builder.model_path);
    assert_eq!(config.default_analyze_options.top_n, 2);
    assert_eq!(config.user_words.len(), 1);
    assert_eq!(config.user_words[0].score, 2.5);
}

#[test]
fn parse_json_field() {
    let json = r#"{"tag_name":"v0.22.2","name":"Kiwi"}"#;
    let tag = extract_json_string_field(json, "tag_name");
    assert_eq!(tag.as_deref(), Some("v0.22.2"));
}

#[test]
fn find_asset_url_from_release_json() {
    let json = r#"{
        "assets": [
            {"name":"kiwi_model_v0.22.2_base.tgz","browser_download_url":"https://example/model.tgz"},
            {"name":"kiwi_mac_arm64_v0.22.2.tgz","browser_download_url":"https://example/mac.tgz"}
        ]
    }"#;
    let url = find_asset_url(json, "kiwi_mac_arm64_v0.22.2.tgz");
    assert_eq!(url.as_deref(), Some("https://example/mac.tgz"));
}

#[test]
fn find_asset_url_returns_none_when_missing() {
    let json = r#"{"assets":[{"name":"a","browser_download_url":"https://example/a"}]}"#;
    let url = find_asset_url(json, "not-found");
    assert!(url.is_none());
}

#[test]
fn parse_json_field_with_escaped_quotes() {
    let json = r#"{"message":"hello \"kiwi\""}"#;
    let message = extract_json_string_field(json, "message");
    assert_eq!(message.as_deref(), Some("hello \"kiwi\""));
}

#[test]
fn builder_config_default_respects_kiwi_model_path() {
    with_env_var("KIWI_MODEL_PATH", "/tmp/kiwi-rs-model", || {
        let config = BuilderConfig::default();
        assert_eq!(config.model_path, Some(PathBuf::from("/tmp/kiwi-rs-model")));
    });
}

#[test]
fn kiwi_config_default_respects_kiwi_library_path() {
    with_env_var("KIWI_LIBRARY_PATH", "/tmp/libkiwi-test.so", || {
        let config = KiwiConfig::default();
        assert_eq!(
            config.library_path,
            Some(PathBuf::from("/tmp/libkiwi-test.so"))
        );
    });
}

#[test]
fn env_test_helper_restores_state_after_panic() {
    let result = std::panic::catch_unwind(|| {
        with_env_vars(&[("KIWI_RS_TEST_PANIC_RESTORE", Some("on"))], || {
            panic!("intentional panic to verify restoration");
        });
    });

    assert!(result.is_err());
    assert!(std::env::var_os("KIWI_RS_TEST_PANIC_RESTORE").is_none());
}
