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

    // Strict language rules, conditionally injected via `[style] strict = true`.
    ctx.insert("strict", &plan.style.strict);
    if plan.style.strict {
        ctx.insert("strict_rules", &profile.strict_rules);
    }

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

/// Render the opencode orchestrator agent prompt (`wiggum-orchestrator.md`).
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_opencode(plan: &Plan, tasks: &[ResolvedTask]) -> Result<String> {
    render_opencode_with(get_tera(), plan, tasks)
}

/// Render the opencode orchestrator agent prompt using a custom Tera instance.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_opencode_with(tera: &Tera, plan: &Plan, tasks: &[ResolvedTask]) -> Result<String> {
    let mut ctx = Context::new();

    ctx.insert("project_name", &plan.project.name);
    ctx.insert("project_path", &plan.project.path);
    ctx.insert("task_count_padded", &format!("{:02}", tasks.len()));
    ctx.insert("preflight_build", &plan.preflight.build);
    ctx.insert("preflight_test", &plan.preflight.test);
    ctx.insert("preflight_lint", &plan.preflight.lint);
    ctx.insert("max_retries", &plan.orchestrator.max_retries);
    ctx.insert("on_failure", &plan.orchestrator.on_failure.to_string());
    ctx.insert("orchestrator_model", &plan.orchestrator.model);
    ctx.insert("subagent_model", &plan.orchestrator.subagent_model);
    ctx.insert(
        "evaluator_model",
        &plan.evaluator.as_ref().and_then(|e| e.model.clone()),
    );
    ctx.insert("has_evaluator", &plan.evaluator.is_some());

    // Strict rules are mirrored into the opencode orchestrator so it can
    // repeat the high-level expectations when briefing subagents.
    ctx.insert("strict", &plan.style.strict);
    if plan.style.strict {
        let profile = plan.project.language.profile();
        ctx.insert("strict_rules", &profile.strict_rules);
    }

    let groups = dag::parallel_groups(tasks)?;
    let groups_value =
        serde_json::to_value(&groups).unwrap_or(serde_json::Value::Array(Vec::new()));
    ctx.insert("parallel_groups", &groups_value);

    tera.render("orchestrator_opencode.md", &ctx)
        .map_err(|e| WiggumError::Template(e.to_string()))
}

/// Render the opencode implementer subagent prompt (`wiggum-implementer.md`).
/// The implementer is a single shared body — the orchestrator references the
/// specific task file via `@path` at dispatch time.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_implementer(plan: &Plan) -> Result<String> {
    render_implementer_with(get_tera(), plan)
}

