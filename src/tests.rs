use crate::bootstrap::{extract_json_string_field, find_asset_url};
use crate::{
    AnalyzeOptions, BuilderConfig, KiwiConfig, KIWI_MATCH_ALL_WITH_NORMALIZING,
    KIWI_TYPO_BASIC_TYPO_SET, KIWI_TYPO_CONTINUAL_TYPO_SET, KIWI_TYPO_WITHOUT_TYPO,
};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn with_env_var<T>(key: &str, value: &str, f: impl FnOnce() -> T) -> T {
    let _guard = env_lock().lock().expect("env lock poisoned");
    let backup = std::env::var_os(key);
    #[allow(unused_unsafe)]
    unsafe {
        std::env::set_var(key, value);
    }

    let result = f();

    match backup {
        Some(original) => {
            #[allow(unused_unsafe)]
            unsafe {
                std::env::set_var(key, original);
            }
        }
        None => {
            #[allow(unused_unsafe)]
            unsafe {
                std::env::remove_var(key);
            }
        }
    }

    result
}

#[test]
fn analyze_options_default_is_reasonable() {
    let options = AnalyzeOptions::default();
    assert_eq!(options.top_n, 1);
    assert_eq!(options.match_options, KIWI_MATCH_ALL_WITH_NORMALIZING);
    assert!(!options.open_ending);
}

#[test]
fn analyze_options_validate_top_n() {
    let err = AnalyzeOptions::default().with_top_n(0).validated_top_n();
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
