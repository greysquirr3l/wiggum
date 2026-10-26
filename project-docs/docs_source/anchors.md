# Code Anchors

An **anchor** pins a task to a verified location in the target repository — a
`path`, optionally a line range, and a note explaining why that location matters.

Anchors are a progressive enhancement. **Every plan is valid without them.**
If a plan declares no anchors, nothing about generation, validation, or linting
changes. Add them to the tasks where a subagent would otherwise have to guess
where the work happens.

## Why they exist

A task goal like _"extend the retry logic in the delivery worker"_ sends the
implementer subagent hunting. It greps, finds three plausible sites, picks one,
and may produce a plausible-looking change to the wrong file.

An anchor replaces the guess with a citation:

```toml
[[phases.tasks.anchors]]
path = "src/adapters/queue.rs"
lines = "88-140"
role = "modify"
note = "the exponential-backoff loop — replace the fixed 30s sleep"
```

The subagent starts at a known location. It also has a cheap way to notice when
its instructions have gone stale: if the cited region no longer looks like the
note describes, trust the code.

## Defining an anchor

Anchors live under a task:

```toml
[[phases.tasks]]
slug = "add-retry-backoff"
title = "Exponential backoff on delivery retry"
goal = "Replace the fixed 30-second retry sleep with exponential backoff."
hints = ["Keep the maximum delay capped at 5 minutes."]
test_hints = ["Test that delay roughly doubles across the first four attempts."]

[[phases.tasks.anchors]]
path  = "src/adapters/queue.rs"
lines = "88-140"
role  = "modify"
note  = "the fixed 30s sleep and the retry counter around it"

[[phases.tasks.anchors]]
path = "src/domain/backoff.rs"
role = "create"
note = "new module — compute the next delay from the attempt count"
```

### Fields

| Field   | Required | Type   | Description                                                                                                            |
| ------- | -------- | ------ | ---------------------------------------------------------------------------------------------------------------------- |
| `path`  | yes      | string | Path **relative to `[project].path`**. Absolute paths and paths containing `..` are rejected.                          |
| `lines` | no       | string | Inclusive 1-based range: `"42"` or `"42-80"`. Reversed input (`"80-42"`) is normalised. Omit to anchor the whole file. |
| `role`  | no       | string | `read` (default), `modify`, or `create`.                                                                               |
| `note`  | no       | string | Why this location matters — the symbol to extend, the behaviour to change.                                             |

`role` drives the checks: `create` asserts the file does **not** exist yet;
`read` and `modify` assert it does.

## Rendered output

Anchors are inlined into the generated task file under a `## Code Anchors`
section, so the subagent sees them without opening anything else:

```markdown
## Code Anchors

Start here before searching. These locations were verified against the codebase
when the plan was written; if one no longer matches, trust the code over the plan.

- `src/adapters/queue.rs:88-140` — **modify**: the fixed 30s sleep and the retry counter around it
- `src/domain/backoff.rs` — **create**: new module — compute the next delay from the attempt count
```

Tasks with no anchors get no such section.

## Drift detection

`wiggum validate --lint` resolves every anchor against the project on disk. This
is what turns "the plan says X" into something checkable.

| Condition                                     | Rule                     | Severity                |
| --------------------------------------------- | ------------------------ | ----------------------- |
| `path` is empty                               | `anchor-empty-path`      | error                   |
| File missing (`read`/`modify`)                | `anchor-file-missing`    | error                   |
| Path is a directory                           | `anchor-is-directory`    | error                   |
| Path is absolute or escapes the project root  | `anchor-escapes-root`    | warning                 |
| Line range ends past the last line            | `anchor-line-drift`      | warning                 |
| `lines` is not parseable                      | `anchor-bad-lines`       | warning                 |
| File not readable as UTF-8                    | `anchor-unreadable`      | info                    |
| `role = "create"` but the file already exists | `anchor-already-created` | info                    |
| No `note` supplied                            | `anchor-no-note`         | info                    |
| `[project].path` does not exist               | `anchor-root-missing`    | warning (reported once) |

```console
$ wiggum validate plan.toml --lint
✅ Plan is valid
   Phases: 1
   Tasks:  1
   Order:  add-retry-backoff

Lint: 1 warning
  ⚠  T01-add-retry-backoff: `src/adapters/queue.rs:88-140` points past end of
     file (2 lines) — the code likely moved
```

Warnings do not fail the run. **Errors do not block generation either** —
`validate --lint` reports them, and `wiggum generate` proceeds. Treat a
`anchor-file-missing` error as "this plan needs a refresh before it is worth
running", not as a hard gate.

The same checks run through the MCP `wiggum_lint_plan` tool, so an agent driving
wiggum through MCP gets drift warnings in its lint result too.

## Keeping anchors honest

Anchors rot. A refactor upstream shifts lines, and a citation written last month
can now point at the wrong function. Two habits keep them trustworthy:

- **Re-run `wiggum validate --lint` before `wiggum generate`.** Drift is cheap
  to find and cheap to fix at that point.
- **Treat a `anchor-line-drift` warning as "re-read the file", not "widen the
  range".** The line numbers moved; the note is still the thing that tells you
  which function you meant.

If an anchor no longer describes anywhere useful, delete it. An anchor that is
present but wrong is worse than no anchor, because the subagent is told to trust
it.

## Relationship to capabilities

Capabilities and anchors answer different questions:

- A **capability** says _what must be true_ of the system — `WHEN ... THEN ...`
  scenarios, invariant across tasks and phases.
- An **anchor** says _where in the code_ a specific task does its work.

They compose: a task can implement a capability _and_ anchor to the region it
edits. Anchors are per-task and usually short-lived; capabilities are
plan-scoped and long-lived.
