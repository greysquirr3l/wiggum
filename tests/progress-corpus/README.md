# Progress corpus

Ten synthetic `PROGRESS.md` files, hand-labelled, used as the test bed for
anything that reads a PROGRESS.md (`parse_progress`, `wiggum retro`, and the
evaluator-verdict and learnings parsing planned for the improvement engine).

## Provenance

Every file here is synthetic. The shapes (heading styles, table layouts,
evaluator verdict headings, out-of-band sections, odd titles) were learned by
reading real Wiggum runs; no text, project name, task title, path, URL,
hash or sentence was copied from them. Project names, tasks, dates and hashes
are all invented. Keep it that way: never paste a real PROGRESS.md here, not
even redacted. `tests/progress_corpus_test.rs` has a leak guard (no URLs, no
hash-like runs other than 7-hex fake hashes in backticks, no home paths, no
email addresses).

## Files

| File | Shape |
|---|---|
| `small-all-done.md` | 8 tasks, 2 phases, all done, no verdicts |
| `large-with-evaluator-verdicts.md` | 64 tasks, 12 phases, out-of-band task rows, many verdict wordings |
| `retries-and-blocked.md` | retries recorded only as notes, `[!]` and `[~]` tasks, a human gate |
| `no-task-rows.md` | free-form remediation list, zero `\| T..` rows |
| `scratch-copy-weird-title.md` | first line is `# Progress`, no project name |
| `legacy-no-approvals.md` | older generated shape, a hyphen phase heading |
| `with-approvals-and-baseline.md` | current shape, task-shaped Approvals and baseline rows |
| `session-handoff-section.md` | `## Session handoff` with a task-shaped table |
| `learnings-heavy.md` | many learnings; retry-ish words that are not retries |
| `evaluator-fail-then-pass.md` | FAIL, PASS and FAIL, FAIL, PASS sequences |

## Label schema (`labels.toml`)

One table per file, keyed by file name. All values are hand-derived truth.

| Key | Meaning |
|---|---|
| `shape` | one-line description of why the file is in the corpus |
| `project_name` | the name a good parser should report |
| `tasks`, `completed`, `in_progress`, `blocked`, `not_started` | task row counts (rows under Approvals, Known baseline failures and Session handoff are not tasks) |
| `phases`, `phase_names` | phase count and names, in file order |
| `real_retries` | recorded retries: FAIL verdict headings plus explicit `retry N` notes. Words such as fix, failed or attempt do not count |
| `explicit_retries` | the part of `real_retries` recorded as `retry N` notes rather than verdict headings |
| `gate_tasks` | ids of tasks that were human-confirmation gates |
| `lessons` | free text: what a good retro would surface |
| `[file.verdicts]` | per task, the evaluator verdict sequence as `FAIL:6.8`, `PASS:9.2` (in order) |
| `[file.retro_today]` | the numbers in the snapshot header: tasks, retries, gates |
| `[file.known_defects]` and `[file.today]` | see below |

The test also cross-checks the labels against the file itself: verdict
headings are re-extracted and compared, `real_retries` must equal FAIL
verdicts plus `explicit_retries`, and `explicit_retries` must equal the
`retry N` mentions found outside headings.

## Known defects

Truth is never bent to match today's code. When the parser or retro disagrees,
the field is named in `[file.known_defects]` (with the reason) and today's value
is recorded in `[file.today]`. The test asserts the defect is still present and
unchanged; when a later task fixes it the test fails and tells you to delete
the entry, which is how the change gets noticed. Today's defects:

- `retro_retries`: retro counts keyword hits (fix, failed, attempt, retry) in
  notes and learnings, so it over-counts ordinary words and under-counts
  retries recorded only as verdict headings. Replaced by typed patches in T06.
- `retro_gates`: retro counts tasks with "gate" in the title, not human gates.
- `project_name` in `scratch-copy-weird-title.md`: parses as `Progress`.
- `phases` in `legacy-no-approvals.md`: a `## Phase 3 - X` heading (hyphen) is
  not recognised.

## Snapshots

`snapshots/<file>.retro.txt` is the output of `wiggum retro --progress <file>`
from the tree at the time the corpus was added, before any change to
`retro.rs`. They record the CURRENT behaviour, including its defects (retries
over-counted, suggestions attributed to a `General` task). Later tasks should
regenerate them and review the diff to see what changed.

Regenerate (the order of the Pattern blocks is nondeterministic, so the blocks
are sorted, ANSI codes stripped):

```bash
cargo build
cd tests/progress-corpus
for f in *.md; do
  [ "$f" = README.md ] && continue
  ../../target/debug/wiggum retro --progress "$f" 2>&1 | sed 's/\x1b\[[0-9;]*m//g' | python3 -c '
import sys
parts = sys.stdin.read().rstrip("\n").split("\n\n")
print("\n\n".join([parts[0]] + sorted(parts[1:])))
' > "snapshots/${f%.md}.retro.txt"
done
```

After regenerating, update `[file.retro_today]` (and `known_defects` / `today`
where the numbers changed) from each snapshot's first line.

## Adding a file

Add `<name>.md`, a `["<name>.md"]` table in `labels.toml` and
`snapshots/<name>.retro.txt`. A corpus file without a labels entry fails the test.
