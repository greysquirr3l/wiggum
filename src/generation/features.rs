//! Generate `features.json` — a structured JSON task registry.
//!
//! Anthropic's harness research found that models are significantly less likely
//! to incorrectly overwrite or corrupt JSON files compared to Markdown files,
//! making JSON a more reliable medium for tracking pass/fail state across tasks.
//! This module produces a machine-readable companion to `PROGRESS.md`.

use serde::Serialize;

use crate::domain::plan::{Plan, ResolvedTask};
use crate::error::Result;

/// A single verifiable criterion within a task.
#[derive(Debug, Clone, Serialize)]
pub struct Criterion {
    pub label: String,
    pub passes: bool,
}

/// Per-task entry in the features registry.
#[derive(Debug, Clone, Serialize)]
pub struct FeatureTask {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub passes: bool,
    pub criteria: Vec<Criterion>,
}

/// The top-level features registry written to `features.json`.
#[derive(Debug, Clone, Serialize)]
pub struct FeaturesRegistry {
    pub project: String,
    pub tasks: Vec<FeatureTask>,
}

/// Render `features.json` content from a resolved task list.
///
/// Default criteria are injected for every task (build, tests, lint, goal).
/// Any task-specific `evaluation_criteria` from the plan TOML supplement these.
///
/// # Errors
///
/// Returns an error if JSON serialization fails.
pub fn render(plan: &Plan, tasks: &[ResolvedTask]) -> Result<String> {
    let registry = build_registry(plan, tasks);
    Ok(serde_json::to_string_pretty(&registry)?)
}

fn build_registry(plan: &Plan, tasks: &[ResolvedTask]) -> FeaturesRegistry {
    let feature_tasks = tasks.iter().map(build_task).collect();
    FeaturesRegistry {
        project: plan.project.name.clone(),
        tasks: feature_tasks,
    }
}

fn build_task(task: &ResolvedTask) -> FeatureTask {
    // Standard exit criteria always present for every task.
    let mut criteria: Vec<Criterion> = vec![
        Criterion {
            label: "build succeeds".to_string(),
            passes: false,
        },
        Criterion {
            label: "all tests pass".to_string(),
            passes: false,
        },
        Criterion {
            label: "linter clean".to_string(),
            passes: false,
        },
        Criterion {
            label: "implementation matches goal".to_string(),
            passes: false,
        },
    ];

    // Task-specific criteria from plan TOML supplement the defaults.
    for label in &task.evaluation_criteria {
        criteria.push(Criterion {
            label: label.clone(),
            passes: false,
        });
    }

    FeatureTask {
        id: format!("T{:02}", task.number),
        slug: task.slug.clone(),
        title: task.title.clone(),
        passes: false,
        criteria,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plan::{
        Language, Orchestrator, Phase, Preflight, Project, ResolvedTask, TaskDef,
    };

    fn make_plan_and_tasks() -> (Plan, Vec<ResolvedTask>) {
        let plan = Plan {
            project: Project {
                name: "test-project".to_string(),
                description: "A test".to_string(),
                language: Language::Rust,
                path: "/tmp/test".to_string(),
                architecture: None,
            },
            preflight: Preflight::default(),
            orchestrator: Orchestrator::default(),
            evaluator: None,
            phases: vec![Phase {
                name: "Foundation".to_string(),
                order: 1,
                tasks: vec![TaskDef {
                    slug: "scaffold".to_string(),
                    title: "Scaffold".to_string(),
                    goal: "Set up the project".to_string(),
                    depends_on: vec![],
                    hints: vec![],
                    test_hints: vec![],
                    must_haves: vec![],
                    gate: None,
                    evaluation_criteria: vec!["README.md exists".to_string()],
                }],
            }],
        };
        let tasks = vec![ResolvedTask {
            number: 1,
            slug: "scaffold".to_string(),
            title: "Scaffold".to_string(),
            goal: "Set up the project".to_string(),
            depends_on: vec![],
            hints: vec![],
            test_hints: vec![],
            must_haves: vec![],
            gate: None,
            evaluation_criteria: vec!["README.md exists".to_string()],
            phase_name: "Foundation".to_string(),
            phase_order: 1,
        }];
        (plan, tasks)
    }

    #[test]
    fn renders_valid_json() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (plan, tasks) = make_plan_and_tasks();
        let json = render(&plan, &tasks)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(
            parsed.get("project").ok_or("project missing")?,
            "test-project"
        );
        Ok(())
    }

    #[test]
    fn task_has_default_criteria()
    -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (plan, tasks) = make_plan_and_tasks();
        let json = render(&plan, &tasks)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        let task = parsed
            .get("tasks")
            .ok_or("tasks missing")?
            .get(0)
            .ok_or("tasks[0] missing")?;
        let criteria = task
            .get("criteria")
            .and_then(|c| c.as_array())
            .ok_or("criteria not array")?;
        // 4 default + 1 from evaluation_criteria
        assert_eq!(criteria.len(), 5);
        assert_eq!(
            criteria
                .first()
                .ok_or("criteria[0] missing")?
                .get("label")
                .ok_or("label missing")?,
            "build succeeds"
        );
        assert_eq!(
            criteria
                .get(4)
                .ok_or("criteria[4] missing")?
                .get("label")
                .ok_or("label missing")?,
            "README.md exists"
        );
        Ok(())
    }

    #[test]
    fn task_id_is_zero_padded() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>
    {
        let (plan, tasks) = make_plan_and_tasks();
        let json = render(&plan, &tasks)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        let task = parsed
            .get("tasks")
            .ok_or("tasks missing")?
            .get(0)
            .ok_or("tasks[0] missing")?;
        assert_eq!(task.get("id").ok_or("id missing")?, "T01");
        Ok(())
    }

    #[test]
    fn all_tasks_start_not_passing()
    -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (plan, tasks) = make_plan_and_tasks();
        let json = render(&plan, &tasks)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        let task = parsed
            .get("tasks")
            .ok_or("tasks missing")?
            .get(0)
            .ok_or("tasks[0] missing")?;
        assert_eq!(task.get("passes").ok_or("passes missing")?, false);
        for criterion in task
            .get("criteria")
            .and_then(|c| c.as_array())
            .ok_or("criteria not array")?
        {
            assert_eq!(criterion.get("passes").ok_or("passes missing")?, false);
        }
        Ok(())
    }
}
