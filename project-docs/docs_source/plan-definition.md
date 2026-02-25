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

## Sections

- **[Project Configuration](./plan-project.md)** — Project metadata and language settings
- **[Phases and Tasks](./plan-phases.md)** — Organizing work into phases with dependency-ordered tasks
- **[Preflight and Orchestrator](./plan-preflight.md)** — Build/test/lint commands and orchestrator persona
