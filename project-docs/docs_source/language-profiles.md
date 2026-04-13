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
- **Security rules** — Language-specific rules covering OWASP categories plus crypto, deserialization, and path traversal (see [Security](./security.md)); profiles currently define 14 or 15 rules depending on the language
- **Audit command** — Supply-chain vulnerability scanner appended to every task's preflight chain
- **AI avoidance rules** — Guidelines to reduce detectability of AI-generated code (see below)
- **Comment guidelines** — Best practices for meaningful, non-robotic comments

Profile defaults are applied in different places during generation: build/test/audit settings flow into generated task and preflight content, while security, AI-avoidance, and comment guidance are injected through the orchestrator and exit-criteria templates rather than directly into `task.md`.

## AI avoidance rules

When `[style] avoid_ai_patterns = true` (the default), each language profile injects five AI avoidance rules:

| Rule | Description |
|------|-------------|
| Slop vocabulary | Avoid words like "robust", "leverage", "comprehensive", "delve", "embark" |
| Filler phrases | Skip "it's worth noting that", "at its core", "let's break this down" |
| Prompt leakage | Never echo instructions or write "As an AI..." in code or comments |
| Natural writing | Use direct, human language — not corporate jargon |
| Self-documenting | Prefer meaningful names over excessive comments |

## Comment guidelines

Profiles also include language-specific comment guidance. Common themes include:

| Guideline | Description |
|-----------|-------------|
| WHY not WHAT | Prefer comments that explain reasoning, tradeoffs, or intent rather than restating the code |
| Preserve important annotations | Keep existing structured comments such as safety or security annotations when the target language and codebase use them |
| Delete tutorial comments | Remove temporary instructional or step-by-step comments after implementation |
| Use project conventions for TODOs | Follow the repository's existing TODO/FIXME ownership and formatting conventions |
| Doc contracts | Document preconditions, postconditions, and error cases when they are important to callers |

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
