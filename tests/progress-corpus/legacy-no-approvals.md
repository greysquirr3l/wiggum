# ember-ledger — Implementation Progress

> Orchestrator reads this file at the start of each loop iteration.
> Subagents update this file after completing a task.

## Status Legend

- `[ ]` — Not started
- `[~]` — In progress (claimed by a subagent)
- `[x]` — Completed
- `[!]` — Blocked / needs human input

---

## Phase 1 — Ledger model

| Task | Status | Notes |
|---|---|---|
| T01 — Account and transaction types | `[x]` | Money is an integer count of minor units. |
| T02 — Double-entry posting rules | `[x]` | Rejects unbalanced entries. |
| T03 — Period close | `[x]` | Idempotent; second close is a no-op. |

---

## Phase 2 — Reporting

| Task | Status | Notes |
|---|---|---|
| T04 — Trial balance report | `[x]` | Matches the hand-computed example. |
| T05 — CSV and JSON exporters | `[x]` | Round-trips through both formats. |

---

## Phase 3 - Hardening

| Task | Status | Notes |
|---|---|---|
| T06 — Input validation sweep | `[x]` | Found two missing currency checks. |
| T07 — Fuzz the posting parser | `[~]` | Harness written; corpus pending. |

---

## Accumulated Learnings

> Subagents append discoveries here after each task.
> The orchestrator reads this section at the start of every iteration
> to avoid repeating past mistakes.

- Store money as integers; floating point sums drifted by a cent in the first draft.
- Fix the lockfile pin for the decimal crate before benchmarking.

## Codebase State

Library crate with a `ledger` module, a `report` module and a small binary.

## Out-of-band cleanup

| Task | Status | Notes |
|---|---|---|
| T08 — Remove the unused currency table | `[x]` | Done in a cleanup commit. |
