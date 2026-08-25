//! Internal handle types and low-level FFI option structures.

use std::os::raw::{c_float, c_int, c_void};

/// Handle to a `Kiwi` instance (`kiwi_h`).
pub(crate) type KiwiHandle = *mut c_void;
/// Handle to a `KiwiBuilder` instance (`kiwi_builder_h`).
pub(crate) type KiwiBuilderHandle = *mut c_void;
/// Handle to an analysis result object (`kiwi_res_h`).
pub(crate) type KiwiResHandle = *mut c_void;
/// Handle to an extracted word set object (`kiwi_ws_h`).
pub(crate) type KiwiWsHandle = *mut c_void;
/// Handle to a sentence-split result object (`kiwi_ss_h`).
pub(crate) type KiwiSsHandle = *mut c_void;
/// Handle to a morpheme joiner object (`kiwi_joiner_h`).
pub(crate) type KiwiJoinerHandle = *mut c_void;
/// Handle to a morpheme set / blocklist object (`kiwi_morphset_h`).
pub(crate) type KiwiMorphsetHandle = *mut c_void;
/// Handle to a pretokenized structure object (`kiwi_pretokenized_h`).
pub(crate) type KiwiPretokenizedHandle = *mut c_void;
/// Handle to a mutable typo set object (`kiwi_typo_h`).
pub(crate) type KiwiTypoHandle = *mut c_void;
/// Handle to a compiled prepared typo transformer (`kiwi_prepared_typo_h`).
pub(crate) type KiwiPreparedTypoHandle = *mut c_void;
/// Handle to a subword tokenizer object (`kiwi_swtokenizer_h`).
pub(crate) type KiwiSwTokenizerHandle = *mut c_void;

/// Mirrors `kiwi_analyze_option_t` from Kiwi's `capi.h` as of v0.23.0+,
/// which appended two typo-correction fields (`typo_transformer`,
/// `typo_threshold`) to the end of the struct. The struct is passed by value
/// to the `kiwi_analyze*` functions, so the field order and layout must match
/// the C definition exactly. Passing this larger struct to libkiwi 0.22.x is
/// still safe: the callee only reads the leading fields it knows about, so the
/// extra trailing fields are simply ignored (backward compatible).
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct KiwiAnalyzeOption {
    pub(crate) match_options: c_int,
    pub(crate) blocklist: KiwiMorphsetHandle,
    pub(crate) open_ending: c_int,
    pub(crate) allowed_dialects: c_int,
    pub(crate) dialect_cost: c_float,
    pub(crate) typo_transformer: KiwiPreparedTypoHandle,
    pub(crate) typo_threshold: c_float,
}
