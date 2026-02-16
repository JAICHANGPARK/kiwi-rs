use std::cell::RefCell;
use std::collections::BTreeMap;
use std::env;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_float, c_int, c_uint, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::rc::Rc;

use regex::Regex;

use crate::bootstrap::prepare_assets;
use crate::config::{
    KiwiAnalyzeOption, KiwiBuilderHandle, KiwiHandle, KiwiJoinerHandle, KiwiMorphsetHandle,
    KiwiPretokenizedHandle, KiwiResHandle, KiwiSsHandle, KiwiSwTokenizerHandle, KiwiTypoHandle,
    KiwiWsHandle,
};
use crate::constants::{KIWI_MATCH_ALL, KIWI_MATCH_Z_CODA};
use crate::discovery::{default_library_candidates, discover_default_library_path};
use crate::error::{KiwiError, Result};
use crate::model::{
    ExtractedWord, GlobalConfig, MorphemeInfo, MorphemeSense, PreAnalyzedToken, SentenceBoundary,
    SimilarityPair, TokenInfo,
};
use crate::native::{
    api_error, c16str_to_string, clear_kiwi_error, cstr_to_string, read_kiwi_error, DynamicLibrary,
    KiwiApi, KiwiReader, KiwiReaderW, KiwiSimilarityPairRaw, KiwiStreamFactory, LoadedLibrary,
};
use crate::types::{
    AnalysisCandidate, AnalyzeOptions, BuilderConfig, KiwiConfig, Sentence, Token, UserWord,
};
#[derive(Debug, Clone)]
struct ReWordRule {
    pattern: Regex,
    tag: String,
}

#[derive(Clone)]
pub struct KiwiLibrary {
    inner: Rc<LoadedLibrary>,
}

impl KiwiLibrary {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let library = DynamicLibrary::open(path)?;
        Self::from_library(library)
    }

    pub fn load_default() -> Result<Self> {
        let mut errors = Vec::new();

        if let Some(path) = discover_default_library_path() {
            match Self::load(&path) {
                Ok(loaded) => return Ok(loaded),
                Err(error) => errors.push(format!("{}: {}", path.display(), error)),
            }
        }

        for candidate in default_library_candidates() {
            let library = match DynamicLibrary::open(candidate) {
                Ok(library) => library,
                Err(error) => {
                    errors.push(format!("{candidate}: {error}"));
                    continue;
                }
            };

            match Self::from_library(library) {
                Ok(loaded) => return Ok(loaded),
                Err(error) => errors.push(format!("{candidate}: {error}")),
            }
        }

        Err(KiwiError::LibraryLoad(format!(
            "set KIWI_LIBRARY_PATH to the dynamic library path. tried: {}",
            errors.join(" | ")
        )))
    }

    pub fn load_from_env_or_default() -> Result<Self> {
        if let Some(path) = env::var_os("KIWI_LIBRARY_PATH") {
            return Self::load(PathBuf::from(path));
        }
        Self::load_default()
    }

    pub fn supports_builder_init_stream(&self) -> bool {
        self.inner.api.kiwi_builder_init_stream.is_some()
    }

    pub fn supports_utf16_api(&self) -> bool {
        let api = &self.inner.api;
        api.kiwi_analyze_w.is_some()
            && api.kiwi_builder_extract_words_w.is_some()
            && api.kiwi_builder_extract_add_words_w.is_some()
            && api.kiwi_get_morpheme_form_w.is_some()
            && api.kiwi_joiner_get_w.is_some()
            && api.kiwi_morphset_add_w.is_some()
            && api.kiwi_pt_add_token_to_span_w.is_some()
            && api.kiwi_res_form_w.is_some()
            && api.kiwi_res_tag_w.is_some()
            && api.kiwi_split_into_sents_w.is_some()
            && api.kiwi_ws_form_w.is_some()
    }

    pub fn version(&self) -> Result<String> {
        let pointer = unsafe { (self.inner.api.kiwi_version)() };
        if pointer.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_version returned a null pointer",
            ));
        }
        Ok(unsafe { CStr::from_ptr(pointer) }
            .to_string_lossy()
            .to_string())
    }

    pub fn builder(&self, config: BuilderConfig) -> Result<KiwiBuilder> {
        let model_path = match config.model_path.as_ref() {
            Some(path) => Some(CString::new(path.to_string_lossy().to_string())?),
            None => None,
        };
        let model_path_ptr = model_path
            .as_ref()
            .map_or(ptr::null(), |value| value.as_ptr());

        clear_kiwi_error(&self.inner.api);
        let handle = unsafe {
            (self.inner.api.kiwi_builder_init)(
                model_path_ptr,
                config.num_threads as c_int,
                config.build_options as c_int,
                config.enabled_dialects as c_int,
            )
        };

        if handle.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_builder_init returned a null handle",
            ));
        }

        Ok(KiwiBuilder {
            inner: self.inner.clone(),
            handle,
            num_threads: config.num_threads,
            build_options: config.build_options,
            typo_cost_threshold: config.typo_cost_threshold,
        })
    }

    /// Creates a [`KiwiBuilder`] that loads model files through a custom stream factory.
    ///
    /// # Safety
    /// The callback returned by `stream_factory` must provide valid function pointers and
    /// `user_data` for the entire lifetime of the builder initialization call.
    /// Callbacks must follow Kiwi C API contracts for read/seek/close and must not
    /// violate Rust aliasing/thread-safety rules.
    pub unsafe fn builder_from_stream_factory(
        &self,
        stream_factory: KiwiStreamFactory,
        config: BuilderConfig,
    ) -> Result<KiwiBuilder> {
        let init_stream = require_optional_api(
            self.inner.api.kiwi_builder_init_stream,
            "kiwi_builder_init_stream",
        )?;

        clear_kiwi_error(&self.inner.api);
        let handle = init_stream(
            stream_factory,
            config.num_threads as c_int,
            config.build_options as c_int,
            config.enabled_dialects as c_int,
        );

        if handle.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_builder_init_stream returned a null handle",
            ));
        }

        Ok(KiwiBuilder {
            inner: self.inner.clone(),
            handle,
            num_threads: config.num_threads,
            build_options: config.build_options,
            typo_cost_threshold: config.typo_cost_threshold,
        })
    }

    pub fn typo(&self) -> Result<KiwiTypo> {
        KiwiTypo::new(self)
    }

    pub fn basic_typo(&self) -> Result<KiwiTypo> {
        KiwiTypo::basic(self)
    }

    pub fn default_typo_set(&self, typo_set: i32) -> Result<KiwiTypo> {
        KiwiTypo::default_set(self, typo_set)
    }

    fn from_library(library: DynamicLibrary) -> Result<Self> {
        let api = unsafe { KiwiApi::load(&library)? };
        Ok(Self {
            inner: Rc::new(LoadedLibrary {
                _library: library,
                api,
            }),
        })
    }
}

pub struct KiwiBuilder {
    inner: Rc<LoadedLibrary>,
    handle: KiwiBuilderHandle,
    num_threads: i32,
    build_options: i32,
    typo_cost_threshold: f32,
}

impl KiwiBuilder {
    pub fn add_user_word(&mut self, word: &str, tag: &str, score: f32) -> Result<()> {
        let word_c = CString::new(word)?;
        let tag_c = CString::new(tag)?;

        clear_kiwi_error(&self.inner.api);
        let result = unsafe {
            (self.inner.api.kiwi_builder_add_word)(
                self.handle,
                word_c.as_ptr(),
                tag_c.as_ptr(),
                score as c_float,
            )
        };
        if result < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_builder_add_word returned an error",
            ));
        }
        Ok(())
    }

    pub fn add_alias_word(
        &mut self,
        alias: &str,
        tag: &str,
        score: f32,
        orig_word: &str,
    ) -> Result<()> {
        let add_alias = require_optional_api(
            self.inner.api.kiwi_builder_add_alias_word,
            "kiwi_builder_add_alias_word",
        )?;

        let alias_c = CString::new(alias)?;
        let tag_c = CString::new(tag)?;
        let orig_word_c = CString::new(orig_word)?;

        clear_kiwi_error(&self.inner.api);
        let result = unsafe {
            add_alias(
                self.handle,
                alias_c.as_ptr(),
                tag_c.as_ptr(),
                score as c_float,
                orig_word_c.as_ptr(),
            )
        };

        if result < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_builder_add_alias_word returned an error",
            ));
        }

        Ok(())
    }

    pub fn add_pre_analyzed_word(
        &mut self,
        form: &str,
        analyzed: &[PreAnalyzedToken],
        score: f32,
    ) -> Result<()> {
        let add_pre = require_optional_api(
            self.inner.api.kiwi_builder_add_pre_analyzed_word,
            "kiwi_builder_add_pre_analyzed_word",
        )?;

        if analyzed.is_empty() {
            return Err(KiwiError::InvalidArgument(
                "Pre-analyzed token list must not be empty".to_string(),
            ));
        }

        if analyzed.len() > c_int::MAX as usize {
            return Err(KiwiError::InvalidArgument(format!(
                "Pre-analyzed token count must be <= {}",
                c_int::MAX
            )));
        }

        let form_c = CString::new(form)?;

        let analyzed_forms: Vec<CString> = analyzed
            .iter()
            .map(|token| CString::new(token.form.clone()))
            .collect::<std::result::Result<_, _>>()?;
        let analyzed_tags: Vec<CString> = analyzed
            .iter()
            .map(|token| CString::new(token.tag.clone()))
            .collect::<std::result::Result<_, _>>()?;

        let analyzed_form_ptrs: Vec<*const i8> =
            analyzed_forms.iter().map(|value| value.as_ptr()).collect();
        let analyzed_tag_ptrs: Vec<*const i8> =
            analyzed_tags.iter().map(|value| value.as_ptr()).collect();

        let has_any_span = analyzed
            .iter()
            .any(|token| token.begin.is_some() || token.end.is_some());
        let has_all_spans = analyzed
            .iter()
            .all(|token| token.begin.is_some() && token.end.is_some());
        if has_any_span && !has_all_spans {
            return Err(KiwiError::InvalidArgument(
                "All pre-analyzed tokens must either provide both begin/end or neither".to_string(),
            ));
        }

        let mut positions = Vec::<c_int>::new();
        if has_all_spans {
            positions.reserve(analyzed.len() * 2);
            for token in analyzed {
                let begin = token.begin.expect("checked above");
                let end = token.end.expect("checked above");
                if begin > end {
                    return Err(KiwiError::InvalidArgument(format!(
                        "Invalid pre-analyzed token span: begin ({begin}) > end ({end})"
                    )));
                }
                if end > c_int::MAX as usize {
                    return Err(KiwiError::InvalidArgument(format!(
                        "Pre-analyzed token span end must be <= {}",
                        c_int::MAX
                    )));
                }
                positions.push(begin as c_int);
                positions.push(end as c_int);
            }
        }

        clear_kiwi_error(&self.inner.api);
        let result = unsafe {
            add_pre(
                self.handle,
                form_c.as_ptr(),
                analyzed.len() as c_int,
                analyzed_form_ptrs.as_ptr(),
                analyzed_tag_ptrs.as_ptr(),
                score as c_float,
                if has_all_spans {
                    positions.as_ptr()
                } else {
                    ptr::null()
                },
            )
        };

        if result < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_builder_add_pre_analyzed_word returned an error",
            ));
        }

        Ok(())
    }

    pub fn load_user_dictionary(&mut self, dict_path: impl AsRef<Path>) -> Result<usize> {
        let load_dict = require_optional_api(
            self.inner.api.kiwi_builder_load_dict,
            "kiwi_builder_load_dict",
        )?;

        let dict_path_c = CString::new(dict_path.as_ref().to_string_lossy().to_string())?;

        clear_kiwi_error(&self.inner.api);
        let result = unsafe { load_dict(self.handle, dict_path_c.as_ptr()) };
        if result < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_builder_load_dict returned an error",
            ));
        }

        Ok(result as usize)
    }

    pub fn add_rule<F>(&mut self, tag: &str, replacer: F, score: f32) -> Result<usize>
    where
        F: Fn(&str) -> String + 'static,
    {
        let add_rule = require_optional_api(
            self.inner.api.kiwi_builder_add_rule,
            "kiwi_builder_add_rule",
        )?;

        let tag_c = CString::new(tag)?;
        let mut context = RuleCallbackContext {
            replacer: Box::new(replacer),
        };

        clear_kiwi_error(&self.inner.api);
        let result = unsafe {
            add_rule(
                self.handle,
                tag_c.as_ptr(),
                rule_replacer_callback,
                (&mut context as *mut RuleCallbackContext).cast::<c_void>(),
                score as c_float,
            )
        };

        if result < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_builder_add_rule returned an error",
            ));
        }

        Ok(result as usize)
    }

    pub fn add_re_rule(
        &mut self,
        tag: &str,
        pattern: &str,
        replacement: &str,
        score: f32,
    ) -> Result<usize> {
        let pattern = Regex::new(pattern).map_err(|error| {
            KiwiError::InvalidArgument(format!("invalid regex pattern for add_re_rule: {error}"))
        })?;
        let replacement = replacement.to_string();
        self.add_rule(
            tag,
            move |input| {
                pattern
                    .replace_all(input, replacement.as_str())
                    .into_owned()
            },
            score,
        )
    }

    pub fn extract_words<I, S>(
        &mut self,
        texts: I,
        min_cnt: i32,
        max_word_len: i32,
        min_score: f32,
        pos_threshold: f32,
    ) -> Result<Vec<ExtractedWord>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let extract_fn = require_optional_api(
            self.inner.api.kiwi_builder_extract_words,
            "kiwi_builder_extract_words",
        )?;
        self.extract_words_inner(
            extract_fn,
            texts,
            min_cnt,
            max_word_len,
            min_score,
            pos_threshold,
        )
    }

    pub fn extract_add_words<I, S>(
        &mut self,
        texts: I,
        min_cnt: i32,
        max_word_len: i32,
        min_score: f32,
        pos_threshold: f32,
    ) -> Result<Vec<ExtractedWord>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let extract_fn = require_optional_api(
            self.inner.api.kiwi_builder_extract_add_words,
            "kiwi_builder_extract_add_words",
        )?;
        self.extract_words_inner(
            extract_fn,
            texts,
            min_cnt,
            max_word_len,
            min_score,
            pos_threshold,
        )
    }

    pub fn extract_words_utf16<I, S>(
        &mut self,
        texts: I,
        min_cnt: i32,
        max_word_len: i32,
        min_score: f32,
        pos_threshold: f32,
    ) -> Result<Vec<ExtractedWord>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let extract_fn = require_optional_api(
            self.inner.api.kiwi_builder_extract_words_w,
            "kiwi_builder_extract_words_w",
        )?;
        self.extract_words_inner_utf16(
            extract_fn,
            texts,
            min_cnt,
            max_word_len,
            min_score,
            pos_threshold,
        )
    }

    pub fn extract_add_words_utf16<I, S>(
        &mut self,
        texts: I,
        min_cnt: i32,
        max_word_len: i32,
        min_score: f32,
        pos_threshold: f32,
    ) -> Result<Vec<ExtractedWord>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let extract_fn = require_optional_api(
            self.inner.api.kiwi_builder_extract_add_words_w,
            "kiwi_builder_extract_add_words_w",
        )?;
        self.extract_words_inner_utf16(
            extract_fn,
            texts,
            min_cnt,
            max_word_len,
            min_score,
            pos_threshold,
        )
    }

    fn extract_words_inner<I, S>(
        &mut self,
        extract_fn: unsafe extern "C" fn(
            KiwiBuilderHandle,
            KiwiReader,
            *mut c_void,
            c_int,
            c_int,
            c_float,
            c_float,
        ) -> KiwiWsHandle,
        texts: I,
        min_cnt: i32,
        max_word_len: i32,
        min_score: f32,
        pos_threshold: f32,
    ) -> Result<Vec<ExtractedWord>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        if min_cnt < 1 {
            return Err(KiwiError::InvalidArgument(
                "min_cnt must be >= 1".to_string(),
            ));
        }
        if max_word_len < 1 {
            return Err(KiwiError::InvalidArgument(
                "max_word_len must be >= 1".to_string(),
            ));
        }

        let lines: Vec<CString> = texts
            .into_iter()
            .map(|value| CString::new(value.as_ref()))
            .collect::<std::result::Result<_, _>>()?;
        let mut context = ReaderContext { lines };

        clear_kiwi_error(&self.inner.api);
        let ws_handle = unsafe {
            extract_fn(
                self.handle,
                reader_callback,
                (&mut context as *mut ReaderContext).cast::<c_void>(),
                min_cnt as c_int,
                max_word_len as c_int,
                min_score as c_float,
                pos_threshold as c_float,
            )
        };

        if ws_handle.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_builder_extract_words returned a null handle",
            ));
        }

        let result = KiwiWordSetResult {
            inner: self.inner.clone(),
            handle: ws_handle,
        };
        result.to_vec()
    }

    fn extract_words_inner_utf16<I, S>(
        &mut self,
        extract_fn: unsafe extern "C" fn(
            KiwiBuilderHandle,
            KiwiReaderW,
            *mut c_void,
            c_int,
            c_int,
            c_float,
            c_float,
        ) -> KiwiWsHandle,
        texts: I,
        min_cnt: i32,
        max_word_len: i32,
        min_score: f32,
        pos_threshold: f32,
    ) -> Result<Vec<ExtractedWord>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        if min_cnt < 1 {
            return Err(KiwiError::InvalidArgument(
                "min_cnt must be >= 1".to_string(),
            ));
        }
        if max_word_len < 1 {
            return Err(KiwiError::InvalidArgument(
                "max_word_len must be >= 1".to_string(),
            ));
        }

        let lines: Vec<Vec<u16>> = texts
            .into_iter()
            .map(|value| value.as_ref().encode_utf16().collect())
            .collect();
        let mut context = ReaderWContext { lines };

        clear_kiwi_error(&self.inner.api);
        let ws_handle = unsafe {
            extract_fn(
                self.handle,
                reader_w_callback,
                (&mut context as *mut ReaderWContext).cast::<c_void>(),
                min_cnt as c_int,
                max_word_len as c_int,
                min_score as c_float,
                pos_threshold as c_float,
            )
        };

        if ws_handle.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_builder_extract_words_w returned a null handle",
            ));
        }

        let result = KiwiWordSetResult {
            inner: self.inner.clone(),
            handle: ws_handle,
        };
        result.to_vec_utf16()
    }

    pub fn add_user_words<I>(&mut self, words: I) -> Result<()>
    where
        I: IntoIterator<Item = UserWord>,
    {
        for word in words {
            self.add_user_word(&word.word, &word.tag, word.score)?;
        }
        Ok(())
    }

    pub fn build(self) -> Result<Kiwi> {
        self.build_with_default_options(AnalyzeOptions::default())
    }

    pub fn build_with_default_options(self, default_options: AnalyzeOptions) -> Result<Kiwi> {
        self.build_with_typo_and_default_options(None, default_options)
    }

    pub fn build_with_typo(self, typo: &KiwiTypo) -> Result<Kiwi> {
        self.build_with_typo_and_default_options(Some(typo), AnalyzeOptions::default())
    }

    pub fn build_with_typo_and_default_options(
        self,
        typo: Option<&KiwiTypo>,
        default_options: AnalyzeOptions,
    ) -> Result<Kiwi> {
        let typo_handle = match typo {
            Some(value) => {
                if !Rc::ptr_eq(&self.inner, &value.inner) {
                    return Err(KiwiError::InvalidArgument(
                        "KiwiTypo was created from a different Kiwi library".to_string(),
                    ));
                }
                value.handle
            }
            None => ptr::null_mut(),
        };

        clear_kiwi_error(&self.inner.api);
        let handle = unsafe {
            (self.inner.api.kiwi_builder_build)(
                self.handle,
                typo_handle,
                self.typo_cost_threshold as c_float,
            )
        };
        if handle.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_builder_build returned a null handle",
            ));
        }
        Ok(Kiwi {
            inner: self.inner.clone(),
            handle,
            default_analyze_options: default_options,
            num_workers: self.num_threads,
            model_type: self.build_options,
            typo_cost_threshold: self.typo_cost_threshold,
            re_word_rules: RefCell::new(Vec::new()),
        })
    }
}

