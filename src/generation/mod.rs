pub mod agent_rules;
pub mod agents_md;
pub mod background_auditor;
pub mod claude;
pub mod clean;
pub mod evaluator;
pub mod features;
pub mod hooks;
pub mod orchestrator;
pub mod plan_doc;
pub mod planner;
pub mod progress;
pub mod task;
pub(crate) mod templates;
pub mod tokens;

use std::path::Path;

use crate::domain::dag::validate_dag;
use crate::domain::plan::Plan;
use crate::domain::targets::TargetSet;
use crate::error::Result;
use crate::ports::ArtifactWriter;

/// Generated artifacts from a plan.
///
/// Each tool target owns its own render slot. The CLI / `write_artifacts` uses
/// `TargetSet` to decide which slots are written. Universal artifacts
/// (`progress`, `plan_doc`, `tasks`, `agents_md`, `features_json`) are always
/// rendered and always written.
pub struct GeneratedArtifacts {
    pub progress: String,
    pub plan_doc: String,
    pub tasks: Vec<(String, String)>, // (filename, content)
    pub agents_md: Option<String>,
    /// Structured JSON task feature/criteria registry (`features.json`).
    pub features_json: String,

    // ── VSCode target ───────────────────────────────────────────────────
    /// `.vscode/orchestrator.prompt.md`. Always rendered for the vscode target.
    pub orchestrator_vscode: String,
    /// `.vscode/evaluator.prompt.md`. Present only when `[evaluator]` is
    /// configured AND the vscode target is selected.
    pub evaluator_vscode: Option<String>,
    /// `.vscode/planner.prompt.md`.
    pub planner_vscode: String,
    /// `.vscode/background-auditor.prompt.md`.
    pub background_auditor_vscode: String,

    // ── opencode target ────────────────────────────────────────────────
    /// `.opencode/agents/wiggum-orchestrator.md`. Always rendered for the
    /// opencode target.
    pub orchestrator_opencode: String,
    /// `.opencode/agents/wiggum-implementer.md`. Shared body — the
    /// orchestrator references the task file via `@path` at dispatch time.
    pub implementer: String,
    /// `.opencode/agents/wiggum-evaluator.md`. Present only when
    /// `[evaluator]` is configured AND the opencode target is selected.
    pub evaluator_opencode: Option<String>,
    /// `.opencode/agents/wiggum-planner.md`.
    pub planner_opencode: String,
    /// `.opencode/agents/wiggum-auditor.md`.
    pub background_auditor_opencode: String,

    // ── Claude target ──────────────────────────────────────────────────
    /// Claude hooks configuration (`.claude/settings.json`).
    pub hooks_json: String,
    /// Claude Code project memory (`CLAUDE.md`) at the repository root.
    pub claude_md: String,

    // ── agent-rules target ───────────────────────────────────────────
    /// `.cursorrules` — Cursor's project-level rules file.
    pub agent_rules_cursorrules: String,
    /// `.windsurfrules` — Windsurf's project-level rules file.
    pub agent_rules_windsurfrules: String,
    /// `.github/copilot-instructions.md` — GitHub Copilot repo-level
    /// instructions (also picked up by some `VSCode` forks).
    pub agent_rules_copilot_instructions: String,
}

impl GeneratedArtifacts {
    /// The full set of targets this artifact batch supports.
    /// Every target is always rendered — `write_artifacts` filters.
    #[must_use]
    pub const fn all_targets() -> TargetSet {
        TargetSet::all()
    }
}

