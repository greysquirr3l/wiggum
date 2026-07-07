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

/// Render the opencode orchestrator agent prompt (`orchestrator.md`).
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_opencode(plan: &Plan, tasks: &[ResolvedTask]) -> Result<String> {
    render_opencode_with(get_tera(), plan, tasks)
}

/// Render the opencode orchestrator agent prompt using a custom Tera instance.
///
/// The opencode orchestrator is a **single-file** prompt: it embeds both the
/// orchestrator instructions (`<ORCHESTRATOR_INSTRUCTIONS>`) and the
/// subagent body (`<SUBAGENT_PROMPT>`). At dispatch time the orchestrator
/// passes the subagent body to the built-in `general` subagent via the
/// `task` tool with `subagent_type: "general"`. No separate implementer
/// file is needed.
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

    // Completion standard — travels verbatim with every subagent dispatch
    // alongside Accumulated Learnings and Codebase State.
    let completion_standard = plan.style.resolved_completion_standard();
    ctx.insert("completion_standard", &completion_standard);

    // Rules injected into the SUBAGENT_PROMPT body (the implementation
    // subagent is the surface that writes the code).
    ctx.insert("rules", &plan.orchestrator.rules);
    ctx.insert("architecture", &plan.project.architecture);
    ctx.insert("avoid_ai_patterns", &plan.style.avoid_ai_patterns);
    ctx.insert("avoid_god_files", &plan.style.avoid_god_files);

    let profile = plan.project.language.profile();
    ctx.insert("security_rules", &profile.security_rules);
    if plan.style.avoid_ai_patterns {
        ctx.insert("ai_avoidance_rules", &profile.ai_avoidance_rules);
        ctx.insert("comment_guidelines", &profile.comment_guidelines);
    }

    // Strict rules mirror to both the orchestrator (for the briefing it
    // gives subagents) and the subagent body (where code is generated).
    ctx.insert("strict", &plan.style.strict);
    if plan.style.strict {
        ctx.insert("strict_rules", &profile.strict_rules);
    }

    let contract_review = plan.evaluator.as_ref().is_some_and(|e| e.contract_review);
    ctx.insert("contract_review", &contract_review);

    let groups = dag::parallel_groups(tasks)?;
    let groups_value =
        serde_json::to_value(&groups).unwrap_or(serde_json::Value::Array(Vec::new()));
    ctx.insert("parallel_groups", &groups_value);

    tera.render("orchestrator_opencode.md", &ctx)
        .map_err(|e| WiggumError::Template(e.to_string()))
}

/// Render the root-level `ORCHESTRATOR.md` workflow reference document.
///
/// This is the long-form, human-readable document at the project root
/// that explains the orchestration loop, the agents, the state machine,
/// the preflight, the evaluator rubric, and failure recovery. It is what
/// a human or a fresh LLM reads to orient themselves mid-stream.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_orchestrator_root(plan: &Plan, tasks: &[ResolvedTask]) -> Result<String> {
    render_orchestrator_root_with(get_tera(), plan, tasks)
}

