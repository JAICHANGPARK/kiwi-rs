use std::env;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};

use crate::constants::{KIWI_BUILD_DEFAULT, KIWI_DIALECT_ALL, KIWI_MATCH_ALL_WITH_NORMALIZING};
use crate::discovery::discover_default_model_path;
use crate::error::{KiwiError, Result};

#[derive(Debug, Clone)]
pub struct UserWord {
    pub word: String,
    pub tag: String,
    pub score: f32,
}

impl UserWord {
    pub fn new(word: impl Into<String>, tag: impl Into<String>, score: f32) -> Self {
        Self {
            word: word.into(),
            tag: tag.into(),
            score,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AnalyzeOptions {
    pub top_n: usize,
    pub match_options: i32,
    pub open_ending: bool,
    pub allowed_dialects: i32,
    pub dialect_cost: f32,
}

impl Default for AnalyzeOptions {
    fn default() -> Self {
        Self {
            top_n: 1,
            match_options: KIWI_MATCH_ALL_WITH_NORMALIZING,
            open_ending: false,
            allowed_dialects: KIWI_DIALECT_ALL,
            dialect_cost: 3.0,
        }
    }
}

impl AnalyzeOptions {
    pub fn with_top_n(mut self, top_n: usize) -> Self {
        self.top_n = top_n;
        self
    }

    pub fn with_match_options(mut self, match_options: i32) -> Self {
        self.match_options = match_options;
        self
    }

    pub fn with_open_ending(mut self, open_ending: bool) -> Self {
        self.open_ending = open_ending;
        self
    }

    pub fn with_allowed_dialects(mut self, allowed_dialects: i32) -> Self {
        self.allowed_dialects = allowed_dialects;
        self
    }

    pub fn with_dialect_cost(mut self, dialect_cost: f32) -> Self {
        self.dialect_cost = dialect_cost;
        self
    }

    pub(crate) fn validated_top_n(&self) -> Result<c_int> {
        if self.top_n == 0 {
            return Err(KiwiError::InvalidArgument(
                "AnalyzeOptions.top_n must be >= 1".to_string(),
            ));
        }
        if self.top_n > c_int::MAX as usize {
            return Err(KiwiError::InvalidArgument(format!(
                "AnalyzeOptions.top_n must be <= {}",
                c_int::MAX
            )));
        }
        Ok(self.top_n as c_int)
    }
}

#[derive(Debug, Clone)]
pub struct BuilderConfig {
    pub model_path: Option<PathBuf>,
    pub num_threads: i32,
    pub build_options: i32,
    pub enabled_dialects: i32,
    pub typo_cost_threshold: f32,
}

impl Default for BuilderConfig {
    fn default() -> Self {
        Self {
            model_path: discover_default_model_path(),
            num_threads: -1,
            build_options: KIWI_BUILD_DEFAULT,
            enabled_dialects: KIWI_DIALECT_ALL,
            typo_cost_threshold: 0.0,
        }
    }
}

impl BuilderConfig {
    pub fn with_model_path(mut self, model_path: impl AsRef<Path>) -> Self {
        self.model_path = Some(model_path.as_ref().to_path_buf());
        self
    }

    pub fn with_num_threads(mut self, num_threads: i32) -> Self {
        self.num_threads = num_threads;
        self
    }

    pub fn with_build_options(mut self, build_options: i32) -> Self {
        self.build_options = build_options;
        self
    }

    pub fn with_enabled_dialects(mut self, enabled_dialects: i32) -> Self {
        self.enabled_dialects = enabled_dialects;
        self
    }

    pub fn with_typo_cost_threshold(mut self, typo_cost_threshold: f32) -> Self {
        self.typo_cost_threshold = typo_cost_threshold;
        self
    }
}

#[derive(Debug, Clone)]
pub struct KiwiConfig {
    pub library_path: Option<PathBuf>,
    pub builder: BuilderConfig,
    pub default_analyze_options: AnalyzeOptions,
    pub user_words: Vec<UserWord>,
}

impl Default for KiwiConfig {
    fn default() -> Self {
        Self {
            library_path: env::var_os("KIWI_LIBRARY_PATH").map(PathBuf::from),
            builder: BuilderConfig::default(),
            default_analyze_options: AnalyzeOptions::default(),
            user_words: Vec::new(),
        }
    }
}

impl KiwiConfig {
    pub fn with_library_path(mut self, library_path: impl AsRef<Path>) -> Self {
        self.library_path = Some(library_path.as_ref().to_path_buf());
        self
    }

    pub fn with_model_path(mut self, model_path: impl AsRef<Path>) -> Self {
        self.builder = self.builder.with_model_path(model_path);
        self
    }

    pub fn with_builder(mut self, builder: BuilderConfig) -> Self {
        self.builder = builder;
        self
    }

    pub fn with_default_analyze_options(mut self, options: AnalyzeOptions) -> Self {
        self.default_analyze_options = options;
        self
    }

    pub fn add_user_word(
        mut self,
        word: impl Into<String>,
        tag: impl Into<String>,
        score: f32,
    ) -> Self {
        self.user_words.push(UserWord::new(word, tag, score));
        self
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub form: String,
    pub tag: String,
    pub position: usize,
    pub length: usize,
    pub word_position: usize,
    pub sent_position: usize,
    pub line_number: usize,
    pub sub_sent_position: usize,
    pub score: f32,
    pub typo_cost: f32,
    pub typo_form_id: u32,
    pub paired_token: Option<usize>,
    pub morpheme_id: Option<u32>,
    pub tag_id: Option<u8>,
    pub sense_or_script: Option<u8>,
    pub dialect: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct AnalysisCandidate {
    pub probability: f32,
    pub tokens: Vec<Token>,
}

pub type Analysis = AnalysisCandidate;

#[derive(Debug, Clone)]
pub struct Sentence {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub tokens: Option<Vec<Token>>,
    pub subs: Option<Vec<Sentence>>,
}
