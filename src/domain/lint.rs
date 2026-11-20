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
    lint_no_verification(resolved, &mut diagnostics);
    lint_weak_criteria(resolved, &mut diagnostics);
    lint_gate_without_scope(resolved, &mut diagnostics);
    lint_gate_scope_without_gate(resolved, &mut diagnostics);
    lint_evaluator_not_independent(plan, &mut diagnostics);
    lint_capability_unused(plan, &mut diagnostics);
    lint_capability_no_scenarios(plan, &mut diagnostics);
    lint_capability_no_requirements(plan, &mut diagnostics);

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

/// Warning: a task with no `verify` commands, no `evaluation_criteria` and no
/// `must_haves` gives the orchestrator nothing to check beyond preflight.
fn lint_no_verification(tasks: &[ResolvedTask], out: &mut Vec<Diagnostic>) {
    for t in tasks {
        if t.verify.is_empty() && t.evaluation_criteria.is_empty() && t.must_haves.is_empty() {
            out.push(Diagnostic {
                severity: Severity::Warning,
                rule: "no-verification",
                message: format!(
                    "T{:02}-{}: no verify, evaluation_criteria or must_haves — completion can only be judged by preflight",
                    t.number, t.slug
                ),
            });
        }
    }
}

/// Info: a gated task whose approval has no recorded scope. Without one, a
/// confirmation given for a test run can be read as covering a deployment.
fn lint_gate_without_scope(tasks: &[ResolvedTask], out: &mut Vec<Diagnostic>) {
    for t in tasks {
        if t.gate.is_some() && t.effective_gate_scope().is_none() {
            out.push(Diagnostic {
                severity: Severity::Info,
                rule: "gate-without-scope",
                message: format!(
                    "T{:02}-{}: gate has no gate_scope — name the task and environment the approval covers",
                    t.number, t.slug
                ),
            });
        }
    }
}

/// Info: `gate_scope` on a task with no `gate`. The scope is only rendered in
/// the gate banner, so it is silently dropped. Info rather than warning: it is
/// harmless to output and usually means the `gate` line was removed or not yet
/// written.
fn lint_gate_scope_without_gate(tasks: &[ResolvedTask], out: &mut Vec<Diagnostic>) {
    for t in tasks {
        if t.gate.is_none() && t.effective_gate_scope().is_some() {
            out.push(Diagnostic {
                severity: Severity::Info,
                rule: "gate-scope-without-gate",
                message: format!(
                    "T{:02}-{}: gate_scope is set but the task has no gate, so the scope is not rendered",
                    t.number, t.slug
                ),
            });
        }
    }
}

/// True when a criterion names something checkable. The heuristic is
/// deliberately crude: a backtick span (a command or identifier), a path
/// separator, a `name.ext` file token, a `--flag` or `::` path (unquoted
/// commands and identifiers), or any digit (a measurable threshold) counts as
/// concrete. Anything else is treated as prose.
fn criterion_is_concrete(criterion: &str) -> bool {
    if criterion.contains('`')
        || criterion.contains('/')
        || criterion.contains(" --")
        || criterion.contains("::")
    {
        return true;
    }
    if criterion.chars().any(|c| c.is_ascii_digit()) {
        return true;
    }
    criterion.split_whitespace().any(|word| {
        let word = word.trim_matches(|c: char| !c.is_alphanumeric());
        word.split_once('.').is_some_and(|(stem, ext)| {
            !stem.is_empty()
                && (1..=5).contains(&ext.len())
                && ext.chars().all(|c| c.is_ascii_alphanumeric())
        })
    })
}