/// Generate all artifacts from a plan (pure — returns strings, no I/O).
///
/// # Errors
///
/// Returns an error if DAG validation or template rendering fails.
pub fn generate_all(plan: &Plan) -> Result<GeneratedArtifacts> {
    let resolved = plan.resolve_tasks()?;
    validate_dag(&resolved)?;

    let progress = progress::render(plan, &resolved)?;
    let orchestrator_vscode = orchestrator::render(plan, &resolved)?;
    let orchestrator_opencode = orchestrator::render_opencode(plan, &resolved)?;
    let implementer = orchestrator::render_implementer(plan)?;
    let plan_doc = plan_doc::render(plan, &resolved)?;

    let mut tasks = Vec::new();
    for t in &resolved {
        let filename = format!("T{:02}-{}.md", t.number, t.slug);
        let content = task::render(plan, t)?;
        tasks.push((filename, content));
    }

    let agents_md = Some(agents_md::render(plan)?);
    let features_json = features::render(plan, &resolved)?;
    let evaluator_vscode = evaluator::render(plan, &resolved)?;
    let evaluator_opencode = evaluator::render_opencode(plan, &resolved)?;
    let planner_vscode = planner::render(plan)?;
    let planner_opencode = planner::render_opencode(plan)?;
    let background_auditor_vscode = background_auditor::render(plan)?;
    let background_auditor_opencode = background_auditor::render_opencode(plan)?;
    let hooks_json = hooks::render().to_string();
    let claude_md = claude::render(plan)?;
    let agent_rules_content = agent_rules::render(plan)?;

    Ok(GeneratedArtifacts {
        progress,
        plan_doc,
        tasks,
        agents_md,
        features_json,
        orchestrator_vscode,
        evaluator_vscode,
        planner_vscode,
        background_auditor_vscode,
        orchestrator_opencode,
        implementer,
        evaluator_opencode,
        planner_opencode,
        background_auditor_opencode,
        hooks_json,
        claude_md,
        agent_rules_cursorrules: agent_rules_content.clone(),
        agent_rules_windsurfrules: agent_rules_content.clone(),
        agent_rules_copilot_instructions: agent_rules_content,
    })
}

/// Generate all artifacts using user template overrides from the project directory.
/// Looks for `.wiggum/templates/` in `project_path`.
///
/// # Errors
///
/// Returns an error if DAG validation, template loading, or rendering fails.
pub fn generate_all_with_overrides(plan: &Plan, project_path: &Path) -> Result<GeneratedArtifacts> {
    let resolved = plan.resolve_tasks()?;
    validate_dag(&resolved)?;

    let tera = templates::get_tera_with_overrides(project_path)?;

    let progress = progress::render_with(&tera, plan, &resolved)?;
    let orchestrator_vscode = orchestrator::render_with(&tera, plan, &resolved)?;
    let orchestrator_opencode = orchestrator::render_opencode_with(&tera, plan, &resolved)?;
    let implementer = orchestrator::render_implementer_with(&tera, plan)?;
    let plan_doc = plan_doc::render_with(&tera, plan, &resolved)?;

    let mut tasks = Vec::new();
    for t in &resolved {
        let filename = format!("T{:02}-{}.md", t.number, t.slug);
        let content = task::render_with(&tera, plan, t)?;
        tasks.push((filename, content));
    }

    let agents_md = Some(agents_md::render_with(&tera, plan)?);
    let features_json = features::render(plan, &resolved)?;
    let evaluator_vscode = evaluator::render_with(&tera, plan, &resolved)?;
    let evaluator_opencode = evaluator::render_opencode_with(&tera, plan, &resolved)?;
    let planner_vscode = planner::render_with(&tera, plan)?;
    let planner_opencode = planner::render_opencode_with(&tera, plan)?;
    let background_auditor_vscode = background_auditor::render_with(&tera, plan)?;
    let background_auditor_opencode = background_auditor::render_opencode_with(&tera, plan)?;
    let hooks_json = hooks::render().to_string();
    let claude_md = claude::render_with(&tera, plan)?;
    let agent_rules_content = agent_rules::render_with(&tera, plan)?;

    Ok(GeneratedArtifacts {
        progress,
        plan_doc,
        tasks,
        agents_md,
        features_json,
        orchestrator_vscode,
        evaluator_vscode,
        planner_vscode,
        background_auditor_vscode,
        orchestrator_opencode,
        implementer,
        evaluator_opencode,
        planner_opencode,
        background_auditor_opencode,
        hooks_json,
        claude_md,
        agent_rules_cursorrules: agent_rules_content.clone(),
        agent_rules_windsurfrules: agent_rules_content.clone(),
        agent_rules_copilot_instructions: agent_rules_content,
    })
}

