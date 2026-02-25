use std::fmt;
use std::path::Path;
use std::process::Command;

/// Status of a single task parsed from PROGRESS.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    NotStarted,
    InProgress,
    Completed,
    Blocked,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotStarted => write!(f, "not started"),
            Self::InProgress => write!(f, "in progress"),
            Self::Completed => write!(f, "completed"),
            Self::Blocked => write!(f, "blocked"),
        }
    }
}

/// A single task entry parsed from PROGRESS.md.
#[derive(Debug, Clone)]
pub struct TaskEntry {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
}

/// A phase with its name and constituent task indices.
#[derive(Debug, Clone)]
pub struct PhaseInfo {
    pub name: String,
    pub task_ids: Vec<String>,
}

/// Summary of progress parsed from PROGRESS.md.
#[derive(Debug)]
pub struct ProgressSummary {
    pub project_name: String,
    pub tasks: Vec<TaskEntry>,
    pub phases: Vec<PhaseInfo>,
    pub learnings_count: usize,
    pub phases_total: usize,
    pub phases_completed: usize,
}

/// Full report combining progress and optional git timeline data.
pub struct Report {
    pub summary: ProgressSummary,
    pub timeline: Vec<GitCommitInfo>,
}

/// A git commit relevant to a task.
#[derive(Debug, Clone)]
pub struct GitCommitInfo {
    pub task_id: String,
    pub timestamp: String,
    pub message: String,
}

/// Parse a PROGRESS.md file content into a [`ProgressSummary`].
#[must_use]
pub fn parse_progress(content: &str) -> ProgressSummary {
    let project_name = content
        .lines()
        .next()
        .and_then(|l| l.strip_prefix("# "))
        .and_then(|l| l.split(" — ").next())
        .unwrap_or("unknown")
        .to_string();

    let mut tasks = Vec::new();
    let mut phase_task_map: Vec<(String, Vec<TaskStatus>)> = Vec::new();
    let mut phase_infos: Vec<PhaseInfo> = Vec::new();
    let mut current_phase_idx: Option<usize> = None;

    for line in content.lines() {
        // Detect phase headers: "## Phase 1 — Scaffold"
        if let Some(rest) = line.strip_prefix("## Phase ")
            && let Some(name) = rest.split(" \u{2014} ").nth(1)
        {
            phase_task_map.push((name.to_string(), Vec::new()));
            phase_infos.push(PhaseInfo {
                name: name.to_string(),
                task_ids: Vec::new(),
            });
            current_phase_idx = Some(phase_task_map.len() - 1);
        }

        // Parse task rows: "| T01 — Workspace Scaffold | `[x]` | notes |"
        if line.starts_with("| T") && line.contains('—') {
            let status = if line.contains("`[x]`") {
                TaskStatus::Completed
            } else if line.contains("`[~]`") {
                TaskStatus::InProgress
            } else if line.contains("`[!]`") {
                TaskStatus::Blocked
            } else {
                TaskStatus::NotStarted
            };

            // Extract "T01" and "Workspace Scaffold"
            let cols: Vec<&str> = line.split('|').collect();
            if let Some(task_col) = cols.get(1) {
                let task_col = task_col.trim();
                if let Some((id, title)) = task_col.split_once(" — ") {
                    let task_id = id.trim().to_string();
                    tasks.push(TaskEntry {
                        id: task_id.clone(),
                        title: title.trim().to_string(),
                        status,
                    });

                    if let Some(idx) = current_phase_idx {
                        if let Some((_, statuses)) = phase_task_map.get_mut(idx) {
                            statuses.push(status);
                        }
                        if let Some(phase) = phase_infos.get_mut(idx) {
                            phase.task_ids.push(task_id);
                        }
                    }
                }
            }
        }
    }

    let phases_total = phase_task_map.len();
    let phases_completed = phase_task_map
        .iter()
        .filter(|(_, statuses)| {
            !statuses.is_empty() && statuses.iter().all(|s| *s == TaskStatus::Completed)
        })
        .count();

    // Count learnings (non-empty lines after "## Accumulated Learnings" that aren't
    // the placeholder or header/blockquote lines)
    let learnings_count = content
        .split("## Accumulated Learnings")
        .nth(1)
        .map_or(0, |section| {
            section
                .lines()
                .filter(|l| {
                    let trimmed = l.trim();
                    !trimmed.is_empty()
                        && !trimmed.starts_with('>')
                        && !trimmed.starts_with('#')
                        && trimmed != "_No learnings yet._"
                        && trimmed != "---"
                })
                .count()
        });

    ProgressSummary {
        project_name,
        tasks,
        phases: phase_infos,
        learnings_count,
        phases_total,
        phases_completed,
    }
}

