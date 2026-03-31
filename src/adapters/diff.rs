//! Diff command implementation — compare two plan.toml files.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::adapters::fs::FsAdapter;
use crate::domain::plan::{Plan, ResolvedTask};
use crate::error::Result;
use crate::ports::PlanReader;

/// Represents a change between two plans.
#[derive(Debug)]
pub enum Change {
    PhaseAdded(String),
    PhaseRemoved(String),
    PhaseOrderChanged { name: String, old: u32, new: u32 },
    TaskAdded { slug: String, phase: String },
    TaskRemoved { slug: String, phase: String },
    TaskModified(TaskDiff),
}

/// Detailed diff for a modified task.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default)]
pub struct TaskDiff {
    pub slug: String,
    pub goal_changed: bool,
    pub title_changed: bool,
    pub hints_added: usize,
    pub hints_removed: usize,
    pub deps_added: Vec<String>,
    pub deps_removed: Vec<String>,
    pub gate_added: bool,
    pub gate_removed: bool,
}

impl TaskDiff {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        !self.goal_changed
            && !self.title_changed
            && self.hints_added == 0
            && self.hints_removed == 0
            && self.deps_added.is_empty()
            && self.deps_removed.is_empty()
            && !self.gate_added
            && !self.gate_removed
    }
}

/// Compare two plans and return list of changes.
///
/// # Errors
///
/// Returns an error if either plan file cannot be read or parsed.
pub fn diff_plans(old_path: &Path, new_path: &Path) -> Result<Vec<Change>> {
    let fs = FsAdapter;

    let old_toml = fs.read_plan(old_path)?;
    let new_toml = fs.read_plan(new_path)?;

    let old_plan = Plan::from_toml(&old_toml)?;
    let new_plan = Plan::from_toml(&new_toml)?;

    let old_tasks = old_plan.resolve_tasks()?;
    let new_tasks = new_plan.resolve_tasks()?;

    let mut changes = Vec::new();

    // Compare phases
    let old_phases: HashMap<&str, u32> = old_plan
        .phases
        .iter()
        .map(|p| (p.name.as_str(), p.order))
        .collect();
    let new_phases: HashMap<&str, u32> = new_plan
        .phases
        .iter()
        .map(|p| (p.name.as_str(), p.order))
        .collect();

    for (name, &old_order) in &old_phases {
        if let Some(&new_order) = new_phases.get(name) {
            if old_order != new_order {
                changes.push(Change::PhaseOrderChanged {
                    name: (*name).to_string(),
                    old: old_order,
                    new: new_order,
                });
            }
        } else {
            changes.push(Change::PhaseRemoved((*name).to_string()));
        }
    }

    for name in new_phases.keys() {
        if !old_phases.contains_key(name) {
            changes.push(Change::PhaseAdded((*name).to_string()));
        }
    }

    // Compare tasks
    let old_by_slug: HashMap<&str, &ResolvedTask> =
        old_tasks.iter().map(|t| (t.slug.as_str(), t)).collect();
    let new_by_slug: HashMap<&str, &ResolvedTask> =
        new_tasks.iter().map(|t| (t.slug.as_str(), t)).collect();

    for (slug, old_task) in &old_by_slug {
        if let Some(new_task) = new_by_slug.get(slug) {
            let diff = diff_task(old_task, new_task);
            if !diff.is_empty() {
                changes.push(Change::TaskModified(diff));
            }
        } else {
            changes.push(Change::TaskRemoved {
                slug: (*slug).to_string(),
                phase: old_task.phase_name.clone(),
            });
        }
    }

    for (slug, new_task) in &new_by_slug {
        if !old_by_slug.contains_key(slug) {
            changes.push(Change::TaskAdded {
                slug: (*slug).to_string(),
                phase: new_task.phase_name.clone(),
            });
        }
    }

    Ok(changes)
}