/// Write all generated artifacts to the target project directory, restricted
/// to the enabled targets in `targets`.
///
/// # Errors
///
/// Returns an error if any file or directory write operation fails.
pub fn write_artifacts(
    writer: &dyn ArtifactWriter,
    project_path: &Path,
    artifacts: &GeneratedArtifacts,
    targets: &TargetSet,
) -> Result<()> {
    use crate::domain::targets::Target;

    // Universal artifacts — always written.
    writer.write_file(&project_path.join("PROGRESS.md"), &artifacts.progress)?;
    writer.write_file(
        &project_path.join("IMPLEMENTATION_PLAN.md"),
        &artifacts.plan_doc,
    )?;

    let tasks_dir = project_path.join("tasks");
    writer.ensure_dir(&tasks_dir)?;
    for (filename, content) in &artifacts.tasks {
        writer.write_file(&tasks_dir.join(filename), content)?;
    }

    if let Some(agents_md) = &artifacts.agents_md {
        writer.write_file(&project_path.join("AGENTS.md"), agents_md)?;
    }

    writer.write_file(
        &project_path.join("features.json"),
        &artifacts.features_json,
    )?;

    // VSCode target.
    if targets.contains(Target::Vscode) {
        let vscode_dir = project_path.join(".vscode");
        writer.ensure_dir(&vscode_dir)?;
        writer.write_file(
            &vscode_dir.join("orchestrator.prompt.md"),
            &artifacts.orchestrator_vscode,
        )?;
        writer.write_file(
            &vscode_dir.join("planner.prompt.md"),
            &artifacts.planner_vscode,
        )?;
        writer.write_file(
            &vscode_dir.join("background-auditor.prompt.md"),
            &artifacts.background_auditor_vscode,
        )?;
        if let Some(eval) = &artifacts.evaluator_vscode {
            writer.write_file(&vscode_dir.join("evaluator.prompt.md"), eval)?;
        }
    }

    // opencode target.
    if targets.contains(Target::Opencode) {
        let opencode_dir = project_path.join(".opencode/agents");
        writer.ensure_dir(&opencode_dir)?;
        writer.write_file(
            &opencode_dir.join("wiggum-orchestrator.md"),
            &artifacts.orchestrator_opencode,
        )?;
        writer.write_file(
            &opencode_dir.join("wiggum-implementer.md"),
            &artifacts.implementer,
        )?;
        writer.write_file(
            &opencode_dir.join("wiggum-planner.md"),
            &artifacts.planner_opencode,
        )?;
        writer.write_file(
            &opencode_dir.join("wiggum-auditor.md"),
            &artifacts.background_auditor_opencode,
        )?;
        if let Some(eval) = &artifacts.evaluator_opencode {
            writer.write_file(&opencode_dir.join("wiggum-evaluator.md"), eval)?;
        }

        // Some opencode-compatible clients (e.g. minimax-m3) look for the
        // orchestrator agent at the project root under the conventional
        // name `ORCHESTRATOR.md` rather than scanning `.opencode/agents/`.
        // Write a symlink so the two paths share content; if symlinks are
        // not supported on this platform, fall back to a regular file copy.
        let root_link = project_path.join("ORCHESTRATOR.md");
        let target_rel = std::path::Path::new(".opencode/agents/wiggum-orchestrator.md");
        write_root_orchestrator_link(&root_link, target_rel, &artifacts.orchestrator_opencode)?;
    }

    // Claude target.
    if targets.contains(Target::Claude) {
        let claude_dir = project_path.join(".claude");
        writer.ensure_dir(&claude_dir)?;
        writer.write_file(&claude_dir.join("settings.json"), &artifacts.hooks_json)?;
        // CLAUDE.md lives at the repo root — Claude Code reads it from
        // there on every session.
        writer.write_file(&project_path.join("CLAUDE.md"), &artifacts.claude_md)?;
    }

    // agent-rules target — fork-neutral rules files for VSCode forks that
    // don't speak the Copilot `runSubagent` or opencode `task` protocols.
    if targets.contains(Target::AgentRules) {
        writer.write_file(
            &project_path.join(".cursorrules"),
            &artifacts.agent_rules_cursorrules,
        )?;
        writer.write_file(
            &project_path.join(".windsurfrules"),
            &artifacts.agent_rules_windsurfrules,
        )?;
        let github_dir = project_path.join(".github");
        writer.ensure_dir(&github_dir)?;
        writer.write_file(
            &github_dir.join("copilot-instructions.md"),
            &artifacts.agent_rules_copilot_instructions,
        )?;
    }

    Ok(())
}

