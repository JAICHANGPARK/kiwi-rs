use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_float, c_int, c_uint, c_void};
use std::path::Path;
use std::ptr;

use crate::config::{
    KiwiAnalyzeOption, KiwiBuilderHandle, KiwiHandle, KiwiJoinerHandle, KiwiMorphsetHandle,
    KiwiPretokenizedHandle, KiwiResHandle, KiwiSsHandle, KiwiSwTokenizerHandle, KiwiTypoHandle,
    KiwiWsHandle,
};
use crate::error::{KiwiError, Result};

type FnKiwiVersion = unsafe extern "C" fn() -> *const c_char;
type FnKiwiError = unsafe extern "C" fn() -> *const c_char;
type FnKiwiClearError = unsafe extern "C" fn();
type FnKiwiBuilderInit =
    unsafe extern "C" fn(*const c_char, c_int, c_int, c_int) -> KiwiBuilderHandle;
pub type KiwiStreamReadFunc = unsafe extern "C" fn(*mut c_void, *mut c_char, usize) -> usize;
pub type KiwiStreamSeekFunc = unsafe extern "C" fn(*mut c_void, i64, c_int) -> i64;
pub type KiwiStreamCloseFunc = unsafe extern "C" fn(*mut c_void);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KiwiStreamObjectRaw {
    pub read: KiwiStreamReadFunc,
    pub seek: KiwiStreamSeekFunc,
    pub close: KiwiStreamCloseFunc,
    pub user_data: *mut c_void,
}

pub type KiwiStreamFactory = unsafe extern "C" fn(*const c_char) -> KiwiStreamObjectRaw;
type FnKiwiBuilderInitStream =
    unsafe extern "C" fn(KiwiStreamFactory, c_int, c_int, c_int) -> KiwiBuilderHandle;
type FnKiwiBuilderClose = unsafe extern "C" fn(KiwiBuilderHandle) -> c_int;
type FnKiwiBuilderAddWord =
    unsafe extern "C" fn(KiwiBuilderHandle, *const c_char, *const c_char, c_float) -> c_int;
type FnKiwiBuilderAddAliasWord = unsafe extern "C" fn(
    KiwiBuilderHandle,
    *const c_char,
    *const c_char,
    c_float,
    *const c_char,
) -> c_int;
type FnKiwiBuilderAddPreAnalyzedWord = unsafe extern "C" fn(
    KiwiBuilderHandle,
    *const c_char,
    c_int,
    *const *const c_char,
    *const *const c_char,
    c_float,
    *const c_int,
) -> c_int;
type FnKiwiBuilderLoadDict = unsafe extern "C" fn(KiwiBuilderHandle, *const c_char) -> c_int;
pub(crate) type KiwiBuilderReplacer =
    unsafe extern "C" fn(*const c_char, c_int, *mut c_char, *mut c_void) -> c_int;
pub(crate) type KiwiReader = unsafe extern "C" fn(c_int, *mut c_char, *mut c_void) -> c_int;
type FnKiwiBuilderAddRule = unsafe extern "C" fn(
    KiwiBuilderHandle,
    *const c_char,
    KiwiBuilderReplacer,
    *mut c_void,
    c_float,
) -> c_int;
type FnKiwiBuilderExtractWords = unsafe extern "C" fn(
    KiwiBuilderHandle,
    KiwiReader,
    *mut c_void,
    c_int,
    c_int,
    c_float,
    c_float,
) -> KiwiWsHandle;
type FnKiwiBuilderExtractWordsW = unsafe extern "C" fn(
    KiwiBuilderHandle,
    KiwiReaderW,
    *mut c_void,
    c_int,
    c_int,
    c_float,
    c_float,
) -> KiwiWsHandle;
type FnKiwiBuilderExtractAddWords = unsafe extern "C" fn(
    KiwiBuilderHandle,
    KiwiReader,
    *mut c_void,
    c_int,
    c_int,
    c_float,
    c_float,
) -> KiwiWsHandle;
type FnKiwiBuilderExtractAddWordsW = unsafe extern "C" fn(
    KiwiBuilderHandle,
    KiwiReaderW,
    *mut c_void,
    c_int,
    c_int,
    c_float,
    c_float,
) -> KiwiWsHandle;
type FnKiwiBuilderBuild =
    unsafe extern "C" fn(KiwiBuilderHandle, KiwiTypoHandle, c_float) -> KiwiHandle;
