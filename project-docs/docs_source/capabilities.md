# Capabilities

A **capability** is a persistent behavioural contract the system must satisfy,
independent of any particular phase or task. Capabilities cut across the
implementation plan: they describe _what_ the system does, while phases
describe _when_ that work happens.

Capabilities are useful when:

- The same contract is satisfied by tasks that live in different phases
  (e.g. validation lives in Phase 1 _and_ Phase 3 — one capability, two
  implementers).
- You want a stable artefact that survives regenerations: the contract
  for "what POST /webhook must do" should not change just because T03 was
  renumbered to T07.
- The evaluator needs a structured acceptance contract more rigorous
  than a flat bullet — `WHEN ... THEN ...` scenarios are machine-readable
  in a way prose is not.

## Defining a capability

Add a `[[capabilities]]` block to `plan.toml`:

```toml
[[capabilities]]
name        = "webhook-reception"
title       = "Inbound Webhook Reception"
description = "Accepts and validates inbound webhook deliveries before fanning out to subscribers."

[[capabilities.requirements]]
value = "MUST validate the HMAC-SHA256 signature on every inbound POST before persisting the event."

[[capabilities.requirements]]
value = "MUST return 202 Accepted within 200ms p95 for a valid request."

[[capabilities.scenarios]]
name = "valid-signature"
when = "POST /webhook receives a request with a valid HMAC-SHA256 signature"
then = "the server returns 202 Accepted and persists the event via the EventStore port"

[[capabilities.scenarios]]
name = "invalid-signature"
when = "POST /webhook receives a request with a tampered payload or wrong secret"
then = "the server returns 400 Bad Request and never persists the event"
```

### Fields

| Field          | Required | Notes                                                                                                                                               |
| -------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `name`         | yes      | Filename-safe slug. Used for `capabilities/<name>.md` and as the cross-reference target from `TaskDef::implements`. Must be unique within the plan. |
| `title`        | yes      | Display title rendered as the file's `# <title>` heading.                                                                                           |
| `description`  | no       | Free-form prose; rendered under `## Description`.                                                                                                   |
| `requirements` | no       | List of MUST / SHALL bullets. Rendered as a flat bullet list under `## Requirements`. Lint warns when empty.                                        |
| `scenarios`    | no       | List of `WHEN/THEN` clauses. Rendered as `### Scenario: <name>` blocks. Lint warns when empty.                                                      |

### Scenario fields

| Field  | Required | Notes                                                                                                                      |
| ------ | -------- | -------------------------------------------------------------------------------------------------------------------------- |
| `name` | yes      | Slug identifying the scenario within its capability.                                                                       |
| `when` | yes      | Precondition / trigger. Free-form prose.                                                                                   |
| `then` | yes      | Observable outcome. Free-form prose. **Must be verifiable** — a THEN clause that cannot be checked is a lint anti-pattern. |

## Referencing a capability from a task

Add `implements = ["<name>"]` to the task. Multiple references are allowed:

```toml
[[phases.tasks]]
slug      = "webhook-router"
title     = "HMAC-validated webhook router"
goal      = "Stand up the router that receives, validates, and persists inbound webhooks."
implements = ["webhook-reception"]
```

When a task declares `implements`, the rendered task file gains a new
section between Goal and Project Context:

```markdown
## Implements

### Capability: [webhook-reception](../capabilities/webhook-reception.md) — Inbound Webhook Reception

- MUST validate the HMAC-SHA256 signature on every inbound POST before persisting the event.
- MUST return 202 Accepted within 200ms p95 for a valid request.

**Scenarios this task must satisfy:**

- **valid-signature** — WHEN `POST /webhook receives a request with a valid HMAC-SHA256 signature` THEN `the server returns 202 Accepted and persists the event via the EventStore port`
- **invalid-signature** — WHEN `POST /webhook receives a request with a tampered payload or wrong secret` THEN `the server returns 400 Bad Request and never persists the event`
```

This means the subagent sees the acceptance contract inline — no extra
file reads required.

## Generated artifacts

When the plan has one or more `[[capabilities]]`, `wiggum generate` emits:

| File                                     | Content                                                                                                     |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `capabilities/<name>.md`                 | One per capability. Self-contained — title, description, requirements, scenarios.                           |
| `IMPLEMENTATION_PLAN.md`                 | New `## Capabilities` section that links to every capability file.                                          |
| `tasks/TNN-<slug>.md`                    | `## Implements` section (only when the task references ≥1 capability) inlining requirements and scenarios.  |
| `ORCHESTRATOR.md` / orchestrator prompts | New Setup step telling the orchestrator to read every file in `capabilities/` before dispatching subagents. |

If the plan has zero capabilities, **no `capabilities/` directory is
created** — this is the default and keeps generated trees minimal for
plans that don't need contracts.

## Validation

`wiggum validate` enforces structural correctness:

- **Unknown reference** (error): `T03 references undefined capability "foo"`
  — every name in `implements = [...]` must resolve to a `[[capabilities]]`
  entry.
- **Duplicate name** (error): `duplicate capability name(s): foo` — every
  capability needs a unique `name`.

`wiggum validate --lint` adds softer checks:

| Rule                         | Severity | Trigger                                        |
| ---------------------------- | -------- | ---------------------------------------------- |
| `capability-unused`          | warning  | Capability declared but no task references it. |
| `capability-no-scenarios`    | warning  | Capability has no acceptance contract.         |
| `capability-no-requirements` | info     | Capability has no prose requirements.          |

## Cleaning up

`wiggum clean` removes every `capabilities/<name>.md` file that
corresponds to a declared capability. The `capabilities/` directory
itself is removed when empty afterwards — same safety rule as
`.vscode/` and `.opencode/`: hand-written files alongside wiggum's
output are preserved.

## Relationship to phases

Capabilities and phases are independent axes:

- A capability is satisfied by **one or more** tasks across one or more
  phases.
- A phase contains **one or more** tasks that may or may not implement
  any capability.
- A task without `implements = [...]` is fine — most tasks aren't
  contract-bearing.

If a capability is satisfied by tasks scattered across phases (e.g.
"audit logging" might span Phase 4 logging, Phase 7 compliance review),
that's normal — the capability file is the persistent contract and the
task files are the per-phase implementers.
