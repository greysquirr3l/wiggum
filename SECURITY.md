# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a vulnerability

If you discover a security vulnerability in Wiggum, please report it responsibly.

**Do not open a public issue.** Instead, use [GitHub's private vulnerability reporting](https://github.com/greysquirr3l/wiggum/security/advisories/new) to submit a report.

You should receive an acknowledgment within 48 hours. We will work with you to understand the issue and coordinate a fix before any public disclosure.

## Scope

Wiggum is a scaffold generator that reads TOML plan files and writes markdown artifacts. Its security surface includes:

- **Plan file parsing** — TOML deserialization of user-provided input
- **Template rendering** — Tera template expansion with plan-derived values
- **Filesystem operations** — Reading plans and writing generated files to user-specified output directories
- **MCP server** — stdio-based Model Context Protocol server

## Practices

- Strict clippy lints with `unwrap`, `expect`, `panic`, and unchecked indexing denied
- Dependency auditing via `cargo-audit` and `cargo-deny` in CI
- OSSF Scorecard analysis on the repository
