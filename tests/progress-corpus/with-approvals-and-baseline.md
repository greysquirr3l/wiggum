# heron-deploy — Implementation Progress

> Orchestrator reads this file at the start of each loop iteration.
> Subagents update this file after completing a task.

## Status Legend

- `[ ]` — Not started
- `[~]` — In progress (claimed by a subagent)
- `[x]` — Completed
- `[!]` — Blocked / needs human input

---

## Phase 1 — Plan engine

| Task | Status | Notes |
|---|---|---|
| T01 — Resource graph model | `[x]` | Cycle detection in place. |
| T02 — Diff engine | `[x]` | Stable ordering of changes. |
| T03 — Dry-run renderer | `[~]` | Output format under review. |

---

## Phase 2 — Apply

> Starts after: Phase 1 is complete

| Task | Status | Notes |
|---|---|---|
| T04 — Apply with rollback journal | `[ ]` |  |
| T05 — Staging rollout rehearsal (gate) | `[ ]` |  |

---

## Known baseline failures

> The orchestrator records preflight failures that exist before the first task.
> Tasks may remove entries from this list but must never add to it.

| Check | Failing since | Note |
|---|---|---|
| T01 — formatting check on the vendored schema | before first task | Pre-existing; not ours. |
| doc build warning about a missing intro page | before first task | Pre-existing. |

## Approvals

> Record every human approval here before acting on it. The Approvals section of the
> generated orchestrator or CLAUDE.md prompt says what an approval covers.

| Scope | Action | Approved by | Expires |
|---|---|---|---|
| T02 — staging sandbox | apply the schema migration | Rosalind | 2027-04-30 |
| T05 — staging rollout | run the rehearsal against staging only | Rosalind | 2027-04-30 |

## Accumulated Learnings

> Subagents append discoveries here after each task.
> The orchestrator reads this section at the start of every iteration
> to avoid repeating past mistakes.

_No learnings yet._

## Codebase State

> Subagents update this section after completing each task.
> Describe what now exists, what is wired up, and what key decisions were made.
> A fresh agent should be able to orient from this section alone.

Plan engine crate only; apply is not started.
