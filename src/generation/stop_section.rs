//! Render the **Stop and ask** block shared by every orchestrator variant.
//!
//! The default list is fixed so a plan without `stop_when` still tells the
//! agent where autonomy ends; the plan's own entries follow it.

use tera::Context;

use crate::domain::plan::Plan;
use crate::generation::approvals::render_approvals_section;

const DEFAULT_STOPS: [&str; 5] = [
    "before deleting important data",
    "before force-pushing",
    "before modifying external systems",
    "before any other destructive operation",
    "before a decision that needs the human's approval",
];

/// Markdown body (heading included) for the Stop and ask section.
#[must_use]
pub fn render_stop_section(plan: &Plan) -> String {
    let mut out = String::from("## Stop and ask\n\nStop and ask the human:\n\n");
    for entry in DEFAULT_STOPS
        .iter()
        .copied()
        .chain(plan.orchestrator.stop_when.iter().map(String::as_str))
    {
        out.push_str("- ");
        out.push_str(entry);
        out.push('\n');
    }
    out.push_str("\nKeep working when the next step does not need the human's input.\n");
    out
}

/// Insert `stop_section` and `approvals_section` into a template context.
/// Both sit at every call site that renders the orchestrator rules.
pub fn insert_stop_section(ctx: &mut Context, plan: &Plan) {
    ctx.insert("stop_section", &render_stop_section(plan));
    ctx.insert("approvals_section", &render_approvals_section());
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const PLAN: &str = r#"
[project]
name = "test"
path = "./test"
description = "A test project"
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

    #[test]
    fn default_list_only_when_stop_when_empty() {
        let plan = Plan::from_toml(PLAN).unwrap();
        let s = render_stop_section(&plan);
        assert_eq!(s.matches("\n- ").count(), DEFAULT_STOPS.len());
        assert!(s.contains("Keep working when the next step does not need the human's input."));
    }

    #[test]
    fn custom_entries_follow_defaults() {
        let mut plan = Plan::from_toml(PLAN).unwrap();
        plan.orchestrator.stop_when = vec!["before touching billing".into()];
        let s = render_stop_section(&plan);
        let last_default = s.find("needs the human's approval").unwrap();
        let custom = s.find("before touching billing").unwrap();
        assert!(custom > last_default);
    }
}
