use tera::{Context, Tera};

use crate::domain::plan::{Plan, ResolvedTask};
use crate::error::{Result, WiggumError};
use crate::generation::templates::get_tera;

/// Render a single task file using the default template.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render(plan: &Plan, task: &ResolvedTask) -> Result<String> {
    render_with(get_tera(), plan, task)
}

/// Render a single task file using a custom Tera instance.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_with(tera: &Tera, plan: &Plan, task: &ResolvedTask) -> Result<String> {
    let mut ctx = Context::new();

    ctx.insert("number_padded", &format!("{:02}", task.number));
    ctx.insert("title", &task.title);
    ctx.insert("goal", &task.goal);
    ctx.insert("project_name", &plan.project.name);
    ctx.insert("project_description", &plan.project.description);
    ctx.insert("language", &plan.project.language.to_string());
    ctx.insert("architecture", &plan.project.architecture);
    ctx.insert("preflight_build", &plan.preflight.build);
    ctx.insert("preflight_test", &plan.preflight.test);
    ctx.insert("preflight_lint", &plan.preflight.lint);
    ctx.insert("hints", &task.hints);
    ctx.insert("test_hints", &task.test_hints);
    ctx.insert("must_haves", &task.must_haves);
    ctx.insert("gate", &task.gate);
    ctx.insert("strategy", &plan.orchestrator.strategy.to_string());
    ctx.insert("evaluation_criteria", &task.evaluation_criteria);

    // Language profile data
    let profile = plan.project.language.profile();
    ctx.insert("build_success_phrase", profile.build_success_phrase);
    ctx.insert("test_file_pattern", profile.test_file_pattern);
    ctx.insert("doc_style", profile.doc_style);
    ctx.insert("error_handling", profile.error_handling);
    let audit_cmd = plan.preflight.audit.as_deref().unwrap_or("");
    ctx.insert("audit_cmd", &audit_cmd);

    // Dependency description
    let depends_on_desc = if task.depends_on.is_empty() {
        "None".to_string()
    } else {
        task.depends_on
            .iter()
            .map(|d| format!("T-{d}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    ctx.insert("depends_on_desc", &depends_on_desc);

    // Conventional commit message template
    let commit_message = format!(
        "feat({}): implement {}",
        task.slug,
        task.title.to_lowercase()
    );
    ctx.insert("commit_message", &commit_message);

    tera.render("task.md", &ctx)
        .map_err(|e| WiggumError::Template(e.to_string()))
}
