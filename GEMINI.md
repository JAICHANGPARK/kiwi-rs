# GEMINI.md - Project Context for `kiwi-rs`

This file provides instructional context and an overview of the `kiwi-rs` project for AI interactions.

## Project Overview

`kiwi-rs` provides ergonomic Rust bindings for the **Kiwi Korean morphological analyzer** via its official C API. It aims to provide a high-level, idiomatic Rust interface while maintaining near-native performance.

### Main Technologies
- **Language:** Rust (Edition 2021, Minimum Supported Rust Version: 1.70)
- **FFI:** Dynamic loading of the Kiwi C library (`libkiwi`) using platform-specific APIs (dlopen/dlsym on Unix, LoadLibrary/GetProcAddress on Windows).
- **Core Dependencies:** `regex` (for rule-based tokenization).
- **Scripting:** Python (for benchmarks) and Shell/PowerShell (for installation).

### Architecture
- **`src/lib.rs`**: The main entry point, re-exporting key types and modules.
- **`src/runtime.rs`**: Implements the high-level Rust API (`Kiwi`, `KiwiBuilder`, `KiwiTypo`, `SwTokenizer`).
- **`src/native.rs`**: Contains low-level FFI bindings, raw struct definitions, and the `KiwiApi` loader that maps C symbols to Rust function pointers.
- **`src/bootstrap.rs`**: Handles automatic discovery and downloading of the Kiwi library and models if they are not found locally.
- **`src/config.rs`**: Defines handle types and internal configuration management.
- **`src/types.rs`**: Contains data structures like `Token`, `Sentence`, `AnalysisCandidate`, and configuration objects.
- **`examples/`**: Comprehensive set of examples demonstrating initialization, basic tokenization, custom dictionaries, typo correction, and benchmarking.

## Building and Running

### Common Cargo Commands
- **Build:** `cargo build`
- **Test:** `cargo test`
- **Lint/Check:** `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
- **Examples:** `cargo run --example <name>` (e.g., `cargo run --example basic`)

### Setup and Installation
The project provides helper scripts and a `Makefile` to install the required Kiwi library and models.

- **macOS/Linux:** `make install-kiwi`
- **Windows:** `make install-kiwi-win`

Alternatively, `Kiwi::init()` can automatically bootstrap the environment by downloading assets to the user's cache directory.

### Environment Variables
- `KIWI_LIBRARY_PATH`: Path to the dynamic library (e.g., `libkiwi.so`, `libkiwi.dylib`, `kiwi.dll`).
- `KIWI_MODEL_PATH`: Path to the directory containing Kiwi model files.
- `KIWI_RS_VERSION`: Desired Kiwi version for bootstrapping (defaults to `latest`).
- `KIWI_RS_CACHE_DIR`: Custom directory for cached library/models.

## Development Conventions

### Coding Style
- **Standard Rust:** Adheres to standard Rust formatting (`rustdoc`, `camelCase` for variables, `PascalCase` for types).
- **Documentation:** Public APIs must be documented. The project enforces `#![deny(missing_docs)]`.
- **Safety:** Uses `unsafe` blocks for FFI calls, but wraps them in safe Rust abstractions.

### Testing Practices
- **Unit Tests:** Located in `src/tests.rs` and within various modules.
- **Integration Tests:** Located in `tests/integration_tests.rs`.
- **Safety Checks:** `tests/safety_check.rs` ensures memory safety and correct FFI behavior.

### API Specifics
- **Offsets:** In UTF-8 APIs, offsets are **character indices** (based on `str.chars()`), not byte indices.
- **Parity:** The project strives for parity with the official Python bindings (`kiwipiepy`), documented in `docs/kiwipiepy_parity.md`.

## Key Files
- `Cargo.toml`: Project metadata and dependencies.
- `README.md`: Comprehensive guide and benchmark results.
- `src/native.rs`: The bridge between Rust and the C API.
- `src/runtime.rs`: The primary interface for users.
- `Makefile`: Convenient shortcuts for installation and quality checks.
