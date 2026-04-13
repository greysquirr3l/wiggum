# Security

Wiggum bakes security into every generated plan at three levels: rules embedded in subagent prompts, a vulnerability audit command appended to every preflight chain, and an automatically injected security hardening task for plans with web-facing surface.

## Why it's automatic

Independent security research consistently finds that AI-generated code introduces OWASP Top 10 vulnerabilities at high rates — particularly hardcoded secrets, SQL injection, missing HTTP security headers, disconnected rate limiting, unsafe file uploads, and SSRF. Wiggum treats these as structural concerns that belong in every plan by default, not optional additions the user must remember to include.

## Level 1 — Security rules in every subagent prompt

Every generated task file and orchestrator prompt includes a `## Security (non-negotiable)` section populated from the language profile. These six rules are always injected:

| Category | Rule |
|----------|------|
| Secrets | Credentials and API keys must only be read from environment variables or a secrets manager — never hardcoded |
| SQL injection | All database queries must use parameterised inputs — never interpolate user input into query strings |
| Security headers | HTTP servers must set Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, and X-Content-Type-Options |
| Rate limiting | Rate-limiting middleware must be wired to the router, not just defined — verified by a smoke test |
| File uploads | Upload handlers must validate MIME type server-side, reject executable extensions, and enforce a maximum file size |
| SSRF | Any feature fetching URLs on behalf of a user must validate the target against an explicit allowlist |

Rules are language-specific (e.g. the SQL rule references `sqlx` for Rust, `PreparedStatement` for Java, Ecto for Elixir) but cover the same six categories for every language.

You can add project-specific security rules on top via `[orchestrator] rules`:

```toml
[orchestrator]
rules = [
    "HMAC secrets must never appear in log output at any log level.",
    "All outbound HTTP requests must use a timeout of 10 seconds.",
]
```

## Level 2 — Vulnerability audit in every preflight

Each language profile includes an `audit_cmd` that is appended to the preflight chain run after every task:

| Language | Audit command |
|----------|--------------|
| Rust | `cargo audit` |
| Go | `govulncheck ./...` |
| TypeScript | `npm audit --audit-level=high` |
| Python | `pip-audit` |
| Java | `mvn dependency-check:check` |
| C# | `dotnet list package --vulnerable` |
| Kotlin | `gradle dependencyCheckAnalyze` |
| Ruby | `bundle exec bundler-audit check --update` |
| Elixir | `mix deps.audit` |
| Swift | _(no standard tool; field left empty)_ |

So for a Rust plan, every task's preflight block becomes:

```bash
cargo build --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo audit
```

And the task's exit criteria automatically includes:

- [ ] `cargo audit` reports no vulnerabilities

### Overriding the audit command

Override per-plan in `[preflight]`:

```toml
[preflight]
audit = "cargo audit --deny warnings"
```

### Disabling the audit

Set `audit` to an empty string:

```toml
[preflight]
audit = ""
```

## Level 3 — Auto-injected security hardening task

When your plan contains web-facing surface, Wiggum automatically appends a `security-hardening` task as the final task. Web surface is detected from task slugs and titles containing any of: `http`, `api`, `server`, `router`, `route`, `endpoint`, `handler`, `webhook`, `upload`, `auth`, `login`, `session`, `request`, `response`, `middleware`, `web`, `rest`, `grpc`, `graphql`.

The injected task has:

- **Goal** — Verify and enforce the six OWASP baseline security properties across the entire codebase
- **Hints** — One concrete guidance item per category (grep for secrets, verify parameterised queries, check headers are wired, write a rate-limit smoke test, inspect upload handlers, check URL-fetching allowlists)
- **Test hints** — Rate-limit smoke test (assert HTTP 429 at N+1 requests), upload rejection test, SSRF rejection test
- **Must-haves** — Six items, one per OWASP category
- **Evaluation criteria** — Five verifiable conditions scored by the evaluator

