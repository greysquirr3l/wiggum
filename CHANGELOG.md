# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.6.2] - 2026-04-08

### Changed

- Updated MCP server protocol version to `2025-06-18` (latest official MCP spec)
- Enhanced Cargo.toml with comprehensive clippy linting configuration: enabled `clippy::all`, `clippy::pedantic`, `clippy::nursery`, `clippy::cargo`, and `clippy::perf` with targeted allow/deny overrides for production-grade code quality enforcement

### Dependencies

- Updated all dependencies to latest versions (clap 4.6.0, tokio 1.51.1, toml 1.1.2, and others)

## [0.6.1] - 2026-04-07

### Changed

- Orchestrator template now explicitly extracts the **Accumulated Learnings** and **Codebase State** sections from `PROGRESS.md` and injects them verbatim into the subagent dispatch message, rather than instructing the subagent to read the file itself — ensures context is active and cannot be skipped
- Subagent prompt preamble in all three strategy variants (`standard`, `tdd`, `gsd`) updated to note that learnings are pre-injected by the orchestrator rather than requiring a redundant file read step

### Added

- Security rules embedded in every generated subagent prompt — six OWASP-derived, language-specific rules (`security_rules` field on `LanguageProfile`) covering secrets management, parameterised queries, HTTP security headers, rate limiting, file upload validation, and SSRF prevention; always injected into the `## Security (non-negotiable)` section of orchestrator and task prompts
- `audit_cmd` on `LanguageProfile` — per-language vulnerability audit command (`cargo audit`, `govulncheck ./...`, `npm audit --audit-level=high`, `pip-audit`, etc.) appended to every task's preflight chain and added as an exit criterion automatically
- `preflight.audit` field in plan TOML — overrides or disables the language-default audit command; inherits from the language profile when absent; set to `""` to disable
- `[security]` section in plan TOML — `skip_hardening_task` boolean (default `false`) to suppress the auto-injected security hardening task
- Auto-injected `security-hardening` task — appended as the final task when web-facing surface is detected from task slugs/titles (`http`, `api`, `server`, `webhook`, `upload`, `auth`, etc.); includes pre-populated `hints`, `test_hints`, `must_haves`, and `evaluation_criteria` for all six OWASP categories; can be suppressed with `[security] skip_hardening_task = true` or by including a task with the slug `security-hardening` manually
- New `Security` book chapter (`docs_source/security.md`) covering all three hardening levels with override and opt-out examples

### Changed

- `LanguageProfile` extended with `security_rules` and `audit_cmd` fields (all ten language profiles updated)
- `Preflight` struct gains optional `audit` field; `with_defaults` populates it from the language profile
- `Plan` struct gains `security: SecurityConfig` field
- `plan-preflight.md`, `language-profiles.md`, `generated-artifacts.md`, and `introduction.md` updated to document the new fields and behavior
- `SUMMARY.md` updated to include the new Security page

### Fixed

- Added `[evaluator]` section and `evaluation_criteria` fields to the example plan (`reference/example-plan.toml`)
- Fixed task fields table in docs — removed phantom `context` and `tests` fields, added `test_hints`, `must_haves`, `gate`, and `evaluation_criteria`
- Added `strategy` field to orchestrator documentation with all three variants (`standard`, `tdd`, `gsd`)

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

[Unreleased]: https://github.com/greysquirr3l/wiggum/compare/v0.6.2...HEAD
[0.6.2]: https://github.com/greysquirr3l/wiggum/compare/v0.6.1...v0.6.2
[0.6.1]: https://github.com/greysquirr3l/wiggum/compare/v0.5.0...v0.6.1
[0.4.0]: https://github.com/greysquirr3l/wiggum/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/greysquirr3l/wiggum/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/greysquirr3l/wiggum/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/greysquirr3l/wiggum/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/greysquirr3l/wiggum/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/greysquirr3l/wiggum/releases/tag/v0.1.0
