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
    // opencode root alias — symlink (or copy on non-unix) to the
    // orchestrator agent for clients that scan the working directory
    // instead of `.opencode/agents/`.
    "ORCHESTRATOR.md",
    // VSCode target
    ".vscode/orchestrator.prompt.md",
    ".vscode/evaluator.prompt.md",
    ".vscode/planner.prompt.md",
    ".vscode/background-auditor.prompt.md",
    // opencode target
    ".opencode/agents/wiggum-orchestrator.md",
    ".opencode/agents/wiggum-implementer.md",
    ".opencode/agents/wiggum-evaluator.md",
    ".opencode/agents/wiggum-planner.md",
    ".opencode/agents/wiggum-auditor.md",
    // Claude target
    ".claude/settings.json",
    "CLAUDE.md",
    // agent-rules target — fork-neutral rules files
    ".cursorrules",
    ".windsurfrules",
    ".github/copilot-instructions.md",
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

    // Clean up .opencode/agents/ if empty after removing agent files
    let opencode_agents_dir = project_path.join(".opencode/agents");
    if opencode_agents_dir.is_dir() && is_dir_empty(&opencode_agents_dir) {
        fs::remove_dir(&opencode_agents_dir)?;
        info!("Removed empty directory: {}", opencode_agents_dir.display());
        removed.push(opencode_agents_dir);
    }
    // Clean up .opencode/ if empty after removing agents/
    let opencode_dir = project_path.join(".opencode");
    if opencode_dir.is_dir() && is_dir_empty(&opencode_dir) {
        fs::remove_dir(&opencode_dir)?;
        info!("Removed empty directory: {}", opencode_dir.display());
        removed.push(opencode_dir);
    }

    // Clean up .claude/ if empty after removing settings.json
    let claude_dir = project_path.join(".claude");
    if claude_dir.is_dir() && is_dir_empty(&claude_dir) {
        fs::remove_dir(&claude_dir)?;
        info!("Removed empty directory: {}", claude_dir.display());
        removed.push(claude_dir);
    }

    // Clean up .github/ if empty after removing copilot-instructions.md.
    // We only remove .github/ when it's empty AND the user is running wiggum
    // clean — we don't want to nuke a hand-written workflows/ directory
    // if it happens to coexist.
    let github_dir = project_path.join(".github");
    if github_dir.is_dir() && is_dir_empty(&github_dir) {
        fs::remove_dir(&github_dir)?;
        info!("Removed empty directory: {}", github_dir.display());
        removed.push(github_dir);
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

    #[test]
    fn collect_targets_includes_opencode_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join(".opencode/agents")).unwrap();
        fs::write(root.join(".opencode/agents/wiggum-orchestrator.md"), "orch").unwrap();
        fs::write(root.join(".opencode/agents/wiggum-implementer.md"), "impl").unwrap();

        let plan = sample_plan(&root.to_string_lossy());
        let targets = collect_targets(&plan, root).unwrap();

        assert!(
            targets
                .iter()
                .any(|p| p.ends_with("wiggum-orchestrator.md")),
            "opencode orchestrator must be in clean targets"
        );
        assert!(
            targets.iter().any(|p| p.ends_with("wiggum-implementer.md")),
            "opencode implementer must be in clean targets"
        );
    }

    #[test]
    fn remove_artifacts_deletes_opencode_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join(".opencode/agents")).unwrap();
        fs::write(root.join(".opencode/agents/wiggum-orchestrator.md"), "orch").unwrap();
        fs::write(root.join(".opencode/agents/wiggum-implementer.md"), "impl").unwrap();
        fs::write(root.join(".opencode/agents/wiggum-planner.md"), "planner").unwrap();
        fs::write(root.join(".opencode/agents/wiggum-auditor.md"), "auditor").unwrap();

        let plan = sample_plan(&root.to_string_lossy());
        remove_artifacts(&plan, root).unwrap();

        assert!(
            !root
                .join(".opencode/agents/wiggum-orchestrator.md")
                .exists()
        );
        assert!(!root.join(".opencode/agents/wiggum-implementer.md").exists());
        assert!(!root.join(".opencode/agents/wiggum-planner.md").exists());
        assert!(!root.join(".opencode/agents/wiggum-auditor.md").exists());
        // Both .opencode/agents/ and .opencode/ should be cleaned up since
        // they're empty.
        assert!(!root.join(".opencode/agents").exists());
        assert!(!root.join(".opencode").exists());
    }

    #[test]
    fn remove_artifacts_deletes_root_orchestrator_md_alias() {
        // The root-level ORCHESTRATOR.md (real file or symlink) must also
        // be removed by clean.
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join(".opencode/agents")).unwrap();
        fs::write(root.join(".opencode/agents/wiggum-orchestrator.md"), "orch").unwrap();
        // Simulate the symlink alias written by wiggum generate.
        std::os::unix::fs::symlink(
            ".opencode/agents/wiggum-orchestrator.md",
            root.join("ORCHESTRATOR.md"),
        )
        .unwrap();

        let plan = sample_plan(&root.to_string_lossy());
        remove_artifacts(&plan, root).unwrap();

        assert!(!root.join("ORCHESTRATOR.md").exists());
        assert!(
            !root
                .join(".opencode/agents/wiggum-orchestrator.md")
                .exists()
        );
    }

    #[test]
    fn remove_artifacts_deletes_claude_md_and_empty_github_dir() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // CLAUDE.md at repo root.
        fs::write(root.join("CLAUDE.md"), "# CLAUDE.md\n").unwrap();
        // .claude/settings.json.
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::write(root.join(".claude/settings.json"), "{}").unwrap();
        // agent-rules files.
        fs::write(root.join(".cursorrules"), "rules").unwrap();
        fs::write(root.join(".windsurfrules"), "rules").unwrap();
        fs::create_dir_all(root.join(".github")).unwrap();
        fs::write(
            root.join(".github/copilot-instructions.md"),
            "copilot rules",
        )
        .unwrap();

        let plan = sample_plan(&root.to_string_lossy());
        remove_artifacts(&plan, root).unwrap();

        assert!(!root.join("CLAUDE.md").exists());
        assert!(!root.join(".claude/settings.json").exists());
        assert!(
            !root.join(".claude").exists(),
            ".claude/ should be cleaned up when empty"
        );
        assert!(!root.join(".cursorrules").exists());
        assert!(!root.join(".windsurfrules").exists());
        assert!(
            !root.join(".github/copilot-instructions.md").exists(),
            ".github/copilot-instructions.md must be removed"
        );
        assert!(
            !root.join(".github").exists(),
            ".github/ should be cleaned up when empty (no other contents)"
        );
    }

    #[test]
    fn remove_artifacts_preserves_non_wiggum_github_files() {
        // If the user has hand-written files in .github/ (e.g. workflows/
        // or CODEOWNERS), we must NOT nuke the .github/ directory just
        // because we removed copilot-instructions.md.
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join(".github")).unwrap();
        fs::write(
            root.join(".github/copilot-instructions.md"),
            "copilot rules",
        )
        .unwrap();
        fs::create_dir_all(root.join(".github/workflows")).unwrap();
        fs::write(root.join(".github/workflows/ci.yml"), "name: CI\n").unwrap();

        let plan = sample_plan(&root.to_string_lossy());
        remove_artifacts(&plan, root).unwrap();

        assert!(!root.join(".github/copilot-instructions.md").exists());
        assert!(
            root.join(".github/workflows/ci.yml").exists(),
            "non-wiggum .github/ contents must survive clean"
        );
        assert!(
            root.join(".github").is_dir(),
            ".github/ must survive because it still has workflows/"
        );
    }
}
