# Workspaces

A Wiggum **workspace** lets you orchestrate multiple `plan.toml` files as a unit. Use it when a project spans several independent components — a shared library, an API service, a background worker — that must be developed in a specific order but each need their own plan.

## `workspace.toml`

A `workspace.toml` file sits at the root of a multi-plan project and lists each component plan:

```toml
[workspace]
name = "my-platform"
description = "Multi-service platform workspace"

[[plans]]
name = "shared-lib"
path = "libs/shared/plan.toml"

[[plans]]
name = "api-service"
path = "services/api/plan.toml"
depends_on = ["shared-lib"]

[[plans]]
name = "worker"
path = "services/worker/plan.toml"
depends_on = ["shared-lib"]
```

### Fields

#### `[workspace]`

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Human-readable workspace name |
| `description` | No | Brief description of what the workspace contains |

#### `[[plans]]`

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Short identifier used in `depends_on` references (e.g. `"api-service"`) |
| `path` | Yes | Relative path to the plan TOML (relative to `workspace.toml`) |
| `depends_on` | No | Names of other plans this plan depends on. Plans in the dependency list must be completed before this plan begins. |

## Inter-plan dependencies

`depends_on` on a `[[plans]]` entry defines ordering at the workspace level, independent of task-level dependencies inside each plan. Wiggum validates that the workspace dependency graph is a valid DAG — circular workspace dependencies are rejected.

## Scaffold generation

Use the `wiggum_draft_plan` MCP tool or `wiggum init` to create individual `plan.toml` files for each component, then wire them together in `workspace.toml`.

Each component plan is generated and run independently. The workspace file provides the dependency ordering so the orchestrator knows which plans can proceed in parallel and which must wait.
