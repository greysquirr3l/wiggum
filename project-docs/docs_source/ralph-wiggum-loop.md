# The Ralph Wiggum Loop

The Ralph Wiggum loop is an orchestration pattern for autonomous AI coding. An **orchestrator agent** drives **subagents** through a dependency-ordered task list until an entire project is implemented.

## How it works

1. The orchestrator reads `PROGRESS.md` to find the next incomplete task
2. It spawns a subagent with the corresponding task file as context
3. The subagent implements the task, runs preflight checks (build, test, lint), and commits
4. The orchestrator marks the task complete in `PROGRESS.md` and records any learnings
5. Repeat until all tasks are done

## Why it works

- **Bounded context** — Each subagent only sees one task file, keeping the prompt focused and reducing hallucination
- **Dependency ordering** — Tasks are topologically sorted, so each subagent builds on verified prior work
- **Preflight gates** — Every task must pass build/test/lint before being marked complete
- **Learnings accumulate** — The progress tracker captures insights from each task, providing growing context

## Origin

The pattern was proven on the [Yakko project](https://github.com/greysquirr3l/yakko), a Rust Microsoft Teams TUI client where 13 tasks across 7 phases were executed sequentially by subagents in a single automated session.

Wiggum makes this pattern reproducible for any project by generating the required artifacts from a structured plan definition.
