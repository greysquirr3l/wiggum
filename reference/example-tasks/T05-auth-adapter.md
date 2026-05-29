# T05 — Authentication Adapter

> **Depends on**: T04 complete.

## Goal

Implement `AadAuthAdapter` in `yakko-infra::auth` with PKCE browser-based login.  
The adapter completes the full chain:  
**AAD OAuth2 PKCE → Teams authsvc → SkypeToken**

## Project Context

- Teams uses an *internal* API at `api.spaces.skype.com` authenticated with a `skypetoken`
- A `skypetoken` is NOT an OAuth token — it's obtained by exchanging an AAD access token at a Teams-specific endpoint
- Microsoft never officially published this flow; reference: `fossteams/teams-api` (Go, 2021)
- The AAD client ID used by the Teams web app is: `5e3ce6c0-2b1f-4285-8d4b-75ee78787346`  
  *(Verify this is still correct via DevTools during the Phase 0 spike)*
- The AAD auth endpoint: `https://login.microsoftonline.com/common/oauth2/v2.0/authorize`
- AAD token endpoint: `https://login.microsoftonline.com/common/oauth2/v2.0/token`
- Teams authsvc: `https://teams.microsoft.com/api/authsvc/v1.0/authz`
- OAuth scope for Skype audience: `https://api.spaces.skype.com/.default offline_access`
- Full architecture in `ARCHITECTURE.md § 6`

## Implementation

### `crates/yakko-infra/src/auth/pkce.rs`

PKCE helpers:

```rust
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Sha256, Digest};

pub struct PkceChallenge {
    pub code_verifier: String,
    pub code_challenge: String,
}

impl PkceChallenge {
    pub fn new() -> Self {
        // 32 random bytes → 43-char base64url verifier
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).expect("getrandom failed");
        let code_verifier = URL_SAFE_NO_PAD.encode(bytes);

        let digest = Sha256::digest(code_verifier.as_bytes());
        let code_challenge = URL_SAFE_NO_PAD.encode(digest);

        Self { code_verifier, code_challenge }
    }
}
```

Add to `yakko-infra/Cargo.toml`: `sha2 = "0.10"`, `base64 = "0.22"`, `getrandom = "0.2"`.

### `crates/yakko-infra/src/auth/callback_server.rs`

Local HTTP server to catch the OAuth redirect:

```rust
use tokio::net::TcpListener;
use std::net::SocketAddr;

pub struct CallbackServer {
    pub addr: SocketAddr,
    listener: TcpListener,
}

impl CallbackServer {
    pub async fn bind() -> anyhow::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        Ok(Self { addr, listener })
    }

    /// Waits for one GET request, extracts `code` + `state` query params.
    pub async fn wait_for_code(self) -> anyhow::Result<(String, String)> {
        let (stream, _) = self.listener.accept().await?;
        // parse HTTP request line, extract query string, return (code, state)
        // Send a brief HTML response so the browser shows success
        // Full implementation: parse minimally, don't use a full HTTP library
        todo!("implement minimal HTTP request parser to extract code and state")
    }
}
```

### `crates/yakko-infra/src/auth/aad.rs`

Main adapter:

```rust
use async_trait::async_trait;
use reqwest::Client;
use yakko_domain::{
    ports::AuthPort,
    value_objects::tokens::{AadToken, SkypeToken},
    error::{DomainError, DomainResult},
};
use super::{pkce::PkceChallenge, callback_server::CallbackServer};
use chrono::{Utc, Duration};

const CLIENT_ID: &str = "5e3ce6c0-2b1f-4285-8d4b-75ee78787346";
const AAD_AUTH_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
const AAD_TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const AUTHSVC_URL: &str = "https://teams.microsoft.com/api/authsvc/v1.0/authz";
const SCOPE: &str = "https://api.spaces.skype.com/.default offline_access";

pub struct AadAuthAdapter {
    http: Client,
}

impl AadAuthAdapter {
    pub fn new(http: Client) -> Self { Self { http } }
}

#[async_trait]
impl AuthPort for AadAuthAdapter {
    async fn login(&self) -> DomainResult<SkypeToken> {
        let pkce = PkceChallenge::new();
        let state = uuid::Uuid::new_v4().to_string();
        let server = CallbackServer::bind().await
            .map_err(|e| DomainError::Auth(e.to_string()))?;
        let redirect_uri = format!("http://127.0.0.1:{}/callback", server.addr.port());

        let auth_url = build_auth_url(&pkce, &state, &redirect_uri);
        open::that(&auth_url).map_err(|e| DomainError::Auth(e.to_string()))?;

        let (code, _returned_state) = server.wait_for_code().await
            .map_err(|e| DomainError::Auth(e.to_string()))?;

        let aad_token = exchange_code_for_token(
            &self.http, &code, &pkce.code_verifier, &redirect_uri,
        ).await?;

        exchange_aad_for_skypetoken(&self.http, &aad_token).await
    }

    async fn refresh(&self, _token: &SkypeToken) -> DomainResult<SkypeToken> {
        // SkypeToken refresh via AAD refresh_token
        // Store refresh_token alongside SkypeToken for v1.1
        Err(DomainError::TokenExpired)
    }
}

fn build_auth_url(pkce: &PkceChallenge, state: &str, redirect_uri: &str) -> String {
    format!(
        "{auth}?client_id={client_id}&response_type=code&redirect_uri={redirect}&scope={scope}&state={state}&code_challenge={challenge}&code_challenge_method=S256",
        auth = AAD_AUTH_URL,
        client_id = CLIENT_ID,
        redirect = urlencoding::encode(redirect_uri),
        scope = urlencoding::encode(SCOPE),
        state = state,
        challenge = pkce.code_challenge,
    )
}

async fn exchange_code_for_token(
    http: &Client, code: &str, verifier: &str, redirect_uri: &str,
) -> DomainResult<AadToken> {
    // POST to AAD_TOKEN_URL with form data
    // Return AadToken from access_token field
    todo!()
}

async fn exchange_aad_for_skypetoken(
    http: &Client, aad_token: &AadToken,
) -> DomainResult<SkypeToken> {
    // POST to AUTHSVC_URL with Bearer aad_token
    // Parse skypeToken + expiresIn from response  
    // expiresIn is in seconds; add to Utc::now()
    todo!()
}
```

Add `urlencoding = "2"` to `yakko-infra/Cargo.toml`.

### `crates/yakko-infra/src/auth/mod.rs`

```rust
pub mod aad;
mod callback_server;
mod pkce;
pub use aad::AadAuthAdapter;
```

## Security Notes

- Do not log the code_verifier, access_token, or skypetoken at any log level
- The `AadToken` type is zeroized on drop; do not destructure it into a plain String outside this module
- The callback server binds to 127.0.0.1 only (loopback) — never 0.0.0.0

## Preflight

```bash
cargo build -p yakko-infra && cargo clippy -p yakko-infra -- -D warnings
```

## Exit Criteria

- Compiles without warnings (TODOs in async functions are acceptable for this task)
- `PkceChallenge::new()` produces valid challenge/verifier pair (add a unit test)
- `CallbackServer::bind()` binds to an ephemeral port (unit test: bind, get addr, verify port > 0)

## After Completion

Update PROGRESS.md row for T05 to `[x]`.  
Commit: `feat(infra): implement PKCE auth adapter with AAD→skypetoken exchange`
