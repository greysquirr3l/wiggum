//! Render fork-neutral rules files for `VSCode`-family IDEs that don't speak
//! the GitHub Copilot `runSubagent` or opencode `task` protocols.
//!
//! Emits three artifacts, all from the same Tera template so the rules
//! stay in lockstep across forks:
//!
//! - `.cursorrules` — Cursor's project-level rules file.
//! - `.windsurfrules` — Windsurf's project-level rules file (same format).
//! - `.github/copilot-instructions.md` — GitHub Copilot repo-level
//!   instructions (also picked up by some `VSCode` forks).
//!
//! Unlike the `vscode` and `opencode` targets, these files contain **rules
//! only** — no orchestrator loop, no subagent dispatch, no per-dispatch
//! model pinning. The receiving IDE drives its own agent loop; wiggum just
//! supplies the security / architecture / style guidance plus project
//! context.
//!
//! Three artifacts, one render function. The template picks the file path
//! at write time; the rendered content is identical for all three.

use tera::{Context, Tera};

use crate::domain::plan::Plan;
use crate::error::{Result, WiggumError};
use crate::generation::templates::get_tera;

/// Render the shared rules content used for `.cursorrules`,
/// `.windsurfrules`, and `.github/copilot-instructions.md` using the
/// default template.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render(plan: &Plan) -> Result<String> {
    render_with(get_tera(), plan)
}

/// Render the shared rules content using a custom Tera instance.
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
    ctx.insert("rules", &plan.orchestrator.rules);
    ctx.insert("strategy", &plan.orchestrator.strategy.to_string());

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

    tera.render("agent_rules.md", &ctx)
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
                name: "test-rules".to_string(),
                description: "A test project for fork-neutral rules".to_string(),
                language: Language::Rust,
                path: "/tmp/test-rules".to_string(),
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
                strategy: Strategy::Tdd,
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
        assert!(output.contains("test-rules"));
        assert!(output.contains("A test project for fork-neutral rules"));
        assert!(output.contains("rust"));
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
    fn render_contains_security_rules_by_default() {
        // security_rules are always injected (not gated by `avoid_ai_patterns`).
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(
            output.contains("hardcoded credentials")
                || output.contains("parameterised queries")
                || output.contains("SQL injection"),
            "expected OWASP-derived security rule to appear; got first 400 chars:\n{}",
            &output[..output.len().min(400)]
        );
    }

    #[test]
    fn render_contains_strict_rules_when_enabled() {
        let mut plan = sample_plan();
        plan.style.strict = true;
        let output = render(&plan).unwrap();
        // Rust profile's strict rules always reference .unwrap() / .expect().
        assert!(output.contains(".unwrap()") || output.contains(".expect()"));
    }

    #[test]
    fn render_omits_strict_rules_when_disabled() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        // The Rust profile's distinctive "no .unwrap() / .expect()" rule must
        // not appear when strict mode is off (the default).
        assert!(!output.contains(".unwrap()") || output.contains("strict"));
        // Simpler check: when strict=false, the strict block header shouldn't render.
        assert!(!output.contains("Strict project standards"));
    }

    #[test]
    fn render_contains_architecture_block_for_hexagonal() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(output.contains("Hexagonal") || output.contains("hexagonal"));
        assert!(output.contains("Domain layer") || output.contains("port trait"));
    }

    #[test]
    fn render_contains_user_rules() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(output.contains("No unwrap() in production code"));
    }

    #[test]
    fn render_does_not_contain_orchestrator_loop_directives() {
        // Agent rules are rules-only — no `runSubagent`, no Copilot Chat
        // references, no opencode `task` tool mentions. The receiving IDE
        // owns its own loop.
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(
            !output.contains("runSubagent"),
            "agent rules must not reference runSubagent"
        );
        assert!(
            !output.contains("Copilot Chat"),
            "agent rules must not reference Copilot Chat picker"
        );
        assert!(
            !output.contains("`task` tool"),
            "agent rules must not reference opencode task tool"
        );
    }

    #[test]
    fn render_omits_architecture_block_when_unset() {
        let mut plan = sample_plan();
        plan.project.architecture = None;
        let output = render(&plan).unwrap();
        assert!(!output.contains("## Architecture"));
    }
}
