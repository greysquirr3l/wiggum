# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.5.0] - 2026-03-31

### Added

- `[evaluator]` section in plan TOML — opt-in QA evaluator agent that independently re-runs preflight, scores every exit criterion, and updates `features.json` with verified pass/fail state
- `features.json` generated artifact — structured JSON registry of all tasks with per-criterion pass/fail tracking; referenced by both the orchestrator and evaluator agent
- `evaluation_criteria` field on tasks — per-task list of verifiable exit criteria embedded in task files and `features.json`
- `.vscode/evaluator.prompt.md` generated artifact — evaluator agent prompt, only emitted when `[evaluator]` is configured
- Codebase State section in `PROGRESS.md` — subagents record file-level changes after each task, giving the next subagent accurate handoff context
- Sprint contract step in all orchestrator strategy variants — orchestrator agrees on scope with the subagent before handoff
- Independent verification step in orchestrator loop — orchestrator re-runs preflight itself before trusting a subagent's `[x]` checkbox
- Premature victory guard in orchestrator prompt — warns the orchestrator not to declare completion until all tasks show `[x]`

## [0.4.0] - 2026-03-15

### Added

- `wiggum resume` command to recover interrupted orchestrator loops — auto-detects the last in-progress or next task, generates resume prompt with context
- `wiggum diff <old.toml> <new.toml>` command to compare two plan files — shows phase changes, task additions/removals, and dependency modifications
- `wiggum retro` command to generate improvement suggestions from PROGRESS.md learnings — analyzes retry patterns, blocking gates, and complexity issues
- `wiggum split --task <slug>` command to interactively split oversized tasks into smaller units — includes dependency rewiring
- `wiggum templates` subcommand with `list`, `show`, and `save` operations — enables reusable task snippets stored in `~/.wiggum/templates/`
- `wiggum prices` command to display model pricing data — shows bundled rates for cost estimation
- Per-task cost estimation in `--dry-run --estimate-tokens` output — shows estimated costs for Claude, GPT-4, and Gemini models
- `domain::pricing` module with bundled model pricing data

### Dependencies

- Added `dirs` crate for cross-platform home directory detection (used by templates)

## [0.3.2] - 2026-03-11

### Added

- `gate` field on tasks — optional human-in-the-loop stop. When set, the orchestrator emits the gate message and pauses before starting the task; the task file renders a `⛔ GATE` banner at the top.

## [0.3.1] - 2026-03-07

### Changed

- Updated to Rust 1.94.0; enhanced Cargo.toml formatting with TOML 1.1 multi-line inline tables
- Set MSRV to 1.94.0 to leverage stable TOML 1.1 support

### Removed

- Removed `.coraline/` from `.gitignore` (Coraline is a separate project)

## [0.3.0] - 2026-03-05

### Added

- TODO/FIXME/HACK/XXX housekeeping section in generated task files
- Exit criteria item for unresolved markers within task scope

## [0.2.0] - 2026-02-26

### Added

- `wiggum clean` command to remove generated artifacts (`--dry-run` supported)

## [0.1.0] - 2026-02-24

### Added

- Interactive plan creation (`wiggum init`)
- Plan generation from TOML definitions (`wiggum generate`)
- Plan validation with DAG cycle detection (`wiggum validate`)
- Lint rules for plan quality checks (`--lint`)
- Interactive task addition (`wiggum add-task`)
- Project bootstrapping from existing codebases (`wiggum bootstrap`)
- Dry-run mode with token estimation (`--dry-run`, `--estimate-tokens`)
- MCP server for agent integration (`wiggum serve --mcp`)
- Post-execution reports from PROGRESS.md (`wiggum report`)
- Live progress monitoring (`wiggum watch`)
- Language profiles for Rust, Go, TypeScript, Python, Java, C#, Kotlin, Swift, Ruby, and Elixir
- AGENTS.md generation with opt-out (`--skip-agents-md`)
- Parallel task group identification
- Learnings column in generated PROGRESS.md
- VCS-aware reporting with git timeline
- mdBook documentation site

[Unreleased]: https://github.com/greysquirr3l/wiggum/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/greysquirr3l/wiggum/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/greysquirr3l/wiggum/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/greysquirr3l/wiggum/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/greysquirr3l/wiggum/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/greysquirr3l/wiggum/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/greysquirr3l/wiggum/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/greysquirr3l/wiggum/releases/tag/v0.1.0
