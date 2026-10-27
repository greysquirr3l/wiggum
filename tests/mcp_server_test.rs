//! End-to-end tests for the MCP server over stdio.
//!
//! These drive the real binary the way a peer does: newline-delimited JSON
//! on stdin, JSON-RPC answers on stdout. The point of testing at this level
//! is the handshake and the tool answers — the parts a peer actually sees.
//! Anything below this is covered by the unit tests in the `mcp` module.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;

/// Sends the given JSON-RPC messages to `wiggum serve --mcp` and returns the
/// answers it wrote back, one per line.
fn call_server(requests: &[Value]) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wiggum"))
        .arg("serve")
        .arg("--mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    {
        let stdin = child.stdin.as_mut().ok_or("no stdin")?;
        for request in requests {
            writeln!(stdin, "{request}")?;
        }
    }

    let output = child.wait_with_output()?;
    let stdout = String::from_utf8(output.stdout)?;

    Ok(stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<Vec<Value>, _>>()?)
}

fn initialize(protocol_version: &str) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": protocol_version,
            "capabilities": {},
            "clientInfo": { "name": "wiggum-test", "version": "1" }
        }
    })
}

fn call_version_tool() -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": { "name": "wiggum_version", "arguments": {} }
    })
}

/// Walks a path of object keys, returning `None` if any step is missing.
fn field<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
}

/// The answer carrying the given JSON-RPC id.
fn answer_with_id(answers: &[Value], id: i64) -> Result<&Value, Box<dyn std::error::Error>> {
    answers
        .iter()
        .find(|answer| answer.get("id").and_then(Value::as_i64) == Some(id))
        .ok_or_else(|| format!("no answer with id {id}").into())
}

/// The text of the first content block of a `tools/call` answer.
fn tool_text(answer: &Value) -> Result<String, Box<dyn std::error::Error>> {
    let text = field(answer, &["result", "content"])
        .and_then(|content| content.get(0))
        .and_then(|block| block.get("text"))
        .and_then(Value::as_str)
        .ok_or("the answer carried no text block")?;
    Ok(text.to_string())
}

#[test]
fn handshake_answers_with_the_revision_the_peer_asked_for() -> Result<(), Box<dyn std::error::Error>>
{
    let answers = call_server(&[initialize("2024-11-05")])?;
    let handshake = answer_with_id(&answers, 1)?;

    assert_eq!(
        field(handshake, &["result", "protocolVersion"]),
        Some(&Value::from("2024-11-05"))
    );
    assert_eq!(
        field(handshake, &["result", "serverInfo", "name"]),
        Some(&Value::from("wiggum"))
    );
    assert_eq!(
        field(handshake, &["result", "serverInfo", "version"]),
        Some(&Value::from(env!("CARGO_PKG_VERSION"))),
        "serverInfo.version should be wiggum's version, not the protocol crate's"
    );

    Ok(())
}

#[test]
fn version_tool_reports_the_negotiated_revision_not_the_ceiling()
-> Result<(), Box<dyn std::error::Error>> {
    // A peer that asked for an old revision must be told about that
    // revision. Reporting the newest revision the build could speak would
    // name a revision this session is not speaking.
    for requested in ["2024-11-05", "2025-06-18"] {
        let answers = call_server(&[initialize(requested), call_version_tool()])?;
        let text = tool_text(answer_with_id(&answers, 2)?)?;

        assert!(
            text.contains(&format!("MCP protocol: {requested}")),
            "the version tool should report the negotiated revision {requested}, got:\n{text}"
        );
        assert!(
            !text.contains("MCP protocol: 2025-11-25"),
            "a {requested} peer should not be told the session speaks 2025-11-25, got:\n{text}"
        );
    }

    Ok(())
}

#[test]
fn version_tool_reports_the_negotiable_range_alongside() -> Result<(), Box<dyn std::error::Error>> {
    let answers = call_server(&[initialize("2024-11-05"), call_version_tool()])?;
    let text = tool_text(answer_with_id(&answers, 2)?)?;

    assert!(
        text.contains("Negotiable: 2024-11-05 through 2025-11-25"),
        "the version tool should also report the range, got:\n{text}"
    );

    Ok(())
}

#[test]
fn input_guardrail_blocks_an_injection_split_across_two_arguments()
-> Result<(), Box<dyn std::error::Error>> {
    // The arguments are searched as a set, so the pattern is caught however
    // it is split and whatever order the keys arrive in.
    let answers = call_server(&[
        initialize("2024-11-05"),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "wiggum_generate_plan",
                "arguments": { "plan_path": "/tmp/ignore", "target": "previous instructions" }
            }
        }),
    ])?;

    let text = tool_text(answer_with_id(&answers, 2)?)?;
    assert!(
        text.contains("blocked by MCP input guardrail"),
        "the injection should have been blocked, got:\n{text}"
    );

    Ok(())
}
