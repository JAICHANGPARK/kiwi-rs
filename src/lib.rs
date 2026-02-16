//! Rust bindings for Kiwi C API.
//!
//! This crate provides a high-level API that is convenient for day-to-day use,
//! while still exposing lower-level handles for advanced scenarios.
//!
//! ## Quick start
//! ```no_run
//! use kiwi_rs::Kiwi;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let kiwi = Kiwi::init()?;
//!     let tokens = kiwi.tokenize("아버지가방에들어가신다.")?;
//!     for token in tokens {
//!         println!("{}/{}", token.form, token.tag);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! Set `KIWI_LIBRARY_PATH` and `KIWI_MODEL_PATH` if you prefer manual paths.

mod bootstrap;
mod config;
mod constants;
mod discovery;
mod error;
mod model;
mod native;
mod runtime;
mod types;

pub use constants::*;
pub use error::{KiwiError, Result};
pub use model::{
    ExtractedWord, GlobalConfig, MorphemeInfo, MorphemeSense, PreAnalyzedToken, SentenceBoundary,
    SimilarityPair, TokenInfo,
};
pub use runtime::{
    Kiwi, KiwiBuilder, KiwiLibrary, KiwiTypo, MorphemeSet, Pretokenized, SwTokenizer,
};
pub use types::{
    Analysis, AnalysisCandidate, AnalyzeOptions, BuilderConfig, KiwiConfig, Sentence, Token,
    UserWord,
};

#[cfg(test)]
mod tests;
