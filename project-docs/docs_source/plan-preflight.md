# Preflight and Orchestrator

## Preflight commands

The `[preflight]` section defines the commands that subagents run to verify their work. Language-specific defaults are provided automatically based on the project language, but you can override them.

```toml
[preflight]
build = "cargo build --workspace"
test = "cargo test --workspace"
lint = "cargo clippy --workspace -- -D warnings"
```

If omitted, Wiggum uses the defaults from the selected [language profile](./language-profiles.md).

## Orchestrator configuration

The `[orchestrator]` section configures the generated orchestrator prompt.

```toml
[orchestrator]
persona = "You are a senior Rust software engineer"
rules = [
    "Never log tokens at any log level",
    "Keep domain crate free of I/O dependencies",
    "Rust edition 2024, stable toolchain",
]
```

### Fields

| Field | Required | Description |
|-------|----------|-------------|
| `persona` | No | The subagent persona baked into every task prompt |
| `rules` | No | Project-specific rules included in each subagent prompt |

## Evaluator configuration

The optional `[evaluator]` section enables an independent QA agent that scores each task after the subagent marks it complete. When present, `.vscode/evaluator.prompt.md` is generated alongside the orchestrator prompt.

```toml
[evaluator]
persona = "You are a skeptical QA engineer"
pass_threshold = 7
hard_fail = true
test_tool = "cargo test --workspace"
```

### Fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `persona` | No | `"You are a rigorous QA evaluator"` | Evaluator agent persona |
| `pass_threshold` | No | `7` | Minimum score (0–10) for a criterion to pass |
| `hard_fail` | No | `false` | If `true`, abort the loop on any failed criterion |
| `test_tool` | No | Inherits `preflight.test` | Command the evaluator uses to run the test suite |
