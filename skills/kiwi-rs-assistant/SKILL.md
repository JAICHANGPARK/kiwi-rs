---
name: kiwi-rs-assistant
description: Guide AI to use kiwi-rs accurately for Korean NLP in Rust. Use when tasks involve kiwi-rs setup, init/new/from_config selection, analyze/tokenize/split/join APIs, builder dictionary or typo customization, UTF-16 or batch APIs, kiwipiepy migration, or kiwi-rs runtime troubleshooting.
---

# Kiwi Rs Assistant

Use this skill to produce runnable, minimal, and correct `kiwi-rs` solutions.

## Workflow

1. Identify intent first.
- Map the request to one primary task: `tokenize`, `analyze`, `split`, `join/space/glue`, builder customization, typo correction, UTF-16 path, batch processing, or semantics APIs.
- Prefer the smallest API surface that satisfies the request.

2. Choose initialization path deliberately.
- Use `Kiwi::init()` for the easiest path with automatic bootstrap.
- Use `Kiwi::new()` when `KIWI_LIBRARY_PATH` and `KIWI_MODEL_PATH` are already managed by environment.
- Use `Kiwi::from_config(KiwiConfig::default()...)` when explicit, reproducible paths are required.
- Use `KiwiLibrary` + `builder(...)` when adding user words, regex rules, dictionaries, or typo sets before building `Kiwi`.

3. Generate runnable Rust code.
- Return complete snippets that compile (`fn main() -> Result<(), Box<dyn std::error::Error>>`).
- Import only required items from `kiwi_rs`.
- Keep text examples Korean unless the user requests otherwise.
- If the user asks for many features, stage the answer as a minimal baseline plus one extension block.

4. Add one validation step.
- Provide at least one command that confirms behavior (`cargo run --example ...` or `cargo run` for the snippet project).
- Include expected high-level output shape (for example: token count, candidate count, sentence boundaries).

5. Add focused troubleshooting guidance.
- Map failure messages to concrete fixes using `references/troubleshooting.md`.
- If bootstrap is used, mention required external commands: `curl`, `tar` (and `powershell` on Windows).

## Accuracy Guardrails

- Treat UTF-8 offsets as character indices (`str.chars()`), not byte indices.
- Check `supports_utf16_api()` before UTF-16 tokenization/sentence APIs.
- Check `supports_analyze_mw()` before `analyze_many_utf16_via_native`.
- Do not claim full `kiwipiepy` parity. Call out missing or partial areas (for example `Template`, `Stopwords`, and other Python/C++-specific layers).
- For deterministic spans or constraints, prefer `MorphemeSet` and `Pretokenized`.
- When discussing custom vocabulary or typo behavior, configure it in builder phase and rebuild `Kiwi`.

## Response Contract

Always structure answers in this order:
1. `Path`: chosen initialization and why.
2. `Code`: copy-paste runnable snippet.
3. `Verify`: one concrete command.
4. `Pitfalls`: only the risks relevant to the user request.

## References

- Read `references/task-to-api.md` to map user goals to APIs and local examples.
- Read `references/troubleshooting.md` when errors, environment issues, or parity confusion appear.
