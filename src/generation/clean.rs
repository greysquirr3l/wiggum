use std::fs;
use std::path::{Path, PathBuf};

use tracing::info;

use crate::domain::plan::Plan;
use crate::error::Result;

/// Files and directories that wiggum generates.
const GENERATED_FILES: &[&str] = &[
    "PROGRESS.md",
    "IMPLEMENTATION_PLAN.md",
    "AGENTS.md",
    ".vscode/orchestrator.prompt.md",
];

/// Collect all wiggum-generated paths that exist on disk.
///
/// # Errors
///
/// Returns an error if task resolution fails.
pub fn collect_targets(plan: &Plan, project_path: &Path) -> Result<Vec<PathBuf>> {
    let mut targets = Vec::new();

    for file in GENERATED_FILES {
        let path = project_path.join(file);
        if path.exists() {
            targets.push(path);
        }
    }

    // Collect individual task files from the plan so we only remove
    // files wiggum would have generated — not hand-written files that
    // happen to live in tasks/.
    let resolved = plan.resolve_tasks()?;
    let tasks_dir = project_path.join("tasks");
    if tasks_dir.is_dir() {
        for t in &resolved {
            let filename = format!("T{:02}-{}.md", t.number, t.slug);
            let path = tasks_dir.join(&filename);
            if path.exists() {
                targets.push(path);
            }
        }
    }

    // If the tasks dir is empty after removal, mark it for cleanup too
    targets.push(tasks_dir);

    targets.sort();
    targets.dedup();
    Ok(targets)
}

/// Remove wiggum-generated artifacts from the project directory.
///
/// Returns the list of paths that were actually removed.
///
/// # Errors
///
/// Returns an error if task resolution or file removal fails.
pub fn remove_artifacts(plan: &Plan, project_path: &Path) -> Result<Vec<PathBuf>> {
    let targets = collect_targets(plan, project_path)?;
    let mut removed = Vec::new();

    // Remove files first, then directories (so dirs are empty when we try)
    for path in targets.iter().filter(|p| p.is_file()) {
        fs::remove_file(path)?;
        info!("Removed file: {}", path.display());
        removed.push(path.clone());
    }

    for path in targets.iter().filter(|p| p.is_dir()) {
        if is_dir_empty(path) {
            fs::remove_dir(path)?;
            info!("Removed directory: {}", path.display());
            removed.push(path.clone());
        }
    }

    // Clean up .vscode/ if empty after removing orchestrator.prompt.md
    let vscode_dir = project_path.join(".vscode");
    if vscode_dir.is_dir() && is_dir_empty(&vscode_dir) {
        fs::remove_dir(&vscode_dir)?;
        info!("Removed empty directory: {}", vscode_dir.display());
        removed.push(vscode_dir);
    }

    Ok(removed)
}

