//! LLM-driven phase decomposition for `wiggum reverse`.
//!
//! When the user passes `--llm anthropic` (or `--llm minimax`), the reverse
//! command sends the detected project context (manifest, README, top-level
//! tree, hints) to the LLM and asks it to emit a structured `plan.toml` shape
//! in JSON. We then translate that JSON back into a [`Plan`] and replace the
//! placeholder phases the deterministic scan produced.
//!
//! Two providers are supported today:
//! - [`anthropic`] — Anthropic Claude (api.anthropic.com)
//! - [`minimax`]  — `MiniMax` (api.minimax.io/anthropic, Anthropic-shaped)
//!
//! Both speak the same JSON wire format (Anthropic Messages API), so the
//! request/response machinery is shared in [`client::AnthropicMessagesClient`].
//!
//! Tests use [`mock::MockLlmClient`] so no API key is required.

#[allow(dead_code)] // public API used by tests + future binary wiring
pub mod client;
pub mod context;
#[allow(dead_code)] // test infrastructure — referenced by integration tests
pub mod mock;
#[allow(dead_code)] // prompt is referenced via re-exports + tests
pub mod plan_json;
#[allow(dead_code)] // prompt rendering only fires when --llm is set
pub mod prompt;

use std::sync::Arc;

use crate::adapters::bootstrap::ScanResult;
use crate::adapters::reverse::hints::Hints;
use crate::domain::plan::Plan;
use crate::error::Result;

use self::client::{AnthropicMessagesClient, LlmClient, LlmProvider, LlmResponse};

/// One provider option selected by the user via `--llm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Anthropic,
    Minimax,
}

impl Provider {
    /// Parse from the CLI string (`"anthropic"` or `"minimax"`).
    #[must_use]
    pub fn from_cli(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "anthropic" | "claude" => Some(Self::Anthropic),
            "minimax" => Some(Self::Minimax),
            _ => None,
        }
    }

    /// Default model for this provider.
    #[must_use]
    pub const fn default_model(self) -> &'static str {
        match self {
            Self::Anthropic => "claude-sonnet-4-5",
            Self::Minimax => "MiniMax-M3",
        }
    }

    /// Environment variable to read the API key from by default.
    #[must_use]
    pub const fn api_key_env(self) -> &'static str {
        match self {
            Self::Anthropic => "ANTHROPIC_API_KEY",
            Self::Minimax => "MINIMAX_API_KEY",
        }
    }

    /// Resolve the API key from the env var or the explicit override.
    ///
    /// # Errors
    ///
    /// Returns `WiggumError::Validation` if neither source is set.
    pub fn resolve_api_key(self, override_key: Option<&str>) -> Result<String> {
        if let Some(key) = override_key {
            if key.trim().is_empty() {
                return Err(crate::error::WiggumError::Validation(format!(
                    "--api-key was provided but empty (provider={self:?})"
                )));
            }
            return Ok(key.to_string());
        }
        let var = self.api_key_env();
        match std::env::var(var) {
            Ok(key) if !key.trim().is_empty() => Ok(key),
            _ => Err(crate::error::WiggumError::Validation(format!(
                "no API key for LLM provider {self:?}: set `{var}` or pass --api-key"
            ))),
        }
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Anthropic => write!(f, "anthropic"),
            Self::Minimax => write!(f, "minimax"),
        }
    }
}

/// Build the concrete [`LlmClient`] for the given provider + key.
#[must_use]
pub fn build_client(provider: Provider, api_key: String) -> Arc<dyn LlmClient> {
    let endpoint = match provider {
        Provider::Anthropic => AnthropicMessagesClient::anthropic_endpoint(),
        Provider::Minimax => AnthropicMessagesClient::minimax_endpoint(),
    };
    Arc::new(AnthropicMessagesClient::new(api_key, endpoint))
}

/// Run the LLM-driven phase decomposition and merge the result into `plan`.
///
/// `repo_path` is the local working tree (already cloned) used to read the
/// README and directory tree for the prompt context.
///
/// # Errors
///
/// - API key missing/invalid
/// - HTTP / serialization errors from the LLM client
/// - LLM returned a JSON plan we can't parse or that fails validation
pub async fn decompose_into_plan(
    client: Arc<dyn LlmClient>,
    repo_path: &std::path::Path,
    scan: &ScanResult,
    hints: Option<&Hints>,
    plan: &mut Plan,
) -> Result<LlmResponse> {
    let ctx = context::gather_context(repo_path, scan, hints)?;
    let user_msg = prompt::render_user_prompt(&ctx);
    let system_msg = prompt::render_system_prompt();

    let model = std::env::var("WIGGUM_LLM_MODEL").unwrap_or_else(|_| "default".to_string());
    let response = client
        .complete(
            LlmProvider::AnthropicMessages,
            &model,
            &system_msg,
            &user_msg,
        )
        .await?;

    let parsed = plan_json::parse_llm_plan(&response.text)?;
    plan_json::apply_llm_plan(plan, &parsed);

    Ok(response)
}
