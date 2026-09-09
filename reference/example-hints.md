# example-hints.md — markdown hints file for `wiggum reverse`

#

# Pass this file with `wiggum reverse <url> --hints example-hints.md` to

# append freeform rules to the orchestrator's rule list.

#

# Only `## Rules` (and aliases `## Code style`, `## Orchestrator Rules`)

# sections are extracted. Everything else is ignored.

#

# For structured overrides (language, architecture, persona, phases) use

# `example-hints.toml` instead.

# Project notes

Some preamble here that `wiggum reverse` will ignore — only bullets under
`## Rules` are picked up.

## Rules

- Use the latest stable Go (1.23+)
- No globals — wire dependencies through constructors
- Use `slog` for structured logging
- Never log secrets or PII at any level
- Prefer table-driven tests with `testify`
- Run `go vet ./...` before every commit

## Code style

- Run `gofmt -s -w .` before committing
- Wrap errors with `%w` and provide context: `fmt.Errorf("opening %s: %w", path, err)`

## Orchestrator Rules

- One concern per package
- Public types live in their own files (no `models.go` catch-alls)

## Other sections are ignored

- This bullet will NOT be picked up by `wiggum reverse`
- Only `## Rules` / `## Code style` / `## Orchestrator Rules` are scanned
