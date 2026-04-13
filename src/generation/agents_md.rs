use tera::{Context, Tera};

use crate::domain::plan::Plan;
use crate::error::{Result, WiggumError};
use crate::generation::templates::get_tera;

/// Render the AGENTS.md file using the default template.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render(plan: &Plan) -> Result<String> {
    render_with(get_tera(), plan)
}

/// Render the AGENTS.md file using a custom Tera instance.
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
    ctx.insert("rules", &plan.orchestrator.rules);
    ctx.insert("strategy", &plan.orchestrator.strategy.to_string());

    tera.render("agents.md", &ctx)
        .map_err(|e| WiggumError::Template(e.to_string()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::plan::{
        IntegrationConfig, Language, Orchestrator, Phase, Plan, Preflight, Project, SecurityConfig,
        Strategy, StyleConfig, TaskDef,
    };

    fn sample_plan() -> Plan {
        Plan {
            project: Project {
                name: "test-project".to_string(),
                description: "A test project for unit testing".to_string(),
                language: Language::Rust,
                path: "/tmp/test".to_string(),
                architecture: Some("hexagonal".to_string()),
            },
            preflight: Preflight {
                build: "cargo build --workspace".to_string(),
                test: "cargo test --workspace".to_string(),
                lint: "cargo clippy --workspace -- -D warnings".to_string(),
                audit: None,
            },
            orchestrator: Orchestrator {
                persona: "You are a senior rust software engineer".to_string(),
                strategy: Strategy::Tdd,
                rules: vec![
                    "No unwrap() in production code".to_string(),
                    "All public functions need # Errors docs".to_string(),
                ],
            },
            evaluator: None,
            security: SecurityConfig::default(),
            integration: IntegrationConfig::default(),
            style: StyleConfig::default(),
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
                }],
            }],
        }
    }

    #[test]
    fn render_contains_project_name() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(output.contains("test-project"));
    }

    #[test]
    fn render_contains_description() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(output.contains("A test project for unit testing"));
    }

    #[test]
    fn render_contains_setup_commands() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(output.contains("cargo build --workspace"));
        assert!(output.contains("cargo test --workspace"));
        assert!(output.contains("cargo clippy --workspace"));
    }

    #[test]
    fn render_contains_rules() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(output.contains("No unwrap() in production code"));
        assert!(output.contains("All public functions need # Errors docs"));
    }

    #[test]
    fn render_contains_architecture() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(output.contains("Hexagonal"));
        assert!(output.contains("Domain layer"));
    }

    #[test]
    fn render_contains_strategy() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(output.contains("TDD"));
    }

    #[test]
    fn render_no_architecture_omits_section() {
        let mut plan = sample_plan();
        plan.project.architecture = None;
        let output = render(&plan).unwrap();
        assert!(!output.contains("## Architecture"));
    }

    #[test]
    fn render_no_rules_omits_section() {
        let mut plan = sample_plan();
        plan.orchestrator.rules.clear();
        let output = render(&plan).unwrap();
        assert!(!output.contains("## Rules"));
    }

    #[test]
    fn render_standard_strategy_omits_strategy_note() {
        let mut plan = sample_plan();
        plan.orchestrator.strategy = Strategy::Standard;
        let output = render(&plan).unwrap();
        // Standard strategy has no special note
        assert!(!output.contains("TDD"));
        assert!(!output.contains("GSD"));
    }

    #[test]
    fn render_gsd_strategy() {
        let mut plan = sample_plan();
        plan.orchestrator.strategy = Strategy::Gsd;
        let output = render(&plan).unwrap();
        assert!(output.contains("GSD"));
        assert!(output.contains("must-haves"));
    }

    #[test]
    fn render_go_language() {
        let mut plan = sample_plan();
        plan.project.language = Language::Go;
        plan.preflight = Preflight {
            build: "go build ./...".to_string(),
            test: "go test -v ./...".to_string(),
            lint: "go vet ./... && golangci-lint run ./...".to_string(),
            audit: None,
        };
        let output = render(&plan).unwrap();
        assert!(output.contains("go"));
        assert!(output.contains("go build"));
    }

    #[test]
    fn render_starts_with_agents_md_header() {
        let plan = sample_plan();
        let output = render(&plan).unwrap();
        assert!(output.starts_with("# AGENTS.md"));
    }
}
