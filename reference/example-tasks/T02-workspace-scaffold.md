# T02 — Workspace Scaffold

> **Depends on**: Phase 0 (T01) all complete.

## Goal

Compilable 3-crate Cargo workspace with logging infrastructure and CI pipeline.  
No business logic yet — just structure, stubs, and toolchain configuration.

## Project Context

- Project: `yakko` — a Microsoft Teams TUI client
- Rust edition 2024, stable toolchain
- Architecture: hexagonal / collapsed DDD-lite
- Three crates: `yakko-domain` (no I/O), `yakko-infra` (adapters), `yakko-tui` (binary + app layer)
- Full architecture in `ARCHITECTURE.md`

## Files to Create

### `/Cargo.toml` (workspace root)

```toml
[workspace]
members = [
    "crates/yakko-domain",
    "crates/yakko-infra",
    "crates/yakko-tui",
]
resolver = "2"

[workspace.package]
edition = "2024"
version = "0.1.0"
authors = ["Yakko Contributors"]
license = "MIT"

[workspace.dependencies]
# Core async
tokio = { version = "1", features = ["full"] }
# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
# Error handling
thiserror = "2"
anyhow = "1"
# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
# Async traits
async-trait = "0.1"
# Domain
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
zeroize = { version = "1", features = ["zeroize_derive"] }
# HTTP
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
url = "2"
# TUI
ratatui = "0.30"
crossterm = "0.28"
ratatui-textarea = "0.8"
# CLI
clap = { version = "4", features = ["derive"] }
# OS keychain
keyring = "3"
# Browser open
open = "5"
# WebSocket
tokio-tungstenite = { version = "0.24", features = ["rustls-tls-native-roots"] }

[profile.release]
lto = true
codegen-units = 1
strip = true
```

### `crates/yakko-domain/Cargo.toml`

```toml
[package]
name = "yakko-domain"
version.workspace = true
edition.workspace = true

[dependencies]
thiserror.workspace = true
serde.workspace = true
uuid.workspace = true
chrono.workspace = true
zeroize.workspace = true
async-trait.workspace = true
```

### `crates/yakko-infra/Cargo.toml`

```toml
[package]
name = "yakko-infra"
version.workspace = true
edition.workspace = true

[dependencies]
yakko-domain = { path = "../yakko-domain" }
tokio.workspace = true
reqwest.workspace = true
serde.workspace = true
serde_json.workspace = true
keyring.workspace = true
tracing.workspace = true
url.workspace = true
anyhow.workspace = true
async-trait.workspace = true
open.workspace = true
tokio-tungstenite.workspace = true
```

### `crates/yakko-tui/Cargo.toml`

```toml
[package]
name = "yakko-tui"
version.workspace = true
edition.workspace = true

[[bin]]
name = "yakko"
path = "src/main.rs"

[dependencies]
yakko-domain = { path = "../yakko-domain" }
yakko-infra = { path = "../yakko-infra" }
tokio.workspace = true
ratatui.workspace = true
crossterm.workspace = true
ratatui-textarea.workspace = true
clap.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
anyhow.workspace = true
```

### Stub source files

**`crates/yakko-domain/src/lib.rs`**:

```rust
pub mod error;
pub mod ports;
pub mod entities;
pub mod value_objects;
```

**`crates/yakko-infra/src/lib.rs`**:

```rust
pub mod auth;
pub mod teams_api;
pub mod token_store;
pub mod trouter;
```

**`crates/yakko-tui/src/main.rs`**:

```rust
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging()?;
    tracing::info!("yakko starting");
    Ok(())
}

fn init_logging() -> Result<()> {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};
    use std::fs;

    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("yakko");
    fs::create_dir_all(&log_dir)?;
    let log_file = fs::File::options()
        .append(true)
        .create(true)
        .open(log_dir.join("yakko.log"))?;

    tracing_subscriber::registry()
        .with(EnvFilter::from_env("YAKKO_LOG"))
        .with(fmt::layer().with_writer(log_file).with_ansi(false))
        .init();

    Ok(())
}
```

You'll also need to add `dirs = "5"` to yakko-tui's dependencies for the log path.

### CI pipeline

**`.github/workflows/ci.yml`**:

```yaml
name: CI

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --workspace
      - run: cargo test --workspace
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo fmt --check
```

## Preflight

```bash
cargo build --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check
```

## Exit Criteria

- `cargo build --workspace` succeeds with zero warnings
- All three crates have stub source files that compile
- CI workflow file exists at `.github/workflows/ci.yml`
- Log file is written to `~/.local/share/yakko/yakko.log` on macOS/Linux (verify path resolves)

## After Completion

Update PROGRESS.md row for T02 to `[x]`.  
Commit message example: `feat: initialize yakko workspace with 3-crate scaffold and CI`
