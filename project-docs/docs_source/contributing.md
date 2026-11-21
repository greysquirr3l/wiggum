# Contributing

Contributions to Wiggum are welcome.

## Building from source

```bash
git clone https://github.com/greysquirr3l/wiggum.git
cd wiggum
cargo build
```

## Running tests

```bash
cargo test --workspace
```

## Golden plan corpus

`tests/golden/*.toml` holds small plans spanning languages, strategies, evaluator
and gate settings, and capabilities. `tests/golden_test.rs` generates each plan
(all targets) into a temp dir and checks that the `check` score is at or above
the plan's floor, that every orchestrator variant and `CLAUDE.md` carries the
Stop and ask section, that every task has a command in its Exit Criteria (a
`Verify:` line, or the preflight command when the task declares none), and that
no generated file contains an unrendered `{{` or `{%`.

- **Add a plan:** drop a `*.toml` into `tests/golden/` with a leading
  `# golden-floor: N` comment (use the score `wiggum check` reports). There is no
  registry list. The plan must pass `wiggum generate` validation; set
  `require_evaluator = false` on purpose when it has no evaluator.
- **Bump a floor:** after improving a plan or the scorer, edit the number in that
  plan's `# golden-floor:` line to the new `wiggum check` score. Never lower a
  floor to silence a failure; fix the plan or the generator.

## Progress corpus

`tests/progress-corpus/*.md` holds synthetic PROGRESS.md files (shapes learned from
real runs, no real content) with hand-labelled truth in `labels.toml` and a
`wiggum retro` snapshot per file under `snapshots/`. `tests/progress_corpus_test.rs`
parses every file and checks task, status, phase and project-name counts, the
evaluator verdict sequences and retry counts against the labels, and has a leak
guard (no URLs, hashes other than 7-hex fake ones, home paths or emails).

- **Add a file:** drop a `*.md` into the corpus dir, add its labels entry and a
  snapshot. A file with no labels entry fails the test.
- **Known defects:** where today's parser or retro disagrees with the truth, the
  label stays true and the gap is recorded under `known_defects` / `today`. Fixing
  the code makes the test fail until you remove the entry.
- **Never** paste a real PROGRESS.md into the corpus, even redacted. See
  `tests/progress-corpus/README.md` for the schema and how to regenerate snapshots.

## Linting

```bash
cargo clippy --workspace -- -D warnings
cargo fmt -- --check
```

## Code style

- Rust edition 2024, MSRV 1.85
- Strict clippy: pedantic, nursery, cargo, and perf lints as warnings
- `unwrap()`, `expect()`, `panic!()`, and indexing with `[]` are denied — use `Result` and `.get()` instead
- Dual licensed under MIT and Apache-2.0

## Project structure

```
src/
├── adapters/    # CLI, filesystem, VCS, MCP server
├── domain/      # Plan model, DAG validation, language profiles, linting
├── generation/  # Template rendering, task/progress generation
├── error.rs     # Error types
├── ports.rs     # Port traits (hexagonal architecture)
├── lib.rs       # Library root
└── main.rs      # CLI entry point
```
