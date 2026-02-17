//! Public data types used by high-level kiwi-rs APIs.
//!
//! Unless stated otherwise, offset fields in this module are character-based
//! indices (`str.chars()`), not UTF-8 byte offsets.

use std::env;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};

use crate::constants::{
    KIWI_BUILD_DEFAULT, KIWI_DIALECT_STANDARD, KIWI_MATCH_ALL_WITH_NORMALIZING,
};
use crate::discovery::discover_default_model_path;
use crate::error::{KiwiError, Result};

/// A user dictionary entry consumed by [`crate::KiwiBuilder::add_user_words`].
#[derive(Debug, Clone)]
pub struct UserWord {
    /// Surface form to add.
    pub word: String,
    /// Part-of-speech tag for the word.
    pub tag: String,
    /// User score used by Kiwi during ranking.
    pub score: f32,
}

impl UserWord {
    /// Creates a user dictionary entry.
    pub fn new(word: impl Into<String>, tag: impl Into<String>, score: f32) -> Self {
        Self {
            word: word.into(),
            tag: tag.into(),
            score,
        }
    }
}

/// Options for `analyze*` and `tokenize*` APIs.
///
/// Most flag values come from constants re-exported by this crate
/// (`KIWI_MATCH_*`, `KIWI_DIALECT_*`).
#[derive(Debug, Clone, Copy)]
pub struct AnalyzeOptions {
    /// Number of candidate analyses to return.
    pub top_n: usize,
    /// Bit flags controlling token matching behavior.
    pub match_options: i32,
    /// Enables open-ended analysis mode.
    pub open_ending: bool,
    /// Allowed dialect bit mask.
    pub allowed_dialects: i32,
    /// Penalty used when selecting dialectal analyses.
    pub dialect_cost: f32,
}

impl Default for AnalyzeOptions {
    fn default() -> Self {
        Self {
            top_n: 1,
            match_options: KIWI_MATCH_ALL_WITH_NORMALIZING,
            open_ending: false,
            allowed_dialects: KIWI_DIALECT_STANDARD,
            dialect_cost: 3.0,
        }
    }
}

impl AnalyzeOptions {
    /// Sets the number of candidates.
    pub fn with_top_n(mut self, top_n: usize) -> Self {
        self.top_n = top_n;
        self
    }

    /// Sets `match_options` bit flags.
    pub fn with_match_options(mut self, match_options: i32) -> Self {
        self.match_options = match_options;
        self
    }

    /// Enables or disables open-ending analysis.
    pub fn with_open_ending(mut self, open_ending: bool) -> Self {
        self.open_ending = open_ending;
        self
    }

    /// Sets allowed dialect bit flags.
    pub fn with_allowed_dialects(mut self, allowed_dialects: i32) -> Self {
        self.allowed_dialects = allowed_dialects;
        self
    }

    /// Sets dialect mismatch penalty.
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

/// Builder-time configuration for constructing a [`crate::Kiwi`] instance.
#[derive(Debug, Clone)]
pub struct BuilderConfig {
    /// Model root directory (for example `.../models/cong/base`).
    pub model_path: Option<PathBuf>,
    /// Number of worker threads. `-1` follows Kiwi defaults.
    pub num_threads: i32,
    /// Kiwi build option bit flags (`KIWI_BUILD_*`).
    pub build_options: i32,
    /// Enabled dialect bit mask.
    pub enabled_dialects: i32,
    /// Cost threshold used when typo model is applied.
    pub typo_cost_threshold: f32,
}

impl Default for BuilderConfig {
    fn default() -> Self {
        Self {
            model_path: discover_default_model_path(),
            num_threads: -1,
            build_options: KIWI_BUILD_DEFAULT,
            enabled_dialects: KIWI_DIALECT_STANDARD,
            typo_cost_threshold: 0.0,
        }
    }
}

impl BuilderConfig {
    /// Sets model path.
    pub fn with_model_path(mut self, model_path: impl AsRef<Path>) -> Self {
        self.model_path = Some(model_path.as_ref().to_path_buf());
        self
    }

    /// Sets worker thread count.
    pub fn with_num_threads(mut self, num_threads: i32) -> Self {
        self.num_threads = num_threads;
        self
    }