/// Render the opencode implementer subagent prompt using a custom Tera instance.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_implementer_with(tera: &Tera, plan: &Plan) -> Result<String> {
    let mut ctx = Context::new();

    ctx.insert("project_name", &plan.project.name);
    ctx.insert("project_path", &plan.project.path);
    ctx.insert("persona", &plan.orchestrator.persona);
    ctx.insert("preflight_build", &plan.preflight.build);
    ctx.insert("preflight_test", &plan.preflight.test);
    ctx.insert("preflight_lint", &plan.preflight.lint);
    ctx.insert("rules", &plan.orchestrator.rules);
    ctx.insert("architecture", &plan.project.architecture);
    ctx.insert("strategy", &plan.orchestrator.strategy.to_string());
    ctx.insert("subagent_model", &plan.orchestrator.subagent_model);
    ctx.insert("avoid_ai_patterns", &plan.style.avoid_ai_patterns);
    ctx.insert("avoid_god_files", &plan.style.avoid_god_files);

    let profile = plan.project.language.profile();
    ctx.insert("security_rules", &profile.security_rules);
    if plan.style.avoid_ai_patterns {
        ctx.insert("ai_avoidance_rules", &profile.ai_avoidance_rules);
        ctx.insert("comment_guidelines", &profile.comment_guidelines);
    }

    // Strict language rules — only injected into the implementer (where code
    // is generated) and only when the plan opts in. The implementer must
    // see the full list because it is the surface that writes the code.
    ctx.insert("strict", &plan.style.strict);
    if plan.style.strict {
        ctx.insert("strict_rules", &profile.strict_rules);
    }

    let contract_review = plan.evaluator.as_ref().is_some_and(|e| e.contract_review);
    ctx.insert("contract_review", &contract_review);

    tera.render("implementer.md", &ctx)
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

    // ── opencode variants ───────────────────────────────────────────────

    fn opencode_plan() -> Plan {
        let mut plan = Plan::from_toml(MINIMAL_PLAN).unwrap();
        plan.orchestrator.model = Some("anthropic/claude-sonnet-4-20250514".to_string());
        plan.orchestrator.subagent_model = Some("anthropic/claude-sonnet-4-20250514".to_string());
        plan
    }

    #[test]
    fn opencode_orchestrator_has_frontmatter_and_mode_primary() {
        let plan = opencode_plan();
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = render_opencode(&plan, &resolved).unwrap();
        assert!(
            rendered.starts_with("---"),
            "must start with YAML frontmatter"
        );
        assert!(rendered.contains("mode: primary"));
        assert!(rendered.contains("description:"));
        assert!(rendered.contains("anthropic/claude-sonnet-4-20250514"));
    }

    #[test]
    fn opencode_orchestrator_dispatches_via_task_tool() {
        let plan = opencode_plan();
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = render_opencode(&plan, &resolved).unwrap();
        assert!(
            rendered.contains("`task` tool"),
            "must reference the `task` tool"
        );
        assert!(rendered.contains("subagent_type: \"wiggum-implementer\""));
        assert!(
            !rendered.contains("runSubagent"),
            "must NOT use VSCode runSubagent"
        );
    }

    #[test]
    fn opencode_orchestrator_pins_model_in_frontmatter_not_dispatch() {
        let mut plan = opencode_plan();
        plan.orchestrator.subagent_model = Some("anthropic/claude-haiku-4-20250514".to_string());
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = render_opencode(&plan, &resolved).unwrap();
        // The orchestrator's own model goes in its frontmatter; the subagent
        // model is only mentioned in the implementer frontmatter.
        assert!(
            !rendered.contains("pass `model:"),
            "opencode has no per-dispatch model arg"
        );
    }

    #[test]
    fn opencode_orchestrator_gates_task_dispatch_behind_permissions() {
        let plan = opencode_plan();
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = render_opencode(&plan, &resolved).unwrap();
        // Frontmatter must include permission.task gating so the orchestrator
        // can only dispatch wiggum-* subagents.
        assert!(rendered.contains("permission:"));
        assert!(rendered.contains("task:"));
        assert!(rendered.contains("\"wiggum-implementer\": allow"));
    }

    #[test]
    fn opencode_implementer_contains_security_and_strategy_body() {
        let plan = opencode_plan();
        let rendered = render_implementer(&plan).unwrap();
        assert!(rendered.contains("mode: subagent"));
        assert!(rendered.contains("Security (non-negotiable)"));
        // Strategy block is one of the four variants.
        assert!(
            rendered.contains("Strategy: ") || rendered.contains("## Your job"),
            "must include strategy body or default job block"
        );
    }

    // ── strict opt-in ───────────────────────────────────────────────

    fn strict_plan() -> Plan {
        let mut plan = opencode_plan();
        plan.style.strict = true;
        plan
    }

    #[test]
    fn strict_off_omits_strict_rules_block_from_implementer() {
        let plan = opencode_plan();
        let rendered = render_implementer(&plan).unwrap();
        assert!(
            !rendered.contains("Strict project standards"),
            "default implementer must NOT include strict block"
        );
    }

    #[test]
    fn strict_on_injects_full_rule_list_into_implementer() {
        let plan = strict_plan();
        let rendered = render_implementer(&plan).unwrap();
        assert!(
            rendered.contains("Strict project standards"),
            "implementer must include the strict block header"
        );
        // Spot-check the most important rules from nick.md (Rust profile).
        assert!(rendered.contains(".unwrap()"));
        assert!(rendered.contains(".expect()"));
        assert!(rendered.contains("panic!"));
        assert!(rendered.contains("index slicing"));
        assert!(rendered.contains("#[allow(clippy::"));
        assert!(rendered.contains(".is_multiple_of"));
    }

    #[test]
    fn strict_on_injects_rules_into_vscode_orchestrator_subagent_prompt() {
        let mut plan = opencode_plan();
        plan.style.strict = true;
        let rendered = render(&plan, &[]).unwrap();
        // The vscode orchestrator embeds the subagent prompt inline.
        assert!(
            rendered.contains("Strict project standards"),
            "vscode orchestrator subagent prompt must include strict block"
        );
    }

    #[test]
    fn strict_on_injects_rules_into_opencode_orchestrator() {
        let plan = strict_plan();
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = render_opencode(&plan, &resolved).unwrap();
        assert!(
            rendered.contains("Strict project standards"),
            "opencode orchestrator must surface strict rules when dispatching subagents"
        );
    }

    // ── model omission (opencode picker fall-through) ───────────────

    #[test]
    fn opencode_orchestrator_omits_model_line_when_unset() {
        // opencode has no model configured on the plan — the picker should
        // fall through to the opencode-configured default, so the frontmatter
        // must not contain a hardcoded `model:` line.
        let plan = Plan::from_toml(MINIMAL_PLAN).unwrap();
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = render_opencode(&plan, &resolved).unwrap();
        let frontmatter = rendered
            .split_once("---")
            .and_then(|(_, rest)| rest.split_once("---"))
            .map_or("", |(fm, _)| fm);
        assert!(
            !frontmatter
                .lines()
                .any(|l| l.trim_start().starts_with("model:")),
            "opencode orchestrator must NOT emit `model:` when orchestrator_model is None; got:\n{frontmatter}",
        );
    }

    #[test]
    fn opencode_orchestrator_omits_model_line_for_subagent_when_unset() {
        let plan = Plan::from_toml(MINIMAL_PLAN).unwrap();
        let rendered = render_implementer(&plan).unwrap();
        let frontmatter = rendered
            .split_once("---")
            .and_then(|(_, rest)| rest.split_once("---"))
            .map_or("", |(fm, _)| fm);
        assert!(
            !frontmatter
                .lines()
                .any(|l| l.trim_start().starts_with("model:")),
            "opencode implementer must NOT emit `model:` when subagent_model is None; got:\n{frontmatter}",
        );
    }

    #[test]
    fn opencode_orchestrator_emits_model_line_when_explicitly_set() {
        // When the user opts in by setting `model = "..."`, the line MUST appear.
        let plan = opencode_plan();
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = render_opencode(&plan, &resolved).unwrap();
        assert!(
            rendered.contains("model: anthropic/claude-sonnet-4-20250514"),
            "explicit orchestrator_model must be rendered"
        );
    }

    #[test]
    fn opencode_evaluator_omits_model_line_when_unset() {
        let toml = r#"
[project]
name = "test"
path = "./test"
description = "test"
language = "rust"

[evaluator]
model = ""
pass_threshold = 8

[[phases]]
name = "Phase 1"
order = 1

[[phases.tasks]]
slug = "t01"
title = "T01"
goal = "Goal"
"#;
        let plan = Plan::from_toml(toml).unwrap();
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = crate::generation::evaluator::render_opencode(&plan, &resolved)
            .unwrap()
            .unwrap();
        let frontmatter = rendered
            .split_once("---")
            .and_then(|(_, rest)| rest.split_once("---"))
            .map_or("", |(fm, _)| fm);
        assert!(
            !frontmatter
                .lines()
                .any(|l| l.trim_start().starts_with("model:")),
            "opencode evaluator must NOT emit `model:` when model is empty; got:\n{frontmatter}",
        );
    }

    #[test]
    fn opencode_evaluator_omits_model_line_when_field_omitted() {
        // Same intent as the empty-string test, but with the `model` key
        // entirely absent from `[evaluator]`. serde + #[serde(default)] should
        // produce `None` here, and Tera's `{% if evaluator_model %}` must
        // treat that as falsy so opencode falls through to its configured
        // default. If TOML deserialisation or Tera ever changed how missing
        // vs empty values are handled, this test catches the regression.
        let toml = r#"
[project]
name = "test"
path = "./test"
description = "test"
language = "rust"

[evaluator]
pass_threshold = 8

[[phases]]
name = "Phase 1"
order = 1

[[phases.tasks]]
slug = "t01"
title = "T01"
goal = "Goal"
"#;
        let plan = Plan::from_toml(toml).unwrap();
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = crate::generation::evaluator::render_opencode(&plan, &resolved)
            .unwrap()
            .unwrap();
        let frontmatter = rendered
            .split_once("---")
            .and_then(|(_, rest)| rest.split_once("---"))
            .map_or("", |(fm, _)| fm);
        assert!(
            !frontmatter
                .lines()
                .any(|l| l.trim_start().starts_with("model:")),
            "opencode evaluator must NOT emit `model:` when model key is absent; got:\n{frontmatter}",
        );
    }
}
