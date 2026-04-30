# MCP Server

Wiggum can run as an [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) server, allowing AI agents to invoke Wiggum's capabilities mid-session.

The server implements **MCP protocol version `2025-11-25`** over stdio transport (newline-delimited JSON).

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
| `wiggum_read_progress` | Parse PROGRESS.md and return structured status |
| `wiggum_update_progress` | Update a task's status in PROGRESS.md |
| `wiggum_list_templates` | List available language/architecture templates |
| `wiggum_report` | Generate a post-execution report from PROGRESS.md |
| `wiggum_generate_agents_md` | Generate an AGENTS.md file from a plan TOML |
| `wiggum_bootstrap` | Scan an existing project directory and generate a skeleton plan TOML |

## Protocol compliance

The server handles all required lifecycle messages:

- `initialize` — responds with `protocolVersion: "2025-11-25"` and tool capabilities
- `notifications/initialized` and `notifications/cancelled` — silently acknowledged (no response, per spec)
- `ping` — responds with an empty result at any lifecycle phase
- `tools/list` — returns the full tool catalogue
- `tools/call` — dispatches to the named tool and returns `content` + optional `isError`

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
