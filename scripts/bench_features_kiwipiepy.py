#!/usr/bin/env python3
import argparse
import time
from collections.abc import Iterable
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Feature-wise benchmark for kiwipiepy."
    )
    parser.add_argument(
        "--text",
        default="아버지가방에들어가신다.",
        help="Input text for single-sentence features.",
    )
    parser.add_argument(
        "--warmup",
        type=int,
        default=100,
        help="Warmup iterations.",
    )
    parser.add_argument(
        "--iters",
        type=int,
        default=5000,
        help="Iterations for single-sentence features.",
    )
    parser.add_argument(
        "--batch-size",
        type=int,
        default=256,
        help="Batch size for batch analyze feature.",
    )
    parser.add_argument(
        "--batch-iters",
        type=int,
        default=500,
        help="Iterations for batch analyze feature.",
    )
    parser.add_argument(
        "--join-lm-search",
        type=str,
        default="true",
        help="Whether join uses LM search (true/false).",
    )
    parser.add_argument(
        "--input-mode",
        choices=("repeated", "varied"),
        default="repeated",
        help="Use repeated identical input or varied rotating input pool.",
    )
    parser.add_argument(
        "--variant-pool",
        type=int,
        default=4096,
        help="Variant pool size for --input-mode varied.",
    )
    parser.add_argument(
        "--dataset-tsv",
        default="",
        help="Optional dataset TSV path (`category<TAB>text`, comments with #).",
    )
    parser.add_argument(
        "--dataset-category",
        default="",
        help="Optional dataset category filter (requires --dataset-tsv).",
    )
    args = parser.parse_args()
    if args.warmup < 0:
        parser.error("--warmup must be >= 0")
    if args.iters <= 0:
        parser.error("--iters must be >= 1")
    if args.batch_size <= 0:
        parser.error("--batch-size must be >= 1")
    if args.batch_iters <= 0:
        parser.error("--batch-iters must be >= 1")
    if args.variant_pool <= 0:
        parser.error("--variant-pool must be >= 1")
    if args.dataset_category and not args.dataset_tsv:
        parser.error("--dataset-category requires --dataset-tsv")
    raw_join_lm_search = str(args.join_lm_search).strip().lower()
    if raw_join_lm_search in {"1", "true", "yes", "on"}:
        args.join_lm_search = True
    elif raw_join_lm_search in {"0", "false", "no", "off"}:
        args.join_lm_search = False
    else:
        parser.error("--join-lm-search must be true/false")
    return args


def print_result(feature: str, avg_ms: float, calls_per_sec: float, sink: int, iters: int):
    print(
        f"feature={feature} avg_ms={avg_ms:.6f} calls_per_sec={calls_per_sec:.2f} sink={sink} iters={iters}"
    )


def run_bench(feature: str, warmup: int, iters: int, fn):
    for _ in range(warmup):
        fn()

    sink = 0
    start = time.perf_counter()
    for _ in range(iters):
        sink += fn()
    elapsed = time.perf_counter() - start
    avg_ms = (elapsed * 1000.0) / iters
    calls_per_sec = iters / elapsed
    print_result(feature, avg_ms, calls_per_sec, sink, iters)
    return avg_ms, calls_per_sec, sink


def first_candidate_token_len(candidates) -> int:
    if not candidates:
        return 0
    return len(candidates[0][0])


def sentence_payload_size(sentences) -> int:
    def visit(sentence) -> int:
        tokens = getattr(sentence, "tokens", None) or []
        subs = getattr(sentence, "subs", None) or []
        nested = sum(visit(sub) for sub in subs)
        return 1 + len(tokens) + len(subs) + nested

    return sum(visit(sentence) for sentence in sentences)


def materialize(iterable_or_value):
    if isinstance(iterable_or_value, (str, bytes)):
        return iterable_or_value
    if isinstance(iterable_or_value, Iterable):
        return list(iterable_or_value)
    return iterable_or_value


def text_sink(value: str) -> int:
    if not value:
        return 0
    return ord(value[0]) + ord(value[-1])


def build_text_variants(base: str, count: int) -> list[str]:
    return [f"{base} [{index}]" for index in range(count)]


