# T10 — Application Services (Read Path)

> **Depends on**: T09 complete.

## Goal

Wire the infra adapters to the TUI via `ChatService` and `AuthService`.  
After this task: authenticate, browse teams/channels, read messages (polling only, no Trouter yet).

## Project Context

- Services run in background tokio tasks; they communicate with the TUI via `mpsc::Sender<AppEvent>`
- `SkypeToken` cannot be `Clone` — store in an `Arc<Mutex<Option<SkypeToken>>>` shared between services
- Auth adapter: `AadAuthAdapter` from `yakko-infra`
- Teams client: `TeamsClient` from `yakko-infra`
- `AppEvent` is defined in `yakko-tui::app::events`
- Full architecture in `ARCHITECTURE.md § 8`

## Files to Create

### `crates/yakko-tui/src/services/mod.rs`

```rust
pub mod auth;
pub mod chat;

use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use yakko_domain::value_objects::tokens::SkypeToken;
use crate::app::events::AppEvent;

pub type EventTx = mpsc::Sender<AppEvent>;
pub type SharedToken = Arc<Mutex<Option<SkypeToken>>>;

pub async fn start(event_tx: EventTx) -> anyhow::Result<()> {
    let shared_token: SharedToken = Arc::new(Mutex::new(None));

    // 1. Auth service: check keychain, trigger login if needed
    let auth_service = auth::AuthService::new(
        Arc::new(yakko_infra::auth::AadAuthAdapter::new(
            reqwest::Client::new(),
        )),
        Arc::new(yakko_infra::token_store::KeyringAdapter::new()),
        shared_token.clone(),
        event_tx.clone(),
    );
    tokio::spawn(auth_service.run());

    // 2. Chat service starts after auth signals success via AuthSuccess event
    // (The event loop will re-invoke start_chat_service once AuthSuccess is received)
    // For Phase 4, wire this manually or trigger from the event loop.

    Ok(())
}
```

### `crates/yakko-tui/src/services/auth.rs`

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use yakko_domain::{ports::{AuthPort, TokenStorePort}, value_objects::tokens::SkypeToken};
use crate::app::events::AppEvent;
use super::{EventTx, SharedToken};

const DEFAULT_UPN: &str = "me"; // replaced with actual UPN post-login

pub struct AuthService {
    auth_port: Arc<dyn AuthPort>,
    token_store: Arc<dyn TokenStorePort>,
    shared_token: SharedToken,
    event_tx: EventTx,
}

impl AuthService {
    pub fn new(
        auth_port: Arc<dyn AuthPort>,
        token_store: Arc<dyn TokenStorePort>,
        shared_token: SharedToken,
        event_tx: EventTx,
    ) -> Self {
        Self { auth_port, token_store, shared_token, event_tx }
    }

    pub async fn run(self) {
        match self.token_store.load_token(DEFAULT_UPN).await {
            Ok(Some(token)) if !token.is_expired() => {
                tracing::info!("loaded existing token from keychain");
                *self.shared_token.lock().await = Some(token);
                let _ = self.event_tx.send(AppEvent::AuthSuccess).await;
            }
            _ => {
                tracing::info!("no valid token found, triggering login");
                match self.auth_port.login().await {
                    Ok(token) => {
                        let _ = self.token_store.save_token(DEFAULT_UPN, &token).await;
                        *self.shared_token.lock().await = Some(token);
                        let _ = self.event_tx.send(AppEvent::AuthSuccess).await;
                    }
                    Err(e) => {
                        tracing::error!("login failed: {e}");
                        let _ = self.event_tx.send(AppEvent::SendFailed(e.to_string())).await;
                    }
                }
            }
        }
    }
}
```

### `crates/yakko-tui/src/services/chat.rs`

```rust
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use yakko_domain::ports::{ConversationPort, MessagePort};
use yakko_domain::value_objects::ids::ConversationId;
use crate::app::events::AppEvent;
use super::{EventTx, SharedToken};

pub struct ChatService {
    conversation_port: Arc<dyn ConversationPort>,
    message_port: Arc<dyn MessagePort>,
    shared_token: SharedToken,
    event_tx: EventTx,
}

impl ChatService {
    pub fn new(
        conversation_port: Arc<dyn ConversationPort>,
        message_port: Arc<dyn MessagePort>,
        shared_token: SharedToken,
        event_tx: EventTx,
    ) -> Self {
        Self { conversation_port, message_port, shared_token, event_tx }
    }

    pub async fn load_initial_data(&self) {
        let guard = self.shared_token.lock().await;
        let Some(token) = guard.as_ref() else {
            tracing::warn!("load_initial_data called without a token");
            return;
        };

        match self.conversation_port.list_teams(token).await {
            Ok(teams) => { let _ = self.event_tx.send(AppEvent::TeamsLoaded(teams)).await; }
            Err(e) => { tracing::error!("list_teams failed: {e}"); }
        }

        match self.conversation_port.list_conversations(token).await {
            Ok(convos) => { let _ = self.event_tx.send(AppEvent::ConversationsLoaded(convos)).await; }
            Err(e) => { tracing::error!("list_conversations failed: {e}"); }
        }
    }

    pub async fn load_messages(&self, conversation_id: ConversationId) {
        let guard = self.shared_token.lock().await;
        let Some(token) = guard.as_ref() else { return; };

        match self.message_port.get_messages(token, &conversation_id, 50).await {
            Ok(messages) => {
                let _ = self.event_tx.send(AppEvent::MessagesLoaded {
                    conversation_id,
                    messages,
                }).await;
            }
            Err(e) => { tracing::error!("get_messages failed: {e}"); }
        }
    }

    /// Polling loop — runs until the channel closes.
    pub async fn run_poll_loop(
        self: Arc<Self>,
        mut active_conversation: tokio::sync::watch::Receiver<Option<ConversationId>>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Some(id) = active_conversation.borrow().clone() {
                self.load_messages(id).await;
            }
        }
    }
}
```

### Update `crates/yakko-tui/src/main.rs`

Uncomment `services::start(event_tx.clone()).await?;` and integrate `ChatService` startup on `AuthSuccess` event.  
After `TeamsLoaded`, trigger `load_initial_data`.

Wire a `tokio::sync::watch` channel for the active conversation ID so the poll loop knows which conversation to refresh.

## Dependencies to Add

In `yakko-tui/Cargo.toml`:

```toml
reqwest = { workspace = true }
yakko-infra = { path = "../yakko-infra" }
```

## Preflight

```bash
cargo build -p yakko-tui && cargo clippy -p yakko-tui -- -D warnings
```

## Exit Criteria

- Running `cargo run -p yakko-tui` triggers browser login (or loads token from keychain)
- After auth: teams/conversations list appears in the sidebar
- Selecting a conversation (Enter key) loads and displays messages
- Polling loop runs every 30 seconds in the background
- Terminal fully restores on `q`

## After Completion

Update PROGRESS.md row for T10 to `[x]`.  
Commit: `feat(tui): wire auth and chat services for read-only Teams browsing`
