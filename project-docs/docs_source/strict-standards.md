# Strict Standards

Beyond the language-specific security rules that every plan inherits, Wiggum offers an opt-in **strict mode** that injects a richer, harder-to-satisfy rule set into every prompt. Strict mode is for projects where "the build passes" is not enough — every commit must produce code that complies with a verified-modern toolchain baseline for the target language.

## Enabling strict mode

Add `strict = true` to your `[style]` section:

```toml
[style]
strict = true   # inject the language's strict ruleset
```

When `strict = true`, the orchestrator injects the matching language block into every subagent's task prompt and **escalates the profile's lint/audit commands** — it does not replace the existing `lint_cmd` / `audit_cmd`, it adds to them. The intent is the same as the Rust profile: fail-secure by default, parse at boundaries, no panics on untrusted input, no weak crypto, supply-chain audited in CI, warnings treated as errors.

Disable per-task if a specific task is a spike or prototype:

```toml
[[phases.tasks]]
slug = "experiment-with-rust-macros"
title = "Investigate derive macro ergonomics"
goal = "Spike to evaluate macro options; produce a recommendation document."
strict = false
```

## What strict mode adds

Each language profile defines a `strict_rules` array that is injected as a new section in the generated prompts only when `strict = true`. The rules are language-specific but share a common theme: they encode the modern, security-centric baseline that the language's toolchain makes possible when fully engaged.

For Rust, strict mode mirrors the rules in your project's `nick.md` (the personal standards file). For every other language, the ruleset is documented in the companion file `docs/strict-lints.md` at the root of the wiggum repo.

Example strict-mode rule excerpts by language:

| Language | Sample strict rules (see `docs/strict-lints.md` for the full set) |
|----------|---------------------------------------------------------------------|
| Rust | No `.unwrap()` / `.expect()` / `panic!` in production code; no index slicing that can panic; no `#[allow(clippy::...)]` suppressions; full pedantic + nursery + perf clippy profile with hard denials |
| Go | `golangci-lint v2` + `gofumpt` + `govulncheck`; never discard errors; `context.Context` everywhere; `depguard` bans on `crypto/md5`, `crypto/sha1`, `math/rand` |
| TypeScript | `typescript-eslint v8` strictTypeChecked; `noUncheckedIndexedAccess`; Zod at every input boundary; no `any` / `!`; `node:crypto` for randomness |
| Python | Ruff with the `S` (bandit) group on; `mypy --strict`; `pip-audit`; no `pickle.loads` / `yaml.load`; `secrets` for tokens |
| Java | `Error Prone` + `NullAway` + `SpotBugs` findsecbugs; `PreparedStatement` only; no `ObjectInputStream` on untrusted data |
| C# / .NET | Roslyn `AnalysisMode=All` + `Nullable=enable` + Security Code Scan; no `!` null-forgiving; no `BinaryFormatter` |
| Kotlin | detekt `allRules` + `explicitApi()`; no `!!`; no `GlobalScope`; structured concurrency only |
| Swift | Swift 6 language mode + complete strict concurrency; no `@unchecked Sendable`; no force-unwrap/try/cast outside tests |
| Ruby | RuboCop `Security/*` + `Lint/*` as errors; Brakeman with `-z`; Sorbet `# typed: strict` |
| Elixir | `--warnings-as-errors` + `mix credo --strict` + Dialyzer + Sobelow `--exit`; never `String.to_atom/1` on user input |
| PHP | PHPStan `level max` + `phpstan-strict-rules` + Psalm `--taint-analysis`; `declare(strict_types=1)`; `password_hash` (Argon2id); `random_bytes` / `random_int` |

## Cross-language baseline

Every language profile's `strict_rules` includes the same closing pair drawn from the cross-language baseline in `docs/strict-lints.md`:

- **Treat warnings as errors** — the language's "warnings as errors" switch stays on; a warning fails the build.
- **No suppression without justification** — never blanket-disable a rule; suppress narrowly, inline, with a rule ID and a one-line reason. Prefer fixing.

These hold regardless of language and are injected alongside the language-specific rules.

## Where strict rules appear

When `strict = true`, every generated artifact that contains prompt content gets the strict block:

- **VSCode target** — `orchestrator.prompt.md`, each `tasks/T{NN}-{slug}.md`, `evaluator.prompt.md` (when `[evaluator]` is configured)
- **opencode target** — `wiggum-orchestrator.md`, `wiggum-implementer.md`, `wiggum-evaluator.md`
- **Claude target** — `CLAUDE.md` (so Claude Code sees the rules on every session)
- **agent-rules target** — `.cursorrules`, `.windsurfrules`, `.github/copilot-instructions.md` (so Cursor / Windsurf / Copilot users see them too)

## Tooling version pins

The strict profiles track specific toolchain versions because the rules are written against those tools' surface area. When you adopt strict mode, pin your project to the version that matches the rules — moving to a newer toolchain without re-pinning the rules can leave gaps.

Current pins (see `docs/strict-lints.md` for the canonical list):

- Go — `golangci-lint v2` + Go 1.24+ + `gofumpt`
- TypeScript — `typescript-eslint v8` flat config + `projectService: true`
- Python — Ruff (linter + formatter) + `mypy --strict` + Python 3.12+
- Java — `Error Prone` + `NullAway` + `SpotBugs` + `findsecbugs` on JDK 21+
- C# / .NET — Roslyn `AnalysisMode=All` + Security Code Scan on .NET 8+
- Kotlin — detekt `allRules = true` + ktlint on JDK 21+
- Swift — Swift 6 language mode + SwiftLint `--strict`
- Ruby — RuboCop (with `rubocop-performance`, `rubocop-rspec`) + Brakeman + Sorbet on Ruby 3.2+
- Elixir — Credo `--strict` + Dialyzer via Dialyxir + Sobelow on Elixir 1.16+ / OTP 26+
- PHP — PHPStan 2.x at `level max` + Psalm `--taint-analysis` on PHP 8.3+

## When to enable strict mode

Enable `strict = true` when:

- The project is security-sensitive (auth, payments, PII, infra)
- The team is multi-engineer and you want a single canonical standard instead of per-developer conventions
- You're starting a new codebase and you want to prevent AI-generated slop from accumulating
- The project will outlive any one AI model's current capabilities — rules survive the model

Leave `strict = false` (the default) when:

- You're prototyping or evaluating wiggum itself
- The codebase predates the strict toolchain baseline (e.g. a Python 2 codebase)
- You need fast iteration and are willing to clean up later

Strict mode is additive, not destructive. You can flip `strict = true` on an existing plan at any time — the next `wiggum generate` injects the new rules into all subagent prompts without altering task content, hints, or preflight commands.