impl Drop for KiwiBuilder {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        unsafe {
            (self.inner.api.kiwi_builder_close)(self.handle);
        }
        self.handle = ptr::null_mut();
    }
}

pub struct KiwiTypo {
    inner: Rc<LoadedLibrary>,
    handle: KiwiTypoHandle,
    owned: bool,
}

impl KiwiTypo {
    pub fn new(library: &KiwiLibrary) -> Result<Self> {
        let init = require_optional_api(library.inner.api.kiwi_typo_init, "kiwi_typo_init")?;
        clear_kiwi_error(&library.inner.api);
        let handle = unsafe { init() };
        Self::from_handle(
            library.inner.clone(),
            handle,
            true,
            "kiwi_typo_init returned a null handle",
        )
    }

    pub fn basic(library: &KiwiLibrary) -> Result<Self> {
        let get_basic =
            require_optional_api(library.inner.api.kiwi_typo_get_basic, "kiwi_typo_get_basic")?;
        clear_kiwi_error(&library.inner.api);
        let handle = unsafe { get_basic() };
        Self::from_handle(
            library.inner.clone(),
            handle,
            false,
            "kiwi_typo_get_basic returned a null handle",
        )
    }

    pub fn default_set(library: &KiwiLibrary, typo_set: i32) -> Result<Self> {
        let get_default = require_optional_api(
            library.inner.api.kiwi_typo_get_default,
            "kiwi_typo_get_default",
        )?;
        clear_kiwi_error(&library.inner.api);
        let handle = unsafe { get_default(typo_set as c_int) };
        Self::from_handle(
            library.inner.clone(),
            handle,
            false,
            "kiwi_typo_get_default returned a null handle",
        )
    }

    pub fn copy(&self) -> Result<Self> {
        let copy_fn = require_optional_api(self.inner.api.kiwi_typo_copy, "kiwi_typo_copy")?;
        clear_kiwi_error(&self.inner.api);
        let handle = unsafe { copy_fn(self.handle) };
        Self::from_handle(
            self.inner.clone(),
            handle,
            true,
            "kiwi_typo_copy returned a null handle",
        )
    }

    pub fn add(&mut self, orig: &[&str], error: &[&str], cost: f32, condition: i32) -> Result<()> {
        if orig.is_empty() || error.is_empty() {
            return Err(KiwiError::InvalidArgument(
                "orig and error must not be empty".to_string(),
            ));
        }
        if orig.len() > c_int::MAX as usize || error.len() > c_int::MAX as usize {
            return Err(KiwiError::InvalidArgument(format!(
                "orig/error length must be <= {}",
                c_int::MAX
            )));
        }

        let add_fn = require_optional_api(self.inner.api.kiwi_typo_add, "kiwi_typo_add")?;
        let orig_c: Vec<CString> = orig
            .iter()
            .map(|value| CString::new(*value))
            .collect::<std::result::Result<_, _>>()?;
        let error_c: Vec<CString> = error
            .iter()
            .map(|value| CString::new(*value))
            .collect::<std::result::Result<_, _>>()?;
        let orig_ptrs: Vec<*const c_char> = orig_c.iter().map(|value| value.as_ptr()).collect();
        let error_ptrs: Vec<*const c_char> = error_c.iter().map(|value| value.as_ptr()).collect();

        clear_kiwi_error(&self.inner.api);
        let result = unsafe {
            add_fn(
                self.handle,
                orig_ptrs.as_ptr(),
                orig_ptrs.len() as c_int,
                error_ptrs.as_ptr(),
                error_ptrs.len() as c_int,
                cost,
                condition as c_int,
            )
        };
        if result != 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_typo_add returned an error",
            ));
        }
        Ok(())
    }

    pub fn update(&mut self, src: &KiwiTypo) -> Result<()> {
        if !Rc::ptr_eq(&self.inner, &src.inner) {
            return Err(KiwiError::InvalidArgument(
                "source typo set was created from a different Kiwi library".to_string(),
            ));
        }
        let update_fn = require_optional_api(self.inner.api.kiwi_typo_update, "kiwi_typo_update")?;
        clear_kiwi_error(&self.inner.api);
        let result = unsafe { update_fn(self.handle, src.handle) };
        if result != 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_typo_update returned an error",
            ));
        }
        Ok(())
    }

    pub fn scale_cost(&mut self, scale: f32) -> Result<()> {
        let scale_fn =
            require_optional_api(self.inner.api.kiwi_typo_scale_cost, "kiwi_typo_scale_cost")?;
        clear_kiwi_error(&self.inner.api);
        let result = unsafe { scale_fn(self.handle, scale) };
        if result != 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_typo_scale_cost returned an error",
            ));
        }
        Ok(())
    }

    pub fn set_continual_typo_cost(&mut self, threshold: f32) -> Result<()> {
        let set_fn = require_optional_api(
            self.inner.api.kiwi_typo_set_continual_typo_cost,
            "kiwi_typo_set_continual_typo_cost",
        )?;
        clear_kiwi_error(&self.inner.api);
        let result = unsafe { set_fn(self.handle, threshold) };
        if result != 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_typo_set_continual_typo_cost returned an error",
            ));
        }
        Ok(())
    }

    pub fn set_lengthening_typo_cost(&mut self, threshold: f32) -> Result<()> {
        let set_fn = require_optional_api(
            self.inner.api.kiwi_typo_set_lengthening_typo_cost,
            "kiwi_typo_set_lengthening_typo_cost",
        )?;
        clear_kiwi_error(&self.inner.api);
        let result = unsafe { set_fn(self.handle, threshold) };
        if result != 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_typo_set_lengthening_typo_cost returned an error",
            ));
        }
        Ok(())
    }

    fn from_handle(
        inner: Rc<LoadedLibrary>,
        handle: KiwiTypoHandle,
        owned: bool,
        fallback: &'static str,
    ) -> Result<Self> {
        if handle.is_null() {
            return Err(api_error(&inner.api, fallback));
        }
        Ok(Self {
            inner,
            handle,
            owned,
        })
    }
}

impl Drop for KiwiTypo {
    fn drop(&mut self) {
        if self.handle.is_null() || !self.owned {
            return;
        }
        if let Some(close) = self.inner.api.kiwi_typo_close {
            unsafe {
                close(self.handle);
            }
        }
        self.handle = ptr::null_mut();
    }
}

pub struct MorphemeSet {
    inner: Rc<LoadedLibrary>,
    handle: KiwiMorphsetHandle,
}

impl MorphemeSet {
    pub fn add(&mut self, form: &str, tag: Option<&str>) -> Result<usize> {
        let add = require_optional_api(self.inner.api.kiwi_morphset_add, "kiwi_morphset_add")?;

        let form_c = CString::new(form)?;
        let tag_c = match tag {
            Some(tag) => Some(CString::new(tag)?),
            None => None,
        };

        clear_kiwi_error(&self.inner.api);
        let result = unsafe {
            add(
                self.handle,
                form_c.as_ptr(),
                tag_c.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
            )
        };

        if result < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_morphset_add returned an error",
            ));
        }

        Ok(result as usize)
    }

    pub fn add_utf16(&mut self, form: &[u16], tag: Option<&str>) -> Result<usize> {
        let add = require_optional_api(self.inner.api.kiwi_morphset_add_w, "kiwi_morphset_add_w")?;
        let form_c16 = to_c16_null_terminated(form)?;
        let tag_c = match tag {
            Some(tag) => Some(CString::new(tag)?),
            None => None,
        };

        clear_kiwi_error(&self.inner.api);
        let result = unsafe {
            add(
                self.handle,
                form_c16.as_ptr(),
                tag_c.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
            )
        };

        if result < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_morphset_add_w returned an error",
            ));
        }

        Ok(result as usize)
    }
}

impl Drop for MorphemeSet {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        if let Some(close) = self.inner.api.kiwi_morphset_close {
            unsafe {
                close(self.handle);
            }
        }
        self.handle = ptr::null_mut();
    }
}

pub struct Pretokenized {
    inner: Rc<LoadedLibrary>,
    handle: KiwiPretokenizedHandle,
}

