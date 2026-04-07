# Language Profiles

Wiggum ships with built-in profiles for 10 programming languages. Each profile provides sensible defaults for build commands, test patterns, documentation style, error handling conventions, security rules, and a vulnerability audit command.

## Supported languages

| Language | Build command | Test command | Lint command | Audit command |
|----------|--------------|-------------|-------------|--------------|
| Rust | `cargo build --workspace` | `cargo test --workspace` | `cargo clippy --workspace -- -D warnings` | `cargo audit` |
| Go | `go build ./...` | `go test -v ./...` | `go vet ./... && golangci-lint run ./...` | `govulncheck ./...` |
| TypeScript | `tsc --noEmit` | `vitest run` | `eslint .` | `npm audit --audit-level=high` |
| Python | `python -m py_compile` | `pytest -v` | `ruff check .` | `pip-audit` |
| Java | `mvn compile` | `mvn test` | `mvn checkstyle:check` | `mvn dependency-check:check` |
| C# / .NET | `dotnet build --nologo -v q` | `dotnet test --nologo -v q` | `dotnet format --verify-no-changes` | `dotnet list package --vulnerable` |
| Kotlin | `gradle build` | `gradle test` | `gradle detekt` | `gradle dependencyCheckAnalyze` |
| Swift | `swift build` | `swift test` | `swiftlint` | _(none)_ |
| Ruby | `ruby -c` | `bundle exec rspec` | `bundle exec rubocop` | `bundle exec bundler-audit check --update` |
| Elixir | `mix compile --warnings-as-errors` | `mix test` | `mix credo --strict` | `mix deps.audit` |

## What profiles provide

Each language profile includes:

- **Build success phrase** — The expected output indicating a successful build (e.g., "Compiling" for Rust, "Build complete" for Go)
- **Test file pattern** — Where test files are typically found (e.g., `tests/` for Rust, `*_test.go` for Go)
- **Doc style** — Documentation conventions (e.g., `///` doc comments for Rust, GoDoc for Go)
- **Error handling** — Idiomatic error handling approach (e.g., `Result<T, E>` for Rust, `error` return values for Go)
- **Security rules** — Six OWASP-derived, language-specific rules injected into every task and orchestrator prompt (see [Security](./security.md))
- **Audit command** — Supply-chain vulnerability scanner appended to every task's preflight chain

These values are injected into generated task files and exit criteria templates, making the output idiomatic for the target language.

## Overriding defaults

You can override any default preflight command in your plan's `[preflight]` section. Language profile defaults are used only when no explicit override is provided.

```toml
[preflight]
build = "cargo build --workspace --release"
audit = "cargo audit --deny warnings"
```

To disable the audit entirely, set it to an empty string:

```toml
[preflight]
audit = ""
```
