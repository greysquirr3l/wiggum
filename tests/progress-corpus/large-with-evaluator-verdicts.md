# quill-mq — Implementation Progress

> Orchestrator reads this file at the start of each loop iteration.
> Subagents update this file after completing a task.

## Status Legend

- `[ ]` — Not started
- `[~]` — In progress (claimed by a subagent)
- `[x]` — Completed
- `[!]` — Blocked / needs human input

---

## Phase 1 — Wire format

| Task | Status | Notes |
|---|---|---|
| T01 — Define the broker handshake | `[x]` | Adds a feature flag, off by default. |
| T02 — Implement the topic registry | `[x]` | No public API change. |
| T03 — Wire the partition map | `[x]` | Adds a feature flag, off by default. |
| T04 — Harden the consumer offsets | `[x]` | Green on first run. |
| T05 — Benchmark the retention sweeper | `[x]` | Green on first run. |

---

## Phase 2 — Broker core

> Starts after: Phase 1 is complete

| Task | Status | Notes |
|---|---|---|
| T06 — Document the dead-letter queue | `[x]` | Reviewed against the wire-format table. |
| T07 — Migrate the message envelope | `[x]` | Reviewed against the wire-format table. |
| T08 — Validate the batch codec | `[x]` | No public API change. |
| T09 — Expose the replay cursor | `[x]` | Docs updated alongside. |
| T10 — Instrument the quota ledger | `[x]` | Adds a feature flag, off by default. |

---

## Phase 3 — Persistence

> Starts after: Phase 2 is complete

| Task | Status | Notes |
|---|---|---|
| T11 — Define the admin socket | `[x]` | Docs updated alongside. |
| T12 — Implement the health probe | `[x]` | **Human gate confirmed 2027-01-28.** Storage layout approved before bulk code. |
| T13 — Wire the snapshot exporter | `[x]` | Mutation scratch run caught every seeded fault. |
| T14 — Harden the rebalance protocol | `[x]` | Docs updated alongside. |
| T15 — Benchmark the producer ack path | `[x]` | Numbers recorded in the bench log. |

---

## Phase 4 — Consumers

> Starts after: Phase 3 is complete

| Task | Status | Notes |
|---|---|---|
| T16 — Document the schema registry client | `[x]` | Docs updated alongside. |
| T17 — Migrate the tracing spans | `[x]` | Numbers recorded in the bench log. |
| T18 — Validate the shutdown sequence | `[x]` | Numbers recorded in the bench log. |
| T19 — Expose the config reload | `[x]` | Numbers recorded in the bench log. |
| T20 — Instrument the backpressure signal | `[x]` | Covered by 11 unit tests and one soak scenario. |

---

## Phase 5 — Delivery guarantees

> Starts after: Phase 4 is complete

| Task | Status | Notes |
|---|---|---|
| T21 — Define the compaction planner | `[x]` | **Independent evaluator PASS, about 8.8/10 (2027-02-02).** First attempt cleared every hard-fail criterion. |
| T22 — Implement the lease manager | `[x]` | Adds a feature flag, off by default. |
| T23 — Wire the metrics endpoint | `[x]` | Reviewed against the wire-format table. |
| T24 — Harden the fan-out router | `[x]` | Reviewed against the wire-format table. |
| T25 — Benchmark the cursor checkpoint | `[x]` | Docs updated alongside. |

---

## Phase 6 — Admin plane

> Starts after: Phase 5 is complete

| Task | Status | Notes |
|---|---|---|
| T26 — Document the tenant namespace | `[x]` | Reviewed against the wire-format table. |
| T27 — Migrate the idempotency table | `[x]` | Covered by 11 unit tests and one soak scenario. |
| T28 — Validate the wire protocol docs | `[x]` | Numbers recorded in the bench log. |
| T29 — Expose the TLS listener | `[x]` | Covered by 11 unit tests and one soak scenario. |
| T30 — Instrument the audit trail | `[x]` | Reviewed against the wire-format table. |

---

## Phase 7 — Observability

> Starts after: Phase 6 is complete

| Task | Status | Notes |
|---|---|---|
| T31 — Define the disk quota guard | `[x]` | Covered by 11 unit tests and one soak scenario. |
| T32 — Implement the client retry policy | `[x]` | Numbers recorded in the bench log. |
| T33 — Wire the bulk import tool | `[x]` | **Final re-evaluation PASS, about 9.2/10 (2027-02-09) after two FAILs.** Details in the evaluator sections below. |
| T34 — Harden the queue inspector | `[x]` | Covered by 11 unit tests and one soak scenario. |
| T35 — Benchmark the soak test harness | `[x]` | Numbers recorded in the bench log. |

---

## Phase 8 — Security

> Starts after: Phase 7 is complete

| Task | Status | Notes |
|---|---|---|
| T36 — Document the broker handshake | `[x]` | Reviewed against the wire-format table. |
| T37 — Migrate the topic registry | `[x]` | Reviewed against the wire-format table. |
| T38 — Validate the partition map | `[x]` | Numbers recorded in the bench log. |
| T39 — Expose the consumer offsets | `[x]` | Covered by 11 unit tests and one soak scenario. |
| T40 — Instrument the retention sweeper | `[x]` | Numbers recorded in the bench log. |

---

## Phase 9 — Tenancy

> Starts after: Phase 8 is complete