impl Pretokenized {
    pub fn add_span(&mut self, begin: usize, end: usize) -> Result<i32> {
        let add_span = require_optional_api(self.inner.api.kiwi_pt_add_span, "kiwi_pt_add_span")?;
        if begin > end {
            return Err(KiwiError::InvalidArgument(format!(
                "Invalid pretokenized span: begin ({begin}) > end ({end})"
            )));
        }
        if end > c_int::MAX as usize {
            return Err(KiwiError::InvalidArgument(format!(
                "Pretokenized span end must be <= {}",
                c_int::MAX
            )));
        }

        clear_kiwi_error(&self.inner.api);
        let span_id = unsafe { add_span(self.handle, begin as c_int, end as c_int) };
        if span_id < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_pt_add_span returned an error",
            ));
        }
        Ok(span_id)
    }

    pub fn add_token_to_span(
        &mut self,
        span_id: i32,
        form: &str,
        tag: &str,
        begin: usize,
        end: usize,
    ) -> Result<()> {
        let add_token = require_optional_api(
            self.inner.api.kiwi_pt_add_token_to_span,
            "kiwi_pt_add_token_to_span",
        )?;
        if begin > end {
            return Err(KiwiError::InvalidArgument(format!(
                "Invalid token span: begin ({begin}) > end ({end})"
            )));
        }
        if end > c_int::MAX as usize {
            return Err(KiwiError::InvalidArgument(format!(
                "Pretokenized token span end must be <= {}",
                c_int::MAX
            )));
        }

        let form_c = CString::new(form)?;
        let tag_c = CString::new(tag)?;

        clear_kiwi_error(&self.inner.api);
        let result = unsafe {
            add_token(
                self.handle,
                span_id as c_int,
                form_c.as_ptr(),
                tag_c.as_ptr(),
                begin as c_int,
                end as c_int,
            )
        };
        if result != 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_pt_add_token_to_span returned an error",
            ));
        }
        Ok(())
    }

    pub fn add_token_to_span_utf16(
        &mut self,
        span_id: i32,
        form: &[u16],
        tag: &str,
        begin: usize,
        end: usize,
    ) -> Result<()> {
        let add_token = require_optional_api(
            self.inner.api.kiwi_pt_add_token_to_span_w,
            "kiwi_pt_add_token_to_span_w",
        )?;
        if begin > end {
            return Err(KiwiError::InvalidArgument(format!(
                "Invalid token span: begin ({begin}) > end ({end})"
            )));
        }
        if end > c_int::MAX as usize {
            return Err(KiwiError::InvalidArgument(format!(
                "Pretokenized token span end must be <= {}",
                c_int::MAX
            )));
        }

        let form_c16 = to_c16_null_terminated(form)?;
        let tag_c = CString::new(tag)?;

        clear_kiwi_error(&self.inner.api);
        let result = unsafe {
            add_token(
                self.handle,
                span_id as c_int,
                form_c16.as_ptr(),
                tag_c.as_ptr(),
                begin as c_int,
                end as c_int,
            )
        };
        if result != 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_pt_add_token_to_span_w returned an error",
            ));
        }
        Ok(())
    }
}

impl Drop for Pretokenized {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        if let Some(close) = self.inner.api.kiwi_pt_close {
            unsafe {
                close(self.handle);
            }
        }
        self.handle = ptr::null_mut();
    }
}

pub struct Kiwi {
    inner: Rc<LoadedLibrary>,
    handle: KiwiHandle,
    default_analyze_options: AnalyzeOptions,
    num_workers: i32,
    model_type: i32,
    typo_cost_threshold: f32,
    re_word_rules: RefCell<Vec<ReWordRule>>,
}

impl Kiwi {
    /// Initializes Kiwi. If local assets are missing, this method attempts to
    /// download a matching library/model pair into a cache directory.
    ///
    /// - Version source: `KIWI_RS_VERSION` env var or `latest`.
    /// - Cache root: `KIWI_RS_CACHE_DIR` env var or OS-specific cache directory.
    pub fn init() -> Result<Self> {
        let version = env::var("KIWI_RS_VERSION").unwrap_or_else(|_| "latest".to_string());
        Self::init_with_version(&version)
    }

    /// Same as [`Kiwi::init`] but allows explicit release tag selection
    /// (for example: `v0.22.2` or `latest`).
    pub fn init_with_version(version: &str) -> Result<Self> {
        match Self::new() {
            Ok(kiwi) => Ok(kiwi),
            Err(initial_error) => {
                let assets = prepare_assets(version).map_err(|download_error| {
                    KiwiError::Bootstrap(format!(
                        "initialization failed ({initial_error}); \
                         auto-download also failed ({download_error})"
                    ))
                })?;

                let config = KiwiConfig::default()
                    .with_library_path(&assets.library_path)
                    .with_model_path(&assets.model_path);

                Self::from_config(config).map_err(|error| {
                    KiwiError::Bootstrap(format!(
                        "assets downloaded to {} (tag {}). \
                         initialization still failed: {}",
                        assets.cache_dir.display(),
                        assets.tag_name,
                        error
                    ))
                })
            }
        }
    }

    pub fn new() -> Result<Self> {
        Self::from_config(KiwiConfig::default())
    }

    pub fn init_direct(
        model_path: Option<&Path>,
        num_threads: i32,
        build_options: i32,
    ) -> Result<Self> {
        let library = KiwiLibrary::load_from_env_or_default()?;
        let init = require_optional_api(library.inner.api.kiwi_init, "kiwi_init")?;
        let model_path_c = model_path
            .map(|path| CString::new(path.to_string_lossy().to_string()))
            .transpose()?;
        let model_path_ptr = model_path_c
            .as_ref()
            .map_or(ptr::null(), |value| value.as_ptr());

        clear_kiwi_error(&library.inner.api);
        let handle = unsafe { init(model_path_ptr, num_threads as c_int, build_options as c_int) };
        if handle.is_null() {
            return Err(api_error(
                &library.inner.api,
                "kiwi_init returned a null handle",
            ));
        }

        Ok(Self {
            inner: library.inner,
            handle,
            default_analyze_options: AnalyzeOptions::default(),
            num_workers: num_threads,
            model_type: build_options,
            typo_cost_threshold: 0.0,
            re_word_rules: RefCell::new(Vec::new()),
        })
    }

    pub fn with_model_path(model_path: impl AsRef<Path>) -> Result<Self> {
        Self::from_config(KiwiConfig::default().with_model_path(model_path))
    }

    pub fn from_config(config: KiwiConfig) -> Result<Self> {
        let library = match config.library_path {
            Some(path) => KiwiLibrary::load(path)?,
            None => KiwiLibrary::load_from_env_or_default()?,
        };

        let mut builder = library.builder(config.builder)?;
        for word in config.user_words {
            builder.add_user_word(&word.word, &word.tag, word.score)?;
        }

        builder.build_with_default_options(config.default_analyze_options)
    }

    pub fn supports_analyze_mw(&self) -> bool {
        self.inner.api.kiwi_analyze_mw.is_some()
    }

    pub fn supports_utf16_api(&self) -> bool {
        let library = KiwiLibrary {
            inner: self.inner.clone(),
        };
        library.supports_utf16_api()
    }

    pub fn typo(&self) -> Result<KiwiTypo> {
        KiwiTypo::new(&KiwiLibrary {
            inner: self.inner.clone(),
        })
    }

    pub fn basic_typo(&self) -> Result<KiwiTypo> {
        KiwiTypo::basic(&KiwiLibrary {
            inner: self.inner.clone(),
        })
    }

    pub fn default_typo_set(&self, typo_set: i32) -> Result<KiwiTypo> {
        KiwiTypo::default_set(
            &KiwiLibrary {
                inner: self.inner.clone(),
            },
            typo_set,
        )
    }

