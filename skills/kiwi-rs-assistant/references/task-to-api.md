# Task-to-API Routing Map

Use this table to route a user request to the most idiomatic `kiwi-rs` APIs and repository examples.

## Quick Routing Table

| User Intent | Primary `kiwi-rs` APIs | Example File | Verification Command |
|---|---|---|---|
| 빠른 시작 / 기본 형태소 토큰화 | `Kiwi::init`, `Kiwi::tokenize` | `examples/basic.rs` | `cargo run --example basic` |
| 다중 후보 분석(`top_n`) 및 매치/오타 옵션 조절 | `AnalyzeOptions::default().with_top_n(..).with_match_options(..).with_typo_threshold(..)`, `Kiwi::analyze_with_options` | `examples/analyze_options.rs` | `cargo run --example analyze_options` |
| 사용자 단어 / 별칭 단어 / 의미번호(`sense_id`)·방언(`dialect`) 정의 | `KiwiLibrary::builder`, `add_user_word`, `add_user_word_with_def`, `add_alias_word`, `add_alias_word_with_def`, `add_pre_analyzed_word`, `add_re_rule` | `examples/builder_custom_words.rs` | `cargo run --example builder_custom_words` |
| 빌드 시점 오타 교정 및 런타임 `PreparedTypo` 분석 | `KiwiLibrary::default_typo_set` (`KIWI_TYPO_BASIC`, `KIWI_TYPO_DIALECT` 등), `KiwiTypo::prepare`, `build_with_typo_and_default_options`, `tokenize_with_typo`, `analyze_with_typo`, `set_default_prepared_typo` | `examples/typo_build.rs` | `cargo run --example typo_build` |
| 특정 형태소 차단(`MorphemeSet`) 및 구간 고정(`Pretokenized`) | `Kiwi::new_morphset`, `Kiwi::new_pretokenized`, `tokenize_with_blocklist_and_pretokenized`, ` tokenize_with_blocklist_and_pretokenized_and_typo` | `examples/blocklist_and_pretokenized.rs` | `cargo run --example blocklist_and_pretokenized` |
| 문장 분리 및 문장별 토큰/하위절 추출 | `Kiwi::split_into_sents`, `Kiwi::split_into_sents_with_options` | `examples/split_sentences.rs` | `cargo run --example split_sentences` |
| 자동 띄어쓰기 보정(`space`) 및 청크 결합(`glue`) | `Kiwi::space`, `Kiwi::space_via_native`, `Kiwi::glue`, `Kiwi::glue_with_options_via_native` | `examples/basic.rs` | `cargo test --test integration_tests` |
| 형태소 결합(`join`) 및 반복 결합 최적화(`PreparedJoinMorphs`) | `Kiwi::join`, `Kiwi::prepare_join_morphs`, `Kiwi::prepare_join_tokens`, `PreparedJoiner::join` | `examples/basic.rs` | `cargo test --test integration_tests` |
| 글로벌 설정 및 OOV 파라미터 튜닝 | `Kiwi::global_config`, `Kiwi::set_global_config`, `oov_chr_bias`, `oov_global_weight`, `oov_local_weight`, `oov_global_min_freq`, `max_unk_form_size_followed_by_j_class` | `tests/integration_tests.rs` | `cargo test --test integration_tests` |
| UTF-16 기반 분석/토크나이즈/문장분리 | `Kiwi::supports_utf16_api`, `tokenize_utf16_with_options`, `analyze_utf16_with_options`, `split_into_sents_utf16_with_options` | `examples/utf16_api.rs` | `cargo run --example utf16_api` |
| 대용량 다중 문장 배치 분석 | `Kiwi::tokenize_many`, `Kiwi::analyze_many_with_options`, `Kiwi::analyze_many_via_native` | `examples/native_batch.rs` | `cargo run --example native_batch` |
| 서브워드 토크나이저(`SwTokenizer`) 인코딩/디코딩 | `Kiwi::open_sw_tokenizer`, `SwTokenizer::encode_with_offsets`, `SwTokenizer::decode` | `examples/sw_tokenizer.rs` | `cargo run --example sw_tokenizer -- /path/to/tokenizer.json` |
| 형태소 의미 검색 및 CoNg 유사도/문맥 예측 | `Kiwi::find_morphemes`, `Kiwi::morpheme`, `Kiwi::most_similar_morphemes`, `Kiwi::predict_next_morpheme`, `Kiwi::to_context_id` | `examples/morpheme_semantics.rs` | `cargo run --example morpheme_semantics` |

## Initialization Decision Matrix

| Scenario | Recommended API | Notes |
|---|---|---|
| 로컬 설치 여부와 무관하게 바로 실행 | `Kiwi::init()` | 캐시 디렉터리에 공식 릴리스(라이브러리 + 모델) 자동 다운로드 |
| 특정 Kiwi 릴리스 버전 고정 부트스트랩 | `Kiwi::init_with_version("v0.24.0")` | 버전 핀 고정 시 유용 |
| 환경 변수(`KIWI_LIBRARY_PATH`, `KIWI_MODEL_PATH`) 기반 배포 | `Kiwi::new()` | 컨테이너/서버 배포 환경에 적합 |
| 코드에서 명시적 경로와 기본 옵션/단어 지정 | `Kiwi::from_config(config)` | `KiwiConfig` 빌더 패턴 사용 |
| 방언(`enabled_dialects`)을 지정한 직접 초기화 | `Kiwi::init_direct_with_dialects(path, threads, opts, dialects)` | `KIWI_DIALECT_*` 상수 조합 전달 |
| 고급 사전 커스터마이징 / 오타 변환기 / 신조어 추출 | `KiwiLibrary::load_from_env_or_default()?.builder(BuilderConfig::default())?` | 빌드 전 `KiwiBuilder` 단계에서 모든 사전 작업 수행 |