type FnKiwiInit = unsafe extern "C" fn(*const c_char, c_int, c_int) -> KiwiHandle;
type FnKiwiClose = unsafe extern "C" fn(KiwiHandle) -> c_int;
type FnKiwiSetGlobalConfig = unsafe extern "C" fn(KiwiHandle, KiwiGlobalConfigRaw);
type FnKiwiGetGlobalConfig = unsafe extern "C" fn(KiwiHandle) -> KiwiGlobalConfigRaw;
type FnKiwiSetOption = unsafe extern "C" fn(KiwiHandle, c_int, c_int);
type FnKiwiGetOption = unsafe extern "C" fn(KiwiHandle, c_int) -> c_int;
type FnKiwiSetOptionF = unsafe extern "C" fn(KiwiHandle, c_int, c_float);
type FnKiwiGetOptionF = unsafe extern "C" fn(KiwiHandle, c_int) -> c_float;
type FnKiwiAnalyze = unsafe extern "C" fn(
    KiwiHandle,
    *const c_char,
    c_int,
    KiwiAnalyzeOption,
    KiwiPretokenizedHandle,
) -> KiwiResHandle;
type FnKiwiAnalyzeW = unsafe extern "C" fn(
    KiwiHandle,
    *const u16,
    c_int,
    KiwiAnalyzeOption,
    KiwiPretokenizedHandle,
) -> KiwiResHandle;
pub(crate) type KiwiReceiver = unsafe extern "C" fn(c_int, KiwiResHandle, *mut c_void) -> c_int;
pub(crate) type KiwiReaderW = unsafe extern "C" fn(c_int, *mut u16, *mut c_void) -> c_int;
type FnKiwiAnalyzeM = unsafe extern "C" fn(
    KiwiHandle,
    KiwiReader,
    KiwiReceiver,
    *mut c_void,
    c_int,
    KiwiAnalyzeOption,
) -> c_int;
type FnKiwiAnalyzeMw = unsafe extern "C" fn(
    KiwiHandle,
    KiwiReaderW,
    KiwiReceiver,
    *mut c_void,
    c_int,
    KiwiAnalyzeOption,
) -> c_int;
type FnKiwiSplitIntoSents =
    unsafe extern "C" fn(KiwiHandle, *const c_char, c_int, *mut KiwiResHandle) -> KiwiSsHandle;
type FnKiwiSplitIntoSentsW =
    unsafe extern "C" fn(KiwiHandle, *const u16, c_int, *mut KiwiResHandle) -> KiwiSsHandle;
type FnKiwiSsSize = unsafe extern "C" fn(KiwiSsHandle) -> c_int;
type FnKiwiSsBeginPosition = unsafe extern "C" fn(KiwiSsHandle, c_int) -> c_int;
type FnKiwiSsEndPosition = unsafe extern "C" fn(KiwiSsHandle, c_int) -> c_int;
type FnKiwiSsClose = unsafe extern "C" fn(KiwiSsHandle) -> c_int;
type FnKiwiNewJoiner = unsafe extern "C" fn(KiwiHandle, c_int) -> KiwiJoinerHandle;
type FnKiwiJoinerAdd =
    unsafe extern "C" fn(KiwiJoinerHandle, *const c_char, *const c_char, c_int) -> c_int;
type FnKiwiJoinerGet = unsafe extern "C" fn(KiwiJoinerHandle) -> *const c_char;
type FnKiwiJoinerGetW = unsafe extern "C" fn(KiwiJoinerHandle) -> *const u16;
type FnKiwiJoinerClose = unsafe extern "C" fn(KiwiJoinerHandle) -> c_int;
type FnKiwiNewMorphset = unsafe extern "C" fn(KiwiHandle) -> KiwiMorphsetHandle;
type FnKiwiMorphsetAdd =
    unsafe extern "C" fn(KiwiMorphsetHandle, *const c_char, *const c_char) -> c_int;
type FnKiwiMorphsetAddW =
    unsafe extern "C" fn(KiwiMorphsetHandle, *const u16, *const c_char) -> c_int;
type FnKiwiMorphsetClose = unsafe extern "C" fn(KiwiMorphsetHandle) -> c_int;
type FnKiwiTagToString = unsafe extern "C" fn(KiwiHandle, u8) -> *const c_char;
type FnKiwiResSize = unsafe extern "C" fn(KiwiResHandle) -> c_int;
type FnKiwiResProb = unsafe extern "C" fn(KiwiResHandle, c_int) -> c_float;
type FnKiwiResWordNum = unsafe extern "C" fn(KiwiResHandle, c_int) -> c_int;
type FnKiwiResTokenInfo =
    unsafe extern "C" fn(KiwiResHandle, c_int, c_int) -> *const KiwiTokenInfoRaw;