/// Render the root-level `ORCHESTRATOR.md` using a custom Tera instance.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_orchestrator_root_with(
    tera: &Tera,
    plan: &Plan,
    tasks: &[ResolvedTask],
) -> Result<String> {
    let mut ctx = Context::new();

    ctx.insert("project_name", &plan.project.name);
    ctx.insert("project_path", &plan.project.path);
    ctx.insert("task_count_padded", &format!("{:02}", tasks.len()));
    ctx.insert("preflight_build", &plan.preflight.build);
    ctx.insert("preflight_test", &plan.preflight.test);
    ctx.insert("preflight_lint", &plan.preflight.lint);
    ctx.insert("preflight_audit", &plan.preflight.audit);
    ctx.insert("max_retries", &plan.orchestrator.max_retries);
    ctx.insert("on_failure", &plan.orchestrator.on_failure.to_string());
    ctx.insert("has_evaluator", &plan.evaluator.is_some());

    let completion_standard = plan.style.resolved_completion_standard();
    ctx.insert("completion_standard", &completion_standard);

    if let Some(eval) = &plan.evaluator {
        ctx.insert("pass_threshold", &eval.pass_threshold);
        let criteria_value =
            serde_json::to_value(&eval.criteria).unwrap_or(serde_json::Value::Array(Vec::new()));
        ctx.insert("criteria", &criteria_value);
    } else {
        // Template always reads `pass_threshold` — provide a sensible
        // default when no evaluator is configured so the rubric section
        // renders cleanly. (The "no evaluator" branch is rendered by
        // the template via `{% if has_evaluator %}`, but Tera still
        // evaluates `{{ pass_threshold }}` if any other part of the
        // template references it.)
        ctx.insert("pass_threshold", &7u8);
        ctx.insert("criteria", &Vec::<String>::new());
    }

    // The plan TOML path is best-effort derived from `project_path` —
    // we don't know the user's filename, so we default to `<project>-plan.toml`.
    let plan_toml = format!(
        "{}-plan.toml",
        plan.project
            .name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
    );
    ctx.insert("plan_toml_path", &plan_toml);

    tera.render("orchestrator_root.md", &ctx).map_err(|e| {
        WiggumError::Template(format!("Failed to render 'orchestrator_root.md': {e:?}"))
    })
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
        assert!(
            rendered.contains("subagent_type: \"general\""),
            "opencode orchestrator must dispatch to the built-in general subagent"
        );
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
        // runs on the orchestrator's session model since `general` doesn't
        // accept a model argument.
        assert!(
            !rendered.contains("pass `model:"),
            "opencode has no per-dispatch model arg"
        );
    }

    #[test]
    fn opencode_orchestrator_has_permissive_permissions_for_preflight() {
        // The orchestrator must independently run preflight (cargo build/test/clippy)
        // and dispatch subagents — both require permissive perms, NOT the restrictive
        // allowlist that the old template had.
        let plan = opencode_plan();
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = render_opencode(&plan, &resolved).unwrap();
        let frontmatter = rendered
            .split_once("---")
            .and_then(|(_, rest)| rest.split_once("---"))
            .map_or("", |(fm, _)| fm);
        assert!(
            frontmatter.contains("edit: allow"),
            "orchestrator must be allowed to edit PROGRESS.md; got:\n{frontmatter}"
        );
        assert!(
            frontmatter.contains("bash: allow"),
            "orchestrator must be allowed to run preflight commands; got:\n{frontmatter}"
        );
        assert!(
            frontmatter.contains("task: allow"),
            "orchestrator must be allowed to dispatch subagents; got:\n{frontmatter}"
        );
    }

    #[test]
    fn opencode_orchestrator_embeds_subagent_prompt_inline() {
        // Single-file pattern: the orchestrator must contain BOTH the
        // orchestrator instructions and the subagent body in the same file,
        // separated by <ORCHESTRATOR_INSTRUCTIONS> / <SUBAGENT_PROMPT> tags.
        let plan = opencode_plan();
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = render_opencode(&plan, &resolved).unwrap();
        assert!(
            rendered.contains("<ORCHESTRATOR_INSTRUCTIONS>"),
            "must contain <ORCHESTRATOR_INSTRUCTIONS> tag"
        );
        assert!(
            rendered.contains("<SUBAGENT_PROMPT>"),
            "must contain <SUBAGENT_PROMPT> tag"
        );
        assert!(
            rendered.contains("Security (non-negotiable)"),
            "subagent body must include the security block"
        );
    }

    #[test]
    fn opencode_orchestrator_injects_default_completion_standard() {
        // The default completion standard (no override in plan) must still
        // appear in the rendered output so every dispatch carries the bar.
        let plan = Plan::from_toml(MINIMAL_PLAN).unwrap();
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = render_opencode(&plan, &resolved).unwrap();
        assert!(
            rendered.contains("<COMPLETION_STANDARD>"),
            "must contain <COMPLETION_STANDARD> reference block"
        );
        assert!(
            rendered.contains("todo!()"),
            "default completion standard must reference placeholder patterns"
        );
        assert!(
            rendered.contains("placeholder implementations"),
            "default completion standard must mention placeholder implementations"
        );
    }

    #[test]
    fn opencode_orchestrator_injects_custom_completion_standard() {
        let toml = r#"
[project]
name = "test"
path = "./test"
description = "test"
language = "rust"

[style]
completion_standard = "MY-CUSTOM-BAR-STRING"

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
        let rendered = render_opencode(&plan, &resolved).unwrap();
        assert!(
            rendered.contains("MY-CUSTOM-BAR-STRING"),
            "custom completion_standard must be injected verbatim"
        );
        assert!(
            !rendered.contains("placeholder implementations"),
            "default completion standard body must be replaced when overridden"
        );
    }

    #[test]
    fn orchestrator_root_doc_includes_state_machine_and_rubric() {
        // The root-level ORCHESTRATOR.md is the long-form reference doc.
        // It must include the state machine, the preflight, and the evaluator
        // rubric so a human or fresh LLM can orient mid-stream.
        let toml = r#"
[project]
name = "test"
path = "./test"
description = "test"
language = "rust"

[preflight]
build = "cargo build"
test = "cargo test"
lint = "cargo clippy"

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
        let rendered = render_orchestrator_root(&plan, &resolved).unwrap();
        assert!(rendered.starts_with("# ORCHESTRATOR"));
        assert!(rendered.contains("Task state machine"));
        assert!(rendered.contains("┌──────┐"));
        assert!(rendered.contains("Preflight"));
        assert!(rendered.contains("cargo test &&"));
        assert!(rendered.contains("Evaluator rubric"));
        assert!(rendered.contains("8/10"));
        assert!(rendered.contains("Completion Standard"));
        assert!(rendered.contains("Accumulated Learnings"));
        assert!(rendered.contains("Codebase State"));
    }

    // ── strict opt-in ───────────────────────────────────────────────

    fn strict_plan() -> Plan {
        let mut plan = opencode_plan();
        plan.style.strict = true;
        plan
    }

    #[test]
    fn strict_off_omits_strict_rules_block_from_orchestrator() {
        let plan = opencode_plan();
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = render_opencode(&plan, &resolved).unwrap();
        assert!(
            !rendered.contains("Strict project standards"),
            "default opencode orchestrator must NOT include strict block"
        );
    }

    #[test]
    fn strict_on_injects_full_rule_list_into_orchestrator_subagent_body() {
        let plan = strict_plan();
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = render_opencode(&plan, &resolved).unwrap();
        assert!(
            rendered.contains("Strict project standards"),
            "orchestrator must include the strict block in the embedded subagent body"
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
    fn opencode_orchestrator_omits_model_line_when_subagent_model_unset() {
        // The `general` subagent doesn't accept a model argument, so the
        // orchestrator frontmatter must not surface subagent_model either.
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
            "opencode orchestrator must NOT emit `model:` when no model is set; got:\n{frontmatter}",
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
