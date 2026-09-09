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
    ctx.insert("kind", &task.kind.to_string());
    ctx.insert("evaluation_criteria", &task.evaluation_criteria);
    ctx.insert("avoid_god_files", &plan.style.avoid_god_files);

    // Language profile data
    let profile = plan.project.language.profile();
    ctx.insert("build_success_phrase", profile.build_success_phrase);
    ctx.insert("test_file_pattern", profile.test_file_pattern);
    ctx.insert("doc_style", profile.doc_style);
    ctx.insert("error_handling", profile.error_handling);
    let audit_cmd = plan.preflight.audit.as_deref().unwrap_or("");
    ctx.insert("audit_cmd", &audit_cmd);

    // Strict language rules — only injected when `[style] strict = true`.
    // Each per-task file carries the rules because the implementer reads
    // the task file in isolation when dispatched.
    ctx.insert("strict", &plan.style.strict);
    if plan.style.strict {
        ctx.insert("strict_rules", &profile.strict_rules);
    }

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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::plan::Plan;

    fn plan_with(strict: bool) -> Plan {
        let toml = format!(
            r#"
[project]
name = "strict-test"
path = "./strict-test"
description = "strict mode test"
language = "rust"

[style]
strict = {strict}

[[phases]]
name = "Phase 1"
order = 1

[[phases.tasks]]
slug = "t01-init"
title = "T01 Init"
goal = "Set up the project."
"#
        );
        Plan::from_toml(&toml).unwrap()
    }

    #[test]
    fn task_omits_strict_rules_by_default() {
        let plan = plan_with(false);
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = render(&plan, resolved.first().unwrap()).unwrap();
        assert!(
            !rendered.contains("Strict Standards"),
            "task must NOT include strict block by default"
        );
    }

    #[test]
    fn task_includes_strict_rules_when_opted_in() {
        let plan = plan_with(true);
        let resolved = plan.resolve_tasks().unwrap();
        let rendered = render(&plan, resolved.first().unwrap()).unwrap();
        assert!(
            rendered.contains("Strict Standards"),
            "task must include the strict block header when `[style] strict = true`"
        );
        assert!(rendered.contains(".unwrap()"));
        assert!(rendered.contains(".is_multiple_of"));
    }

    // ── T04: gate banner ─────────────────────────────────────────────

    fn gated_plan() -> Plan {
        let toml = r#"
[project]
name = "gate-test"
path = "./gate-test"
description = "gate banner test"
language = "rust"

[orchestrator]
gates = ["auth"]

[[phases]]
name = "Phase 1"
order = 1

[[phases.tasks]]
slug = "auth-handler"
title = "Auth Handler"
goal = "Implement auth."
gate = "auth"

[[phases.tasks]]
slug = "fetch-events"
title = "Fetch Events"
goal = "Fetch events."
"#;
        Plan::from_toml(toml).unwrap()
    }

    #[test]
    fn gated_task_renders_gate_banner_above_h1() {
        let plan = gated_plan();
        let resolved = plan.resolve_tasks().unwrap();
        let gated = resolved.iter().find(|t| t.slug == "auth-handler").unwrap();
        let rendered = render(&plan, gated).unwrap();
        assert!(
            rendered.contains("GATE"),
            "gated task must render a GATE banner, got:\n{rendered}"
        );
        // Banner appears above the H1 title (the first '# T' line).
        let banner_pos = rendered.find("GATE").unwrap();
        let h1_pos = rendered.find("# T").unwrap();
        assert!(
            banner_pos < h1_pos,
            "GATE banner must precede the H1 title, got banner@{banner_pos} h1@{h1_pos}"
        );
    }

    #[test]
    fn non_gated_task_omits_gate_banner() {
        let plan = gated_plan();
        let resolved = plan.resolve_tasks().unwrap();
        let plain_task = resolved.iter().find(|t| t.slug == "fetch-events").unwrap();
        let rendered = render(&plan, plain_task).unwrap();
        assert!(
            !rendered.contains("GATE"),
            "non-gated task must NOT render a GATE banner, got:\n{rendered}"
        );
    }
}
