use tera::{Context, Tera};

use crate::domain::{
    dag,
    plan::{Plan, ResolvedTask},
};
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
    ctx.insert("max_retries", &plan.orchestrator.max_retries);
    ctx.insert("on_failure", &plan.orchestrator.on_failure.to_string());
    ctx.insert("orchestrator_model", &plan.orchestrator.model);
    ctx.insert("subagent_model", &plan.orchestrator.subagent_model);
    ctx.insert(
        "evaluator_model",
        &plan.evaluator.as_ref().and_then(|e| e.model.clone()),
    );
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

    // Parallel execution groups for concurrent subagent dispatch.
    let groups = dag::parallel_groups(tasks)?;
    let groups_value =
        serde_json::to_value(&groups).unwrap_or(serde_json::Value::Array(Vec::new()));
    ctx.insert("parallel_groups", &groups_value);

    // Contract review gate (requires evaluator).
    let contract_review = plan.evaluator.as_ref().is_some_and(|e| e.contract_review);
    ctx.insert("contract_review", &contract_review);

    tera.render("orchestrator.md", &ctx)
        .map_err(|e| WiggumError::Template(e.to_string()))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::plan::{FailureAction, Plan};

    const MINIMAL_PLAN: &str = r#"
[project]
name = "test"
path = "./test"
description = "test"
language = "rust"

[[phases]]
name = "Phase 1"
order = 1

[[phases.tasks]]
slug = "t01-init"
title = "T01 — Init"
phase = "Phase 1"
goal = "Set up the project."
"#;

    /// The orchestrator template branches on the *string* value of `on_failure`.
    /// Pin those values here so a future Display change breaks tests, not users.
    #[test]
    fn failure_action_display_values_match_template_branches() {
        assert_eq!(FailureAction::Pause.to_string(), "pause");
        assert_eq!(FailureAction::Skip.to_string(), "skip");
        assert_eq!(FailureAction::Escalate.to_string(), "escalate");
    }

    #[test]
    fn each_failure_action_renders_its_template_section() {
        let base = Plan::from_toml(MINIMAL_PLAN).unwrap();

        let cases = [
            (FailureAction::Pause, "**Pause**"),
            (FailureAction::Skip, "**Skip**"),
            (FailureAction::Escalate, "**Escalate**"),
        ];
        for (action, expected_marker) in cases {
            let mut plan = base.clone();
            plan.orchestrator.on_failure = action;
            plan.orchestrator.max_retries = 1;
            let rendered = render(&plan, &[]).unwrap();
            assert!(
                rendered.contains(expected_marker),
                "Expected '{expected_marker}' section for {action:?}, got:\n{rendered}",
            );
        }
    }

    #[test]
    fn orchestrator_model_renders_recommended_header_when_set() {
        let mut plan = Plan::from_toml(MINIMAL_PLAN).unwrap();
        plan.orchestrator.model = Some("claude-opus-4.7".to_string());
        let rendered = render(&plan, &[]).unwrap();
        assert!(rendered.contains("**Recommended model:** `claude-opus-4.7`"));
    }

    #[test]
    fn orchestrator_omits_model_header_when_unset() {
        let plan = Plan::from_toml(MINIMAL_PLAN).unwrap();
        let rendered = render(&plan, &[]).unwrap();
        assert!(!rendered.contains("**Recommended model:**"));
    }

    #[test]
    fn subagent_model_injects_runsubagent_model_argument() {
        let mut plan = Plan::from_toml(MINIMAL_PLAN).unwrap();
        plan.orchestrator.subagent_model = Some("claude-sonnet-4.5".to_string());
        let rendered = render(&plan, &[]).unwrap();
        assert!(
            rendered.contains("pass `model: \"claude-sonnet-4.5\"`"),
            "expected runSubagent model directive, got:\n{rendered}",
        );
    }

    #[test]
    fn subagent_model_omitted_when_unset() {
        let plan = Plan::from_toml(MINIMAL_PLAN).unwrap();
        let rendered = render(&plan, &[]).unwrap();
        assert!(!rendered.contains("pass `model:"));
    }
}
