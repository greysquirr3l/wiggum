# T04 — Token Store Adapter

> **Depends on**: T03 complete.

## Goal

Implement `KeyringAdapter` in `yakko-infra` that satisfies `TokenStorePort`.

## Project Context

- `TokenStorePort` is defined in `yakko-domain::ports`
- The `keyring` crate (v3) wraps the OS native credential store (macOS Keychain, Linux Secret Service, Windows Credential Manager)
- Service name: `"yakko"`, username: the user's UPN (email)
- The `SkypeToken` type is zeroized on drop — serialize to string for storage, zeroize immediately after reading

## Implementation

### `crates/yakko-infra/src/token_store/keyring.rs`

```rust
use keyring::Entry;
use async_trait::async_trait;
use yakko_domain::{
    ports::TokenStorePort,
    value_objects::tokens::SkypeToken,
    error::{DomainError, DomainResult},
};
use chrono::{DateTime, Utc};

pub struct KeyringAdapter;

impl KeyringAdapter {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl TokenStorePort for KeyringAdapter {
    async fn save_token(&self, upn: &str, token: &SkypeToken) -> DomainResult<()> {
        // Serialize: "TOKEN|EXPIRES_RFC3339"
        let value = format!("{}|{}", token.as_str(), token.expires_at().to_rfc3339());
        let entry = Entry::new("yakko", upn)
            .map_err(|e| DomainError::TokenStore(e.to_string()))?;
        entry.set_password(&value)
            .map_err(|e| DomainError::TokenStore(e.to_string()))?;
        Ok(())
    }

    async fn load_token(&self, upn: &str) -> DomainResult<Option<SkypeToken>> {
        let entry = Entry::new("yakko", upn)
            .map_err(|e| DomainError::TokenStore(e.to_string()))?;
        match entry.get_password() {
            Ok(value) => {
                let (token_str, expires_str) = value
                    .split_once('|')
                    .ok_or_else(|| DomainError::TokenStore("malformed stored token".into()))?;
                let expires_at = expires_str.parse::<DateTime<Utc>>()
                    .map_err(|e| DomainError::TokenStore(e.to_string()))?;
                Ok(Some(SkypeToken::new(token_str.to_string(), expires_at)))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(DomainError::TokenStore(e.to_string())),
        }
    }

    async fn clear_token(&self, upn: &str) -> DomainResult<()> {
        let entry = Entry::new("yakko", upn)
            .map_err(|e| DomainError::TokenStore(e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // already absent — not an error
            Err(e) => Err(DomainError::TokenStore(e.to_string())),
        }
    }
}
```

### `crates/yakko-infra/src/token_store/mod.rs`

```rust
pub mod keyring;
pub use keyring::KeyringAdapter;
```

### Update `crates/yakko-infra/src/lib.rs`

Ensure `pub mod token_store;` is present.

## Tests

Add an integration test gated by `#[cfg(feature = "integration")]` or just a `#[ignore]` attribute:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    #[ignore = "requires OS keychain — run manually with: cargo test -p yakko-infra -- --ignored"]
    async fn token_roundtrip() {
        let adapter = KeyringAdapter::new();
        let upn = "test@example.com";
        let token = SkypeToken::new(
            "fake-token-value".to_string(),
            Utc::now() + chrono::Duration::hours(24),
        );

        adapter.save_token(upn, &token).await.unwrap();
        let loaded = adapter.load_token(upn).await.unwrap().expect("token should exist");
        assert_eq!(loaded.as_str(), "fake-token-value");

        adapter.clear_token(upn).await.unwrap();
        let cleared = adapter.load_token(upn).await.unwrap();
        assert!(cleared.is_none());
    }
}
```

## Preflight

```bash
cargo build -p yakko-infra && cargo clippy -p yakko-infra -- -D warnings
```

## Exit Criteria

- `KeyringAdapter` compiles and implements `TokenStorePort`
- Integration test exists (marked `#[ignore]`)
- `cargo clippy` clean

## After Completion

Update PROGRESS.md row for T04 to `[x]`.  
Commit: `feat(infra): implement KeyringAdapter for OS keychain token storage`
