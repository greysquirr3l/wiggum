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
