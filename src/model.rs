use crate::native::{KiwiGlobalConfigRaw, KiwiMorphemeRaw, KiwiTokenInfoRaw};

#[derive(Debug, Clone)]
pub struct PreAnalyzedToken {
    pub form: String,
    pub tag: String,
    pub begin: Option<usize>,
    pub end: Option<usize>,
}

impl PreAnalyzedToken {
    pub fn new(form: impl Into<String>, tag: impl Into<String>) -> Self {
        Self {
            form: form.into(),
            tag: tag.into(),
            begin: None,
            end: None,
        }
    }

    pub fn with_span(mut self, begin: usize, end: usize) -> Self {
        self.begin = Some(begin);
        self.end = Some(end);
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SentenceBoundary {
    pub begin: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct SimilarityPair {
    pub id: u32,
    pub score: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct TokenInfo {
    pub chr_position: u32,
    pub word_position: u32,
    pub sent_position: u32,
    pub line_number: u32,
    pub length: u16,
    pub tag: u8,
    pub sense_or_script: u8,
    pub score: f32,
    pub typo_cost: f32,
    pub typo_form_id: u32,
    pub paired_token: u32,
    pub sub_sent_position: u32,
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

#[derive(Debug, Clone)]
pub struct ExtractedWord {
    pub form: String,
    pub score: f32,
    pub frequency: i32,
    pub pos_score: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct MorphemeInfo {
    pub tag: u8,
    pub sense_id: u8,
    pub user_score: f32,
    pub lm_morpheme_id: u32,
    pub orig_morpheme_id: u32,
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

#[derive(Debug, Clone)]
pub struct MorphemeSense {
    pub morph_id: u32,
    pub form: String,
    pub tag: String,
    pub sense_id: u8,
    pub dialect: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct GlobalConfig {
    pub integrate_allomorph: bool,
    pub cut_off_threshold: f32,
    pub unk_form_score_scale: f32,
    pub unk_form_score_bias: f32,
    pub space_penalty: f32,
    pub typo_cost_weight: f32,
    pub max_unk_form_size: u32,
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
