# CLI Reference

## `wiggum init`

Interactively create a new plan file.

```bash
wiggum init [--plan <path>]
```

| Option | Description |
|--------|-------------|
| `--plan`, `-p` | Path to write the generated plan TOML (default: `plan.toml`) |

## `wiggum generate`

Generate all artifacts from a plan file.

```bash
wiggum generate <plan> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `<plan>` | Path to the plan TOML file |
| `--output`, `-o` | Override the output directory (defaults to `project.path` from the plan) |
| `--force` | Overwrite existing files without prompting |
| `--dry-run` | Preview what would be generated without writing files |
| `--estimate-tokens` | Show estimated token counts for generated artifacts |
| `--skip-agents-md` | Skip `AGENTS.md` generation |

## `wiggum check`

Score the quality of a plan file before generating. Unlike `validate --lint`, which checks structural correctness, `check` scores the *substance* of a plan on five dimensions and produces concrete improvement suggestions.

```bash
wiggum check <plan> [--json]
```

| Option | Description |
|--------|-------------|
| `<plan>` | Path to the plan TOML file |
| `--json` | Output results as JSON instead of human-readable text |

The six scoring dimensions are:

| Dimension | What it measures |
|-----------|------------------|
| **Granularity** | Whether tasks are sized for a single agent session (not too broad or too narrow) |
| **Dependency health** | DAG fan-out, orphan detection, and over-coupling |
| **Coverage** | Balance of task kinds across the plan |
| **Richness** | Presence of hints, must-haves, evaluation criteria |
| **Token budget** | Estimated prompt size across all generated artifacts |
| **Harness complexity** | Whether the evaluator harness matches plan scale — penalises over-engineered harnesses on tiny plans and rewards high criteria coverage |

Each dimension scores 0–10. The overall score is a weighted composite. Plans scoring **≥ 7** are considered healthy; `wiggum check` exits with a non-zero status if the plan is below this threshold.

Run `check` before `generate` to catch low-quality plans early:

```bash
wiggum check plan.toml
wiggum check plan.toml --json
```

## `wiggum validate`

Validate a plan file without generating artifacts.

```bash
wiggum validate <plan> [--lint]
```

| Option | Description |
|--------|-------------|
| `<plan>` | Path to the plan TOML file |
| `--lint` | Run lint rules to check plan quality |

## `wiggum add-task`

Add a task to an existing plan file interactively.

```bash
wiggum add-task <plan>
```

## `wiggum bootstrap`

Bootstrap a plan from an existing project directory. Detects language, build system, and project structure.

```bash
wiggum bootstrap [path] [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `[path]` | Path to the project directory (default: `.`) |
| `--output`, `-o` | Path to write the generated plan TOML (default: `<path>/plan.toml`) |
| `--force` | Overwrite existing plan file without prompting |

## `wiggum serve`

Start the MCP server for agent integration.

```bash
wiggum serve --mcp
```

## `wiggum version`

Show the CLI version and embedded git SHA.

```bash
wiggum version
```

Output format:

```text
wiggum <version> (<sha|unknown>)
```

## `wiggum retro`

Analyse a completed `PROGRESS.md` and emit retrospective improvement suggestions.

```bash
wiggum retro [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--progress` | Path to `PROGRESS.md` (default: `PROGRESS.md`) |
| `--plan` | Path to the plan TOML (used to correlate task slugs) |
| `--save` | Save the retrospective as a reusable pattern in `~/.wiggum/patterns/` |

## `wiggum replan`

Re-generate a single task file after a failure. Reads the current plan and any failure
evidence recorded in `PROGRESS.md`, then re-renders the task's `.md` file with augmented hints.

```bash
wiggum replan <plan> --task <slug> [--dry-run]
```

| Option | Description |
|--------|-------------|
| `<plan>` | Path to the plan TOML file |
| `--task`, `-t` | Slug of the task to re-generate (required) |
| `--dry-run` | Print the new content to stdout instead of writing to disk |

Failure evidence is extracted from lines in `PROGRESS.md` that are tagged `[~]` or contain
"required fix". Extracted lines are prepended as `[Previous failure]` hints in the regenerated
task file so the subagent has full context for the retry.

## `wiggum patterns`

Manage the local pattern store (`~/.wiggum/patterns/`). Patterns are reusable TOML files derived
from retrospectives that can be applied as hint augmentations to future plans.

```bash
wiggum patterns <action> [OPTIONS]
```

### `list`

List all saved patterns.

```bash
wiggum patterns list
```

### `save`

Save a pattern from a PROGRESS.md file (or an existing retro output).

```bash
wiggum patterns save --from <progress-path> --plan <plan-path>
```

| Option | Description |
|--------|-------------|
| `--from` | Path to the source `PROGRESS.md` |
| `--plan` | Path to the plan TOML (provides language metadata) |

### `apply`

Apply matching patterns as additional hints to a plan file. Patterns are matched by language.

```bash
wiggum patterns apply --plan <plan-path>
```

| Option | Description |
|--------|-------------|
| `--plan` | Path to the plan TOML to augment |

## `wiggum report`

Generate a post-execution report from `PROGRESS.md`.

```bash
wiggum report [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--progress` | Path to `PROGRESS.md` (default: `PROGRESS.md`) |
| `--project-dir` | Project directory for git timeline (optional) |

## `wiggum watch`

Watch `PROGRESS.md` for live progress updates.

```bash
wiggum watch [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--progress` | Path to `PROGRESS.md` (default: `PROGRESS.md`) |
| `--poll-ms` | Poll interval in milliseconds (default: `1000`) |
| `--stall-secs` | Seconds before an in-progress task triggers a stall warning (default: `1800`; set to `0` to disable) |

When `--stall-secs` is non-zero, the watch display emits a `⚠ HEALTH` warning next to any task that has remained `[~]` in-progress for longer than the threshold. The warning continues to refresh on every poll cycle until the task transitions to a terminal state.
