# kiwi-rs

[English README](README.md) | [kiwipiepy 호환성 문서(EN)](docs/kiwipiepy_parity.md) | [kiwipiepy 호환성 문서(KO)](docs/kiwipiepy_parity.ko.md)

`kiwi-rs`는 Kiwi 공식 C API(`include/kiwi/capi.h`)를 Rust에서 사용할 수 있게 만든 바인딩입니다.

## 현재 지원 수준

2026-02-16 기준:

- C API 심볼 로딩: 완료 (`capi.h` 기준 `101/101`)
- 핵심 고수준 사용 흐름: 구현 완료 (`init/new/from_config`, `analyze/tokenize/split/join`, `MorphemeSet`, `Pretokenized`, 오타 API, `SwTokenizer`, CoNg API)
- `kiwipiepy` 표면 완전 호환: 부분 지원 (Python/C++ 전용 계층 일부 미구현)

## 설치

```toml
[dependencies]
kiwi-rs = "0.1"
```

## 런타임 준비 방식

### 방식 1: 코드에서 자동 준비 (`Kiwi::init`)

`Kiwi::init()`은 로컬 경로를 먼저 확인하고, 없으면 Kiwi release에서 라이브러리/모델을 자동 다운로드해 캐시에 저장합니다.

```rust
use kiwi_rs::Kiwi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;
    let tokens = kiwi.tokenize("아버지가방에들어가신다.")?;
    println!("{}", tokens.len());
    Ok(())
}
```

자동 준비 관련 환경변수:

- `KIWI_RS_VERSION` (기본: `latest`, 예: `v0.22.2`)
- `KIWI_RS_CACHE_DIR` (기본: OS 캐시 디렉터리)

자동 준비 시 필요한 외부 명령:

- 공통: `curl`, `tar`
- Windows zip 해제: `powershell` (`Expand-Archive`)

### 방식 2: 설치 스크립트 선실행

Linux/macOS:

```bash
cd kiwi-rs
make install-kiwi
```

Windows (PowerShell):

```powershell
cd kiwi-rs
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install_kiwi.ps1
```

설치 옵션:

- `KIWI_VERSION` / `-Version` (기본: `latest`)
- `KIWI_PREFIX` / `-Prefix` (기본: Unix `$HOME/.local/kiwi`, Windows `%LOCALAPPDATA%\\kiwi`)
- `KIWI_MODEL_VARIANT` / `-ModelVariant` (기본: `base`)

## 라이브러리/모델 경로 직접 지정

### 환경변수 기반 (`Kiwi::new`)

- `KIWI_LIBRARY_PATH`: 동적 라이브러리 경로
- `KIWI_MODEL_PATH`: 모델 디렉터리 경로

### 코드 기반 (`Kiwi::from_config`)

```rust
use kiwi_rs::{Kiwi, KiwiConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = KiwiConfig::default()
        .with_library_path("/path/to/libkiwi.dylib")
        .with_model_path("/path/to/models/cong/base")
        .add_user_word("러스트", "NNP", 0.0);

    let kiwi = Kiwi::from_config(config)?;
    let analyses = kiwi.analyze_top_n("형태소 분석 예시", 2)?;
    println!("{} candidates", analyses.len());
    Ok(())
}
```

## API 개요

### 핵심 API

- 초기화: `Kiwi::init`, `Kiwi::new`, `Kiwi::from_config`, `Kiwi::init_direct`
- 분석/토크나이즈: `analyze*`, `tokenize*`, `analyze_many*`, `tokenize_many*`
- 문장 분리: `split_into_sents*`, `split_into_sents_with_options*`
- 결합/띄어쓰기: `join*`, `space*`, `glue*`

### 고급 API

- Builder: 사용자 단어/별칭 단어/기분석 단어/사용자 사전/정규식 규칙/단어 추출
- 제약 객체: `MorphemeSet`, `Pretokenized`
- 오타: `KiwiTypo`, 기본 오타셋, 비용 조정
- 서브워드: `SwTokenizer`
- CoNg: 유사도/문맥/예측/context-id 변환

### UTF-16 및 옵션 API 지원 여부 확인

- `Kiwi::supports_utf16_api`
- `Kiwi::supports_analyze_mw`
- `KiwiLibrary::supports_builder_init_stream`

