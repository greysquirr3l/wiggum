use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::domain::plan::{Plan, ResolvedTask};

/// Severity level for a lint diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// A single lint diagnostic.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub rule: &'static str,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let icon = match self.severity {
            Severity::Error => "✗",
            Severity::Warning => "⚠",
            Severity::Info => "ℹ",
        };
        write!(f, "  {icon}  {}", self.message)
    }
}

/// Summary counts from a lint run.
pub struct LintSummary {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

/// Run all lint rules against a plan and its resolved tasks.
#[must_use]
pub fn lint_plan(plan: &Plan, resolved: &[ResolvedTask]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    lint_no_goal(resolved, &mut diagnostics);
    lint_no_hints(resolved, &mut diagnostics);
    lint_no_test_hints(resolved, &mut diagnostics);
    lint_large_task(resolved, &mut diagnostics);
    lint_deep_chain(resolved, &mut diagnostics);
    lint_orphan_task(resolved, &mut diagnostics);
    lint_missing_arch(plan, &mut diagnostics);
    lint_wide_fan_out(resolved, &mut diagnostics);

    diagnostics.sort_by_key(|d| d.severity);
    diagnostics.reverse(); // errors first
    diagnostics
}

/// Compute a summary of diagnostic counts.
#[must_use]
pub fn summarize(diagnostics: &[Diagnostic]) -> LintSummary {
    let mut errors = 0;
    let mut warnings = 0;
    let mut infos = 0;
    for d in diagnostics {
        match d.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
            Severity::Info => infos += 1,
        }
    }
    LintSummary {
        errors,
        warnings,
        infos,
    }
}

/// Format the summary line (e.g. "3 warnings, 1 info").
#[must_use]
pub fn format_summary(summary: &LintSummary) -> String {
    let mut parts = Vec::new();
    if summary.errors > 0 {
        parts.push(format!(
            "{} error{}",
            summary.errors,
            if summary.errors == 1 { "" } else { "s" }
        ));
    }
    if summary.warnings > 0 {
        parts.push(format!(
            "{} warning{}",
            summary.warnings,
            if summary.warnings == 1 { "" } else { "s" }
        ));
    }
    if summary.infos > 0 {
        parts.push(format!(
            "{} info{}",
            summary.infos,
            if summary.infos == 1 { "" } else { "s" }
        ));
    }
    if parts.is_empty() {
        "no issues".to_string()
    } else {
        parts.join(", ")
    }
}

// ─── Lint Rules ─────────────────────────────────────────────────────

/// Memoized depth computation for dependency chains.
fn compute_depth<'a>(
    slug: &'a str,
    deps: &HashMap<&str, &'a [String]>,
    cache: &mut HashMap<&'a str, usize>,
    visited: &mut HashSet<&'a str>,
) -> usize {
    if let Some(&cached) = cache.get(slug) {
        return cached;
    }
    if !visited.insert(slug) {
        return 0; // cycle guard
    }
    let max_parent = deps.get(slug).map_or(0, |parent_slugs| {
        parent_slugs
            .iter()
            .map(|p| compute_depth(p.as_str(), deps, cache, visited) + 1)
            .max()
            .unwrap_or(0)
    });
    cache.insert(slug, max_parent);
    max_parent
}

/// Error: Task has an empty goal.
fn lint_no_goal(tasks: &[ResolvedTask], out: &mut Vec<Diagnostic>) {
    for t in tasks {
        if t.goal.trim().is_empty() {
            out.push(Diagnostic {
                severity: Severity::Error,
                rule: "no-goal",
                message: format!(
                    "T{:02}-{}: empty goal — subagent won't know what to build",
                    t.number, t.slug
                ),
            });
        }
    }
}

/// Warning: Task has no implementation hints.
fn lint_no_hints(tasks: &[ResolvedTask], out: &mut Vec<Diagnostic>) {
    for t in tasks {
        if t.hints.is_empty() {
            out.push(Diagnostic {
                severity: Severity::Warning,
                rule: "no-hints",
                message: format!(
                    "T{:02}-{}: no hints — subagent will improvise implementation",
                    t.number, t.slug
                ),
            });
        }
    }
}

