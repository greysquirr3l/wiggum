//! Split command implementation — interactively split oversized tasks.

use std::io::{self, Write};
use std::path::Path;

use crate::adapters::fs::FsAdapter;
use crate::domain::plan::{Plan, TaskDef};
use crate::error::{Result, WiggumError};
use crate::generation::tokens::estimate_tokens;
use crate::ports::PlanReader;

/// Token threshold for oversized task warning.
pub const OVERSIZED_THRESHOLD: usize = 3000;

/// Result of analyzing a task for splitting.
pub struct SplitAnalysis {
    pub slug: String,
    pub title: String,
    pub estimated_tokens: usize,
    pub is_oversized: bool,
    pub dependents: Vec<String>,
}

/// Analyze a task to determine if it should be split.
///
/// # Errors
///
/// Returns an error if the plan cannot be read or the task is not found.
pub fn analyze_task(plan_path: &Path, task_slug: &str) -> Result<SplitAnalysis> {
    let fs = FsAdapter;
    let toml_content = fs.read_plan(plan_path)?;
    let plan = Plan::from_toml(&toml_content)?;
    let resolved = plan.resolve_tasks()?;

    let task = resolved
        .iter()
        .find(|t| t.slug == task_slug)
        .ok_or_else(|| WiggumError::Validation(format!("Task '{task_slug}' not found")))?;

    // Estimate tokens for this task's prompt
    let task_content = format!(
        "{}\n{}\n{}\n{}",
        task.title,
        task.goal,
        task.hints.join("\n"),
        task.test_hints.join("\n")
    );
    let tokens = estimate_tokens(&task_content);

    // Find tasks that depend on this one
    let dependents: Vec<String> = resolved
        .iter()
        .filter(|t| t.depends_on.contains(&task_slug.to_string()))
        .map(|t| t.slug.clone())
        .collect();

    Ok(SplitAnalysis {
        slug: task.slug.clone(),
        title: task.title.clone(),
        estimated_tokens: tokens,
        is_oversized: tokens > OVERSIZED_THRESHOLD,
        dependents,
    })
}

/// Split context for generating the split plan.
pub struct SplitPlan {
    pub original_slug: String,
    pub parts: Vec<SplitPart>,
    pub rewire_dependents: bool,
}

pub struct SplitPart {
    pub slug: String,
    pub goal: String,
    pub depends_on_previous: bool,
}

/// Run interactive split flow.
///
/// # Errors
///
/// Returns an error if the task cannot be found or user input fails.
pub fn run_interactive_split(plan_path: &Path, task_slug: &str) -> Result<SplitPlan> {
    let analysis = analyze_task(plan_path, task_slug)?;

    println!(
        "Task: {} (~{} tokens)",
        analysis.title, analysis.estimated_tokens
    );
    if analysis.is_oversized {
        println!("⚠️  Task exceeds recommended size (~{OVERSIZED_THRESHOLD} tokens)");
    }
    println!();

    // Ask how many parts
    let count = prompt_number("Split into how many tasks?", 2)?;

    let mut parts = Vec::with_capacity(count);

    for i in 0..count {
        println!();
        let default_slug = if i == 0 {
            format!("{task_slug}-core")
        } else if i == count - 1 {
            format!("{task_slug}-final")
        } else {
            format!("{task_slug}-part{}", i + 1)
        };

        let slug = prompt_string(&format!("Task {} slug", i + 1), &default_slug)?;
        let goal = prompt_string(&format!("Task {} goal", i + 1), "")?;

        let depends_on_previous = if i > 0 {
            prompt_yes_no(&format!(
                "{} depends on {}?",
                slug,
                parts.last().map_or("", |p: &SplitPart| &p.slug)
            ))?
        } else {
            false
        };

        parts.push(SplitPart {
            slug,
            goal,
            depends_on_previous,
        });
    }

    // Ask about rewiring dependents
    let rewire = if analysis.dependents.is_empty() {
        false
    } else {
        println!();
        println!(
            "Tasks that depend on {}: {}",
            task_slug,
            analysis.dependents.join(", ")
        );
        prompt_yes_no(&format!(
            "Rewire them to depend on {}?",
            parts.last().map_or("", |p| &p.slug)
        ))?
    };

    Ok(SplitPlan {
        original_slug: task_slug.to_string(),
        parts,
        rewire_dependents: rewire,
    })
}

