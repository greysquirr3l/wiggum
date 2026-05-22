//! Plan quality scoring — the `wiggum check` command.
//!
//! Unlike `validate --lint`, which checks structural correctness, `check`
//! scores the *substance* of a plan on five dimensions and produces concrete
//! improvement suggestions. Run it before `generate` to catch bad plans early.

use std::collections::HashSet;

use crate::domain::plan::{Plan, ResolvedTask, TaskKind};
use crate::generation::tokens::estimate_tokens;

// ─── Public surface ───────────────────────────────────────────────────────────

/// Per-dimension score (0–10) and its explanation.
#[derive(Debug, Clone)]
pub struct DimensionScore {
    pub name: &'static str,
    pub score: u8,
    pub verdict: &'static str,
    pub findings: Vec<String>,
}

/// Overall plan quality scorecard.
#[derive(Debug)]
pub struct PlanScore {
    pub dimensions: Vec<DimensionScore>,
    /// Weighted composite (0–10, rounded).
    pub overall: u8,
    /// Actionable improvement suggestions.
    pub suggestions: Vec<Suggestion>,
    /// Estimated total prompt tokens across all generated artifacts.
    pub estimated_tokens: usize,
}

impl PlanScore {
    /// `true` when the overall score is high enough to proceed without warnings.
    #[must_use]
    pub const fn is_healthy(&self) -> bool {
        self.overall >= 7
    }
}

/// A concrete improvement suggestion with a severity label.
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub severity: SuggestionSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SuggestionSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for SuggestionSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// Score a plan on all quality dimensions.
///
/// This is a pure function — it performs no I/O.
#[must_use]
pub fn score_plan(plan: &Plan, resolved: &[ResolvedTask]) -> PlanScore {
    let granularity = score_granularity(resolved);
    let dependency = score_dependency_health(resolved);
    let coverage = score_coverage(plan, resolved);
    let richness = score_richness(resolved);
    let token_dim = score_token_budget(resolved);
    let harness = score_harness_complexity(plan, resolved);

    let estimated_tokens = estimated_total_tokens(resolved);

    let mut suggestions = Vec::new();
    collect_suggestions(&granularity, &mut suggestions);
    collect_suggestions(&dependency, &mut suggestions);
    collect_suggestions(&coverage, &mut suggestions);
    collect_suggestions(&richness, &mut suggestions);
    collect_suggestions(&token_dim, &mut suggestions);
    collect_suggestions(&harness, &mut suggestions);

    // Overall: weighted average (granularity 22%, dep 18%, coverage 22%, richness 18%, tokens 10%, harness 10%)
    #[expect(
        clippy::suboptimal_flops,
        reason = "sequential form preserves score boundary semantics relied on by tests"
    )]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let overall = (f64::from(granularity.score) * 0.22
        + f64::from(dependency.score) * 0.18
        + f64::from(coverage.score) * 0.22
        + f64::from(richness.score) * 0.23
        + f64::from(token_dim.score) * 0.10
        + f64::from(harness.score) * 0.05)
        .round() as u8;

    let dimensions = vec![
        granularity,
        dependency,
        coverage,
        richness,
        token_dim,
        harness,
    ];

    PlanScore {
        dimensions,
        overall,
        suggestions,
        estimated_tokens,
    }
}

// ─── Dimension: Granularity ───────────────────────────────────────────────────

