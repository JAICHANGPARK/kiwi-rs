use crate::native::{KiwiGlobalConfigRaw, KiwiMorphemeRaw, KiwiTokenInfoRaw};

/// Pre-analyzed token element passed to
/// [`crate::KiwiBuilder::add_pre_analyzed_word`].
///
/// `begin`/`end` are character offsets in the given surface form
/// (Rust `str.chars()` index space, not byte offsets).
#[derive(Debug, Clone)]
pub struct PreAnalyzedToken {
    /// Surface form.
    pub form: String,
    /// Part-of-speech tag.
    pub tag: String,
    /// Optional begin character offset.
    pub begin: Option<usize>,
    /// Optional end character offset.
    pub end: Option<usize>,
}

impl PreAnalyzedToken {
    /// Creates a token with only `form` and `tag`.
    pub fn new(form: impl Into<String>, tag: impl Into<String>) -> Self {
        Self {
            form: form.into(),
            tag: tag.into(),
            begin: None,
            end: None,
        }
    }

    /// Sets explicit span offsets.
    pub fn with_span(mut self, begin: usize, end: usize) -> Self {
        self.begin = Some(begin);
        self.end = Some(end);
        self
    }
}

/// Begin/end boundary for a sentence in character offsets.
///
/// Offsets are based on Rust `str.chars()` indexing.
#[derive(Debug, Clone, Copy)]
pub struct SentenceBoundary {
    /// Inclusive begin offset.
    pub begin: usize,
    /// Exclusive end offset.
    pub end: usize,
}

/// `(id, score)` pair returned by similarity and prediction APIs.
#[derive(Debug, Clone, Copy)]
pub struct SimilarityPair {
    /// Identifier of a morpheme or context.
    pub id: u32,
    /// Similarity or prediction score.
    pub score: f32,
}

/// Low-level token metadata returned by Kiwi C API.
///
/// Position-like fields (`chr_position`, `word_position`, `sent_position`) use
/// Kiwi's character/token indexing semantics.
#[derive(Debug, Clone, Copy)]
pub struct TokenInfo {
    /// Character position.
    pub chr_position: u32,
    /// Word position.
    pub word_position: u32,
    /// Sentence position.
    pub sent_position: u32,
    /// Line number.
    pub line_number: u32,
    /// Token length.
    pub length: u16,
    /// Numeric tag id.
    pub tag: u8,
    /// Sense id or script id.
    pub sense_or_script: u8,
    /// Token score.
    pub score: f32,
    /// Typo cost.
    pub typo_cost: f32,
    /// Typo form id.
    pub typo_form_id: u32,
    /// Paired token id.
    pub paired_token: u32,
    /// Sub-sentence position.
    pub sub_sent_position: u32,
    /// Dialect id.
    pub dialect: u16,
}

impl From<KiwiTokenInfoRaw> for TokenInfo {
    fn from(value: KiwiTokenInfoRaw) -> Self {
        Self {
            chr_position: value.chr_position,
            word_position: value.word_position,
            sent_position: value.sent_position,
            line_number: value.line_number,
            length: value.length,
            tag: value.tag,
            sense_or_script: value.sense_or_script,
            score: value.score,
            typo_cost: value.typo_cost,
            typo_form_id: value.typo_form_id,
            paired_token: value.paired_token,
            sub_sent_position: value.sub_sent_position,
            dialect: value.dialect,
        }
    }
}

/// Candidate extracted word from `extract_words*` builder APIs.
#[derive(Debug, Clone)]
pub struct ExtractedWord {
    /// Surface form.
    pub form: String,
    /// Extraction score.
    pub score: f32,
    /// Observed frequency.
    pub frequency: i32,
    /// POS-specific score from Kiwi.
    pub pos_score: f32,
}

/// Morpheme metadata from dictionary lookup APIs.
#[derive(Debug, Clone, Copy)]
pub struct MorphemeInfo {
    /// Numeric tag id.
    pub tag: u8,
    /// Sense id.
    pub sense_id: u8,
    /// User dictionary score.
    pub user_score: f32,
    /// Language-model morpheme id.
    pub lm_morpheme_id: u32,
    /// Original morpheme id.
    pub orig_morpheme_id: u32,
    /// Dialect id.
    pub dialect: u16,
}

impl From<KiwiMorphemeRaw> for MorphemeInfo {
    fn from(value: KiwiMorphemeRaw) -> Self {
        Self {
            tag: value.tag,
            sense_id: value.sense_id,
            user_score: value.user_score,
            lm_morpheme_id: value.lm_morpheme_id,
            orig_morpheme_id: value.orig_morpheme_id,
            dialect: value.dialect,
        }
    }
}

/// Morpheme information with resolved string fields.
#[derive(Debug, Clone)]
pub struct MorphemeSense {
    /// Morpheme id.
    pub morph_id: u32,
    /// Morpheme form.
    pub form: String,
    /// Morpheme tag.
    pub tag: String,
    /// Sense id.
    pub sense_id: u8,
    /// Dialect id.
    pub dialect: u16,
}

/// Global runtime parameters for Kiwi inference behavior.
#[derive(Debug, Clone, Copy)]
pub struct GlobalConfig {
    /// Whether to integrate allomorph variants.
    pub integrate_allomorph: bool,
    /// Candidate cut-off threshold.
    pub cut_off_threshold: f32,
    /// Scale applied to unknown-form score.
    pub unk_form_score_scale: f32,
    /// Bias applied to unknown-form score.
    pub unk_form_score_bias: f32,
    /// Penalty for spacing decisions.
    pub space_penalty: f32,
    /// Weight applied to typo costs.
    pub typo_cost_weight: f32,
    /// Maximum unknown token length.
    pub max_unk_form_size: u32,
    /// Allowed whitespace tolerance during analysis.
    pub space_tolerance: u32,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        KiwiGlobalConfigRaw::default().into()
    }
}