/// Apply split plan to the plan file.
///
/// # Errors
///
/// Returns an error if the plan cannot be read or written.
pub fn apply_split(plan_path: &Path, split: &SplitPlan) -> Result<String> {
    let fs = FsAdapter;
    let toml_content = fs.read_plan(plan_path)?;

    // Parse and modify
    let mut plan = Plan::from_toml(&toml_content)?;

    // Find the phase containing the original task
    let (phase_idx, task_idx) = plan
        .phases
        .iter()
        .enumerate()
        .find_map(|(pi, phase)| {
            phase
                .tasks
                .iter()
                .position(|t| t.slug == split.original_slug)
                .map(|ti| (pi, ti))
        })
        .ok_or_else(|| {
            WiggumError::Validation(format!("Task '{}' not found", split.original_slug))
        })?;

    if split.parts.len() < 2 {
        return Err(WiggumError::Validation(format!(
            "Cannot split task '{}': split plan must have at least 2 parts, got {}",
            split.original_slug,
            split.parts.len()
        )));
    }

    let phase_count = plan.phases.len();
    let phase = plan.phases.get_mut(phase_idx).ok_or_else(|| {
        WiggumError::Validation(format!(
            "Invalid phase index while splitting task '{}': phase_idx={phase_idx} but phase_count={phase_count}",
            split.original_slug
        ))
    })?;
    let task_count = phase.tasks.len();
    if task_idx >= task_count {
        return Err(WiggumError::Validation(format!(
            "Invalid task index while splitting task '{}': phase_idx={phase_idx}, task_idx={task_idx} but task_count={task_count}",
            split.original_slug
        )));
    }
    let original = phase.tasks.remove(task_idx);
    let new_tasks = build_split_tasks(&original, split);
    for (i, task) in new_tasks.into_iter().enumerate() {
        phase.tasks.insert(task_idx + i, task);
    }

    // Rewire dependents if requested
    if split.rewire_dependents {
        let final_slug = split.parts.last().map_or(&split.original_slug, |p| &p.slug);
        for phase in &mut plan.phases {
            for task in &mut phase.tasks {
                for dependency in &mut task.depends_on {
                    if dependency == &split.original_slug {
                        final_slug.clone_into(dependency);
                    }
                }
            }
        }
    }

    // Serialize back to TOML
    let new_toml =
        toml::to_string_pretty(&plan).map_err(|e| WiggumError::PlanParse(e.to_string()))?;

    // Write back
    std::fs::write(plan_path, &new_toml)?;

    Ok(new_toml)
}

fn build_split_tasks(original: &TaskDef, split: &SplitPlan) -> Vec<TaskDef> {
    let last = split.parts.len() - 1;
    split
        .parts
        .iter()
        .enumerate()
        .map(|(i, part)| {
            let depends_on = if i == 0 {
                original.depends_on.clone()
            } else if part.depends_on_previous {
                split
                    .parts
                    .get(i - 1)
                    .map(|prev| vec![prev.slug.clone()])
                    .unwrap_or_default()
            } else {
                vec![]
            };
            TaskDef {
                slug: part.slug.clone(),
                title: if part.goal.is_empty() {
                    format!("{} (part {})", original.title, i + 1)
                } else {
                    part.goal.clone()
                },
                goal: if part.goal.is_empty() {
                    format!("Part {} of: {}", i + 1, original.goal)
                } else {
                    part.goal.clone()
                },
                depends_on,
                hints: if i == 0 {
                    original.hints.clone()
                } else {
                    vec![]
                },
                test_hints: if i == last {
                    original.test_hints.clone()
                } else {
                    vec![]
                },
                must_haves: if i == last {
                    original.must_haves.clone()
                } else {
                    vec![]
                },
                gate: if i == last {
                    original.gate.clone()
                } else {
                    None
                },
                evaluation_criteria: if i == last {
                    original.evaluation_criteria.clone()
                } else {
                    vec![]
                },
            }
        })
        .collect()
}

fn prompt_string(prompt: &str, default: &str) -> Result<String> {
    if default.is_empty() {
        print!("{prompt}: ");
    } else {
        print!("{prompt} [{default}]: ");
    }
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();

    if trimmed.is_empty() && !default.is_empty() {
        Ok(default.to_string())
    } else if trimmed.is_empty() {
        Err(WiggumError::Validation("Input required".into()))
    } else {
        Ok(trimmed.to_string())
    }
}

