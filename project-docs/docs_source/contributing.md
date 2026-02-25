# Contributing

Contributions to Wiggum are welcome.

## Building from source

```bash
git clone https://github.com/greysquirr3l/wiggum.git
cd wiggum
cargo build
```

## Running tests

```bash
cargo test --workspace
```

## Linting

```bash
cargo clippy --workspace -- -D warnings
cargo fmt -- --check
```

## Code style

- Rust edition 2024, MSRV 1.85
- Strict clippy: pedantic, nursery, cargo, and perf lints as warnings
- `unwrap()`, `expect()`, `panic!()`, and indexing with `[]` are denied — use `Result` and `.get()` instead
- Dual licensed under MIT and Apache-2.0

## Project structure

```
src/
├── adapters/    # CLI, filesystem, VCS, MCP server
├── domain/      # Plan model, DAG validation, language profiles, linting
├── generation/  # Template rendering, task/progress generation
├── error.rs     # Error types
├── ports.rs     # Port traits (hexagonal architecture)
├── lib.rs       # Library root
└── main.rs      # CLI entry point
```
