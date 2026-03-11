# Wiggum

AI orchestration scaffold generator for the Ralph Wiggum loop.

Wiggum generates structured task files, progress trackers, and orchestrator prompts from a TOML plan definition — enabling autonomous AI coding loops where an orchestrator agent drives subagents through dependency-ordered tasks until a project is fully implemented.

## Install

```bash
cargo install wiggum
```

## Quick start

```bash
# Create a plan interactively
wiggum init

# Or bootstrap from an existing project
wiggum bootstrap /path/to/project

# Validate the plan
wiggum validate plan.toml --lint

# Preview output
wiggum generate plan.toml --dry-run

# Generate artifacts
wiggum generate plan.toml
```

## Commands

| Command | Description |
|---------|-------------|
| `init` | Interactively create a new plan |
| `generate` | Generate task files, progress tracker, and orchestrator prompt |
| `validate` | Validate plan structure and dependency graph |
| `add-task` | Add a task to an existing plan |
| `bootstrap` | Generate a plan from an existing project |
| `serve --mcp` | Start the MCP server |
| `report` | Generate a post-execution report |
| `watch` | Live progress monitoring |

## Generated artifacts

```text
project/
├── IMPLEMENTATION_PLAN.md
├── PROGRESS.md
├── AGENTS.md
├── orchestrator.prompt.md
└── tasks/
    ├── T01-{slug}.md
    ├── T02-{slug}.md
    └── ...
```

## Running the loop

After generating artifacts, open your AI coding tool in agent mode and load the generated `orchestrator.prompt.md` as the prompt. The orchestrator will:

1. Read `PROGRESS.md` to find the next incomplete task
2. Open the corresponding `tasks/T{NN}-{slug}.md` file
3. Spawn a subagent to implement the task
4. Run preflight checks (build, test, lint) to verify the work
5. Mark the task complete in `PROGRESS.md` and record learnings
6. Repeat until all tasks are done

In VS Code with Copilot, copy `orchestrator.prompt.md` into your project as a prompt file:

```bash
cp orchestrator.prompt.md .github/orchestrator.prompt.md
```

Then start Copilot in agent mode and send:

> Read `.github/orchestrator.prompt.md` and follow its instructions. Begin by reading `PROGRESS.md` to identify the next incomplete task, then execute it. After each task passes preflight, update `PROGRESS.md` and continue to the next task.

Monitor progress in a separate terminal with:

```bash
wiggum watch
```

## Example plan

See [`reference/example-plan.toml`](reference/example-plan.toml) for a fully annotated plan covering all supported fields — project metadata, preflight commands, orchestrator persona and rules, multiple phases with dependency wiring, and per-task hints, test hints, must-haves, and gates.

### Gates (human-in-the-loop stops)

Add a `gate` to any task to require human confirmation before the orchestrator proceeds:

```toml
[[phases.tasks]]
slug  = "deploy"
gate  = "Confirm staging tests passed before the orchestrator runs this task."
# ... rest of task
```

The generated task file opens with a `⛔ GATE` banner, and the orchestrator prompt instructs the loop to stop and wait for confirmation before marking the task in-progress.

## Language support

Rust, Go, TypeScript, Python, Java, C#, Kotlin, Swift, Ruby, Elixir — each with idiomatic defaults for build, test, and lint commands.

## Documentation

Full docs: [greysquirr3l.github.io/wiggum](https://greysquirr3l.github.io/wiggum)

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
