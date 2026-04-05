# Introduction

**Wiggum** is a CLI tool and MCP server that generates structured task files for autonomous AI coding loops. It codifies the **Ralph Wiggum loop** pattern — an orchestrator agent that drives subagents through a dependency-ordered task list until an entire project is implemented, hands-off.

## What it does

Given a structured plan definition (TOML), Wiggum produces:

- **Task files** (`tasks/T{NN}-{slug}.md`) — Structured markdown specs with goals, dependencies, implementation guidance, test requirements, preflight commands, and exit criteria
- **Progress tracker** (`PROGRESS.md`) — A phase/task table with status tracking and learnings
- **Orchestrator prompt** (`orchestrator.prompt.md`) — The agent-mode prompt that drives the loop
- **Implementation plan** (`IMPLEMENTATION_PLAN.md`) — Architecture overview for subagent context
- **Agents manifest** (`AGENTS.md`) — Agent role definitions

## Why it exists

Setting up an AI orchestration loop currently requires hand-authoring all of these artifacts. The structural and mechanical parts — numbering, dependency wiring, progress tables, preflight commands, orchestrator boilerplate — should be generated. The creative parts — what to build, architecture decisions, implementation details — come from the user.

## Design principles

- **Agent-agnostic** — Wiggum generates artifacts, not agent invocations. Works with any AI coding tool that can read markdown.
- **Scaffold, don't execute** — Wiggum produces plans and task files. Execution is someone else's job.
- **Language-aware** — Ships with profiles for Rust, Go, TypeScript, Python, Java, C#, Kotlin, Swift, Ruby, and Elixir, providing sensible defaults for build, test, lint, and security audit commands.
- **Security by default** — Six OWASP-derived rules are injected into every task and orchestrator prompt automatically. Supply-chain audits run on every task completion. Plans with web-facing surface get an auto-appended security hardening task with verifiable exit criteria. None of this requires configuration.
