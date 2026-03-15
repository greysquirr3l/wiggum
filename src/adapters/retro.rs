//! Retro command implementation — generate plan improvement suggestions.

use std::collections::HashMap;
use std::path::Path;

use crate::domain::report::{TaskStatus, parse_progress};
use crate::error::{Result, WiggumError};

/// A retrospective suggestion.
#[derive(Debug, Clone)]
pub struct RetroSuggestion {
    pub pattern: String,
    pub suggestion: String,
    pub task_slug: Option<String>,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Recommendation,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "ℹ️"),
            Self::Warning => write!(f, "⚠️"),
            Self::Recommendation => write!(f, "💡"),
        }
    }
}

/// Retrospective summary.
pub struct RetroSummary {
    pub project_name: String,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub retry_count: usize,
    pub gate_count: usize,
    pub suggestions: Vec<RetroSuggestion>,
}

/// Parse PROGRESS.md and generate retrospective suggestions.
///
/// # Errors
///
/// Returns an error if the progress file cannot be read.
#[allow(clippy::too_many_lines)]
pub fn analyze_progress(progress_path: &Path) -> Result<RetroSummary> {
    let content = std::fs::read_to_string(progress_path).map_err(|e| {
        WiggumError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read {}: {}", progress_path.display(), e),
        ))
    })?;

    let summary = parse_progress(&content);
    let mut suggestions = Vec::new();

    // Pattern: detect retry information from learnings
    let learnings = extract_learnings(&content);
    let mut retry_hints: HashMap<String, Vec<String>> = HashMap::new();

    for (task_id, learning) in &learnings {
        let lower = learning.to_lowercase();

        // Detect retry patterns
        if lower.contains("retry")
            || lower.contains("attempt")
            || lower.contains("failed")
            || lower.contains("fix")
        {
            retry_hints
                .entry(task_id.clone())
                .or_default()
                .push(learning.clone());
        }

        // Detect common issues
        if lower.contains("version mismatch")
            || lower.contains("api changed")
            || lower.contains("breaking change")
        {
            suggestions.push(RetroSuggestion {
                pattern: format!("{task_id} hit version/API issues"),
                suggestion: format!(
                    "Add hint to {task_id}: specify exact versions and link to migration guides."
                ),
                task_slug: Some(task_id.clone()),
                severity: Severity::Recommendation,
            });
        }

        if lower.contains("clippy") || lower.contains("lint") {
            suggestions.push(RetroSuggestion {
                pattern: format!("{task_id} had lint issues"),
                suggestion:
                    "Add a `lint-config` task early in Phase 1 to establish lint rules first."
                        .to_string(),
                task_slug: None,
                severity: Severity::Recommendation,
            });
        }

        if lower.contains("gate") || lower.contains("waiting") || lower.contains("blocked") {
            suggestions.push(RetroSuggestion {
                pattern: format!("{task_id} hit a blocking gate"),
                suggestion:
                    "Gates add latency. Consider if this gate is necessary or if automated preflight is sufficient."
                        .to_string(),
                task_slug: Some(task_id.clone()),
                severity: Severity::Warning,
            });
        }

        if lower.contains("complex") || lower.contains("split") || lower.contains("too large") {
            suggestions.push(RetroSuggestion {
                pattern: format!("{task_id} was too complex"),
                suggestion: format!(
                    "Consider using `wiggum split --task {task_id}` to break it down."
                ),
                task_slug: Some(task_id.clone()),
                severity: Severity::Warning,
            });
        }
    }

    // Generate suggestions for tasks with multiple retries
    for (task_id, hints) in &retry_hints {
        if hints.len() >= 2 {
            suggestions.push(RetroSuggestion {
                pattern: format!("{task_id} required {} attempts", hints.len()),
                suggestion: format!(
                    "Add hints from learnings: {}",
                    hints.iter().take(2).cloned().collect::<Vec<_>>().join("; ")
                ),
                task_slug: Some(task_id.clone()),
                severity: Severity::Recommendation,
            });
        }
    }

    // Check for blocked tasks
    let blocked_count = summary
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Blocked)
        .count();
    if blocked_count > 0 {
        suggestions.push(RetroSuggestion {
            pattern: format!("{blocked_count} task(s) currently blocked"),
            suggestion: "Review blocked tasks and consider adjusting dependencies or adding hints."
                .to_string(),
            task_slug: None,
            severity: Severity::Warning,
        });
    }

    let completed = summary
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Completed)
        .count();

    let gates = summary
        .tasks
        .iter()
        .filter(|t| t.title.to_lowercase().contains("gate"))
        .count();

    Ok(RetroSummary {
        project_name: summary.project_name,
        total_tasks: summary.tasks.len(),
        completed_tasks: completed,
        retry_count: retry_hints.values().map(Vec::len).sum(),
        gate_count: gates,
        suggestions,
    })
}

/// Extract learnings from PROGRESS.md content.
fn extract_learnings(content: &str) -> Vec<(String, String)> {
    let mut learnings = Vec::new();

    // Parse task rows with learnings: "| T01 — Title | `[x]` | Learning text |"
    for line in content.lines() {
        if line.starts_with("| T") && line.contains('—') {
            let cols: Vec<&str> = line.split('|').collect();
            if let (Some(task_col), Some(learning_col)) = (cols.get(1), cols.get(3)) {
                let task_col = task_col.trim();
                if let Some((id, _)) = task_col.split_once(" — ") {
                    let task_id = id.trim().to_string();
                    let learning = learning_col.trim();
                    if !learning.is_empty() && learning != "_—_" {
                        learnings.push((task_id, learning.to_string()));
                    }
                }
            }
        }
    }

    // Also parse accumulated learnings section
    if let Some(section) = content.split("## Accumulated Learnings").nth(1) {
        for line in section.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                let learning = trimmed.trim_start_matches("- ").trim_start_matches("* ");
                if !learning.is_empty() && learning != "_No learnings yet._" {
                    // Try to extract task ID if mentioned
                    let task_id = learning
                        .find("T0")
                        .and_then(|idx| learning.get(idx..))
                        .and_then(|s| s.split_whitespace().next())
                        .unwrap_or("General");
                    learnings.push((task_id.to_string(), learning.to_string()));
                }
            }
        }
    }

    learnings
}

/// Format retrospective for display.
#[must_use]
pub fn format_retro(summary: &RetroSummary) -> String {
    let mut lines = Vec::new();

    lines.push(format!(
        "Retrospective — {} ({} tasks, {} retries, {} gates)",
        summary.project_name, summary.total_tasks, summary.retry_count, summary.gate_count
    ));
    lines.push(String::new());

    if summary.suggestions.is_empty() {
        lines.push("No improvement suggestions detected.".to_string());
        lines.push("Learnings were clean — great execution!".to_string());
    } else {
        for suggestion in &summary.suggestions {
            lines.push(format!(
                "{} Pattern: {}",
                suggestion.severity, suggestion.pattern
            ));
            lines.push(format!("  Suggestion: {}", suggestion.suggestion));
            lines.push(String::new());
        }
    }

    lines.join("\n")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn extract_learnings_basic() {
        let content = r"# Project — status
## Phase 1 — Setup
| Task | Status | Learnings |
|------|--------|-----------|  
| T01 — Setup | `[x]` | Had to retry due to version mismatch |
| T02 — Config | `[ ]` | _—_ |
";
        let learnings = extract_learnings(content);
        assert_eq!(learnings.len(), 1);
        assert!(
            learnings
                .first()
                .expect("should have learning")
                .1
                .contains("version mismatch")
        );
    }
}