    pub fn library_version(&self) -> Result<String> {
        let pointer = unsafe { (self.inner.api.kiwi_version)() };
        if pointer.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_version returned a null pointer",
            ));
        }
        Ok(unsafe { CStr::from_ptr(pointer) }
            .to_string_lossy()
            .to_string())
    }

    pub fn global_config(&self) -> Result<GlobalConfig> {
        let get_config = require_optional_api(
            self.inner.api.kiwi_get_global_config,
            "kiwi_get_global_config",
        )?;

        clear_kiwi_error(&self.inner.api);
        let config = unsafe { get_config(self.handle) };
        if read_kiwi_error(&self.inner.api).is_some() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_get_global_config returned an error",
            ));
        }
        Ok(config.into())
    }

    pub fn set_global_config(&mut self, config: GlobalConfig) -> Result<()> {
        let set_config = require_optional_api(
            self.inner.api.kiwi_set_global_config,
            "kiwi_set_global_config",
        )?;

        clear_kiwi_error(&self.inner.api);
        unsafe {
            set_config(self.handle, config.into());
        }
        if read_kiwi_error(&self.inner.api).is_some() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_set_global_config returned an error",
            ));
        }
        Ok(())
    }

    pub fn set_option(&mut self, option: i32, value: i32) -> Result<()> {
        let set_option = require_optional_api(self.inner.api.kiwi_set_option, "kiwi_set_option")?;
        clear_kiwi_error(&self.inner.api);
        unsafe {
            set_option(self.handle, option as c_int, value as c_int);
        }
        if read_kiwi_error(&self.inner.api).is_some() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_set_option returned an error",
            ));
        }
        Ok(())
    }

    pub fn get_option(&self, option: i32) -> Result<i32> {
        let get_option = require_optional_api(self.inner.api.kiwi_get_option, "kiwi_get_option")?;
        clear_kiwi_error(&self.inner.api);
        let value = unsafe { get_option(self.handle, option as c_int) };
        if read_kiwi_error(&self.inner.api).is_some() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_get_option returned an error",
            ));
        }
        Ok(value as i32)
    }

    pub fn set_option_f(&mut self, option: i32, value: f32) -> Result<()> {
        let set_option_f =
            require_optional_api(self.inner.api.kiwi_set_option_f, "kiwi_set_option_f")?;
        clear_kiwi_error(&self.inner.api);
        unsafe {
            set_option_f(self.handle, option as c_int, value as c_float);
        }
        if read_kiwi_error(&self.inner.api).is_some() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_set_option_f returned an error",
            ));
        }
        Ok(())
    }

    pub fn get_option_f(&self, option: i32) -> Result<f32> {
        let get_option_f =
            require_optional_api(self.inner.api.kiwi_get_option_f, "kiwi_get_option_f")?;
        clear_kiwi_error(&self.inner.api);
        let value = unsafe { get_option_f(self.handle, option as c_int) };
        if read_kiwi_error(&self.inner.api).is_some() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_get_option_f returned an error",
            ));
        }
        Ok(value)
    }

    pub fn default_analyze_options(&self) -> AnalyzeOptions {
        self.default_analyze_options
    }

    pub fn set_default_analyze_options(&mut self, options: AnalyzeOptions) {
        self.default_analyze_options = options;
    }

    pub fn num_workers(&self) -> i32 {
        self.num_workers
    }

    pub fn model_type(&self) -> i32 {
        self.model_type
    }

    pub fn typo_cost_threshold(&self) -> f32 {
        self.typo_cost_threshold
    }

    pub fn cutoff_threshold(&self) -> Result<f32> {
        Ok(self.global_config()?.cut_off_threshold)
    }

    pub fn set_cutoff_threshold(&mut self, value: f32) -> Result<()> {
        let mut config = self.global_config()?;
        config.cut_off_threshold = value;
        self.set_global_config(config)
    }

    pub fn integrate_allomorph(&self) -> Result<bool> {
        Ok(self.global_config()?.integrate_allomorph)
    }

    pub fn set_integrate_allomorph(&mut self, value: bool) -> Result<()> {
        let mut config = self.global_config()?;
        config.integrate_allomorph = value;
        self.set_global_config(config)
    }

    pub fn space_penalty(&self) -> Result<f32> {
        Ok(self.global_config()?.space_penalty)
    }

    pub fn set_space_penalty(&mut self, value: f32) -> Result<()> {
        let mut config = self.global_config()?;
        config.space_penalty = value;
        self.set_global_config(config)
    }

    pub fn space_tolerance(&self) -> Result<u32> {
        Ok(self.global_config()?.space_tolerance)
    }

    pub fn set_space_tolerance(&mut self, value: u32) -> Result<()> {
        let mut config = self.global_config()?;
        config.space_tolerance = value;
        self.set_global_config(config)
    }

    pub fn max_unk_form_size(&self) -> Result<u32> {
        Ok(self.global_config()?.max_unk_form_size)
    }

    pub fn set_max_unk_form_size(&mut self, value: u32) -> Result<()> {
        let mut config = self.global_config()?;
        config.max_unk_form_size = value;
        self.set_global_config(config)
    }

    pub fn typo_cost_weight(&self) -> Result<f32> {
        Ok(self.global_config()?.typo_cost_weight)
    }

    pub fn set_typo_cost_weight(&mut self, value: f32) -> Result<()> {
        let mut config = self.global_config()?;
        config.typo_cost_weight = value;
        self.set_global_config(config)
    }

    pub fn add_re_word(&self, pattern: &str, tag: &str) -> Result<()> {
        let compiled = Regex::new(pattern).map_err(|error| {
            KiwiError::InvalidArgument(format!("invalid regex pattern for add_re_word: {error}"))
        })?;
        let mut rules = self.re_word_rules.borrow_mut();
        rules.push(ReWordRule {
            pattern: compiled,
            tag: tag.to_string(),
        });
        Ok(())
    }

    pub fn clear_re_words(&self) {
        self.re_word_rules.borrow_mut().clear();
    }

    pub fn analyze(&self, text: &str) -> Result<Vec<AnalysisCandidate>> {
        self.analyze_with_options(text, self.default_analyze_options)
    }

    pub fn analyze_top_n(&self, text: &str, top_n: usize) -> Result<Vec<AnalysisCandidate>> {
        self.analyze_with_options(text, self.default_analyze_options.with_top_n(top_n))
    }

    pub fn analyze_with_options(
        &self,
        text: &str,
        options: AnalyzeOptions,
    ) -> Result<Vec<AnalysisCandidate>> {
        self.analyze_with_overrides(text, options, None, None)
    }

    pub fn analyze_with_blocklist(
        &self,
        text: &str,
        options: AnalyzeOptions,
        blocklist: Option<&MorphemeSet>,
    ) -> Result<Vec<AnalysisCandidate>> {
        self.analyze_with_overrides(text, options, blocklist, None)
    }

    pub fn analyze_with_pretokenized(
        &self,
        text: &str,
        options: AnalyzeOptions,
        pretokenized: Option<&Pretokenized>,
    ) -> Result<Vec<AnalysisCandidate>> {
        self.analyze_with_overrides(text, options, None, pretokenized)
    }

    pub fn analyze_with_blocklist_and_pretokenized(
        &self,
        text: &str,
        options: AnalyzeOptions,
        blocklist: Option<&MorphemeSet>,
        pretokenized: Option<&Pretokenized>,
    ) -> Result<Vec<AnalysisCandidate>> {
        self.analyze_with_overrides(text, options, blocklist, pretokenized)
    }

    fn analyze_with_overrides(
        &self,
        text: &str,
        options: AnalyzeOptions,
        blocklist: Option<&MorphemeSet>,
        pretokenized: Option<&Pretokenized>,
    ) -> Result<Vec<AnalysisCandidate>> {
        let top_n = options.validated_top_n()?;
        let text_c = CString::new(text)?;

        let blocklist_handle = match blocklist {
            Some(set) => {
                if !Rc::ptr_eq(&self.inner, &set.inner) {
                    return Err(KiwiError::InvalidArgument(
                        "MorphemeSet was created from a different Kiwi instance".to_string(),
                    ));
                }
                set.handle
            }
            None => ptr::null_mut(),
        };

        if pretokenized.is_some() && !self.re_word_rules.borrow().is_empty() {
            return Err(KiwiError::InvalidArgument(
                "explicit pretokenized input cannot be combined with add_re_word rules yet"
                    .to_string(),
            ));
        }

        let reword_pretokenized = if pretokenized.is_none() {
            self.build_re_word_pretokenized(text)?
        } else {
            None
        };
        let pretokenized_handle = match pretokenized {
            Some(value) => {
                if !Rc::ptr_eq(&self.inner, &value.inner) {
                    return Err(KiwiError::InvalidArgument(
                        "Pretokenized was created from a different Kiwi instance".to_string(),
                    ));
                }
                value.handle
            }
            None => reword_pretokenized
                .as_ref()
                .map_or(ptr::null_mut(), |value| value.handle),
        };

        let analyze_option = KiwiAnalyzeOption {
            match_options: options.match_options as c_int,
            blocklist: blocklist_handle,
            open_ending: if options.open_ending { 1 } else { 0 },
            allowed_dialects: options.allowed_dialects as c_int,
            dialect_cost: options.dialect_cost,
        };

        clear_kiwi_error(&self.inner.api);
        let result_handle = unsafe {
            (self.inner.api.kiwi_analyze)(
                self.handle,
                text_c.as_ptr(),
                top_n,
                analyze_option,
                pretokenized_handle,
            )
        };
        if result_handle.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_analyze returned a null handle",
            ));
        }

        let parsed = {
            let result = KiwiAnalyzeResult {
                inner: self.inner.clone(),
                handle: result_handle,
                kiwi_handle: self.handle,
            };
            result.to_vec()?
        };
        Ok(parsed)
    }

    pub fn analyze_utf16(&self, text: &[u16]) -> Result<Vec<AnalysisCandidate>> {
        self.analyze_utf16_with_options(text, self.default_analyze_options)
    }

    pub fn analyze_utf16_with_options(
        &self,
        text: &[u16],
        options: AnalyzeOptions,
    ) -> Result<Vec<AnalysisCandidate>> {
        self.analyze_utf16_with_overrides(text, options, None, None)
    }

    pub fn analyze_utf16_with_blocklist(
        &self,
        text: &[u16],
        options: AnalyzeOptions,
        blocklist: Option<&MorphemeSet>,
    ) -> Result<Vec<AnalysisCandidate>> {
        self.analyze_utf16_with_overrides(text, options, blocklist, None)
    }

    pub fn analyze_utf16_with_pretokenized(
        &self,
        text: &[u16],
        options: AnalyzeOptions,
        pretokenized: Option<&Pretokenized>,
    ) -> Result<Vec<AnalysisCandidate>> {
        self.analyze_utf16_with_overrides(text, options, None, pretokenized)
    }

    pub fn analyze_utf16_with_blocklist_and_pretokenized(
        &self,
        text: &[u16],
        options: AnalyzeOptions,
        blocklist: Option<&MorphemeSet>,
        pretokenized: Option<&Pretokenized>,
    ) -> Result<Vec<AnalysisCandidate>> {
        self.analyze_utf16_with_overrides(text, options, blocklist, pretokenized)
    }

    fn analyze_utf16_with_overrides(
        &self,
        text: &[u16],
        options: AnalyzeOptions,
        blocklist: Option<&MorphemeSet>,
        pretokenized: Option<&Pretokenized>,
    ) -> Result<Vec<AnalysisCandidate>> {
        let analyze_w = require_optional_api(self.inner.api.kiwi_analyze_w, "kiwi_analyze_w")?;
        let top_n = options.validated_top_n()?;
        let text_c16 = to_c16_null_terminated(text)?;

        let blocklist_handle = match blocklist {
            Some(set) => {
                if !Rc::ptr_eq(&self.inner, &set.inner) {
                    return Err(KiwiError::InvalidArgument(
                        "MorphemeSet was created from a different Kiwi instance".to_string(),
                    ));
                }
                set.handle
            }
            None => ptr::null_mut(),
        };

        if pretokenized.is_some() && !self.re_word_rules.borrow().is_empty() {
            return Err(KiwiError::InvalidArgument(
                "explicit pretokenized input cannot be combined with add_re_word rules yet"
                    .to_string(),
            ));
        }
        if pretokenized.is_none() && !self.re_word_rules.borrow().is_empty() {
            return Err(KiwiError::InvalidArgument(
                "add_re_word rules are currently only supported for UTF-8 analyze APIs".to_string(),
            ));
        }

        let pretokenized_handle = match pretokenized {
            Some(value) => {
                if !Rc::ptr_eq(&self.inner, &value.inner) {
                    return Err(KiwiError::InvalidArgument(
                        "Pretokenized was created from a different Kiwi instance".to_string(),
                    ));
                }
                value.handle
            }
            None => ptr::null_mut(),
        };

        let analyze_option = KiwiAnalyzeOption {
            match_options: options.match_options as c_int,
            blocklist: blocklist_handle,
            open_ending: if options.open_ending { 1 } else { 0 },
            allowed_dialects: options.allowed_dialects as c_int,
            dialect_cost: options.dialect_cost,
        };

        clear_kiwi_error(&self.inner.api);
        let result_handle = unsafe {
            analyze_w(
                self.handle,
                text_c16.as_ptr(),
                top_n,
                analyze_option,
                pretokenized_handle,
            )
        };
        if result_handle.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_analyze_w returned a null handle",
            ));
        }

        let parsed = {
            let result = KiwiAnalyzeResult {
                inner: self.inner.clone(),
                handle: result_handle,
                kiwi_handle: self.handle,
            };
            result.to_vec_utf16()?
        };
        Ok(parsed)
    }

    fn build_re_word_pretokenized(&self, text: &str) -> Result<Option<Pretokenized>> {
        let rules = self.re_word_rules.borrow();
        if rules.is_empty() {
            return Ok(None);
        }

        let mut accepted_ranges: Vec<(usize, usize)> = Vec::new();
        let mut spans: Vec<(usize, usize, String, String)> = Vec::new();

        for rule in rules.iter() {
            for mat in rule.pattern.find_iter(text) {
                if mat.start() == mat.end() {
                    continue;
                }
                let begin = byte_to_char_index(text, mat.start());
                let end = byte_to_char_index(text, mat.end());
                if begin >= end {
                    continue;
                }
                if accepted_ranges
                    .iter()
                    .any(|(a, b)| ranges_overlap(begin, end, *a, *b))
                {
                    continue;
                }

                accepted_ranges.push((begin, end));
                spans.push((begin, end, mat.as_str().to_string(), rule.tag.clone()));
            }
        }

        if spans.is_empty() {
            return Ok(None);
        }

        let mut pretokenized = self.new_pretokenized()?;
        for (begin, end, form, tag) in spans {
            let span_id = pretokenized.add_span(begin, end)?;
            pretokenized.add_token_to_span(span_id, &form, &tag, 0, end - begin)?;
        }

        Ok(Some(pretokenized))
    }

    fn top_candidate_score(&self, text: &str) -> Result<f32> {
        let mut candidates = self.analyze_top_n(text, 1)?;
        Ok(candidates
            .get_mut(0)
            .map(|candidate| candidate.probability)
            .unwrap_or(f32::NEG_INFINITY))
    }

    pub fn tokenize(&self, text: &str) -> Result<Vec<Token>> {
        let mut candidates = self.analyze_top_n(text, 1)?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        Ok(candidates.remove(0).tokens)
    }

    pub fn tokenize_utf16(&self, text: &[u16]) -> Result<Vec<Token>> {
        let options = self.default_analyze_options.with_top_n(1);
        let mut candidates = self.analyze_utf16_with_options(text, options)?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        Ok(candidates.remove(0).tokens)
    }

    pub fn tokenize_with_match_options(
        &self,
        text: &str,
        match_options: i32,
    ) -> Result<Vec<Token>> {
        let options = self
            .default_analyze_options
            .with_top_n(1)
            .with_match_options(match_options);
        let mut candidates = self.analyze_with_options(text, options)?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        Ok(candidates.remove(0).tokens)
    }

    pub fn tokenize_with_options(&self, text: &str, options: AnalyzeOptions) -> Result<Vec<Token>> {
        let mut candidates = self.analyze_with_options(text, options.with_top_n(1))?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        Ok(candidates.remove(0).tokens)
    }

    pub fn tokenize_with_blocklist(
        &self,
        text: &str,
        options: AnalyzeOptions,
        blocklist: Option<&MorphemeSet>,
    ) -> Result<Vec<Token>> {
        let mut candidates = self.analyze_with_blocklist(text, options.with_top_n(1), blocklist)?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        Ok(candidates.remove(0).tokens)
    }

    pub fn tokenize_with_pretokenized(
        &self,
        text: &str,
        options: AnalyzeOptions,
        pretokenized: Option<&Pretokenized>,
    ) -> Result<Vec<Token>> {
        let mut candidates =
            self.analyze_with_pretokenized(text, options.with_top_n(1), pretokenized)?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        Ok(candidates.remove(0).tokens)
    }

    pub fn tokenize_with_blocklist_and_pretokenized(
        &self,
        text: &str,
        options: AnalyzeOptions,
        blocklist: Option<&MorphemeSet>,
        pretokenized: Option<&Pretokenized>,
    ) -> Result<Vec<Token>> {
        let mut candidates = self.analyze_with_blocklist_and_pretokenized(
            text,
            options.with_top_n(1),
            blocklist,
            pretokenized,
        )?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        Ok(candidates.remove(0).tokens)
    }

    pub fn tokenize_utf16_with_match_options(
        &self,
        text: &[u16],
        match_options: i32,
    ) -> Result<Vec<Token>> {
        let options = self
            .default_analyze_options
            .with_top_n(1)
            .with_match_options(match_options);
        let mut candidates = self.analyze_utf16_with_options(text, options)?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        Ok(candidates.remove(0).tokens)
    }

    pub fn tokenize_utf16_with_options(
        &self,
        text: &[u16],
        options: AnalyzeOptions,
    ) -> Result<Vec<Token>> {
        let mut candidates = self.analyze_utf16_with_options(text, options.with_top_n(1))?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        Ok(candidates.remove(0).tokens)
    }

    pub fn tokenize_utf16_with_blocklist(
        &self,
        text: &[u16],
        options: AnalyzeOptions,
        blocklist: Option<&MorphemeSet>,
    ) -> Result<Vec<Token>> {
        let mut candidates =
            self.analyze_utf16_with_blocklist(text, options.with_top_n(1), blocklist)?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        Ok(candidates.remove(0).tokens)
    }

    pub fn tokenize_utf16_with_pretokenized(
        &self,
        text: &[u16],
        options: AnalyzeOptions,
        pretokenized: Option<&Pretokenized>,
    ) -> Result<Vec<Token>> {
        let mut candidates =
            self.analyze_utf16_with_pretokenized(text, options.with_top_n(1), pretokenized)?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        Ok(candidates.remove(0).tokens)
    }

    pub fn tokenize_utf16_with_blocklist_and_pretokenized(
        &self,
        text: &[u16],
        options: AnalyzeOptions,
        blocklist: Option<&MorphemeSet>,
        pretokenized: Option<&Pretokenized>,
    ) -> Result<Vec<Token>> {
        let mut candidates = self.analyze_utf16_with_blocklist_and_pretokenized(
            text,
            options.with_top_n(1),
            blocklist,
            pretokenized,
        )?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        Ok(candidates.remove(0).tokens)
    }

    pub fn analyze_many_with_options<I, S>(
        &self,
        texts: I,
        options: AnalyzeOptions,
    ) -> Result<Vec<Vec<AnalysisCandidate>>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut out = Vec::new();
        for text in texts {
            out.push(self.analyze_with_options(text.as_ref(), options)?);
        }
        Ok(out)
    }

    pub fn analyze_many_via_native<I, S>(
        &self,
        texts: I,
        options: AnalyzeOptions,
    ) -> Result<Vec<Vec<AnalysisCandidate>>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let analyze_m = require_optional_api(self.inner.api.kiwi_analyze_m, "kiwi_analyze_m")?;
        let top_n = options.validated_top_n()?;

        let lines: Vec<CString> = texts
            .into_iter()
            .map(|value| CString::new(value.as_ref()))
            .collect::<std::result::Result<_, _>>()?;
        let mut context = AnalyzeManyContext {
            lines,
            inner: self.inner.clone(),
            kiwi_handle: self.handle,
            results: Vec::new(),
            error: None,
        };

        let analyze_option = KiwiAnalyzeOption {
            match_options: options.match_options as c_int,
            blocklist: ptr::null_mut(),
            open_ending: if options.open_ending { 1 } else { 0 },
            allowed_dialects: options.allowed_dialects as c_int,
            dialect_cost: options.dialect_cost,
        };

        clear_kiwi_error(&self.inner.api);
        let result = unsafe {
            analyze_m(
                self.handle,
                analyze_m_reader_callback,
                analyze_receiver_callback,
                (&mut context as *mut AnalyzeManyContext).cast::<c_void>(),
                top_n,
                analyze_option,
            )
        };

        if result < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_analyze_m returned an error",
            ));
        }

        if let Some(error) = context.error {
            return Err(error);
        }

        Ok(context.results)
    }

    pub fn analyze_many_utf16_via_native<I>(
        &self,
        texts: I,
        options: AnalyzeOptions,
    ) -> Result<Vec<Vec<AnalysisCandidate>>>
    where
        I: IntoIterator<Item = Vec<u16>>,
    {
        let analyze_mw = require_optional_api(self.inner.api.kiwi_analyze_mw, "kiwi_analyze_mw")?;
        let top_n = options.validated_top_n()?;

        let lines: Vec<Vec<u16>> = texts.into_iter().collect();
        let mut context = AnalyzeManyWContext {
            lines,
            inner: self.inner.clone(),
            kiwi_handle: self.handle,
            results: Vec::new(),
            error: None,
        };

        let analyze_option = KiwiAnalyzeOption {
            match_options: options.match_options as c_int,
            blocklist: ptr::null_mut(),
            open_ending: if options.open_ending { 1 } else { 0 },
            allowed_dialects: options.allowed_dialects as c_int,
            dialect_cost: options.dialect_cost,
        };

        clear_kiwi_error(&self.inner.api);
        let result = unsafe {
            analyze_mw(
                self.handle,
                analyze_mw_reader_callback,
                analyze_w_receiver_callback,
                (&mut context as *mut AnalyzeManyWContext).cast::<c_void>(),
                top_n,
                analyze_option,
            )
        };

        if result < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_analyze_mw returned an error",
            ));
        }

        if let Some(error) = context.error {
            return Err(error);
        }

        Ok(context.results)
    }

    pub fn tokenize_many<I, S>(&self, texts: I) -> Result<Vec<Vec<Token>>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut out = Vec::new();
        for text in texts {
            out.push(self.tokenize(text.as_ref())?);
        }
        Ok(out)
    }

    pub fn tokenize_many_with_echo<I, S>(&self, texts: I) -> Result<Vec<(Vec<Token>, String)>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut out = Vec::new();
        for text in texts {
            let raw = text.as_ref().to_string();
            out.push((self.tokenize(raw.as_str())?, raw));
        }
        Ok(out)
    }

    pub fn space(&self, text: &str, reset_whitespace: bool) -> Result<String> {
        let normalized = if reset_whitespace {
            reset_hangul_whitespace(text)
        } else {
            text.to_string()
        };

        let options = self
            .default_analyze_options
            .with_top_n(1)
            .with_match_options(KIWI_MATCH_ALL | KIWI_MATCH_Z_CODA);
        let mut analyzed = self.analyze_with_options(&normalized, options)?;
        if analyzed.is_empty() {
            return Ok(normalized);
        }

        Ok(reconstruct_spaced_text(
            &normalized,
            &analyzed.remove(0).tokens,
        ))
    }

    pub fn space_many<I, S>(&self, texts: I, reset_whitespace: bool) -> Result<Vec<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut out = Vec::new();
        for text in texts {
            out.push(self.space(text.as_ref(), reset_whitespace)?);
        }
        Ok(out)
    }

    pub fn glue<S>(&self, text_chunks: &[S]) -> Result<String>
    where
        S: AsRef<str>,
    {
        Ok(self.glue_with_options(text_chunks, None, false)?.0)
    }

    pub fn glue_with_options<S>(
        &self,
        text_chunks: &[S],
        insert_new_lines: Option<&[bool]>,
        return_space_insertions: bool,
    ) -> Result<(String, Option<Vec<bool>>)>
    where
        S: AsRef<str>,
    {
        if text_chunks.is_empty() {
            return Ok((
                String::new(),
                if return_space_insertions {
                    Some(Vec::new())
                } else {
                    None
                },
            ));
        }

        let chunks: Vec<String> = text_chunks
            .iter()
            .map(|chunk| chunk.as_ref().trim().to_string())
            .collect();

        if let Some(new_lines) = insert_new_lines {
            if new_lines.len() != chunks.len().saturating_sub(1) {
                return Err(KiwiError::InvalidArgument(format!(
                    "insert_new_lines length must be {}",
                    chunks.len().saturating_sub(1)
                )));
            }
        }

        let mut result = String::new();
        let mut space_insertions = Vec::with_capacity(chunks.len().saturating_sub(1));

        for index in 0..chunks.len().saturating_sub(1) {
            let left = &chunks[index];
            let right = &chunks[index + 1];

            result.push_str(left);

            let with_space = format!("{left} {right}");
            let without_space = format!("{left}{right}");
            let score_with_space = self.top_candidate_score(&with_space)?;
            let score_without_space = self.top_candidate_score(&without_space)?;

            let insert_space =
                score_with_space >= score_without_space || ends_with_ascii_word(left);
            space_insertions.push(insert_space);

            if insert_space {
                let use_newline = insert_new_lines
                    .and_then(|flags| flags.get(index))
                    .copied()
                    .unwrap_or(false);
                result.push(if use_newline { '\n' } else { ' ' });
            }
        }

        if let Some(last) = chunks.last() {
            result.push_str(last);
        }

        Ok((
            result,
            if return_space_insertions {
                Some(space_insertions)
            } else {
                None
            },
        ))
    }

    pub fn new_morphset(&self) -> Result<MorphemeSet> {
        let new_morphset =
            require_optional_api(self.inner.api.kiwi_new_morphset, "kiwi_new_morphset")?;

        clear_kiwi_error(&self.inner.api);
        let handle = unsafe { new_morphset(self.handle) };
        if handle.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_new_morphset returned a null handle",
            ));
        }

        Ok(MorphemeSet {
            inner: self.inner.clone(),
            handle,
        })
    }

    pub fn new_pretokenized(&self) -> Result<Pretokenized> {
        let init = require_optional_api(self.inner.api.kiwi_pt_init, "kiwi_pt_init")?;

        clear_kiwi_error(&self.inner.api);
        let handle = unsafe { init() };
        if handle.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_pt_init returned a null handle",
            ));
        }

        Ok(Pretokenized {
            inner: self.inner.clone(),
            handle,
        })
    }

    pub fn split_into_sents(
        &self,
        text: &str,
        match_options: i32,
    ) -> Result<Vec<SentenceBoundary>> {
        let split = require_optional_api(
            self.inner.api.kiwi_split_into_sents,
            "kiwi_split_into_sents",
        )?;
        let text_c = CString::new(text)?;

        clear_kiwi_error(&self.inner.api);
        let handle = unsafe {
            split(
                self.handle,
                text_c.as_ptr(),
                match_options as c_int,
                ptr::null_mut(),
            )
        };

        if handle.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_split_into_sents returned a null handle",
            ));
        }

        let result = KiwiSentenceResult {
            inner: self.inner.clone(),
            handle,
        };
        result.to_vec()
    }

    pub fn split_into_sents_with_options(
        &self,
        text: &str,
        options: AnalyzeOptions,
        return_tokens: bool,
        return_sub_sents: bool,
    ) -> Result<Vec<Sentence>> {
        let tokens = self.tokenize_with_options(text, options.with_top_n(1))?;
        Ok(build_sentences_from_tokens(
            text,
            tokens,
            return_tokens,
            return_sub_sents,
        ))
    }

    pub fn split_into_sents_utf16(
        &self,
        text: &[u16],
        match_options: i32,
    ) -> Result<Vec<SentenceBoundary>> {
        let split = require_optional_api(
            self.inner.api.kiwi_split_into_sents_w,
            "kiwi_split_into_sents_w",
        )?;
        let text_c16 = to_c16_null_terminated(text)?;

        clear_kiwi_error(&self.inner.api);
        let handle = unsafe {
            split(
                self.handle,
                text_c16.as_ptr(),
                match_options as c_int,
                ptr::null_mut(),
            )
        };

        if handle.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_split_into_sents_w returned a null handle",
            ));
        }

        let result = KiwiSentenceResult {
            inner: self.inner.clone(),
            handle,
        };
        result.to_vec()
    }

    pub fn split_into_sents_utf16_with_options(
        &self,
        text: &[u16],
        options: AnalyzeOptions,
        return_tokens: bool,
        return_sub_sents: bool,
    ) -> Result<Vec<Sentence>> {
        let tokens = self.tokenize_utf16_with_options(text, options.with_top_n(1))?;
        let raw_text: String = std::char::decode_utf16(text.iter().copied())
            .map(|value| value.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect();
        Ok(build_sentences_from_tokens(
            &raw_text,
            tokens,
            return_tokens,
            return_sub_sents,
        ))
    }

    pub fn join(&self, morphs: &[(&str, &str)], lm_search: bool) -> Result<String> {
        let new_joiner = require_optional_api(self.inner.api.kiwi_new_joiner, "kiwi_new_joiner")?;

        clear_kiwi_error(&self.inner.api);
        let handle = unsafe { new_joiner(self.handle, if lm_search { 1 } else { 0 }) };
        if handle.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_new_joiner returned a null handle",
            ));
        }

        let mut joiner = KiwiJoiner {
            inner: self.inner.clone(),
            handle,
        };

        for (form, tag) in morphs {
            let auto_option = !tag.contains('-');
            joiner.add(form, tag, auto_option)?;
        }

        joiner.get()
    }

    pub fn join_utf16(&self, morphs: &[(&str, &str)], lm_search: bool) -> Result<String> {
        let new_joiner = require_optional_api(self.inner.api.kiwi_new_joiner, "kiwi_new_joiner")?;

        clear_kiwi_error(&self.inner.api);
        let handle = unsafe { new_joiner(self.handle, if lm_search { 1 } else { 0 }) };
        if handle.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_new_joiner returned a null handle",
            ));
        }

        let mut joiner = KiwiJoiner {
            inner: self.inner.clone(),
            handle,
        };

        for (form, tag) in morphs {
            let auto_option = !tag.contains('-');
            joiner.add(form, tag, auto_option)?;
        }

        joiner.get_utf16()
    }

    pub fn open_sw_tokenizer(&self, path: impl AsRef<Path>) -> Result<SwTokenizer> {
        SwTokenizer::open(self, path)
    }

    pub fn tag_to_string(&self, tag: u8) -> Result<String> {
        let tag_to_string =
            require_optional_api(self.inner.api.kiwi_tag_to_string, "kiwi_tag_to_string")?;

        clear_kiwi_error(&self.inner.api);
        let pointer = unsafe { tag_to_string(self.handle, tag) };
        if pointer.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_tag_to_string returned a null pointer",
            ));
        }

        Ok(cstr_to_string(pointer))
    }

    pub fn find_morphemes(
        &self,
        form: &str,
        tag: Option<&str>,
        sense_id: i32,
        max_count: usize,
    ) -> Result<Vec<u32>> {
        let find = require_optional_api(self.inner.api.kiwi_find_morphemes, "kiwi_find_morphemes")?;
        self.find_morphemes_inner(find, form, tag, sense_id, max_count)
    }

    pub fn find_morphemes_with_prefix(
        &self,
        form_prefix: &str,
        tag: Option<&str>,
        sense_id: i32,
        max_count: usize,
    ) -> Result<Vec<u32>> {
        let find = require_optional_api(
            self.inner.api.kiwi_find_morphemes_with_prefix,
            "kiwi_find_morphemes_with_prefix",
        )?;
        self.find_morphemes_inner(find, form_prefix, tag, sense_id, max_count)
    }

    fn find_morphemes_inner(
        &self,
        find_fn: unsafe extern "C" fn(
            KiwiHandle,
            *const i8,
            *const i8,
            c_int,
            *mut c_uint,
            c_int,
        ) -> c_int,
        form: &str,
        tag: Option<&str>,
        sense_id: i32,
        max_count: usize,
    ) -> Result<Vec<u32>> {
        if max_count > c_int::MAX as usize {
            return Err(KiwiError::InvalidArgument(format!(
                "max_count must be <= {}",
                c_int::MAX
            )));
        }

        let form_c = CString::new(form)?;
        let tag_c = match tag {
            Some(value) => Some(CString::new(value)?),
            None => None,
        };

        let mut morph_ids = vec![0 as c_uint; max_count];

        clear_kiwi_error(&self.inner.api);
        let size = unsafe {
            find_fn(
                self.handle,
                form_c.as_ptr(),
                tag_c.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
                sense_id as c_int,
                morph_ids.as_mut_ptr(),
                max_count as c_int,
            )
        };

        if size < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_find_morphemes returned an error",
            ));
        }

        morph_ids.truncate(size as usize);
        Ok(morph_ids)
    }

    pub fn morpheme_info(&self, morph_id: u32) -> Result<MorphemeInfo> {
        let get_info = require_optional_api(
            self.inner.api.kiwi_get_morpheme_info,
            "kiwi_get_morpheme_info",
        )?;

        clear_kiwi_error(&self.inner.api);
        let info = unsafe { get_info(self.handle, morph_id as c_uint) };
        if read_kiwi_error(&self.inner.api).is_some() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_get_morpheme_info returned an error",
            ));
        }

        Ok(info.into())
    }

    pub fn morpheme_form(&self, morph_id: u32) -> Result<String> {
        let get_form = require_optional_api(
            self.inner.api.kiwi_get_morpheme_form,
            "kiwi_get_morpheme_form",
        )?;
        let free_form = require_optional_api(
            self.inner.api.kiwi_free_morpheme_form,
            "kiwi_free_morpheme_form",
        )?;

        clear_kiwi_error(&self.inner.api);
        let pointer = unsafe { get_form(self.handle, morph_id as c_uint) };
        if pointer.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_get_morpheme_form returned a null pointer",
            ));
        }

        let form = cstr_to_string(pointer);

        let free_result = unsafe { free_form(pointer) };
        if free_result != 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_free_morpheme_form returned an error",
            ));
        }

        Ok(form)
    }

    pub fn morpheme_form_utf16(&self, morph_id: u32) -> Result<String> {
        let get_form = require_optional_api(
            self.inner.api.kiwi_get_morpheme_form_w,
            "kiwi_get_morpheme_form_w",
        )?;

        clear_kiwi_error(&self.inner.api);
        let pointer = unsafe { get_form(self.handle, morph_id as c_uint) };
        if pointer.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_get_morpheme_form_w returned a null pointer",
            ));
        }

        Ok(c16str_to_string(pointer))
    }

    pub fn morpheme(&self, morph_id: u32) -> Result<MorphemeSense> {
        let info = self.morpheme_info(morph_id)?;
        Ok(MorphemeSense {
            morph_id,
            form: self.morpheme_form(morph_id)?,
            tag: self.tag_to_string(info.tag)?,
            sense_id: info.sense_id,
            dialect: info.dialect,
        })
    }

    pub fn list_senses(&self, form: &str, max_count: usize) -> Result<Vec<MorphemeSense>> {
        let morph_ids = self.find_morphemes(form, None, -1, max_count)?;
        let mut out = Vec::with_capacity(morph_ids.len());
        for morph_id in morph_ids {
            let info = self.morpheme_info(morph_id)?;
            let morph_form = self.morpheme_form(morph_id)?;
            let tag = self.tag_to_string(info.tag)?;
            out.push(MorphemeSense {
                morph_id,
                form: morph_form,
                tag,
                sense_id: info.sense_id,
                dialect: info.dialect,
            });
        }
        Ok(out)
    }

    pub fn most_similar_morphemes(
        &self,
        morph_id: u32,
        top_n: usize,
    ) -> Result<Vec<SimilarityPair>> {
        let func = require_optional_api(
            self.inner.api.kiwi_cong_most_similar_words,
            "kiwi_cong_most_similar_words",
        )?;
        self.collect_similarity_pairs(func, morph_id, top_n)
    }

    pub fn most_similar_contexts(
        &self,
        context_id: u32,
        top_n: usize,
    ) -> Result<Vec<SimilarityPair>> {
        let func = require_optional_api(
            self.inner.api.kiwi_cong_most_similar_contexts,
            "kiwi_cong_most_similar_contexts",
        )?;
        self.collect_similarity_pairs(func, context_id, top_n)
    }

    pub fn predict_words_from_context(
        &self,
        context_id: u32,
        top_n: usize,
    ) -> Result<Vec<SimilarityPair>> {
        let func = require_optional_api(
            self.inner.api.kiwi_cong_predict_words_from_context,
            "kiwi_cong_predict_words_from_context",
        )?;
        self.collect_similarity_pairs(func, context_id, top_n)
    }

    pub fn predict_next_morpheme(
        &self,
        context_id: u32,
        top_n: usize,
    ) -> Result<Vec<SimilarityPair>> {
        self.predict_words_from_context(context_id, top_n)
    }

    pub fn predict_words_from_context_diff(
        &self,
        context_id: u32,
        bg_context_id: u32,
        weight: f32,
        top_n: usize,
    ) -> Result<Vec<SimilarityPair>> {
        let func = require_optional_api(
            self.inner.api.kiwi_cong_predict_words_from_context_diff,
            "kiwi_cong_predict_words_from_context_diff",
        )?;

        if top_n > c_int::MAX as usize {
            return Err(KiwiError::InvalidArgument(format!(
                "top_n must be <= {}",
                c_int::MAX
            )));
        }

        let mut pairs = vec![KiwiSimilarityPairRaw::default(); top_n];

        clear_kiwi_error(&self.inner.api);
        let size = unsafe {
            func(
                self.handle,
                context_id as c_uint,
                bg_context_id as c_uint,
                weight,
                pairs.as_mut_ptr(),
                top_n as c_int,
            )
        };

        if size < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_cong_predict_words_from_context_diff returned an error",
            ));
        }

        pairs.truncate(size as usize);
        Ok(pairs
            .into_iter()
            .map(|pair| SimilarityPair {
                id: pair.id,
                score: pair.score,
            })
            .collect())
    }

    pub fn predict_next_morpheme_diff(
        &self,
        context_id: u32,
        bg_context_id: u32,
        weight: f32,
        top_n: usize,
    ) -> Result<Vec<SimilarityPair>> {
        self.predict_words_from_context_diff(context_id, bg_context_id, weight, top_n)
    }

    pub fn morpheme_similarity(&self, morph_id1: u32, morph_id2: u32) -> Result<f32> {
        let func =
            require_optional_api(self.inner.api.kiwi_cong_similarity, "kiwi_cong_similarity")?;

        clear_kiwi_error(&self.inner.api);
        let score = unsafe { func(self.handle, morph_id1 as c_uint, morph_id2 as c_uint) };
        if score.is_nan() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_cong_similarity returned NaN",
            ));
        }
        Ok(score)
    }

    pub fn context_similarity(&self, context_id1: u32, context_id2: u32) -> Result<f32> {
        let func = require_optional_api(
            self.inner.api.kiwi_cong_context_similarity,
            "kiwi_cong_context_similarity",
        )?;

        clear_kiwi_error(&self.inner.api);
        let score = unsafe { func(self.handle, context_id1 as c_uint, context_id2 as c_uint) };
        if score.is_nan() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_cong_context_similarity returned NaN",
            ));
        }
        Ok(score)
    }

    pub fn to_context_id(&self, morph_ids: &[u32]) -> Result<u32> {
        let func = require_optional_api(
            self.inner.api.kiwi_cong_to_context_id,
            "kiwi_cong_to_context_id",
        )?;

        if morph_ids.len() > c_int::MAX as usize {
            return Err(KiwiError::InvalidArgument(format!(
                "morph_ids length must be <= {}",
                c_int::MAX
            )));
        }

        clear_kiwi_error(&self.inner.api);
        let context_id = unsafe { func(self.handle, morph_ids.as_ptr(), morph_ids.len() as c_int) };
        if context_id == 0 && read_kiwi_error(&self.inner.api).is_some() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_cong_to_context_id returned an error",
            ));
        }

        Ok(context_id)
    }

    pub fn from_context_id(&self, context_id: u32, max_size: usize) -> Result<Vec<u32>> {
        let func = require_optional_api(
            self.inner.api.kiwi_cong_from_context_id,
            "kiwi_cong_from_context_id",
        )?;

        if max_size > c_int::MAX as usize {
            return Err(KiwiError::InvalidArgument(format!(
                "max_size must be <= {}",
                c_int::MAX
            )));
        }

        let mut morph_ids = vec![0 as c_uint; max_size];

        clear_kiwi_error(&self.inner.api);
        let size = unsafe {
            func(
                self.handle,
                context_id as c_uint,
                morph_ids.as_mut_ptr(),
                max_size as c_int,
            )
        };

        if size < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_cong_from_context_id returned an error",
            ));
        }

        morph_ids.truncate(size as usize);
        Ok(morph_ids)
    }

    pub fn script_name(&self, script: u8) -> Result<String> {
        let func =
            require_optional_api(self.inner.api.kiwi_get_script_name, "kiwi_get_script_name")?;
        let pointer = unsafe { func(script) };
        if pointer.is_null() {
            return Err(KiwiError::Api(
                "kiwi_get_script_name returned a null pointer".to_string(),
            ));
        }
        Ok(cstr_to_string(pointer))
    }

    pub fn list_all_scripts(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for script in 0u8..=u8::MAX {
            let name = self.script_name(script)?;
            if name == "Unknown" {
                continue;
            }
            if !names.contains(&name) {
                names.push(name);
            }
        }
        Ok(names)
    }

    fn collect_similarity_pairs(
        &self,
        func: unsafe extern "C" fn(KiwiHandle, c_uint, *mut KiwiSimilarityPairRaw, c_int) -> c_int,
        id: u32,
        top_n: usize,
    ) -> Result<Vec<SimilarityPair>> {
        if top_n > c_int::MAX as usize {
            return Err(KiwiError::InvalidArgument(format!(
                "top_n must be <= {}",
                c_int::MAX
            )));
        }

        let mut pairs = vec![KiwiSimilarityPairRaw::default(); top_n];

        clear_kiwi_error(&self.inner.api);
        let size = unsafe {
            func(
                self.handle,
                id as c_uint,
                pairs.as_mut_ptr(),
                top_n as c_int,
            )
        };

        if size < 0 {
            return Err(api_error(
                &self.inner.api,
                "CoNg similarity API returned an error",
            ));
        }

        pairs.truncate(size as usize);
        Ok(pairs
            .into_iter()
            .map(|pair| SimilarityPair {
                id: pair.id,
                score: pair.score,
            })
            .collect())
    }
}

