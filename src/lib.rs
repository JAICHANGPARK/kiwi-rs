#![deny(missing_docs)]

//! Rust bindings for Kiwi C API.
//!
//! This crate provides a high-level API that is convenient for day-to-day use,
//! while still exposing lower-level handles for advanced scenarios.
//!
//! ## Quick Start
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
//! ## Initialization Paths
//! `kiwi-rs` supports two common initialization modes:
//!
//! 1. Automatic bootstrap via [`Kiwi::init`]
//!   - Uses local paths first.
//!   - If unavailable, downloads a matching Kiwi library/model pair into cache.
//! 2. Explicit setup via [`Kiwi::from_config`]
//!   - For controlled deployments with fixed library/model paths.
//!
//! ```no_run
//! use kiwi_rs::{Kiwi, KiwiConfig};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = KiwiConfig::default()
//!         .with_library_path("/path/to/libkiwi.dylib")
//!         .with_model_path("/path/to/models/cong/base");
//!     let kiwi = Kiwi::from_config(config)?;
//!     let _tokens = kiwi.tokenize("형태소 분석 예시")?;
//!     Ok(())
//! }
//! ```
//!
//! ## Offset And Unit Rules
//! - For UTF-8 APIs, offsets are character indices (based on `str.chars()`),
//!   not byte indices.
//! - UTF-16 APIs accept `&[u16]`, but returned text in this crate is converted
//!   back to Rust UTF-8 `String`.
//!
//! ## Environment Variables
//! - `KIWI_LIBRARY_PATH`: explicit dynamic library path.
//! - `KIWI_MODEL_PATH`: explicit model directory path.
//! - `KIWI_RS_VERSION`: version used by [`Kiwi::init`] bootstrap (`latest` by default).
//! - `KIWI_RS_CACHE_DIR`: cache root used by [`Kiwi::init`] bootstrap.

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
    Kiwi, KiwiBuilder, KiwiLibrary, KiwiTypo, MorphemeSet, PreparedJoinMorphs, PreparedJoiner,
    Pretokenized, SwTokenizer,
};
pub use types::{
    Analysis, AnalysisCandidate, AnalyzeOptions, BuilderConfig, KiwiConfig, Sentence, Token,
    UserWord,
};

#[cfg(test)]
mod tests;
