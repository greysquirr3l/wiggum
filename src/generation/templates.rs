use std::path::Path;
use std::sync::LazyLock;

use tera::Tera;

use crate::error::Result;

#[allow(clippy::panic)]
static TEMPLATES: LazyLock<Tera> = LazyLock::new(|| {
    let mut tera = Tera::default();

    // Register all embedded templates
    let templates = [
        ("progress.md", PROGRESS_TEMPLATE),
        ("orchestrator.md", ORCHESTRATOR_TEMPLATE),
        ("task.md", TASK_TEMPLATE),
        ("plan_doc.md", PLAN_DOC_TEMPLATE),
        ("agents.md", AGENTS_MD_TEMPLATE),
    ];

    for (name, content) in templates {
        tera.add_raw_template(name, content)
            .unwrap_or_else(|e| panic!("Failed to parse template {name}: {e}"));
    }

    tera
});

/// Get the default embedded Tera instance.
pub fn get_tera() -> &'static Tera {
    &TEMPLATES
}

/// Build a Tera instance with user template overrides from `.wiggum/templates/`.
/// User templates take precedence over embedded defaults.
pub fn get_tera_with_overrides(project_path: &Path) -> Result<Tera> {
    let override_dir = project_path.join(".wiggum/templates");

    if !override_dir.is_dir() {
        // No overrides — clone from the static defaults
        return Ok(TEMPLATES.clone());
    }

    // Start with embedded defaults
    let mut tera = TEMPLATES.clone();

    // Layer user templates on top (overwriting any matching names)
    let template_names = [
        "progress.md",
        "orchestrator.md",
        "task.md",
        "plan_doc.md",
        "agents.md",
    ];
    for name in template_names {
        let user_file = override_dir.join(name);
        if user_file.is_file() {
            let content = std::fs::read_to_string(&user_file).map_err(|e| {
                crate::error::WiggumError::Template(format!(
                    "failed to read user template {}: {e}",
                    user_file.display()
                ))
            })?;
            tera.add_raw_template(name, &content).map_err(|e| {
                crate::error::WiggumError::Template(format!(
                    "failed to parse user template {}: {e}",
                    user_file.display()
                ))
            })?;
        }
    }

    Ok(tera)
}

// ─── Embedded templates ──────────────────────────────────────────────

const PROGRESS_TEMPLATE: &str = r"# {{ project_name }} — Implementation Progress

> Orchestrator reads this file at the start of each loop iteration.
> Subagents update this file after completing a task.

## Status Legend

- `[ ]` — Not started
- `[~]` — In progress (claimed by a subagent)
- `[x]` — Completed
- `[!]` — Blocked / needs human input

---
{% for phase in phases %}

## Phase {{ phase.order }} — {{ phase.name }}
{% if phase.depends_on_desc %}
> Depends on: {{ phase.depends_on_desc }}
{% endif %}

| Task | Status | Notes |
|---|---|---|
{% for task in phase.tasks %}| T{{ task.number_padded }} — {{ task.title }} | `[ ]` | |
{% endfor %}
---
{% endfor %}

## Accumulated Learnings

> Subagents append discoveries here after each task.
> The orchestrator reads this section at the start of every iteration
> to avoid repeating past mistakes.

_No learnings yet._
";

const ORCHESTRATOR_TEMPLATE: &str = r#"---
agent: agent
description: Orchestrator for the Ralph Wiggum loop — drives subagents to implement all {{ project_name }} tasks
---

<PLAN>{{ project_path }}/IMPLEMENTATION_PLAN.md</PLAN>

<TASKS>{{ project_path }}/tasks</TASKS>

<PROGRESS>{{ project_path }}/PROGRESS.md</PROGRESS>

<ORCHESTRATOR_INSTRUCTIONS>

You are an orchestration agent. Your sole job is to drive subagents to implement the {{ project_name }} project until all tasks in PROGRESS.md are marked `[x]`.

**You do NOT implement code yourself. You only spawn subagents and verify their output.**

## Setup

1. Read PROGRESS.md to understand current state.
2. If PROGRESS.md does not exist, fail immediately — it should have been created.

## Implementation loop

Repeat until all tasks (T01–T{{ task_count_padded }}) in PROGRESS.md are `[x]`:

