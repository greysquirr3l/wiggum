//! Generate `.vscode/background-auditor.prompt.md` — the continuous quality auditor agent prompt.

use tera::{Context, Tera};

use crate::domain::plan::Plan;
use crate::error::{Result, WiggumError};
use crate::generation::templates::get_tera;

/// Render the background auditor prompt using the default template.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render(plan: &Plan) -> Result<String> {
    render_with(get_tera(), plan)
}

/// Render the background auditor prompt using a custom Tera instance.
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

    tera.render("background_auditor.md", &ctx)
        .map_err(|e| WiggumError::Template(e.to_string()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const MINIMAL_PLAN: &str = r#"
[project]
name = "auditor-test"
path = "/tmp/auditor-test"
description = "Testing the background auditor"
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
            output.contains("auditor-test"),
            "expected project name in output"
        );
    }

    #[test]
    fn render_contains_wiring_check() {
        let plan = crate::domain::plan::Plan::from_toml(MINIMAL_PLAN).unwrap();
        let output = render(&plan).unwrap();
        assert!(output.contains("Wiring"), "expected wiring check in output");
    }
}
