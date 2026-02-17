# Benchmark Defense Guide (SWE Review Lens)

This guide is a preemptive checklist for common objections in `kiwi-rs` vs `kiwipiepy` performance claims.

## 1. SWE Metrics

- `throughput (calls/sec)`: calls processed per second (higher is better).
- `latency (avg_ms)`: average time per call (lower is better).
- `startup (init_ms)`: initialization latency, interpreted separately from steady-state.
- `ratio`: `kiwi-rs / kiwipiepy`.
- `95% bootstrap CI`: confidence interval for ratio from resampling.
- `P(ratio > 1)`: estimated probability that Rust is faster.
- `sink parity`: integrity check that both sides processed equivalent workload.

## 2. Decision Rule (Recommended)

- Practical equivalence band: `±5%` (`[0.95, 1.05]`).
- `CI low > 1.05`: `kiwi-rs faster (robust)`.
- `CI high < 0.95`: `kiwipiepy faster (robust)`.
- `CI` entirely inside `[0.95, 1.05]`: `practically equivalent`.
- Otherwise: `inconclusive` or `likely faster` (avoid strong winner claims).

## 3. Common Reviewer Objections and Responses

- “This is cache-driven.”
  - Always publish both `input_mode=repeated` and `input_mode=varied`.
- “Order bias exists.”
  - Use `--engine-order alternate`.
- “Workload is not equivalent.”
  - Validate `sink parity`; use `--strict-sink-check` in CI.
- “Sample size is too small.”
  - Use `repeats >= 5` (recommended 7) and `bootstrap_samples >= 1000`.
- “Environment mismatch invalidates results.”
  - Publish OS/CPU/memory/rustc/python/kiwipiepy/Git SHA/dirty/KIWI path metadata.

## 4. Minimum Publication Checklist

- Same commands, same text, same options.
- Rust must run in `--release`.
- Report startup and steady-state separately.
- Report `ratio` with `95% CI` and `P(ratio>1)`.
- Do not publish winner claims when `sink` warnings remain unresolved.

## 5. Recommended Command Set

Use the command set in README section “Publication-grade benchmark run (recommended)”.

## 6. Dataset Reference

- Dataset spec: `docs/benchmark_dataset_spec.md`
- Dataset file (current): `benchmarks/datasets/swe_textset_v2.tsv`
