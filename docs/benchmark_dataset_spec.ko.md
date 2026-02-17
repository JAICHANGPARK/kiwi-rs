# 벤치마크 데이터셋 스펙

## 목적

이 데이터셋은 `kiwi-rs`와 `kiwipiepy` 런타임 성능 비교를 위한 재현 가능한 텍스트 셋입니다.
정확도(품질) 골드셋이 아니며, 언어학적 품질 우위를 주장하는 용도로 사용하면 안 됩니다.

## 형식

- 파일 형식: UTF-8 TSV
- 경로(현재): `benchmarks/datasets/swe_textset_v2.tsv`
- 라인 형식: `category<TAB>text`
- 빈 줄과 `#` 주석 라인은 무시됩니다.
- 탭이 없으면 category는 `default`로 해석합니다.

## 재현성 규칙

- 보고서에 데이터셋 경로와 SHA-256 해시를 반드시 공개합니다.
- category 필터(`all` 또는 특정 category)를 반드시 공개합니다.
- 실제 실행 커맨드 플래그를 메타데이터에 함께 남깁니다.
- 데이터셋 row를 변경할 때는 파일 버전을 올립니다(`*_v2.tsv`).

## 카테고리 정책

현재 v2 데이터셋은 다음 운영 스타일을 포함합니다.

- `news`
- `colloquial`
- `typo_noisy`
- `code_mixed`
- `ecommerce`
- `finance`
- `tech`
- `longform`

## 권장 사용 방식

- 전체 벤치: 전체 데이터셋으로 실행 (`--dataset-tsv ...`)
- 층화 벤치: 카테고리별 실행 (`--dataset-category <name>`)
- 공개 시 전체 + 카테고리별 결과를 함께 제시
