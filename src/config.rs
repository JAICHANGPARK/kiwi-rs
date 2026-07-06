use std::os::raw::{c_float, c_int, c_void};

pub(crate) type KiwiHandle = *mut c_void;
pub(crate) type KiwiBuilderHandle = *mut c_void;
pub(crate) type KiwiResHandle = *mut c_void;
pub(crate) type KiwiWsHandle = *mut c_void;
pub(crate) type KiwiSsHandle = *mut c_void;
pub(crate) type KiwiJoinerHandle = *mut c_void;
pub(crate) type KiwiMorphsetHandle = *mut c_void;
pub(crate) type KiwiPretokenizedHandle = *mut c_void;
pub(crate) type KiwiTypoHandle = *mut c_void;
pub(crate) type KiwiPreparedTypoHandle = *mut c_void;
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
