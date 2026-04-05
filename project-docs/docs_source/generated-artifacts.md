# Generated Artifacts

When you run `wiggum generate`, the following artifacts are produced in your project directory.

## Task files — `tasks/T{NN}-{slug}.md`

Each task becomes a numbered markdown file with a consistent structure:

- **Goal** — What the task accomplishes
- **Dependencies** — Which tasks must be complete first
- **Project Context** — Where this fits in the architecture
- **Implementation** — Guidance, file paths, type signatures, code snippets
- **Tests** — What to test and where
- **Security** — Six non-negotiable OWASP rules from the language profile, always present
- **Preflight** — Commands to run before marking complete (build, test, lint, and security audit)
- **Exit Criteria** — Verifiable conditions for completion, including a `cargo audit` (or equivalent) check

## Progress tracker — `PROGRESS.md`

A markdown table tracking all phases and tasks with status columns:

| Status | Meaning |
|--------|---------|
| `[ ]` | Not started |
| `[~]` | In progress |
| `[x]` | Complete |
| `[!]` | Blocked |

Includes a **Codebase State** section where subagents record which files were created or modified. This gives each subsequent subagent accurate context about what the previous one actually changed.

Includes a learnings column where the orchestrator records insights from each completed task.

## Implementation plan — `IMPLEMENTATION_PLAN.md`

A high-level architecture document derived from your plan's project description and phase structure. Subagents reference this for context about how their task fits into the overall project.

## Orchestrator prompt — `.vscode/orchestrator.prompt.md`

The agent-mode prompt that drives the Ralph Wiggum loop. It tells the orchestrator how to read progress, spawn subagents, verify their output independently, and update the tracker. Includes a sprint contract step, a codebase state handoff step, and a guard against premature completion.

## Feature registry — `features.json`

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

## Auto-injected security hardening task

When your plan contains web-facing surface (task slugs or titles containing `http`, `api`, `server`, `webhook`, `upload`, `auth`, etc.), Wiggum automatically appends a `security-hardening` task as the final task in the resolved task list. This task has pre-populated `hints`, `test_hints`, `must_haves`, and `evaluation_criteria` covering all six OWASP baseline categories.

Suppress with `[security] skip_hardening_task = true` in your plan, or by including your own task with the slug `security-hardening`. See [Security](./security.md) for details.

## Evaluator prompt — `.vscode/evaluator.prompt.md`

Only generated when `[evaluator]` is configured in the plan. Defines a skeptical QA agent that re-runs preflight independently, scores each exit criterion, and updates `features.json` with verified results. Prevents false completions caused by the orchestrator trusting the subagent's self-report.

## Agents manifest — `AGENTS.md`

Defines agent roles and capabilities. Can be skipped with `--skip-agents-md`.

## Parallel groups

If tasks have no mutual dependencies, Wiggum identifies them as parallelizable and notes this in the generated progress tracker.
