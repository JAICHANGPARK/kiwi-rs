# kiwi-rs의 kiwipiepy 호환성 현황

기준 시점: 2026-02-16

비교 기준:

- `ref/kiwipiepy-main/kiwipiepy/_wrap.py`
- `ref/kiwipiepy-main/src/KiwiPy.cpp`
- `ref/Kiwi-main/include/kiwi/capi.h`

## 상태 정의

- `Equivalent`: Rust API 형태는 달라도 실질 기능은 대응 가능
- `Partial`: 관련 기능은 있으나 동작/표면이 1:1은 아님
- `Unavailable`: 현재 `kiwi-rs`에서 미지원

## 계층별 진행 수준

1. C API 커버리지: `완료` (`capi.h` 심볼 `101/101` 로딩)
2. 고수준 Rust API 커버리지: 핵심 NLP 흐름은 `높은 수준`
3. `kiwipiepy` 전체 표면 호환: `부분 지원`

## Core API

| kiwipiepy API | 상태 | kiwi-rs API / 비고 |
|---|---|---|
| `Kiwi.add_user_word` | Partial | `KiwiBuilder::add_user_word` (builder 단계) |
| `Kiwi.add_pre_analyzed_word` | Partial | `KiwiBuilder::add_pre_analyzed_word` (builder 단계) |
| `Kiwi.load_user_dictionary` | Partial | `KiwiBuilder::load_user_dictionary` (builder 단계) |
| `Kiwi.add_rule` | Equivalent | `KiwiBuilder::add_rule` |
| `Kiwi.add_re_rule` | Partial | `KiwiBuilder::add_re_rule`; Python `re.sub`와 완전 동일하진 않음 |
| `Kiwi.add_re_word` | Partial | `Kiwi::add_re_word(pattern, tag)`; callback/user_value 변형 미노출 |
| `Kiwi.clear_re_words` | Equivalent | `Kiwi::clear_re_words` |
| `Kiwi.analyze` | Partial | `Kiwi::analyze*`, `Kiwi::analyze_many_with_options`, `Kiwi::analyze_many_via_native` |
| `Kiwi.tokenize` | Partial | `Kiwi::tokenize*`, blocklist/pretokenized/batch 변형 지원; `Stopwords` 헬퍼 없음 |
| `Kiwi.split_into_sents` | Partial | `split_into_sents*`, `split_into_sents_with_options*`; Python namedtuple 표면과는 다름 |

## 런타임 설정/메타데이터

| kiwipiepy API | 상태 | kiwi-rs API / 비고 |
|---|---|---|
| `Kiwi.global_config` | Equivalent | `Kiwi::global_config`, `Kiwi::set_global_config` |
| deprecated property들 (`cutoff_threshold` 등) | Equivalent | Rust 메서드 형태로 제공 (`cutoff_threshold`, `set_cutoff_threshold`, ...) |
| `Kiwi.num_workers` | Equivalent | `Kiwi::num_workers` |
| `Kiwi.model_type` | Equivalent | `Kiwi::model_type` |
| `Kiwi.typo_cost_threshold` | Equivalent | `Kiwi::typo_cost_threshold` |
| `Kiwi.list_senses` | Equivalent | `Kiwi::list_senses` |
| `Kiwi.list_all_scripts` | Equivalent | `Kiwi::list_all_scripts` |
| `Kiwi.morpheme` | Partial | `Kiwi::morpheme(morph_id)`는 ID 중심 |
| `Kiwi.tag_to_string` | Equivalent | `Kiwi::tag_to_string` |

## 결합/띄어쓰기 편의 API

| kiwipiepy API | 상태 | kiwi-rs API / 비고 |
|---|---|---|
| `Kiwi.join` | Partial | `Kiwi::join`; `return_positions`류 출력 미지원 |
| `Kiwi.space` | Partial | `Kiwi::space`, `Kiwi::space_many`; Rust 측 휴리스틱 구현 |
| `Kiwi.glue` | Partial | `Kiwi::glue`, `Kiwi::glue_with_options` |
| `Kiwi.template` | Unavailable | Python 템플릿 계층, 대응 C API 없음 |

## CoNg 의미 API

| kiwipiepy API | 상태 | kiwi-rs API / 비고 |
|---|---|---|
| `Kiwi.most_similar_morphemes` | Partial | `Kiwi::most_similar_morphemes(morph_id, top_n)` |
| `Kiwi.most_similar_contexts` | Partial | `Kiwi::most_similar_contexts(context_id, top_n)` |
| `Kiwi.predict_next_morpheme` | Partial | `Kiwi::predict_next_morpheme*` (ID 기반) |
| `Kiwi.morpheme_similarity` | Equivalent | `Kiwi::morpheme_similarity` |
| `Kiwi.context_similarity` | Equivalent | `Kiwi::context_similarity` |
| context id 변환 | Equivalent | `Kiwi::to_context_id`, `Kiwi::from_context_id` |

## 데이터셋/학습 보조 API

| kiwipiepy API | 상태 | kiwi-rs API / 비고 |
|---|---|---|
| `Kiwi.extract_words` | Equivalent | `KiwiBuilder::extract_words` |
| `Kiwi.extract_add_words` | Equivalent | `KiwiBuilder::extract_add_words` |
| `Kiwi.convert_hsdata` | Unavailable | C++ API 유틸, C API 엔트리 없음 |
| `Kiwi.make_hsdataset` | Unavailable | C++ API 유틸, C API 엔트리 없음 |
| `Kiwi.evaluate`, `Kiwi.predict_next` | Unavailable | kiwipiepy에서도 런타임 미구현(`NotImplementedError`) |

## `Kiwi` 클래스 외 패키지 단위

| kiwipiepy 표면 | kiwi-rs 상태 | 비고 |
|---|---|---|
| `TypoTransformer`, `TypoDefinition` preset | Partial | `KiwiTypo`는 있으나 Python 클래스 구조/preset과 다름 |
| `HSDataset` | Unavailable | Python/C++ 계층 |
| `NgramExtractor` | Unavailable | 대응 C API 없음 |
| `KNLangModel` | Unavailable | 대응 C API 없음 |
| `Template` / `Kiwi.template` | Unavailable | Python 템플릿 계층 |
| `utils.Stopwords` | Unavailable | Rust 헬퍼 타입 미제공 |
| `extract_substrings` | Unavailable | C++ 계층 유틸 |

## 미지원 항목이 남는 이유

`kiwi-rs`는 의도적으로 공개 C API만 바인딩합니다. 반면 `kiwipiepy`의 일부 기능은 Python 확장에서 C++ API를 직접 호출해 노출하므로, `kiwi-rs`에 C++ bridge를 추가하지 않는 한 1:1 재현이 불가능합니다.