impl Drop for Kiwi {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        unsafe {
            (self.inner.api.kiwi_close)(self.handle);
        }
        self.handle = ptr::null_mut();
    }
}

pub struct SwTokenizer {
    inner: Rc<LoadedLibrary>,
    handle: KiwiSwTokenizerHandle,
    _kiwi_handle: KiwiHandle,
}

type SwTokenizerOffsets = Vec<(i32, i32)>;

impl SwTokenizer {
    pub fn open(kiwi: &Kiwi, path: impl AsRef<Path>) -> Result<Self> {
        let init = require_optional_api(kiwi.inner.api.kiwi_swt_init, "kiwi_swt_init")?;
        let path_c = CString::new(path.as_ref().to_string_lossy().to_string())?;

        clear_kiwi_error(&kiwi.inner.api);
        let handle = unsafe { init(path_c.as_ptr(), kiwi.handle) };
        if handle.is_null() {
            return Err(api_error(
                &kiwi.inner.api,
                "kiwi_swt_init returned a null handle",
            ));
        }

        Ok(Self {
            inner: kiwi.inner.clone(),
            handle,
            _kiwi_handle: kiwi.handle,
        })
    }

    pub fn encode(&self, text: &str) -> Result<Vec<i32>> {
        Ok(self.encode_internal(text, false)?.0)
    }

