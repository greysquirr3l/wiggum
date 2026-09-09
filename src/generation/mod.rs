pub mod agent_rules;
pub mod agents_md;
pub mod background_auditor;
pub mod budget;
pub mod claude;
pub mod clean;
pub mod evaluator;
pub mod features;
pub mod hooks;
pub mod orchestrator;
pub mod plan_doc;
pub mod planner;
pub mod progress;
pub mod run_log;
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
    /// `.opencode/agents/orchestrator.md`. Single-file orchestrator prompt —
    /// embeds both `<ORCHESTRATOR_INSTRUCTIONS>` and `<SUBAGENT_PROMPT>` so the
    /// orchestrator can dispatch to the built-in `general` subagent via
    /// `task(subagent_type="general", prompt=<SUBAGENT_PROMPT>)`.
    pub orchestrator_opencode: String,
    /// `.opencode/agents/evaluator.md`. Present only when `[evaluator]` is
    /// configured AND the opencode target is selected.
    pub evaluator_opencode: Option<String>,
    /// `.opencode/agents/planner.md`.
    pub planner_opencode: String,
    /// `.opencode/agents/background-auditor.md`.
    pub background_auditor_opencode: String,
    /// `ORCHESTRATOR.md` at the project root — a long-form workflow reference
    /// document (state machine, agents table, evaluator rubric, completion
    /// standard, failure modes) that anyone — human or fresh LLM — can read
    /// to orient themselves mid-stream. Real file, not a symlink.
    pub orchestrator_root: String,

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

    // ── Universal scaffold (not gated by target) ────────────────────
    /// `BUDGET.md` — per-task token estimates, warn/critical thresholds,
    /// recommended daily cap. T06.
    pub budget: String,
    /// `RUN_LOG.md` — empty per-iteration audit log scaffold. T07.
    pub run_log: String,
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
    let orchestrator_root = orchestrator::render_orchestrator_root(plan, &resolved)?;
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
    let budget = budget::render(plan, &resolved)?;
    let run_log = run_log::render()?;

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
        evaluator_opencode,
        planner_opencode,
        background_auditor_opencode,
        orchestrator_root,
        hooks_json,
        claude_md,
        agent_rules_cursorrules: agent_rules_content.clone(),
        agent_rules_windsurfrules: agent_rules_content.clone(),
        agent_rules_copilot_instructions: agent_rules_content,
        budget,
        run_log,
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
    let orchestrator_root = orchestrator::render_orchestrator_root_with(&tera, plan, &resolved)?;
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
    let budget = budget::render(plan, &resolved)?;
    let run_log = run_log::render()?;

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
        evaluator_opencode,
        planner_opencode,
        background_auditor_opencode,
        orchestrator_root,
        hooks_json,
        claude_md,
        agent_rules_cursorrules: agent_rules_content.clone(),
        agent_rules_windsurfrules: agent_rules_content.clone(),
        agent_rules_copilot_instructions: agent_rules_content,
        budget,
        run_log,
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

    // RUN_LOG.md — universal scaffold artifact (T07).
    writer.write_file(&project_path.join("RUN_LOG.md"), &artifacts.run_log)?;

    // BUDGET.md — universal scaffold artifact (T06).
    writer.write_file(&project_path.join("BUDGET.md"), &artifacts.budget)?;

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
        let opencode_dir = project_path.join(".opencode");
        writer.ensure_dir(&opencode_dir)?;
        let agents_dir = opencode_dir.join("agents");
        writer.ensure_dir(&agents_dir)?;
        writer.write_file(
            &agents_dir.join("orchestrator.md"),
            &artifacts.orchestrator_opencode,
        )?;
        writer.write_file(&agents_dir.join("planner.md"), &artifacts.planner_opencode)?;
        writer.write_file(
            &agents_dir.join("background-auditor.md"),
            &artifacts.background_auditor_opencode,
        )?;
        if let Some(eval) = &artifacts.evaluator_opencode {
            writer.write_file(&agents_dir.join("evaluator.md"), eval)?;
        }

        // `ORCHESTRATOR.md` at the project root — long-form workflow
        // reference document, separate from the `.opencode/agents/orchestrator.md`
        // agent prompt. Both files have distinct purposes: the agent prompt
        // tells the orchestrator what to do; this document tells anyone
        // (human or LLM) how the loop works.
        writer.write_file(
            &project_path.join("ORCHESTRATOR.md"),
            &artifacts.orchestrator_root,
        )?;
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
