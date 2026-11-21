# zephyr-kv — Implementation Progress

> Orchestrator reads this file at the start of each loop iteration.
> Subagents update this file after completing a task.

## Status Legend

- `[ ]` — Not started
- `[~]` — In progress (claimed by a subagent)
- `[x]` — Completed
- `[!]` — Blocked / needs human input

---

## Phase 1 — Storage engine

| Task | Status | Notes |
|---|---|---|
| T01 — Skeleton crate and CI workflow | `[x]` | Workspace builds; CI runs on push. |
| T02 — Write-ahead log record format | `[x]` | Length-prefixed records, 14 tests. |
| T03 — In-memory memtable with tombstones | `[x]` | Ordered map; tombstones shadow older values. |
| T04 — Flush memtable to sorted segment files | `[x]` | Segments are immutable once written. |

---

## Phase 2 — Client surface

> Starts after: Phase 1 is complete

| Task | Status | Notes |
|---|---|---|
| T05 — Get/put/delete API | `[x]` | Returns typed errors, never panics on missing keys. |
| T06 — Prefix scan iterator | `[x]` | Lazy iterator over merged segments. |
| T07 — Command line shell | `[x]` | Reads commands from stdin; 9 tests. |
| T08 — Docs and release notes | `[x]` | README quickstart verified by running it. |

---

## Known baseline failures

> The orchestrator records preflight failures that exist before the first task.
> Tasks may remove entries from this list but must never add to it.

_None recorded._

## Approvals

> Record every human approval here before acting on it. The Approvals section of the
> generated orchestrator or CLAUDE.md prompt says what an approval covers.

| Scope | Action | Approved by | Expires |
|---|---|---|---|

## Accumulated Learnings

> Subagents append discoveries here after each task.
> The orchestrator reads this section at the start of every iteration
> to avoid repeating past mistakes.

- The segment reader must open files read-only; the first draft held a write lock and starved the compactor.
- Fix the lockfile before running the audit step, otherwise it resolves a different dependency set.
- Property tests for the log format found two off-by-one errors that example tests missed.

## Codebase State

> Subagents update this section after completing each task.
> Describe what now exists, what is wired up, and what key decisions were made.
> A fresh agent should be able to orient from this section alone.

Single crate `zephyr-kv` with a `storage` module (log, memtable, segments) and a thin `client` module.
The shell binary is a feature-gated second target.