/// Scores task granularity.
///
/// - Perfect: goals 80–400 chars, no single phase dominates (>60% tasks).
/// - Penalised: oversized goals, single-task phases, monolithic phases.
fn score_granularity(tasks: &[ResolvedTask]) -> DimensionScore {
    let mut findings = Vec::new();
    let mut penalty = 0u8;

    let total = tasks.len();

    // Oversized goals
    let oversized: Vec<&str> = tasks
        .iter()
        .filter(|t| t.goal.len() > 500)
        .map(|t| t.slug.as_str())
        .collect();
    if !oversized.is_empty() {
        let pct = (oversized.len() * 100) / total.max(1);
        penalty = penalty.saturating_add(u8::try_from((pct / 10).min(4)).unwrap_or(4));
        findings.push(format!(
            "{} task(s) have goals >500 chars ({}): {}",
            oversized.len(),
            pct_label(pct),
            oversized.join(", ")
        ));
    }

    // Undersized goals (likely placeholder)
    let undersized: Vec<&str> = tasks
        .iter()
        .filter(|t| t.goal.trim().len() < 30 && !t.goal.trim().is_empty())
        .map(|t| t.slug.as_str())
        .collect();
    if !undersized.is_empty() {
        penalty = penalty.saturating_add(2);
        findings.push(format!(
            "{} task(s) have very short goals (<30 chars) — likely placeholders: {}",
            undersized.len(),
            undersized.join(", ")
        ));
    }

    // Phase imbalance: any phase containing >60% of tasks
    let phase_counts = phase_task_counts(tasks);
    for (phase, count) in &phase_counts {
        let pct = (*count * 100) / total.max(1);
        if pct > 60 && *count > 3 {
            penalty = penalty.saturating_add(2);
            findings.push(format!(
                "Phase \"{phase}\" contains {count} tasks ({pct}% of total) — consider splitting"
            ));
        }
    }

    // Single-task phases
    let single_task_phases: Vec<&str> = phase_counts
        .iter()
        .filter(|(_, c)| *c == 1)
        .map(|(name, _)| name.as_str())
        .collect();
    if single_task_phases.len() > 1 {
        penalty = penalty.saturating_add(1);
        findings.push(format!(
            "{} phases have only one task — consider merging or expanding: {}",
            single_task_phases.len(),
            single_task_phases.join(", ")
        ));
    }

    score_from_penalty("Granularity", penalty, findings)
}

// ─── Dimension: Dependency health ────────────────────────────────────────────