/// Collect git commits that mention task IDs (e.g. "T01", "T02") from the
/// project directory. Returns an empty Vec if git is unavailable.
#[must_use]
pub fn collect_git_timeline(project_path: &Path) -> Vec<GitCommitInfo> {
    let output = Command::new("git")
        .args(["log", "--oneline", "--format=%aI %s", "--reverse"])
        .current_dir(project_path)
        .output();

    let Ok(out) = output else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut commits = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Format: "2026-02-24T14:02:00-05:00 T01: scaffold workspace"
        let (timestamp, message) = trimmed.split_at(trimmed.find(' ').unwrap_or(trimmed.len()));
        let message = message.trim();

        // Check if the commit message references a task ID like T01, T02, etc.
        for word in message.split_whitespace() {
            let upper = word.trim_end_matches(':').to_uppercase();
            if upper.starts_with('T')
                && upper.len() >= 3
                && upper[1..].chars().take(2).all(|c| c.is_ascii_digit())
            {
                commits.push(GitCommitInfo {
                    task_id: upper.clone(),
                    timestamp: timestamp.to_string(),
                    message: message.to_string(),
                });
                break;
            }
        }
    }

    commits
}

/// Generate a full report from a PROGRESS.md and optional project path.
#[must_use]
pub fn generate_report(progress_content: &str, project_path: Option<&Path>) -> Report {
    let summary = parse_progress(progress_content);
    let timeline = project_path.map_or_else(Vec::new, collect_git_timeline);
    Report { summary, timeline }
}

/// Format a [`Report`] as a human-readable string.
#[must_use]
pub fn format_report(report: &Report) -> String {
    let s = &report.summary;
    let mut lines = Vec::new();

    lines.push(format!("# Execution Report — {}\n", s.project_name));

    // Summary
    let completed = s
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Completed)
        .count();
    let blocked = s
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Blocked)
        .count();
    let in_progress = s
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::InProgress)
        .count();
    let total = s.tasks.len();

    lines.push("## Summary".to_string());
    lines.push(format!("- Tasks: {completed}/{total} completed"));
    if in_progress > 0 {
        lines.push(format!("- In progress: {in_progress}"));
    }
    if blocked > 0 {
        lines.push(format!("- Blocked: {blocked}"));
    }
    lines.push(format!(
        "- Phases: {}/{} completed",
        s.phases_completed, s.phases_total
    ));

    // Per-task status
    lines.push(String::new());
    lines.push("## Task Status".to_string());
    for task in &s.tasks {
        let icon = match task.status {
            TaskStatus::Completed => "✅",
            TaskStatus::InProgress => "🔄",
            TaskStatus::Blocked => "❌",
            TaskStatus::NotStarted => "⬜",
        };
        lines.push(format!("  {icon} {} — {}", task.id, task.title));
    }

    // Timeline (from git)
    if !report.timeline.is_empty() {
        lines.push(String::new());
        lines.push("## Timeline (from git log)".to_string());
        for commit in &report.timeline {
            lines.push(format!(
                "  {} {}  {}",
                commit.task_id, commit.timestamp, commit.message
            ));
        }
    }

    // Learnings
    lines.push(String::new());
    lines.push("## Learnings".to_string());
    if s.learnings_count > 0 {
        lines.push(format!("  {} learnings captured", s.learnings_count));
    } else {
        lines.push("  No learnings captured yet.".to_string());
    }

    lines.join("\n")
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    const SAMPLE_PROGRESS: &str = r"# my-project — Implementation Progress

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
    fn parse_project_name() {
        let summary = parse_progress(SAMPLE_PROGRESS);
        assert_eq!(summary.project_name, "my-project");
    }

    #[test]
    fn parse_task_count() {
        let summary = parse_progress(SAMPLE_PROGRESS);
        assert_eq!(summary.tasks.len(), 5);
    }

    #[test]
    fn parse_task_statuses() {
        let summary = parse_progress(SAMPLE_PROGRESS);
        assert_eq!(summary.tasks[0].status, TaskStatus::Completed);
        assert_eq!(summary.tasks[0].id, "T01");
        assert_eq!(summary.tasks[2].status, TaskStatus::InProgress);
        assert_eq!(summary.tasks[3].status, TaskStatus::NotStarted);
        assert_eq!(summary.tasks[4].status, TaskStatus::Blocked);
    }

    #[test]
    fn parse_phases() {
        let summary = parse_progress(SAMPLE_PROGRESS);
        assert_eq!(summary.phases_total, 2);
        assert_eq!(summary.phases_completed, 1); // Only phase 1 all [x]
    }

    #[test]
    fn parse_learnings_count() {
        let summary = parse_progress(SAMPLE_PROGRESS);
        assert_eq!(summary.learnings_count, 2);
    }

    #[test]
    fn parse_no_learnings() {
        let content = r"# test — Implementation Progress

## Phase 1 — Setup

| Task | Status | Notes |
|---|---|---|
| T01 — Init | `[ ]` | |

## Accumulated Learnings

> Subagents append discoveries here.

_No learnings yet._
";
        let summary = parse_progress(content);
        assert_eq!(summary.learnings_count, 0);
    }

    #[test]
    fn format_report_contains_key_sections() {
        let summary = parse_progress(SAMPLE_PROGRESS);
        let report = Report {
            summary,
            timeline: vec![],
        };
        let output = format_report(&report);
        assert!(output.contains("# Execution Report — my-project"));
        assert!(output.contains("Tasks: 2/5 completed"));
        assert!(output.contains("Blocked: 1"));
        assert!(output.contains("In progress: 1"));
        assert!(output.contains("Phases: 1/2 completed"));
        assert!(output.contains("2 learnings captured"));
        assert!(output.contains("✅ T01 — Workspace Setup"));
        assert!(output.contains("❌ T05 — Integration"));
    }
}
