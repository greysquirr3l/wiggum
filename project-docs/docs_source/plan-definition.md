# Plan Definition

Wiggum plans are defined in TOML files. A plan describes your project metadata, phases, tasks, preflight commands, and orchestrator configuration.

## Minimal example

```toml
[project]
name = "my-app"
description = "A web API for widget management"
language = "rust"
path = "/path/to/project"

[[phases]]
name = "Foundation"
order = 1

[[phases.tasks]]
slug = "project-setup"
title = "Initialize project structure"
goal = "Set up the repo with build tooling and CI"
depends_on = []

[[phases.tasks]]
slug = "domain-model"
title = "Define domain entities"
goal = "Create the core domain types and traits"
depends_on = ["project-setup"]
```

## Per-task evaluation criteria

You can attach verifiable exit criteria to any task. These are embedded in the generated task file and registered in `features.json` for the evaluator agent to score:

```toml
[[phases.tasks]]
slug = "domain-model"
title = "Define domain entities"
goal = "Create the core domain types and traits"
depends_on = ["project-setup"]
evaluation_criteria = [
    "All domain types implement Serialize/Deserialize",
    "No business logic leaks into the infrastructure layer",
]
```

Four default criteria are added to every task automatically: build succeeds, all tests pass, linter clean, and implementation matches goal.

## Sections

- **[Project Configuration](./plan-project.md)** — Project metadata and language settings
- **[Phases and Tasks](./plan-phases.md)** — Organizing work into phases with dependency-ordered tasks
- **[Preflight and Orchestrator](./plan-preflight.md)** — Build/test/lint commands, orchestrator persona, and evaluator configuration