    pub fn encode_with_offsets(&self, text: &str) -> Result<(Vec<i32>, SwTokenizerOffsets)> {
        let (token_ids, raw_offsets) = self.encode_internal(text, true)?;
        let mut offsets = Vec::with_capacity(raw_offsets.len() / 2);
        for chunk in raw_offsets.chunks_exact(2) {
            offsets.push((chunk[0], chunk[1]));
        }
        Ok((token_ids, offsets))
    }

    fn encode_internal(&self, text: &str, with_offsets: bool) -> Result<(Vec<i32>, Vec<i32>)> {
        let encode = require_optional_api(self.inner.api.kiwi_swt_encode, "kiwi_swt_encode")?;
        let text_c = CString::new(text)?;

        clear_kiwi_error(&self.inner.api);
        let token_size = unsafe {
            encode(
                self.handle,
                text_c.as_ptr(),
                -1,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                0,
            )
        };
        if token_size < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_swt_encode size query returned an error",
            ));
        }

        if token_size == 0 {
            return Ok((Vec::new(), Vec::new()));
        }

        let token_size_usize = token_size as usize;
        let mut token_ids = vec![0; token_size_usize];
        let mut raw_offsets = if with_offsets {
            vec![0; token_size_usize.saturating_mul(2)]
        } else {
            Vec::new()
        };

        clear_kiwi_error(&self.inner.api);
        let written = unsafe {
            encode(
                self.handle,
                text_c.as_ptr(),
                -1,
                token_ids.as_mut_ptr(),
                token_ids.len() as c_int,
                if with_offsets {
                    raw_offsets.as_mut_ptr()
                } else {
                    ptr::null_mut()
                },
                if with_offsets {
                    raw_offsets.len() as c_int
                } else {
                    0
                },
            )
        };
        if written < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_swt_encode returned an error",
            ));
        }

        let written = written as usize;
        token_ids.truncate(written);
        if with_offsets {
            raw_offsets.truncate(written.saturating_mul(2));
        }
        Ok((token_ids, raw_offsets))
    }

    pub fn decode(&self, token_ids: &[i32]) -> Result<String> {
        if token_ids.len() > c_int::MAX as usize {
            return Err(KiwiError::InvalidArgument(format!(
                "token_ids length must be <= {}",
                c_int::MAX
            )));
        }

        let decode = require_optional_api(self.inner.api.kiwi_swt_decode, "kiwi_swt_decode")?;

        clear_kiwi_error(&self.inner.api);
        let text_size = unsafe {
            decode(
                self.handle,
                token_ids.as_ptr(),
                token_ids.len() as c_int,
                ptr::null_mut(),
                0,
            )
        };
        if text_size < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_swt_decode size query returned an error",
            ));
        }
        if text_size == 0 {
            return Ok(String::new());
        }

        let mut out = vec![0u8; text_size as usize];
        clear_kiwi_error(&self.inner.api);
        let written = unsafe {
            decode(
                self.handle,
                token_ids.as_ptr(),
                token_ids.len() as c_int,
                out.as_mut_ptr() as *mut c_char,
                out.len() as c_int,
            )
        };
        if written < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_swt_decode returned an error",
            ));
        }
        out.truncate(written as usize);
        String::from_utf8(out).map_err(|error| {
            KiwiError::Api(format!("kiwi_swt_decode returned invalid utf-8: {error}"))
        })
    }
}

impl Drop for SwTokenizer {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        if let Some(close) = self.inner.api.kiwi_swt_close {
            unsafe {
                close(self.handle);
            }
        }
        self.handle = ptr::null_mut();
    }
}

struct KiwiAnalyzeResult {
    inner: Rc<LoadedLibrary>,
    handle: KiwiResHandle,
    kiwi_handle: KiwiHandle,
}

impl KiwiAnalyzeResult {
    fn to_vec(&self) -> Result<Vec<AnalysisCandidate>> {
        self.to_vec_with_mode(false)
    }

    fn to_vec_utf16(&self) -> Result<Vec<AnalysisCandidate>> {
        self.to_vec_with_mode(true)
    }

