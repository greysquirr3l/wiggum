pub mod agents_md;
pub mod clean;
pub mod orchestrator;
pub mod plan_doc;
pub mod progress;
pub mod task;
pub(crate) mod templates;
pub mod tokens;

use std::path::Path;

use crate::domain::dag::validate_dag;
use crate::domain::plan::Plan;
use crate::error::Result;
use crate::ports::ArtifactWriter;

/// Generated artifacts from a plan.
pub struct GeneratedArtifacts {
    pub progress: String,
    pub orchestrator: String,
    pub plan_doc: String,
    pub tasks: Vec<(String, String)>, // (filename, content)
    pub agents_md: Option<String>,
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
    let orchestrator = orchestrator::render(plan, &resolved)?;
    let plan_doc = plan_doc::render(plan, &resolved)?;

    let mut tasks = Vec::new();
    for t in &resolved {
        let filename = format!("T{:02}-{}.md", t.number, t.slug);
        let content = task::render(plan, t)?;
        tasks.push((filename, content));
    }

    let agents_md = Some(agents_md::render(plan)?);

    Ok(GeneratedArtifacts {
        progress,
        orchestrator,
        plan_doc,
        tasks,
        agents_md,
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
    let orchestrator = orchestrator::render_with(&tera, plan, &resolved)?;
    let plan_doc = plan_doc::render_with(&tera, plan, &resolved)?;

    let mut tasks = Vec::new();
    for t in &resolved {
        let filename = format!("T{:02}-{}.md", t.number, t.slug);
        let content = task::render_with(&tera, plan, t)?;
        tasks.push((filename, content));
    }

    let agents_md = Some(agents_md::render_with(&tera, plan)?);

    Ok(GeneratedArtifacts {
        progress,
        orchestrator,
        plan_doc,
        tasks,
        agents_md,
    })
}

/// Write all generated artifacts to the target project directory.
///
/// # Errors
///
/// Returns an error if any file or directory write operation fails.
pub fn write_artifacts(
    writer: &dyn ArtifactWriter,
    project_path: &Path,
    artifacts: &GeneratedArtifacts,
) -> Result<()> {
    // Write PROGRESS.md
    writer.write_file(&project_path.join("PROGRESS.md"), &artifacts.progress)?;

    // Write IMPLEMENTATION_PLAN.md
    writer.write_file(
        &project_path.join("IMPLEMENTATION_PLAN.md"),
        &artifacts.plan_doc,
    )?;

    // Write orchestrator prompt
    let vscode_dir = project_path.join(".vscode");
    writer.ensure_dir(&vscode_dir)?;
    writer.write_file(
        &vscode_dir.join("orchestrator.prompt.md"),
        &artifacts.orchestrator,
    )?;

    // Write task files
    let tasks_dir = project_path.join("tasks");
    writer.ensure_dir(&tasks_dir)?;
    for (filename, content) in &artifacts.tasks {
        writer.write_file(&tasks_dir.join(filename), content)?;
    }

    // Write AGENTS.md
    if let Some(agents_md) = &artifacts.agents_md {
        writer.write_file(&project_path.join("AGENTS.md"), agents_md)?;
    }

    Ok(())
}