type FnKiwiResMorphemeId = unsafe extern "C" fn(KiwiResHandle, c_int, c_int, KiwiHandle) -> c_int;
type FnKiwiResFormW = unsafe extern "C" fn(KiwiResHandle, c_int, c_int) -> *const u16;
type FnKiwiResTagW = unsafe extern "C" fn(KiwiResHandle, c_int, c_int) -> *const u16;
type FnKiwiResForm = unsafe extern "C" fn(KiwiResHandle, c_int, c_int) -> *const c_char;
type FnKiwiResTag = unsafe extern "C" fn(KiwiResHandle, c_int, c_int) -> *const c_char;
type FnKiwiResPosition = unsafe extern "C" fn(KiwiResHandle, c_int, c_int) -> c_int;
type FnKiwiResLength = unsafe extern "C" fn(KiwiResHandle, c_int, c_int) -> c_int;
type FnKiwiResWordPosition = unsafe extern "C" fn(KiwiResHandle, c_int, c_int) -> c_int;
type FnKiwiResSentPosition = unsafe extern "C" fn(KiwiResHandle, c_int, c_int) -> c_int;
type FnKiwiResScore = unsafe extern "C" fn(KiwiResHandle, c_int, c_int) -> c_float;
type FnKiwiResTypoCost = unsafe extern "C" fn(KiwiResHandle, c_int, c_int) -> c_float;
type FnKiwiResClose = unsafe extern "C" fn(KiwiResHandle) -> c_int;
type FnKiwiWsSize = unsafe extern "C" fn(KiwiWsHandle) -> c_int;
type FnKiwiWsFormW = unsafe extern "C" fn(KiwiWsHandle, c_int) -> *const u16;
type FnKiwiWsForm = unsafe extern "C" fn(KiwiWsHandle, c_int) -> *const c_char;
type FnKiwiWsScore = unsafe extern "C" fn(KiwiWsHandle, c_int) -> c_float;
type FnKiwiWsFreq = unsafe extern "C" fn(KiwiWsHandle, c_int) -> c_int;
type FnKiwiWsPosScore = unsafe extern "C" fn(KiwiWsHandle, c_int) -> c_float;
type FnKiwiWsClose = unsafe extern "C" fn(KiwiWsHandle) -> c_int;
type FnKiwiPtInit = unsafe extern "C" fn() -> KiwiPretokenizedHandle;
type FnKiwiPtAddSpan = unsafe extern "C" fn(KiwiPretokenizedHandle, c_int, c_int) -> c_int;
type FnKiwiPtAddTokenToSpan = unsafe extern "C" fn(
    KiwiPretokenizedHandle,
    c_int,
    *const c_char,
    *const c_char,
    c_int,
    c_int,
) -> c_int;
type FnKiwiPtAddTokenToSpanW = unsafe extern "C" fn(
    KiwiPretokenizedHandle,
    c_int,
    *const u16,
    *const c_char,
    c_int,
    c_int,
) -> c_int;
type FnKiwiPtClose = unsafe extern "C" fn(KiwiPretokenizedHandle) -> c_int;
type FnKiwiTypoInit = unsafe extern "C" fn() -> KiwiTypoHandle;
type FnKiwiTypoGetBasic = unsafe extern "C" fn() -> KiwiTypoHandle;
type FnKiwiTypoGetDefault = unsafe extern "C" fn(c_int) -> KiwiTypoHandle;
type FnKiwiTypoAdd = unsafe extern "C" fn(
    KiwiTypoHandle,
    *const *const c_char,
    c_int,
    *const *const c_char,
    c_int,
    c_float,
    c_int,
) -> c_int;
type FnKiwiTypoCopy = unsafe extern "C" fn(KiwiTypoHandle) -> KiwiTypoHandle;
type FnKiwiTypoUpdate = unsafe extern "C" fn(KiwiTypoHandle, KiwiTypoHandle) -> c_int;
type FnKiwiTypoScaleCost = unsafe extern "C" fn(KiwiTypoHandle, c_float) -> c_int;
type FnKiwiTypoSetContinualTypoCost = unsafe extern "C" fn(KiwiTypoHandle, c_float) -> c_int;
type FnKiwiTypoSetLengtheningTypoCost = unsafe extern "C" fn(KiwiTypoHandle, c_float) -> c_int;
type FnKiwiTypoClose = unsafe extern "C" fn(KiwiTypoHandle) -> c_int;

type FnKiwiSwtInit = unsafe extern "C" fn(*const c_char, KiwiHandle) -> KiwiSwTokenizerHandle;
type FnKiwiSwtEncode = unsafe extern "C" fn(
    KiwiSwTokenizerHandle,
    *const c_char,
    c_int,
    *mut c_int,
    c_int,
    *mut c_int,
    c_int,
) -> c_int;
type FnKiwiSwtDecode =
    unsafe extern "C" fn(KiwiSwTokenizerHandle, *const c_int, c_int, *mut c_char, c_int) -> c_int;
type FnKiwiSwtClose = unsafe extern "C" fn(KiwiSwTokenizerHandle) -> c_int;
type FnKiwiFindMorphemes = unsafe extern "C" fn(
    KiwiHandle,
    *const c_char,
    *const c_char,
    c_int,
    *mut c_uint,
    c_int,
) -> c_int;
type FnKiwiFindMorphemesWithPrefix = unsafe extern "C" fn(
    KiwiHandle,
    *const c_char,
    *const c_char,
    c_int,
    *mut c_uint,
    c_int,
) -> c_int;

type FnKiwiGetMorphemeInfo = unsafe extern "C" fn(KiwiHandle, c_uint) -> KiwiMorphemeRaw;
type FnKiwiGetMorphemeFormW = unsafe extern "C" fn(KiwiHandle, c_uint) -> *const u16;
type FnKiwiGetMorphemeForm = unsafe extern "C" fn(KiwiHandle, c_uint) -> *const c_char;
type FnKiwiFreeMorphemeForm = unsafe extern "C" fn(*const c_char) -> c_int;

type FnKiwiCongMostSimilarWords =
    unsafe extern "C" fn(KiwiHandle, c_uint, *mut KiwiSimilarityPairRaw, c_int) -> c_int;
type FnKiwiCongSimilarity = unsafe extern "C" fn(KiwiHandle, c_uint, c_uint) -> c_float;
type FnKiwiCongMostSimilarContexts =
    unsafe extern "C" fn(KiwiHandle, c_uint, *mut KiwiSimilarityPairRaw, c_int) -> c_int;
type FnKiwiCongContextSimilarity = unsafe extern "C" fn(KiwiHandle, c_uint, c_uint) -> c_float;
type FnKiwiCongPredictWordsFromContext =
    unsafe extern "C" fn(KiwiHandle, c_uint, *mut KiwiSimilarityPairRaw, c_int) -> c_int;