## 예제 실행

```bash
cd kiwi-rs
cargo run --example basic
cargo run --example analyze_options
cargo run --example builder_custom_words
cargo run --example typo_build
cargo run --example blocklist_and_pretokenized
cargo run --example split_sentences
cargo run --example utf16_api
cargo run --example native_batch
cargo run --example sw_tokenizer -- /path/to/tokenizer.json
cargo run --example morpheme_semantics
```

각 예제가 확인하는 내용:

| 예제 | 확인 목적 | 핵심 API | 비고 |
|---|---|---|---|
| `basic` | 가장 기본적인 초기화 + 토크나이즈 흐름 | `Kiwi::init`, `Kiwi::tokenize` | 로컬 자산이 없을 때 캐시 기반 bootstrap 동작을 확인할 수 있습니다. |
| `analyze_options` | 분석 옵션이 후보 결과에 미치는 영향 | `AnalyzeOptions`, `Kiwi::analyze_with_options` | `top_n`, `match_options`, 후보 확률 출력 예시입니다. |
| `builder_custom_words` | 사용자 단어/규칙을 반영한 분석기 빌드 | `KiwiLibrary::builder`, `add_user_words`, `add_re_rule` | builder 단계 커스터마이징 API를 다룹니다. |
| `typo_build` | 오타 교정 기반 분석 활성화 | `default_typo_set`, `build_with_typo_and_default_options` | 토큰의 typo 관련 메타데이터를 출력합니다. |
| `blocklist_and_pretokenized` | 특정 형태소 차단 + 구간 강제 토큰화 | `new_morphset`, `new_pretokenized`, `tokenize_with_blocklist_and_pretokenized` | 도메인 제약/고정 구간 분석 시 유용합니다. |
| `split_sentences` | 문장 분리 + 문장별 토큰/하위문장 구조 확인 | `split_into_sents_with_options` | `Sentence` 구조(`text/start/end/tokens/subs`)를 확인합니다. |
| `utf16_api` | UTF-16 경로 분석/토크나이즈/문장분리 | `supports_utf16_api`, `analyze_utf16*`, `tokenize_utf16*`, `split_into_sents_utf16*` | 런타임 UTF-16 지원 여부 확인 로직 포함입니다. |
| `native_batch` | 네이티브 콜백 기반 배치 분석 경로 | `analyze_many_via_native`, `analyze_many_utf16_via_native` | 다중 문장 고처리량 시나리오에 맞는 예제입니다. |
| `sw_tokenizer` | 서브워드 토크나이저 인코딩/디코딩 | `open_sw_tokenizer`, `encode_with_offsets`, `decode` | `tokenizer.json` 경로 인자가 필요합니다. |
| `morpheme_semantics` | 형태소 ID 조회 + CoNg 의미 API 흐름 | `find_morphemes`, `morpheme`, `most_similar_morphemes`, `to_context_id` | 형태소/문맥 ID 기반 의미 API 사용법입니다. |

## kiwipiepy 호환성

상세 호환성 표:

- 영어: `docs/kiwipiepy_parity.md`
- 한국어: `docs/kiwipiepy_parity.ko.md`

요약하면, `kiwi-rs`는 C API로 가능한 핵심 워크플로는 대부분 지원하지만, Python/C++ 전용 계층(template/dataset/ngram 유틸)은 pure C API 바인딩 범위를 벗어납니다.

## 자주 발생하는 오류

- `failed to load library`
  - 라이브러리 경로가 잘못됐거나 접근 불가. `KIWI_LIBRARY_PATH`를 지정하거나 `Kiwi::init()` 사용.

- `Cannot open extract.mdl for WordDetector`
  - 모델 경로 오류. 모델 파일이 있는 디렉터리를 `KIWI_MODEL_PATH`(또는 config)로 지정.

- `reading type 'Ds' failed` 같은 iostream 계열 에러
  - 라이브러리/모델 버전 불일치 가능성이 높음. 같은 Kiwi release 태그의 자산으로 맞추기.

## 로컬 품질 검증

```bash
cd kiwi-rs
cargo fmt
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
cargo check --examples
cargo package --allow-dirty
```

## 라이선스

Kiwi와 동일하게 LGPL v3를 따릅니다.