1. Read PROGRESS.md.
2. Find the next task that is `[ ]` and whose dependencies are all `[x]`.
3. **Check for a gate** — if the task file begins with a `⛔ GATE` banner, emit it verbatim
   and **stop**. The human must confirm (e.g. by restarting the orchestrator) before you proceed.
4. Mark it `[~]` in PROGRESS.md.
5. **Read the Accumulated Learnings section** — apply any relevant insights.
6. Start a subagent with the SUBAGENT_PROMPT below.
7. Wait for the subagent to complete.
8. Read PROGRESS.md again.
9. Verify the task is now `[x]`. If it is not, mark it `[!]` and output a warning, then continue to the next available task.
10. Repeat.

When all tasks are `[x]`, output:

```
✅ All {{ project_name }} implementation tasks complete.
```

## You MUST have access to the `#tool:agent/runSubagent` tool

If this tool is not available, fail immediately with:

```
⛔ runSubagent tool is not available. Switch to Agent mode in VS Code Copilot and retry.
```

</ORCHESTRATOR_INSTRUCTIONS>

<SUBAGENT_PROMPT>

{{ persona }}

## Your context

- Project plan: read `{{ project_path }}/IMPLEMENTATION_PLAN.md`
- Progress tracker: `{{ project_path }}/PROGRESS.md`
- Task files: `{{ project_path }}/tasks/`
{% if strategy == "tdd" %}
## Strategy: Test-Driven Development (TDD)

Follow the Red-Green-Refactor cycle strictly:

1. Read PROGRESS.md.
2. **Read the Accumulated Learnings section** — apply relevant insights from prior tasks.
3. Find the highest-priority task that is `[ ]` and whose dependencies are all `[x]`.
4. Mark it `[~]` in PROGRESS.md immediately.
5. Read the corresponding task file in `tasks/`.
6. **RED** — Write failing tests first based on the test hints. Run them to confirm they fail.
7. **GREEN** — Write the minimum code to make all tests pass. Do not add extra functionality.
8. **REFACTOR** — Clean up the code while keeping all tests green. Remove duplication, improve naming.
9. Run the preflight check from the task file:
   ```bash
   {{ preflight_build }} && {{ preflight_test }} && {{ preflight_lint }}
   ```
   Fix all errors and warnings until preflight passes.
10. Verify all exit criteria from the task file are met.
11. Update PROGRESS.md: change `[~]` to `[x]` for this task.
12. **Append any learnings** to the Accumulated Learnings section in PROGRESS.md.
    Format: `- T{NN}: {what you learned}`
13. Commit with a conventional commit message focused on user impact (not file counts or line numbers).
14. Stop.
{% elif strategy == "gsd" %}
## Strategy: Get Stuff Done (GSD)

Focus on must-haves. No gold-plating.

1. Read PROGRESS.md.
2. **Read the Accumulated Learnings section** — apply relevant insights from prior tasks.
3. Find the highest-priority task that is `[ ]` and whose dependencies are all `[x]`.
4. Mark it `[~]` in PROGRESS.md immediately.
5. Read the corresponding task file in `tasks/`.
6. **Identify must-haves** — list the concrete deliverables from the goal and must_haves.
7. **Implement each must-have** — work through them one by one. No extras.
8. **Verify all must-haves** — confirm every deliverable is present and working.
9. Run the preflight check from the task file:
   ```bash
   {{ preflight_build }} && {{ preflight_test }} && {{ preflight_lint }}
   ```
   Fix all errors and warnings until preflight passes.
10. Verify all exit criteria from the task file are met.
11. Update PROGRESS.md: change `[~]` to `[x]` for this task.
12. **Append any learnings** to the Accumulated Learnings section in PROGRESS.md.
    Format: `- T{NN}: {what you learned}`
13. Commit with a conventional commit message focused on user impact (not file counts or line numbers).
14. Stop.
{% else %}
## Your job

1. Read PROGRESS.md.
2. **Read the Accumulated Learnings section** — apply relevant insights from prior tasks.
3. Find the highest-priority task that is `[ ]` and whose dependencies are all `[x]`.
4. Mark it `[~]` in PROGRESS.md immediately.
4. Read the corresponding task file in `tasks/`.
5. Implement the task completely — create all files, write all code, add all tests specified.
6. Run the preflight check from the task file:
   ```bash
   {{ preflight_build }} && {{ preflight_test }} && {{ preflight_lint }}
   ```
   Fix all errors and warnings until preflight passes.
