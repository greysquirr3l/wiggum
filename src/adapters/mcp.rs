//! The wiggum MCP server.
//!
//! The protocol itself is protocol-droid's: it speaks every published spec
//! revision — 2024-11-05 through 2026-07-28 — with a per-revision codec
//! behind each one, so wiggum does not carry a second description of the
//! wire format and cannot fall out of date with the spec.
//!
//! What lives here is the part that is wiggum's: the tools, the guardrails
//! applied to them, and the wiring that puts the two together. The tools
//! live in a private `tools` module and the guardrails in a private
//! `guardrail` module — neither is part of wiggum's public surface, because
//! the public surface of an MCP server is the protocol it speaks, not the
//! functions behind it.
//!
//! # How the guardrails are applied
//!
//! Each tool is registered with a handler that wraps the generated one:
//!
//! ```text
//! peer ──▶ guardrail wrapper ──▶ generated handler ──▶ tool body
//! ```
//!
//! The wrapper is where the three guardrails live, and it is one place for
//! all of them rather than three places in each of twelve tool bodies:
//!
//! - the **input guardrail** reads the peer's arguments before the tool sees
//!   them, and blocks a mutating call whose arguments carry a
//!   prompt-injection pattern;
//! - the **session guardrail** counts calls and reports a read-then-write
//!   anomaly;
//! - the **output guardrail** redacts the softer secret patterns in what the
//!   tool returned and hard-blocks the hard ones.
//!
//! The input guardrail needs the *raw* arguments, and the generated handler
//! is where the raw arguments are — protocol-droid decodes them into the
//! tool's argument struct inside the handler, so a wrapper is the last
//! place the undecoded form is still available.

pub(crate) mod guardrail;
pub(crate) mod tools;

use std::sync::Arc;

use protocol_droid::application::server::{BoxFuture, RequestContext, Server, ToolHandler};
use tracing::warn;

use crate::error::{Result, WiggumError};

use guardrail::{
    contains_blocked_output, evaluate_session_guardrail, is_mutating_tool, redact_output_content,
    security_event, strict_guardrail_mode_enabled,
};

/// Build the wiggum MCP server.
///
/// The server is assembled in [`tools`], next to the tools themselves, because
/// `#[protocol_droid::server]` expands into module-scope items that a sibling
/// module cannot name.
fn build_server() -> std::result::Result<Server, protocol_droid::domain::ir::JsonObjectError> {
    tools::build_server()
}

/// Wrap a tool handler with the three guardrails.
///
/// The wrapper is the whole of wiggum's security surface over MCP, and it
/// sits where the hand-rolled server's `handle_tool_call` sat: after the
/// peer has named a tool and supplied its arguments, before the tool's own
/// body runs.
pub(crate) fn guarded(tool_name: &'static str, inner: ToolHandler) -> ToolHandler {
    Arc::new(move |ctx: RequestContext, arguments| {
        let inner = Arc::clone(&inner);
        let fut: BoxFuture = Box::pin(async move {
            if let Some(refusal) = input_guardrail(tool_name, &arguments) {
                return refusal;
            }

            if let Some(refusal) = session_guardrail(tool_name) {
                return refusal;
            }

            let result = inner(ctx, arguments).await;
            output_guardrail(tool_name, result)
        });
        fut
    })
}

/// The input guardrail: refuse a mutating call whose arguments carry a
/// prompt-injection pattern.
///
/// The check is on the raw arguments rather than on the decoded ones,
/// because the raw form is what the peer actually sent — and a pattern
/// split across two arguments is a pattern in the raw form and two innocent
/// strings in the decoded one.
fn input_guardrail(
    tool_name: &str,
    arguments: &protocol_droid::domain::ir::JsonObject,
) -> Option<protocol_droid::domain::ir::JsonObject> {
    if !is_mutating_tool(tool_name) {
        return None;
    }

    let decoded = serde_json::from_str::<serde_json::Value>(arguments.as_str()).ok()?;
    let pattern = guardrail::detect_input_guardrail_hit(&decoded)?;

    security_event(
        tool_name,
        "input_guardrail_blocked",
        &format!("matched pattern: {pattern}"),
    );

    Some(tool_error(format!(
        "Error: blocked by MCP input guardrail (suspicious instruction pattern: {pattern})"
    )))
}

/// The session guardrail: report a read-then-write anomaly.
///
/// A peer that reads a dozen files and then writes is doing something a
/// peer that reads and writes is not, and the count is the evidence. In
/// strict mode the anomaly is refused; otherwise it is logged and the call
/// proceeds, because a threshold that fires on ordinary use trains people
/// to ignore it.
fn session_guardrail(tool_name: &str) -> Option<protocol_droid::domain::ir::JsonObject> {
    let detail = evaluate_session_guardrail(tool_name)?;
    let strict_mode = strict_guardrail_mode_enabled();
    let event = if strict_mode {
        "session_guardrail_blocked"
    } else {
        "session_guardrail_alert"
    };
    security_event(tool_name, event, &detail);

    if strict_mode {
        Some(tool_error(format!(
            "Error: blocked by MCP session guardrail (detail: {detail})"
        )))
    } else {
        None
    }
}

