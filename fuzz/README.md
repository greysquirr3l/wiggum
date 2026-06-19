# Fuzzing

Local fuzz harness for the TOML plan parser and the Tera template renderers.
The CI workflow at `.github/workflows/fuzz.yml` runs these targets weekly under
ClusterFuzzLite; the harness here exists so contributors can reproduce CI
crashes and run ad-hoc fuzzing on their own machines.

## Prerequisites

- **Nightly Rust**: pinned by `fuzz/rust-toolchain.toml` — picked up
  automatically when invoking cargo from this directory.
- **`cargo-fuzz`**: install once with
  `cargo install cargo-fuzz --locked`.

If you don't have nightly yet, `rustup toolchain install nightly --component rust-src`
will fetch it.

## Targets

| Target | What it exercises |
| --- | --- |
| `plan_parser` | `Plan::from_toml` and `Workspace::from_toml` (the two TOML entry points) |
| `template_render` | Every `generation::*::render` function reachable from a parsed plan |

## Common commands

`cargo-fuzz` is the external `cargo fuzz` subcommand; it walks up from the
current directory to find the enclosing fuzz crate, so the commands below
must be run from inside `fuzz/`.

```sh
cd fuzz

# List available targets
cargo fuzz list

# Build every target (sanitizer is picked up from $SANITIZER, default: address)
cargo fuzz build

# Fuzz a single target indefinitely (Ctrl-C to stop)
cargo fuzz run plan_parser

# Fuzz for a fixed duration
cargo fuzz run plan_parser -- -max_total_time=120

# Run a target under the address sanitizer explicitly
cargo fuzz run plan_parser -- -sanitizer=address

# Run under the coverage sanitizer (faster, no ASan, useful for longer runs)
cargo fuzz run template_render -- -sanitizer=coverage
```

## Reproducing a crash

When the CI fuzz job finds a crash it uploads a `fuzz-crashes-<sanitizer>`
artifact. Download and unpack it, then run:

```sh
cargo fuzz run <target> fuzz/artifacts/<target>/crash-<sha1>
```

A crashing input that fails the assertion will print the input bytes; capture
them into a regression test under `tests/` so the fix can be verified.

## Corpus

Fuzzing state (corpus, artefacts, dict) lives under `fuzz/`. It's ignored by
`.gitignore` via `/fuzz/target/` for build artefacts; the `fuzz/corpus/` and
`fuzz/artifacts/` directories are also local-only. Re-running `cargo fuzz` after
a fresh clone will start with an empty corpus and discover new coverage from
scratch.
