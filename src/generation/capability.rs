//! Render `capabilities/<name>.md` files.
//!
//! Each `[[capabilities]]` entry in `plan.toml` produces a standalone Markdown
//! file that describes the behavioural contract the system must satisfy.
//! Tasks reference these files via `implements = ["<name>"]` and the task
//! template inlines the linked capabilities so the subagent sees the
//! scenarios without having to open a separate file.

use tera::{Context, Tera};

use crate::domain::plan::Plan;
use crate::error::Result;
use crate::generation::templates::get_tera;

/// Render every capability in the plan to its `(filename, content)` pair.
///
/// The filename is always `capabilities/<name>.md` where `<name>` is the
/// capability slug from `Capability::name`. Empty plan → empty vec (no
/// `capabilities/` directory is created).
///
/// # Errors
///
/// Returns an error if template rendering fails for any capability.
pub fn render_all(plan: &Plan) -> Result<Vec<(String, String)>> {
    let tera = get_tera();
    plan.capabilities
        .iter()
        .map(|cap| {
            let mut ctx = Context::new();
            ctx.insert("name", &cap.name);
            ctx.insert("title", &cap.title);
            ctx.insert("description", &cap.description);
            ctx.insert("requirements", &cap.requirements);
            ctx.insert("scenarios", &cap.scenarios);
            let content = tera
                .render("capability.md", &ctx)
                .map_err(|e| crate::error::WiggumError::Template(e.to_string()))?;
            Ok((format!("{}.md", cap.name), content))
        })
        .collect()
}

/// Render a single capability using a custom Tera instance.
///
/// Useful for tests that want to swap in alternate template bodies without
/// touching the embedded default.
pub fn render_with(tera: &Tera, plan: &Plan, capability_name: &str) -> Result<Option<String>> {
    let Some(cap) = plan.capabilities.iter().find(|c| c.name == capability_name) else {
        return Ok(None);
    };
    let mut ctx = Context::new();
    ctx.insert("name", &cap.name);
    ctx.insert("title", &cap.title);
    ctx.insert("description", &cap.description);
    ctx.insert("requirements", &cap.requirements);
    ctx.insert("scenarios", &cap.scenarios);
    let content = tera
        .render("capability.md", &ctx)
        .map_err(|e| crate::error::WiggumError::Template(e.to_string()))?;
    Ok(Some(content))
}

/// Look up a capability by name. Returns `None` if no capability with that
/// name exists in the plan.
#[must_use]
pub fn find<'a>(plan: &'a Plan, name: &str) -> Option<&'a crate::domain::plan::Capability> {
    plan.capabilities.iter().find(|c| c.name == name)
}

