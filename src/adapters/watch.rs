use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

use crate::domain::report::{TaskStatus, parse_progress};

// ── ANSI escape codes ──────────────────────────────────────────────────
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";
const CLEAR_SCREEN: &str = "\x1b[2J\x1b[H";

// ── Braille progress bar ───────────────────────────────────────────────
// Uses braille characters for a smooth 2-step-per-cell progress bar.
// Full cell = ⣿, half cell = ⡇, empty cell = ⠀ (braille blank U+2800).

/// Render a progress bar using braille characters.
///
/// `done` tasks completed out of `total`, rendered in `width` character cells.
fn braille_bar(done: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return " ".repeat(width);
    }
    // Each cell represents 2 sub-steps for smooth rendering.
    let steps = width * 2;
    let filled_steps = (done * steps + total / 2) / total; // round
    let full_cells = filled_steps / 2;
    let half = filled_steps % 2;
    let empty_cells = width.saturating_sub(full_cells + half);

    let mut bar = String::with_capacity(width * 3);
    for _ in 0..full_cells {
        bar.push('⣿');
    }
    if half > 0 {
        bar.push('⡇');
    }
    for _ in 0..empty_cells {
        bar.push('\u{2800}'); // braille blank
    }
    bar
}

/// Phase status icon.
const fn phase_icon(completed: usize, total: usize) -> &'static str {
    if total == 0 {
        "\u{2800}" // braille blank
    } else if completed == total {
        "✓"
    } else if completed > 0 {
        "~"
    } else {
        " "
    }
}