7. Verify all exit criteria from the task file are met.
8. Update PROGRESS.md: change `[~]` to `[x]` for this task.
9. **Append any learnings** to the Accumulated Learnings section in PROGRESS.md.
   Format: `- T{NN}: {what you learned}`
10. Commit with a conventional commit message focused on user impact (not file counts or line numbers).
11. Stop.
{% endif %}

## Rules

- Implement THIS TASK ONLY. Do not touch code from other tasks.
{% for rule in rules %}- {{ rule }}
{% endfor %}
{% if architecture == "hexagonal" %}
## Architecture: Hexagonal

- Domain layer must have zero I/O dependencies
- All external interactions go through port traits
- Adapters implement port traits and live in `adapters/`
- New capabilities require a new port trait before an adapter
{% elif architecture == "layered" %}
## Architecture: Layered

- Respect layer boundaries: presentation → application → domain → infrastructure
- Domain entities must not reference application or infrastructure types
- Use DTOs at layer boundaries — don't leak domain types to presentation
{% elif architecture == "modular" %}
## Architecture: Modular

- Keep changes within the module being modified
- Cross-module communication must go through public module interfaces
- Shared types belong in the shared/common module
{% endif %}
</SUBAGENT_PROMPT>
"#;

const TASK_TEMPLATE: &str = r#"# T{{ number_padded }} — {{ title }}
{% if gate %}
> ⛔ **GATE — Human confirmation required before starting this task.**
> {{ gate }}
{% endif %}
> **Depends on**: {{ depends_on_desc }}.

## Goal

{{ goal }}

## Project Context

- Project: `{{ project_name }}` — {{ project_description }}
- Language: {{ language }}
{% if architecture %}- Architecture: {{ architecture }}
{% endif %}
{% if architecture == "hexagonal" %}### Architecture: Hexagonal (Ports & Adapters)

- **Domain layer** (`domain/`): Pure business logic, no I/O dependencies
- **Ports** (`ports/`): Trait boundaries that define capabilities the domain needs
- **Adapters** (`adapters/`): Implementations of ports (HTTP, DB, filesystem, etc.)
- Keep domain types free of framework-specific derives (no `#[sqlx::FromRow]` etc.)
- Depend inward: adapters → ports ← domain
{% elif architecture == "layered" %}### Architecture: Layered

- **Presentation**: API handlers / CLI / UI
- **Application**: Use cases, orchestration, DTOs
- **Domain**: Entities, value objects, business rules
- **Infrastructure**: Database, external services, config
- Each layer depends only on the layer below it
{% elif architecture == "modular" %}### Architecture: Modular

- Each module is self-contained with its own models, handlers, and storage
- Modules communicate through well-defined public interfaces
- Shared code goes in a `common/` or `shared/` module
- Prefer module-level encapsulation over cross-cutting layers
{% elif architecture == "flat" %}### Architecture: Flat

- Simple project structure without deep nesting
- All source files at the top level of `src/`
- Appropriate for small tools, CLIs, and single-purpose utilities
{% endif %}
{% if strategy == "tdd" %}
## Strategy: TDD (Red-Green-Refactor)

### 1. RED — Write failing tests first

{% if test_hints %}{% for hint in test_hints %}- {{ hint }}
{% endfor %}{% else %}<!-- TODO: Add test requirements here.
   Include: what to test, edge cases, test file locations. -->
{% endif %}

### 2. GREEN — Implement to pass

{% if hints %}{% for hint in hints %}- {{ hint }}
{% endfor %}{% else %}<!-- TODO: Add implementation guidance here.
   Include: file paths, type signatures, key design decisions. -->
{% endif %}

### 3. REFACTOR — Clean up while green

- Remove duplication
- Improve naming and structure
- Keep all tests passing
{% elif strategy == "gsd" %}
## Strategy: GSD (Get Stuff Done)

### Must-Haves

{% if must_haves %}{% for mh in must_haves %}- [ ] {{ mh }}
{% endfor %}{% else %}- [ ] Implementation matches the goal
{% endif %}

### Implementation

{% if hints %}{% for hint in hints %}- {{ hint }}
{% endfor %}{% else %}<!-- TODO: Add implementation guidance here. -->
{% endif %}

### Tests

{% if test_hints %}{% for hint in test_hints %}- {{ hint }}
{% endfor %}{% else %}<!-- TODO: Add test requirements here. -->
{% endif %}
{% else %}
## Implementation

