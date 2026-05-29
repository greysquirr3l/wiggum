# T07 — App State + Event Loop

> **Depends on**: T06 complete.

## Goal

Implement the application state machine at the core of `yakko-tui`.  
This is a pure Elm/Redux-style unidirectional data flow:  
**Event → reduce(state, event) → new state → render**

## Project Context

- Crate: `yakko-tui`
- TUI framework: `ratatui 0.30`, runtime: `tokio 1`
- The reduce function must be a **pure function** — no I/O  
- I/O (Teams API calls) happens in background tokio tasks that send events via mpsc channel
- Full architecture in `ARCHITECTURE.md § 9`

## Files to Create

### `crates/yakko-tui/src/app/state.rs`

```rust
use yakko_domain::entities::{
    team::Team,
    conversation::Conversation,
    message::Message,
};
use yakko_domain::value_objects::ids::ConversationId;
use yakko_domain::value_objects::tokens::SkypeToken;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum AppScreen {
    #[default]
    Loading,
    Login { auth_url: String },
    Main,
    Help,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Navigation,
    Compose,
}

#[derive(Debug, Default)]
pub struct SidebarState {
    pub teams: Vec<Team>,
    pub conversations: Vec<Conversation>,
    pub selected_index: usize,
    pub focused: bool,
}

#[derive(Debug, Default)]
pub struct MessagesState {
    pub conversation_id: Option<ConversationId>,
    pub messages: Vec<Message>,
    pub scroll_offset: usize,
}

#[derive(Debug, Default)]
pub struct AppState {
    pub screen: AppScreen,
    pub input_mode: InputMode,
    pub sidebar: SidebarState,
    pub messages: MessagesState,
    pub compose_buffer: String,
    pub status_message: Option<String>,  // error/info shown in status bar
    pub is_loading: bool,
    pub connection_status: ConnectionStatus,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Polling,
    Realtime,
}
```

Note: `SkypeToken` is stored elsewhere (in the service layer, not in AppState) because it cannot be Cloned.

### `crates/yakko-tui/src/app/events.rs`

```rust
use yakko_domain::entities::{
    team::Team,
    conversation::Conversation,
    message::Message,
};
use yakko_domain::value_objects::ids::ConversationId;
use crossterm::event::KeyEvent;

#[derive(Debug)]
pub enum AppEvent {
    // Terminal input
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,

    // Auth
    AuthRequired { auth_url: String },
    AuthSuccess,

    // Data loading
    TeamsLoaded(Vec<Team>),
    ConversationsLoaded(Vec<Conversation>),
    MessagesLoaded {
        conversation_id: ConversationId,
        messages: Vec<Message>,
    },

    // User actions
    SelectConversation(ConversationId),
    SendMessage(String),
    SendComplete(Message),
    SendFailed(String),

    // Connection
    ConnectionChanged(ConnectionStatus),

    // Shutdown
    Quit,
}
```

### `crates/yakko-tui/src/app/reduce.rs`

A **pure function** — no `async`, no I/O, no unwrap on network calls.

```rust
use super::{state::*, events::AppEvent};
use crossterm::event::{KeyCode, KeyModifiers};

pub fn reduce(state: &mut AppState, event: AppEvent) {
    match event {
        AppEvent::AuthRequired { auth_url } => {
            state.screen = AppScreen::Login { auth_url };
        }
        AppEvent::AuthSuccess => {
            state.screen = AppScreen::Loading;
            state.is_loading = true;
        }
        AppEvent::TeamsLoaded(teams) => {
            state.sidebar.teams = teams;
            state.is_loading = false;
            state.screen = AppScreen::Main;
        }
        AppEvent::ConversationsLoaded(conversations) => {
            state.sidebar.conversations = conversations;
        }
        AppEvent::MessagesLoaded { conversation_id, messages } => {
            state.messages.conversation_id = Some(conversation_id);
            state.messages.messages = messages;
            state.messages.scroll_offset = 0;
        }
        AppEvent::SendComplete(msg) => {
            state.messages.messages.push(msg);
            state.compose_buffer.clear();
            state.status_message = None;
        }
        AppEvent::SendFailed(err) => {
            state.status_message = Some(format!("Send failed: {err}"));
        }
        AppEvent::ConnectionChanged(status) => {
            state.connection_status = status;
        }
        AppEvent::Key(key) => handle_key(state, key),
        AppEvent::Quit => { /* handled by event loop */ }
        _ => {}
    }
}

fn handle_key(state: &mut AppState, key: crossterm::event::KeyEvent) {
    match state.input_mode {
        InputMode::Navigation => handle_nav_key(state, key),
        InputMode::Compose => handle_compose_key(state, key),
    }
}

fn handle_nav_key(state: &mut AppState, key: crossterm::event::KeyEvent) {
    use KeyCode::*;
    match (key.modifiers, key.code) {
        (_, Char('q')) | (KeyModifiers::CONTROL, Char('c')) => { /* quit */ }
        (_, Char('?')) => {
            state.screen = if state.screen == AppScreen::Help {
                AppScreen::Main
            } else {
                AppScreen::Help
            };
        }
        (_, Char('i')) => {
            state.input_mode = InputMode::Compose;
        }
        (_, Char('j')) | (_, Down) => {
            // scroll down in focused panel
        }
        (_, Char('k')) | (_, Up) => {
            // scroll up
        }
        _ => {}
    }
}

fn handle_compose_key(state: &mut AppState, key: crossterm::event::KeyEvent) {
    use KeyCode::*;
    match key.code {
        Esc => {
            state.input_mode = InputMode::Navigation;
        }
        Backspace => {
            state.compose_buffer.pop();
        }
        Char(c) => {
            state.compose_buffer.push(c);
        }
        _ => {}
    }
}
```

