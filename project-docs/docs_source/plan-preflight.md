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
| `strategy` | No | `standard` | Execution strategy: `standard` (goal → implement → test → preflight), `tdd` (red → green → refactor → preflight), `gsd` (must-haves checklist → implement → verify), `complete` (root-fix end-to-end → tests including failure paths → docs update → preflight) |
| `max_retries` | No | `2` | Maximum number of preflight-fail/retry cycles before `on_failure` is applied |
| `on_failure` | No | `pause` | Action taken when a task exhausts `max_retries`: `pause`, `skip`, or `escalate` |
| `model` | No | — | Recommended model identifier (e.g. `"claude-opus-4.7"`, `"gpt-5"`, `"gemini-2.5-pro"`) for the orchestrator agent itself. Rendered as a header note in `orchestrator.prompt.md`; Wiggum cannot enforce the picker choice from a prompt file. |
| `subagent_model` | No | — | Model identifier passed as `model:` to every `runSubagent` call the orchestrator dispatches for implementation work. Lets you run the orchestrator on a stronger model (e.g. Opus) while implementation subagents use a cheaper one (e.g. Sonnet or Haiku). |
| `rules` | No | | Project-specific rules included in each subagent prompt. Appended after the automatic security rules from the language profile. |

### Failure actions

When a task exhausts its `max_retries` budget, the orchestrator applies `on_failure`:

| Value | Behaviour |
|-------|-----------|
| `pause` | Emit a `GATE` banner and stop — a human must restart to proceed. **Default.** |
| `skip` | Mark the task `[!]` (blocked) and continue to the next available task |
| `escalate` | Emit a structured failure block with a diagnosis summary into `PROGRESS.md`, then continue |

```toml
[orchestrator]
max_retries = 3
on_failure  = "escalate"
```

### Model selection

By default the orchestrator, every implementation subagent, and the evaluator all
inherit whatever model is selected in the VS Code Copilot Chat picker at the time
you run the prompt. Three optional fields let you pin different models for each role
so you can drive orchestration with a stronger reasoning model while implementation
runs on something cheaper:

```toml
[orchestrator]
model          = "claude-opus-4.7"     # orchestrator agent (recommendation header)
subagent_model = "claude-sonnet-4.5"   # every runSubagent call for implementation

[evaluator]
model          = "claude-sonnet-4.5"   # evaluator agent
```

How each field is applied depends on the active [target](./targets.md):

- **VSCode target** (default):
  - `[orchestrator] model` — rendered as a **Recommended model** header at the top
    of `orchestrator.prompt.md`. Wiggum cannot enforce a model from a prompt file,
    so this is a reminder to set the picker before you start the loop.
  - `[orchestrator] subagent_model` — injected into the orchestrator's instructions
    as `model: "<name>"` on every `#tool:agent/runSubagent` call dispatched for
    implementation work.
  - `[evaluator] model` — header note on `evaluator.prompt.md` plus the `model:`
    argument when the orchestrator dispatches the evaluator as a subagent.
- **opencode target:**
  - `[orchestrator] model` — written into the orchestrator agent's frontmatter
    (`model:` field). opencode uses the frontmatter model when the agent runs.
  - `[orchestrator] subagent_model` — written into the implementer agent's
    frontmatter. opencode does not support per-dispatch `model:` arguments —
    the subagent's model is pinned by the agent definition itself.
  - `[evaluator] model` — written into the evaluator agent's frontmatter.
- **Claude target:**
  - `[orchestrator] model`, `[orchestrator] subagent_model`, and `[evaluator] model`
    are not currently rendered into `CLAUDE.md` — Claude Code uses the model
    selected in its own picker at session start. The fields are preserved on the
    plan for future use and for consistency with other targets.

Equivalent ChatGPT or Gemini model identifiers work the same way — the string is
passed through to `runSubagent` verbatim, so whatever the picker accepts is valid
(e.g. `"gpt-5"`, `"gpt-5-mini"`, `"gemini-2.5-pro"`, `"gemini-2.5-flash"`).

#### Local / BYOK models

Any model registered in the VS Code Copilot Chat picker is valid — including
local runtimes and OpenAI-compatible endpoints added through **Manage Models…**
(Ollama, LM Studio, llama.cpp server, vLLM, Azure OpenAI, etc.). Use the exact
label the picker shows, in the form `"Model Name (Vendor)"`:

```toml
[orchestrator]
model          = "Claude Opus 4.7 (Anthropic)"
subagent_model = "Qwen 2.5 Coder 32B (Ollama)"

[evaluator]
model = "GLM 4.6 (LM Studio)"
```

Practical caveats when pinning a local model as the subagent runner:

- The local server must be running before the orchestrator dispatches a subagent —
  wiggum does not start it for you.
- Local models typically have much smaller effective context windows than hosted
  ones. Keep `subagent_model` for narrow implementation tasks and leave the
  orchestrator on a hosted model with a large context, or shrink task scope via
  `wiggum check` and per-task hints before running.
