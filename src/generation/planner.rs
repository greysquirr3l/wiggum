//! Generate `.vscode/planner.prompt.md` and
//! `.opencode/agents/wiggum-planner.md` — the task-decomposition planner
//! agent prompt.

use tera::{Context, Tera};

use crate::domain::plan::Plan;
use crate::error::{Result, WiggumError};
use crate::generation::templates::get_tera;

/// Render the `VSCode` planner prompt using the default `Tera` instance.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render(plan: &Plan) -> Result<String> {
    render_with(get_tera(), plan)
}

/// Render the `VSCode` planner prompt using a custom `Tera` instance.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_with(tera: &Tera, plan: &Plan) -> Result<String> {
    let mut ctx = Context::new();

    ctx.insert("project_name", &plan.project.name);
    ctx.insert("project_path", &plan.project.path);
    ctx.insert("language", &plan.project.language.to_string());
    ctx.insert("architecture", &plan.project.architecture);
    ctx.insert("preflight_build", &plan.preflight.build);
    ctx.insert("preflight_test", &plan.preflight.test);
    ctx.insert("preflight_lint", &plan.preflight.lint);

    tera.render("planner.md", &ctx)
        .map_err(|e| WiggumError::Template(e.to_string()))
}

/// Render the opencode planner subagent prompt (`wiggum-planner.md`).
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_opencode(plan: &Plan) -> Result<String> {
    render_opencode_with(get_tera(), plan)
}

/// Render the opencode planner subagent prompt using a custom Tera instance.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_opencode_with(tera: &Tera, plan: &Plan) -> Result<String> {
    let mut ctx = Context::new();

    ctx.insert("project_name", &plan.project.name);
    ctx.insert("project_path", &plan.project.path);
    ctx.insert("language", &plan.project.language.to_string());
    ctx.insert("architecture", &plan.project.architecture);
    ctx.insert("preflight_build", &plan.preflight.build);
    ctx.insert("preflight_test", &plan.preflight.test);
    ctx.insert("preflight_lint", &plan.preflight.lint);

    tera.render("planner_opencode.md", &ctx)
        .map_err(|e| WiggumError::Template(e.to_string()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const MINIMAL_PLAN: &str = r#"
[project]
name = "planner-test"
path = "/tmp/planner-test"
description = "Testing the planner"
language = "rust"

[[phases]]
name = "Phase 1"
order = 1

[[phases.tasks]]
slug = "t01"
title = "T01 Init"
goal = "Set up the project."
"#;

    #[test]
    fn render_contains_project_name() {
        let plan = crate::domain::plan::Plan::from_toml(MINIMAL_PLAN).unwrap();
        let output = render(&plan).unwrap();
        assert!(
            output.contains("planner-test"),
            "expected project name in output"
        );
    }

    #[test]
    fn render_contains_preflight_build() {
        let plan = crate::domain::plan::Plan::from_toml(MINIMAL_PLAN).unwrap();
        let output = render(&plan).unwrap();
        assert!(
            output.contains("cargo build"),
            "expected build cmd in output"
        );
    }

    #[test]
    fn render_opencode_contains_subagent_frontmatter() {
        let plan = crate::domain::plan::Plan::from_toml(MINIMAL_PLAN).unwrap();
        let output = render_opencode(&plan).unwrap();
        assert!(output.starts_with("---"), "must start with YAML frontmatter");
        assert!(output.contains("mode: subagent"));
        assert!(output.contains("bash: deny"));
    }
}
