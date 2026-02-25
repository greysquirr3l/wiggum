# Language Profiles

Wiggum ships with built-in profiles for 10 programming languages. Each profile provides sensible defaults for build commands, test patterns, documentation style, and error handling conventions.

## Supported languages

| Language | Build command | Test command | Lint command |
|----------|--------------|-------------|-------------|
| Rust | `cargo build --workspace` | `cargo test --workspace` | `cargo clippy --workspace -- -D warnings` |
| Go | `go build ./...` | `go test -v ./...` | `go vet ./... && golangci-lint run ./...` |
| TypeScript | `tsc --noEmit` | `vitest run` | `eslint .` |
| Python | `python -m py_compile` | `pytest -v` | `ruff check .` |
| Java | `mvn compile` | `mvn test` | `mvn checkstyle:check` |
| C# | `dotnet build` | `dotnet test` | `dotnet format --verify-no-changes` |
| Kotlin | `gradle build` | `gradle test` | `gradle detekt` |
| Swift | `swift build` | `swift test` | `swiftlint` |
| Ruby | `ruby -c` | `bundle exec rspec` | `bundle exec rubocop` |
| Elixir | `mix compile --warnings-as-errors` | `mix test` | `mix credo --strict` |

## What profiles provide

Each language profile includes:

- **Build success phrase** — The expected output indicating a successful build (e.g., "Compiling" for Rust, "Build complete" for Go)
- **Test file pattern** — Where test files are typically found (e.g., `tests/` for Rust, `*_test.go` for Go)
- **Doc style** — Documentation conventions (e.g., `///` doc comments for Rust, GoDoc for Go)
- **Error handling** — Idiomatic error handling approach (e.g., `Result<T, E>` for Rust, `error` return values for Go)

These values are injected into generated task files and exit criteria templates, making the output idiomatic for the target language.

## Overriding defaults

You can override the default preflight commands in your plan's `[preflight]` section. The language profile defaults are used only when no explicit override is provided.
