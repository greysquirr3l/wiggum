# Phases and Tasks

Work in a Wiggum plan is organized into **phases**, each containing one or more **tasks**. Tasks are dependency-ordered across the entire plan, forming a directed acyclic graph (DAG).

## Phases

Phases are logical groupings. They appear in `PROGRESS.md` as section headers but don't affect task ordering — dependencies do.

```toml
[[phases]]
name = "Foundation"
order = 1

[[phases]]
name = "Core Features"
order = 2
```

## Tasks

Tasks are defined within their parent phase. Each task becomes a `T{NN}-{slug}.md` file.

```toml
[[phases.tasks]]
slug = "api-routes"
title = "Define API route handlers"
goal = "Implement REST endpoints for CRUD operations"
depends_on = ["domain-model", "database-adapter"]
```

### Task fields

| Field | Required | Description |
|-------|----------|-------------|
| `slug` | Yes | URL-safe identifier, used in filenames and dependency references |
| `title` | Yes | Human-readable task title |
| `goal` | Yes | What this task should accomplish |
| `depends_on` | Yes | Array of task slugs this task depends on (empty array `[]` for no dependencies) |
| `context` | No | Additional context for the subagent |
| `hints` | No | Implementation hints or code snippets |
| `tests` | No | Test requirements |

## Task numbering

Tasks are numbered sequentially across all phases: T01, T02, T03, and so on. The ordering follows phase order first, then task order within each phase, but dependencies can cross phase boundaries.

## Dependency graph

Wiggum validates that task dependencies form a valid DAG (no cycles). Use `wiggum validate plan.toml` to check before generating.