{% if hints %}{% for hint in hints %}- {{ hint }}
{% endfor %}{% else %}<!-- TODO: Add implementation guidance here before running the orchestrator.
   Include: file paths, type signatures, key design decisions, code snippets where helpful. -->
{% endif %}

## Tests

{% if test_hints %}{% for hint in test_hints %}- {{ hint }}
{% endfor %}{% else %}<!-- TODO: Add test requirements here.
   Include: what to test, edge cases, test file locations. -->
{% endif %}
{% endif %}

## Housekeeping: TODO / FIXME Sweep

Before running preflight, scan all files you created or modified in this task for
`TODO`, `FIXME`, `HACK`, `XXX`, and similar markers.

- **Resolve** any that fall within the scope of this task's goal.
- **Leave in place** any that reference work belonging to a later task or phase — but ensure they include a task reference (e.g. `// TODO(T07): wire up auth adapter`).
- **Remove** any placeholder markers that are no longer relevant after your implementation.

If none are found, move on.

## Preflight

```bash
{{ preflight_build }} && {{ preflight_test }} && {{ preflight_lint }}
```

## Exit Criteria

- [ ] {{ build_success_phrase }}
- [ ] All tests pass
- [ ] Linter passes with no warnings
- [ ] Implementation matches the goal described above
- [ ] No unresolved TODO/FIXME/HACK markers that belong to this task's scope

## After Completion

Update PROGRESS.md row for T{{ number_padded }} to `[x]`.
Commit: `{{ commit_message }}`
"#;

const PLAN_DOC_TEMPLATE: &str = r"# {{ project_name }} — Implementation Plan

## Overview

{{ project_description }}

{% if architecture %}**Architecture**: {{ architecture }}
{% endif %}**Language**: {{ language }}

---

## Phases
{% for phase in phases %}

### Phase {{ phase.order }} — {{ phase.name }}

{% for task in phase.tasks %}{{ loop.index }}. **T{{ task.number_padded }} — {{ task.title }}**
   {{ task.goal }}
{% if task.depends_on_list %}   _Depends on: {{ task.depends_on_list }}_
{% endif %}{% endfor %}{% endfor %}

---

## Preflight Commands

```bash
{{ preflight_build }}
{{ preflight_test }}
{{ preflight_lint }}
```

---

_Generated by [wiggum](https://github.com/greysquirr3l/wiggum)._
";

const AGENTS_MD_TEMPLATE: &str = r#"# AGENTS.md

## Project

{{ project_name }} — {{ project_description }}

## Setup commands

- Build: `{{ preflight_build }}`
- Test: `{{ preflight_test }}`
- Lint: `{{ preflight_lint }}`
{% if architecture == "hexagonal" %}
## Architecture: Hexagonal

- Domain layer must have zero I/O dependencies
- All external interactions go through port traits
- Adapters implement port traits and live in `adapters/`
- New capabilities require a new port trait before an adapter
- Depend inward: adapters → ports ← domain
{% elif architecture == "layered" %}
## Architecture: Layered

- Respect layer boundaries: presentation → application → domain → infrastructure
- Domain entities must not reference application or infrastructure types
- Use DTOs at layer boundaries — don't leak domain types to presentation
- Each layer depends only on the layer below it
{% elif architecture == "modular" %}
## Architecture: Modular

- Each module is self-contained with its own models, handlers, and storage
- Modules communicate through well-defined public interfaces
- Shared code goes in a `common/` or `shared/` module
- Prefer module-level encapsulation over cross-cutting layers
{% elif architecture == "flat" %}
## Architecture: Flat

- Simple project structure without deep nesting
- All source files at the top level of `src/`
{% endif %}
## Code style

- Language: {{ language }}
{% if strategy == "tdd" %}- Strategy: TDD — write a failing test before any implementation code
{% elif strategy == "gsd" %}- Strategy: GSD — focus on must-haves, no gold-plating
{% endif %}
{% if rules | length > 0 %}
## Rules
{% for rule in rules %}
- {{ rule }}
{% endfor %}{% endif %}
## Testing instructions

- Run `{{ preflight_test }}` before committing
- Every new public function needs at least one test
- Fix all test failures before marking a task complete

## Commit conventions

- Use conventional commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`
- Focus commit messages on user impact, not file counts or line numbers

---

_Generated by [wiggum](https://github.com/greysquirr3l/wiggum)._
"#;
