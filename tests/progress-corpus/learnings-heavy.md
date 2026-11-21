# marl-parse — Implementation Progress

> Orchestrator reads this file at the start of each loop iteration.
> Subagents update this file after completing a task.

## Status Legend

- `[ ]` — Not started
- `[~]` — In progress (claimed by a subagent)
- `[x]` — Completed
- `[!]` — Blocked / needs human input

---

## Phase 1 — Grammar

| Task | Status | Notes |
|---|---|---|
| T01 — Tokenizer with error recovery | `[x]` | Spans survive recovery. |
| T02 — Expression parser | `[x]` | Pratt parser; precedence table is data. |
| T03 — Statement parser | `[x]` | Synchronises on semicolons. |
| T04 — Pretty printer | `[x]` | Round-trips the whole sample suite. |
| T05 — Linter rule engine | `[x]` | Rules are plain structs; 6 shipped. |

---

## Phase 2 — Diagnostics

| Task | Status | Notes |
|---|---|---|
| T06 — Source snippets in errors | `[x]` | Tabs expand to four columns. |
| T07 — Machine-readable output | `[x]` | One JSON object per line. |
| T08 — Editor protocol adapter | `[~]` | Initial handshake works. |

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

### Parser

- T01: Spans must be byte offsets; character offsets broke on multi-byte input.
- T02: Retry 1 on this task: the precedence table was inverted for unary minus; the test suite now pins it.
- T02: The Pratt loop needs a binding-power check before consuming, not after.
- T03: Failed to synchronise after a missing brace in the first draft; added a recovery set per statement kind.
- Fix the lockfile before running the fuzz job; a stale lock selected an old regex engine.
- Attempt to inline the tokenizer into the parser was reverted; the profile showed no gain.

### Tooling

- T05: Clippy flagged the rule trait for returning owned strings; switched to borrowed messages.
- T05: The lint config belongs in the first task of the phase, not the last.
- T07: Keep the JSON schema in one file and generate the docs from it.
- The docs build is slow; run it only in the release task.
- Waiting for CI is not the same as blocked; blocked means a human must act.
- A split between the lexer and parser crates was considered and rejected: too much glue for the gain.

### Process

- Every task should end with the command that proves it, not a prose claim.
- Evaluator notes belong in a dated section, not in the learnings list.
- Prefer deleting dead code in the same commit that makes it dead.

## Codebase State

> Subagents update this section after completing each task.
> Describe what now exists, what is wired up, and what key decisions were made.
> A fresh agent should be able to orient from this section alone.

Crates `marl-lex`, `marl-parse` and `marl-cli`. The editor adapter is partial.