type FnKiwiCongPredictWordsFromContextDiff = unsafe extern "C" fn(
    KiwiHandle,
    c_uint,
    c_uint,
    c_float,
    *mut KiwiSimilarityPairRaw,
    c_int,
) -> c_int;
type FnKiwiCongToContextId = unsafe extern "C" fn(KiwiHandle, *const c_uint, c_int) -> c_uint;
type FnKiwiCongFromContextId =
    unsafe extern "C" fn(KiwiHandle, c_uint, *mut c_uint, c_int) -> c_int;
type FnKiwiGetScriptName = unsafe extern "C" fn(u8) -> *const c_char;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct KiwiMorphemeRaw {
    pub(crate) tag: u8,
    pub(crate) sense_id: u8,
    pub(crate) user_score: c_float,
    pub(crate) lm_morpheme_id: c_uint,
    pub(crate) orig_morpheme_id: c_uint,
    pub(crate) dialect: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct KiwiTokenInfoRaw {
    pub(crate) chr_position: u32,
    pub(crate) word_position: u32,
    pub(crate) sent_position: u32,
    pub(crate) line_number: u32,
    pub(crate) length: u16,
    pub(crate) tag: u8,
    pub(crate) sense_or_script: u8,
    pub(crate) score: c_float,
    pub(crate) typo_cost: c_float,
    pub(crate) typo_form_id: u32,
    pub(crate) paired_token: u32,
    pub(crate) sub_sent_position: u32,
    pub(crate) dialect: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct KiwiGlobalConfigRaw {
    pub(crate) integrate_allomorph: u8,
    pub(crate) cut_off_threshold: c_float,
    pub(crate) unk_form_score_scale: c_float,
    pub(crate) unk_form_score_bias: c_float,
    pub(crate) space_penalty: c_float,
    pub(crate) typo_cost_weight: c_float,
    pub(crate) max_unk_form_size: u32,
    pub(crate) space_tolerance: u32,
}

impl Default for KiwiGlobalConfigRaw {
    fn default() -> Self {
        Self {
            integrate_allomorph: 1,
            cut_off_threshold: 8.0,
            unk_form_score_scale: 5.0,
            unk_form_score_bias: 5.0,
            space_penalty: 7.0,
            typo_cost_weight: 6.0,
            max_unk_form_size: 6,
            space_tolerance: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct KiwiSimilarityPairRaw {
    pub(crate) id: c_uint,
    pub(crate) score: c_float,
}

#[derive(Clone, Copy)]
pub(crate) struct KiwiApi {
    pub(crate) kiwi_version: FnKiwiVersion,
    pub(crate) kiwi_error: FnKiwiError,
    pub(crate) kiwi_clear_error: FnKiwiClearError,
    pub(crate) kiwi_builder_init: FnKiwiBuilderInit,
    pub(crate) kiwi_builder_init_stream: Option<FnKiwiBuilderInitStream>,
    pub(crate) kiwi_builder_close: FnKiwiBuilderClose,
    pub(crate) kiwi_builder_add_word: FnKiwiBuilderAddWord,
    pub(crate) kiwi_builder_build: FnKiwiBuilderBuild,
    pub(crate) kiwi_init: Option<FnKiwiInit>,
    pub(crate) kiwi_close: FnKiwiClose,
    pub(crate) kiwi_set_option: Option<FnKiwiSetOption>,
    pub(crate) kiwi_get_option: Option<FnKiwiGetOption>,
    pub(crate) kiwi_set_option_f: Option<FnKiwiSetOptionF>,
    pub(crate) kiwi_get_option_f: Option<FnKiwiGetOptionF>,
    pub(crate) kiwi_analyze: FnKiwiAnalyze,
    pub(crate) kiwi_analyze_w: Option<FnKiwiAnalyzeW>,
    pub(crate) kiwi_analyze_m: Option<FnKiwiAnalyzeM>,
    pub(crate) kiwi_analyze_mw: Option<FnKiwiAnalyzeMw>,
    pub(crate) kiwi_res_size: FnKiwiResSize,
    pub(crate) kiwi_res_prob: FnKiwiResProb,
    pub(crate) kiwi_res_word_num: FnKiwiResWordNum,
    pub(crate) kiwi_res_token_info: Option<FnKiwiResTokenInfo>,
    pub(crate) kiwi_res_morpheme_id: Option<FnKiwiResMorphemeId>,
    pub(crate) kiwi_res_form_w: Option<FnKiwiResFormW>,
    pub(crate) kiwi_res_tag_w: Option<FnKiwiResTagW>,
    pub(crate) kiwi_res_form: FnKiwiResForm,
    pub(crate) kiwi_res_tag: FnKiwiResTag,
    pub(crate) kiwi_res_position: FnKiwiResPosition,
    pub(crate) kiwi_res_length: FnKiwiResLength,
    pub(crate) kiwi_res_word_position: FnKiwiResWordPosition,
    pub(crate) kiwi_res_sent_position: FnKiwiResSentPosition,
    pub(crate) kiwi_res_score: FnKiwiResScore,
    pub(crate) kiwi_res_typo_cost: FnKiwiResTypoCost,
    pub(crate) kiwi_res_close: FnKiwiResClose,
    pub(crate) kiwi_ws_size: Option<FnKiwiWsSize>,
    pub(crate) kiwi_ws_form_w: Option<FnKiwiWsFormW>,
    pub(crate) kiwi_ws_form: Option<FnKiwiWsForm>,
    pub(crate) kiwi_ws_score: Option<FnKiwiWsScore>,
    pub(crate) kiwi_ws_freq: Option<FnKiwiWsFreq>,
    pub(crate) kiwi_ws_pos_score: Option<FnKiwiWsPosScore>,
    pub(crate) kiwi_ws_close: Option<FnKiwiWsClose>,
    pub(crate) kiwi_pt_init: Option<FnKiwiPtInit>,
    pub(crate) kiwi_pt_add_span: Option<FnKiwiPtAddSpan>,
    pub(crate) kiwi_pt_add_token_to_span: Option<FnKiwiPtAddTokenToSpan>,
    pub(crate) kiwi_pt_add_token_to_span_w: Option<FnKiwiPtAddTokenToSpanW>,
    pub(crate) kiwi_pt_close: Option<FnKiwiPtClose>,
    pub(crate) kiwi_typo_init: Option<FnKiwiTypoInit>,
    pub(crate) kiwi_typo_get_basic: Option<FnKiwiTypoGetBasic>,
    pub(crate) kiwi_typo_get_default: Option<FnKiwiTypoGetDefault>,
    pub(crate) kiwi_typo_add: Option<FnKiwiTypoAdd>,
    pub(crate) kiwi_typo_copy: Option<FnKiwiTypoCopy>,
    pub(crate) kiwi_typo_update: Option<FnKiwiTypoUpdate>,
    pub(crate) kiwi_typo_scale_cost: Option<FnKiwiTypoScaleCost>,
    pub(crate) kiwi_typo_set_continual_typo_cost: Option<FnKiwiTypoSetContinualTypoCost>,
    pub(crate) kiwi_typo_set_lengthening_typo_cost: Option<FnKiwiTypoSetLengtheningTypoCost>,
    pub(crate) kiwi_typo_close: Option<FnKiwiTypoClose>,
    pub(crate) kiwi_set_global_config: Option<FnKiwiSetGlobalConfig>,
    pub(crate) kiwi_get_global_config: Option<FnKiwiGetGlobalConfig>,
    pub(crate) kiwi_builder_add_rule: Option<FnKiwiBuilderAddRule>,
    pub(crate) kiwi_builder_add_alias_word: Option<FnKiwiBuilderAddAliasWord>,
    pub(crate) kiwi_builder_add_pre_analyzed_word: Option<FnKiwiBuilderAddPreAnalyzedWord>,
    pub(crate) kiwi_builder_load_dict: Option<FnKiwiBuilderLoadDict>,
    pub(crate) kiwi_builder_extract_words: Option<FnKiwiBuilderExtractWords>,
    pub(crate) kiwi_builder_extract_words_w: Option<FnKiwiBuilderExtractWordsW>,
    pub(crate) kiwi_builder_extract_add_words: Option<FnKiwiBuilderExtractAddWords>,
    pub(crate) kiwi_builder_extract_add_words_w: Option<FnKiwiBuilderExtractAddWordsW>,
    pub(crate) kiwi_split_into_sents: Option<FnKiwiSplitIntoSents>,
    pub(crate) kiwi_split_into_sents_w: Option<FnKiwiSplitIntoSentsW>,
    pub(crate) kiwi_ss_size: Option<FnKiwiSsSize>,
    pub(crate) kiwi_ss_begin_position: Option<FnKiwiSsBeginPosition>,
    pub(crate) kiwi_ss_end_position: Option<FnKiwiSsEndPosition>,
    pub(crate) kiwi_ss_close: Option<FnKiwiSsClose>,
    pub(crate) kiwi_new_joiner: Option<FnKiwiNewJoiner>,
    pub(crate) kiwi_joiner_add: Option<FnKiwiJoinerAdd>,
    pub(crate) kiwi_joiner_get: Option<FnKiwiJoinerGet>,
    pub(crate) kiwi_joiner_get_w: Option<FnKiwiJoinerGetW>,
    pub(crate) kiwi_joiner_close: Option<FnKiwiJoinerClose>,
    pub(crate) kiwi_new_morphset: Option<FnKiwiNewMorphset>,
    pub(crate) kiwi_morphset_add: Option<FnKiwiMorphsetAdd>,
    pub(crate) kiwi_morphset_add_w: Option<FnKiwiMorphsetAddW>,
    pub(crate) kiwi_morphset_close: Option<FnKiwiMorphsetClose>,
    pub(crate) kiwi_tag_to_string: Option<FnKiwiTagToString>,
    pub(crate) kiwi_find_morphemes: Option<FnKiwiFindMorphemes>,
    pub(crate) kiwi_find_morphemes_with_prefix: Option<FnKiwiFindMorphemesWithPrefix>,
    pub(crate) kiwi_get_morpheme_info: Option<FnKiwiGetMorphemeInfo>,
    pub(crate) kiwi_get_morpheme_form_w: Option<FnKiwiGetMorphemeFormW>,
    pub(crate) kiwi_get_morpheme_form: Option<FnKiwiGetMorphemeForm>,
    pub(crate) kiwi_free_morpheme_form: Option<FnKiwiFreeMorphemeForm>,
    pub(crate) kiwi_cong_most_similar_words: Option<FnKiwiCongMostSimilarWords>,
    pub(crate) kiwi_cong_similarity: Option<FnKiwiCongSimilarity>,
    pub(crate) kiwi_cong_most_similar_contexts: Option<FnKiwiCongMostSimilarContexts>,
    pub(crate) kiwi_cong_context_similarity: Option<FnKiwiCongContextSimilarity>,
    pub(crate) kiwi_cong_predict_words_from_context: Option<FnKiwiCongPredictWordsFromContext>,
    pub(crate) kiwi_cong_predict_words_from_context_diff:
        Option<FnKiwiCongPredictWordsFromContextDiff>,
    pub(crate) kiwi_cong_to_context_id: Option<FnKiwiCongToContextId>,
    pub(crate) kiwi_cong_from_context_id: Option<FnKiwiCongFromContextId>,
    pub(crate) kiwi_swt_init: Option<FnKiwiSwtInit>,
    pub(crate) kiwi_swt_encode: Option<FnKiwiSwtEncode>,
    pub(crate) kiwi_swt_decode: Option<FnKiwiSwtDecode>,
    pub(crate) kiwi_swt_close: Option<FnKiwiSwtClose>,
    pub(crate) kiwi_get_script_name: Option<FnKiwiGetScriptName>,
}

impl KiwiApi {
    pub(crate) unsafe fn load(library: &DynamicLibrary) -> Result<Self> {
        Ok(Self {
            kiwi_version: library.load_symbol("kiwi_version")?,
            kiwi_error: library.load_symbol("kiwi_error")?,
            kiwi_clear_error: library.load_symbol("kiwi_clear_error")?,
            kiwi_builder_init: library.load_symbol("kiwi_builder_init")?,
            kiwi_builder_init_stream: library.load_symbol_optional("kiwi_builder_init_stream")?,
            kiwi_builder_close: library.load_symbol("kiwi_builder_close")?,
            kiwi_builder_add_word: library.load_symbol("kiwi_builder_add_word")?,
            kiwi_builder_build: library.load_symbol("kiwi_builder_build")?,
            kiwi_init: library.load_symbol_optional("kiwi_init")?,
            kiwi_close: library.load_symbol("kiwi_close")?,
            kiwi_set_option: library.load_symbol_optional("kiwi_set_option")?,
            kiwi_get_option: library.load_symbol_optional("kiwi_get_option")?,
            kiwi_set_option_f: library.load_symbol_optional("kiwi_set_option_f")?,
            kiwi_get_option_f: library.load_symbol_optional("kiwi_get_option_f")?,
            kiwi_analyze: library.load_symbol("kiwi_analyze")?,
            kiwi_analyze_w: library.load_symbol_optional("kiwi_analyze_w")?,
            kiwi_analyze_m: library.load_symbol_optional("kiwi_analyze_m")?,
            kiwi_analyze_mw: library.load_symbol_optional("kiwi_analyze_mw")?,
            kiwi_res_size: library.load_symbol("kiwi_res_size")?,
            kiwi_res_prob: library.load_symbol("kiwi_res_prob")?,
            kiwi_res_word_num: library.load_symbol("kiwi_res_word_num")?,
            kiwi_res_token_info: library.load_symbol_optional("kiwi_res_token_info")?,
            kiwi_res_morpheme_id: library.load_symbol_optional("kiwi_res_morpheme_id")?,
            kiwi_res_form_w: library.load_symbol_optional("kiwi_res_form_w")?,
            kiwi_res_tag_w: library.load_symbol_optional("kiwi_res_tag_w")?,
            kiwi_res_form: library.load_symbol("kiwi_res_form")?,
            kiwi_res_tag: library.load_symbol("kiwi_res_tag")?,
            kiwi_res_position: library.load_symbol("kiwi_res_position")?,
            kiwi_res_length: library.load_symbol("kiwi_res_length")?,
            kiwi_res_word_position: library.load_symbol("kiwi_res_word_position")?,
            kiwi_res_sent_position: library.load_symbol("kiwi_res_sent_position")?,
            kiwi_res_score: library.load_symbol("kiwi_res_score")?,
            kiwi_res_typo_cost: library.load_symbol("kiwi_res_typo_cost")?,
            kiwi_res_close: library.load_symbol("kiwi_res_close")?,
            kiwi_ws_size: library.load_symbol_optional("kiwi_ws_size")?,
            kiwi_ws_form_w: library.load_symbol_optional("kiwi_ws_form_w")?,
            kiwi_ws_form: library.load_symbol_optional("kiwi_ws_form")?,
            kiwi_ws_score: library.load_symbol_optional("kiwi_ws_score")?,
            kiwi_ws_freq: library.load_symbol_optional("kiwi_ws_freq")?,
            kiwi_ws_pos_score: library.load_symbol_optional("kiwi_ws_pos_score")?,
            kiwi_ws_close: library.load_symbol_optional("kiwi_ws_close")?,
            kiwi_pt_init: library.load_symbol_optional("kiwi_pt_init")?,
            kiwi_pt_add_span: library.load_symbol_optional("kiwi_pt_add_span")?,
            kiwi_pt_add_token_to_span: library.load_symbol_optional("kiwi_pt_add_token_to_span")?,
            kiwi_pt_add_token_to_span_w: library
                .load_symbol_optional("kiwi_pt_add_token_to_span_w")?,
            kiwi_pt_close: library.load_symbol_optional("kiwi_pt_close")?,
            kiwi_typo_init: library.load_symbol_optional("kiwi_typo_init")?,
            kiwi_typo_get_basic: library.load_symbol_optional("kiwi_typo_get_basic")?,
            kiwi_typo_get_default: library.load_symbol_optional("kiwi_typo_get_default")?,
            kiwi_typo_add: library.load_symbol_optional("kiwi_typo_add")?,
            kiwi_typo_copy: library.load_symbol_optional("kiwi_typo_copy")?,
            kiwi_typo_update: library.load_symbol_optional("kiwi_typo_update")?,
            kiwi_typo_scale_cost: library.load_symbol_optional("kiwi_typo_scale_cost")?,
            kiwi_typo_set_continual_typo_cost: library
                .load_symbol_optional("kiwi_typo_set_continual_typo_cost")?,
            kiwi_typo_set_lengthening_typo_cost: library
                .load_symbol_optional("kiwi_typo_set_lengthening_typo_cost")?,
            kiwi_typo_close: library.load_symbol_optional("kiwi_typo_close")?,
            kiwi_set_global_config: library.load_symbol_optional("kiwi_set_global_config")?,
            kiwi_get_global_config: library.load_symbol_optional("kiwi_get_global_config")?,
            kiwi_builder_add_rule: library.load_symbol_optional("kiwi_builder_add_rule")?,
            kiwi_builder_add_alias_word: library
                .load_symbol_optional("kiwi_builder_add_alias_word")?,
            kiwi_builder_add_pre_analyzed_word: library
                .load_symbol_optional("kiwi_builder_add_pre_analyzed_word")?,
            kiwi_builder_load_dict: library.load_symbol_optional("kiwi_builder_load_dict")?,
            kiwi_builder_extract_words: library
                .load_symbol_optional("kiwi_builder_extract_words")?,
            kiwi_builder_extract_words_w: library
                .load_symbol_optional("kiwi_builder_extract_words_w")?,
            kiwi_builder_extract_add_words: library
                .load_symbol_optional("kiwi_builder_extract_add_words")?,
            kiwi_builder_extract_add_words_w: library
                .load_symbol_optional("kiwi_builder_extract_add_words_w")?,
            kiwi_split_into_sents: library.load_symbol_optional("kiwi_split_into_sents")?,
            kiwi_split_into_sents_w: library.load_symbol_optional("kiwi_split_into_sents_w")?,
            kiwi_ss_size: library.load_symbol_optional("kiwi_ss_size")?,
            kiwi_ss_begin_position: library.load_symbol_optional("kiwi_ss_begin_position")?,
            kiwi_ss_end_position: library.load_symbol_optional("kiwi_ss_end_position")?,
            kiwi_ss_close: library.load_symbol_optional("kiwi_ss_close")?,
            kiwi_new_joiner: library.load_symbol_optional("kiwi_new_joiner")?,
            kiwi_joiner_add: library.load_symbol_optional("kiwi_joiner_add")?,
            kiwi_joiner_get: library.load_symbol_optional("kiwi_joiner_get")?,
            kiwi_joiner_get_w: library.load_symbol_optional("kiwi_joiner_get_w")?,
            kiwi_joiner_close: library.load_symbol_optional("kiwi_joiner_close")?,
            kiwi_new_morphset: library.load_symbol_optional("kiwi_new_morphset")?,
            kiwi_morphset_add: library.load_symbol_optional("kiwi_morphset_add")?,
            kiwi_morphset_add_w: library.load_symbol_optional("kiwi_morphset_add_w")?,
            kiwi_morphset_close: library.load_symbol_optional("kiwi_morphset_close")?,
            kiwi_tag_to_string: library.load_symbol_optional("kiwi_tag_to_string")?,
            kiwi_find_morphemes: library.load_symbol_optional("kiwi_find_morphemes")?,
            kiwi_find_morphemes_with_prefix: library
                .load_symbol_optional("kiwi_find_morphemes_with_prefix")?,
            kiwi_get_morpheme_info: library.load_symbol_optional("kiwi_get_morpheme_info")?,
            kiwi_get_morpheme_form_w: library.load_symbol_optional("kiwi_get_morpheme_form_w")?,
            kiwi_get_morpheme_form: library.load_symbol_optional("kiwi_get_morpheme_form")?,
            kiwi_free_morpheme_form: library.load_symbol_optional("kiwi_free_morpheme_form")?,
            kiwi_cong_most_similar_words: library
                .load_symbol_optional("kiwi_cong_most_similar_words")?,
            kiwi_cong_similarity: library.load_symbol_optional("kiwi_cong_similarity")?,
            kiwi_cong_most_similar_contexts: library
                .load_symbol_optional("kiwi_cong_most_similar_contexts")?,
            kiwi_cong_context_similarity: library
                .load_symbol_optional("kiwi_cong_context_similarity")?,
            kiwi_cong_predict_words_from_context: library
                .load_symbol_optional("kiwi_cong_predict_words_from_context")?,
            kiwi_cong_predict_words_from_context_diff: library
                .load_symbol_optional("kiwi_cong_predict_words_from_context_diff")?,
            kiwi_cong_to_context_id: library.load_symbol_optional("kiwi_cong_to_context_id")?,
            kiwi_cong_from_context_id: library.load_symbol_optional("kiwi_cong_from_context_id")?,
            kiwi_swt_init: library.load_symbol_optional("kiwi_swt_init")?,
            kiwi_swt_encode: library.load_symbol_optional("kiwi_swt_encode")?,
            kiwi_swt_decode: library.load_symbol_optional("kiwi_swt_decode")?,
            kiwi_swt_close: library.load_symbol_optional("kiwi_swt_close")?,
            kiwi_get_script_name: library.load_symbol_optional("kiwi_get_script_name")?,
        })
    }
}

pub(crate) struct LoadedLibrary {
    pub(crate) _library: DynamicLibrary,
    pub(crate) api: KiwiApi,
}

#[derive(Debug)]
pub(crate) struct DynamicLibrary {
    handle: *mut c_void,
}

impl DynamicLibrary {
    pub(crate) fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_string = path.as_ref().to_string_lossy().to_string();
        let path_c = CString::new(path_string.clone())?;
        let handle = unsafe { platform_open(path_c.as_ptr()) };
        if handle.is_null() {
            return Err(KiwiError::LibraryLoad(format!(
                "{} ({})",
                path_string,
                platform_last_error()
            )));
        }
        Ok(Self { handle })
    }

    pub(crate) unsafe fn load_symbol<T: Copy>(&self, symbol_name: &str) -> Result<T> {
        let symbol_c = CString::new(symbol_name)?;
        let symbol_ptr = platform_symbol(self.handle, symbol_c.as_ptr());
        if symbol_ptr.is_null() {
            return Err(KiwiError::SymbolLoad(format!(
                "{} ({})",
                symbol_name,
                platform_last_error()
            )));
        }
        Ok(std::mem::transmute_copy::<*mut c_void, T>(&symbol_ptr))
    }

    pub(crate) unsafe fn load_symbol_optional<T: Copy>(
        &self,
        symbol_name: &str,
    ) -> Result<Option<T>> {
        let symbol_c = CString::new(symbol_name)?;
        let symbol_ptr = platform_symbol(self.handle, symbol_c.as_ptr());
        if symbol_ptr.is_null() {
            return Ok(None);
        }
        Ok(Some(std::mem::transmute_copy::<*mut c_void, T>(
            &symbol_ptr,
        )))
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        unsafe {
            platform_close(self.handle);
        }
        self.handle = ptr::null_mut();
    }
}

pub(crate) fn clear_kiwi_error(api: &KiwiApi) {
    unsafe {
        (api.kiwi_clear_error)();
    }
}

pub(crate) fn read_kiwi_error(api: &KiwiApi) -> Option<String> {
    let message_ptr = unsafe { (api.kiwi_error)() };
    if message_ptr.is_null() {
        return None;
    }
    let message = unsafe { CStr::from_ptr(message_ptr) }
        .to_string_lossy()
        .trim()
        .to_string();
    if message.is_empty() {
        None
    } else {
        Some(message)
    }
}

pub(crate) fn api_error(api: &KiwiApi, fallback: &str) -> KiwiError {
    match read_kiwi_error(api) {
        Some(message) => KiwiError::Api(message),
        None => KiwiError::Api(fallback.to_string()),
    }
}

pub(crate) fn cstr_to_string(pointer: *const c_char) -> String {
    if pointer.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .to_string()
}

pub(crate) fn c16str_to_string(pointer: *const u16) -> String {
    if pointer.is_null() {
        return String::new();
    }

    let mut len = 0usize;
    unsafe {
        while *pointer.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(pointer, len);
        std::char::decode_utf16(slice.iter().copied())
            .map(|value| value.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect()
    }
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryA(lp_lib_file_name: *const c_char) -> *mut c_void;
    fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const c_char) -> *mut c_void;
    fn FreeLibrary(h_lib_module: *mut c_void) -> i32;
    fn GetLastError() -> u32;
}

#[cfg(target_os = "windows")]
unsafe fn platform_open(path: *const c_char) -> *mut c_void {
    LoadLibraryA(path)
}

#[cfg(target_os = "windows")]
unsafe fn platform_symbol(handle: *mut c_void, symbol: *const c_char) -> *mut c_void {
    GetProcAddress(handle, symbol)
}

#[cfg(target_os = "windows")]
unsafe fn platform_close(handle: *mut c_void) {
    let _ = FreeLibrary(handle);
}

#[cfg(target_os = "windows")]
fn platform_last_error() -> String {
    format!("GetLastError={}", unsafe { GetLastError() })
}

#[cfg(target_os = "linux")]
#[link(name = "dl")]
extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlerror() -> *const c_char;
}

#[cfg(target_os = "macos")]
extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlerror() -> *const c_char;
}

#[cfg(unix)]
unsafe fn platform_open(path: *const c_char) -> *mut c_void {
    const RTLD_NOW: c_int = 2;
    const RTLD_LOCAL: c_int = 0;
    dlopen(path, RTLD_NOW | RTLD_LOCAL)
}

#[cfg(unix)]
unsafe fn platform_symbol(handle: *mut c_void, symbol: *const c_char) -> *mut c_void {
    dlsym(handle, symbol)
}

#[cfg(unix)]
unsafe fn platform_close(handle: *mut c_void) {
    let _ = dlclose(handle);
}

#[cfg(unix)]
fn platform_last_error() -> String {
    let pointer = unsafe { dlerror() };
    if pointer.is_null() {
        "unknown error".to_string()
    } else {
        let full = unsafe { CStr::from_ptr(pointer) }
            .to_string_lossy()
            .to_string();
        full.split(": tried:").next().unwrap_or(&full).to_string()
    }
}
