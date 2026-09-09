//! LLM HTTP client abstraction + Anthropic-Messages wire implementation.
//!
//! Both supported providers (Anthropic Claude + `MiniMax`) speak the same
//! Anthropic-Messages JSON format, so the same client struct drives both —
//! only the base URL changes. New providers that also follow this shape
//! should add an endpoint constant + CLI variant; no new client struct needed.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Result, WiggumError};

/// Provider enum used by the trait so different wire shapes can share it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProvider {
    AnthropicMessages,
}

/// Raw response from an LLM call.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// The text the model emitted (concatenated across all content blocks).
    pub text: String,
    /// Tokens billed on the input side.
    pub input_tokens: u32,
    /// Tokens billed on the output side.
    pub output_tokens: u32,
    /// Which model produced the response (echoed back from the API).
    pub model: String,
}

/// Trait every LLM client implements.
///
/// `Send + Sync` so the client can live behind an `Arc` and be passed across
/// the async boundary.
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    /// Send a single-turn system + user prompt and return the model's reply.
    async fn complete(
        &self,
        provider: LlmProvider,
        model: &str,
        system: &str,
        user: &str,
    ) -> Result<LlmResponse>;
}

// ── Anthropic-Messages wire implementation ────────────────────────────────

/// Anthropic-Messages-shaped client. Works for both Anthropic Claude and
/// `MiniMax` (same wire format, different base URL).
pub struct AnthropicMessagesClient {
    api_key: String,
    endpoint: String,
    http: reqwest::Client,
}

impl AnthropicMessagesClient {
    /// Endpoint URL for the standard Anthropic API (`api.anthropic.com`).
    pub fn anthropic_endpoint() -> String {
        String::from("https://api.anthropic.com/v1/messages")
    }

    /// Endpoint URL for the `MiniMax` Anthropic-compatible API.
    pub fn minimax_endpoint() -> String {
        String::from("https://api.minimax.io/anthropic/v1/messages")
    }

    /// Construct a client with an explicit endpoint.
    pub fn new(api_key: String, endpoint: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_else(|e| {
                // Unreachable in practice — reqwest only fails on builder misuse,
                // not on timeout values. Fall back to a default client.
                tracing::error!("reqwest client builder failed: {e}");
                reqwest::Client::new()
            });
        Self {
            api_key,
            endpoint,
            http,
        }
    }
}

#[async_trait::async_trait]
impl LlmClient for AnthropicMessagesClient {
    async fn complete(
        &self,
        provider: LlmProvider,
        model: &str,
        system: &str,
        user: &str,
    ) -> Result<LlmResponse> {
        match provider {
            LlmProvider::AnthropicMessages => {
                self.complete_anthropic_messages(model, system, user).await
            }
        }
    }
}

impl AnthropicMessagesClient {
    async fn complete_anthropic_messages(
        &self,
        model: &str,
        system: &str,
        user: &str,
    ) -> Result<LlmResponse> {
        let req = MessagesRequest {
            model,
            max_tokens: 4096,
            system,
            messages: vec![Message {
                role: "user",
                content: user,
            }],
        };

        let response = self
            .http
            .post(&self.endpoint)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&req)
            .send()
            .await
            .map_err(|e| WiggumError::Validation(format!("LLM HTTP request failed: {e}")))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| WiggumError::Validation(format!("read LLM response body: {e}")))?;

        if !status.is_success() {
            return Err(WiggumError::Validation(format!(
                "LLM API returned HTTP {status}: {body}"
            )));
        }

        let parsed: MessagesResponse = serde_json::from_str(&body).map_err(|e| {
            WiggumError::Validation(format!("LLM response JSON parse failed: {e}; body={body}"))
        })?;

        let text = parsed
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                ContentBlock::Other => None,
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(LlmResponse {
            text,
            input_tokens: parsed.usage.input_tokens,
            output_tokens: parsed.usage.output_tokens,
            model: parsed.model,
        })
    }
}

// ── Wire types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<Message<'a>>,
}

#[derive(Debug, Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct MessagesResponse {
    model: String,
    content: Vec<ContentBlock>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}
