#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "benchmarks/datasets/swe_textset_v1.tsv"
DST = ROOT / "benchmarks/datasets/swe_textset_v2.tsv"


def parse_rows(path: Path) -> list[tuple[str, str]]:
    rows: list[tuple[str, str]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        raw = line.strip()
        if not raw or raw.startswith("#"):
            continue
        if "\t" not in raw:
            continue
        category, text = raw.split("\t", 1)
        category = category.strip()
        text = text.strip()
        if category and text:
            rows.append((category, text))
    return rows


def strip_trailing_period(text: str) -> str:
    return text[:-1] if text.endswith(".") else text


def augment(category: str, text: str, index: int) -> list[str]:
    base = strip_trailing_period(text)
    if category == "news":
        return [
            text,
            f"{base}고, 관련 부처는 후속 조치를 이번 주 안에 공지할 계획이다.",
            f"{base}는 현장 체감과 통계 수치의 차이를 함께 점검해야 한다.",
        ]
    if category == "colloquial":
        return [
            text,
            f"{base} 오늘은 일정이 조금 빡빡해서 답장이 늦을 수도 있어.",
            f"{base} 나중에 만나서 자세히 얘기하자.",
        ]
    if category == "typo_noisy":
        compact = base.replace(" ", "")
        return [
            text,
            f"{compact}지금상태로도확인부탁",
            f"{compact}근데이거왜이러지",
        ]
    if category == "code_mixed":
        return [
            text,
            f"{base} 그리고 profiling 결과를 기준으로 bottleneck을 먼저 줄여야 한다.",
            f"{base} incident 대응을 위해 fallback path와 retry policy도 같이 검토한다.",
        ]
    if category == "ecommerce":
        return [
            text,
            f"{base} 고객센터 문의 기록과 결제 로그를 함께 확인해 주세요.",
            f"{base} 환불/교환 정책 적용 시점도 주문 상태와 같이 안내해 주세요.",
        ]
    if category == "finance":
        return [
            text,
            f"{base} 추가로 현금성 자산과 단기 부채 만기 구조를 같이 검토해야 한다.",
            f"{base} 스트레스 시나리오에서는 금리·환율 동시 충격도 반영해야 한다.",
        ]
    if category == "tech":
        return [
            text,
            f"{base} 회귀 방지를 위해 동일 입력셋으로 benchmark와 test를 함께 돌린다.",
            f"{base} 메모리 사용량과 tail latency까지 같이 측정해야 병목을 정확히 찾을 수 있다.",
        ]
    if category == "longform":
        return [
            text,
            f"{base} 또한 운영 지표 해석에서는 평균값만 보지 말고 분산과 tail 구간을 함께 공개해야 한다.",
            f"{base} 마지막으로 실행 명령과 데이터셋 해시를 함께 제공해야 재현성과 검증 가능성이 확보된다.",
        ]
    return [text, f"{base} (variant-a {index})", f"{base} (variant-b {index})"]


def main() -> int:
    rows = parse_rows(SRC)
    if not rows:
        raise SystemExit(f"no rows found in {SRC}")

    out_lines = [
        "# SWE benchmark textset v2",
        "# source: swe_textset_v1.tsv + deterministic category-aware augmentation",
        "# format: category<TAB>text",
    ]
    for index, (category, text) in enumerate(rows):
        for variant in augment(category, text, index):
            out_lines.append(f"{category}\t{variant}")

    DST.parent.mkdir(parents=True, exist_ok=True)
    DST.write_text("\n".join(out_lines) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
