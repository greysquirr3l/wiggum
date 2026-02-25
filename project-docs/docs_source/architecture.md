# Architecture

Wiggum follows a hexagonal (ports and adapters) architecture.

## Layers

### Domain (`src/domain/`)

Pure business logic with no I/O dependencies:

- **Plan model** — The `Plan` struct parsed from TOML, with phases, tasks, and project metadata
- **DAG validation** — Topological sort and cycle detection on the task dependency graph
- **Language profiles** — Built-in profiles for 10 programming languages
- **Lint rules** — Plan quality checks (e.g., missing descriptions, unreachable tasks)

### Ports (`src/ports.rs`)

Trait definitions for I/O boundaries:

- `PlanReader` — Reading plan files from the filesystem

### Adapters (`src/adapters/`)

Concrete implementations of ports and external integrations:

- **CLI** — Clap-based command definitions
- **Filesystem** — File reading/writing
- **VCS** — Git integration for reports
- **MCP** — Model Context Protocol server (stdio transport)
- **Init** — Interactive plan creation
- **Bootstrap** — Project detection and plan generation

### Generation (`src/generation/`)

Template-based artifact rendering:

- **Task files** — Tera templates producing `T{NN}-{slug}.md` files
- **Progress tracker** — `PROGRESS.md` generation with parallel group annotations
- **Orchestrator prompt** — Agent-mode prompt rendering
- **Implementation plan** — Architecture overview generation
- **Token estimation** — Approximate token counts for generated content

## Data flow

```
Plan TOML → Parse → Validate DAG → Generate artifacts
                                      ├── PROGRESS.md
                                      ├── orchestrator.prompt.md
                                      ├── IMPLEMENTATION_PLAN.md
                                      ├── AGENTS.md
                                      └── tasks/T{NN}-{slug}.md
```
