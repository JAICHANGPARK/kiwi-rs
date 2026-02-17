# kiwi-rs 개발기: Rust언어 kiwi 한국어 형태소 분석기 라이브러리 만들기
## kiwi-rs 실전 가이드: 구현, 코드 해설, 벤치마크, 운영 전략

한국어 NLP 파이프라인에서 형태소 분석기는 성능과 정확도를 동시에 좌우하는 핵심 컴포넌트입니다. Rust 생태계에서는 프로덕션에 바로 적용하기 쉬운 한국어 형태소 분석 바인딩 선택지가 제한적이어서, 이 프로젝트에서는 `kiwi-rs`를 직접 구현했습니다. 문제는 대부분의 팀이 "정확도"에는 민감하지만, 실제 서비스에서 체감되는 병목(초기화 지연, 배치 처리량, 캐시 hit/miss 편차, 호출 경계 비용)에는 상대적으로 덜 민감하다는 점입니다.

이 글은 `kiwi-rs`를 기준으로 다음을 한 번에 다룹니다.

- Rust 애플리케이션에서 Kiwi C API를 안정적으로 붙이는 방법
- 실전에서 자주 쓰는 코드 패턴과 왜 그렇게 써야 하는지
- 재현 가능한 벤치마크 방법론
- 2026-02-17 실측 결과 해석(반복 입력 vs 다양한 입력)
- 운영 환경에서의 성능/안정성 체크리스트

대상 독자는 다음과 같습니다.

- Rust 기반 텍스트 처리/검색/추천/에이전트 파이프라인을 운영하는 엔지니어
- Python 기반 PoC를 Rust 서비스로 이식하려는 팀
- "왜 내 벤치마크는 환경마다 다르게 보일까?"를 설명해야 하는 DevRel/기술 리더

---

## 1. 배경: 왜 `kiwi-rs`인가

`kiwi-rs`는 Kiwi 공식 C API(`include/kiwi/capi.h`)를 Rust에서 사용할 수 있도록 만든 바인딩입니다. 단순히 FFI를 연결한 수준을 넘어서, 실제 애플리케이션에서 필요한 상위 워크플로를 다룹니다.

- 초기화: `Kiwi::init`, `Kiwi::new`, `Kiwi::from_config`
- 분석/토크나이즈: `analyze*`, `tokenize*`
- 문장 분리/결합/띄어쓰기: `split_into_sents*`, `join*`, `space*`, `glue*`
- 배치 경로: `analyze_many*`, `tokenize_many*`
- 제약/강제 토큰화: `MorphemeSet`, `Pretokenized`
- 고급 기능: typo, UTF-16 API, subword tokenizer, CoNg semantics

중요한 맥락은, Kiwi 자체가 "새로운 엔진"이 아니고 이미 여러 언어/플랫폼 진입점이 존재했다는 점입니다. 다만 Rust에서 바로 쓸 수 있는 프로덕션 지향 바인딩이 부족했습니다.