    /// Sets build option flags.
    pub fn with_build_options(mut self, build_options: i32) -> Self {
        self.build_options = build_options;
        self
    }

    /// Sets enabled dialect bit mask.
    pub fn with_enabled_dialects(mut self, enabled_dialects: i32) -> Self {
        self.enabled_dialects = enabled_dialects;
        self
    }

    /// Sets typo cost threshold.
    pub fn with_typo_cost_threshold(mut self, typo_cost_threshold: f32) -> Self {
        self.typo_cost_threshold = typo_cost_threshold;
        self
    }
}

/// Top-level configuration used by [`crate::Kiwi::from_config`].
#[derive(Debug, Clone)]
pub struct KiwiConfig {
    /// Dynamic library path. Defaults to `KIWI_LIBRARY_PATH` env var.
    pub library_path: Option<PathBuf>,
    /// Builder-related options.
    pub builder: BuilderConfig,
    /// Default analysis options applied by convenience APIs.
    pub default_analyze_options: AnalyzeOptions,
    /// User dictionary entries inserted during initialization.
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
    /// Sets dynamic library path.
    pub fn with_library_path(mut self, library_path: impl AsRef<Path>) -> Self {
        self.library_path = Some(library_path.as_ref().to_path_buf());
        self
    }

    /// Sets model path inside [`Self::builder`].
    pub fn with_model_path(mut self, model_path: impl AsRef<Path>) -> Self {
        self.builder = self.builder.with_model_path(model_path);
        self
    }

    /// Replaces builder config.
    pub fn with_builder(mut self, builder: BuilderConfig) -> Self {
        self.builder = builder;
        self
    }

    /// Replaces default analysis options.
    pub fn with_default_analyze_options(mut self, options: AnalyzeOptions) -> Self {
        self.default_analyze_options = options;
        self
    }

    /// Adds one user dictionary entry.
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

/// A single morpheme token produced by Kiwi analysis.
#[derive(Debug, Clone)]
pub struct Token {
    /// Surface form.
    pub form: String,
    /// Part-of-speech tag string.
    pub tag: String,
    /// Character-based start offset in the original UTF-8 text (`str.chars()`).
    pub position: usize,
    /// Character length (`str.chars()` count), not byte length.
    pub length: usize,
    /// Word index inside the analyzed sentence.
    pub word_position: usize,
    /// Sentence index in multi-sentence analysis output.
    pub sent_position: usize,
    /// Line number metadata from Kiwi output.
    pub line_number: usize,
    /// Sub-sentence index metadata from Kiwi output.
    pub sub_sent_position: usize,
    /// Token score from language model.
    pub score: f32,
    /// Typo correction cost for this token.
    pub typo_cost: f32,
    /// Typo form identifier from Kiwi internals.
    pub typo_form_id: u32,
    /// Optional paired-token index (for paired punctuation etc.).
    pub paired_token: Option<usize>,
    /// Optional morpheme id for dictionary-backed APIs.
    pub morpheme_id: Option<u32>,
    /// Optional numeric tag id.
    pub tag_id: Option<u8>,
    /// Optional sense id or script id depending on tag.
    pub sense_or_script: Option<u8>,
    /// Optional dialect id.
    pub dialect: Option<u16>,
}

/// One analysis candidate, including probability and token list.
#[derive(Debug, Clone)]
pub struct AnalysisCandidate {
    /// Candidate probability score.
    pub probability: f32,
    /// Token sequence for this candidate.
    pub tokens: Vec<Token>,
}

/// Alias kept for readability in user code.
pub type Analysis = AnalysisCandidate;

/// Sentence split result used by `split_into_sents*_with_options`.
#[derive(Debug, Clone)]
pub struct Sentence {
    /// Raw sentence text slice (owned).
    pub text: String,
    /// Character-based start offset (`str.chars()` index).
    pub start: usize,
    /// Character-based end offset (`str.chars()` index).
    pub end: usize,
    /// Tokens in this sentence when requested.
    pub tokens: Option<Vec<Token>>,
    /// Nested sub-sentences when requested.
    pub subs: Option<Vec<Sentence>>,
}
