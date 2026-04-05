# Preflight and Orchestrator

## Preflight commands

The `[preflight]` section defines the commands that subagents run to verify their work. Language-specific defaults are provided automatically based on the project language, but you can override them.

```toml
[preflight]
build = "cargo build --workspace"
test  = "cargo test --workspace"
lint  = "cargo clippy --workspace -- -D warnings"
```

If omitted, Wiggum uses the defaults from the selected [language profile](./language-profiles.md).

### Security audit command

Each language profile includes a default vulnerability audit command that is appended to the preflight chain and added as an exit criterion on every task. For Rust this is `cargo audit`; for TypeScript, `npm audit --audit-level=high`; for Python, `pip-audit`; etc.

Override it per-plan:

```toml
[preflight]
audit = "cargo audit --deny warnings"
```

Disable it by setting an empty string:

```toml
[preflight]
audit = ""
```

See the full list of per-language defaults in [Language Profiles](./language-profiles.md).

## Orchestrator configuration

The `[orchestrator]` section configures the generated orchestrator prompt.

```toml
[orchestrator]
persona   = "You are a senior Rust software engineer"
strategy  = "standard"
rules = [
    "Never log tokens at any log level",
    "Keep domain crate free of I/O dependencies",
    "Rust edition 2024, stable toolchain",
]
```

### Fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `persona` | No | `"You are a senior software engineer"` | The subagent persona baked into every task prompt |
| `strategy` | No | `standard` | Execution strategy: `standard` (goal → implement → test → preflight), `tdd` (red → green → refactor → preflight), `gsd` (must-haves checklist → implement → verify) |
| `rules` | No | | Project-specific rules included in each subagent prompt. Appended after the automatic security rules from the language profile. |

## Evaluator configuration

The optional `[evaluator]` section enables an independent QA agent that scores each task after the subagent marks it complete. When present, `.vscode/evaluator.prompt.md` is generated alongside the orchestrator prompt.

```toml
[evaluator]
persona        = "You are a skeptical QA engineer"
pass_threshold = 7
hard_fail      = true
test_tool      = "cargo test --workspace"
```

### Fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `persona` | No | `"You are a rigorous QA evaluator"` | Evaluator agent persona |
| `pass_threshold` | No | `7` | Minimum score (0–10) for a criterion to pass |
| `hard_fail` | No | `false` | If `true`, abort the loop on any failed criterion |
| `test_tool` | No | Inherits `preflight.test` | Command the evaluator uses to run the test suite |

## Security configuration

The optional `[security]` section controls Wiggum's automatic security features.

```toml
[security]
skip_hardening_task = false
```

### Fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `skip_hardening_task` | No | `false` | When `true`, suppresses auto-injection of the `security-hardening` task even if web-surface keywords are detected in task slugs |

See [Security](./security.md) for a complete description of all three levels of automatic security hardening.
