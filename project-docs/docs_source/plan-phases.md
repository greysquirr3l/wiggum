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
| `kind` | No | Task archetype (default: `feature`) — see [Task kinds](#task-kinds) below |
| `depends_on` | Yes | Array of task slugs this task depends on (empty array `[]` for no dependencies) |
| `hints` | No | Implementation hints or code snippets |
| `test_hints` | No | Suggested tests the subagent should write |
| `must_haves` | No | Hard exit criteria the task must satisfy |
| `gate` | No | Precondition the orchestrator must verify before starting this task |
| `evaluation_criteria` | No | Verifiable criteria scored by the [evaluator](./plan-preflight.md#evaluator-configuration) agent |

## Task kinds

The `kind` field selects a task archetype that influences the generated task file template — the section headings, suggested exit criteria, and approach guidance are all tailored to the kind of work involved.

| Kind | Description |
|------|-------------|
| `feature` | New functionality: emphasises implementation and tests. **Default.** |
| `refactor` | Code quality change: emphasises behavioural equivalence before and after |
| `infrastructure` | CI, config, tooling, or IaC work |
| `research` | Exploratory spike: produces a document or recommendation, not code |
| `audit` | Security or quality audit: produces findings, not new behaviour |

```toml
[[phases.tasks]]
slug  = "api-routes"
title = "Define API route handlers"
kind  = "feature"          # default, can be omitted
goal  = "Implement REST endpoints for CRUD operations"
depends_on = ["domain-model"]

[[phases.tasks]]
slug  = "dependency-audit"
title = "Audit third-party dependencies"
kind  = "audit"
goal  = "Identify outdated or vulnerable packages and produce a remediation report"
depends_on = []
```

Audit-kind tasks are automatically excluded from the orphan-detection lint rule, since they intentionally produce findings rather than code artefacts.

## Task numbering

Tasks are numbered sequentially across all phases: T01, T02, T03, and so on. The ordering follows phase order first, then task order within each phase, but dependencies can cross phase boundaries.

## Dependency graph

Wiggum validates that task dependencies form a valid DAG (no cycles). Use `wiggum validate plan.toml` to check before generating.
