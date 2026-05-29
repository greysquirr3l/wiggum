# T13 — Polish and Reliability

> **Depends on**: T12 complete.

## Goal

Production-quality error handling, UX improvements, and release distribution.

## Project Context

- This task is a batch of improvements, not a single feature
- Prioritize within the task: error handling stability → UX features → distribution
- Full architecture in `ARCHITECTURE.md`

## Error Handling & Stability

### E-1: Audit unwrap/expect

```bash
grep -rn "\.unwrap()\|\.expect(" crates/
```

Replace each one with proper propagation (`?`, `map_err`, `unwrap_or_default`, or a log + continue) unless it's a genuinely unrecoverable invariant (e.g., mutex poisoning).

### E-2: Retry logic in TeamsClient

Wrap all `reqwest` calls with exponential backoff using the `backon` crate:

```toml
# Add to yakko-infra/Cargo.toml
backon = "1"
```

```rust
use backon::{ExponentialBuilder, Retryable};

let response = (|| async {
    self.authed_get(path, token).send().await
})
.retry(ExponentialBuilder::default().with_max_times(3))
.await
.map_err(|e| DomainError::Network(e.to_string()))?;
```

### E-3: SIGINT/SIGTERM handler

In `main.rs`:

```rust
use tokio::signal;

tokio::select! {
    result = run(&mut terminal) => result,
    _ = signal::ctrl_c() => {
        tracing::info!("received Ctrl-C, exiting");
        Ok(())
    }
}
```

### E-4: Log rotation

At startup, if `yakko.log` exceeds 10 MB, rename it to `yakko.log.1` before creating a new one.

### E-5: Resize safety

After `AppEvent::Resize`, call `terminal.autoresize()` in the event loop and verify no state corruption.

## UX Improvements

### U-1: Unread badge

In `SidebarState`, track `unread_count: HashMap<ConversationId, u32>`.  
Increment on `DomainNotification::NewMessage` when a message arrives for a non-active conversation.  
Reset to 0 when the conversation is selected.

### U-2: Typing indicator

When `DomainNotification::TypingIndicator` arrives, set a timestamp; show `(typing…)` in the status bar for 5 seconds.

### U-3: Presence dot

In the sidebar, prefix each conversation with:

- `●` (green) for Available
- `○` (yellow) for Away/Busy
- `–` (red) for DoNotDisturb
- ` ` for Unknown/Offline

### U-4: Date separators in message list

When rendering messages, insert a `── Monday, Feb 24 ──` separator between messages from different calendar days.

### U-5: @mention highlight

Scan `Message.content` for `@DisplayName` patterns; render them with `Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)`.

### U-6: Config file

Create `~/.config/yakko/config.toml`:

```toml
[display]
timestamp_format = "%H:%M"
show_presence = true

[keybinds]
send = "Enter"
cancel = "Esc"
```

Use `toml = "0.8"` and `serde` to deserialize. Load at startup; fall back to defaults on any error. Never crash due to config issues.

## Distribution

### D-1: Release profile

`Cargo.toml` already has the release profile from T02. Verify:

```bash
cargo build --release && ls -lh target/release/yakko
```

Binary should be ≤ 20 MB.

### D-2: GitHub Actions release workflow

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags: ['v*']

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release --target ${{ matrix.target }}
      - uses: actions/upload-artifact@v4
        with:
          name: yakko-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/yakko
```

### D-3: README.md

Create a user-facing README with:

- What yakko is (one paragraph)
- Prerequisites (Teams account, Rust or pre-built binary)
- Install instructions
- First-run: how auth works (browser popup)
- Keybindings reference (copy from IMPLEMENTATION_PLAN.md keybinding table)
- Known limitations (Trouter protocol stability, no file attachments, etc.)

## Preflight

```bash
cargo build --release && cargo test --workspace && cargo clippy --workspace -- -D warnings
```

## Exit Criteria

- Zero `unwrap()`/`expect()` on error paths (invariants may keep them with an explanatory comment)
- `cargo build --release` produces a working binary
- README.md exists with install + first-run instructions
- Release workflow file exists

## After Completion

Update PROGRESS.md row for T13 to `[x]`.  
Commit: `feat: polish error handling, UX, and add release distribution`