/// Warning: Task has no test hints.
fn lint_no_test_hints(tasks: &[ResolvedTask], out: &mut Vec<Diagnostic>) {
    for t in tasks {
        if t.test_hints.is_empty() {
            out.push(Diagnostic {
                severity: Severity::Warning,
                rule: "no-test-hints",
                message: format!(
                    "T{:02}-{}: no test_hints — testing may be skipped",
                    t.number, t.slug
                ),
            });
        }
    }
}

/// Warning: Task goal exceeds 500 characters.
fn lint_large_task(tasks: &[ResolvedTask], out: &mut Vec<Diagnostic>) {
    const MAX_GOAL_LEN: usize = 500;
    for t in tasks {
        if t.goal.len() > MAX_GOAL_LEN {
            out.push(Diagnostic {
                severity: Severity::Warning,
                rule: "large-task",
                message: format!(
                    "T{:02}-{}: goal is {} chars — consider splitting",
                    t.number,
                    t.slug,
                    t.goal.len()
                ),
            });
        }
    }
}

/// Warning: Critical path exceeds 8 sequential tasks.
fn lint_deep_chain(tasks: &[ResolvedTask], out: &mut Vec<Diagnostic>) {
    const MAX_CHAIN: usize = 8;

    if tasks.is_empty() {
        return;
    }

    // Build dependency lookup: slug -> depends_on slugs
    let deps: HashMap<&str, &[String]> = tasks
        .iter()
        .map(|t| (t.slug.as_str(), t.depends_on.as_slice()))
        .collect();

    // Compute chain depth for each task (memoized)
    let mut depth_cache: HashMap<&str, usize> = HashMap::new();

    for t in tasks {
        let mut visited = HashSet::new();
        compute_depth(t.slug.as_str(), &deps, &mut depth_cache, &mut visited);
    }

    // Find the deepest task and reconstruct the critical path
    let deepest = depth_cache.iter().max_by_key(|&(_, &d)| d);

    if let Some((&deepest_slug, &max_depth)) = deepest
        && max_depth >= MAX_CHAIN
    {
        let path = reconstruct_critical_path(deepest_slug, &deps, &depth_cache);
        out.push(Diagnostic {
            severity: Severity::Warning,
            rule: "deep-chain",
            message: format!(
                "critical path is {} tasks deep ({})",
                max_depth + 1,
                path.join(" → ")
            ),
        });
    }
}

/// Reconstruct the critical path backwards from the deepest task.
fn reconstruct_critical_path<'a>(
    start: &'a str,
    deps: &HashMap<&str, &'a [String]>,
    depth_cache: &HashMap<&'a str, usize>,
) -> Vec<&'a str> {
    let mut path = vec![start];
    let mut current = start;

    loop {
        let parents = match deps.get(current) {
            Some(p) if !p.is_empty() => p,
            _ => break,
        };

        // Pick the parent with the highest depth (i.e. the critical predecessor)
        let best_parent = parents
            .iter()
            .filter_map(|p| depth_cache.get(p.as_str()).map(|&d| (p.as_str(), d)))
            .max_by_key(|(_, d)| *d);

        match best_parent {
            Some((parent_slug, _)) => {
                path.push(parent_slug);
                current = parent_slug;
            }
            None => break,
        }
    }

    path.reverse();
    path
}

/// Warning: Task has no dependents AND depends on nothing — isolated.
fn lint_orphan_task(tasks: &[ResolvedTask], out: &mut Vec<Diagnostic>) {
    if tasks.len() <= 1 {
        return;
    }

    let all_slugs: HashSet<&str> = tasks.iter().map(|t| t.slug.as_str()).collect();
    let depended_on: HashSet<&str> = tasks
        .iter()
        .flat_map(|t| t.depends_on.iter().map(String::as_str))
        .collect();

    for t in tasks {
        let has_dependents = depended_on.contains(t.slug.as_str());
        let has_dependencies = !t.depends_on.is_empty();

        // Only flag if the task is truly isolated — no connections in either direction
        // and it's not the only task
        if !has_dependents && !has_dependencies && all_slugs.len() > 1 {
            out.push(Diagnostic {
                severity: Severity::Warning,
                rule: "orphan-task",
                message: format!(
                    "T{:02}-{}: no dependencies and nothing depends on it — isolated task",
                    t.number, t.slug
                ),
            });
        }
    }
}