fn prompt_number(prompt: &str, default: usize) -> Result<usize> {
    print!("{prompt} [{default}]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();

    if trimmed.is_empty() {
        Ok(default)
    } else {
        trimmed
            .parse()
            .map_err(|_| WiggumError::Validation("Invalid number".into()))
    }
}

fn prompt_yes_no(prompt: &str) -> Result<bool> {
    print!("{prompt} [Y/n]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim().to_lowercase();

    Ok(trimmed.is_empty() || trimmed == "y" || trimmed == "yes")
}

/// Format split preview for dry-run.
#[must_use]
pub fn format_split_preview(analysis: &SplitAnalysis) -> String {
    let mut lines = Vec::new();

    lines.push(format!(
        "Task: {} (~{} tokens)",
        analysis.slug, analysis.estimated_tokens
    ));

    if analysis.is_oversized {
        lines.push(format!(
            "⚠️  Exceeds recommended size (~{OVERSIZED_THRESHOLD} tokens)"
        ));
        lines.push(format!(
            "Suggestion: `wiggum split --task {}`",
            analysis.slug
        ));
    } else {
        lines.push("✅ Within recommended size".to_string());
    }

    if !analysis.dependents.is_empty() {
        lines.push(format!("Dependents: {}", analysis.dependents.join(", ")));
    }

    lines.join("\n")
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::plan::{
        IntegrationConfig, Language, Orchestrator, Phase, Plan, Preflight, Project, SecurityConfig,
        Strategy, StyleConfig, TaskDef,
    };

    #[test]
    fn oversized_threshold_is_reasonable() {
        const { assert!(OVERSIZED_THRESHOLD >= 2000) };
        const { assert!(OVERSIZED_THRESHOLD <= 5000) };
    }

    fn make_plan_file(tasks: Vec<TaskDef>) -> tempfile::NamedTempFile {
        let plan = Plan {
            project: Project {
                name: "test".to_string(),
                description: "test project".to_string(),
                language: Language::Rust,
                path: "/tmp/test".to_string(),
                architecture: None,
            },
            preflight: Preflight::default(),
            orchestrator: Orchestrator {
                persona: "test".to_string(),
                strategy: Strategy::default(),
                rules: Vec::new(),
            },
            evaluator: None,
            security: SecurityConfig::default(),
            integration: IntegrationConfig::default(),
            style: StyleConfig::default(),
            phases: vec![Phase {
                name: "Phase 1".to_string(),
                order: 1,
                tasks,
            }],
        };
        let toml = toml::to_string_pretty(&plan).expect("serializable plan");
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        std::fs::write(tmp.path(), toml).expect("write temp plan");
        tmp
    }

    fn task(slug: &str) -> TaskDef {
        TaskDef {
            slug: slug.to_string(),
            title: format!("Task {slug}"),
            goal: format!("Goal for {slug}"),
            depends_on: Vec::new(),
            hints: Vec::new(),
            test_hints: Vec::new(),
            must_haves: Vec::new(),
            gate: None,
            evaluation_criteria: Vec::new(),
        }
    }

    #[test]
    fn apply_split_rejects_empty_parts() {
        let tmp = make_plan_file(vec![task("t1")]);
        let split = SplitPlan {
            original_slug: "t1".to_string(),
            parts: vec![],
            rewire_dependents: false,
        };
        let err = apply_split(tmp.path(), &split).unwrap_err();
        let message = if let WiggumError::Validation(message) = err {
            message
        } else {
            String::new()
        };
        assert!(!message.is_empty(), "expected validation error variant");
        assert!(
            message.contains("at least 2 parts"),
            "unexpected validation message: {message}"
        );
    }

    #[test]
    fn apply_split_rejects_single_part() {
        let tmp = make_plan_file(vec![task("t1")]);
        let split = SplitPlan {
            original_slug: "t1".to_string(),
            parts: vec![SplitPart {
                slug: "t1a".to_string(),
                goal: "part a".to_string(),
                depends_on_previous: false,
            }],
            rewire_dependents: false,
        };
        let err = apply_split(tmp.path(), &split).unwrap_err();
        let message = if let WiggumError::Validation(message) = err {
            message
        } else {
            String::new()
        };
        assert!(!message.is_empty(), "expected validation error variant");
        assert!(
            message.contains("at least 2 parts"),
            "unexpected validation message: {message}"
        );
    }

    #[test]
    fn apply_split_replaces_task_and_rewires() {
        let mut t2 = task("t2");
        t2.depends_on = vec!["t1".to_string()];
        let tmp = make_plan_file(vec![task("t1"), t2]);
        let split = SplitPlan {
            original_slug: "t1".to_string(),
            parts: vec![
                SplitPart {
                    slug: "t1a".to_string(),
                    goal: "part a".to_string(),
                    depends_on_previous: false,
                },
                SplitPart {
                    slug: "t1b".to_string(),
                    goal: "part b".to_string(),
                    depends_on_previous: true,
                },
            ],
            rewire_dependents: true,
        };
        apply_split(tmp.path(), &split).expect("apply_split succeeded");
        let content = std::fs::read_to_string(tmp.path()).expect("read plan");
        // original task is gone, both parts are present
        assert!(!content.contains("slug = \"t1\""), "original slug found");
        assert!(content.contains("slug = \"t1a\""));
        assert!(content.contains("slug = \"t1b\""));
        // t2 was rewired to final part (t1b)
        assert!(content.contains("t1b"));
        assert!(!content.contains("\"t1\""));
    }
}
