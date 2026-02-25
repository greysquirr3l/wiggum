# Generated Artifacts

When you run `wiggum generate`, the following artifacts are produced in your project directory.

## Task files — `tasks/T{NN}-{slug}.md`

Each task becomes a numbered markdown file with a consistent structure:

- **Goal** — What the task accomplishes
- **Dependencies** — Which tasks must be complete first
- **Project Context** — Where this fits in the architecture
- **Implementation** — Guidance, file paths, type signatures, code snippets
- **Tests** — What to test and where
- **Preflight** — Commands to run before marking complete
- **Exit Criteria** — Verifiable conditions for completion

## Progress tracker — `PROGRESS.md`

A markdown table tracking all phases and tasks with status columns:

| Status | Meaning |
|--------|---------|
| `[ ]` | Not started |
| `[~]` | In progress |
| `[x]` | Complete |
| `[!]` | Blocked |

Includes a learnings column where the orchestrator records insights from each completed task.

## Implementation plan — `IMPLEMENTATION_PLAN.md`

A high-level architecture document derived from your plan's project description and phase structure. Subagents reference this for context about how their task fits into the overall project.

## Orchestrator prompt — `orchestrator.prompt.md`

The agent-mode prompt that drives the Ralph Wiggum loop. It tells the orchestrator how to read progress, spawn subagents, verify their output, and update the tracker.

## Agents manifest — `AGENTS.md`

Defines agent roles and capabilities. Can be skipped with `--skip-agents-md`.

## Parallel groups

If tasks have no mutual dependencies, Wiggum identifies them as parallelizable and notes this in the generated progress tracker.
