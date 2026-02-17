# 벤치마크 방어 가이드 (SWE 심사 관점)

이 문서는 `kiwi-rs` vs `kiwipiepy` 성능 비교에서 자주 제기되는 의문을 선제적으로 차단하기 위한 점검표입니다.

## 1. SWE 평가 지표 정의

- `throughput (calls/sec)`: 초당 호출 처리량. 클수록 좋음.
- `latency (avg_ms)`: 호출당 평균 지연시간. 작을수록 좋음.
- `startup (init_ms)`: 초기화 시간. steady-state와 분리 해석.
- `ratio`: `kiwi-rs / kiwipiepy`.
- `95% bootstrap CI`: ratio의 신뢰구간(재표집 기반).
- `P(ratio > 1)`: Rust가 더 빠를 확률 추정.
- `sink parity`: 양측이 같은 작업량을 처리했는지 확인하는 무결성 지표.

## 2. 승패 판정 규칙 (권장)

- Practical equivalence band: `±5%` (`[0.95, 1.05]`).
- `CI low > 1.05`: `kiwi-rs faster (robust)`.
- `CI high < 0.95`: `kiwipiepy faster (robust)`.
- `CI`가 `[0.95, 1.05]` 내부: `practically equivalent`.
- 그 외: `inconclusive` 또는 `likely faster`로 표기(강한 주장 금지).

## 3. 자주 받는 반박과 대응

- “캐시빨 아닌가?”
  - `input_mode=repeated`와 `input_mode=varied`를 항상 함께 공개.
- “실행 순서 편향 아닌가?”
  - `--engine-order alternate`로 교차 실행.
- “작업량이 다른데 비교한 것 아닌가?”
  - `sink parity` 표가 1.0x 근처인지 확인하고, CI에서는 `--strict-sink-check` 사용.
- “샘플 수가 너무 적다”
  - `repeats >= 5`(권장 7), `bootstrap_samples >= 1000` 유지.
- “환경 다르면 숫자 의미 없지 않나?”
  - OS/CPU/메모리/rustc/python/kiwipiepy/Git SHA/dirty/KIWI 경로를 함께 공개.

## 4. 공개 전 최소 체크리스트

- 동일 명령, 동일 입력, 동일 옵션 사용.
- Rust는 반드시 `--release`.
- `init_ms`와 steady-state를 분리 보고.
- `ratio`와 함께 `95% CI`, `P(ratio>1)` 동시 보고.
- `sink` 경고가 남아 있으면 원인 분석 없이 승패 주장 금지.

## 5. 권장 커맨드

README의 “외부 공개용 벤치 실행 세트 (권장)” 섹션 명령을 그대로 사용합니다.

## 6. 데이터셋 참고

- 데이터셋 스펙: `docs/benchmark_dataset_spec.ko.md`
- 데이터셋 파일(현재): `benchmarks/datasets/swe_textset_v2.tsv`