impl From<KiwiGlobalConfigRaw> for GlobalConfig {
    fn from(value: KiwiGlobalConfigRaw) -> Self {
        Self {
            integrate_allomorph: value.integrate_allomorph != 0,
            cut_off_threshold: value.cut_off_threshold,
            unk_form_score_scale: value.unk_form_score_scale,
            unk_form_score_bias: value.unk_form_score_bias,
            space_penalty: value.space_penalty,
            typo_cost_weight: value.typo_cost_weight,
            max_unk_form_size: value.max_unk_form_size,
            space_tolerance: value.space_tolerance,
        }
    }
}

impl From<GlobalConfig> for KiwiGlobalConfigRaw {
    fn from(value: GlobalConfig) -> Self {
        Self {
            integrate_allomorph: if value.integrate_allomorph { 1 } else { 0 },
            cut_off_threshold: value.cut_off_threshold,
            unk_form_score_scale: value.unk_form_score_scale,
            unk_form_score_bias: value.unk_form_score_bias,
            space_penalty: value.space_penalty,
            typo_cost_weight: value.typo_cost_weight,
            max_unk_form_size: value.max_unk_form_size,
            space_tolerance: value.space_tolerance,
        }
    }
}

#[cfg(test)]
mod model_tests {
    use super::{
        GlobalConfig, KiwiGlobalConfigRaw, KiwiMorphemeRaw, KiwiTokenInfoRaw, MorphemeInfo,
        PreAnalyzedToken, TokenInfo,
    };

    #[test]
    fn pre_analyzed_token_new_and_span() {
        let token = PreAnalyzedToken::new("형태소", "NNG").with_span(1, 3);
        assert_eq!(token.form, "형태소");
        assert_eq!(token.tag, "NNG");
        assert_eq!(token.begin, Some(1));
        assert_eq!(token.end, Some(3));
    }

    #[test]
    fn token_info_from_raw_copies_all_fields() {
        let raw = KiwiTokenInfoRaw {
            chr_position: 3,
            word_position: 2,
            sent_position: 1,
            line_number: 4,
            length: 5,
            tag: 9,
            sense_or_script: 7,
            score: 1.5,
            typo_cost: 0.25,
            typo_form_id: 11,
            paired_token: 12,
            sub_sent_position: 6,
            dialect: 8,
        };
        let info = TokenInfo::from(raw);

        assert_eq!(info.chr_position, 3);
        assert_eq!(info.word_position, 2);
        assert_eq!(info.sent_position, 1);
        assert_eq!(info.line_number, 4);
        assert_eq!(info.length, 5);
        assert_eq!(info.tag, 9);
        assert_eq!(info.sense_or_script, 7);
        assert_eq!(info.score, 1.5);
        assert_eq!(info.typo_cost, 0.25);
        assert_eq!(info.typo_form_id, 11);
        assert_eq!(info.paired_token, 12);
        assert_eq!(info.sub_sent_position, 6);
        assert_eq!(info.dialect, 8);
    }

    #[test]
    fn morpheme_info_from_raw_copies_all_fields() {
        let raw = KiwiMorphemeRaw {
            tag: 5,
            sense_id: 2,
            user_score: 0.5,
            lm_morpheme_id: 31,
            orig_morpheme_id: 17,
            dialect: 4,
        };
        let info = MorphemeInfo::from(raw);

        assert_eq!(info.tag, 5);
        assert_eq!(info.sense_id, 2);
        assert_eq!(info.user_score, 0.5);
        assert_eq!(info.lm_morpheme_id, 31);
        assert_eq!(info.orig_morpheme_id, 17);
        assert_eq!(info.dialect, 4);
    }

    #[test]
    fn global_config_default_matches_raw_defaults() {
        let config = GlobalConfig::default();
        assert!(config.integrate_allomorph);
        assert_eq!(config.cut_off_threshold, 8.0);
        assert_eq!(config.unk_form_score_scale, 5.0);
        assert_eq!(config.unk_form_score_bias, 5.0);
        assert_eq!(config.space_penalty, 7.0);
        assert_eq!(config.typo_cost_weight, 6.0);
        assert_eq!(config.max_unk_form_size, 6);
        assert_eq!(config.space_tolerance, 0);
    }

    #[test]
    fn global_config_round_trip_preserves_values() {
        let original = GlobalConfig {
            integrate_allomorph: false,
            cut_off_threshold: 4.2,
            unk_form_score_scale: 1.3,
            unk_form_score_bias: 0.4,
            space_penalty: 9.0,
            typo_cost_weight: 2.5,
            max_unk_form_size: 12,
            space_tolerance: 3,
        };

        let raw = KiwiGlobalConfigRaw::from(original);
        assert_eq!(raw.integrate_allomorph, 0);

        let round_trip = GlobalConfig::from(raw);
        assert!(!round_trip.integrate_allomorph);
        assert_eq!(round_trip.cut_off_threshold, 4.2);
        assert_eq!(round_trip.unk_form_score_scale, 1.3);
        assert_eq!(round_trip.unk_form_score_bias, 0.4);
        assert_eq!(round_trip.space_penalty, 9.0);
        assert_eq!(round_trip.typo_cost_weight, 2.5);
        assert_eq!(round_trip.max_unk_form_size, 12);
        assert_eq!(round_trip.space_tolerance, 3);
    }

    #[test]
    fn global_config_to_raw_sets_integrate_allomorph_flag() {
        let raw = KiwiGlobalConfigRaw::from(GlobalConfig {
            integrate_allomorph: true,
            ..GlobalConfig::default()
        });
        assert_eq!(raw.integrate_allomorph, 1);
    }
}