fn is_dir_empty(path: &Path) -> bool {
    path.read_dir()
        .is_ok_and(|mut entries| entries.next().is_none())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::plan::Plan;
    use std::fs;
    use tempfile::TempDir;

    fn sample_plan(path: &str) -> Plan {
        // Replace backslashes with forward slashes for cross-platform TOML compatibility
        let normalized_path = path.replace('\\', "/");
        let toml = format!(
            r#"
[project]
name = "test-project"
description = "test"
language = "rust"
path = "{normalized_path}"

[[phases]]
name = "Phase 1"
order = 1

[[phases.tasks]]
slug = "setup"
title = "Project setup"
goal = "Set up the project"
depends_on = []

[[phases.tasks]]
slug = "model"
title = "Domain model"
goal = "Define domain types"
depends_on = ["setup"]
"#
        );
        Plan::from_toml(&toml).unwrap()
    }

    #[test]
    fn collect_targets_finds_existing_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Create the files wiggum would generate
        fs::write(root.join("PROGRESS.md"), "progress").unwrap();
        fs::write(root.join("IMPLEMENTATION_PLAN.md"), "plan").unwrap();
        fs::write(root.join("AGENTS.md"), "agents").unwrap();
        fs::create_dir_all(root.join(".vscode")).unwrap();
        fs::write(root.join(".vscode/orchestrator.prompt.md"), "orch").unwrap();
        fs::create_dir_all(root.join("tasks")).unwrap();
        fs::write(root.join("tasks/T01-setup.md"), "task1").unwrap();
        fs::write(root.join("tasks/T02-model.md"), "task2").unwrap();

        let plan = sample_plan(&root.to_string_lossy());
        let targets = collect_targets(&plan, root).unwrap();

        assert!(targets.iter().any(|p| p.ends_with("PROGRESS.md")));
        assert!(
            targets
                .iter()
                .any(|p| p.ends_with("IMPLEMENTATION_PLAN.md"))
        );
        assert!(targets.iter().any(|p| p.ends_with("AGENTS.md")));
        assert!(
            targets
                .iter()
                .any(|p| p.ends_with("orchestrator.prompt.md"))
        );
        assert!(targets.iter().any(|p| p.ends_with("T01-setup.md")));
        assert!(targets.iter().any(|p| p.ends_with("T02-model.md")));
    }

    #[test]
    fn collect_targets_ignores_missing_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let plan = sample_plan(&root.to_string_lossy());
        let targets = collect_targets(&plan, root).unwrap();

        // Only the tasks/ dir entry (which doesn't exist either)
        // should be present — all file entries are skipped
        assert!(!targets.iter().any(|p| p.is_file()));
    }

    #[test]
    fn remove_artifacts_deletes_generated_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::write(root.join("PROGRESS.md"), "progress").unwrap();
        fs::write(root.join("IMPLEMENTATION_PLAN.md"), "plan").unwrap();
        fs::write(root.join("AGENTS.md"), "agents").unwrap();
        fs::create_dir_all(root.join(".vscode")).unwrap();
        fs::write(root.join(".vscode/orchestrator.prompt.md"), "orch").unwrap();
        fs::create_dir_all(root.join("tasks")).unwrap();
        fs::write(root.join("tasks/T01-setup.md"), "task1").unwrap();
        fs::write(root.join("tasks/T02-model.md"), "task2").unwrap();

        // Also create a non-wiggum file that should survive
        fs::write(root.join("Cargo.toml"), "[package]").unwrap();

        let plan = sample_plan(&root.to_string_lossy());
        let removed = remove_artifacts(&plan, root).unwrap();

        assert!(!root.join("PROGRESS.md").exists());
        assert!(!root.join("IMPLEMENTATION_PLAN.md").exists());
        assert!(!root.join("AGENTS.md").exists());
        assert!(!root.join(".vscode/orchestrator.prompt.md").exists());
        assert!(!root.join(".vscode").exists()); // empty dir cleaned up
        assert!(!root.join("tasks/T01-setup.md").exists());
        assert!(!root.join("tasks/T02-model.md").exists());
        assert!(!root.join("tasks").exists()); // empty dir cleaned up

        // Non-wiggum file survives
        assert!(root.join("Cargo.toml").exists());

        assert!(removed.len() >= 7);
    }

    #[test]
    fn remove_artifacts_preserves_non_wiggum_task_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("tasks")).unwrap();
        fs::write(root.join("tasks/T01-setup.md"), "task1").unwrap();
        fs::write(root.join("tasks/my-custom-notes.md"), "keep me").unwrap();

        let plan = sample_plan(&root.to_string_lossy());
        remove_artifacts(&plan, root).unwrap();

        assert!(!root.join("tasks/T01-setup.md").exists());
        // Custom file survives
        assert!(root.join("tasks/my-custom-notes.md").exists());
        // tasks/ dir survives because it's not empty
        assert!(root.join("tasks").is_dir());
    }

    #[test]
    fn remove_artifacts_preserves_non_wiggum_vscode_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join(".vscode")).unwrap();
        fs::write(root.join(".vscode/orchestrator.prompt.md"), "orch").unwrap();
        fs::write(root.join(".vscode/settings.json"), "{}").unwrap();

        let plan = sample_plan(&root.to_string_lossy());
        remove_artifacts(&plan, root).unwrap();

        assert!(!root.join(".vscode/orchestrator.prompt.md").exists());
        // settings.json survives
        assert!(root.join(".vscode/settings.json").exists());
        // .vscode/ dir survives because it's not empty
        assert!(root.join(".vscode").is_dir());
    }
}
