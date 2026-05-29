# T06 — Teams HTTP Client

> **Depends on**: T05 complete.

## Goal

Implement `TeamsClient` with all wire models and implement `ConversationPort`, `MessagePort`, and `PresencePort` for it.

## Project Context

- Teams internal API base: `https://teams.microsoft.com/api/csa/api/v1`
- Auth: inject `Authorization: skypetoken {token}` AND `X-Skypetoken: {token}` on every request
- Wire models live in `yakko-infra` only — never exposed to the domain crate
- Mapping from wire → domain happens in `teams_api/mapping.rs`
- On 401: attempt one token refresh via `AuthPort`, retry, then return `DomainError::TokenExpired`
- Refer to `docs/api-notes.md` for confirmed endpoint URLs (created during Phase 0 spike)
- Full architecture in `ARCHITECTURE.md § 7`

## Implementation

### `crates/yakko-infra/src/teams_api/client.rs`

```rust
use reqwest::{Client, RequestBuilder};
use yakko_domain::value_objects::tokens::SkypeToken;

pub struct TeamsClient {
    http: Client,
    base_url: String,
}

impl TeamsClient {
    pub fn new() -> Self {
        let http = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Teams/24.24.0 Chrome/120.0.6099.291 Electron/28.2.10 Safari/537.36")
            .build()
            .expect("failed to build reqwest client");
        Self {
            http,
            base_url: "https://teams.microsoft.com/api/csa/api/v1".to_string(),
        }
    }

    /// For tests — override base URL
    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }

    pub(crate) fn authed_get(&self, path: &str, token: &SkypeToken) -> RequestBuilder {
        self.http
            .get(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("skypetoken {}", token.as_str()))
            .header("X-Skypetoken", token.as_str())
            .header("Accept", "application/json")
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("X-Ms-Client-Version", "24/24.24.0")
    }

    pub(crate) fn authed_post(&self, path: &str, token: &SkypeToken) -> RequestBuilder {
        self.authed_get(path, token)
            .method(reqwest::Method::POST)
    }
}
```

### `crates/yakko-infra/src/teams_api/models.rs`

Wire-format serde structs. Annotate all fields with `#[serde(rename_all = "camelCase")]` as needed.  
Use `#[serde(default)]` liberally — the Teams API omits many optional fields.

Create structs to deserialize:

- `WireUser` — from `GET /teams/users/me`
- `WireConversationList` — from `GET /users/ME/conversations` (array wrapper)
- `WireConversation` — individual item including `id`, `displayName`, `type`, `memberCount`
- `WireMessagePage` — from `GET /users/ME/conversations/{id}/messages`
- `WireMessage` — `id`, `type`, `content`, `from`, `originalarrivaltime`
- Check `docs/api-notes.md` for the exact field names from your Phase 0 spike

### `crates/yakko-infra/src/teams_api/mapping.rs`

Conversion functions (not `From` impls — keep those out of the domain crate):

```rust
use super::models::*;
use yakko_domain::entities::*;
use yakko_domain::value_objects::ids::*;

pub fn wire_to_conversation(wire: WireConversation, my_user_id: &UserId) -> Conversation {
    todo!()
}

pub fn wire_to_message(wire: WireMessage, conversation_id: ConversationId, my_user_id: &UserId) -> Message {
    todo!()
}

pub fn wire_to_team(wire: WireTeam) -> Team {
    todo!()
}
```

### Port implementations

**`crates/yakko-infra/src/teams_api/ports/conversation.rs`**:

```rust
use async_trait::async_trait;
use yakko_domain::{
    ports::ConversationPort,
    entities::{conversation::Conversation, team::Team},
    value_objects::tokens::SkypeToken,
    error::{DomainError, DomainResult},
};
use crate::teams_api::{client::TeamsClient, mapping};

#[async_trait]
impl ConversationPort for TeamsClient {
    async fn list_conversations(&self, token: &SkypeToken) -> DomainResult<Vec<Conversation>> {
        let resp = self
            .authed_get("/users/ME/conversations", token)
            .send()
            .await
            .map_err(|e| DomainError::Network(e.to_string()))?;

        if resp.status() == 401 {
            return Err(DomainError::TokenExpired);
        }
        if !resp.status().is_success() {
            return Err(DomainError::Api {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let wire: WireConversationList = resp.json().await
            .map_err(|e| DomainError::Network(e.to_string()))?;
        // TODO: get my_user_id from a cached profile call
        let my_id = UserId::new("me");
        Ok(wire.conversations.into_iter()
            .map(|c| mapping::wire_to_conversation(c, &my_id))
            .collect())
    }

    async fn list_teams(&self, token: &SkypeToken) -> DomainResult<Vec<Team>> {
        // GET /teams/users/me returns the joined teams list
        todo!()
    }
}
```

Implement `MessagePort` and `PresencePort` similarly.

### Integration example

Create `crates/yakko-infra/examples/list_conversations.rs`:

```rust
/// Run with:
///   YAKKO_TOKEN=xxxx cargo run --example list_conversations -p yakko-infra
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let token_str = std::env::var("YAKKO_TOKEN")?;
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
    let token = yakko_domain::value_objects::tokens::SkypeToken::new(token_str, expires_at);

    let client = yakko_infra::teams_api::client::TeamsClient::new();
    let convos = yakko_domain::ports::ConversationPort::list_conversations(&client, &token).await?;
    for c in &convos {
        println!("{}: {}", c.id, c.display_name);
    }
    println!("Total: {}", convos.len());
    Ok(())
}
```

## Preflight

```bash
cargo build -p yakko-infra && cargo clippy -p yakko-infra -- -D warnings
```

## Exit Criteria

- `TeamsClient` compiles with all three port trait implementations
- Integration example compiles
- `cargo clippy` clean
- No wire model types leak into the `yakko-domain` crate

## After Completion

Update PROGRESS.md row for T06 to `[x]`.  
Commit: `feat(infra): implement TeamsClient with ConversationPort, MessagePort, PresencePort`
