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
pub(crate) type KiwiSwTokenizerHandle = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct KiwiAnalyzeOption {
    pub(crate) match_options: c_int,
    pub(crate) blocklist: KiwiMorphsetHandle,
    pub(crate) open_ending: c_int,
    pub(crate) allowed_dialects: c_int,
    pub(crate) dialect_cost: c_float,
}
