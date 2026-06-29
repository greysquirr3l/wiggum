# Targets

Wiggum can generate scaffold artifacts for one or more AI coding tools at the
same time. Each "target" is a different way of running the orchestrator
loop — the underlying plan, tasks, progress tracker, and features registry are
shared, but the agent prompts and configuration differ.

## Supported targets

| Target | Stable identifier | Agent file(s) | Dispatch mechanism |
|---|---|---|---|
| **VSCode** (default) | `vscode` | `.vscode/orchestrator.prompt.md` (and three siblings) | GitHub Copilot `runSubagent` tool |
| **opencode** | `opencode` | `.opencode/agents/wiggum-orchestrator.md` (and four siblings) | opencode `task` tool with subagent frontmatter |
| **Claude** | `claude` | `CLAUDE.md` (project memory) + `.claude/settings.json` (hooks) | Claude Code reads both files on every session; PreCompact hook blocks compaction mid-task |
| **agent-rules** | `agent-rules` | `.cursorrules` + `.windsurfrules` + `.github/copilot-instructions.md` | The receiving IDE drives its own agent loop; wiggum supplies only rules + project context |

You can enable any combination of targets for a single generate run.

> **Note on the `vscode` target:** it targets GitHub Copilot Chat specifically (the `#tool:agent/runSubagent` tool). Other VSCode forks — Cursor, Windsurf, Antigravity, Trae, Cody, Cline, Roo Code, Continue — do not implement that tool. If you use one of those forks, enable the **`agent-rules`** target instead so its corresponding rules file (`.cursorrules`, `.windsurfrules`, etc.) gets written.

## Selection

Targets are selected via the plan TOML or a CLI flag. The CLI flag always wins.

### Plan-level: `[targets]`

```toml
[targets]
vscode   = true   # default if [targets] is absent
opencode = true
claude   = false
```

Each field is optional. When `[targets]` is absent entirely, the default is
`vscode = true` and the others are `false` — this preserves the pre-`[targets]`
behavior exactly.

When the `[targets]` section is present, only the fields you set take effect;
absent fields are treated as `false`.

### CLI: `--target`

```bash
wiggum generate plan.toml --target opencode          # just opencode
wiggum generate plan.toml --target all               # all four
wiggum generate plan.toml --target agent-rules       # Cursor / Windsurf / Copilot rules
wiggum generate plan.toml --target vscode,opencode   # not supported — single value
```

`--target` accepts a single value: `vscode`, `opencode`, `claude`, `agent-rules`, or `all`.

### Precedence

1. The `--target` CLI flag (if provided) overrides everything.
2. Otherwise, the plan's `[targets]` section.
3. Otherwise, the default (`vscode` only) for back-compat.

If the resolved `TargetSet` is empty (every field explicitly `false`),
`wiggum generate` errors out — at least one target must be enabled.

## How the targets differ

### VSCode target

- **Files:** `.vscode/orchestrator.prompt.md`, `.vscode/evaluator.prompt.md`,
  `.vscode/planner.prompt.md`, `.vscode/background-auditor.prompt.md`.
- **Format:** Each file is a Copilot prompt file with a YAML frontmatter
  (`agent: agent`, `description:`) and a body that includes a `<SUBAGENT_PROMPT>`
  block the orchestrator dispatches via `#tool:agent/runSubagent`.
- **Model selection:** the `[orchestrator].model` field is rendered as a
  *recommendation* note in the prompt. The actual model is selected by the
  user in the Copilot chat picker.
- **Per-dispatch model:** `[orchestrator].subagent_model` is passed as the
  `model:` argument on each `runSubagent` call.
- **Evaluator prompt** is generated only when `[evaluator]` is configured.

### opencode target

- **Files:** `.opencode/agents/wiggum-orchestrator.md`,
  `.opencode/agents/wiggum-implementer.md`,
  `.opencode/agents/wiggum-evaluator.md`,
  `.opencode/agents/wiggum-planner.md`,
  `.opencode/agents/wiggum-auditor.md`.
- **Format:** Each file is an opencode agent with full YAML frontmatter
  (`description:`, `mode: primary|subagent`, `model: provider/model-id`,
  `permission:`, `prompt:`).
- **Subagent dispatch:** the orchestrator uses the `task` tool with
  `subagent_type: "wiggum-implementer"`. There is no per-dispatch `model:`
  argument — the model is pinned in the implementer agent's own frontmatter.
- **Permissions:** the orchestrator frontmatter allows `task` only for
  `wiggum-implementer`, `wiggum-evaluator`, and `wiggum-auditor`; subagents
  deny `task` entirely.
- **Implementer body** is shared across all dispatches — the orchestrator
  passes the task file path as an `@path` reference at dispatch time.
- **Evaluator agent** is generated only when `[evaluator]` is configured.

### Claude target

- **Files:** `CLAUDE.md` (project memory at repo root) and
  `.claude/settings.json` (hooks).