/// The output guardrail: redact the soft patterns, block the hard ones.
///
/// Redaction runs first, so that a value which is *both* an email and part
/// of a larger secret has the softer treatment. The blocked check runs on
/// the redacted text, because the point of the check is what would reach
/// the peer.
fn output_guardrail(
    tool_name: &str,
    result: protocol_droid::domain::ir::JsonObject,
) -> protocol_droid::domain::ir::JsonObject {
    let Some(text) = result_text(&result) else {
        return result;
    };

    let (redacted, count) = redact_output_content(&text);
    if count > 0 {
        security_event(tool_name, "output_redacted", &format!("redactions={count}"));
    }

    if let Some(pattern) = contains_blocked_output(&redacted) {
        security_event(
            tool_name,
            "output_guardrail_blocked",
            &format!("matched pattern: {pattern}"),
        );
        return tool_error(format!(
            "Error: blocked by MCP output guardrail (sensitive marker: {pattern})"
        ));
    }

    if redacted == text {
        return result;
    }

    replace_result_text(result, &redacted)
}

/// The text of a tool result, if it has one.
///
/// A result is `{"content": [{"type": "text", "text": "..."}], "isError":
/// bool}`, and the text is the only part a secret can be in — the
/// structured content of a wiggum tool is a rendering of the same string.
fn result_text(result: &protocol_droid::domain::ir::JsonObject) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(result.as_str()).ok()?;
    let content = value.get("content")?.as_array()?;
    let first = content.first()?;
    Some(first.get("text")?.as_str()?.to_string())
}

/// Put a new text into a tool result, leaving the rest of it alone.
fn replace_result_text(
    result: protocol_droid::domain::ir::JsonObject,
    text: &str,
) -> protocol_droid::domain::ir::JsonObject {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(result.as_str()) else {
        return result;
    };
    if let Some(content) = value.get_mut("content").and_then(|c| c.as_array_mut())
        && let Some(first) = content.first_mut()
        && let Some(slot) = first.get_mut("text")
    {
        *slot = serde_json::Value::String(text.to_string());
    }
    protocol_droid::domain::ir::JsonObject::from_value(value).unwrap_or(result)
}

/// A tool result carrying an error, in the shape protocol-droid uses.
fn tool_error(message: String) -> protocol_droid::domain::ir::JsonObject {
    protocol_droid::application::server::result::failure(message)
}

/// Serve the MCP server over stdio until the peer closes the stream.
///
/// This is the whole of `wiggum serve --mcp`: the transport, the JSON-RPC
/// framing, the version negotiation and the wire shapes are protocol-droid's.
///
/// # Errors
///
/// When the server cannot be built, when the Tokio runtime cannot start, or
/// when the stdio transport fails while serving.
pub fn run_mcp_server() -> Result<()> {
    let server = build_server()
        .map_err(|e| WiggumError::Validation(format!("failed to build MCP server: {e}")))?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| WiggumError::Validation(format!("failed to start MCP runtime: {e}")))?;

    runtime.block_on(async move {
        let report = server
            .serve_stdio()
            .await
            .map_err(|e| WiggumError::Validation(format!("MCP server failed: {e}")))?;
        warn!(
            target: "wiggum::mcp",
            frames = report.frames,
            requests = report.requests,
            replies = report.replies,
            "MCP server session ended"
        );
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The error a test in this module can fail with.
    ///
    /// The schema builders and the server builder both fail with a
    /// `JsonObjectError`, so a test that uses them can carry that error out
    /// of its body with `?` rather than unwrapping it.
    type TestResult = std::result::Result<(), protocol_droid::domain::ir::JsonObjectError>;

    #[test]
    fn mutating_tools_are_the_ones_that_write() {
        assert!(is_mutating_tool("wiggum_generate_plan"));
        assert!(is_mutating_tool("wiggum_update_progress"));
        assert!(is_mutating_tool("wiggum_bootstrap"));
        assert!(is_mutating_tool("wiggum_generate_agents_md"));
    }

    #[test]
    fn read_tools_are_not_mutating() {
        assert!(!is_mutating_tool("wiggum_validate_plan"));
        assert!(!is_mutating_tool("wiggum_lint_plan"));
        assert!(!is_mutating_tool("wiggum_read_progress"));
        assert!(!is_mutating_tool("wiggum_version"));
    }

    #[test]
    fn server_reports_wiggum_version_not_the_dependency_version() -> TestResult {
        let server = build_server()?;
        assert_eq!(server.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(server.name(), "wiggum");
        Ok(())
    }

    #[test]
    fn server_registers_every_tool() -> TestResult {
        let server = build_server()?;
        assert_eq!(server.tool_count(), 12);
        Ok(())
    }

    #[test]
    fn result_text_reads_the_first_text_block() -> TestResult {
        let result = protocol_droid::domain::ir::JsonObject::new(
            r#"{"content":[{"type":"text","text":"hello"}],"isError":false}"#,
        )?;
        assert_eq!(result_text(&result).as_deref(), Some("hello"));
        Ok(())
    }

    #[test]
    fn replace_result_text_rewrites_only_the_text() -> TestResult {
        let result = protocol_droid::domain::ir::JsonObject::new(
            r#"{"content":[{"type":"text","text":"hello"}],"isError":false}"#,
        )?;
        let replaced = replace_result_text(result, "goodbye");
        assert_eq!(
            result_text(&replaced).as_deref(),
            Some("goodbye"),
            "the text should be the new one"
        );
        assert!(
            replaced.as_str().contains("\"isError\":false"),
            "the rest of the result should survive: {}",
            replaced.as_str()
        );
        Ok(())
    }
}
