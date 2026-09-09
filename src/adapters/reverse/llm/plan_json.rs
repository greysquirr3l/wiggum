//! JSON contract returned by the LLM + translator to [`crate::domain::plan::Plan`].
//!
//! Schema is mirrored 1:1 from [`super::prompt::render_system_prompt`]. If you
//! change one, change the other and add a fixture test in
//! `tests/reverse_llm_test.rs`.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::domain::plan::{Plan, TaskDef, TaskKind};
use crate::error::{Result, WiggumError};

/// Top-level shape the LLM must emit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmPlan {
    pub phases: Vec<LlmPhase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmPhase {
    pub name: String,
    pub tasks: Vec<LlmTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTask {
    pub slug: String,
    pub title: String,
    pub goal: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub hints: Vec<String>,
    #[serde(default)]
    pub gate: Option<String>,
}

/// Parse the raw LLM text into [`LlmPlan`].
///
/// Tolerant: strips surrounding markdown fences and trailing prose that some
/// models sneak in despite the "ONLY the JSON" instruction.
///
/// # Errors
///
/// Returns an error if the JSON is missing, unparseable, or fails schema
/// validation (duplicate slugs, unknown dep targets, empty slugs).
pub fn parse_llm_plan(text: &str) -> Result<LlmPlan> {
    let json = extract_json(text);
    let plan: LlmPlan = serde_json::from_str(&json).map_err(|e| {
        WiggumError::Validation(format!("LLM plan JSON parse failed: {e}; body={json}"))
    })?;
    validate(&plan)?;
    Ok(plan)
}

/// Validate cross-task invariants.
fn validate(plan: &LlmPlan) -> Result<()> {
    let mut seen_slugs: HashSet<String> = HashSet::new();
    let mut all_slugs: HashSet<String> = HashSet::new();

    for ph in &plan.phases {
        for t in &ph.tasks {
            if t.slug.is_empty() {
                return Err(WiggumError::Validation(
                    "LLM plan has a task with empty slug".to_string(),
                ));
            }
            if !is_kebab_case(&t.slug) {
                return Err(WiggumError::Validation(format!(
                    "LLM plan task slug '{slug}' is not kebab-case",
                    slug = t.slug
                )));
            }
            if !seen_slugs.insert(t.slug.clone()) {
                return Err(WiggumError::Validation(format!(
                    "LLM plan has duplicate task slug '{slug}'",
                    slug = t.slug
                )));
            }
            all_slugs.insert(t.slug.clone());
        }
    }

    for ph in &plan.phases {
        for t in &ph.tasks {
            for dep in &t.depends_on {
                if !all_slugs.contains(dep) {
                    return Err(WiggumError::Validation(format!(
                        "LLM plan task '{slug}' depends on unknown slug '{dep}'",
                        slug = t.slug
                    )));
                }
                if dep == &t.slug {
                    return Err(WiggumError::Validation(format!(
                        "LLM plan task '{slug}' depends on itself",
                        slug = t.slug
                    )));
                }
            }
        }
    }

    Ok(())
}

fn is_kebab_case(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !s.starts_with('-')
        && !s.ends_with('-')
        && !s.contains("--")
}

/// Replace the plan's phases with those from the LLM.
///
/// Preserves the project / orchestrator / preflight that the deterministic
/// scan already filled in; only the phase + task list is rewritten.
pub fn apply_llm_plan(plan: &mut Plan, llm: &LlmPlan) {
    let mut next_order: u32 = 1;
    let new_phases: Vec<_> = llm
        .phases
        .iter()
        .map(|p| crate::domain::plan::Phase {
            order: {
                let o = next_order;
                next_order += 1;
                o
            },
            name: p.name.clone(),
            tasks: p
                .tasks
                .iter()
                .map(|t| TaskDef {
                    slug: t.slug.clone(),
                    title: t.title.clone(),
                    goal: t.goal.clone(),
                    depends_on: t.depends_on.clone(),
                    hints: t.hints.clone(),
                    test_hints: Vec::new(),
                    must_haves: Vec::new(),
                    gate: t.gate.clone(),
                    evaluation_criteria: Vec::new(),
                    kind: TaskKind::default(),
                })
                .collect(),
        })
        .collect();

    plan.phases = new_phases;
}

/// Pull a JSON object out of a string that might have surrounding prose or
/// markdown fences.
fn extract_json(text: &str) -> String {
    let trimmed = text.trim();

    // Strip ``` fences if present.
    let stripped = trimmed.strip_prefix("```").map_or(trimmed, |rest| {
        // Drop the optional language tag on the first fence line.
        let without_lang = rest.find('\n').map_or(rest, |i| &rest[i + 1..]);
        without_lang
            .strip_suffix("```")
            .unwrap_or(without_lang)
            .trim()
    });

    // Find the outermost { ... } block.
    let start = stripped.find('{');
    let end = stripped.rfind('}');
    match (start, end) {
        (Some(s), Some(e)) if e > s => stripped[s..=e].to_string(),
        _ => stripped.to_string(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_plan() {
        let text = r#"{
            "phases": [
                {
                    "name": "Domain Core",
                    "tasks": [
                        {
                            "slug": "define-aggregates",
                            "title": "Define aggregates",
                            "goal": "Identify core aggregates",
                            "depends_on": [],
                            "hints": [],
                            "gate": null
                        }
                    ]
                }
            ]
        }"#;
        let plan = parse_llm_plan(text).unwrap();
        assert_eq!(plan.phases.len(), 1);
        assert_eq!(plan.phases[0].tasks[0].slug, "define-aggregates");
    }

    #[test]
    fn parse_strips_markdown_fences() {
        let text = "```json\n{\"phases\":[]}\n```";
        let plan = parse_llm_plan(text).unwrap();
        assert!(plan.phases.is_empty());
    }

    #[test]
    fn parse_strips_surrounding_prose() {
        let text = "Here you go:\n{\"phases\":[]}\nHope that helps!";
        let plan = parse_llm_plan(text).unwrap();
        assert!(plan.phases.is_empty());
    }

    #[test]
    fn rejects_duplicate_slugs() {
        let text = r#"{
            "phases": [
                {"name":"P1","tasks":[{"slug":"x","title":"t","goal":"g","depends_on":[],"hints":[],"gate":null}]},
                {"name":"P2","tasks":[{"slug":"x","title":"t","goal":"g","depends_on":[],"hints":[],"gate":null}]}
            ]
        }"#;
        assert!(parse_llm_plan(text).is_err());
    }

    #[test]
    fn rejects_non_kebab_slugs() {
        let text = r#"{
            "phases": [
                {"name":"P","tasks":[{"slug":"NotKebab","title":"t","goal":"g","depends_on":[],"hints":[],"gate":null}]}
            ]
        }"#;
        assert!(parse_llm_plan(text).is_err());
    }

    #[test]
    fn rejects_unknown_dependency() {
        let text = r#"{
            "phases": [
                {"name":"P","tasks":[{"slug":"a","title":"t","goal":"g","depends_on":["nonexistent"],"hints":[],"gate":null}]}
            ]
        }"#;
        assert!(parse_llm_plan(text).is_err());
    }

    #[test]
    fn rejects_self_dependency() {
        let text = r#"{
            "phases": [
                {"name":"P","tasks":[{"slug":"a","title":"t","goal":"g","depends_on":["a"],"hints":[],"gate":null}]}
            ]
        }"#;
        assert!(parse_llm_plan(text).is_err());
    }

    #[test]
    fn apply_llm_plan_replaces_phases() {
        let mut plan = make_test_plan();
        let llm = LlmPlan {
            phases: vec![LlmPhase {
                name: "Phase A".to_string(),
                tasks: vec![LlmTask {
                    slug: "task-1".to_string(),
                    title: "Task 1".to_string(),
                    goal: "Do thing 1".to_string(),
                    depends_on: vec![],
                    hints: vec![],
                    gate: None,
                }],
            }],
        };
        apply_llm_plan(&mut plan, &llm);
        assert_eq!(plan.phases.len(), 1);
        assert_eq!(plan.phases[0].name, "Phase A");
        assert_eq!(plan.phases[0].tasks[0].slug, "task-1");
    }

    fn make_test_plan() -> crate::domain::plan::Plan {
        use crate::domain::plan::{
            IntegrationConfig, Orchestrator, Preflight, Project, SecurityConfig, StyleConfig,
            TargetConfig,
        };
        crate::domain::plan::Plan {
            project: Project {
                name: "test".to_string(),
                description: String::new(),
                language: crate::domain::plan::Language::Rust,
                path: "<repo>".to_string(),
                architecture: None,
            },
            preflight: Preflight::default(),
            orchestrator: Orchestrator::default(),
            evaluator: None,
            security: SecurityConfig::default(),
            integration: IntegrationConfig::default(),
            style: StyleConfig::default(),
            targets: TargetConfig::default(),
            phases: Vec::new(),
        }
    }
}