- C API: `include/kiwi/capi.h`
- 컴파일 바이너리: [GitHub Releases](https://github.com/bab2min/Kiwi/releases)에서 Windows/Linux/macOS/Android용 라이브러리 + 모델 제공
- C# Wrapper: [kiwi-gui](https://github.com/bab2min/kiwi-gui) (공식 GUI에서 사용), 커뮤니티 기여 래퍼 `NetKiwi`
- Python3 Wrapper: [kiwipiepy](https://github.com/bab2min/kiwipiepy)
- Java Wrapper: Java 1.8+용 `KiwiJava` (`bindings/java` 참조)
- Android Library: NDK 기반 AAR(`kiwi-android-VERSION.aar`), 최소 Android API 21+ / ARM64, 사용법은 `bindings/java` 문서 참조
- R Wrapper: `elbird` (커뮤니티 기여)
- Go Wrapper: `kiwigo` (codingpot 커뮤니티 작업)
- WebAssembly (JavaScript/TypeScript): `bindings/wasm` (RicBent 기여)
- GUI 응용 프로그램: Windows용 [kiwi-gui](https://github.com/bab2min/kiwi-gui)

실무에서 장점은 명확합니다.

1. Rust 서비스로의 통합이 쉽다.
2. 반복 호출 성능을 Rust 측 캐시/재사용 전략으로 확장하기 좋다.
3. C API 기반이라 경계가 명확하고, 환경 검증(라이브러리/모델 버전 정합성) 포인트가 분명하다.

---

## 2. 프로젝트 셋업: 초기화 전략부터 결정하자

형태소 분석기 통합에서 가장 먼저 결정해야 할 것은 **초기화 경로 하나를 고정하는 것**입니다. 실제 운영에서 초기화 경로가 섞이면 재현성이 급격히 떨어집니다.

### 2.1 `Kiwi::init()` 경로

가장 빠르게 시작하는 방식입니다.

- 로컬 자산을 먼저 탐색
- 없으면 캐시에 필요한 라이브러리/모델을 내려받아 부트스트랩

```rust
use kiwi_rs::Kiwi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // local assets가 없으면 cache로 bootstrap
    let kiwi = Kiwi::init()?;

    let text = "아버지가방에들어가신다.";
    let tokens = kiwi.tokenize(text)?;

    for token in tokens {
        println!(
            "{} / {} (pos={}, len={}, word={}, sent={}, score={}, typo_cost={})",
            token.form,
            token.tag,
            token.position,
            token.length,
            token.word_position,
            token.sent_position,
            token.score,
            token.typo_cost
        );
    }

    Ok(())
}
```

핵심 포인트:

- `position`/`length`는 UTF-8 문자 인덱스 기준으로 해석해야 합니다.
- 부트스트랩이 포함되어 있으므로 `init_ms`는 steady-state 처리량과 분리해서 봐야 합니다.

### 2.2 `Kiwi::new` / `Kiwi::from_config` 경로

운영 환경에서는 보통 자산 경로를 명시적으로 고정합니다.

- `KIWI_LIBRARY_PATH`: 동적 라이브러리 경로
- `KIWI_MODEL_PATH`: 모델 경로

혹은 설정 객체를 통해 코드에서 통제할 수 있습니다.

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

권장 운영 원칙:

- 개발/데모: `Kiwi::init()`
- 프로덕션: `Kiwi::from_config` 또는 `Kiwi::new`로 자산 버전 고정

---

## 3. 코드 패턴 1: 분석 후보를 안전하게 다루기

많은 팀이 `analyze_top_n` 결과를 UI나 후속 랭킹 모델에 바로 넣습니다. 이때 후보별 확률과 토큰 경계 해석이 흔한 실수 포인트입니다.

```rust
use kiwi_rs::{AnalyzeOptions, Kiwi, KIWI_MATCH_ALL_WITH_NORMALIZING};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;

    let options = AnalyzeOptions::default()
        .with_top_n(3)
        .with_match_options(KIWI_MATCH_ALL_WITH_NORMALIZING)
        .with_open_ending(false);

    let text = "형태소 분석 결과 후보를 여러 개 보고 싶습니다.";
    let candidates = kiwi.analyze_with_options(text, options)?;

    for (index, candidate) in candidates.iter().enumerate() {
        println!("candidate #{index} prob={}", candidate.probability);
        for token in &candidate.tokens {
            println!("  {}/{} @{}+{}", token.form, token.tag, token.position, token.length);
        }
    }

    Ok(())
}
```

해설:

- `with_top_n(3)`: 단일 1-best 결과만 쓰면 애매한 문장에서 오탐이 증가합니다.
- `with_match_options(...)`: normalize 포함 여부에 따라 실무 정규화 정책이 달라집니다.
- `with_open_ending(false)`: 분석 결과의 폐쇄성을 높여 후보 폭주를 제어할 때 유용합니다.

실전 팁:

- 로그/디버그 단계에서는 반드시 `probability`를 함께 남기세요.
- downstream 모델이 있다면 candidate N-best를 그대로 feature로 보내는 편이 낫습니다.

---

## 4. 코드 패턴 2: 도메인 제약(블록리스트) + 강제 스팬 토큰화

사내 검색, 법률/의학 도메인, 제품명/브랜드명 인식에서는 일부 구간을 반드시 하나의 토큰으로 처리해야 할 때가 많습니다. `MorphemeSet` + `Pretokenized` 조합이 이때 강력합니다.

```rust
use kiwi_rs::{AnalyzeOptions, Kiwi};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;

    let mut blocklist = kiwi.new_morphset()?;
    // 특정 후보 형태소를 차단
    let _ = blocklist.add("하", Some("VV"))?;

    let mut pretokenized = kiwi.new_pretokenized()?;
    let text = "AI엔지니어링팀에서테스트중";

    // [0,2) 영역을 AI/NNP 단일 토큰으로 강제
    let span_id = pretokenized.add_span(0, 2)?;
    pretokenized.add_token_to_span(span_id, "AI", "NNP", 0, 2)?;

    let tokens = kiwi.tokenize_with_blocklist_and_pretokenized(
        text,
        AnalyzeOptions::default(),
        Some(&blocklist),
        Some(&pretokenized),
    )?;

    for token in tokens {
        println!("{}/{} @{}+{}", token.form, token.tag, token.position, token.length);
    }

    Ok(())
}
```

해설:

- `MorphemeSet`: "절대 나오면 안 되는 분석"을 먼저 제거
- `Pretokenized`: "무조건 이 토큰으로 해석"할 구간을 고정
- 두 기능을 조합하면 결과 공간이 줄어들고 도메인 일관성이 올라갑니다

이 패턴은 특히 다음 상황에서 효과적입니다.

- 제품 코드/버전 문자열이 일반 명사로 깨지는 문제
- 브랜드명/사내 용어가 분해되어 검색 품질이 떨어지는 문제
- 규정상 특정 엔터티를 분리 없이 보존해야 하는 문제

---

## 5. 코드 패턴 3: 배치 + UTF-16 경로 점검

대량 문장 처리에서는 개별 호출보다 batch 경로가 안정적입니다. 그리고 외부 시스템(Windows 계열, UTF-16 기반 인터페이스)과의 연동에서는 UTF-16 API 지원 여부를 반드시 런타임에서 확인해야 합니다.

### 5.1 Native batch 분석

```rust
use kiwi_rs::{AnalyzeOptions, Kiwi};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;

    let lines = vec![
        "첫 번째 문장입니다.",
        "두 번째 문장도 형태소 분석합니다.",
        "세 번째 입력입니다.",
    ];

    let options = AnalyzeOptions::default().with_top_n(2);

    let batched = kiwi.analyze_many_via_native(&lines, options)?;
    for (index, candidates) in batched.iter().enumerate() {
        println!("line #{index}: {} candidates", candidates.len());
    }

    if kiwi.supports_analyze_mw() {
        let utf16_lines: Vec<Vec<u16>> = lines
            .iter()
            .map(|line| line.encode_utf16().collect())
            .collect();
        let batched_w = kiwi.analyze_many_utf16_via_native(utf16_lines, options)?;
        println!("utf16 batch count: {}", batched_w.len());
    }

    Ok(())
}
```

포인트:

- `supports_analyze_mw()` 체크 없이 UTF-16 native batch를 가정하면 런타임 에러가 날 수 있습니다.
- 배치 경로에서는 `batch_size`, 입력 다양성, 모델 cache 상태가 숫자에 큰 영향을 줍니다.

### 5.2 UTF-16 API 기능 체크

```rust
use kiwi_rs::{AnalyzeOptions, Kiwi};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kiwi = Kiwi::init()?;

    if !kiwi.supports_utf16_api() {
        println!("Loaded Kiwi library does not support UTF-16 API on this runtime.");
        return Ok(());
    }

    let text = "UTF16 경로도 동일하게 분석할 수 있습니다.";
    let utf16: Vec<u16> = text.encode_utf16().collect();

    let tokens = kiwi.tokenize_utf16_with_options(&utf16, AnalyzeOptions::default())?;
    println!("token count: {}", tokens.len());

    let candidates =
        kiwi.analyze_utf16_with_options(&utf16, AnalyzeOptions::default().with_top_n(2))?;
    println!("candidate count: {}", candidates.len());

    let sentences =
        kiwi.split_into_sents_utf16_with_options(&utf16, AnalyzeOptions::default(), true, true)?;
    println!("sentence count: {}", sentences.len());

    Ok(())
}
```

---

## 6. 벤치마크 방법론: 숫자보다 먼저 실험 설계를 맞춰라

형태소 분석 벤치마크에서 흔한 오류는 다음입니다.

- 초기화 비용과 steady-state 비용을 한 숫자로 뭉개는 것
- 반복 동일 입력만 측정해서 캐시 효과를 과대평가하는 것
- 반대로 varied 입력만 측정해서 warm-cache 시나리오를 놓치는 것

`kiwi-rs` 저장소는 이 문제를 피하기 위해 2개 시나리오를 분리합니다.

1. `input_mode=repeated`: warm-cache에 가까운 반복 처리
2. `input_mode=varied`: near no-cache에 가까운 다양한 입력 처리

재현 명령:

```bash
cd kiwi-rs
mkdir -p tmp

# repeated
.venv-bench/bin/python scripts/compare_feature_bench.py \
  --text "아버지가방에들어가신다." \
  --warmup 100 --iters 5000 \
  --batch-size 256 --batch-iters 500 \
  --input-mode repeated --variant-pool 4096 \
  --repeats 1 \
  --md-out tmp/feature_bench_repeated.md \
  --json-out tmp/feature_bench_repeated.json

# varied
.venv-bench/bin/python scripts/compare_feature_bench.py \
  --text "아버지가방에들어가신다." \
  --warmup 100 --iters 5000 \
  --batch-size 256 --batch-iters 500 \
  --input-mode varied --variant-pool 8192 \
  --repeats 1 \
  --md-out tmp/feature_bench_varied.md \
  --json-out tmp/feature_bench_varied.json
```

---

## 7. 벤치마크 결과 (로컬 측정 스냅샷)

출처 파일:

- `tmp/feature_bench_repeated.md` (측정 시각: `<LOCAL_TIMESTAMP_REDACTED>`)
- `tmp/feature_bench_varied.md` (측정 시각: `<LOCAL_TIMESTAMP_REDACTED>`)

공통 환경 요약:

- OS: `<HOST_OS_REDACTED>`
- rustc: 1.93.1 (2026-02-11)
- cargo: 1.93.1
- Python: 3.14.3
- kiwipiepy: 0.22.2
- text: `아버지가방에들어가신다.`
- warmup: 100, iters: 5000, batch_size: 256, batch_iters: 500
- Git HEAD: `<REPO_COMMIT_SHA>`

### 7.1 Repeated 입력(캐시 유리) 결과

| Feature | kiwi-rs calls/sec | kiwipiepy calls/sec | Ratio (kiwi-rs / kiwipiepy) |
|---|---:|---:|---:|
| tokenize | 1,185,489.51 | 7,792.55 | 152.13x |
| analyze_top1 | 1,199,112.66 | 7,612.25 | 157.52x |
| split_into_sents | 28,908,752.41 | 3,802.38 | 7602.80x |
| split_into_sents_with_tokens | 250,558.01 | 4,872.41 | 51.42x |
| space | 357,757.20 | 4,768.69 | 75.02x |
| join | 2,402,355.08 | 675,759.32 | 3.56x |
| glue | 6,221,490.02 | 7,613.64 | 817.15x |
| analyze_many_native | 166.11 | 165.71 | 1.00x |
| tokenize_many_batch | 3,134.67 | 184.16 | 17.02x |
| space_many_batch | 161.79 | 160.39 | 1.01x |

초기화 시간(`init_ms`, 낮을수록 좋음):

- `Kiwi::init()` / `Kiwi()`: kiwi-rs 1417.905 ms vs kiwipiepy 680.748 ms

### 7.2 Varied 입력(near no-cache) 결과

| Feature | kiwi-rs calls/sec | kiwipiepy calls/sec | Ratio |
|---|---:|---:|---:|
| tokenize | 6,956.95 | 7,393.81 | 0.94x |
| analyze_top1 | 7,319.22 | 7,212.44 | 1.01x |
| split_into_sents | 5,104.73 | 4,399.49 | 1.16x |
| split_into_sents_with_tokens | 4,372.13 | 4,282.95 | 1.02x |
| space | 4,944.59 | 4,497.21 | 1.10x |
| glue | 5,692.86 | 4,965.80 | 1.15x |
| join | 2,927,258.22 | 669,983.08 | 4.37x |
| analyze_many_native | 158.62 | 192.74 | 0.82x |
| tokenize_many_batch | 151.12 | 190.38 | 0.79x |
| space_many_batch | 150.76 | 159.43 | 0.95x |

초기화 시간(`init_ms`):

- kiwi-rs 1368.136 ms vs kiwipiepy 751.911 ms

---

## 8. 결과 해석: 숫자의 의미를 운영 관점으로 번역하기

### 8.1 왜 repeated에서 차이가 크게 보이나

반복 입력에서는 내부 캐시가 재사용됩니다. 그래서 다음 항목이 크게 튀는 것이 자연스럽습니다.

- `tokenize`, `analyze_top1`
- `split_into_sents`, `split_into_sents_with_tokens`
- `glue`, `join`

이 값은 "실제 운영에서 절대적으로 이만큼 빠르다"라기보다, **"캐시 hit 조건에서의 상한치"**에 가깝습니다.

### 8.2 왜 varied에서 격차가 줄어드나

입력이 매번 달라지면 miss-path 비용이 드러납니다. 따라서 실제 사용자 쿼리가 다양할수록 varied 수치가 더 현실적입니다.

- `tokenize`: 0.94x
- `analyze_top1`: 1.01x
- `split_into_sents`: 1.16x
- `space`: 1.10x
- `glue`: 1.15x

즉, "핵심 단건 기능"은 대체로 1.0x 내외~소폭 우세 구간으로 수렴합니다.

### 8.3 `join`이 여전히 강한 이유

`join`은 varied에서도 4.37x로 측정되었습니다. 이는 결합 경로의 내부 최적화와 joiner 재사용 전략 영향이 큽니다.

Rust 전용 측정 항목을 보면 이 경향이 더 분명합니다.

- `joiner_reuse`: 3,213,367.61 calls/sec (varied)
- `join_prepared`: 324,130.72 calls/sec (varied)

운영 팁:

- 동일/유사 형태소 시퀀스를 반복 결합한다면 joiner 준비/재사용 전략을 적극 쓰는 편이 유리합니다.

### 8.4 배치 native 항목에서 주의할 점

`analyze_many_native`, `tokenize_many_batch`는 varied에서 불리하게 나올 수 있습니다.

- `analyze_many_native`: 0.82x
- `tokenize_many_batch`: 0.79x

이 구간은 단순 엔진 성능보다도 다음 요소 영향이 큽니다.

- 배치 크기(`batch_size`)
- 데이터 다양성(`variant_pool`)
- 호출 경계/메모리 배치
- 내부 스케줄링/런타임 상호작용

즉, 배치 항목은 반드시 **실제 서비스 트래픽과 유사한 입력 분포**로 재측정해야 합니다.

---

## 9. 실전 성능 튜닝 가이드

### 9.1 측정 분리 원칙

- startup: `init_ms`를 별도로 추적
- steady-state: `avg_ms`, `calls_per_sec`
- 캐시 효과: repeated + varied 둘 다 공개

### 9.2 API 선택 원칙

- 단건 요청 중심: `tokenize`, `analyze_with_options`
- 다건 고정 배치: `*_many_*` 경로 + 배치 크기 튜닝
- 강제 도메인 규칙: `MorphemeSet` + `Pretokenized`
- UTF-16 연동: `supports_utf16_api()`, `supports_analyze_mw()` 체크 우선

### 9.3 데이터/캐시 전략

- 요청 전처리(normalize) 정책을 명시화해 캐시 hit를 높일지 결정
- 쿼리 재사용성이 높은 제품(검색 자동완성 등)은 repeated 성능 이점이 큼
- 사용자 생성 텍스트처럼 다양성이 큰 제품은 varied 기준으로 용량 계획

### 9.4 장애 예방 포인트

자주 만나는 오류는 대부분 환경 정합성 문제입니다.

- `failed to load library`
  - 라이브러리 경로 점검 (`KIWI_LIBRARY_PATH`) 또는 `Kiwi::init()` 경로 사용
- `Cannot open extract.mdl for WordDetector`
  - 모델 디렉터리 경로 오류 (`KIWI_MODEL_PATH`)
- `reading type 'Ds' failed`
  - 라이브러리/모델 버전 mismatch

운영 규칙:

- 라이브러리 + 모델을 같은 Kiwi release 태그로 고정
- CI에서 smoke test로 초기화/기본 토크나이즈를 반드시 실행

---

## 10. 품질 게이트: 릴리즈 전 체크리스트

```bash
cd kiwi-rs
cargo fmt
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
cargo check --examples
cargo package --allow-dirty
```

권장 추가 항목:

- 벤치 스냅샷을 JSON으로 아카이브하고 회귀 감시
- PR마다 최소 1개 repeated/varied 샘플 비교
- 문서에 실험 날짜와 정확한 커맨드를 함께 남길 것

---

## 11. 마이그레이션 관점의 결론

정리하면 `kiwi-rs`의 성능 해석은 "어느 쪽이 몇 배 빠르다"로 끝내면 안 됩니다.

- 반복 입력에서는 캐시 이점이 크게 나타난다.
- 다양한 입력에서는 기능별로 격차가 줄고, 일부 배치 항목은 역전될 수 있다.
- startup(`init_ms`)은 별도 지표로 운영해야 한다.

실무적으로는 다음 접근이 가장 안전합니다.

1. 내 서비스 트래픽을 repeated/varied로 분해한다.
2. 같은 명령, 같은 텍스트, 같은 배치 설정으로 정기 측정한다.
3. 기능별 의사결정(단건/배치/결합/강제 스팬)을 분리한다.

이 과정을 지키면, 형태소 분석기는 "정확도만 보는 블랙박스"가 아니라, 서비스 SLO를 관리 가능한 엔지니어링 컴포넌트가 됩니다.

---

## 12. AI 활용 가이드 (Codex/ChatGPT/Claude/Gemini)

`kiwi-rs`는 API 표면이 넓고(UTF-16, batch, builder, typo, join 계열 등) 환경 정합성 이슈가 자주 나기 때문에, AI에게 "코드 생성"만 맡기기보다 "검증 가능한 출력 계약"을 함께 주는 방식이 안정적입니다.

### 12.1 AI에게 줄 출력 계약

아래 4가지를 항상 같이 요청하면 재작업이 크게 줄어듭니다.

1. 초기화 경로를 하나만 선택하게 하기 (`Kiwi::init`, `Kiwi::new`, `Kiwi::from_config`) + 선택 이유.
2. 복붙 실행 가능한 Rust 코드로 답하게 하기 (`fn main() -> Result<(), Box<dyn std::error::Error>>`).
3. 검증 명령 1개를 반드시 포함시키기 (`cargo run --example ...` 또는 `cargo run`).
4. 일반론 말고, 이 작업에만 해당하는 주의점 2~3개를 적게 하기.

프롬프트 템플릿:

```text
kiwi-rs로 아래 형식으로 답해줘.
1) 초기화 경로 선택과 이유
2) 복붙 실행 가능한 Rust 코드
3) 검증 명령 1개
4) 이 작업에만 해당하는 주의점
작업: <원하는 작업 설명>
환경: <OS / KIWI_LIBRARY_PATH, KIWI_MODEL_PATH 설정 여부>
```

### 12.2 정확성 체크리스트 (AI 결과 검토용)

- UTF-8 오프셋을 바이트 인덱스로 잘못 쓰지 않았는지 확인
- UTF-16 API 사용 전 `supports_utf16_api()` 체크가 있는지 확인
- `analyze_many_utf16_via_native` 사용 전 `supports_analyze_mw()` 체크가 있는지 확인
- `kiwipiepy`와 완전 동일 동작을 가정하는 문장이 없는지 확인
- 에러 처리에서 라이브러리/모델 버전 mismatch 가능성을 명시했는지 확인

### 12.3 `skills`를 활용한 작업 방식

저장소에는 `kiwi-rs` 전용 스킬이 포함되어 있어, AI가 레포 맥락을 놓치지 않도록 도와줍니다.

- 스킬 파일: `skills/kiwi-rs-assistant/SKILL.md`
- 참조 문서: `skills/kiwi-rs-assistant/references/`
- 에이전트 메타: `skills/kiwi-rs-assistant/agents/openai.yaml`

AI 호출 예시:

```text
$kiwi-rs-assistant를 사용해서 다음 작업을 구현해줘:
- 목표: <예: tokenize_many_batch 기반 배치 파이프라인 작성>
- 제약: <예: UTF-16 미지원 환경 fallback 필수>
- 출력: 코드 + 검증 명령 + 이 작업의 주의점 3개
```

### 12.4 `llms.txt`를 먼저 읽게 하기

프롬프트에 아래 한 줄을 추가하면, AI가 레포의 실제 API/예제 기준으로 답할 확률이 올라갑니다.

```text
먼저 llms.txt를 읽고, 저장소의 실제 API와 예제 기준으로만 답해줘.
```

### 12.5 추천 AI 협업 루프

1. 작업 요구사항을 짧게 고정한다 (입력/출력/성능 목표/실패 조건).
2. AI가 제시한 코드에 대해 즉시 검증 명령을 실행한다.
3. 실패 시 에러 로그 + 환경 정보를 포함해 같은 세션에서 수정 요청한다.
4. 통과 후 `bench_features` 또는 관련 벤치로 성능 회귀를 확인한다.

---

## 부록 A. 빠른 실행 명령 모음

```bash
# 기본 예제
cargo run --example basic
cargo run --example analyze_options
cargo run --example blocklist_and_pretokenized
cargo run --example native_batch
cargo run --example utf16_api

# 벤치마크 (Rust 측)
cargo run --release --example bench_tokenize -- --iters 1000 --warmup 100
cargo run --release --example bench_features -- --iters 5000 --warmup 100 --batch-size 256 --batch-iters 500

# Rust vs Python 비교
python3 scripts/bench_kiwipiepy.py --text "아버지가방에들어가신다." --warmup 100 --iters 5000
```

---

## 부록 B. 참고 파일

- `README.md`
- `README.ko.md`
- `examples/basic.rs`
- `examples/analyze_options.rs`
- `examples/blocklist_and_pretokenized.rs`
- `examples/native_batch.rs`
- `examples/utf16_api.rs`
- `examples/bench_features.rs`
- `scripts/compare_feature_bench.py`
- `tmp/feature_bench_repeated.md`
- `tmp/feature_bench_varied.md`
