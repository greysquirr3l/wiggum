use tera::{Context, Tera};

use crate::domain::plan::{Plan, ResolvedTask};
use crate::error::{Result, WiggumError};
use crate::generation::templates::get_tera;

/// Render the orchestrator prompt using the default template.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render(plan: &Plan, tasks: &[ResolvedTask]) -> Result<String> {
    render_with(get_tera(), plan, tasks)
}

/// Render the orchestrator prompt using a custom Tera instance.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_with(tera: &Tera, plan: &Plan, tasks: &[ResolvedTask]) -> Result<String> {
    let mut ctx = Context::new();

    ctx.insert("project_name", &plan.project.name);
    ctx.insert("project_path", &plan.project.path);
    ctx.insert("task_count_padded", &format!("{:02}", tasks.len()));
    ctx.insert("persona", &plan.orchestrator.persona);
    ctx.insert("preflight_build", &plan.preflight.build);
    ctx.insert("preflight_test", &plan.preflight.test);
    ctx.insert("preflight_lint", &plan.preflight.lint);
    ctx.insert("rules", &plan.orchestrator.rules);
    ctx.insert("architecture", &plan.project.architecture);
    ctx.insert("strategy", &plan.orchestrator.strategy.to_string());
    ctx.insert("has_evaluator", &plan.evaluator.is_some());

    // Security rules from the language profile, always injected.
    let profile = plan.project.language.profile();
    ctx.insert("security_rules", &profile.security_rules);

    // AI pattern avoidance rules, conditionally injected.
    ctx.insert("avoid_ai_patterns", &plan.style.avoid_ai_patterns);
    if plan.style.avoid_ai_patterns {
        ctx.insert("ai_avoidance_rules", &profile.ai_avoidance_rules);
        ctx.insert("comment_guidelines", &profile.comment_guidelines);
    }

    // File-structure guidance, conditionally injected.
    ctx.insert("avoid_god_files", &plan.style.avoid_god_files);

    tera.render("orchestrator.md", &ctx)
        .map_err(|e| WiggumError::Template(e.to_string()))
}
