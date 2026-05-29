# T08 — Terminal Infrastructure

> **Depends on**: T07 complete.

## Goal

Implement terminal setup/teardown, panic recovery, and ensure all tracing output goes to a log file only (never stderr) while the TUI is active.

## Project Context

- TUI framework: `ratatui 0.30` + `crossterm 0.28`
- If the program panics without restoring the terminal, the user's shell becomes unusable
- `tracing` must write to a file sink during TUI operation — stderr output corrupts the ratatui display
- Log path: `~/.local/share/yakko/yakko.log` (macOS: `~/Library/Application Support/yakko/yakko.log`)
- Use the `dirs` crate (`dirs = "5"`) for platform-correct data directory

## Files to Create/Modify

### `crates/yakko-tui/src/ui/terminal.rs`

```rust
use std::io::{self, Stdout};
use anyhow::Result;
use crossterm::{
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    event::EnableMouseCapture,
    event::DisableMouseCapture,
};
use ratatui::{Terminal, backend::CrosstermBackend};

pub type Term = Terminal<CrosstermBackend<Stdout>>;

/// Set up raw mode + alternate screen. Returns a ratatui Terminal.
pub fn setup() -> Result<Term> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

/// Restore terminal to original state. Safe to call multiple times.
pub fn teardown(terminal: &mut Term) -> Result<()> {
    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Install a panic hook that always restores the terminal before printing the panic message.
pub fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Best-effort restore — ignore errors here
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            io::stderr(),
            LeaveAlternateScreen,
            DisableMouseCapture,
        );
        original_hook(panic_info);
    }));
}
```

### Update `crates/yakko-tui/src/main.rs`

```rust
use anyhow::Result;

mod app;
mod services;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging()?;
    ui::terminal::install_panic_hook();

    let mut terminal = ui::terminal::setup()?;

    // run the main application loop
    let result = run(&mut terminal).await;

    // Always restore terminal, even if run() errored
    let _ = ui::terminal::teardown(&mut terminal);

    result
}

async fn run(terminal: &mut ui::terminal::Term) -> Result<()> {
    use tokio::sync::mpsc;
    use app::state::AppState;
    use ui::event_loop::run_event_loop;

    let (event_tx, event_rx) = mpsc::channel(256);
    let state = AppState::default();

    // Spawn background services (implemented in T10)
    // services::start(event_tx.clone()).await?;

    run_event_loop(state, event_rx, |state| {
        let _ = terminal.draw(|frame| {
            ui::render::render(frame, state);
        });
    }).await
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

### `crates/yakko-tui/src/ui/render.rs` (stub)

```rust
use ratatui::Frame;
use crate::app::state::AppState;

pub fn render(frame: &mut Frame, state: &AppState) {
    // Implemented in T09
    let _ = (frame, state);
}
```

### `crates/yakko-tui/src/ui/mod.rs`

```rust
pub mod event_loop;
pub mod render;
pub mod terminal;
```

## Dependencies to Add

In `crates/yakko-tui/Cargo.toml`:

```toml
dirs = "5"
futures = "0.3"
```

## Manual Verification

After building:

1. Run `cargo run -p yakko-tui`
2. Verify the terminal enters alternate screen
3. Press `q` to quit
4. Verify the terminal is fully restored (prompt visible, no artifacts)
5. Check `~/.local/share/yakko/yakko.log` (or macOS equivalent) contains startup log line
6. Test panic recovery: temporarily add `panic!("test")` in main, run, verify terminal is restored

## Preflight

```bash
cargo build -p yakko-tui && cargo clippy -p yakko-tui -- -D warnings
```

## Exit Criteria

- Binary runs, enters alternate screen, restores terminal on `q`
- Panic hook is installed before `setup()`
- Log file path resolves correctly on macOS
- Zero warnings from clippy

## After Completion

Update PROGRESS.md row for T08 to `[x]`.  
Commit: `feat(tui): add terminal setup/teardown with panic recovery and file logging`