/// Write `ORCHESTRATOR.md` at the project root for opencode-compatible
/// clients that scan the working directory instead of `.opencode/agents/`.
///
/// Tries to create a symlink to `.opencode/agents/wiggum-orchestrator.md`
/// first so the two paths always share content. If symlinks are not
/// supported on this platform (or symlinking fails for any other reason),
/// falls back to a regular file copy.
fn write_root_orchestrator_link(root_link: &Path, target_rel: &Path, content: &str) -> Result<()> {
    use std::fs;
    use std::io::Write;

    // If something already exists at this path (a real file from a previous
    // generate, a dangling symlink from before the target existed), remove
    // it so the new symlink / copy can take its place.
    match fs::symlink_metadata(root_link) {
        Ok(_) => {
            fs::remove_file(root_link)?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }

    #[cfg(unix)]
    {
        if std::os::unix::fs::symlink(target_rel, root_link).is_ok() {
            return Ok(());
        }
        // Fall through to the file-copy fallback.
    }

    let mut f = fs::File::create(root_link)?;
    f.write_all(content.as_bytes())?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;

    use super::write_root_orchestrator_link;

    fn make_target(tmp: &TempDir) -> std::path::PathBuf {
        let target = tmp.path().join(".opencode/agents/wiggum-orchestrator.md");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "ORCH BODY").unwrap();
        target
    }

    #[test]
    fn write_root_orchestrator_link_writes_symlink_or_copy_on_disk() {
        // Exercise the helper against a real tempdir so we cover the
        // symlink path on unix and the file-copy fallback otherwise.
        let tmp = TempDir::new().unwrap();
        let root_link = tmp.path().join("ORCHESTRATOR.md");
        let target_rel = Path::new(".opencode/agents/wiggum-orchestrator.md");
        make_target(&tmp);

        write_root_orchestrator_link(&root_link, target_rel, "ORCH BODY").unwrap();

        // The link path exists in some form (symlink on unix, regular file
        // on the file-copy fallback).
        let md = fs::symlink_metadata(&root_link)
            .unwrap_or_else(|e| panic!("ORCHESTRATOR.md must exist after link: {e}"));
        assert!(
            md.file_type().is_symlink() || md.file_type().is_file(),
            "ORCHESTRATOR.md must be a symlink or regular file"
        );

        #[cfg(unix)]
        {
            assert!(
                md.file_type().is_symlink(),
                "on unix, ORCHESTRATOR.md must be a symlink"
            );
            let resolved = fs::read_link(&root_link).unwrap();
            assert_eq!(resolved, target_rel);
        }

        // Content is reachable through the link (or copy).
        let body = fs::read_to_string(&root_link).unwrap();
        assert_eq!(body, "ORCH BODY");
    }

    #[test]
    fn write_root_orchestrator_link_replaces_existing_file() {
        let tmp = TempDir::new().unwrap();
        let root_link = tmp.path().join("ORCHESTRATOR.md");
        fs::write(&root_link, "stale content").unwrap();
        make_target(&tmp);

        let target_rel = Path::new(".opencode/agents/wiggum-orchestrator.md");
        write_root_orchestrator_link(&root_link, target_rel, "fresh").unwrap();

        // After the call the existing entry has been replaced — either by
        // a fresh symlink to the canonical target or by a fresh file copy.
        let md = fs::symlink_metadata(&root_link).unwrap();
        assert!(
            md.file_type().is_symlink() || md.file_type().is_file(),
            "ORCHESTRATOR.md must still be a symlink or regular file after replace"
        );
    }

    #[test]
    fn write_root_orchestrator_link_produces_link_entry_even_for_missing_target() {
        // Symlink targets may legitimately be missing during a smoke test.
        // The helper should still create the link entry (the link will
        // become reachable once the canonical target is generated).
        let tmp = TempDir::new().unwrap();
        let root_link = tmp.path().join("ORCHESTRATOR.md");
        let target_rel = Path::new("missing/target.md");

        write_root_orchestrator_link(&root_link, target_rel, "BODY").unwrap();

        #[cfg(unix)]
        {
            let md = fs::symlink_metadata(&root_link).unwrap();
            assert!(
                md.file_type().is_symlink(),
                "on unix, ORCHESTRATOR.md must be a symlink even when target is missing"
            );
            assert_eq!(fs::read_link(&root_link).unwrap(), target_rel);
        }
    }
}
