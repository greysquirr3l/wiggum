//! Render `CLAUDE.md` — Claude Code's project memory file.
//!
//! Unlike the existing `.claude/settings.json` hooks (which are a small
//! static JSON file), `CLAUDE.md` is the rich, human-readable project
//! memory Claude Code reads on every session. It carries the project
//! persona, security rules, architecture guidance, and style settings so
//! the human-in-the-loop Claude Code session stays aligned with the plan
//! without needing to read every task file.
//!
//! This module sits alongside `hooks.rs`; both belong to the `claude`
//! target. Together they constitute "full Claude Code support":

use tera::{Context, Tera};

use crate::domain::plan::Plan;
use crate::error::{Result, WiggumError};
use crate::generation::templates::get_tera;

/// Render the `CLAUDE.md` project memory file using the default template.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render(plan: &Plan) -> Result<String> {
    render_with(get_tera(), plan)
}

/// Render `CLAUDE.md` using a custom Tera instance.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_with(tera: &Tera, plan: &Plan) -> Result<String> {
    let mut ctx = Context::new();

    ctx.insert("project_name", &plan.project.name);
    ctx.insert("project_description", &plan.project.description);
    ctx.insert("language", &plan.project.language.to_string());
    ctx.insert("architecture", &plan.project.architecture);
    ctx.insert("preflight_build", &plan.preflight.build);
    ctx.insert("preflight_test", &plan.preflight.test);
    ctx.insert("preflight_lint", &plan.preflight.lint);
    let audit_cmd = plan.preflight.audit.as_deref().unwrap_or("");
    ctx.insert("audit_cmd", &audit_cmd);
    ctx.insert("persona", &plan.orchestrator.persona);
    ctx.insert("rules", &plan.orchestrator.rules);
    ctx.insert("strategy", &plan.orchestrator.strategy.to_string());
    ctx.insert("max_retries", &plan.orchestrator.max_retries);

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

    tera.render("claude.md", &ctx)
        .map_err(|e| WiggumError::Template(e.to_string()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::plan::{
        IntegrationConfig, Language, Orchestrator, Phase, Plan, Preflight, Project, SecurityConfig,
        Strategy, StyleConfig, TaskDef, TaskKind,
    };

    fn sample_plan() -> Plan {
        Plan {
            project: Project {
                name: "test-claude".to_string(),
                description: "A test project for Claude Code memory".to_string(),
                language: Language::Rust,
                path: "/tmp/test-claude".to_string(),
                architecture: Some("hexagonal".to_string()),
            },
            preflight: Preflight {
                build: "cargo build --workspace".to_string(),
                test: "cargo test --workspace".to_string(),
                lint: "cargo clippy --workspace -- -D warnings".to_string(),
                audit: None,
            },
            orchestrator: Orchestrator {
                persona: "You are a senior rust engineer".to_string(),
                strategy: Strategy::Complete,
                rules: vec!["No unwrap() in production code".to_string()],
                ..Default::default()
            },
            evaluator: None,
            security: SecurityConfig::default(),
            integration: IntegrationConfig::default(),
            style: StyleConfig::default(),
            targets: crate::domain::plan::TargetConfig::default(),
            phases: vec![Phase {
                name: "Foundation".to_string(),
                order: 1,
                tasks: vec![TaskDef {
                    slug: "scaffold".to_string(),
                    title: "Project scaffold".to_string(),
                    goal: "Set up the project".to_string(),
                    depends_on: vec![],
                    hints: vec![],
                    test_hints: vec![],
                    must_haves: vec![],
                    gate: None,
                    evaluation_criteria: vec![],
                    kind: TaskKind::default(),
                }],
            }],
        }
    }

    #[test]
    fn render_contains_project_metadata() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(output.contains("test-claude"));
        assert!(output.contains("A test project for Claude Code memory"));
        assert!(output.contains("rust"));
    }

    #[test]
    fn render_starts_with_claude_md_header() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(output.starts_with("# CLAUDE.md"));
    }

    #[test]
    fn render_contains_persona() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(
            output.contains("You are a senior rust engineer"),
            "CLAUDE.md should embed the orchestrator persona"
        );
    }

    #[test]
    fn render_contains_preflight_commands() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(output.contains("cargo build --workspace"));
        assert!(output.contains("cargo test --workspace"));
        assert!(output.contains("cargo clippy --workspace"));
    }

    #[test]
    fn render_contains_security_rules() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(
            output.contains("hardcoded credentials")
                || output.contains("parameterised queries")
                || output.contains("SQL injection"),
            "expected OWASP-derived security rule to appear"
        );
    }

    #[test]
    fn render_contains_strict_rules_when_enabled() {
        let mut plan = sample_plan();
        plan.style.strict = true;
        let output = render(&plan).unwrap();
        assert!(output.contains(".unwrap()") || output.contains(".expect()"));
    }

    #[test]
    fn render_mentions_precompact_hook() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(
            output.contains("PreCompact") || output.contains("compact"),
            "CLAUDE.md should mention the companion .claude/settings.json hook"
        );
    }

    #[test]
    fn render_contains_architecture_block() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(output.contains("Hexagonal") || output.contains("hexagonal"));
    }

    #[test]
    fn render_contains_user_rules() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(output.contains("No unwrap() in production code"));
    }

    #[test]
    fn render_omits_strict_block_when_disabled() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(!output.contains("Strict project standards"));
    }
}
