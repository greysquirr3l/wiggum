# Yakko — Implementation Progress

> Orchestrator reads this file at the start of each loop iteration.  
> Subagents update this file after completing a task.

## Status Legend

- `[ ]` — Not started
- `[~]` — In progress (claimed by a subagent)
- `[x]` — Completed
- `[!]` — Blocked / needs human input

---

## Phase 0 — API Spike

> Automated via **chrome-devtools MCP**.  
> **Human pre-req**: launch Chrome with `--remote-debugging-port=9222` at `teams.microsoft.com` and log in.

| Task | Status | Notes |
|---|---|---|
| T01 — DevTools inspection → docs/api-notes.md | `[x]` | Completed 2026-02-24. Major finding: Teams v2 migrated to `teams.cloud.microsoft` with AAD Bearer auth. See `docs/api-notes.md`. |

---

## Phase 1 — Workspace Scaffold

> Depends on: Phase 0 all `[x]`

| Task | Status | Notes |
|---|---|---|
| T02 — Cargo workspace + 3 crates + CI pipeline | `[x]` | Completed 2026-02-24. 3-crate workspace scaffold with stub modules, logging init, and CI pipeline. Preflight clean. |

---

## Phase 2 — Domain Model

> Depends on: T02

| Task | Status | Notes |
|---|---|---|
| T03 — Entities, value objects, port traits, error types | `[x]` | Completed 2026-02-24. All domain types, 6 port traits, 6 ID tests passing. Preflight clean. |

---

## Phase 3 — Infrastructure Adapters

> Depends on: T03

| Task | Status | Notes |
|---|---|---|
| T04 — Token store (KeyringAdapter + TokenStorePort) | `[x]` | Completed 2026-02-24. KeyringAdapter impl with save/load/clear, chrono serialization, ignored integration test. Preflight clean. |
| T05 — Auth adapter (PKCE + AAD + skypetoken exchange) | `[x]` | Completed 2026-02-24. PKCE S256, callback server, AadAuthAdapter with AuthPort impl. Exchange functions are todo!(). 14 new tests passing. Preflight clean. |
| T06 — Teams HTTP client + wire models + port impls | `[x]` | Completed 2026-02-24. TeamsClient with authed_get/post, 11 wire models (WireMessage, WireMessagePage, WireUpdatesResponse, WireChannelResponse, etc.), mapping layer (wire_to_message/conversation/team), ConversationPort+MessagePort+PresencePort impls. 10 new tests. Preflight clean. |

---

## Phase 4 — Basic TUI (Read-Only)

> Depends on: T06

| Task | Status | Notes |
|---|---|---|
| T07 — AppState + AppEvent + reduce + event loop | `[x]` | Completed 2026-02-24. Elm/Redux pure reduce, AppState/AppScreen/InputMode/ConnectionStatus, event loop with tokio::select!, 10 unit tests passing. Preflight clean. |
| T08 — Terminal setup/teardown + panic hook + logging | `[x]` | Completed 2026-02-24. Terminal setup/teardown, panic hook, stub render, main rewritten with proper lifecycle. Preflight clean. |
| T09 — Screens and widgets (sidebar, messages, status, help) | `[x]` | Completed 2026-02-24. Sidebar, message list, status bar, help overlay, loading screen, login screen. Full render dispatch. Preflight clean. |
| T10 — Application services read path + polling | `[x]` | Completed 2026-02-24. AuthService, ChatService, ServiceHandle, poll loop with watch channel. Preflight clean. |

---

## Phase 5 — Message Sending

> Depends on: T10

| Task | Status | Notes |
|---|---|---|
| T11 — InputBoxWidget + send flow + error handling | `[x]` | Completed 2026-02-24. InputBox widget (ratatui-textarea), compose row in render layout, send_message in ChatService, optimistic UI with rollback, 3 new reducer tests. Preflight clean. |

---

## Phase 6 — Real-Time Notifications

> Depends on: T11 AND T01 (Trouter info in docs/api-notes.md)

| Task | Status | Notes |
|---|---|---|
| T12 — Trouter WS client + parser + NotificationPort + fan-out | `[x]` | Completed 2026-02-24. TrouterClient with reconnect loop (exp backoff, 5 attempts), alive watch for fallback detection, notification forwarding in ChatService. Parser handles NewMessage, PresenceChanged, TypingIndicator. Preflight clean. |

---

## Phase 7 — Polish & Reliability

> Depends on: T12

| Task | Status | Notes |
|---|---|---|
| T13 — Error handling audit, UX polish, distribution | `[x]` | Completed 2026-02-24. unwrap/expect audit (2 kept with justification, 0 fixed in prod), SIGINT handler, resize safety, release workflow, README, date separators, unread badges. Preflight clean. |

---

## Post-Phase — Trouter Session Allocation

| Task | Status | Notes |
|---|---|---|
| Trouter allocate flow (`/v4/a`) | `[x]` | Completed 2026-02-24. `allocate_session()` + `build_ws_url()` in `yakko-infra::trouter::allocate`. `AuthPort::login()` now returns `(SkypeToken, AadToken)`. Service bootstrap wires allocate → WS URL → `TrouterClient`. Falls back to polling when no AAD token available. |
| Registrar registration | `[x]` | Completed 2026-02-24. `register_with_registrar()` in `yakko-infra::trouter::registrar`. POSTs to `registrar/prod/V2/registrations` with `surl` and `epid` from allocate. Called in service bootstrap after successful allocate, before WebSocket connect. |

---

_Last updated: 2026-02-24_
