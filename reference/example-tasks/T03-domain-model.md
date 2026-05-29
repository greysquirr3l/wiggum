# T03 — Domain Model

> **Depends on**: T02 complete.

## Goal

Implement all domain entities, value objects, port traits, and error types in `yakko-domain`.  
This crate must have **zero I/O dependencies** — no `tokio`, no `reqwest`, no `keyring`.

## Project Context

- Project: `yakko` — a Microsoft Teams TUI client
- Architecture: hexagonal. The domain crate defines the core model and port interfaces.
- Adapters (infra crate) implement the port traits; the domain crate never depends on them.
- Full architecture in `ARCHITECTURE.md`

## Files to Create

### `crates/yakko-domain/src/error.rs`

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("authentication error: {0}")]
    Auth(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("token expired")]
    TokenExpired,
    #[error("token store error: {0}")]
    TokenStore(String),
    #[error("API error {status}: {message}")]
    Api { status: u16, message: String },
    #[error("unknown error: {0}")]
    Unknown(String),
}

pub type DomainResult<T> = Result<T, DomainError>;
```

### `crates/yakko-domain/src/value_objects/ids.rs`

Newtype wrappers for all ID types. Each must be:

- `#[repr(transparent)]`
- Derives: `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`
- Implements `Display` and `FromStr`
- Has a `fn new(s: impl Into<String>) -> Self` constructor
- Has a `fn as_str(&self) -> &str` accessor

Create: `TeamId`, `ChannelId`, `ConversationId`, `MessageId`, `UserId`.

### `crates/yakko-domain/src/value_objects/tokens.rs`

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};
use chrono::{DateTime, Utc};

/// AAD access token. Zeroed on drop. No Clone, no Debug.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct AadToken(String);

impl AadToken {
    pub fn new(token: String) -> Self { Self(token) }
    pub fn as_str(&self) -> &str { &self.0 }
}

/// Teams internal skypetoken. Zeroed on drop. No Clone, no Debug.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SkypeToken {
    token: String,
    expires_at: DateTime<Utc>,
}

impl SkypeToken {
    pub fn new(token: String, expires_at: DateTime<Utc>) -> Self {
        Self { token, expires_at }
    }
    pub fn as_str(&self) -> &str { &self.token }
    pub fn is_expired(&self) -> bool { Utc::now() >= self.expires_at }
    pub fn expires_at(&self) -> DateTime<Utc> { self.expires_at }
}
```

### `crates/yakko-domain/src/entities/user.rs`

```rust
use crate::value_objects::ids::UserId;
use crate::entities::presence::PresenceStatus;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub display_name: String,
    pub email: String,
    pub presence: PresenceStatus,
}
```

### `crates/yakko-domain/src/entities/presence.rs`

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PresenceStatus {
    Available,
    Away,
    Busy,
    DoNotDisturb,
    #[default]
    Unknown,
    Offline,
}
```

### `crates/yakko-domain/src/entities/team.rs`

```rust
use crate::value_objects::ids::{TeamId, ChannelId};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: TeamId,
    pub display_name: String,
    pub channels: Vec<Channel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: ChannelId,
    pub display_name: String,
    pub unread_count: u32,
}
```

### `crates/yakko-domain/src/entities/conversation.rs`

```rust
use crate::value_objects::ids::{ConversationId, UserId};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConversationType {
    DirectMessage,
    GroupChat,
    Channel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: ConversationId,
    pub display_name: String,
    pub conversation_type: ConversationType,
    pub participants: Vec<UserId>,
    pub unread_count: u32,
}
```

### `crates/yakko-domain/src/entities/message.rs`

```rust
use crate::value_objects::ids::{MessageId, ConversationId, UserId};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    Plain(String),
    Html(String),  // stripped to plain for display in v1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub conversation_id: ConversationId,
    pub sender_id: UserId,
    pub sender_display_name: String,
    pub content: MessageContent,
    pub timestamp: DateTime<Utc>,
    pub is_mine: bool,
    pub edited: bool,
}
```

### `crates/yakko-domain/src/ports/mod.rs`