/// Info: No architecture specified.
fn lint_missing_arch(plan: &Plan, out: &mut Vec<Diagnostic>) {
    if plan.project.architecture.is_none() {
        out.push(Diagnostic {
            severity: Severity::Info,
            rule: "missing-arch",
            message: "no architecture specified — templates will be generic".to_string(),
        });
    }
}

/// Info: Task has more than 5 direct dependents.
fn lint_wide_fan_out(tasks: &[ResolvedTask], out: &mut Vec<Diagnostic>) {
    const MAX_DEPENDENTS: usize = 5;

    let mut dependent_count: HashMap<&str, usize> = HashMap::new();
    for t in tasks {
        for dep in &t.depends_on {
            *dependent_count.entry(dep.as_str()).or_insert(0) += 1;
        }
    }

    for (slug, count) in &dependent_count {
        if *count > MAX_DEPENDENTS {
            // Find the task number for display
            let number = tasks
                .iter()
                .find(|t| t.slug == *slug)
                .map_or(0, |t| t.number);
            out.push(Diagnostic {
                severity: Severity::Info,
                rule: "wide-fan-out",
                message: format!("T{number:02}-{slug}: {count} dependents — may be a bottleneck"),
            });
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::domain::plan::{
        Language, Orchestrator, Phase, Plan, Preflight, Project, Strategy, TaskDef,
    };

    fn make_plan(tasks: Vec<TaskDef>, architecture: Option<String>) -> (Plan, Vec<ResolvedTask>) {
        let plan = Plan {
            project: Project {
                name: "test".to_string(),
                description: "test project".to_string(),
                language: Language::Rust,
                path: "/tmp/test".to_string(),
                architecture,
            },
            preflight: Preflight::default(),
            orchestrator: Orchestrator {
                persona: "test".to_string(),
                strategy: Strategy::default(),
                rules: Vec::new(),
            },
            phases: vec![Phase {
                name: "Phase 1".to_string(),
                order: 1,
                tasks: tasks.clone(),
            }],
        };
        let resolved: Vec<ResolvedTask> = tasks
            .into_iter()
            .enumerate()
            .map(|(i, t)| ResolvedTask {
                number: u32::try_from(i + 1).unwrap(),
                slug: t.slug,
                title: t.title,
                goal: t.goal,
                depends_on: t.depends_on,
                hints: t.hints,
                test_hints: t.test_hints,
                must_haves: t.must_haves,
                gate: t.gate,
                phase_name: "Phase 1".to_string(),
                phase_order: 1,
            })
            .collect();
        (plan, resolved)
    }

    fn task(slug: &str, goal: &str, deps: &[&str]) -> TaskDef {
        TaskDef {
            slug: slug.to_string(),
            title: format!("Task {slug}"),
            goal: goal.to_string(),
            depends_on: deps.iter().map(|s| (*s).to_string()).collect(),
            hints: Vec::new(),
            test_hints: Vec::new(),
            must_haves: Vec::new(),
            gate: None,
        }
    }

    #[test]
    fn empty_goal_is_error() {
        let (plan, resolved) = make_plan(vec![task("t1", "", &[])], Some("hexagonal".to_string()));
        let diags = lint_plan(&plan, &resolved);
        assert!(
            diags
                .iter()
                .any(|d| d.rule == "no-goal" && d.severity == Severity::Error)
        );
    }

    #[test]
    fn missing_arch_is_info() {
        let (plan, resolved) = make_plan(vec![task("t1", "build it", &[])], None);
        let diags = lint_plan(&plan, &resolved);
        assert!(
            diags
                .iter()
                .any(|d| d.rule == "missing-arch" && d.severity == Severity::Info)
        );
    }

    #[test]
    fn deep_chain_warns() {
        // Build a chain of 10 tasks: t0 -> t1 -> t2 -> ... -> t9
        let tasks: Vec<TaskDef> = (0..10)
            .map(|i| {
                let slug = format!("t{i}");
                let deps = if i == 0 {
                    vec![]
                } else {
                    vec![format!("t{}", i - 1)]
                };
                TaskDef {
                    slug,
                    title: format!("Task {i}"),
                    goal: "do stuff".to_string(),
                    depends_on: deps,
                    hints: vec!["hint".to_string()],
                    test_hints: vec!["test".to_string()],
                    must_haves: Vec::new(),
                    gate: None,
                }
            })
            .collect();
        let (plan, resolved) = make_plan(tasks, Some("hexagonal".to_string()));
        let diags = lint_plan(&plan, &resolved);
        assert!(diags.iter().any(|d| d.rule == "deep-chain"));
    }

    #[test]
    fn no_warnings_for_good_plan() {
        let tasks = vec![
            TaskDef {
                slug: "setup".to_string(),
                title: "Setup".to_string(),
                goal: "scaffold the project".to_string(),
                depends_on: vec![],
                hints: vec!["use cargo init".to_string()],
                test_hints: vec!["verify compiles".to_string()],
                must_haves: Vec::new(),
                gate: None,
            },
            TaskDef {
                slug: "domain".to_string(),
                title: "Domain".to_string(),
                goal: "define types".to_string(),
                depends_on: vec!["setup".to_string()],
                hints: vec!["use serde".to_string()],
                test_hints: vec!["unit test parsing".to_string()],
                must_haves: Vec::new(),
                gate: None,
            },
        ];
        let (plan, resolved) = make_plan(tasks, Some("hexagonal".to_string()));
        let diags = lint_plan(&plan, &resolved);
        let serious = diags
            .iter()
            .filter(|d| d.severity == Severity::Error || d.severity == Severity::Warning)
            .count();
        assert_eq!(serious, 0);
    }

    #[test]
    fn wide_fan_out_detected() {
        // One root task with 6 dependents
        let mut tasks = vec![task("root", "base task", &[])];
        for i in 0..6 {
            let mut t = task(&format!("child{i}"), "do stuff", &["root"]);
            t.hints = vec!["hint".to_string()];
            t.test_hints = vec!["test".to_string()];
            tasks.push(t);
        }
        tasks[0].hints = vec!["hint".to_string()];
        tasks[0].test_hints = vec!["test".to_string()];
        let (plan, resolved) = make_plan(tasks, Some("hex".to_string()));
        let diags = lint_plan(&plan, &resolved);
        assert!(diags.iter().any(|d| d.rule == "wide-fan-out"));
    }

    #[test]
    fn orphan_task_detected() {
        let tasks = vec![
            task("connected", "has deps", &[]),
            task("also-connected", "depends", &["connected"]),
            task("orphan", "alone", &[]),
        ];
        let (plan, resolved) = make_plan(tasks, Some("hex".to_string()));
        let diags = lint_plan(&plan, &resolved);
        let orphan_diags: Vec<_> = diags.iter().filter(|d| d.rule == "orphan-task").collect();
        assert_eq!(orphan_diags.len(), 1);
        assert!(orphan_diags[0].message.contains("orphan"));
    }

    #[test]
    fn summary_format() {
        let diags = vec![
            Diagnostic {
                severity: Severity::Error,
                rule: "no-goal",
                message: "test".to_string(),
            },
            Diagnostic {
                severity: Severity::Warning,
                rule: "no-hints",
                message: "test".to_string(),
            },
            Diagnostic {
                severity: Severity::Warning,
                rule: "no-hints",
                message: "test".to_string(),
            },
            Diagnostic {
                severity: Severity::Info,
                rule: "missing-arch",
                message: "test".to_string(),
            },
        ];
        let summary = summarize(&diags);
        assert_eq!(format_summary(&summary), "1 error, 2 warnings, 1 info");
    }
}