Note: The `Enter` key in compose mode (to send) is wired in T11.

### `crates/yakko-tui/src/ui/event_loop.rs`

```rust
use std::time::Duration;
use tokio::sync::mpsc;
use crossterm::event::{self, EventStream};
use futures::StreamExt;
use crate::app::{state::AppState, events::AppEvent, reduce::reduce};

pub async fn run_event_loop(
    mut app_state: AppState,
    event_rx: mpsc::Receiver<AppEvent>,
    // render closure injected so event loop stays testable
    mut render_fn: impl FnMut(&AppState),
) -> anyhow::Result<()> {
    let tick_rate = Duration::from_millis(33); // ~30fps
    let mut terminal_stream = EventStream::new();
    let mut event_rx = event_rx;
    let mut tick = tokio::time::interval(tick_rate);

    loop {
        render_fn(&app_state);

        tokio::select! {
            // Terminal keyboard/resize events
            Some(Ok(crossterm_event)) = terminal_stream.next() => {
                match crossterm_event {
                    event::Event::Key(k) => {
                        let evt = AppEvent::Key(k);
                        // Check quit before reducing
                        if matches!(k.code, crossterm::event::KeyCode::Char('q'))
                            && app_state.input_mode == crate::app::state::InputMode::Navigation {
                            break;
                        }
                        reduce(&mut app_state, evt);
                    }
                    event::Event::Resize(w, h) => {
                        reduce(&mut app_state, AppEvent::Resize(w, h));
                    }
                    _ => {}
                }
            }
            // App events from background tasks
            Some(evt) = event_rx.recv() => {
                if matches!(evt, AppEvent::Quit) { break; }
                reduce(&mut app_state, evt);
            }
            // Tick
            _ = tick.tick() => {
                reduce(&mut app_state, AppEvent::Tick);
            }
        }
    }

    Ok(())
}
```

Add `futures = "0.3"` to `yakko-tui/Cargo.toml` and `crossterm = { version = "0.28", features = ["event-stream"] }`.

### `crates/yakko-tui/src/app/mod.rs`

```rust
pub mod events;
pub mod reduce;
pub mod state;
```

## Tests

Add unit tests for `reduce` in `reduce.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_required_sets_login_screen() {
        let mut state = AppState::default();
        reduce(&mut state, AppEvent::AuthRequired { auth_url: "https://example.com".into() });
        assert!(matches!(state.screen, AppScreen::Login { .. }));
    }

    #[test]
    fn send_failed_sets_status_message() {
        let mut state = AppState::default();
        reduce(&mut state, AppEvent::SendFailed("timeout".into()));
        assert!(state.status_message.is_some());
    }
}
```

## Preflight

```bash
cargo test -p yakko-tui && cargo clippy -p yakko-tui -- -D warnings
```

## Exit Criteria

- `reduce` is a pure function (no I/O, no async)
- Unit tests for at least 5 reduce cases pass
- Event loop compiles with `tokio::select!`

## After Completion

Update PROGRESS.md row for T07 to `[x]`.  
Commit: `feat(tui): implement app state machine with reduce and event loop`