fn score_dependency_health(tasks: &[ResolvedTask]) -> DimensionScore {
    let mut findings = Vec::new();
    let mut penalty = 0u8;

    let all_slugs: HashSet<&str> = tasks.iter().map(|t| t.slug.as_str()).collect();

    // Orphan tasks: tasks with no dependents and no dependencies (islands)
    let has_dependents: HashSet<&str> = tasks
        .iter()
        .flat_map(|t| t.depends_on.iter().map(String::as_str))
        .collect();

    let orphans: Vec<&str> = tasks
        .iter()
        .filter(|t| t.depends_on.is_empty() && !has_dependents.contains(t.slug.as_str()))
        .map(|t| t.slug.as_str())
        .collect();

    // Exclude the case of a single-task plan — that's not an orphan problem.
    // Also exclude Audit tasks: they naturally stand alone (findings, not code).
    let audit_slugs: HashSet<&str> = tasks
        .iter()
        .filter(|t| t.kind == TaskKind::Audit)
        .map(|t| t.slug.as_str())
        .collect();
    let meaningful_orphans: Vec<&&str> = orphans
        .iter()
        .filter(|s| all_slugs.len() > 1 && !audit_slugs.contains(*s))
        .collect();

    if meaningful_orphans.len() > 1 {
        penalty =
            penalty.saturating_add(u8::try_from(meaningful_orphans.len().min(4)).unwrap_or(4));
        findings.push(format!(
            "{} tasks are islands (no deps, no dependents): {}",
            meaningful_orphans.len(),
            meaningful_orphans
                .iter()
                .map(|s| **s)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Fan-out: single tasks with 4+ direct dependents creates a bottleneck
    let mut fan_out: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for t in tasks {
        for dep in &t.depends_on {
            *fan_out.entry(dep.as_str()).or_insert(0) += 1;
        }
    }
    for (slug, count) in &fan_out {
        if *count >= 4 {
            penalty = penalty.saturating_add(1);
            findings.push(format!(
                "\"{slug}\" has {count} direct dependents — high fan-out; consider an intermediary task"
            ));
        }
    }

    // Depth: chains longer than 6 sequential tasks lose agility
    let max_depth = max_chain_depth(tasks);
    if max_depth > 6 {
        penalty = penalty.saturating_add(2);
        findings.push(format!(
            "Longest dependency chain is {max_depth} tasks deep — consider parallelising mid-chain work"
        ));
    }

    score_from_penalty("Dependency health", penalty, findings)
}

fn max_chain_depth(tasks: &[ResolvedTask]) -> usize {
    let slug_map: std::collections::HashMap<&str, &ResolvedTask> =
        tasks.iter().map(|t| (t.slug.as_str(), t)).collect();

    let mut cache: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

    tasks
        .iter()
        .map(|t| depth_of(t.slug.as_str(), &slug_map, &mut cache))
        .max()
        .unwrap_or(0)
}

fn depth_of<'a>(
    slug: &'a str,
    map: &std::collections::HashMap<&str, &'a ResolvedTask>,
    cache: &mut std::collections::HashMap<&'a str, usize>,
) -> usize {
    if let Some(&d) = cache.get(slug) {
        return d;
    }
    let depth = map.get(slug).map_or(0, |t| {
        t.depends_on
            .iter()
            .map(|dep| depth_of(dep.as_str(), map, cache) + 1)
            .max()
            .unwrap_or(0)
    });
    cache.insert(slug, depth);
    depth
}

// ─── Dimension: Coverage ─────────────────────────────────────────────────────

/// Checks whether the plan has adequate coverage of cross-cutting concerns:
/// security, integration verification, test harness setup, and documentation.
fn score_coverage(plan: &Plan, tasks: &[ResolvedTask]) -> DimensionScore {
    let mut findings = Vec::new();
    let mut penalty = 0u8;

    let all_goals_lower: String = tasks
        .iter()
        .map(|t| t.goal.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let all_titles_lower: String = tasks
        .iter()
        .map(|t| t.title.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");

    // Security coverage
    let has_security = tasks.iter().any(|t| {
        t.slug.contains("security")
            || t.slug.contains("hardening")
            || t.slug.contains("auth")
            || t.goal.to_lowercase().contains("security")
    }) || plan.security.skip_hardening_task;
    if !has_security {
        penalty = penalty.saturating_add(2);
        findings
            .push("No security-focused tasks — add a security audit or hardening task".to_string());
    }

    // Test harness / CI coverage
    let has_ci = all_goals_lower.contains("ci")
        || all_goals_lower.contains("pipeline")
        || all_titles_lower.contains("ci")
        || all_titles_lower.contains("scaffold")
        || tasks
            .iter()
            .any(|t| t.slug.contains("scaffold") || t.slug.contains("ci"));
    if !has_ci && tasks.len() > 4 {
        penalty = penalty.saturating_add(1);
        findings.push(
            "No CI/scaffold task detected — consider adding a Phase 1 task for workspace and CI setup"
                .to_string(),
        );
    }

    // Integration / wiring coverage
    let has_integration = tasks.iter().any(|t| {
        t.slug.contains("integration") || t.slug.contains("wiring") || t.slug.contains("wire")
    }) || plan.integration.skip_wiring_audit;
    if !has_integration && tasks.len() > 5 {
        penalty = penalty.saturating_add(1);
        findings.push(
            "No integration or wiring task — large plans should verify all components connect"
                .to_string(),
        );
    }

    // Error handling coverage
    let has_error_handling = all_goals_lower.contains("error")
        || all_goals_lower.contains("failure")
        || all_goals_lower.contains("recovery");
    if !has_error_handling && tasks.len() > 6 {
        penalty = penalty.saturating_add(1);
        findings.push("No tasks explicitly cover error handling or failure paths".to_string());
    }

    score_from_penalty("Coverage", penalty, findings)
}

// ─── Dimension: Richness ─────────────────────────────────────────────────────

/// Scores how well tasks are enriched with hints, test hints, and evaluation criteria.
fn score_richness(tasks: &[ResolvedTask]) -> DimensionScore {
    let mut findings = Vec::new();
    let mut penalty = 0u8;

    let total = tasks.len();
    if total == 0 {
        return score_from_penalty("Richness", 10, vec!["No tasks to score".to_string()]);
    }

    let no_hints = tasks.iter().filter(|t| t.hints.is_empty()).count();
    let no_test_hints = tasks.iter().filter(|t| t.test_hints.is_empty()).count();
    let no_criteria = tasks
        .iter()
        .filter(|t| t.evaluation_criteria.is_empty())
        .count();

    if no_hints > total / 2 {
        let pct = (no_hints * 100) / total;
        penalty = penalty.saturating_add(2);
        findings.push(format!(
            "{no_hints}/{total} tasks ({pct}%) have no hints — subagents will improvise"
        ));
    }

    if no_test_hints > total / 2 {
        let pct = (no_test_hints * 100) / total;
        penalty = penalty.saturating_add(2);
        findings.push(format!(
            "{no_test_hints}/{total} tasks ({pct}%) have no test_hints — testing may be skipped"
        ));
    }

    if no_criteria > (total * 3) / 4 {
        let pct = (no_criteria * 100) / total;
        penalty = penalty.saturating_add(1);
        findings.push(format!(
            "{no_criteria}/{total} tasks ({pct}%) have no evaluation_criteria"
        ));
    }

    // Check for tasks that have goals but are likely placeholder descriptions
    let placeholder_words = ["tbd", "todo", "placeholder", "fill in", "to be determined"];
    let placeholders: Vec<&str> = tasks
        .iter()
        .filter(|t| {
            let g = t.goal.to_lowercase();
            placeholder_words.iter().any(|p| g.contains(p))
        })
        .map(|t| t.slug.as_str())
        .collect();
    if !placeholders.is_empty() {
        penalty = penalty.saturating_add(2);
        findings.push(format!(
            "{} task(s) have placeholder goals: {}",
            placeholders.len(),
            placeholders.join(", ")
        ));
        // All tasks are placeholders — plan is not ready to generate
        if placeholders.len() == total {
            penalty = penalty.saturating_add(3);
            findings.push(
                "All tasks have placeholder goals — plan is not ready to generate".to_string(),
            );
        }
    }

    score_from_penalty("Richness", penalty, findings)
}

// ─── Dimension: Token budget ──────────────────────────────────────────────────

const TOKEN_WARN_THRESHOLD: usize = 80_000;
const TOKEN_CRITICAL_THRESHOLD: usize = 150_000;

fn score_token_budget(tasks: &[ResolvedTask]) -> DimensionScore {
    let mut findings = Vec::new();
    let total = estimated_total_tokens(tasks);

    let score = if total > TOKEN_CRITICAL_THRESHOLD {
        findings.push(format!(
            "Estimated ~{total} tokens — may exceed context windows for some models (critical: >{TOKEN_CRITICAL_THRESHOLD})"
        ));
        3
    } else if total > TOKEN_WARN_THRESHOLD {
        findings.push(format!(
            "Estimated ~{total} tokens — approaching context limits (warn: >{TOKEN_WARN_THRESHOLD})"
        ));
        6
    } else {
        findings.push(format!("Estimated ~{total} tokens — within healthy range"));
        10
    };

    // Per-task cost outliers: tasks whose goal is >1000 chars
    let heavy: Vec<String> = tasks
        .iter()
        .filter(|t| t.goal.len() + t.hints.join("").len() > 1000)
        .map(|t| {
            format!(
                "{} (~{} chars)",
                t.slug,
                t.goal.len() + t.hints.join("").len()
            )
        })
        .collect();
    if !heavy.is_empty() {
        findings.push(format!(
            "Heavy tasks (goal + hints >1000 chars): {}",
            heavy.join("; ")
        ));
    }

    DimensionScore {
        name: "Token budget",
        score,
        verdict: verdict_for(score),
        findings,
    }
}

fn estimated_total_tokens(tasks: &[ResolvedTask]) -> usize {
    tasks
        .iter()
        .map(|t| {
            estimate_tokens(&t.goal)
                + t.hints.iter().map(|h| estimate_tokens(h)).sum::<usize>()
                + t.test_hints
                    .iter()
                    .map(|h| estimate_tokens(h))
                    .sum::<usize>()
        })
        .sum()
}

// ─── Dimension: Harness complexity ───────────────────────────────────────────

/// Scores whether the evaluation harness is proportionate to plan complexity.
///
/// - Penalises plans with an evaluator but ≤2 tasks (over-engineered harness).
/// - Penalises plans with >3 phases but no evaluator (under-invested harness).
/// - Rewards plans where ≥50% of tasks have `evaluation_criteria` and a non-default
///   evaluator strategy is configured.
fn score_harness_complexity(plan: &Plan, tasks: &[ResolvedTask]) -> DimensionScore {
    let mut findings = Vec::new();
    let mut penalty = 0i8;

    let task_count = tasks.len();
    let phase_count = plan.phases.len();
    let has_evaluator = plan.evaluator.is_some();

    // Penalise: evaluator configured but plan is tiny (≤2 tasks)
    if has_evaluator && task_count <= 2 {
        penalty += 2;
        findings.push(format!(
            "Evaluator configured but plan has only {task_count} task(s) — harness is over-engineered for a micro-plan"
        ));
    }

    // Penalise: large plan (>3 phases) with no evaluator
    if !has_evaluator && phase_count > 3 {
        penalty += 1;
        findings.push(format!(
            "Plan has {phase_count} phases but no [evaluator] section — multi-phase plans benefit from automated QA"
        ));
    }

    // Reward: ≥50% tasks have evaluation_criteria AND non-default evaluator mode
    if task_count > 0 {
        let tasks_with_criteria = tasks
            .iter()
            .filter(|t| !t.evaluation_criteria.is_empty())
            .count();
        let pct = (tasks_with_criteria * 100)
            .checked_div(task_count)
            .unwrap_or(0);
        if pct >= 50 {
            // Negative penalty = bonus; floor at 0 via saturating conversion
            penalty -= 1;
            findings.push(format!(
                "{tasks_with_criteria}/{task_count} tasks ({pct}%) have evaluation_criteria — good harness investment"
            ));
        } else if pct < 25 && has_evaluator {
            penalty += 1;
            findings.push(format!(
                "Evaluator is configured but only {tasks_with_criteria}/{task_count} tasks ({pct}%) have evaluation_criteria"
            ));
        }
    }

    // Convert signed penalty to score (10 base, clamped to [0, 10])
    let clamped_penalty = penalty.clamp(0_i8, 10_i8).cast_unsigned();
    score_from_penalty("Harness complexity", clamped_penalty, findings)
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn phase_task_counts(tasks: &[ResolvedTask]) -> Vec<(String, usize)> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for t in tasks {
        *counts.entry(t.phase_name.clone()).or_insert(0) += 1;
    }
    counts.into_iter().collect()
}

const fn score_from_penalty(
    name: &'static str,
    penalty: u8,
    findings: Vec<String>,
) -> DimensionScore {
    let score = 10u8.saturating_sub(penalty);
    DimensionScore {
        name,
        score,
        verdict: verdict_for(score),
        findings,
    }
}

const fn verdict_for(score: u8) -> &'static str {
    if score >= 9 {
        "excellent"
    } else if score >= 7 {
        "good"
    } else if score >= 5 {
        "fair"
    } else if score >= 3 {
        "poor"
    } else {
        "critical"
    }
}

/// Public wrapper around the internal `verdict_for` helper.
#[must_use]
pub const fn verdict_for_score(score: u8) -> &'static str {
    verdict_for(score)
}

const fn pct_label(pct: usize) -> &'static str {
    if pct >= 75 {
        "majority"
    } else if pct >= 50 {
        "half"
    } else {
        "minority"
    }
}

fn collect_suggestions(dim: &DimensionScore, out: &mut Vec<Suggestion>) {
    let severity = match dim.score {
        0..=4 => SuggestionSeverity::Critical,
        5..=6 => SuggestionSeverity::Warning,
        _ => SuggestionSeverity::Info,
    };
    for finding in &dim.findings {
        out.push(Suggestion {
            severity,
            message: format!("[{}] {}", dim.name, finding),
        });
    }
}

// ─── Formatting helpers ───────────────────────────────────────────────────────

/// Format a [`PlanScore`] as a human-readable terminal report.
#[must_use]
pub fn format_score_report(score: &PlanScore) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(1024);

    let health_icon = if score.is_healthy() { "✓" } else { "✗" };
    let _ = writeln!(
        out,
        "Plan Quality Report  {health_icon} Overall: {}/10 ({})\n",
        score.overall,
        verdict_for(score.overall)
    );
    let _ = writeln!(out, "Dimensions:");
    for dim in &score.dimensions {
        let _ = writeln!(
            out,
            "  {:<22} {:>2}/10  {}",
            dim.name, dim.score, dim.verdict
        );
        for f in &dim.findings {
            let _ = writeln!(out, "    · {f}");
        }
    }

    if !score.suggestions.is_empty() {
        let _ = writeln!(out, "\nSuggestions ({}):", score.suggestions.len());
        for s in &score.suggestions {
            let _ = writeln!(out, "  [{}] {}", s.severity, s.message);
        }
    }

    let _ = writeln!(
        out,
        "\nEstimated tokens: ~{}",
        format_thousands(score.estimated_tokens)
    );
    out
}

