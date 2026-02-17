#!/usr/bin/env python3
import argparse
import sys
import time


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Benchmark kiwipiepy init/tokenize latency with Rust benchmark-compatible fields."
    )
    parser.add_argument(
        "--text",
        default="아버지가방에들어가신다.",
        help="Input text for tokenization.",
    )
    parser.add_argument(
        "--warmup",
        type=int,
        default=50,
        help="Warmup tokenize iterations before measurement.",
    )
    parser.add_argument(
        "--iters",
        type=int,
        default=1000,
        help="Measured tokenize iterations.",
    )
    args = parser.parse_args()
    if args.warmup < 0:
        parser.error("--warmup must be >= 0")
    if args.iters <= 0:
        parser.error("--iters must be >= 1")
    return args


def main() -> int:
    args = parse_args()

    try:
        from kiwipiepy import Kiwi
    except Exception as exc:
        print(f"failed to import kiwipiepy: {exc}", file=sys.stderr)
        return 1

    init_start = time.perf_counter()
    kiwi = Kiwi()
    init_elapsed = time.perf_counter() - init_start

    first_start = time.perf_counter()
    first_tokens = kiwi.tokenize(args.text)
    first_elapsed = time.perf_counter() - first_start
    first_token_count = len(first_tokens)

    for _ in range(args.warmup):
        kiwi.tokenize(args.text)

    bench_start = time.perf_counter()
    total_tokens = 0
    for _ in range(args.iters):
        tokens = kiwi.tokenize(args.text)
        total_tokens += len(tokens)
    bench_elapsed = time.perf_counter() - bench_start

    avg_ms = (bench_elapsed * 1000.0) / args.iters
    calls_per_sec = args.iters / bench_elapsed
    tokens_per_sec = total_tokens / bench_elapsed

    print("engine=kiwipiepy")
    print(f"text={args.text}")
    print(f"warmup={args.warmup}")
    print(f"iters={args.iters}")
    print(f"first_token_count={first_token_count}")
    print(f"init_ms={init_elapsed * 1000.0:.3f}")
    print(f"first_tokenize_ms={first_elapsed * 1000.0:.3f}")
    print(f"bench_total_ms={bench_elapsed * 1000.0:.3f}")
    print(f"bench_avg_ms={avg_ms:.6f}")
    print(f"calls_per_sec={calls_per_sec:.2f}")
    print(f"tokens_per_sec={tokens_per_sec:.2f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
