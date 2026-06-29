# Generated Artifacts

When you run `wiggum generate`, the following artifacts are produced in your project directory.

## Universal artifacts (always emitted)

These files are produced regardless of the selected target set. They are tool-agnostic and form the shared state of the loop.

### Task files — `tasks/T{NN}-{slug}.md`

Each task becomes a numbered markdown file with a consistent structure:

- **Goal** — What the task accomplishes
- **Dependencies** — Which tasks must be complete first
- **Project Context** — Where this fits in the architecture
- **Implementation** — Guidance, file paths, type signatures, code snippets
- **Tests** — What to test and where
- **Security** — Six non-negotiable OWASP rules from the language profile, always present
- **Preflight** — Commands to run before marking complete (build, test, lint, and security audit)
- **Exit Criteria** — Verifiable conditions for completion, including a `cargo audit` (or equivalent) check

### Progress tracker — `PROGRESS.md`

A markdown table tracking all phases and tasks with status columns:

| Status | Meaning |
|--------|---------|
| `[ ]` | Not started |
| `[~]` | In progress |
| `[x]` | Complete |
| `[!]` | Blocked |

Includes a **Codebase State** section where subagents record which files were created or modified. This gives each subsequent subagent accurate context about what the previous one actually changed.

Includes a learnings column where the orchestrator records insights from each completed task.

### Implementation plan — `IMPLEMENTATION_PLAN.md`

A high-level architecture document derived from your plan's project description and phase structure. Subagents reference this for context about how their task fits into the overall project.

### Agents manifest — `AGENTS.md`

Defines agent roles and capabilities. Auto-discovered by opencode (and any other tool that follows the `AGENTS.md` convention). Can be skipped with `--skip-agents-md`.

### Feature registry — `features.json`

A structured JSON file listing every task with its pass/fail state and per-criterion results. Both the orchestrator and evaluator reference this as the source of truth for what is actually complete.

```json
{
  "project": "my-app",
  "tasks": [
    {
      "id": "T01",
      "slug": "project-setup",
      "title": "Initialize project structure",
      "passes": false,
      "criteria": [
        { "label": "build succeeds", "passes": false },
        { "label": "all tests pass", "passes": false },
        { "label": "linter clean", "passes": false },
        { "label": "implementation matches goal", "passes": false }
      ]
    }
  ]
}
```

Custom criteria can be added per-task via the `evaluation_criteria` field in your plan TOML.

### Auto-injected security hardening task

When your plan contains web-facing surface (task slugs or titles containing `http`, `api`, `server`, `webhook`, `upload`, `auth`, etc.), Wiggum automatically appends a `security-hardening` task as the final task in the resolved task list. This task has pre-populated `hints`, `test_hints`, `must_haves`, and `evaluation_criteria` covering all six OWASP baseline categories.

Suppress with `[security] skip_hardening_task = true` in your plan, or by including your own task with the slug `security-hardening`. See [Security](./security.md) for details.

## Per-target artifacts

Wiggum emits tool-specific agent prompts and configuration based on the active `[targets]` set. See [Targets](./targets.md) for how to select targets.

### VSCode target — `.vscode/*.prompt.md`

| File | Role |
|---|---|
| `.vscode/orchestrator.prompt.md` | The agent-mode prompt that drives the implementation loop. It tells the orchestrator how to read progress, spawn subagents, verify their output independently, and update the tracker. Includes a sprint contract step, a codebase state handoff step, and a guard against premature completion. |
| `.vscode/evaluator.prompt.md` | Only generated when `[evaluator]` is configured. Defines a skeptical QA agent that re-runs preflight independently, scores each exit criterion, and updates `features.json` with verified results. |
| `.vscode/planner.prompt.md` | An agent-mode prompt for the planning phase. The planner subagent assists with breaking down new work items, estimating complexity, and suggesting task decompositions — without touching the implementation. |
| `.vscode/background-auditor.prompt.md` | A continuously running QA companion that watches for regressions while the orchestrator advances through tasks. |

These prompts use GitHub Copilot's `runSubagent` tool to dispatch subagents.

### opencode target — `.opencode/agents/wiggum-*.md`

| File | Role |
|---|---|
| `.opencode/agents/wiggum-orchestrator.md` | Primary agent (`mode: primary`) that drives the loop. Dispatches the implementer via the `task` tool with the per-task context. |
| `.opencode/agents/wiggum-implementer.md` | Subagent (`mode: subagent`) that executes a single task file. The orchestrator references the specific task file via `@path` at dispatch time. |
| `.opencode/agents/wiggum-evaluator.md` | Subagent. Only generated when `[evaluator]` is configured. |
| `.opencode/agents/wiggum-planner.md` | Subagent for the planning phase. |
| `.opencode/agents/wiggum-auditor.md` | Subagent for continuous cross-task regression watching. |

The agent frontmatter pins the model and declares permissions — for example, the orchestrator allows `task` only for `wiggum-implementer`, `wiggum-evaluator`, and `wiggum-auditor`, and denies `edit` so it can only update `PROGRESS.md` through the implementer.

### Claude target — `CLAUDE.md` + `.claude/settings.json`

The Claude target gives Claude Code **full project context** at session start, plus a
companion hook that protects active work. Two files:

- **`CLAUDE.md`** (repo root) — Claude Code's project memory file, loaded on every
  session. It contains the project persona, preflight commands, architecture rules,
  user-defined rules, security rules, AI-avoidance guidance (when
  `[style] avoid_ai_patterns = true`), and the workflow loop. Claude Code IS its own
  orchestrator — wiggum supplies context + rules, Claude Code drives dispatch.
- **`.claude/settings.json`** — A `PreCompact` hook that blocks context compression
  while any in-progress task marker (`[~]`) exists in `PROGRESS.md`. This prevents
  Claude from compacting away active working state mid-task, preserving the full task
  context until the task is marked complete.

### agent-rules target — `.cursorrules`, `.windsurfrules`, `.github/copilot-instructions.md`

The agent-rules target emits three fork-neutral rules files from a single shared
template, so the rules stay in lockstep across forks. It is designed for VSCode-family
IDEs that do not speak the GitHub Copilot `runSubagent` or opencode `task` protocols.

| File | Read by |
|---|---|
| `.cursorrules` | Cursor (project-level rules) |
| `.windsurfrules` | Windsurf (project-level rules; same format as `.cursorrules`) |
| `.github/copilot-instructions.md` | GitHub Copilot (repo-level instructions); also picked up by some VSCode forks |

Each file contains the project metadata, preflight, architecture, user rules, security
rules, AI-avoidance guidance (when enabled), strict rules (when enabled), strategy,
and commit conventions — **no orchestrator-loop directives**. The receiving IDE
drives its own agent loop; wiggum never dispatches subagents on its behalf.

> **Why a separate target?** The `vscode` target emits prompts that call
> `#tool:agent/runSubagent`, which is GitHub Copilot Chat-specific. Cursor, Windsurf,
> and other VSCode forks do not implement that tool. Use the `agent-rules` target
> when targeting those forks.

## Parallel groups

If tasks have no mutual dependencies, Wiggum identifies them as parallelizable and notes this in the generated progress tracker.