| Task | Status | Notes |
|---|---|---|
| T41 — Define the dead-letter queue | `[x]` | Mutation scratch run caught every seeded fault. |
| T42 — Implement the message envelope | `[x]` | Numbers recorded in the bench log. |
| T43 — Wire the batch codec | `[x]` | Numbers recorded in the bench log. |
| T44 — Harden the replay cursor | `[x]` | Green on first run. |
| T45 — Benchmark the quota ledger | `[x]` | Numbers recorded in the bench log. |

---

## Phase 10 — Tooling

> Starts after: Phase 9 is complete

| Task | Status | Notes |
|---|---|---|
| T46 — Document the admin socket | `[x]` | No public API change. |
| T47 — Migrate the health probe | `[x]` | **Re-evaluation PASS, 9/10 (2027-02-12) after a first FAIL.** |
| T48 — Validate the snapshot exporter | `[x]` | Green on first run. |
| T49 — Expose the rebalance protocol | `[x]` | Adds a feature flag, off by default. |
| T50 — Instrument the producer ack path | `[x]` | Green on first run. |

---

## Phase 11 — Hardening

> Starts after: Phase 10 is complete

| Task | Status | Notes |
|---|---|---|
| T51 — Define the schema registry client | `[x]` | Green on first run. |
| T52 — Implement the tracing spans | `[x]` | Numbers recorded in the bench log. |
| T53 — Wire the shutdown sequence | `[x]` | Reviewed against the wire-format table. |
| T54 — Harden the config reload | `[x]` | Covered by 11 unit tests and one soak scenario. |
| T55 — Benchmark the backpressure signal | `[ ]` |  |

---

## Phase 12 — Release

> Starts after: Phase 11 is complete

| Task | Status | Notes |
|---|---|---|
| T56 — Document the compaction planner | `[ ]` |  |
| T57 — Migrate the lease manager | `[ ]` |  |
| T58 — Validate the metrics endpoint | `[~]` | Claimed; schema check in progress. |
| T59 — Expose the fan-out router | `[!]` | Waiting on a human decision about the retention default. |
| T60 — Instrument the cursor checkpoint | `[ ]` |  |

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

- Keep the batch codec allocation-free on the hot path; the first draft cloned every envelope.
- The offsets table must be written before the ack is sent or restarts replay acknowledged messages.
- T21 is the template for the evaluator hand-off: record the hash, the mutations and the score.
- Fix the lockfile drift before touching the benches; stale numbers cost an afternoon.
- T33 hit a snapshot-format surprise; the sections below say what the evaluator had to see.
- Compaction and retention share a lock; always take them in the same order.
- Soak tests need a fixed seed or failures cannot be replayed.

## Codebase State

> Subagents update this section after completing each task.
> Describe what now exists, what is wired up, and what key decisions were made.
> A fresh agent should be able to orient from this section alone.

Cargo workspace with `broker`, `codec`, `admin` and `tools` crates. The wire format is frozen at version 2.

## T21 evaluator PASS (2027-02-02, about 8.8/10) on `dd051a0`

Independent evaluator: every hard-fail criterion passed; three scratch mutations were caught.

## T33 learnings

- Segment snapshots must include the offsets table or restores lose consumer positions.
- A partial write must be detectable by checksum, not by length alone.

## T33 evaluator FAIL (2027-02-04, about 6.8/10) on `4c46423` — retry 1 of 2

Findings:

1. The snapshot omits the offsets table.
2. No test restores from a truncated file.

## T33 re-evaluation FAIL (2027-02-06, about 8.3/10) on `3b99202` — retry 2 of 2 (final retry; another failure stops the loop and asks the human)

Offsets restored; the truncated-file test only asserts an error, not which one.

## T33 final re-evaluation PASS (2027-02-09, about 9.2/10) on `7d97677`

Both findings verified fixed; four mutations caught.

## T47 evaluator FAIL (2027-02-11, 7/10) on `de68e7e` — retry 1 of 2

The fan-out router drops messages when a consumer disconnects mid-batch.

## T47 re-evaluation PASS (2027-02-12, 9/10) on `6050eee`

The disconnect test now asserts redelivery.

## T39 security evaluation: conditional PASS (2027-02-10, about 9.1/10) on `6d47bdb` — follow-up round 1 of 2

Conditional on a follow-up to rotate the listener certificates; recorded as a non-blocking item.

## T39 re-evaluation PASS (2027-02-10, about 9.2/10) on `08c52fe`

Rotation follow-up landed.

## T44 crypto review FAIL (2027-02-11, about 8.3/10) on `cf67581` — retry 1 of 2 (scoped to the cipher path)

Nonce reuse is possible after a counter wrap.

## T44 final re-review PASS (2027-02-12, about 9.2/10) on `e260b7f`

Counter wrap now refuses to send.

## Gates confirmed by the human 2027-01-28: T12 (storage layout)

The human approved the storage layout before any bulk code was written.

## Out-of-band follow-ups

| Task | Status | Notes |
|---|---|---|
| T61 — Rotate the listener certificates | `[x]` | Done in the cleanup commit set. |
| T62 — Delete the deprecated v1 codec | `[x]` | No remaining callers. |
| T63 — Add a restore drill to the soak harness | `[~]` | In progress. |
| T64 — Publish the wire protocol docs | `[ ]` |  |