fn diff_task(old: &ResolvedTask, new: &ResolvedTask) -> TaskDiff {
    let old_deps: HashSet<_> = old.depends_on.iter().collect();
    let new_deps: HashSet<_> = new.depends_on.iter().collect();

    TaskDiff {
        slug: old.slug.clone(),
        goal_changed: old.goal != new.goal,
        title_changed: old.title != new.title,
        hints_added: new.hints.len().saturating_sub(old.hints.len()),
        hints_removed: old.hints.len().saturating_sub(new.hints.len()),
        deps_added: new_deps
            .difference(&old_deps)
            .map(|s| (*s).clone())
            .collect(),
        deps_removed: old_deps
            .difference(&new_deps)
            .map(|s| (*s).clone())
            .collect(),
        gate_added: old.gate.is_none() && new.gate.is_some(),
        gate_removed: old.gate.is_some() && new.gate.is_none(),
    }
}

/// Format changes for human-readable output.
#[must_use]
pub fn format_diff(changes: &[Change]) -> String {
    if changes.is_empty() {
        return "No changes detected.".to_string();
    }

    let mut lines = Vec::new();

    // Phases section
    let phase_changes: Vec<_> = changes
        .iter()
        .filter(|c| {
            matches!(
                c,
                Change::PhaseAdded(_) | Change::PhaseRemoved(_) | Change::PhaseOrderChanged { .. }
            )
        })
        .collect();

    if !phase_changes.is_empty() {
        lines.push("Phases:".to_string());
        for change in phase_changes {
            match change {
                Change::PhaseAdded(name) => lines.push(format!("  + {name}")),
                Change::PhaseRemoved(name) => lines.push(format!("  - {name}")),
                Change::PhaseOrderChanged { name, old, new } => {
                    lines.push(format!("  ~ \"{name}\" (order {old} → {new})"));
                }
                _ => {}
            }
        }
        lines.push(String::new());
    }

    // Tasks section
    let task_changes: Vec<_> = changes
        .iter()
        .filter(|c| {
            matches!(
                c,
                Change::TaskAdded { .. } | Change::TaskRemoved { .. } | Change::TaskModified(_)
            )
        })
        .collect();

    if !task_changes.is_empty() {
        lines.push("Tasks:".to_string());
        for change in task_changes {
            match change {
                Change::TaskAdded { slug, phase } => {
                    lines.push(format!("  + {slug:<24} [Phase: {phase}]"));
                }
                Change::TaskRemoved { slug, phase } => {
                    lines.push(format!("  - {slug:<24} [Phase: {phase}]"));
                }
                Change::TaskModified(diff) => {
                    let mut mods = Vec::new();
                    if diff.goal_changed {
                        mods.push("goal changed");
                    }
                    if diff.title_changed {
                        mods.push("title changed");
                    }
                    if diff.hints_added > 0 {
                        mods.push("hints added");
                    }
                    if !diff.deps_added.is_empty() || !diff.deps_removed.is_empty() {
                        mods.push("deps changed");
                    }
                    if diff.gate_added {
                        mods.push("gate added");
                    }
                    if diff.gate_removed {
                        mods.push("gate removed");
                    }
                    lines.push(format!("  ~ {:<24} {}", diff.slug, mods.join(", ")));

                    if !diff.deps_added.is_empty() {
                        lines.push(format!(
                            "                             depends_on: +[{}]",
                            diff.deps_added.join(", ")
                        ));
                    }
                    if !diff.deps_removed.is_empty() {
                        lines.push(format!(
                            "                             depends_on: -[{}]",
                            diff.deps_removed.join(", ")
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_diff_empty_when_identical() {
        let task = ResolvedTask {
            number: 1,
            slug: "test".into(),
            title: "Test".into(),
            goal: "Test goal".into(),
            depends_on: vec![],
            hints: vec![],
            test_hints: vec![],
            must_haves: vec![],
            gate: None,
            evaluation_criteria: vec![],
            phase_name: "Phase 1".into(),
            phase_order: 1,
        };
        let diff = diff_task(&task, &task);
        assert!(diff.is_empty());
    }
}