    fn to_vec_with_mode(&self, use_utf16_strings: bool) -> Result<Vec<AnalysisCandidate>> {
        let result_count = unsafe { (self.inner.api.kiwi_res_size)(self.handle) };
        if result_count < 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_res_size returned an error",
            ));
        }

        let mut out = Vec::with_capacity(result_count as usize);
        for i in 0..result_count {
            let probability = unsafe { (self.inner.api.kiwi_res_prob)(self.handle, i) };
            let token_count = unsafe { (self.inner.api.kiwi_res_word_num)(self.handle, i) };
            if token_count < 0 {
                return Err(api_error(
                    &self.inner.api,
                    "kiwi_res_word_num returned an error",
                ));
            }

            let mut tokens = Vec::with_capacity(token_count as usize);
            for j in 0..token_count {
                let (form, tag) = if use_utf16_strings {
                    match (
                        self.inner.api.kiwi_res_form_w,
                        self.inner.api.kiwi_res_tag_w,
                    ) {
                        (Some(get_form_w), Some(get_tag_w)) => {
                            let form_ptr = unsafe { get_form_w(self.handle, i, j) };
                            let tag_ptr = unsafe { get_tag_w(self.handle, i, j) };
                            if form_ptr.is_null() || tag_ptr.is_null() {
                                return Err(api_error(
                                    &self.inner.api,
                                    "kiwi_res_form_w/tag_w returned a null pointer",
                                ));
                            }
                            (c16str_to_string(form_ptr), c16str_to_string(tag_ptr))
                        }
                        _ => {
                            let form_ptr =
                                unsafe { (self.inner.api.kiwi_res_form)(self.handle, i, j) };
                            let tag_ptr =
                                unsafe { (self.inner.api.kiwi_res_tag)(self.handle, i, j) };
                            if form_ptr.is_null() || tag_ptr.is_null() {
                                return Err(api_error(
                                    &self.inner.api,
                                    "kiwi_res_form/tag returned a null pointer",
                                ));
                            }
                            (cstr_to_string(form_ptr), cstr_to_string(tag_ptr))
                        }
                    }
                } else {
                    let form_ptr = unsafe { (self.inner.api.kiwi_res_form)(self.handle, i, j) };
                    let tag_ptr = unsafe { (self.inner.api.kiwi_res_tag)(self.handle, i, j) };
                    if form_ptr.is_null() || tag_ptr.is_null() {
                        return Err(api_error(
                            &self.inner.api,
                            "kiwi_res_form/tag returned a null pointer",
                        ));
                    }
                    (cstr_to_string(form_ptr), cstr_to_string(tag_ptr))
                };

                let token_info_raw = self.inner.api.kiwi_res_token_info.and_then(|get_info| {
                    let pointer = unsafe { get_info(self.handle, i, j) };
                    if pointer.is_null() {
                        None
                    } else {
                        Some(unsafe { *pointer })
                    }
                });
                let token_info = token_info_raw.map(TokenInfo::from);

                let position = unsafe { (self.inner.api.kiwi_res_position)(self.handle, i, j) };
                let length = unsafe { (self.inner.api.kiwi_res_length)(self.handle, i, j) };
                let word_position =
                    unsafe { (self.inner.api.kiwi_res_word_position)(self.handle, i, j) };
                let sent_position =
                    unsafe { (self.inner.api.kiwi_res_sent_position)(self.handle, i, j) };
                let score = unsafe { (self.inner.api.kiwi_res_score)(self.handle, i, j) };
                let typo_cost = unsafe { (self.inner.api.kiwi_res_typo_cost)(self.handle, i, j) };

                if position < 0 || length < 0 || word_position < 0 || sent_position < 0 {
                    return Err(api_error(
                        &self.inner.api,
                        "kiwi_res_* returned an invalid index",
                    ));
                }

                let morpheme_id = self.inner.api.kiwi_res_morpheme_id.and_then(|get_id| {
                    let id = unsafe { get_id(self.handle, i, j, self.kiwi_handle) };
                    if id < 0 {
                        None
                    } else {
                        Some(id as u32)
                    }
                });

                tokens.push(Token {
                    form,
                    tag,
                    position: token_info
                        .map(|info| info.chr_position as usize)
                        .unwrap_or(position as usize),
                    length: token_info
                        .map(|info| info.length as usize)
                        .unwrap_or(length as usize),
                    word_position: token_info
                        .map(|info| info.word_position as usize)
                        .unwrap_or(word_position as usize),
                    sent_position: token_info
                        .map(|info| info.sent_position as usize)
                        .unwrap_or(sent_position as usize),
                    line_number: token_info
                        .map(|info| info.line_number as usize)
                        .unwrap_or_default(),
                    sub_sent_position: token_info
                        .map(|info| info.sub_sent_position as usize)
                        .unwrap_or_default(),
                    score: token_info.map(|info| info.score).unwrap_or(score),
                    typo_cost: token_info.map(|info| info.typo_cost).unwrap_or(typo_cost),
                    typo_form_id: token_info.map(|info| info.typo_form_id).unwrap_or_default(),
                    paired_token: token_info.and_then(|info| {
                        if info.paired_token == u32::MAX {
                            None
                        } else {
                            Some(info.paired_token as usize)
                        }
                    }),
                    morpheme_id,
                    tag_id: token_info.map(|info| info.tag),
                    sense_or_script: token_info.map(|info| info.sense_or_script),
                    dialect: token_info.map(|info| info.dialect),
                });
            }

            out.push(AnalysisCandidate {
                probability,
                tokens,
            });
        }

        Ok(out)
    }
}

impl Drop for KiwiAnalyzeResult {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        unsafe {
            (self.inner.api.kiwi_res_close)(self.handle);
        }
        self.handle = ptr::null_mut();
    }
}

struct KiwiSentenceResult {
    inner: Rc<LoadedLibrary>,
    handle: KiwiSsHandle,
}

impl KiwiSentenceResult {
    fn to_vec(&self) -> Result<Vec<SentenceBoundary>> {
        let size_fn = require_optional_api(self.inner.api.kiwi_ss_size, "kiwi_ss_size")?;
        let begin_fn = require_optional_api(
            self.inner.api.kiwi_ss_begin_position,
            "kiwi_ss_begin_position",
        )?;
        let end_fn =
            require_optional_api(self.inner.api.kiwi_ss_end_position, "kiwi_ss_end_position")?;

        let size = unsafe { size_fn(self.handle) };
        if size < 0 {
            return Err(api_error(&self.inner.api, "kiwi_ss_size returned an error"));
        }

        let mut out = Vec::with_capacity(size as usize);
        for i in 0..size {
            let begin = unsafe { begin_fn(self.handle, i) };
            let end = unsafe { end_fn(self.handle, i) };
            if begin < 0 || end < 0 {
                return Err(api_error(
                    &self.inner.api,
                    "kiwi_ss_begin_position/end_position returned an error",
                ));
            }
            out.push(SentenceBoundary {
                begin: begin as usize,
                end: end as usize,
            });
        }

        Ok(out)
    }
}

impl Drop for KiwiSentenceResult {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        if let Some(close_fn) = self.inner.api.kiwi_ss_close {
            unsafe {
                close_fn(self.handle);
            }
        }
        self.handle = ptr::null_mut();
    }
}

struct KiwiJoiner {
    inner: Rc<LoadedLibrary>,
    handle: KiwiJoinerHandle,
}

impl KiwiJoiner {
    fn add(&mut self, form: &str, tag: &str, auto_option: bool) -> Result<()> {
        let add_fn = require_optional_api(self.inner.api.kiwi_joiner_add, "kiwi_joiner_add")?;

        let form_c = CString::new(form)?;
        let tag_c = CString::new(tag)?;

        clear_kiwi_error(&self.inner.api);
        let result = unsafe {
            add_fn(
                self.handle,
                form_c.as_ptr(),
                tag_c.as_ptr(),
                if auto_option { 1 } else { 0 },
            )
        };

        if result != 0 {
            return Err(api_error(
                &self.inner.api,
                "kiwi_joiner_add returned an error",
            ));
        }

        Ok(())
    }

    fn get(&self) -> Result<String> {
        let get_fn = require_optional_api(self.inner.api.kiwi_joiner_get, "kiwi_joiner_get")?;

        clear_kiwi_error(&self.inner.api);
        let pointer = unsafe { get_fn(self.handle) };
        if pointer.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_joiner_get returned a null pointer",
            ));
        }

        Ok(cstr_to_string(pointer))
    }

    fn get_utf16(&self) -> Result<String> {
        let get_fn = require_optional_api(self.inner.api.kiwi_joiner_get_w, "kiwi_joiner_get_w")?;

        clear_kiwi_error(&self.inner.api);
        let pointer = unsafe { get_fn(self.handle) };
        if pointer.is_null() {
            return Err(api_error(
                &self.inner.api,
                "kiwi_joiner_get_w returned a null pointer",
            ));
        }

        Ok(c16str_to_string(pointer))
    }
}

impl Drop for KiwiJoiner {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        if let Some(close_fn) = self.inner.api.kiwi_joiner_close {
            unsafe {
                close_fn(self.handle);
            }
        }
        self.handle = ptr::null_mut();
    }
}

struct KiwiWordSetResult {
    inner: Rc<LoadedLibrary>,
    handle: KiwiWsHandle,
}

impl KiwiWordSetResult {
    fn to_vec(&self) -> Result<Vec<ExtractedWord>> {
        let size_fn = require_optional_api(self.inner.api.kiwi_ws_size, "kiwi_ws_size")?;
        let form_fn = require_optional_api(self.inner.api.kiwi_ws_form, "kiwi_ws_form")?;
        let score_fn = require_optional_api(self.inner.api.kiwi_ws_score, "kiwi_ws_score")?;
        let freq_fn = require_optional_api(self.inner.api.kiwi_ws_freq, "kiwi_ws_freq")?;
        let pos_score_fn =
            require_optional_api(self.inner.api.kiwi_ws_pos_score, "kiwi_ws_pos_score")?;

        let size = unsafe { size_fn(self.handle) };
        if size < 0 {
            return Err(api_error(&self.inner.api, "kiwi_ws_size returned an error"));
        }

        let mut out = Vec::with_capacity(size as usize);
        for index in 0..size {
            let form_ptr = unsafe { form_fn(self.handle, index) };
            if form_ptr.is_null() {
                return Err(api_error(
                    &self.inner.api,
                    "kiwi_ws_form returned a null pointer",
                ));
            }
            let score = unsafe { score_fn(self.handle, index) };
            let frequency = unsafe { freq_fn(self.handle, index) };
            let pos_score = unsafe { pos_score_fn(self.handle, index) };

            out.push(ExtractedWord {
                form: cstr_to_string(form_ptr),
                score,
                frequency,
                pos_score,
            });
        }
        Ok(out)
    }

    fn to_vec_utf16(&self) -> Result<Vec<ExtractedWord>> {
        let size_fn = require_optional_api(self.inner.api.kiwi_ws_size, "kiwi_ws_size")?;
        let form_fn = require_optional_api(self.inner.api.kiwi_ws_form_w, "kiwi_ws_form_w")?;
        let score_fn = require_optional_api(self.inner.api.kiwi_ws_score, "kiwi_ws_score")?;
        let freq_fn = require_optional_api(self.inner.api.kiwi_ws_freq, "kiwi_ws_freq")?;
        let pos_score_fn =
            require_optional_api(self.inner.api.kiwi_ws_pos_score, "kiwi_ws_pos_score")?;

        let size = unsafe { size_fn(self.handle) };
        if size < 0 {
            return Err(api_error(&self.inner.api, "kiwi_ws_size returned an error"));
        }

        let mut out = Vec::with_capacity(size as usize);
        for index in 0..size {
            let form_ptr = unsafe { form_fn(self.handle, index) };
            if form_ptr.is_null() {
                return Err(api_error(
                    &self.inner.api,
                    "kiwi_ws_form_w returned a null pointer",
                ));
            }
            let score = unsafe { score_fn(self.handle, index) };
            let frequency = unsafe { freq_fn(self.handle, index) };
            let pos_score = unsafe { pos_score_fn(self.handle, index) };

            out.push(ExtractedWord {
                form: c16str_to_string(form_ptr),
                score,
                frequency,
                pos_score,
            });
        }
        Ok(out)
    }
}

impl Drop for KiwiWordSetResult {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        if let Some(close_fn) = self.inner.api.kiwi_ws_close {
            unsafe {
                close_fn(self.handle);
            }
        }
        self.handle = ptr::null_mut();
    }
}

struct RuleCallbackContext {
    replacer: Box<dyn Fn(&str) -> String>,
}

unsafe extern "C" fn rule_replacer_callback(
    input: *const c_char,
    input_len: c_int,
    output: *mut c_char,
    user_data: *mut c_void,
) -> c_int {
    if user_data.is_null() {
        return -1;
    }

    let context = &mut *(user_data as *mut RuleCallbackContext);
    let input_slice = if input.is_null() || input_len < 0 {
        &[][..]
    } else {
        std::slice::from_raw_parts(input as *const u8, input_len as usize)
    };

    let input_text = String::from_utf8_lossy(input_slice);
    let replaced = (context.replacer)(&input_text);
    let replaced_bytes = replaced.as_bytes();

    if replaced_bytes.len() > c_int::MAX as usize {
        return -1;
    }

    if output.is_null() {
        return replaced_bytes.len() as c_int;
    }

    ptr::copy_nonoverlapping(
        replaced_bytes.as_ptr(),
        output as *mut u8,
        replaced_bytes.len(),
    );
    replaced_bytes.len() as c_int
}

struct ReaderContext {
    lines: Vec<CString>,
}

struct ReaderWContext {
    lines: Vec<Vec<u16>>,
}

struct AnalyzeManyContext {
    lines: Vec<CString>,
    inner: Rc<LoadedLibrary>,
    kiwi_handle: KiwiHandle,
    results: Vec<Vec<AnalysisCandidate>>,
    error: Option<KiwiError>,
}

struct AnalyzeManyWContext {
    lines: Vec<Vec<u16>>,
    inner: Rc<LoadedLibrary>,
    kiwi_handle: KiwiHandle,
    results: Vec<Vec<AnalysisCandidate>>,
    error: Option<KiwiError>,
}

unsafe extern "C" fn reader_callback(
    id: c_int,
    buffer: *mut c_char,
    user_data: *mut c_void,
) -> c_int {
    if user_data.is_null() || id < 0 {
        return -1;
    }

    let context = &mut *(user_data as *mut ReaderContext);
    let line = match context.lines.get(id as usize) {
        Some(line) => line.as_bytes(),
        None => return 0,
    };

    if line.len() > c_int::MAX as usize {
        return -1;
    }

    if buffer.is_null() {
        return line.len() as c_int;
    }

    ptr::copy_nonoverlapping(line.as_ptr(), buffer as *mut u8, line.len());
    line.len() as c_int
}

unsafe extern "C" fn reader_w_callback(
    id: c_int,
    buffer: *mut u16,
    user_data: *mut c_void,
) -> c_int {
    if user_data.is_null() || id < 0 {
        return -1;
    }

    let context = &mut *(user_data as *mut ReaderWContext);
    let line = match context.lines.get(id as usize) {
        Some(line) => line,
        None => return 0,
    };

    if line.len() > c_int::MAX as usize {
        return -1;
    }

    if buffer.is_null() {
        return line.len() as c_int;
    }

    ptr::copy_nonoverlapping(line.as_ptr(), buffer, line.len());
    line.len() as c_int
}

unsafe extern "C" fn analyze_m_reader_callback(
    id: c_int,
    buffer: *mut c_char,
    user_data: *mut c_void,
) -> c_int {
    if user_data.is_null() || id < 0 {
        return -1;
    }

    let context = &mut *(user_data as *mut AnalyzeManyContext);
    let line = match context.lines.get(id as usize) {
        Some(line) => line.as_bytes(),
        None => return 0,
    };

    if line.len() > c_int::MAX as usize {
        return -1;
    }

    if buffer.is_null() {
        return line.len() as c_int;
    }

    ptr::copy_nonoverlapping(line.as_ptr(), buffer as *mut u8, line.len());
    line.len() as c_int
}

