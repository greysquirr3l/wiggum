//! `replan` command — re-generate a single task file after a failure.
//!
//! Reads the current plan and PROGRESS.md, extracts failure evidence for the
//! named task, then re-renders the task's `.md` file with augmented hints.

use std::path::Path;

use crate::domain::plan::Plan;
use crate::error::{Result, WiggumError};
use crate::generation::task as task_gen;
use crate::generation::templates::get_tera;

/// Re-render a single task file, injecting any failure evidence from PROGRESS.md
/// as additional hints.
///
/// When `dry_run` is `true`, prints the new content to stdout instead of writing
/// to disk.
///
/// # Errors
///
/// Returns an error if the plan cannot be parsed, the task slug is not found,
/// or the template rendering fails.
pub fn run_replan(plan_path: &Path, task_slug: &str, dry_run: bool) -> Result<()> {
    let toml_content = std::fs::read_to_string(plan_path).map_err(|e| {
        WiggumError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read {}: {e}", plan_path.display()),
        ))
    })?;
    let mut plan = Plan::from_toml(&toml_content)?;

    let resolved = plan.resolve_tasks()?;
    let task_idx = resolved
        .iter()
        .position(|t| t.slug == task_slug)
        .ok_or_else(|| {
            WiggumError::Validation(format!("task slug '{task_slug}' not found in plan"))
        })?;

    // Extract failure evidence from PROGRESS.md if it exists alongside the plan.
    let evidence = extract_failure_evidence(plan_path, task_slug);

    // Augment task hints in-place on the mutable plan before re-rendering.
    // We find the task across phases and append the extracted hints.
    if !evidence.is_empty() {
        for phase in &mut plan.phases {
            for task_def in &mut phase.tasks {
                if task_def.slug == task_slug {
                    for item in &evidence {
                        let hint = format!("[Previous failure] {item}");
                        if !task_def.hints.contains(&hint) {
                            task_def.hints.push(hint);
                        }
                    }
                    break;
                }
            }
        }
    }

    // Re-resolve after augmentation
    let resolved = plan.resolve_tasks()?;
    let task = resolved.get(task_idx).ok_or_else(|| {
        WiggumError::Validation(format!("task index {task_idx} out of range after replan"))
    })?;

    let tera = get_tera();
    let content = task_gen::render_with(tera, &plan, task)?;

    if dry_run {
        println!("--- Replan dry-run for task: {task_slug} ---\n");
        println!("{content}");
        return Ok(());
    }

    // Determine the tasks directory relative to the plan file
    let plan_dir = plan_path.parent().unwrap_or_else(|| Path::new("."));
    let tasks_dir = plan_dir.join("tasks");
    if !tasks_dir.is_dir() {
        return Err(WiggumError::Validation(format!(
            "tasks directory not found at {}",
            tasks_dir.display()
        )));
    }

    let filename = format!("T{:02}-{}.md", task.number, task.slug);
    let out_path = tasks_dir.join(&filename);
    std::fs::write(&out_path, &content).map_err(|e| {
        WiggumError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to write {}: {e}", out_path.display()),
        ))
    })?;

    println!("✅ Replanned task {task_slug} → {}", out_path.display());
    if !evidence.is_empty() {
        println!(
            "   Injected {} failure hint(s) from PROGRESS.md.",
            evidence.len()
        );
    }

    Ok(())
}

/// Extract failure evidence lines for a task from PROGRESS.md.
///
/// Looks for lines in PROGRESS.md under the section for `task_slug` that
/// contain `Required fix`, `FAIL`, or `failure` keywords.
fn extract_failure_evidence(plan_path: &Path, task_slug: &str) -> Vec<String> {
    let plan_dir = plan_path.parent().unwrap_or_else(|| Path::new("."));
    let progress_path = plan_dir.join("PROGRESS.md");

    let Ok(content) = std::fs::read_to_string(&progress_path) else {
        return Vec::new();
    };

    let mut evidence = Vec::new();
    let mut in_task_section = false;

    for line in content.lines() {
        // Detect task section header
        if line.contains(task_slug) && (line.starts_with('#') || line.starts_with('-')) {
            in_task_section = true;
            continue;
        }

        // Exit section on next same-level header
        if in_task_section && line.starts_with("## ") && !line.contains(task_slug) {
            in_task_section = false;
        }

        if in_task_section {
            let lower = line.to_lowercase();
            if lower.contains("required fix")
                || lower.contains("fail")
                || lower.contains("failure")
                || lower.contains("error")
                || lower.contains("must fix")
            {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    evidence.push(trimmed.trim_start_matches('-').trim().to_string());
                }
            }
        }
    }

    evidence
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_plan(dir: &TempDir) -> std::path::PathBuf {
        let plan = r#"
[project]
name = "replan-test"
path = "/tmp/replan-test"
description = "Replan test"
language = "rust"

[[phases]]
name = "Phase 1"
order = 1

[[phases.tasks]]
slug = "first-task"
title = "First task"
goal = "Implement the first feature"
hints = ["Check the docs"]
"#;
        let path = dir.path().join("plan.toml");
        std::fs::write(&path, plan).unwrap();

        // Create tasks directory
        let tasks_dir = dir.path().join("tasks");
        std::fs::create_dir_all(&tasks_dir).unwrap();

        // Pre-write the task file
        std::fs::write(
            tasks_dir.join("T01-first-task.md"),
            "# T01 — First task\n\n## Goal\n\nImplement the first feature\n",
        )
        .unwrap();

        path
    }

    #[test]
    fn dry_run_does_not_write_file() {
        let dir = TempDir::new().unwrap();
        let plan_path = write_plan(&dir);

        // Dry run should succeed and not modify the file
        run_replan(&plan_path, "first-task", true).unwrap();
    }

    #[test]
    fn unknown_slug_returns_error() {
        let dir = TempDir::new().unwrap();
        let plan_path = write_plan(&dir);
        let result = run_replan(&plan_path, "nonexistent-slug", true);
        assert!(result.is_err(), "expected error for unknown slug");
    }

    #[test]
    fn apply_writes_task_file() {
        let dir = TempDir::new().unwrap();
        let plan_path = write_plan(&dir);
        run_replan(&plan_path, "first-task", false).unwrap();

        let task_file = dir.path().join("tasks").join("T01-first-task.md");
        assert!(task_file.exists(), "task file should be written");
        let content = std::fs::read_to_string(task_file).unwrap();
        assert!(
            content.contains("First task"),
            "task file should contain title"
        );
    }
}