This task depends on the last explicit task in your plan, so it always runs last. The evaluator will hard-fail if any criterion is not met when `[evaluator] hard_fail = true`.

### Opting out

If you're handling security via a separate process or your plan doesn't actually have web surface, suppress injection with:

```toml
[security]
skip_hardening_task = true
```

## Integration Audits

AI-generated code often compiles successfully but has two common failure modes:

1. **Disconnected wiring** — modules, services, and handlers are created but never actually connected to the application (e.g., a service class exists but is never instantiated and used)
2. **Stub implementations** — placeholder code like `todo!()`, `unimplemented!()`, or `raise NotImplementedError` that compiles but crashes at runtime

Wiggum auto-injects two late-stage audit tasks when your plan has 3+ tasks:

### Integration wiring audit

Verifies all components are properly connected:

- All public exports are actually imported and used somewhere
- All route handlers/controllers are registered with the router
- All service interfaces have implementations that are instantiated
- Middleware is mounted on the request pipeline

### Stub cleanup audit

Finds and replaces placeholder implementations:

- Searches for language-specific stub patterns (`todo!()`, `NotImplementedError`, etc.)
- Ensures all TODOs for completed tasks are resolved
- Verifies all code paths are reachable and functional

Each language profile includes specific patterns and hints tailored to its ecosystem.

### Opting out of integration audits

```toml
[integration]
skip_wiring_audit = true   # Disable wiring audit
skip_stub_audit = true     # Disable stub cleanup audit
```

See [Preflight and Orchestrator — Integration configuration](./plan-preflight.md#integration-configuration) for field details.

You can also manually include a task with the slug `security-hardening` in your plan — if that slug is already present, auto-injection is skipped automatically.

## Integration Audits

Beyond security vulnerabilities, AI-generated code frequently has two structural failure modes that lead to runtime crashes:

1. **Disconnected wiring** — modules, services, and handlers are created but never actually connected to the application
2. **Stub implementations** — placeholder code like `todo!()`, `unimplemented!()`, or `raise NotImplementedError` that compiles but crashes at runtime

Wiggum auto-injects two late-stage audit tasks when your plan has 3+ tasks:

### Integration wiring audit

The `integration-wiring` task verifies all components are properly connected:

| Check | Description |
|-------|-------------|
| Public exports | All public items from library modules are imported and used somewhere |
| Route registration | All handlers/controllers are registered with the router/framework |
| Service instantiation | All interfaces have implementations that are actually instantiated |
| Background tasks | All workers/jobs are spawned in application startup |
| Middleware | All middleware/interceptors are mounted on the request pipeline |
| Configuration | Config values are read and passed to components that need them |

Each language profile provides specific wiring hints tailored to its ecosystem (e.g., "Confirm every port trait has at least one adapter implementation wired in `main.rs`" for Rust hexagonal architecture).

### Stub cleanup audit

The `stub-cleanup` task finds and replaces placeholder implementations:

| Language | Stub patterns searched |
|----------|----------------------|
| Rust | `todo!()`, `unimplemented!()`, `panic!("not implemented")`, `// TODO`, `// FIXME` |
| Go | `panic("not implemented")`, `// TODO:`, `return nil, errors.New("not implemented")` |
| TypeScript | `throw new Error("Not implemented")`, `// TODO:`, `// @ts-ignore` |
| Python | `raise NotImplementedError`, `pass # TODO`, `# FIXME:` |
| Java | `throw new UnsupportedOperationException()`, `// TODO`, `return null; // stub` |
| ... | (full list in each language profile) |

### Opting out

Suppress either or both audits with:

```toml
[integration]
skip_wiring_audit = true   # Disable wiring audit
skip_stub_audit = true     # Disable stub cleanup audit
```

You can also manually include tasks with slugs `integration-wiring` or `stub-cleanup` — if either slug is already present, the corresponding auto-injection is skipped.