unsafe extern "C" fn analyze_mw_reader_callback(
    id: c_int,
    buffer: *mut u16,
    user_data: *mut c_void,
) -> c_int {
    if user_data.is_null() || id < 0 {
        return -1;
    }

    let context = &mut *(user_data as *mut AnalyzeManyWContext);
    let line = match context.lines.get(id as usize) {
        Some(line) => line,
        None => return 0,
    };

    if line.len() > c_int::MAX as usize {
        return -1;
    }

    if buffer.is_null() {
        return line.len() as c_int;
    }

    ptr::copy_nonoverlapping(line.as_ptr(), buffer, line.len());
    line.len() as c_int
}

unsafe extern "C" fn analyze_receiver_callback(
    id: c_int,
    result: KiwiResHandle,
    user_data: *mut c_void,
) -> c_int {
    if user_data.is_null() {
        return -1;
    }

    let context = &mut *(user_data as *mut AnalyzeManyContext);
    if context.error.is_some() {
        return -1;
    }
    if id < 0 {
        context.error = Some(KiwiError::InvalidArgument(
            "kiwi_analyze_m callback returned a negative id".to_string(),
        ));
        return -1;
    }

    let parsed = {
        let analyze_result = KiwiAnalyzeResult {
            inner: context.inner.clone(),
            handle: result,
            kiwi_handle: context.kiwi_handle,
        };
        analyze_result.to_vec()
    };

    match parsed {
        Ok(value) => {
            let index = id as usize;
            if context.results.len() <= index {
                context.results.resize_with(index + 1, Vec::new);
            }
            context.results[index] = value;
            0
        }
        Err(error) => {
            context.error = Some(error);
            -1
        }
    }
}

unsafe extern "C" fn analyze_w_receiver_callback(
    id: c_int,
    result: KiwiResHandle,
    user_data: *mut c_void,
) -> c_int {
    if user_data.is_null() {
        return -1;
    }

    let context = &mut *(user_data as *mut AnalyzeManyWContext);
    if context.error.is_some() {
        return -1;
    }
    if id < 0 {
        context.error = Some(KiwiError::InvalidArgument(
            "kiwi_analyze_mw callback returned a negative id".to_string(),
        ));
        return -1;
    }

    let parsed = {
        let analyze_result = KiwiAnalyzeResult {
            inner: context.inner.clone(),
            handle: result,
            kiwi_handle: context.kiwi_handle,
        };
        analyze_result.to_vec_utf16()
    };

    match parsed {
        Ok(value) => {
            let index = id as usize;
            if context.results.len() <= index {
                context.results.resize_with(index + 1, Vec::new);
            }
            context.results[index] = value;
            0
        }
        Err(error) => {
            context.error = Some(error);
            -1
        }
    }
}

fn to_c16_null_terminated(value: &[u16]) -> Result<Vec<u16>> {
    if value.contains(&0) {
        return Err(KiwiError::InvalidArgument(
            "UTF-16 input must not contain interior NUL".to_string(),
        ));
    }
    let mut out = Vec::with_capacity(value.len() + 1);
    out.extend_from_slice(value);
    out.push(0);
    Ok(out)
}

fn require_optional_api<T: Copy>(function: Option<T>, name: &'static str) -> Result<T> {
    function.ok_or_else(|| {
        KiwiError::Api(format!(
            "{name} is unavailable in the loaded Kiwi library version"
        ))
    })
}

fn ranges_overlap(a_begin: usize, a_end: usize, b_begin: usize, b_end: usize) -> bool {
    a_begin < b_end && b_begin < a_end
}

fn byte_to_char_index(text: &str, byte_index: usize) -> usize {
    if byte_index >= text.len() {
        return text.chars().count();
    }
    let mut boundary = byte_index;
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    text[..boundary].chars().count()
}

fn build_char_to_byte_map(text: &str) -> Vec<usize> {
    let mut map = Vec::with_capacity(text.chars().count() + 1);
    for (index, _) in text.char_indices() {
        map.push(index);
    }
    map.push(text.len());
    map
}

fn slice_char_range<'a>(text: &'a str, map: &[usize], begin: usize, end: usize) -> &'a str {
    let max = map.len().saturating_sub(1);
    let safe_begin = begin.min(max);
    let safe_end = end.min(max).max(safe_begin);
    &text[map[safe_begin]..map[safe_end]]
}

fn strip_all_whitespace(value: &str) -> String {
    value.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn is_hangul_syllable(ch: char) -> bool {
    ('\u{AC00}'..='\u{D7A3}').contains(&ch)
}

fn is_hangul_or_sentence_punct(ch: char) -> bool {
    is_hangul_syllable(ch) || matches!(ch, '.' | ',' | '?' | '!' | ':' | ';')
}

fn reset_hangul_whitespace(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;

    while index < chars.len() {
        if chars[index].is_whitespace() {
            let start = index;
            while index < chars.len() && chars[index].is_whitespace() {
                index += 1;
            }
            let prev = if start > 0 {
                Some(chars[start - 1])
            } else {
                None
            };
            let next = chars.get(index).copied();
            let remove = prev.map(is_hangul_syllable).unwrap_or(false)
                && next.map(is_hangul_or_sentence_punct).unwrap_or(false);
            if !remove {
                for ch in &chars[start..index] {
                    out.push(*ch);
                }
            }
            continue;
        }

        out.push(chars[index]);
        index += 1;
    }

    out
}

fn starts_with_any(tag: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| tag.starts_with(prefix))
}

fn is_space_insertable_target(tag: &str) -> bool {
    tag.starts_with('N')
        || tag.starts_with('M')
        || tag.starts_with('I')
        || starts_with_any(
            tag,
            &["VV", "VA", "VX", "VCN", "XR", "XPN", "SW", "SL", "SH", "SN"],
        )
}

fn is_space_insertable_target_strict(tag: &str) -> bool {
    tag.starts_with('M')
        || tag.starts_with('I')
        || starts_with_any(
            tag,
            &[
                "NP", "NR", "NNG", "NNP", "VV", "VA", "VX", "VCN", "XR", "XPN", "SW", "SH",
            ],
        )
}

fn is_space_insertable_prev(tag: &str) -> bool {
    let first = tag.chars().next().unwrap_or('\0');
    (!matches!(first, 'S' | 'U' | 'W' | 'X'))
        || tag.starts_with("XR")
        || tag.starts_with("XS")
        || tag.starts_with("SE")
        || tag.starts_with("SH")
}

fn should_insert_space_between(prev_tag: &str, tag: &str, form: &str) -> bool {
    if tag == "VX" && (form == "하" || form == "지") {
        return false;
    }

    (is_space_insertable_prev(prev_tag) && is_space_insertable_target(tag))
        || (prev_tag == "SN" && is_space_insertable_target_strict(tag))
        || (starts_with_any(prev_tag, &["SF", "SP", "SL"])
            && is_space_insertable_target_strict(tag))
}

fn should_strip_gap(prev_tag: Option<&str>, tag: &str, form: &str) -> bool {
    tag.starts_with('E')
        || tag.starts_with('J')
        || tag.starts_with("XS")
        || (tag == "VX" && (form == "하" || form == "지"))
        || (prev_tag == Some("SN") && tag == "NNB")
}

fn reconstruct_spaced_text(raw: &str, tokens: &[Token]) -> String {
    if tokens.is_empty() {
        return raw.to_string();
    }

    let map = build_char_to_byte_map(raw);
    let text_len = map.len().saturating_sub(1);
    let mut out = String::new();
    let mut last = 0usize;
    let mut prev_tag: Option<&str> = None;

    for (index, token) in tokens.iter().enumerate() {
        let start = token.position.min(text_len);
        let end = token
            .position
            .saturating_add(token.length)
            .min(text_len)
            .max(start);

        if last < start {
            let gap = slice_char_range(raw, &map, last, start);
            let mut gap_text = gap.to_string();
            if should_strip_gap(prev_tag, &token.tag, &token.form) {
                gap_text = strip_all_whitespace(&gap_text);
            }
            if !gap_text.is_empty() {
                out.push_str(&gap_text);
            }
            last = start;
        }

        if let Some(prev) = prev_tag {
            if should_insert_space_between(prev, &token.tag, &token.form)
                && !out.is_empty()
                && !out
                    .chars()
                    .last()
                    .map(|ch| ch.is_whitespace())
                    .unwrap_or(false)
            {
                out.push(' ');
            }
        }

        if last < end {
            let token_text = if token.tag.starts_with("NN")
                && (index + 1 >= tokens.len() || end <= tokens[index + 1].position)
            {
                token.form.clone()
            } else {
                strip_all_whitespace(slice_char_range(raw, &map, last, end))
            };

            if !token_text.is_empty() {
                out.push_str(&token_text);
            }
        }

        last = end;
        prev_tag = Some(&token.tag);
    }

    if last < text_len {
        out.push_str(slice_char_range(raw, &map, last, text_len));
    }

    out
}

fn token_end(token: &Token) -> usize {
    token.position.saturating_add(token.length)
}

fn build_sentences_from_tokens(
    text: &str,
    tokens: Vec<Token>,
    return_tokens: bool,
    return_sub_sents: bool,
) -> Vec<Sentence> {
    if tokens.is_empty() {
        return Vec::new();
    }

    let map = build_char_to_byte_map(text);
    let mut grouped: BTreeMap<usize, Vec<Token>> = BTreeMap::new();
    for token in tokens {
        grouped.entry(token.sent_position).or_default().push(token);
    }

    let mut out = Vec::with_capacity(grouped.len());
    for sentence_tokens in grouped.into_values() {
        let start = sentence_tokens
            .iter()
            .map(|token| token.position)
            .min()
            .unwrap_or(0);
        let end = sentence_tokens.iter().map(token_end).max().unwrap_or(start);
        let sentence_text = slice_char_range(text, &map, start, end).to_string();

        let subs = if return_sub_sents {
            Some(build_sub_sentences_from_tokens(
                text,
                &map,
                &sentence_tokens,
                return_tokens,
            ))
        } else {
            None
        };

        out.push(Sentence {
            text: sentence_text,
            start,
            end,
            tokens: if return_tokens {
                Some(sentence_tokens.clone())
            } else {
                None
            },
            subs,
        });
    }

    out
}

fn build_sub_sentences_from_tokens(
    text: &str,
    map: &[usize],
    sentence_tokens: &[Token],
    return_tokens: bool,
) -> Vec<Sentence> {
    let mut out = Vec::new();
    let mut current_sub_id = 0usize;
    let mut current_start = 0usize;
    let mut current_end = 0usize;
    let mut current_tokens: Vec<Token> = Vec::new();

    for token in sentence_tokens {
        let sub_id = token.sub_sent_position;
        if sub_id == 0 {
            if current_sub_id != 0 {
                out.push(Sentence {
                    text: slice_char_range(text, map, current_start, current_end).to_string(),
                    start: current_start,
                    end: current_end,
                    tokens: if return_tokens {
                        Some(std::mem::take(&mut current_tokens))
                    } else {
                        current_tokens.clear();
                        None
                    },
                    subs: None,
                });
                current_sub_id = 0;
            }
            continue;
        }

        if current_sub_id != sub_id {
            if current_sub_id != 0 {
                out.push(Sentence {
                    text: slice_char_range(text, map, current_start, current_end).to_string(),
                    start: current_start,
                    end: current_end,
                    tokens: if return_tokens {
                        Some(std::mem::take(&mut current_tokens))
                    } else {
                        current_tokens.clear();
                        None
                    },
                    subs: None,
                });
            }
            current_sub_id = sub_id;
            current_start = token.position;
        }

        current_end = token_end(token);
        current_tokens.push(token.clone());
    }

    if current_sub_id != 0 {
        out.push(Sentence {
            text: slice_char_range(text, map, current_start, current_end).to_string(),
            start: current_start,
            end: current_end,
            tokens: if return_tokens {
                Some(current_tokens)
            } else {
                None
            },
            subs: None,
        });
    }

    out
}

fn ends_with_ascii_word(value: &str) -> bool {
    value
        .chars()
        .rev()
        .find(|ch| !ch.is_whitespace())
        .map(|ch| ch.is_ascii_alphanumeric())
        .unwrap_or(false)
}

#[cfg(test)]
mod runtime_tests {
    use super::build_sentences_from_tokens;
    use crate::types::Token;

    fn token(
        form: &str,
        position: usize,
        length: usize,
        sent_position: usize,
        sub_sent_position: usize,
    ) -> Token {
        Token {
            form: form.to_string(),
            tag: "NNG".to_string(),
            position,
            length,
            word_position: 0,
            sent_position,
            line_number: 0,
            sub_sent_position,
            score: 0.0,
            typo_cost: 0.0,
            typo_form_id: 0,
            paired_token: None,
            morpheme_id: None,
            tag_id: None,
            sense_or_script: None,
            dialect: None,
        }
    }

    #[test]
    fn build_sentences_groups_by_sentence_and_sub_sentence_positions() {
        let text = "가 나 다 라";
        let tokens = vec![
            token("가", 0, 1, 0, 0),
            token("나", 2, 1, 0, 1),
            token("다", 4, 1, 0, 1),
            token("라", 6, 1, 1, 0),
        ];

        let sentences = build_sentences_from_tokens(text, tokens, true, true);
        assert_eq!(sentences.len(), 2);

        let first = &sentences[0];
        assert_eq!(first.start, 0);
        assert_eq!(first.end, 5);
        assert_eq!(first.text, "가 나 다");
        assert_eq!(first.tokens.as_ref().map(Vec::len), Some(3));
        assert_eq!(first.subs.as_ref().map(Vec::len), Some(1));

        let sub = &first.subs.as_ref().expect("sub-sents missing")[0];
        assert_eq!(sub.start, 2);
        assert_eq!(sub.end, 5);
        assert_eq!(sub.text, "나 다");
        assert_eq!(sub.tokens.as_ref().map(Vec::len), Some(2));

        let second = &sentences[1];
        assert_eq!(second.start, 6);
        assert_eq!(second.end, 7);
        assert_eq!(second.text, "라");
        assert_eq!(second.tokens.as_ref().map(Vec::len), Some(1));
        assert_eq!(second.subs.as_ref().map(Vec::len), Some(0));
    }
}