/// Info: `evaluation_criteria` entries with no command, file path or
/// measurable condition (see [`criterion_is_concrete`]).
fn lint_weak_criteria(tasks: &[ResolvedTask], out: &mut Vec<Diagnostic>) {
    for t in tasks {
        for criterion in &t.evaluation_criteria {
            if !criterion_is_concrete(criterion) {
                out.push(Diagnostic {
                    severity: Severity::Info,
                    rule: "weak-criteria",
                    message: format!(
                        "T{:02}-{}: criterion \"{criterion}\" names no command, file path or measurable condition",
                        t.number, t.slug
                    ),
                });
            }
        }
    }
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

/// Normalise a model name for comparison: trimmed, lower-cased, and `None`
/// when empty.
fn normalized_model(model: Option<&String>) -> Option<String> {
    model
        .map(|m| m.trim().to_lowercase())
        .filter(|m| !m.is_empty())
}

/// Warning: The evaluator is configured but is not guaranteed to run on a
/// different model from the implementer. With no `[evaluator] model` the
/// evaluator inherits the dispatcher's model, and with the same model as
/// `[orchestrator] subagent_model` the assistant grades its own work. A plan
/// with no `[evaluator]` at all is not reported here.
fn lint_evaluator_not_independent(plan: &Plan, out: &mut Vec<Diagnostic>) {
    let Some(evaluator) = &plan.evaluator else {
        return;
    };
    let eval_model = normalized_model(evaluator.model.as_ref());
    let impl_model = normalized_model(plan.orchestrator.subagent_model.as_ref());
    let message = match eval_model {
        None => "evaluator has no `[evaluator] model`, so it inherits the dispatcher's model and the same model writes and grades the work; set `[evaluator] model` to a different model than `[orchestrator] subagent_model`".to_string(),
        Some(m) if Some(&m) == impl_model.as_ref() => format!(
            "`[evaluator] model` (`{}`) is the same as `[orchestrator] subagent_model`, so the model that writes the code also grades it; set `[evaluator] model` to a different model",
            evaluator.model.as_deref().unwrap_or_default().trim()
        ),
        Some(_) => return,
    };
    out.push(Diagnostic {
        severity: Severity::Warning,
        rule: "evaluator-not-independent",
        message,
    });
}

/// Warning: A capability is declared but no task references it via
/// `implements = [...]`. Dead capability — the contract exists but no
/// task is accountable for satisfying it.
fn lint_capability_unused(plan: &Plan, out: &mut Vec<Diagnostic>) {
    if plan.capabilities.is_empty() {
        return;
    }
    let referenced: std::collections::HashSet<&str> = plan
        .phases
        .iter()
        .flat_map(|p| p.tasks.iter())
        .flat_map(|t| t.implements.iter())
        .map(String::as_str)
        .collect();
    for cap in &plan.capabilities {
        if !referenced.contains(cap.name.as_str()) {
            out.push(Diagnostic {
                severity: Severity::Warning,
                rule: "capability-unused",
                message: format!(
                    "capability `{}` is declared but no task has `implements = [...]` referencing it",
                    cap.name
                ),
            });
        }
    }
}

/// Warning: A capability has no `[[capabilities.scenarios]]` entries.
/// Scenarios are the machine-readable acceptance contract; without them
/// the evaluator has nothing to verify against.
fn lint_capability_no_scenarios(plan: &Plan, out: &mut Vec<Diagnostic>) {
    for cap in &plan.capabilities {
        if cap.scenarios.is_empty() {
            out.push(Diagnostic {
                severity: Severity::Warning,
                rule: "capability-no-scenarios",
                message: format!(
                    "capability `{}` has no scenarios — the evaluator cannot verify completion",
                    cap.name
                ),
            });
        }
    }
}

/// Info: A capability has no `requirements` bullets. Requirements are
/// the prose contract; scenarios are the testable form. Both are useful.
fn lint_capability_no_requirements(plan: &Plan, out: &mut Vec<Diagnostic>) {
    for cap in &plan.capabilities {
        if cap.requirements.is_empty() {
            out.push(Diagnostic {
                severity: Severity::Info,
                rule: "capability-no-requirements",
                message: format!(
                    "capability `{}` has no requirements — consider adding prose bullets",
                    cap.name
                ),
            });
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::domain::plan::{
        IntegrationConfig, Language, Orchestrator, Phase, Plan, Preflight, Project, SecurityConfig,
        Strategy, StyleConfig, TaskDef, TaskKind,
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
                ..Default::default()
            },
            evaluator: None,
            security: SecurityConfig::default(),
            integration: IntegrationConfig::default(),
            style: StyleConfig::default(),
            targets: crate::domain::plan::TargetConfig::default(),
            capabilities: Vec::new(),
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
                gate_scope: t.gate_scope,
                evaluation_criteria: t.evaluation_criteria,
                verify: t.verify,
                phase_name: "Phase 1".to_string(),
                phase_order: 1,
                kind: t.kind,
                implements: t.implements,
                anchors: vec![],
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
            gate_scope: None,
            evaluation_criteria: Vec::new(),
            verify: Vec::new(),
            kind: TaskKind::default(),
            implements: Vec::new(),
            anchors: vec![],
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
                    gate_scope: None,
                    evaluation_criteria: Vec::new(),
                    verify: Vec::new(),
                    kind: TaskKind::default(),
                    implements: Vec::new(),
                    anchors: vec![],
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
                gate_scope: None,
                evaluation_criteria: Vec::new(),
                verify: vec!["cargo build".to_string()],
                kind: TaskKind::default(),
                implements: Vec::new(),
                anchors: vec![],
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
                gate_scope: None,
                evaluation_criteria: Vec::new(),
                verify: vec!["cargo test".to_string()],
                kind: TaskKind::default(),
                implements: Vec::new(),
                anchors: vec![],
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
    fn gate_without_scope_is_info() {
        let mut t = task("deploy", "Ship it", &[]);
        t.gate = Some("deploy".into());
        let (plan, resolved) = make_plan(vec![t], Some("hexagonal".into()));
        let diags = lint_plan(&plan, &resolved);
        assert!(
            diags
                .iter()
                .any(|d| d.rule == "gate-without-scope" && d.severity == Severity::Info)
        );
    }

    #[test]
    fn gate_without_scope_quiet_when_scoped_or_ungated() {
        let mut scoped = task("deploy", "Ship it", &[]);
        scoped.gate = Some("deploy".into());
        scoped.gate_scope = Some("T01, staging only".into());
        let ungated = task("build", "Build it", &[]);
        let (plan, resolved) = make_plan(vec![scoped, ungated], Some("hexagonal".into()));
        let diags = lint_plan(&plan, &resolved);
        assert!(!diags.iter().any(|d| d.rule == "gate-without-scope"));
    }

    #[test]
    fn gate_without_scope_flags_blank_scope() {
        let mut t = task("deploy", "Ship it", &[]);
        t.gate = Some("deploy".into());
        t.gate_scope = Some("  ".into());
        let (plan, resolved) = make_plan(vec![t], Some("hexagonal".into()));
        let diags = lint_plan(&plan, &resolved);
        assert!(diags.iter().any(|d| d.rule == "gate-without-scope"));
    }

    #[test]
    fn gate_scope_without_gate_is_info() {
        let mut t = task("deploy", "Ship it", &[]);
        t.gate_scope = Some("T01, staging only".into());
        let (plan, resolved) = make_plan(vec![t], Some("hexagonal".into()));
        let diags = lint_plan(&plan, &resolved);
        assert!(
            diags
                .iter()
                .any(|d| d.rule == "gate-scope-without-gate" && d.severity == Severity::Info)
        );
    }

    #[test]
    fn gate_scope_without_gate_quiet_when_gated_blank_or_absent() {
        let mut gated = task("deploy", "Ship it", &[]);
        gated.gate = Some("deploy".into());
        gated.gate_scope = Some("T01, staging only".into());
        let mut blank = task("build", "Build it", &[]);
        blank.gate_scope = Some("   ".into());
        let ordinary = task("lint", "Lint it", &[]);
        let (plan, resolved) = make_plan(vec![gated, blank, ordinary], Some("hexagonal".into()));
        let diags = lint_plan(&plan, &resolved);
        assert!(!diags.iter().any(|d| d.rule == "gate-scope-without-gate"));
    }

    #[test]
    fn no_verification_warns_when_nothing_to_check() {
        let (plan, resolved) = make_plan(vec![task("a", "do a", &[])], None);
        let diags = lint_plan(&plan, &resolved);
        assert!(
            diags
                .iter()
                .any(|d| d.rule == "no-verification" && d.severity == Severity::Warning)
        );
    }

    #[test]
    fn no_verification_quiet_with_verify_criteria_or_must_haves() {
        let mut with_verify = task("a", "do a", &[]);
        with_verify.verify = vec!["cargo test".to_string()];
        let mut with_criteria = task("b", "do b", &[]);
        with_criteria.evaluation_criteria = vec!["prose only".to_string()];
        let mut with_must_haves = task("c", "do c", &[]);
        with_must_haves.must_haves = vec!["a thing".to_string()];
        let (plan, resolved) = make_plan(vec![with_verify, with_criteria, with_must_haves], None);
        let diags = lint_plan(&plan, &resolved);
        assert!(!diags.iter().any(|d| d.rule == "no-verification"));
    }

    #[test]
    fn weak_criteria_flags_prose_only() {
        let mut t = task("a", "do a", &[]);
        t.evaluation_criteria = vec!["The code is clean and well designed".to_string()];
        let (plan, resolved) = make_plan(vec![t], None);
        let diags = lint_plan(&plan, &resolved);
        assert!(
            diags
                .iter()
                .any(|d| d.rule == "weak-criteria" && d.severity == Severity::Info)
        );
    }

    #[test]
    fn weak_criteria_accepts_commands_paths_and_numbers() {
        let mut t = task("a", "do a", &[]);
        t.evaluation_criteria = vec![
            "`cargo test` passes".to_string(),
            "src/main.rs exists".to_string(),
            "Writes config.toml".to_string(),
            "p95 latency under 200ms".to_string(),
        ];
        let (plan, resolved) = make_plan(vec![t], None);
        let diags = lint_plan(&plan, &resolved);
        assert!(!diags.iter().any(|d| d.rule == "weak-criteria"));
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

    fn eval_plan(eval_model: Option<&str>, sub_model: Option<&str>, with_eval: bool) -> Plan {
        let (mut plan, _) = make_plan(vec![task("t1", "build it", &[])], None);
        if with_eval {
            plan.evaluator = Some(crate::domain::plan::EvaluatorConfig {
                model: eval_model.map(str::to_string),
                ..Default::default()
            });
        }
        plan.orchestrator.subagent_model = sub_model.map(str::to_string);
        plan
    }

    fn independence_diags(plan: &Plan) -> Vec<Diagnostic> {
        lint_plan(plan, &[])
            .into_iter()
            .filter(|d| d.rule == "evaluator-not-independent")
            .collect()
    }

    #[test]
    fn evaluator_model_unset_warns() {
        let d = independence_diags(&eval_plan(None, Some("sonnet"), true));
        assert_eq!(
            d.len(),
            1,
            "expected exactly one evaluator-not-independent diagnostic, got {d:?}"
        );
        assert_eq!(d[0].severity, Severity::Warning);
        assert!(d[0].message.contains("[evaluator] model"));
        assert!(d[0].message.contains("inherits the dispatcher's model"));
    }

    #[test]
    fn evaluator_model_empty_string_counts_as_unset() {
        let d = independence_diags(&eval_plan(Some("  "), Some("sonnet"), true));
        assert_eq!(
            d.len(),
            1,
            "expected exactly one evaluator-not-independent diagnostic, got {d:?}"
        );
        assert!(d[0].message.contains("inherits the dispatcher's model"));
    }

    #[test]
    fn evaluator_model_equal_to_subagent_model_warns() {
        let d = independence_diags(&eval_plan(Some("sonnet"), Some("sonnet"), true));
        assert_eq!(
            d.len(),
            1,
            "expected exactly one evaluator-not-independent diagnostic, got {d:?}"
        );
        assert!(d[0].message.contains("`sonnet`"));
        assert!(d[0].message.contains("[orchestrator] subagent_model"));
    }

    #[test]
    fn evaluator_model_equal_ignoring_case_and_whitespace_warns() {
        let d = independence_diags(&eval_plan(
            Some(" Claude-Sonnet "),
            Some("claude-sonnet"),
            true,
        ));
        assert_eq!(
            d.len(),
            1,
            "expected exactly one evaluator-not-independent diagnostic, got {d:?}"
        );
        let d = independence_diags(&eval_plan(
            Some("claude-sonnet"),
            Some("  CLAUDE-SONNET\t"),
            true,
        ));
        assert_eq!(
            d.len(),
            1,
            "expected exactly one evaluator-not-independent diagnostic, got {d:?}"
        );
    }

    #[test]
    fn evaluator_model_distinct_does_not_warn() {
        assert!(independence_diags(&eval_plan(Some("opus"), Some("sonnet"), true)).is_empty());
    }

    #[test]
    fn evaluator_pinned_with_unset_subagent_model_does_not_warn() {
        assert!(independence_diags(&eval_plan(Some("opus"), None, true)).is_empty());
    }

    #[test]
    fn no_evaluator_does_not_raise_independence_warning() {
        assert!(independence_diags(&eval_plan(None, None, false)).is_empty());
        assert!(independence_diags(&eval_plan(None, Some("sonnet"), false)).is_empty());
    }

    #[test]
    fn both_models_unset_warns() {
        let d = independence_diags(&eval_plan(None, None, true));
        assert_eq!(
            d.len(),
            1,
            "expected exactly one evaluator-not-independent diagnostic, got {d:?}"
        );
        assert!(d[0].message.contains("set `[evaluator] model`"));
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
