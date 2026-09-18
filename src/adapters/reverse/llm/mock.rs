//! In-memory mock LLM client for tests.
//!
//! Returns a canned [`LlmResponse`] for every call. Tests can inspect
//! the recorded prompts via [`MockLlmClient::calls`] to assert that the
//! context was assembled correctly.

use std::sync::Mutex;

use super::client::{LlmClient, LlmProvider, LlmResponse};

/// Pre-canned response to return from a [`MockLlmClient`].
#[derive(Debug, Clone)]
pub struct CannedResponse {
    pub text: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub model: String,
}

impl Default for CannedResponse {
    fn default() -> Self {
        Self {
            text: r#"{"phases":[]}"#.to_string(),
            input_tokens: 100,
            output_tokens: 10,
            model: "mock".to_string(),
        }
    }
}

/// One captured call. Useful in tests to assert on the prompts.
#[derive(Debug, Clone)]
pub struct RecordedCall {
    pub provider: LlmProvider,
    pub model: String,
    pub system: String,
    pub user: String,
}

/// Mock LLM client. Thread-safe via internal mutex.
pub struct MockLlmClient {
    response: CannedResponse,
    calls: Mutex<Vec<RecordedCall>>,
}

impl MockLlmClient {
    /// Create a mock returning the given canned response on every call.
    #[must_use]
    pub const fn new(response: CannedResponse) -> Self {
        Self {
            response,
            calls: Mutex::new(Vec::new()),
        }
    }

    /// Create a mock returning a single phase+task plan.
    #[must_use]
    pub fn with_plan(plan_json: &str) -> Self {
        Self::new(CannedResponse {
            text: plan_json.to_string(),
            ..Default::default()
        })
    }

    /// Snapshot of all calls made so far. Poisoning the mutex in a test is a
    /// programmer error, so we unwrap via `unwrap_or_else(PoisonError::into_inner)`
    /// — keeps the lock usable even after a panic in another test thread.
    #[must_use]
    pub fn calls(&self) -> Vec<RecordedCall> {
        match self.calls.lock() {
            Ok(g) => g.clone(),
            Err(p) => p.into_inner().clone(),
        }
    }
}

impl Default for MockLlmClient {
    fn default() -> Self {
        Self::new(CannedResponse::default())
    }
}

#[async_trait::async_trait]
impl LlmClient for MockLlmClient {
    async fn complete(
        &self,
        provider: LlmProvider,
        model: &str,
        system: &str,
        user: &str,
    ) -> crate::error::Result<LlmResponse> {
        let call = RecordedCall {
            provider,
            model: model.to_string(),
            system: system.to_string(),
            user: user.to_string(),
        };
        match self.calls.lock() {
            Ok(mut g) => g.push(call),
            Err(p) => p.into_inner().push(call),
        }
        Ok(LlmResponse {
            text: self.response.text.clone(),
            input_tokens: self.response.input_tokens,
            output_tokens: self.response.output_tokens,
            model: self.response.model.clone(),
        })
    }
}
