//! Renders `RUN_LOG.md`, a universal scaffold artifact that ships as an
//! empty iteration log the orchestrator can append to as the loop runs.
//!
//! Unlike `` `BUDGET.md` `` (which is fully populated), `RUN_LOG.md` ships with
//! an empty markdown table whose header row is pre-populated and whose
//! body rows are blank. Pure function: no filesystem, no IO, no plan
//! dependency.

use std::fmt::Write as _;

use crate::error::Result;

/// Render the empty `RUN_LOG.md` scaffold.
///
/// Layout:
/// 1. H1 heading.
/// 2. Short explanation of what this file is for.
/// 3. "How to use" section with the per-iteration convention.
/// 4. Markdown table with seven columns and one example placeholder row
///    so the orchestrator (or a human) can see the expected shape.
pub fn render() -> Result<String> {
    let mut out = String::with_capacity(1024);

    let _ = writeln!(out, "# RUN_LOG");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Per-iteration audit log for the orchestrator loop. The orchestrator \
         (or a human) appends one row per dispatch. Rows must remain in \
         chronological order — never edit or delete an existing row."
    );
    let _ = writeln!(out);

    // ── How to use ───────────────────────────────────────────────────
    let _ = writeln!(out, "## How to use");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "After each orchestrator dispatch — successful or not — append a row \
         to the table below. Capture enough detail that the loop can be \
         replayed from this file alone."
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- `timestamp` — UTC ISO-8601 (`YYYY-MM-DDTHH:MM:SSZ`)."
    );
    let _ = writeln!(
        out,
        "- `task_id` — `T{{NN}}-{{slug}}` from `tasks/`. Use `T00-bootstrap` for the initial setup dispatch."
    );
    let _ = writeln!(
        out,
        "- `attempt` — attempt count for this task within the current orchestrator run (1-indexed)."
    );
    let _ = writeln!(
        out,
        "- `preflight_result` — `pass` / `fail:{{reason}}` / `skip`."
    );
    let _ = writeln!(
        out,
        "- `commit_sha` — short SHA of the commit produced by this iteration, or `-` if no commit."
    );
    let _ = writeln!(
        out,
        "- `duration_seconds` — wall-clock time spent in the subagent dispatch."
    );
    let _ = writeln!(
        out,
        "- `notes` — anything else worth recording (retries, gate decisions, eval verdicts)."
    );
    let _ = writeln!(out);

    // ── Empty table ───────────────────────────────────────────────────
    let _ = writeln!(out, "## Iterations");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| timestamp | task_id | attempt | preflight_result | commit_sha | duration_seconds | notes |"
    );
    let _ = writeln!(
        out,
        "|-----------|---------|---------|------------------|------------|------------------|-------|"
    );
    let _ = writeln!(
        out,
        "| YYYY-MM-DDTHH:MM:SSZ | T00-bootstrap | 1 | skip | - | 0 | Initial scaffold; no subagent dispatch. |"
    );
    let _ = writeln!(out);

    Ok(out)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn render_contains_all_seven_column_headers() {
        let md = render().unwrap();
        let expected = [
            "timestamp",
            "task_id",
            "attempt",
            "preflight_result",
            "commit_sha",
            "duration_seconds",
            "notes",
        ];
        for header in expected {
            assert!(md.contains(header), "missing column header: {header}\n{md}");
        }
    }

    #[test]
    fn render_contains_placeholder_row() {
        let md = render().unwrap();
        assert!(
            md.contains("| YYYY-MM-DD"),
            "missing example row with YYYY-MM-DD placeholder:\n{md}"
        );
    }

    #[test]
    fn render_has_how_to_use_section() {
        let md = render().unwrap();
        assert!(md.contains("## How to use"), "missing How to use section");
        assert!(
            md.contains("## Iterations"),
            "missing Iterations table section"
        );
    }
}
