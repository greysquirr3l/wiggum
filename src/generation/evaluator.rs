//! Generate `.vscode/evaluator.prompt.md` — the QA evaluator agent prompt.
//!
//! Anthropic's harness research found that pairing each implementation subagent
//! with a skeptical QA evaluator dramatically reduces false completions. The
//! evaluator agent runs independently, re-runs preflight, checks every exit
//! criterion, and updates `features.json` with the verified pass/fail state.

use tera::{Context, Tera};

use crate::domain::plan::{EvaluatorConfig, Plan, ResolvedTask};
use crate::error::{Result, WiggumError};
use crate::generation::templates::get_tera;

/// Render the evaluator prompt using the default template.
///
/// Returns `None` when the plan has no `[evaluator]` section — evaluator
/// generation is opt-in so that existing plans continue to work unchanged.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render(plan: &Plan, tasks: &[ResolvedTask]) -> Result<Option<String>> {
    render_with(get_tera(), plan, tasks)
}

/// Render the evaluator prompt using a custom Tera instance.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_with(tera: &Tera, plan: &Plan, tasks: &[ResolvedTask]) -> Result<Option<String>> {
    let Some(evaluator) = &plan.evaluator else {
        return Ok(None);
    };

    let rendered = render_evaluator(tera, plan, tasks, evaluator)?;
    Ok(Some(rendered))
}

fn render_evaluator(
    tera: &Tera,
    plan: &Plan,
    tasks: &[ResolvedTask],
    evaluator: &EvaluatorConfig,
) -> Result<String> {
    let mut ctx = Context::new();

    ctx.insert("project_name", &plan.project.name);
    ctx.insert("project_path", &plan.project.path);
    ctx.insert("evaluator_persona", &evaluator.persona);
    ctx.insert("pass_threshold", &evaluator.pass_threshold);
    ctx.insert("hard_fail", &evaluator.hard_fail);
    ctx.insert(
        "test_tool",
        evaluator
            .test_tool
            .as_deref()
            .unwrap_or(&plan.preflight.test),
    );
    ctx.insert("preflight_build", &plan.preflight.build);
    ctx.insert("preflight_test", &plan.preflight.test);
    ctx.insert("preflight_lint", &plan.preflight.lint);
    ctx.insert("task_count_padded", &format!("{:02}", tasks.len()));

    tera.render("evaluator.md", &ctx)
        .map_err(|e| WiggumError::Template(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plan::{
        EvaluatorConfig, IntegrationConfig, Language, Orchestrator, Phase, Plan, Preflight,
        Project, ResolvedTask, SecurityConfig, StyleConfig, TaskDef,
    };

    fn make_plan(with_evaluator: bool) -> Plan {
        Plan {
            project: Project {
                name: "test-project".to_string(),
                description: "A test".to_string(),
                language: Language::Rust,
                path: "/tmp/test".to_string(),
                architecture: None,
            },
            preflight: Preflight {
                build: "cargo build".to_string(),
                test: "cargo test".to_string(),
                lint: "cargo clippy".to_string(),
                audit: None,
            },
            orchestrator: Orchestrator::default(),
            evaluator: if with_evaluator {
                Some(EvaluatorConfig::default())
            } else {
                None
            },
            security: SecurityConfig::default(),
            integration: IntegrationConfig::default(),
            style: StyleConfig::default(),
            phases: vec![Phase {
                name: "Foundation".to_string(),
                order: 1,
                tasks: vec![TaskDef {
                    slug: "scaffold".to_string(),
                    title: "Scaffold".to_string(),
                    goal: "Set up the project".to_string(),
                    depends_on: vec![],
                    hints: vec![],
                    test_hints: vec![],
                    must_haves: vec![],
                    gate: None,
                    evaluation_criteria: vec![],
                }],
            }],
        }
    }

    fn make_tasks() -> Vec<ResolvedTask> {
        vec![ResolvedTask {
            number: 1,
            slug: "scaffold".to_string(),
            title: "Scaffold".to_string(),
            goal: "Set up the project".to_string(),
            depends_on: vec![],
            hints: vec![],
            test_hints: vec![],
            must_haves: vec![],
            gate: None,
            evaluation_criteria: vec![],
            phase_name: "Foundation".to_string(),
            phase_order: 1,
        }]
    }

    #[test]
    fn returns_none_without_evaluator_config()
    -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let plan = make_plan(false);
        let tasks = make_tasks();
        let result = render(&plan, &tasks)?;
        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn returns_some_with_evaluator_config()
    -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let plan = make_plan(true);
        let tasks = make_tasks();
        let result = render(&plan, &tasks)?;
        assert!(result.is_some());
        Ok(())
    }

    #[test]
    fn contains_project_name() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>
    {
        let plan = make_plan(true);
        let tasks = make_tasks();
        let output = render(&plan, &tasks)?.ok_or("expected Some")?;
        assert!(output.contains("test-project"));
        Ok(())
    }

    #[test]
    fn contains_preflight_commands()
    -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let plan = make_plan(true);
        let tasks = make_tasks();
        let output = render(&plan, &tasks)?.ok_or("expected Some")?;
        assert!(output.contains("cargo build"));
        assert!(output.contains("cargo test"));
        assert!(output.contains("cargo clippy"));
        Ok(())
    }
}
