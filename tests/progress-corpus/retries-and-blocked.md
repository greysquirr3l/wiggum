# tern-sync — Implementation Progress

> Orchestrator reads this file at the start of each loop iteration.
> Subagents update this file after completing a task.

## Status Legend

- `[ ]` — Not started
- `[~]` — In progress (claimed by a subagent)
- `[x]` — Completed
- `[!]` — Blocked / needs human input

---

## Phase 1 — Core sync

| Task | Status | Notes |
|---|---|---|
| T01 — Directory scanner | `[x]` | Handles symlink loops. |
| T02 — Content hashing with a block cache | `[x]` | Retry 1: first cache was keyed by path and went stale; re-keyed by inode. |
| T03 — Delta computation | `[x]` | Retry 1 and retry 2: the rolling hash collided on zero blocks; added a salted second hash. |
| T04 — Conflict detection | `[x]` | Needed the human to pick the winner policy; resolved 2027-03-01. |
| T05 — Transfer queue | `[!]` | Blocked: the remote test host is not reachable from the runner. |

---

## Phase 2 — Edge cases

> Starts after: Phase 1 is complete

| Task | Status | Notes |
|---|---|---|
| T06 — Partial transfer resume | `[~]` | Claimed; resume tokens drafted. |
| T07 — Sparse file support | `[ ]` |  |
| T08 — Case-insensitive filesystem handling | `[!]` | Blocked on T05 and a decision about normalisation. |
| T09 — Quality gate script for CI | `[x]` | Plain shell script; not a human gate. |

---

## Phase 3 — Release

| Task | Status | Notes |
|---|---|---|
| T10 — Human sign-off before publishing (gate) | `[ ]` | Gate: waiting for the human. |
| T11 — Packaging for three platforms | `[ ]` |  |
| T12 — Changelog and migration notes | `[ ]` |  |

---

## Known baseline failures

> The orchestrator records preflight failures that exist before the first task.
> Tasks may remove entries from this list but must never add to it.

- Two flaky tests in the network module (pre-existing; tracked by the orchestrator).

## Approvals

> Record every human approval here before acting on it. The Approvals section of the
> generated orchestrator or CLAUDE.md prompt says what an approval covers.

| Scope | Action | Approved by | Expires |
|---|---|---|---|

## Accumulated Learnings

> Subagents append discoveries here after each task.
> The orchestrator reads this section at the start of every iteration
> to avoid repeating past mistakes.

- Rolling hashes need a second independent hash for zero-filled blocks (T03 needed two retries for this).
- Do not key caches by path; use inode plus mtime.
- The remote host is firewalled from CI; blocked tasks must say what they are waiting for.

## Codebase State

> Subagents update this section after completing each task.
> Describe what now exists, what is wired up, and what key decisions were made.
> A fresh agent should be able to orient from this section alone.

Library crate `tern-sync` plus a `tern` binary. Transfer queue is stubbed pending the reachable host.