/// Format elapsed time as "Xh Ym Zs" or "Ym Zs" or "Zs".
fn format_elapsed(elapsed: std::time::Duration) -> String {
    let secs = elapsed.as_secs();
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h {m}m {s}s")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

/// Render the watch display to a string (testable, no I/O).
#[must_use]
pub fn render_display(content: &str, elapsed: std::time::Duration) -> String {
    let summary = parse_progress(content);
    let mut out = String::with_capacity(1024);

    // Build task status lookup
    let task_status: HashMap<&str, TaskStatus> = summary
        .tasks
        .iter()
        .map(|t| (t.id.as_str(), t.status))
        .collect();

    // Counts
    let completed = summary
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Completed)
        .count();
    let total = summary.tasks.len();
    let blocked = summary
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Blocked)
        .count();

    // Current task (first in-progress, or first not-started)
    let current = summary
        .tasks
        .iter()
        .find(|t| t.status == TaskStatus::InProgress)
        .or_else(|| {
            summary
                .tasks
                .iter()
                .find(|t| t.status == TaskStatus::NotStarted)
        });

    // Header
    let _ = writeln!(
        out,
        "{BOLD}{CYAN}⬡ wiggum watch{RESET} — {BOLD}{}{RESET}\n",
        summary.project_name
    );

    // Phase bars
    let bar_width = 20;
    for phase in &summary.phases {
        let phase_total = phase.task_ids.len();
        let phase_done = phase
            .task_ids
            .iter()
            .filter(|id| task_status.get(id.as_str()) == Some(&TaskStatus::Completed))
            .count();
        let icon = phase_icon(phase_done, phase_total);

        let (bar_color, icon_color) = if phase_done == phase_total && phase_total > 0 {
            (GREEN, GREEN)
        } else if phase_done > 0 {
            (YELLOW, YELLOW)
        } else {
            (DIM, DIM)
        };

        let bar = braille_bar(phase_done, phase_total, bar_width);
        let _ = writeln!(
            out,
            "  {icon_color}[{icon}]{RESET} {:<24} {bar_color}{bar}{RESET}  {phase_done}/{phase_total}",
            phase.name,
        );
    }

    out.push('\n');

    // Current task
    if let Some(task) = current {
        let status_str = match task.status {
            TaskStatus::InProgress => format!("{YELLOW}[~]{RESET}"),
            TaskStatus::NotStarted => format!("{DIM}[ ]{RESET}"),
            TaskStatus::Completed => format!("{GREEN}[x]{RESET}"),
            TaskStatus::Blocked => format!("{RED}[!]{RESET}"),
        };
        let _ = writeln!(
            out,
            "  Current: {BOLD}{} — {}{RESET}  {status_str}",
            task.id, task.title
        );
    } else if completed == total && total > 0 {
        let _ = writeln!(out, "  {GREEN}{BOLD}All tasks completed!{RESET}");
    }

    // Stats line
    #[allow(clippy::cast_precision_loss)]
    let pct = if total > 0 {
        (completed as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let _ = write!(
        out,
        "  Elapsed: {DIM}{}{RESET}    Completed: {completed}/{total}  ({pct:.0}%)",
        format_elapsed(elapsed),
    );
    if blocked > 0 {
        let _ = write!(out, "    {RED}Blocked: {blocked}{RESET}");
    }
    out.push('\n');

    // Learnings
    if summary.learnings_count > 0 {
        let _ = writeln!(out, "  {DIM}Learnings: {}{RESET}", summary.learnings_count);
    }

    out
}

/// Run the watch loop, polling the progress file and re-rendering on change.
///
/// # Errors
///
/// Returns an error if the progress file cannot be read initially.
pub fn run_watch(progress_path: &Path, poll_ms: u64) -> crate::error::Result<()> {
    let start = Instant::now();
    let mut last_content = String::new();
    let mut stdout = io::stdout();

    // Verify file exists before entering loop
    if !progress_path.exists() {
        return Err(crate::error::WiggumError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Progress file not found: {}", progress_path.display()),
        )));
    }

    // Print initial instructions
    eprintln!("Watching {} (Ctrl+C to stop)\n", progress_path.display());

    loop {
        let content = std::fs::read_to_string(progress_path).unwrap_or_default();

        // Only re-render if the file changed
        if content != last_content {
            last_content.clone_from(&content);
            let display = render_display(&content, start.elapsed());
            write!(stdout, "{CLEAR_SCREEN}{display}").ok();
            stdout.flush().ok();
        }

        std::thread::sleep(std::time::Duration::from_millis(poll_ms));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r"# my-project — Implementation Progress

> Orchestrator reads this file at the start of each loop iteration.

## Status Legend

- `[ ]` — Not started
- `[~]` — In progress
- `[x]` — Completed
- `[!]` — Blocked

---

## Phase 1 — Scaffold

| Task | Status | Notes |
|---|---|---|
| T01 — Workspace Setup | `[x]` | Done |
| T02 — Domain Model | `[x]` | |

---

## Phase 2 — Core

| Task | Status | Notes |
|---|---|---|
| T03 — Auth Service | `[~]` | Working on it |
| T04 — HTTP Client | `[ ]` | |
| T05 — Integration | `[!]` | Blocked on T03 |

---

## Accumulated Learnings

> Subagents append discoveries here.

- Edition 2024 requires resolver v3 in workspace Cargo.toml
- Use `thiserror` v2 for custom error types
";

    #[test]
    fn braille_bar_empty() {
        let bar = braille_bar(0, 5, 10);
        // All braille blanks
        assert_eq!(bar.chars().count(), 10);
        assert!(bar.chars().all(|c| c == '\u{2800}'));
    }

    #[test]
    fn braille_bar_full() {
        let bar = braille_bar(5, 5, 10);
        assert_eq!(bar.chars().count(), 10);
        assert!(bar.chars().all(|c| c == '⣿'));
    }

    #[test]
    fn braille_bar_half() {
        let bar = braille_bar(1, 2, 10);
        // Should be roughly half filled
        let full_count = bar.chars().filter(|c| *c == '⣿').count();
        assert_eq!(full_count, 5);
    }

    #[test]
    fn braille_bar_zero_total() {
        let bar = braille_bar(0, 0, 10);
        assert_eq!(bar.len(), 10); // 10 spaces
    }

    #[test]
    fn format_elapsed_seconds() {
        let d = std::time::Duration::from_secs(42);
        assert_eq!(format_elapsed(d), "42s");
    }

    #[test]
    fn format_elapsed_minutes() {
        let d = std::time::Duration::from_secs(125);
        assert_eq!(format_elapsed(d), "2m 5s");
    }

    #[test]
    fn format_elapsed_hours() {
        let d = std::time::Duration::from_secs(3723);
        assert_eq!(format_elapsed(d), "1h 2m 3s");
    }

    #[test]
    fn render_contains_project_name() {
        let output = render_display(SAMPLE, std::time::Duration::from_mins(1));
        assert!(output.contains("my-project"));
    }

    #[test]
    fn render_shows_phases() {
        let output = render_display(SAMPLE, std::time::Duration::from_secs(0));
        assert!(output.contains("Scaffold"));
        assert!(output.contains("Core"));
    }

    #[test]
    fn render_shows_current_task() {
        let output = render_display(SAMPLE, std::time::Duration::from_secs(0));
        assert!(output.contains("T03"));
        assert!(output.contains("Auth Service"));
    }

    #[test]
    fn render_shows_completion() {
        let output = render_display(SAMPLE, std::time::Duration::from_secs(0));
        assert!(output.contains("2/5"));
        assert!(output.contains("40%"));
    }

    #[test]
    fn render_shows_blocked_count() {
        let output = render_display(SAMPLE, std::time::Duration::from_secs(0));
        assert!(output.contains("Blocked: 1"));
    }

    #[test]
    fn render_shows_learnings() {
        let output = render_display(SAMPLE, std::time::Duration::from_secs(0));
        assert!(output.contains("Learnings: 2"));
    }

    #[test]
    fn render_all_done() {
        let content = r"# done — Implementation Progress

## Phase 1 — Setup

| Task | Status | Notes |
|---|---|---|
| T01 — Init | `[x]` | |

## Accumulated Learnings

> Subagents append discoveries here.

_No learnings yet._
";
        let output = render_display(content, std::time::Duration::from_secs(100));
        assert!(output.contains("All tasks completed!"));
        assert!(output.contains("1/1"));
        assert!(output.contains("100%"));
    }
}
