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
| `diff` | Compare two plan files |
| `resume` | Recover an interrupted orchestrator loop |
| `retro` | Generate improvement suggestions from PROGRESS.md |
| `split` | Split an oversized task into smaller units |
| `templates` | Manage reusable task templates |
| `version` | Show CLI version with embedded git SHA (`wiggum <version> (<sha\|unknown>)`) |
| `serve --mcp` | Start the MCP server |
| `report` | Generate a post-execution report |
| `watch` | Live progress monitoring |

## Generated artifacts

```text
project/
├── IMPLEMENTATION_PLAN.md
├── PROGRESS.md
├── AGENTS.md
├── features.json
└── tasks/
    ├── T01-{slug}.md
    ├── T02-{slug}.md
    └── ...
.vscode/
├── orchestrator.prompt.md
└── evaluator.prompt.md   # only when [evaluator] is configured
```

## Running the loop

After generating artifacts, open your AI coding tool in agent mode and load the generated `orchestrator.prompt.md` as the prompt. The orchestrator will:

1. Read `PROGRESS.md` to find the next incomplete task
2. Open the corresponding `tasks/T{NN}-{slug}.md` file
3. Spawn a subagent to implement the task
4. Run preflight checks (build, test, lint **+ security audit**) to verify the work
5. Independently verify — re-runs preflight before trusting the subagent's completion mark
6. Updates `features.json` with per-criterion pass/fail results
7. If an `[evaluator]` agent is configured, spawns it to score the task independently
8. Mark the task complete in `PROGRESS.md` and record learnings
9. Repeat until all tasks are done

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

Rust, Go, TypeScript, Python, Java, C#, Kotlin, Swift, Ruby, Elixir — each with idiomatic defaults for build, test, lint, and **security audit** commands.

## Security

Wiggum bakes security into every generated plan at three levels:

**1. Non-negotiable rules in every subagent prompt**
Six OWASP-derived rules are injected automatically into the `## Security` section of every task file and orchestrator prompt — covering secrets management, parameterised queries, HTTP security headers, rate limiting, file upload validation, and SSRF prevention. You don't have to add them; they're always there.

**2. Vulnerability audit in every preflight**
The language profile's audit command (`cargo audit`, `govulncheck`, `npm audit`, `pip-audit`, etc.) is appended to every task's preflight chain and exit criteria. Supply-chain CVEs are checked on every task completion, not just at the end. Override or disable per-plan with `preflight.audit`.

**3. Automatic security hardening task**
When your plan contains web-facing surface (detected from task slugs containing `http`, `api`, `server`, `webhook`, `upload`, `auth`, etc.), Wiggum auto-appends a `security-hardening` task as the final task in your plan. Its `must_haves` and `evaluation_criteria` map directly to the six OWASP categories with concrete, verifiable conditions. Suppress with `[security] skip_hardening_task = true` if you're handling security separately.

```toml
# Opt out of the auto-injected security task if desired
[security]
skip_hardening_task = true
```

## Integration Audits

AI-generated code often compiles successfully but has two common failure modes:

1. **Disconnected wiring** — modules, services, and handlers are created but never actually connected to the application (e.g., a service class exists but is never instantiated and used)
2. **Stub implementations** — placeholder code like `todo!()`, `unimplemented!()`, or `raise NotImplementedError` that compiles but crashes at runtime

Wiggum auto-injects two late-stage audit tasks when your plan has 3+ explicit (user-defined) tasks:

**Integration wiring audit** — verifies all components are properly connected:

- All public exports are actually imported and used somewhere
- All route handlers/controllers are registered with the router
- All service interfaces have implementations that are instantiated
- Middleware is mounted on the request pipeline

**Stub cleanup audit** — finds and replaces placeholder implementations:

- Searches for language-specific stub patterns (`todo!()`, `NotImplementedError`, etc.)
- Ensures all TODOs for completed tasks are resolved
- Verifies all code paths are reachable and functional

Each language profile includes specific patterns and hints tailored to its ecosystem.

```toml
# Opt out of the auto-injected integration audits if desired
[integration]
skip_wiring_audit = true   # Disable wiring audit
skip_stub_audit = true     # Disable stub cleanup audit
```

## Documentation

Full docs: [greysquirr3l.github.io/wiggum](https://greysquirr3l.github.io/wiggum)

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