- `runSubagent` calls the local model through the same VS Code language-model API
  as hosted providers; tool calling, parallel groups, and the preflight loop work
  the same way, but availability of advanced features depends on what the local
  backend implements.

Common pairings:

| Use case | `model` | `subagent_model` | Evaluator `model` |
|---|---|---|---|
| Highest quality | `claude-opus-4.7` | `claude-sonnet-4.5` | `claude-sonnet-4.5` |
| Budget-conscious | `claude-sonnet-4.5` | `claude-haiku-4.5` | `claude-haiku-4.5` |
| ChatGPT stack | `gpt-5` | `gpt-5-mini` | `gpt-5-mini` |
| Gemini stack | `gemini-2.5-pro` | `gemini-2.5-flash` | `gemini-2.5-flash` |

All three fields are optional and independent — omit any of them to fall back to
the picker-selected model.

### `complete` strategy

> _Inspired by Gary Tam's (Y Combinator) execution standard for AI-assisted development: every task must be a finished deliverable, not a partial checkpoint._

Use `strategy = "complete"` when you want each task treated as a finished deliverable instead of a partial checkpoint. Generated prompts will require:

- Root-cause fix (not workaround) when in scope
- Tests for behavior changes, including edge/failure path coverage
- Documentation updates in the same task
- Full preflight pass before task completion

The completion contract is baked into the orchestrator prompt, each task file, and AGENTS.md so every participant in the loop sees the same standard.

Use `--dry-run` to preview the generated output before running:

```bash
# Preview what each strategy generates without writing any files
wiggum generate plan.toml --dry-run
```

Change `strategy` in `[orchestrator]`, run `--dry-run`, and compare. The orchestrator prompt is the primary artifact that changes between strategies.

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
| `model` | No | — | Model identifier for the evaluator agent. Rendered as a header note in `evaluator.prompt.md` and passed as `model:` when the orchestrator dispatches the evaluator via `runSubagent`. |

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

## Integration configuration

The optional `[integration]` section controls Wiggum's automatic integration audit tasks that catch common AI failure modes.

```toml
[integration]
skip_wiring_audit = false
skip_stub_audit = false
```

### Fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `skip_wiring_audit` | No | `false` | When `true`, suppresses auto-injection of the `integration-wiring` task |
| `skip_stub_audit` | No | `false` | When `true`, suppresses auto-injection of the `stub-cleanup` task |

Both audit tasks are auto-injected when your plan has 3+ tasks. The wiring audit verifies that all components are properly connected (routes registered, services instantiated, middleware mounted). The stub cleanup audit searches for placeholder implementations like `todo!()`, `unimplemented!()`, or `raise NotImplementedError`.

See [Security — Integration Audits](./security.md#integration-audits) for full details.

## Style configuration

The optional `[style]` section controls writing style guidance to reduce detectability of AI-generated code.

```toml
[style]
avoid_ai_patterns = true
avoid_god_files = true
```

### Fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `avoid_ai_patterns` | No | `true` | When enabled, prompts receive hints to avoid common AI writing patterns |
| `avoid_god_files` | No | `true` | When enabled, prompts include file-structure guidance that discourages creating "God" files |
| `strict` | No | `false` | When `true`, injects the language-specific strict rule set (full pedantic clippy for Rust, golangci-lint v2 for Go, PHPStan `level max` for PHP, etc.) into every prompt. See [Strict Standards](./strict-standards.md). |

When `avoid_ai_patterns` is enabled, generated prompts include guidance to:

- **Avoid "slop" vocabulary** — Words like "robust", "comprehensive", "leverage", "utilize", "delve", "embark", "streamlined", "cutting-edge", "pivotal", "seamless", "synergistic", "transformative", "harness", "facilitate", "innovative"
- **Skip filler phrases** — Phrases like "it's worth noting that", "at its core", "let's break this down", "in order to", "from a broader perspective", "a key takeaway is"
- **Prevent prompt leakage** — Avoid echoing instructions or stating "As an AI..." in comments
- **Write naturally** — Prefer direct, human-like language over formal or corporate phrasing
- **Self-documenting code** — Favor meaningful names over excessive comments

Each language profile includes `ai_avoidance_rules` and `comment_guidelines` that are injected when this setting is enabled.

When `avoid_god_files` is enabled, generated prompts also include guidance to:

- Keep files focused on one primary responsibility
- Create a focused module/file for new concerns instead of extending unrelated files
- Avoid catch-all files (`utils`, `helpers`, `common`) containing unrelated logic
- Split overloaded files before adding more behavior

When `avoid_god_files` is enabled **and** `architecture = "hexagonal"`, prompts additionally include:

- Introduce the port trait first when splitting an overloaded file, then move the implementation — never invent the interface and migrate code in the same step

### Disabling AI pattern avoidance

```toml
[style]
avoid_ai_patterns = false
avoid_god_files = false
```