/// Serialize `s` as a JSON string literal using RFC 8259-compliant escaping.
///
/// Delegates to `serde_json` to avoid the `\u{XX}` Rust-Debug pitfall.
fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| r#""""#.to_string())
}

/// Format a [`PlanScore`] as a JSON string.
#[must_use]
pub fn format_score_json(score: &PlanScore) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(512);
    let _ = write!(
        out,
        r#"{{"overall":{overall},"healthy":{healthy},"estimated_tokens":{tokens},"dimensions":["#,
        overall = score.overall,
        healthy = score.is_healthy(),
        tokens = score.estimated_tokens,
    );
    for (i, dim) in score.dimensions.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let findings_json: String = dim
            .findings
            .iter()
            .map(|f| json_str(f))
            .collect::<Vec<_>>()
            .join(",");
        let _ = write!(
            out,
            r#"{{"name":{name},"score":{score},"verdict":{verdict},"findings":[{findings}]}}"#,
            name = json_str(dim.name),
            score = dim.score,
            verdict = json_str(dim.verdict),
            findings = findings_json,
        );
    }
    out.push_str(r#"],"suggestions":["#);
    for (i, s) in score.suggestions.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"severity":{sev},"message":{msg}}}"#,
            sev = json_str(&s.severity.to_string()),
            msg = json_str(&s.message),
        );
    }
    out.push_str("]}");
    out
}

