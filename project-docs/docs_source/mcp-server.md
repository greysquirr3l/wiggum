# MCP Server

Wiggum can run as an [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) server, allowing AI agents to invoke Wiggum's capabilities mid-session.

The server speaks **both** MCP `2025-11-25` and the upcoming `draft` revision in parallel over stdio transport (newline-delimited JSON). The active protocol version is negotiated per request from `params._meta["io.modelcontextprotocol/protocolVersion"]` — older clients that don't advertise a version fall back to the legacy shape automatically.

## Protocol versions

| Version | Status | Negotiation |
|---------|--------|-------------|
| `2025-11-25` | Stable | Default for requests without `_meta`; legacy `initialize` handshake always responds in this shape |
| `draft` | Pre-release (next MCP revision) | Opt-in by setting `params._meta.io.modelcontextprotocol/protocolVersion = "draft"` |

The `draft` revision is the next MCP spec change (anticipated end of July). It removes the session-level handshake and moves identity into per-request `_meta`. Wiggum is forward-compatible with both shapes until the draft is finalized.

## Draft entry point: `server/discover`

Clients that want to speak `draft` should call `server/discover` first to learn the server's supported versions and capabilities:

```json
{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{}}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "resultType": "complete",
    "protocolVersions": ["draft", "2025-11-25"],
    "defaultProtocolVersion": "draft",
    "capabilities": { "tools": { "listChanged": false }, "extensions": {} },
    "serverInfo": { "name": "wiggum", "version": "<package-version>" }
  }
}
```

Clients then include the chosen version on every subsequent request:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": { "_meta": { "io.modelcontextprotocol/protocolVersion": "draft" } }
}
```

Draft responses include:

- `resultType: "complete"` on every result
- `ttlMs` and `cacheScope` on `tools/list` results so clients can avoid redundant polling
- `extensions` field on capabilities (empty today)

Legacy responses omit these fields. Both code paths serve the same tool catalogue.

## Starting the server

```bash
wiggum serve --mcp
```

This starts the MCP server using stdio transport.

## Integration

To use Wiggum as an MCP server with your AI coding tool, add it to your MCP configuration. For example, in VS Code's MCP settings:

```json
{
  "servers": {
    "wiggum": {
      "command": "wiggum",
      "args": ["serve", "--mcp"]
    }
  }
}
```

This enables agents to generate plans and task scaffolds directly within a coding session, without leaving the editor.

## Available tools

| Tool | Description |
|------|-------------|
| `wiggum_version` | Return wiggum version metadata (package, git SHA, MCP protocol) |
| `wiggum_generate_plan` | Generate full scaffold from a plan TOML file path |
| `wiggum_validate_plan` | Validate a plan TOML file (dependency DAG check, missing fields) |
| `wiggum_lint_plan` | Run quality lint rules against a plan TOML file |
| `wiggum_check_plan` | Score plan quality on five dimensions (granularity, dependency health, coverage, richness, token budget); returns overall score 0–10 and actionable suggestions |
| `wiggum_draft_plan` | Generate a skeleton `plan.toml` from a natural-language description; takes `project_name`, `description`, `language`, and optional `task_slugs` |
| `wiggum_read_progress` | Parse PROGRESS.md and return structured status |
| `wiggum_update_progress` | Update a task's status in PROGRESS.md |
| `wiggum_list_templates` | List available language/architecture templates |
| `wiggum_report` | Generate a post-execution report from PROGRESS.md |
| `wiggum_generate_agents_md` | Generate an AGENTS.md file from a plan TOML |
| `wiggum_bootstrap` | Scan an existing project directory and generate a skeleton plan TOML |

## Protocol compliance

The server handles all required lifecycle messages:

- `initialize` — always responds with the legacy `2025-11-25` shape regardless of `_meta` (so older clients that still send the handshake keep working)
- `server/discover` — new draft entry point. Returns `protocolVersions`, `defaultProtocolVersion`, `capabilities`, and `serverInfo`
- `notifications/initialized` and `notifications/cancelled` — silently acknowledged (no response, per spec)
- `notifications/roots/list_changed` — accepted and ignored (removed in the draft; notifications don't elicit responses)
- `ping` — responds with an empty result at any lifecycle phase (removed in the draft but still accepted for back-compat)
- `logging/setLevel` — accepted and ignored (removed in the draft)
- `tools/list` — returns the full tool catalogue; adds `ttlMs`/`cacheScope` and `resultType: "complete"` for draft clients
- `tools/call` — dispatches to the named tool and returns `content` + optional `isError`; adds `resultType: "complete"` for draft clients

Unknown methods return JSON-RPC error `-32601`. Tool execution errors are returned as tool results with `isError: true` rather than protocol errors, so agents can self-correct.

## Runtime security guardrails

MCP `tools/call` execution includes a baseline guardrail pipeline:

- **Input guardrail**: blocks mutating tools when arguments contain common prompt-injection patterns (for example, attempts to override instructions or request exfiltration).
- **Output redaction**: redacts common sensitive values in tool output text before returning content to the caller (emails, SSN format, bearer tokens, and basic secret-assignment patterns).
- **Output hard block**: blocks responses that still contain high-risk secret markers (for example private key headers).
- **Security events**: emits structured security events via `tracing` with event type, tool name, and detail for incident investigation.
- **Session anomaly monitoring**: tracks tool-call sequences in-process and emits alerts for high read volume and suspicious read-to-write pivots.

Set `WIGGUM_MCP_GUARDRAIL_STRICT=true` to hard-block on session anomalies instead of alert-only mode.

These controls are intentionally lightweight and deterministic. They provide a production baseline for MCP sessions and can be extended with stricter policy engines where required.
