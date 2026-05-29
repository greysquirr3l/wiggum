# T11 — Message Sending

> **Depends on**: T10 complete.

## Goal

Implement the compose input box and the full message send flow.

## Project Context

- Input widget: `ratatui-textarea 0.8`
- Modal input: press `i` to enter compose mode, `Esc` to cancel, `Enter` to send
- Optimistic UI: append message to local list immediately, roll back on failure
- `MessagePort::send_message` is in `yakko-infra::TeamsClient`
- Full architecture in `ARCHITECTURE.md § 9`

## Files to Create/Modify

### `crates/yakko-tui/src/ui/widgets/input_box.rs`

```rust
use ratatui::{Frame, layout::Rect};
use ratatui_textarea::TextArea;

pub struct InputBox<'a> {
    pub textarea: TextArea<'a>,
}

impl<'a> InputBox<'a> {
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Type a message… (Enter to send, Esc to cancel)");
        Self { textarea }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        use ratatui::widgets::{Block, Borders};
        self.textarea.set_block(
            Block::default().title("Compose").borders(Borders::ALL)
        );
        frame.render_widget(&self.textarea, area);
    }

    /// Returns the current content and clears the buffer.
    pub fn take_content(&mut self) -> String {
        let content = self.textarea.lines().join("\n");
        self.textarea = TextArea::default(); // reset
        content
    }

    pub fn is_empty(&self) -> bool {
        self.textarea.lines().iter().all(|l| l.is_empty())
    }
}
```

### Update `crates/yakko-tui/src/app/state.rs`

The `compose_buffer: String` field from T07 is replaced by the `InputBox` widget state in the render layer. In AppState, keep:

```rust
pub compose_content: String,  // mirrors textarea content for the reduce function
```

### Update `crates/yakko-tui/src/app/reduce.rs`

In `handle_compose_key`, add:

```rust
KeyCode::Enter if !key.modifiers.contains(KeyModifiers::ALT) => {
    // Trigger send — the actual send happens in the service layer,
    // but we optimistically update state immediately
    let content = state.compose_content.trim().to_string();
    if !content.is_empty() {
        // AppEvent::SendMessage is dispatched by the event loop
        // The reduce function just clears the buffer
        state.compose_content.clear();
        state.input_mode = InputMode::Navigation;
    }
}
```

The event loop detects `KeyCode::Enter` in Compose mode → dispatches `AppEvent::SendMessage(content)` → `ChatService::send_message` is called in a spawned task.

### Update `crates/yakko-tui/src/services/chat.rs`

Add send method:

```rust
pub async fn send_message(&self, conversation_id: ConversationId, content: String) {
    let guard = self.shared_token.lock().await;
    let Some(token) = guard.as_ref() else { return; };

    match self.message_port.send_message(token, &conversation_id, &content).await {
        Ok(message) => {
            let _ = self.event_tx.send(AppEvent::SendComplete(message)).await;
        }
        Err(e) => {
            tracing::error!("send_message failed: {e}");
            let _ = self.event_tx.send(AppEvent::SendFailed(e.to_string())).await;
        }
    }
}
```

### Update `crates/yakko-tui/src/ui/render.rs`

In `render_main`, add a compose row when in `InputMode::Compose`:

```rust
fn render_main(frame: &mut Frame, state: &AppState) {
    let has_compose = state.input_mode == InputMode::Compose;

    let body_constraints = if has_compose {
        vec![Constraint::Min(1), Constraint::Length(4), Constraint::Length(1)]
    } else {
        vec![Constraint::Min(1), Constraint::Length(1)]
    };

    let chunks = Layout::vertical(body_constraints).split(frame.area());
    // ... render sidebar + messages in chunks[0]
    // ... compose box in chunks[1] if has_compose
    // ... status bar in last chunk
}
```

Note: `TextArea` needs to be part of the render state (it holds cursor position).  
Create a separate `RenderState` struct in `ui/render.rs` that owns the `InputBox` and is kept alive across frames.

### Update `crates/yakko-tui/src/ui/event_loop.rs`

Pass compose key events through to the `TextArea` widget when in Compose mode:

```rust
if state.input_mode == InputMode::Compose {
    // Forward all keys to the textarea
    render_state.input_box.textarea.input(key);
    // Sync content to AppState
    state.compose_content = render_state.input_box.textarea.lines().join("\n");

    // Check for send trigger (plain Enter)
    if matches!(k.code, KeyCode::Enter) && !k.modifiers.contains(KeyModifiers::ALT) {
        let content = render_state.input_box.take_content();
        if !content.trim().is_empty() {
            let conv_id = state.messages.conversation_id.clone();
            if let Some(id) = conv_id {
                let tx = event_tx.clone();
                let svc = chat_service.clone();
                tokio::spawn(async move {
                    svc.send_message(id, content.trim().to_string()).await;
                });
            }
        }
    }
}
```

## Preflight

```bash
cargo build -p yakko-tui && cargo clippy -p yakko-tui -- -D warnings
```

## Exit Criteria

- Press `i` → compose box appears at bottom of screen
- Type a message, press `Enter` → message appears in the list
- Press `Esc` → compose box disappears, content preserved until next `i` focus (or cleared on send)
- On send failure: error appears in status bar, input buffer is NOT cleared
- `alt+Enter` inserts a newline in the compose box

## After Completion

Update PROGRESS.md row for T11 to `[x]`.  
Commit: `feat(tui): add compose input box and message send flow`
