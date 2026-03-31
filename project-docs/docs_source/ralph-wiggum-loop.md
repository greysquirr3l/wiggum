# The Ralph Wiggum Loop

The Ralph Wiggum loop is an orchestration pattern for autonomous AI coding. An **orchestrator agent** drives **subagents** through a dependency-ordered task list until an entire project is implemented.

## How it works

1. The orchestrator reads `PROGRESS.md` and `features.json` to find the next incomplete task
2. It agrees a sprint contract with the subagent before handoff (scope, files, exit criteria)
3. The subagent implements the task, runs preflight checks (build, test, lint), and commits
4. The orchestrator independently re-runs preflight to verify — it does not trust the subagent's self-report
5. If an `[evaluator]` agent is configured, it scores each exit criterion and updates `features.json`
6. The orchestrator marks the task complete in `PROGRESS.md`, records the codebase state diff and any learnings
7. Repeat until all tasks show `[x]`

## Why it works

- **Bounded context** — Each subagent only sees one task file, keeping the prompt focused and reducing hallucination
- **Dependency ordering** — Tasks are topologically sorted, so each subagent builds on verified prior work
- **Preflight gates** — Every task must pass build/test/lint before being marked complete
- **Independent verification** — The orchestrator re-runs preflight itself rather than trusting the subagent's checkbox
- **Feature registry** — `features.json` tracks per-criterion pass/fail state as objective ground truth
- **Codebase state handoff** — Each subagent records what it changed, giving the next one accurate context
- **Learnings accumulate** — The progress tracker captures insights from each task, providing growing context

## Origin

The pattern was proven on the [Yakko project](https://github.com/greysquirr3l/yakko), a Rust Microsoft Teams TUI client where 13 tasks across 7 phases were executed sequentially by subagents in a single automated session.

Wiggum makes this pattern reproducible for any project by generating the required artifacts from a structured plan definition.