- **`CLAUDE.md`** — Claude Code reads this file on every session. It
  contains the project persona, preflight commands, architecture rules,
  user-defined rules, security rules, AI-avoidance guidance (if enabled),
  and a workflow loop. Claude Code IS its own orchestrator; wiggum just
  supplies the context + rules it needs.
- **Hook:** `PreCompact` blocks context compression while any `[~]` task
  exists in `PROGRESS.md`.
- Combined, the two files constitute "full Claude Code support" — wiggum
  drives neither the loop nor the dispatch; Claude Code does.

### agent-rules target

- **Files:** `.cursorrules`, `.windsurfrules`, and
  `.github/copilot-instructions.md`. All three are emitted from a single
  shared template, so the rules stay in lockstep across forks.
- **Use case:** VSCode forks that don't speak the GitHub Copilot
  `runSubagent` or opencode `task` protocols — Cursor, Windsurf,
  Antigravity, Trae, Cody, Cline, Roo Code, Continue. Each of those IDEs
  reads its own format's file as project-level instructions.
- **No orchestrator loop.** Unlike `vscode` and `opencode`, these files
  contain rules + project context only. The receiving IDE drives its own
  agent loop; wiggum never dispatches subagents on its behalf.
- **GitHub Copilot** reads `.github/copilot-instructions.md` as
  repository-level instructions — this works in VSCode + Copilot even
  when the `vscode` target is also enabled (the two are complementary).

## Universal artifacts

The following files are always emitted, regardless of the active target set:

- `PROGRESS.md` — the task tracker
- `IMPLEMENTATION_PLAN.md` — the high-level plan
- `AGENTS.md` — tool-agnostic agent instructions
- `features.json` — structured task/criteria registry
- `tasks/T{NN}-{slug}.md` — per-task files

## Examples

### Default (back-compat)

A plan with no `[targets]` section generates only the VSCode artifacts —
exactly the pre-`[targets]` behavior.

```bash
wiggum generate plan.toml
# → .vscode/orchestrator.prompt.md
# → .vscode/evaluator.prompt.md   (if [evaluator] configured)
# → .vscode/planner.prompt.md
# → .vscode/background-auditor.prompt.md
# → .claude/settings.json          (when claude = true)
```

### opencode-only

```toml
[targets]
vscode   = false
opencode = true
```

```bash
wiggum generate plan.toml
# → .opencode/agents/wiggum-orchestrator.md
# → .opencode/agents/wiggum-implementer.md
# → .opencode/agents/wiggum-evaluator.md   (if [evaluator] configured)
# → .opencode/agents/wiggum-planner.md
# → .opencode/agents/wiggum-auditor.md
```

### agent-rules-only (Cursor / Windsurf / Copilot)

```toml
[targets]
vscode      = false
opencode    = false
claude      = false
agent-rules = true
```

```bash
wiggum generate plan.toml
# → .cursorrules                    (Cursor)
# → .windsurfrules                  (Windsurf)
# → .github/copilot-instructions.md (GitHub Copilot)
```

### Multi-target

```bash
wiggum generate plan.toml --target all
# → VSCode files AND opencode files AND CLAUDE.md + .claude/settings.json
#   AND .cursorrules + .windsurfrules + .github/copilot-instructions.md
```

## Cleaning up

`wiggum clean` removes generated files for all targets. To clean only one
target's files, delete the relevant directory by hand
(e.g. `rm -rf .opencode`).

## Custom templates

`.wiggum/templates/` overrides still work, with two layouts:

- **Flat (legacy):** `.wiggum/templates/orchestrator.opencode.md` overrides
  the opencode orchestrator only.
- **Subdir (new):** `.wiggum/templates/opencode/orchestrator.md` is also
  discovered and takes priority over the flat layout. Subdirs map to target
  names: `vscode`, `opencode`.

Custom template names that match the opencode variants:

| Subdir layout | Flat layout |
|---|---|
| `.wiggum/templates/vscode/orchestrator.md` | `.wiggum/templates/orchestrator.md` |
| `.wiggum/templates/vscode/evaluator.md` | `.wiggum/templates/evaluator.md` |
| `.wiggum/templates/vscode/planner.md` | `.wiggum/templates/planner.md` |
| `.wiggum/templates/vscode/background-auditor.md` | `.wiggum/templates/background-auditor.md` |
| `.wiggum/templates/opencode/orchestrator.md` | `.wiggum/templates/orchestrator_opencode.md` |
| `.wiggum/templates/opencode/implementer.md` | `.wiggum/templates/implementer.md` |
| `.wiggum/templates/opencode/evaluator.md` | `.wiggum/templates/evaluator_opencode.md` |
| `.wiggum/templates/opencode/planner.md` | `.wiggum/templates/planner_opencode.md` |
| `.wiggum/templates/opencode/background-auditor.md` | `.wiggum/templates/background_auditor_opencode.md` |
