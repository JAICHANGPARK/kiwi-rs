//! Constants mirrored from Kiwi C API option and flag values.

/// Build option: integrate allomorph variants.
pub const KIWI_BUILD_INTEGRATE_ALLOMORPH: i32 = 1;
/// Build option: load bundled default dictionary.
pub const KIWI_BUILD_LOAD_DEFAULT_DICT: i32 = 2;
/// Build option: load typo dictionary.
pub const KIWI_BUILD_LOAD_TYPO_DICT: i32 = 4;
/// Build option: load multi-word dictionary.
pub const KIWI_BUILD_LOAD_MULTI_DICT: i32 = 8;
/// Default build option mask.
pub const KIWI_BUILD_DEFAULT: i32 = 15;
/// Build option: default model type.
pub const KIWI_BUILD_MODEL_TYPE_DEFAULT: i32 = 0x0000;
/// Build option: largest model type.
pub const KIWI_BUILD_MODEL_TYPE_LARGEST: i32 = 0x0100;
/// Build option: KNLM model type.
pub const KIWI_BUILD_MODEL_TYPE_KNLM: i32 = 0x0200;
/// Build option: SBG model type.
pub const KIWI_BUILD_MODEL_TYPE_SBG: i32 = 0x0300;
/// Build option: CoNg model type.
pub const KIWI_BUILD_MODEL_TYPE_CONG: i32 = 0x0400;
/// Build option: global CoNg model type.
pub const KIWI_BUILD_MODEL_TYPE_CONG_GLOBAL: i32 = 0x0500;
/// Default build options with CoNg model.
pub const KIWI_BUILD_DEFAULT_WITH_CONG: i32 = KIWI_BUILD_DEFAULT | KIWI_BUILD_MODEL_TYPE_CONG;
/// Option key for setting worker threads through `set_option`.
pub const KIWI_NUM_THREADS: i32 = 0x8001;

/// Typo preset: disable typo correction.
pub const KIWI_TYPO_WITHOUT_TYPO: i32 = 0;
/// Typo preset: basic typo set.
pub const KIWI_TYPO_BASIC_TYPO_SET: i32 = 1;
/// Typo preset: continual typo set.
pub const KIWI_TYPO_CONTINUAL_TYPO_SET: i32 = 2;
/// Typo preset: basic + continual typo sets.
pub const KIWI_TYPO_BASIC_TYPO_SET_WITH_CONTINUAL: i32 = 3;
/// Typo preset: lengthening typo set.
pub const KIWI_TYPO_LENGTHENING_TYPO_SET: i32 = 4;
/// Typo preset: basic + continual + lengthening typo sets.
pub const KIWI_TYPO_BASIC_TYPO_SET_WITH_CONTINUAL_AND_LENGTHENING: i32 = 5;

/// Match option: URL detection.
pub const KIWI_MATCH_URL: i32 = 1;
/// Match option: email detection.
pub const KIWI_MATCH_EMAIL: i32 = 2;
/// Match option: hashtag detection.
pub const KIWI_MATCH_HASHTAG: i32 = 4;
/// Match option: mention detection.
pub const KIWI_MATCH_MENTION: i32 = 8;
/// Match option: serial number detection.
pub const KIWI_MATCH_SERIAL: i32 = 16;
/// Match option: normalize coda.
pub const KIWI_MATCH_NORMALIZE_CODA: i32 = 1 << 16;
/// Match option: join noun prefixes.
pub const KIWI_MATCH_JOIN_NOUN_PREFIX: i32 = 1 << 17;
/// Match option: join noun suffixes.
pub const KIWI_MATCH_JOIN_NOUN_SUFFIX: i32 = 1 << 18;
/// Match option: join verb suffixes.
pub const KIWI_MATCH_JOIN_VERB_SUFFIX: i32 = 1 << 19;
/// Match option: join adjective suffixes.
pub const KIWI_MATCH_JOIN_ADJ_SUFFIX: i32 = 1 << 20;
/// Match option: join adverb suffixes.
pub const KIWI_MATCH_JOIN_ADV_SUFFIX: i32 = 1 << 21;
/// Match option convenience mask for verb/adjective suffix joins.
pub const KIWI_MATCH_JOIN_V_SUFFIX: i32 = KIWI_MATCH_JOIN_VERB_SUFFIX | KIWI_MATCH_JOIN_ADJ_SUFFIX;
/// Match option convenience mask for all affix-join flags.
pub const KIWI_MATCH_JOIN_AFFIX: i32 = KIWI_MATCH_JOIN_NOUN_PREFIX
    | KIWI_MATCH_JOIN_NOUN_SUFFIX
    | KIWI_MATCH_JOIN_V_SUFFIX
    | KIWI_MATCH_JOIN_ADV_SUFFIX;
/// Match option: split complex morphemes.
pub const KIWI_MATCH_SPLIT_COMPLEX: i32 = 1 << 22;
/// Match option: z-coda handling.
pub const KIWI_MATCH_Z_CODA: i32 = 1 << 23;
/// Match option: emit compatible jamo.
pub const KIWI_MATCH_COMPATIBLE_JAMO: i32 = 1 << 24;
/// Match option: split saisiot.
pub const KIWI_MATCH_SPLIT_SAISIOT: i32 = 1 << 25;
/// Match option: merge saisiot.
pub const KIWI_MATCH_MERGE_SAISIOT: i32 = 1 << 26;

/// Common default match options.
pub const KIWI_MATCH_ALL: i32 = KIWI_MATCH_URL
    | KIWI_MATCH_EMAIL
    | KIWI_MATCH_HASHTAG
    | KIWI_MATCH_MENTION
    | KIWI_MATCH_SERIAL
    | KIWI_MATCH_Z_CODA;
/// `KIWI_MATCH_ALL` with coda normalization.
pub const KIWI_MATCH_ALL_WITH_NORMALIZING: i32 = KIWI_MATCH_ALL | KIWI_MATCH_NORMALIZE_CODA;

/// Dialect mask: standard language only.
pub const KIWI_DIALECT_STANDARD: i32 = 0;
/// Dialect flag: Gyeonggi.
pub const KIWI_DIALECT_GYEONGGI: i32 = 1 << 0;
/// Dialect flag: Chungcheong.
pub const KIWI_DIALECT_CHUNGCHEONG: i32 = 1 << 1;
/// Dialect flag: Gangwon.
pub const KIWI_DIALECT_GANGWON: i32 = 1 << 2;
/// Dialect flag: Gyeongsang.
pub const KIWI_DIALECT_GYEONGSANG: i32 = 1 << 3;
/// Dialect flag: Jeolla.
pub const KIWI_DIALECT_JEOLLA: i32 = 1 << 4;
/// Dialect flag: Jeju.
pub const KIWI_DIALECT_JEJU: i32 = 1 << 5;
/// Dialect flag: Hwanghae.
pub const KIWI_DIALECT_HWANGHAE: i32 = 1 << 6;
/// Dialect flag: Hamgyeong.
pub const KIWI_DIALECT_HAMGYEONG: i32 = 1 << 7;
/// Dialect flag: Pyeongan.
pub const KIWI_DIALECT_PYEONGAN: i32 = 1 << 8;
/// Dialect flag: archaic expressions.
pub const KIWI_DIALECT_ARCHAIC: i32 = 1 << 9;
/// Dialect mask containing all supported dialect flags.
pub const KIWI_DIALECT_ALL: i32 = (1 << 10) - 1;

pub(crate) const KIWI_RELEASES_API_BASE: &str =
    "https://api.github.com/repos/bab2min/Kiwi/releases";
