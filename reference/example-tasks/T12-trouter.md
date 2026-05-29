# T12 — Trouter Real-Time Notifications

> **Depends on**: T11 complete AND Phase 0 task S-7 confirmed in `docs/api-notes.md`.

## Goal

Replace the 30-second polling loop with live WebSocket push delivery using Microsoft's Trouter protocol.

## Project Context

- Trouter is Microsoft's internal push notification relay for Teams
- The WebSocket URL is embedded in the `authsvc` response (the same response that gives us the skypetoken)
- The protocol is undocumented; the registration frame and event shapes are in `docs/api-notes.md`
- On WS failure: auto-reconnect with exponential backoff; after 5 attempts fall back to polling
- `NotificationPort` returns a `tokio::sync::broadcast::Receiver<DomainNotification>`
- Full architecture in `ARCHITECTURE.md § 8`

## Pre-Task Check

**Before implementing, read `docs/api-notes.md`** — specifically:

- The Trouter WS URL pattern
- The registration frame JSON
- The incoming event frame JSON

If S-7 is not yet completed (docs/api-notes.md is empty or missing Trouter section), mark this task `[!]` in PROGRESS.md and skip it.

## Implementation

### `crates/yakko-infra/src/trouter/client.rs`

```rust
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures::{SinkExt, StreamExt};
use yakko_domain::{
    ports::{NotificationPort, DomainNotification},
    value_objects::tokens::SkypeToken,
    error::{DomainError, DomainResult},
};
use super::parser;

const MAX_RECONNECT_ATTEMPTS: u32 = 5;
const HEARTBEAT_INTERVAL_SECS: u64 = 30;

pub struct TrouterClient {
    ws_url_template: String, // from authsvc response, e.g. "wss://trouter3.msg.skype.com/..."
    tx: broadcast::Sender<DomainNotification>,
}

impl TrouterClient {
    pub fn new(ws_url: String) -> Self {
        let (tx, _) = broadcast::channel(256);
        Self { ws_url_template: ws_url, tx }
    }

    async fn connect_and_listen(&self, token: &SkypeToken) -> DomainResult<()> {
        let (ws_stream, _) = connect_async(&self.ws_url_template)
            .await
            .map_err(|e| DomainError::Network(e.to_string()))?;

        let (mut write, mut read) = ws_stream.split();

        // Send registration frame (format from docs/api-notes.md)
        let registration = serde_json::json!({
            "skypetoken": token.as_str(),
        });
        write.send(Message::Text(registration.to_string()))
            .await
            .map_err(|e| DomainError::Network(e.to_string()))?;

        // Heartbeat task
        let mut heartbeat = tokio::time::interval(
            std::time::Duration::from_secs(HEARTBEAT_INTERVAL_SECS)
        );

        loop {
            tokio::select! {
                Some(msg) = read.next() => {
                    match msg {
                        Ok(Message::Text(text)) => {
                            match parser::parse_frame(&text) {
                                Ok(Some(notification)) => {
                                    let _ = self.tx.send(notification);
                                }
                                Ok(None) => {} // heartbeat ack or unknown frame — ignore
                                Err(e) => tracing::warn!("trouter parse error: {e}"),
                            }
                        }
                        Ok(Message::Close(_)) => {
                            tracing::info!("trouter WS closed by server");
                            return Err(DomainError::Network("connection closed".into()));
                        }
                        Err(e) => {
                            return Err(DomainError::Network(e.to_string()));
                        }
                        _ => {}
                    }
                }
                _ = heartbeat.tick() => {
                    let _ = write.send(Message::Ping(vec![])).await;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl NotificationPort for TrouterClient {
    async fn connect(&self, token: &SkypeToken) -> DomainResult<broadcast::Receiver<DomainNotification>> {
        let rx = self.tx.subscribe();

        // Spawn reconnect loop
        let client = self.clone_for_task();
        let token_str = token.as_str().to_string();
        let expires_at = token.expires_at();
        tokio::spawn(async move {
            let mut attempts = 0u32;
            loop {
                let t = SkypeToken::new(token_str.clone(), expires_at);
                match client.connect_and_listen(&t).await {
                    Ok(()) => break, // clean disconnect
                    Err(e) => {
                        attempts += 1;
                        if attempts >= MAX_RECONNECT_ATTEMPTS {
                            tracing::error!("trouter: max reconnect attempts reached, giving up");
                            break;
                        }
                        let delay = 2u64.pow(attempts.min(6));
                        tracing::warn!("trouter disconnected ({e}), retrying in {delay}s");
                        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn disconnect(&self) -> DomainResult<()> {
        Ok(()) // Drop of the sender closes the broadcast channel
    }
}
```

Note: `clone_for_task()` is a helper that creates a cheap clone sharing the broadcast sender.

### `crates/yakko-infra/src/trouter/parser.rs`

Parse raw WS text frames into `DomainNotification`.  
Refer to `docs/api-notes.md` for the exact JSON shape — implement based on what you find there.

```rust
use yakko_domain::ports::DomainNotification;

pub fn parse_frame(text: &str) -> anyhow::Result<Option<DomainNotification>> {
    // Parse based on docs/api-notes.md Trouter frame format
    // Return Ok(None) for frames that don't map to a DomainNotification
    todo!("implement based on docs/api-notes.md")
}
```

### `crates/yakko-infra/src/trouter/mod.rs`

```rust
pub mod client;
pub mod parser;
pub use client::TrouterClient;
```

### Update `crates/yakko-tui/src/services/chat.rs`

Subscribe to notifications and fan-out to AppEvent:

```rust
pub async fn start_realtime(&self, notification_rx: broadcast::Receiver<DomainNotification>) {
    let tx = self.event_tx.clone();
    let mut rx = notification_rx;
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(DomainNotification::NewMessage { conversation_id, message }) => {
                    let _ = tx.send(AppEvent::MessagesLoaded {
                        conversation_id,
                        messages: vec![message],
                    }).await;
                }
                Ok(DomainNotification::PresenceChanged { user_id, status }) => {
                    // handle in T13
                }
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("notification channel lagged by {n} messages");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
```

### Update status bar and polling

- Remove the 30-second polling interval from `ChatService::run_poll_loop`
- Replace with Trouter subscription
- Show `ConnectionStatus::Realtime` in the status bar when connected

Keep the polling loop available as a fallback — activate it if Trouter gives up after 5 reconnect attempts.

## Preflight

```bash
cargo build -p yakko-infra -p yakko-tui && cargo clippy --workspace -- -D warnings
```

## Exit Criteria

- New messages from other users appear within 2 seconds without manual refresh
- Status bar shows `● LIVE` when Trouter is connected
- Status bar shows `◷ POLL` when falling back to polling
- On WS disconnect: reconnect attempt is logged; UI continues working in polling mode

## After Completion

Update PROGRESS.md row for T12 to `[x]`.  
Commit: `feat(infra): implement Trouter WebSocket client for real-time notifications`