Six port traits. Use `#[async_trait::async_trait]` on each.

**`AuthPort`**:

```rust
#[async_trait::async_trait]
pub trait AuthPort: Send + Sync {
    /// Initiates browser-based PKCE login and returns a SkypeToken.
    async fn login(&self) -> DomainResult<SkypeToken>;
    /// Refreshes an expired SkypeToken if possible.
    async fn refresh(&self, token: &SkypeToken) -> DomainResult<SkypeToken>;
}
```

**`TokenStorePort`**:

```rust
#[async_trait::async_trait]
pub trait TokenStorePort: Send + Sync {
    async fn save_token(&self, upn: &str, token: &SkypeToken) -> DomainResult<()>;
    async fn load_token(&self, upn: &str) -> DomainResult<Option<SkypeToken>>;
    async fn clear_token(&self, upn: &str) -> DomainResult<()>;
}
```

**`ConversationPort`**:

```rust
#[async_trait::async_trait]
pub trait ConversationPort: Send + Sync {
    async fn list_conversations(&self, token: &SkypeToken) -> DomainResult<Vec<Conversation>>;
    async fn list_teams(&self, token: &SkypeToken) -> DomainResult<Vec<Team>>;
}
```

**`MessagePort`**:

```rust
#[async_trait::async_trait]
pub trait MessagePort: Send + Sync {
    async fn get_messages(
        &self,
        token: &SkypeToken,
        conversation_id: &ConversationId,
        limit: usize,
    ) -> DomainResult<Vec<Message>>;

    async fn send_message(
        &self,
        token: &SkypeToken,
        conversation_id: &ConversationId,
        content: &str,
    ) -> DomainResult<Message>;
}
```

**`PresencePort`**:

```rust
#[async_trait::async_trait]
pub trait PresencePort: Send + Sync {
    async fn get_presence(
        &self,
        token: &SkypeToken,
        user_ids: &[UserId],
    ) -> DomainResult<Vec<(UserId, PresenceStatus)>>;
}
```

**`NotificationPort`**:

```rust
use tokio::sync::broadcast;

#[async_trait::async_trait]
pub trait NotificationPort: Send + Sync {
    async fn connect(&self, token: &SkypeToken) -> DomainResult<broadcast::Receiver<DomainNotification>>;
    async fn disconnect(&self) -> DomainResult<()>;
}

#[derive(Debug, Clone)]
pub enum DomainNotification {
    NewMessage { conversation_id: ConversationId, message: Message },
    MessageUpdated { message_id: MessageId, content: MessageContent },
    PresenceChanged { user_id: UserId, status: PresenceStatus },
    TypingIndicator { user_id: UserId, conversation_id: ConversationId },
}
```

Note: `NotificationPort` uses `tokio::sync::broadcast` — add `tokio = { workspace = true, features = ["sync"] }` to `yakko-domain` if needed (or use just the `tokio-stream` subset). Keep it minimal.

### Module files

Update `crates/yakko-domain/src/lib.rs` to re-export all public types.

Create `crates/yakko-domain/src/entities/mod.rs`, `crates/yakko-domain/src/value_objects/mod.rs`, `crates/yakko-domain/src/ports/mod.rs`.

## Tests to Add

In `crates/yakko-domain/src/value_objects/ids.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_id_roundtrip() {
        let id = TeamId::new("abc-123");
        assert_eq!(id.as_str(), "abc-123");
        assert_eq!(id.to_string(), "abc-123");
        let parsed: TeamId = "abc-123".parse().unwrap();
        assert_eq!(id, parsed);
    }
}
```

Write similar tests for all ID types.

## Preflight

```bash
cargo test -p yakko-domain && cargo clippy -p yakko-domain -- -D warnings
```

## Exit Criteria

- `cargo test -p yakko-domain` passes
- `yakko-domain` has zero dependencies on `tokio` (except optionally `tokio::sync`)
- All six port traits are defined with doc comments
- All value objects zeroize-on-drop verified by test or comment

## After Completion

Update PROGRESS.md row for T03 to `[x]`.  
Commit: `feat(domain): add entities, value objects, and port trait definitions`
