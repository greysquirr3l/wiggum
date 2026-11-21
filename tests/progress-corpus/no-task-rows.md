# basalt-scan — Remediation Progress

Tracks remediation of a one-off review of the scanner (three parallel passes: parser,
report writer, packaging). Findings are tracked as numbered items; there are no phased tasks.

Item states are `open`, `in-progress`, `done` and `wontfix`.

## High severity

| # | Finding | Status |
|---|---------|--------|
| 12 | Report writer truncates paths longer than 255 bytes | done |
| 7 | Parser accepts a negative section length | in-progress |
| 3 | Packaging script overwrites an existing output directory | open |

## Low severity backlog

- Rename the `--quiet` flag to match the other tools.
- Add a progress bar for large inputs.
- Clarify the exit codes in the manual page.
