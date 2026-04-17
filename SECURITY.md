# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.7.x   | Yes       |
| 0.6.x   | Yes       |
| < 0.6.0 | No        |

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

- **Strict clippy lints** — `unwrap`, `expect`, `panic`, and unchecked indexing **denied** at compile time
- **Dependency auditing** — `cargo-audit` runs on every push; supply-chain CVEs caught early
- **Automated security analysis** — CodeQL SAST scans on every push and PR
- **Automated dependency updates** — Dependabot weekly updates with PR reviews; major versions reviewed manually
- **OSSF Scorecard** — Continuous security posture monitoring; results at <https://api.securityscorecards.dev/projects/github.com/greysquirr3l/wiggum>
- **Token permissions** — GitHub Actions use minimal required permissions (least privilege)
- **Release-chain hardening** — Auto-tag runs only after CI succeeds on `main`; release publishing is gated on CI success for the tagged SHA
- **Dangerous workflow protection** — No `pull_request_target`; `workflow_run` is used only for non-privileged chaining (no token escalation)

## Recommendations for maintainers

To further strengthen this repository's security:

1. **Require branch protection on main:**
   - Require at least 1 approved review before merging
   - Require status checks to pass (CI, clippy, tests, CodeQL)
   - Dismiss stale pull request approvals when new commits are pushed
   - Restrict who can push to main (admin/maintainers only)
   - Keep **admin bypass enabled** for emergency direct commits while preserving all protections for non-admin contributors

2. **Consider signing commits:**
   - Enable "Vigilant mode" in GitHub user settings to require GPG-signed commits
   - Add repository rule requiring signed commits if your GitHub plan supports it

3. **Monitor Dependabot alerts:**
   - Review and merge Dependabot PRs promptly
   - Optional: Configure branch protection rules to auto-merge minor/patch updates after CI passes

4. **Publish releases:**
   - Create GitHub Releases from tags with signed tags (optional GPG signing)
   - Keep release notes updated in CHANGELOG.md

5. **Protect release tags and secrets:**
   - Restrict who can create `v*` tags
   - Keep `CARGO_REGISTRY_TOKEN` scoped to crates publish only
   - Rotate registry tokens periodically and after maintainer changes
