//! Resume command implementation — recover interrupted orchestrator loops.

use std::path::Path;

use crate::adapters::fs::FsAdapter;
use crate::domain::plan::Plan;
use crate::domain::report::{TaskStatus, parse_progress};
use crate::error::{Result, WiggumError};
use crate::generation::task;
use crate::ports::PlanReader;

/// Resume context with task information.
pub struct ResumeContext {
    pub task_slug: String,
    pub task_title: String,
    pub task_number: u32,
    pub status: TaskStatus,
    pub prompt: String,
}

/// Find the task to resume from and generate the resume prompt.
///
/// # Errors
///
/// Returns an error if progress file cannot be read or no resumable task found.
pub fn find_resume_task(
    progress_path: &Path,
    plan_path: &Path,
    task_override: Option<&str>,
) -> Result<ResumeContext> {
    let fs = FsAdapter;

    // Read progress file
    let progress_content = std::fs::read_to_string(progress_path).map_err(|e| {
        WiggumError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read {}: {}", progress_path.display(), e),
        ))
    })?;

    let summary = parse_progress(&progress_content);

    // Find the task to resume
    let resume_task = if let Some(slug) = task_override {
        summary
            .tasks
            .iter()
            .find(|t| t.id.ends_with(&format!("-{slug}")) || t.title.to_lowercase().contains(slug))
            .ok_or_else(|| WiggumError::Validation(format!("Task '{slug}' not found")))?
    } else {
        // Auto-detect: find in-progress or last completed
        summary
            .tasks
            .iter()
            .find(|t| t.status == TaskStatus::InProgress)
            .or_else(|| {
                // Find last completed task and resume from the next one
                let last_completed_idx = summary
                    .tasks
                    .iter()
                    .rposition(|t| t.status == TaskStatus::Completed);

                last_completed_idx.and_then(|idx| summary.tasks.get(idx + 1))
            })
            .or_else(|| {
                // Fall back to first not-started task
                summary
                    .tasks
                    .iter()
                    .find(|t| t.status == TaskStatus::NotStarted)
            })
            .ok_or_else(|| {
                WiggumError::Validation(
                    "No resumable task found — all tasks may be completed".into(),
                )
            })?
    };

    // Extract task number from ID (e.g., "T01" -> 1)
    let task_number: u32 = resume_task.id.trim_start_matches('T').parse().unwrap_or(0);

    // Read the plan to get full task details
    let toml_content = fs.read_plan(plan_path)?;
    let plan = Plan::from_toml(&toml_content)?;
    let resolved = plan.resolve_tasks()?;

    let resolved_task = resolved
        .iter()
        .find(|t| t.number == task_number)
        .ok_or_else(|| {
            WiggumError::Validation(format!("Task T{task_number:02} not found in plan"))
        })?;

    // Generate the task prompt with resume preamble
    let base_prompt = task::render(&plan, resolved_task)?;
    let resume_prompt = format!(
        r"## Resume Context

You were mid-execution on this task when the session was interrupted.
Review what was already done (check for partial implementations, uncommitted changes, test files).
Then continue from where you left off.

---

{base_prompt}"
    );

    Ok(ResumeContext {
        task_slug: resolved_task.slug.clone(),
        task_title: resolved_task.title.clone(),
        task_number,
        status: resume_task.status,
        prompt: resume_prompt,
    })
}

/// Format the resume command output for display.
#[must_use]
pub fn format_resume_info(ctx: &ResumeContext, dry_run: bool) -> String {
    let mut lines = Vec::new();

    if dry_run {
        lines.push("Dry run — would resume:\n".to_string());
    } else {
        lines.push("Resuming:\n".to_string());
    }

    lines.push(format!(
        "  Task:   T{:02} — {}",
        ctx.task_number, ctx.task_title
    ));
    lines.push(format!("  Slug:   {}", ctx.task_slug));
    lines.push(format!("  Status: {}", ctx.status));

    if dry_run {
        lines.push(String::new());
        lines.push("Prompt preview (first 500 chars):".to_string());
        lines.push("─".repeat(50));
        let preview: String = ctx.prompt.chars().take(500).collect();
        lines.push(preview);
        if ctx.prompt.len() > 500 {
            lines.push("...".to_string());
        }
    }

    lines.join("\n")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    #[test]
    fn parse_task_number() {
        let id = "T03";
        let num: u32 = id
            .trim_start_matches('T')
            .parse()
            .expect("valid task number");
        assert_eq!(num, 3);
    }
}
