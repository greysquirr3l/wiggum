# onyx-auth — Implementation Progress

> Orchestrator reads this file at the start of each loop iteration.
> Subagents update this file after completing a task.

## Status Legend

- `[ ]` — Not started
- `[~]` — In progress (claimed by a subagent)
- `[x]` — Completed
- `[!]` — Blocked / needs human input

---

## Phase 1 — Credentials

| Task | Status | Notes |
|---|---|---|
| T01 — Password hashing with a tuned cost | `[x]` | Cost chosen from a measured table. |
| T02 — Session token issue and verify | `[x]` | **Re-evaluation PASS, about 9/10 (2027-05-04) after a first FAIL.** |
| T03 — Refresh token rotation | `[x]` | **Final re-evaluation PASS, about 9.1/10 (2027-05-07) after two FAILs.** |

---

## Phase 2 — Policy

> Starts after: Phase 1 is complete

| Task | Status | Notes |
|---|---|---|
| T04 — Role and permission model | `[x]` | Evaluator PASS, about 8.9/10, first attempt. |
| T05 — Rate limiting on login | `[!]` | Blocked: waiting for the budget to be agreed with the human. |
| T06 — Audit log writer | `[ ]` |  |

---

## Approvals

> Record every human approval here before acting on it. The Approvals section of the
> generated orchestrator or CLAUDE.md prompt says what an approval covers.

| Scope | Action | Approved by | Expires |
|---|---|---|---|

## Accumulated Learnings

> Subagents append discoveries here after each task.
> The orchestrator reads this section at the start of every iteration
> to avoid repeating past mistakes.

- Token verification must compare in constant time; the first review caught a plain equality check.
- Rotation needs a grace window for the previous refresh token or concurrent requests log users out.

## Codebase State

> Subagents update this section after completing each task.
> Describe what now exists, what is wired up, and what key decisions were made.
> A fresh agent should be able to orient from this section alone.

Single `onyx-auth` crate. Tokens and rotation are done; policy is partial.

## T02 evaluator FAIL (2027-05-02, about 6.4/10) on `b26f85a` — retry 1 of 2

Equality check on the token signature is not constant time; no negative test for expired tokens.

## T02 re-evaluation PASS (2027-05-04, about 9/10) on `603fe35`

Both findings fixed and pinned by tests.

## T03 evaluator FAIL (2027-05-05, about 7.2/10) on `d22fa53` — retry 1 of 2

Rotation races when two requests present the same token.

## T03 re-evaluation FAIL (2027-05-06, about 8.4/10) on `4f05767` — retry 2 of 2 (final retry; another failure stops the loop and asks the human)

Grace window added but the audit event is missing.

## T03 final re-evaluation PASS (2027-05-07, about 9.1/10) on `397e7f8`

All findings verified fixed.

## T04 evaluator PASS (2027-05-08, about 8.9/10) on `0ea8f13`

First attempt; no findings above minor.