/// Resolve the linked capabilities for a task in `implements` order.
///
/// Unknown references are silently dropped —
/// `Plan::validate_capabilities` surfaces those errors during validation,
/// not at render time.
#[must_use]
pub fn linked_for_task<'a>(
    plan: &'a Plan,
    task: &crate::domain::plan::ResolvedTask,
) -> Vec<&'a crate::domain::plan::Capability> {
    task.implements
        .iter()
        .filter_map(|name| plan.capabilities.iter().find(|c| &c.name == name))
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::plan::{Capability, Scenario};

    fn sample_plan() -> Plan {
        let toml = r#"
[project]
name = "spec-demo"
path = "/tmp/spec-demo"
description = "Demo"
language = "rust"

[[capabilities]]
name = "webhook-reception"
title = "Webhook Reception"
description = "Accepts and validates inbound webhook deliveries."
[[capabilities.scenarios]]
name = "valid-signature"
when = "POST /webhook receives a request with a valid HMAC signature"
then = "the server returns 202 Accepted and persists the event"

[[phases]]
name = "Inbound"
order = 1
[[phases.tasks]]
slug = "router"
title = "Router"
goal = "Stand up the router"
implements = ["webhook-reception"]
"#;
        Plan::from_toml(toml).unwrap()
    }

    #[expect(clippy::panic, reason = "test helper")]
    fn first_cap(out: &[(String, String)]) -> &(String, String) {
        match out {
            [first, ..] => first,
            [] => panic!("expected at least one capability file"),
        }
    }

    #[expect(clippy::panic, reason = "test helper")]
    fn first_linked<'inner>(linked: &[&'inner Capability]) -> &'inner Capability {
        match linked {
            [first, ..] => first,
            [] => panic!("expected at least one linked capability"),
        }
    }

    #[test]
    fn render_all_returns_one_file_per_capability() {
        let plan = sample_plan();
        let out = render_all(&plan).unwrap();
        assert_eq!(out.len(), 1);
        let (filename, body) = first_cap(&out);
        assert_eq!(filename, "webhook-reception.md");
        assert!(body.contains("Webhook Reception"));
        assert!(body.contains("**WHEN**"));
        assert!(body.contains("**THEN**"));
        assert!(body.contains("202 Accepted"));
    }

    #[test]
    fn render_all_empty_for_plan_with_no_capabilities() {
        let toml = r#"
[project]
name = "no-caps"
path = "/tmp/no-caps"
description = "No caps"
language = "rust"

[[phases]]
name = "P"
order = 1
[[phases.tasks]]
slug = "t"
title = "T"
goal = "g"
"#;
        let plan = Plan::from_toml(toml).unwrap();
        let out = render_all(&plan).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn find_returns_named_capability() {
        let plan = sample_plan();
        assert!(find(&plan, "webhook-reception").is_some());
        assert!(find(&plan, "nope").is_none());
    }

    #[test]
    fn linked_for_task_resolves_implements_in_order() {
        let plan = sample_plan();
        let resolved = plan.resolve_tasks().unwrap();
        let task = resolved.iter().find(|t| t.slug == "router").unwrap();
        let linked = linked_for_task(&plan, task);
        assert_eq!(linked.len(), 1);
        let first = first_linked(&linked);
        assert_eq!(first.name, "webhook-reception");
    }

    #[test]
    fn linked_for_task_silently_drops_unknown_references() {
        let toml = r#"
[project]
name = "dangling"
path = "/tmp/dangling"
description = "x"
language = "rust"

[[capabilities]]
name = "real"
title = "Real"
description = ""

[[phases]]
name = "P"
order = 1
[[phases.tasks]]
slug = "t"
title = "T"
goal = "g"
implements = ["real", "phantom"]
"#;
        let plan = Plan::from_toml(toml).unwrap();
        let resolved = plan.resolve_tasks().unwrap();
        let task = resolved.iter().find(|t| t.slug == "t").unwrap();
        let linked = linked_for_task(&plan, task);
        assert_eq!(linked.len(), 1);
        let first = first_linked(&linked);
        assert_eq!(first.name, "real");
    }

    #[test]
    fn scenario_round_trips_when_then_lines() {
        let plan = sample_plan();
        let out = render_all(&plan).unwrap();
        let (_, body) = first_cap(&out);
        // Both lines must be present and on their own line for grep-ability.
        assert!(body.contains("\n- **WHEN** POST /webhook"));
        assert!(body.contains("\n- **THEN** the server returns 202"));
    }

    #[test]
    fn capability_with_no_scenarios_renders_placeholder() {
        let cap = Capability {
            name: "empty".to_string(),
            title: "Empty Capability".to_string(),
            description: "Has no scenarios".to_string(),
            requirements: vec!["MUST do the thing".to_string()],
            scenarios: vec![],
        };
        let plan = Plan {
            project: crate::domain::plan::Project {
                name: "x".to_string(),
                description: "x".to_string(),
                language: crate::domain::plan::Language::Rust,
                path: "/tmp/x".to_string(),
                architecture: None,
            },
            preflight: crate::domain::plan::Preflight::default(),
            orchestrator: crate::domain::plan::Orchestrator::default(),
            evaluator: None,
            security: crate::domain::plan::SecurityConfig::default(),
            integration: crate::domain::plan::IntegrationConfig::default(),
            style: crate::domain::plan::StyleConfig::default(),
            targets: crate::domain::plan::TargetConfig::default(),
            capabilities: vec![cap],
            phases: vec![],
        };
        let out = render_all(&plan).unwrap();
        assert_eq!(out.len(), 1);
        let (_, body) = first_cap(&out);
        assert!(body.contains("MUST do the thing"));
        assert!(body.contains("TODO: Add at least one WHEN/THEN"));
    }

    #[test]
    fn scenario_with_structured_fields_renders_under_scenario_heading() {
        let cap = Capability {
            name: "x".to_string(),
            title: "X".to_string(),
            description: String::new(),
            requirements: vec![],
            scenarios: vec![Scenario {
                name: "first-scenario".to_string(),
                when: "the user clicks save".to_string(),
                then: "the document is persisted".to_string(),
            }],
        };
        let plan = Plan {
            project: crate::domain::plan::Project {
                name: "x".to_string(),
                description: "x".to_string(),
                language: crate::domain::plan::Language::Rust,
                path: "/tmp/x".to_string(),
                architecture: None,
            },
            preflight: crate::domain::plan::Preflight::default(),
            orchestrator: crate::domain::plan::Orchestrator::default(),
            evaluator: None,
            security: crate::domain::plan::SecurityConfig::default(),
            integration: crate::domain::plan::IntegrationConfig::default(),
            style: crate::domain::plan::StyleConfig::default(),
            targets: crate::domain::plan::TargetConfig::default(),
            capabilities: vec![cap],
            phases: vec![],
        };
        let out = render_all(&plan).unwrap();
        let (_, body) = first_cap(&out);
        assert!(body.contains("### Scenario: first-scenario"));
        assert!(body.contains("the user clicks save"));
        assert!(body.contains("the document is persisted"));
    }

    #[test]
    fn validate_capabilities_rejects_unknown_implements_references() {
        let toml = r#"
[project]
name = "dangling"
path = "/tmp/dangling"
description = "x"
language = "rust"

[[phases]]
name = "P"
order = 1
[[phases.tasks]]
slug = "t"
title = "T"
goal = "g"
implements = ["missing-capability"]
"#;
        let plan = Plan::from_toml(toml).unwrap();
        let resolved = plan.resolve_tasks().unwrap();
        let err = plan.validate_capabilities(&resolved).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("capability references"), "got: {msg}");
        assert!(msg.contains("missing-capability"), "got: {msg}");
        assert!(msg.contains("T01"), "got: {msg}");
    }

    #[test]
    fn validate_capabilities_rejects_duplicate_capability_names() {
        let toml = r#"
[project]
name = "dupes"
path = "/tmp/dupes"
description = "x"
language = "rust"

[[capabilities]]
name = "shared"
title = "First"
description = ""

[[capabilities]]
name = "shared"
title = "Second"
description = ""

[[phases]]
name = "P"
order = 1
[[phases.tasks]]
slug = "t"
title = "T"
goal = "g"
"#;
        let plan = Plan::from_toml(toml).unwrap();
        let resolved = plan.resolve_tasks().unwrap();
        let err = plan.validate_capabilities(&resolved).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("duplicate capability name"), "got: {msg}");
        assert!(msg.contains("shared"), "got: {msg}");
    }

    #[test]
    fn validate_capabilities_passes_when_references_resolve() {
        let plan = sample_plan();
        let resolved = plan.resolve_tasks().unwrap();
        plan.validate_capabilities(&resolved).unwrap();
    }
}
