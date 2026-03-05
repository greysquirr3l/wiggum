# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

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

[Unreleased]: https://github.com/greysquirr3l/wiggum/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/greysquirr3l/wiggum/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/greysquirr3l/wiggum/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/greysquirr3l/wiggum/releases/tag/v0.1.0