def load_dataset_texts(path: str, category_filter: str = "") -> list[str]:
    file_path = Path(path)
    if not file_path.exists():
        raise FileNotFoundError(f"dataset file not found: {path}")
    texts: list[str] = []
    wanted = category_filter.strip().lower()
    for line_no, raw in enumerate(file_path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if "\t" in line:
            category, text = line.split("\t", 1)
            category = category.strip()
            text = text.strip()
        else:
            category = "default"
            text = line
        if wanted and category.lower() != wanted:
            continue
        if not text:
            raise ValueError(f"dataset line {line_no} has empty text")
        texts.append(text)
    if not texts:
        raise ValueError("dataset selection produced zero texts")
    return texts


def cycle_text_pool(seed: list[str], total: int) -> list[str]:
    return [seed[index % len(seed)] for index in range(total)]


def pick_text(mode: str, fallback: str, pool: list[str], index: int) -> str:
    if mode == "varied":
        return pool[index % len(pool)]
    return fallback


def round_batch_pool_size(value: int, batch_size: int) -> int:
    return ((value + batch_size - 1) // batch_size) * batch_size


def main() -> int:
    args = parse_args()

    try:
        from kiwipiepy import Kiwi, Match
    except Exception as exc:
        print(f"failed to import kiwipiepy: {exc}")
        return 1

    init_start = time.perf_counter()
    kiwi = Kiwi()
    init_ms = (time.perf_counter() - init_start) * 1000.0

    if args.dataset_tsv:
        single_variants = load_dataset_texts(args.dataset_tsv, args.dataset_category)
    else:
        single_variants = build_text_variants(args.text, args.variant_pool)
    if len(single_variants) > args.variant_pool:
        single_variants = single_variants[: args.variant_pool]

    split_text = single_variants[0] if args.dataset_tsv else f"{args.text} 두 번째 문장입니다."
    split_variants = (
        list(single_variants)
        if args.dataset_tsv
        else [f"{text} 두 번째 문장입니다." for text in single_variants]
    )
    join_seed_text = single_variants[0] if single_variants else args.text
    join_tokens = kiwi.tokenize(join_seed_text, match_options=Match.ALL)
    glue_chunks = ["오늘은", "날씨가", "좋다."]
    glue_variant_pool = [[text, "날씨가", "좋다."] for text in single_variants]
    batch_texts = (
        cycle_text_pool(single_variants, args.batch_size)
        if args.dataset_tsv
        else [f"{args.text} {i}" for i in range(args.batch_size)]
    )
    batch_pool_size = round_batch_pool_size(
        max(args.variant_pool, args.batch_size * 64),
        args.batch_size,
    )
    batch_text_pool = (
        cycle_text_pool(single_variants, batch_pool_size)
        if args.dataset_tsv
        else [f"{args.text} {i} {i % 17}" for i in range(batch_pool_size)]
    )
    batch_groups = [
        batch_text_pool[offset : offset + args.batch_size]
        for offset in range(0, len(batch_text_pool), args.batch_size)
    ]

    print("engine=kiwipiepy")
    print(f"text={args.text}")
    print(f"warmup={args.warmup}")
    print(f"iters={args.iters}")
    print(f"batch_size={args.batch_size}")
    print(f"batch_iters={args.batch_iters}")
    print(f"join_lm_search={str(args.join_lm_search).lower()}")
    print(f"input_mode={args.input_mode}")
    print(f"variant_pool={args.variant_pool}")
    print(f"dataset_tsv={args.dataset_tsv}")
    print(f"dataset_category={args.dataset_category}")
    print(f"dataset_entries={len(single_variants)}")
    print(f"init_ms={init_ms:.3f}")

    tokenize_round = [0]

    def tokenize_len():
        text = pick_text(args.input_mode, args.text, single_variants, tokenize_round[0])
        tokenize_round[0] += 1
        return len(kiwi.tokenize(text, match_options=Match.ALL))

    run_bench("tokenize", args.warmup, args.iters, tokenize_len)

    analyze_top1_round = [0]

    def analyze_top1_len():
        text = pick_text(
            args.input_mode,
            args.text,
            single_variants,
            analyze_top1_round[0],
        )
        analyze_top1_round[0] += 1
        candidates = kiwi.analyze(text, top_n=1, match_options=Match.ALL)
        return first_candidate_token_len(candidates)

    run_bench("analyze_top1", args.warmup, args.iters, analyze_top1_len)

    split_round = [0]

    def split_len():
        text = pick_text(args.input_mode, split_text, split_variants, split_round[0])
        split_round[0] += 1
        return len(kiwi.split_into_sents(text, match_options=Match.ALL))

    run_bench("split_into_sents", args.warmup, args.iters, split_len)

    split_with_tokens_round = [0]

    def split_with_tokens_len():
        text = pick_text(
            args.input_mode,
            split_text,
            split_variants,
            split_with_tokens_round[0],
        )
        split_with_tokens_round[0] += 1
        return sentence_payload_size(
            kiwi.split_into_sents(
                text,
                match_options=Match.ALL,
                return_tokens=True,
                return_sub_sents=True,
            )
        )

    run_bench("split_into_sents_with_tokens", args.warmup, args.iters, split_with_tokens_len)

    space_round = [0]

    def space_len():
        text = pick_text(args.input_mode, split_text, split_variants, space_round[0])
        space_round[0] += 1
        return text_sink(kiwi.space(text, reset_whitespace=True))

    run_bench("space", args.warmup, args.iters, space_len)

    run_bench(
        "join",
        args.warmup,
        args.iters,
        lambda: text_sink(kiwi.join(join_tokens, lm_search=args.join_lm_search)),
    )

    glue_round = [0]

    def glue_len():
        if args.input_mode == "varied":
            chunks = glue_variant_pool[glue_round[0] % len(glue_variant_pool)]
            glue_round[0] += 1
        else:
            chunks = glue_chunks
        return text_sink(kiwi.glue(chunks))

    run_bench("glue", args.warmup, args.iters, glue_len)

    analyze_many_loop_round = [0]

    def pick_batch(round_state: list[int]) -> list[str]:
        if args.input_mode == "varied":
            batch = batch_groups[round_state[0] % len(batch_groups)]
            round_state[0] += 1
            return batch
        return batch_texts

    def analyze_many_loop_len():
        batch = pick_batch(analyze_many_loop_round)
        total = 0
        for text in batch:
            candidates = kiwi.analyze(text, top_n=1, match_options=Match.ALL)
            total += first_candidate_token_len(candidates)
        return total

    run_bench("analyze_many_loop", args.warmup, args.batch_iters, analyze_many_loop_len)

    analyze_many_native_round = [0]

    def analyze_many_native_len():
        batch = pick_batch(analyze_many_native_round)
        all_candidates = materialize(
            kiwi.analyze(batch, top_n=1, match_options=Match.ALL)
        )
        total = 0
        for candidates in all_candidates:
            total += first_candidate_token_len(candidates)
        return total

    analyze_many_native_stats = run_bench(
        "analyze_many_native",
        args.warmup,
        args.batch_iters,
        analyze_many_native_len,
    )

    tokenize_many_loop_round = [0]

    def tokenize_many_loop_len():
        batch = pick_batch(tokenize_many_loop_round)
        total = 0
        for text in batch:
            total += len(kiwi.tokenize(text))
        return total

    run_bench("tokenize_many_loop", args.warmup, args.batch_iters, tokenize_many_loop_len)

    tokenize_many_batch_round = [0]

    def tokenize_many_batch_len():
        batch = pick_batch(tokenize_many_batch_round)
        batches = materialize(kiwi.tokenize(batch))
        return sum(len(tokens) for tokens in batches)

    run_bench("tokenize_many_batch", args.warmup, args.batch_iters, tokenize_many_batch_len)

    split_many_loop_round = [0]

    def split_many_loop_len():
        batch = pick_batch(split_many_loop_round)
        total = 0
        for text in batch:
            total += len(kiwi.split_into_sents(text, match_options=Match.ALL))
        return total

    run_bench("split_many_loop", args.warmup, args.batch_iters, split_many_loop_len)

    split_many_batch_round = [0]

    def split_many_batch_len():
        batch = pick_batch(split_many_batch_round)
        batches = materialize(kiwi.split_into_sents(batch, match_options=Match.ALL))
        return sum(len(sentences) for sentences in batches)

    run_bench("split_many_batch", args.warmup, args.batch_iters, split_many_batch_len)

    space_many_loop_round = [0]

    def space_many_loop_len():
        batch = pick_batch(space_many_loop_round)
        total = 0
        for text in batch:
            total += text_sink(kiwi.space(text, reset_whitespace=True))
        return total

    run_bench("space_many_loop", args.warmup, args.batch_iters, space_many_loop_len)

    space_many_batch_round = [0]

    def space_many_batch_len():
        batch = pick_batch(space_many_batch_round)
        batches = materialize(kiwi.space(batch, reset_whitespace=True))
        return sum(text_sink(text) for text in batches)

    run_bench("space_many_batch", args.warmup, args.batch_iters, space_many_batch_len)

    print_result(
        "batch_analyze_native",
        analyze_many_native_stats[0],
        analyze_many_native_stats[1],
        analyze_many_native_stats[2],
        args.batch_iters,
    )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
