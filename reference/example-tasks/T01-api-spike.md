# T01 — API Spike (Agent Task — requires Chrome open)

> **This task is automated via the chrome-devtools MCP.**  
> The human must start Chrome with remote debugging before this subagent runs.

## Pre-requisite (human action — 30 seconds)

Launch Chrome with remote debugging enabled and navigate to Teams:

```bash
# macOS
open -a "Google Chrome" --args --remote-debugging-port=9222 https://teams.microsoft.com
```

Log in to Teams if prompted. Wait until the main Teams UI has loaded and you can see conversations. Then start the orchestrator — it will handle the rest.

---

## Why

All phases depend on current Teams internal API shapes. The fossteams reference is 4 years old; endpoints may have drifted. This task validates before writing any Rust code.

---

## Agent Steps

Use the **chrome-devtools MCP** tools (`mcp_chrome-devtools_*`) to complete the following. The MCP connects to Chrome at `localhost:9222`.

### S-1 — Verify Teams API domains are active

Use the chrome-devtools MCP to list recent network requests. Confirm requests to:

- `api.spaces.skype.com`
- `chatsvcagg.teams.microsoft.com`

If neither domain appears, navigate to `https://teams.microsoft.com` in the attached Chrome tab and wait for it to settle, then re-inspect.

### S-2 — Extract skypetoken

From a request to either domain, extract the value of the `Authorization` header. It begins with `skypetoken`. Strip the prefix — the remainder is the raw token.

Also extract:

- `X-Ms-Client-Version` header value (documents the Teams build)
- `User-Agent` header value
- The full URL of the request (to confirm the base path)

**Do not log the token value in any output that persists beyond this task. Write it only to the docs/api-notes.md file under a placeholder comment.**

### S-3 — Confirm current endpoint shapes via network inspection

Inspect the captured network requests and find responses for:

1. **User profile** — a request to a path containing `users/me` or `teams/users/me`
   - Note: full URL, response status, top-level JSON keys

2. **Conversation list** — a request returning a list of conversations/chats  
   - Note: full URL, response status, the key that contains the array, shape of one item

3. **Message list** — a request returning messages for a specific conversation  
   - Note: full URL with conversation ID pattern, response status, shape of one message item

4. **authsvc response** — look for a request to `authsvc/v1.0/authz`  
   - Note: full URL, response body keys (especially any `trouter`-related fields)

### S-4 — Confirm message send endpoint

Inspect POST requests made when a message is sent (type a test message in Teams if no recent POSTs are visible). Note:

- Full URL
- Request body shape (redact any real content — use `"content": "<redacted>"` in notes)
- Response status

### S-5 — WebSocket / Trouter inspection

Use the chrome-devtools MCP to inspect WebSocket connections. Find any WS connection to a `trouter` or `msg.skype.com` URL. Note:

- Full WS URL
- First few frames sent after connection (registration frame shape)
- Shape of an incoming notification frame (message received event)

If no active WS is visible, check the `authsvc` JSON response for fields named `trouter`, `registrationToken`, `socketio`, or similar.

---

## Output: create `docs/api-notes.md`

Create the file `/Users/nickcampbell/Projects/rust/yakko/docs/api-notes.md` with:

```markdown
# Teams Internal API Notes
_Last validated: YYYY-MM-DD_
_Teams client version: (from X-Ms-Client-Version header)_

## Auth

### skypetoken
- Obtained from: (describe)
- Header names used: Authorization (value: `skypetoken <token>`), X-Skypetoken
- Expiry: (from authsvc response if present)

### authsvc endpoint
- URL: 
- Request method + body shape:
- Response body keys:

## Conversations

### List conversations
- URL: 
- Method: GET
- Required headers: (list exact names)
- Response shape: (top-level key, array item fields)

## Messages

### Get messages
- URL pattern: (use {conversationId} as placeholder)
- Method: GET
- Response shape: (top-level key, item fields: id, content, from, timestamp key names)

### Send message
- URL pattern:
- Method: POST
- Request body shape:
- Response: (status + shape)

## Trouter WebSocket

### Connection
- WS URL (pattern): 
- Source: (authsvc field name / hardcoded / other)

### Registration frame
```json
{}
```

### Incoming notification frame (new message)

```json
{}
```

## Discrepancies from fossteams/teams-api (2021)

- (list any URL or field name differences)

```

Fill in every section from your DevTools observations. Where a value is unknown, write `(not observed — needs follow-up)`.

---

## Completion

Update PROGRESS.md: mark all Phase 0 `S-*` rows as `[x]`.  
Commit: `chore(spike): document Teams internal API shapes via DevTools inspection`