fn format_thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::plan::{Language, Plan, Preflight, Project, TaskKind};

    fn minimal_plan(task_goals: &[(&str, &str)]) -> (Plan, Vec<ResolvedTask>) {
        let tasks: Vec<ResolvedTask> = task_goals
            .iter()
            .enumerate()
            .map(|(i, (slug, goal))| ResolvedTask {
                number: u32::try_from(i + 1).unwrap_or(1),
                slug: (*slug).to_string(),
                title: slug.to_string(),
                goal: (*goal).to_string(),
                depends_on: Vec::new(),
                hints: Vec::new(),
                test_hints: Vec::new(),
                must_haves: Vec::new(),
                gate: None,
                evaluation_criteria: Vec::new(),
                phase_name: "Phase 1".to_string(),
                phase_order: 1,
                kind: TaskKind::Feature,
            })
            .collect();

        let plan = Plan {
            project: Project {
                name: "test".to_string(),
                description: "test".to_string(),
                language: Language::Rust,
                path: "/tmp/test".to_string(),
                architecture: None,
            },
            preflight: Preflight::default(),
            orchestrator: crate::domain::plan::Orchestrator::default(),
            evaluator: None,
            security: crate::domain::plan::SecurityConfig::default(),
            integration: crate::domain::plan::IntegrationConfig::default(),
            style: crate::domain::plan::StyleConfig::default(),
            phases: Vec::new(),
        };

        (plan, tasks)
    }

    #[test]
    fn score_healthy_plan() {
        let goals: Vec<(&str, &str)> = vec![
            (
                "scaffold",
                "Set up the Cargo workspace with CI pipeline and workspace-level Cargo.toml",
            ),
            (
                "domain-model",
                "Implement entities, value objects, and port traits for the domain layer with no I/O dependencies",
            ),
            (
                "infra-db",
                "Implement the PostgreSQL repository adapter using sqlx with connection pooling and error mapping",
            ),
            (
                "auth",
                "Implement JWT authentication middleware with token validation and refresh",
            ),
            (
                "security-hardening",
                "Audit for OWASP top 10 vulnerabilities including injection, XSS, CSRF",
            ),
        ];
        let (plan, tasks) = minimal_plan(&goals);
        let score = score_plan(&plan, &tasks);
        assert!(
            score.overall >= 5,
            "healthy plan should score at least 5, got {}",
            score.overall
        );
    }

    #[test]
    fn score_poor_plan_placeholder_goals() {
        let goals = vec![
            ("task-a", "TBD"),
            ("task-b", "TODO: fill in"),
            ("task-c", "placeholder"),
        ];
        let (plan, tasks) = minimal_plan(&goals);
        let score = score_plan(&plan, &tasks);
        // Richness and granularity should both be penalised
        assert!(
            score.overall < 7,
            "plan with placeholder goals should score below 7, got {}",
            score.overall
        );
        assert!(!score.suggestions.is_empty(), "should generate suggestions");
    }

    #[test]
    fn score_oversized_goals() {
        let long_goal = "a".repeat(600);
        let goals = vec![
            ("big-task", long_goal.as_str()),
            (
                "normal",
                "Implement a basic CRUD API endpoint with validation",
            ),
        ];
        let (plan, tasks) = minimal_plan(&goals);
        let score = score_plan(&plan, &tasks);
        let granularity = score
            .dimensions
            .iter()
            .find(|d| d.name == "Granularity")
            .unwrap();
        assert!(
            granularity.score < 10,
            "oversized goal should reduce granularity score"
        );
    }

    #[test]
    fn overall_score_in_range() {
        let goals = vec![
            (
                "task-a",
                "Implement the core business logic with validation",
            ),
            ("task-b", "Write integration tests for the API endpoints"),
        ];
        let (plan, tasks) = minimal_plan(&goals);
        let score = score_plan(&plan, &tasks);
        assert!(score.overall <= 10, "overall score should not exceed 10");
    }
}
