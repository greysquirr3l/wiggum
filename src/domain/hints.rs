//! First-command hint formatter — emits the single `Next: ...` line that
//! wiggum generate prints after the scorecard and warnings.
//!
//! Pure function over `Plan` + `PlanScore`: no filesystem, no IO, easy to
//! unit-test.

use crate::domain::check::PlanScore;
use crate::domain::plan::Plan;

/// Maximum length of the hint line, in bytes.
const MAX_HINT_LEN: usize = 80;

/// Build the one-line `Next: ...` hint that follows the scorecard at
/// scaffold time.
///
/// Decision tree (first match wins):
/// - Plan has any task with a `gate` declaration → review gate tasks first.
/// - Plan auto-derives `require_evaluator=true` but no `[evaluator]` block
///   is configured → add `[evaluator]` or opt out.
/// - Plan's quality score is unhealthy (overall < 7) → fix flagged
///   dimensions first.
/// - Otherwise → open ORCHESTRATOR.md and run the first task.
///
/// The returned string is at most 80 bytes; longer hints are truncated on
/// a char boundary with an ellipsis suffix.
#[must_use]
pub fn first_command_hint(plan: &Plan, score: &PlanScore) -> String {
    let raw = if plan_has_gated_tasks(plan) {
        "Next: review ⚠-marked tasks in PROGRESS.md before starting".to_string()
    } else if plan_missing_evaluator(plan) {
        "Next: add [evaluator] or set require_evaluator = false".to_string()
    } else if !score.is_healthy() {
        "Next: address flagged dimensions in the scorecard above".to_string()
    } else {
        "Next: open ORCHESTRATOR.md and run the first task".to_string()
    };

    truncate_to(&raw, MAX_HINT_LEN)
}

fn plan_has_gated_tasks(plan: &Plan) -> bool {
    plan.phases
        .iter()
        .flat_map(|p| p.tasks.iter())
        .any(|t| t.gate.is_some())
}

fn plan_missing_evaluator(plan: &Plan) -> bool {
    plan.evaluator.is_none()
        && !plan.orchestrator.require_evaluator.unwrap_or(true)
        && has_security_sensitive_phase(plan)
}

/// Returns true if any task in any phase has a security-sensitive slug
/// (matches the auto-derive rule for `require_evaluator`).
fn has_security_sensitive_phase(plan: &Plan) -> bool {
    plan.phases
        .iter()
        .flat_map(|p| p.tasks.iter())
        .any(|t| crate::domain::plan::has_security_sensitive_slug(&t.slug))
}

/// Truncate `s` to at most `max_len` bytes on a char boundary, appending
/// "…" if any characters were dropped.
fn truncate_to(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    // Walk back from the byte boundary until the prefix fits.
    let mut end = max_len.saturating_sub(1); // reserve 1 byte for the ellipsis
    while end > 0 && !s.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    let mut out = String::with_capacity(end + 1);
    out.push_str(&s[..end]);
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plan::{Phase, Plan, Preflight, Project, TaskDef, TaskKind};

    fn empty_plan() -> Plan {
        Plan {
            project: Project {
                name: "test".into(),
                description: "test".into(),
                language: crate::domain::plan::Language::Rust,
                path: "/tmp/test".into(),
                architecture: None,
            },
            preflight: Preflight::default(),
            orchestrator: crate::domain::plan::Orchestrator::default(),
            evaluator: None,
            security: crate::domain::plan::SecurityConfig::default(),
            integration: crate::domain::plan::IntegrationConfig::default(),
            style: crate::domain::plan::StyleConfig::default(),
            targets: crate::domain::plan::TargetConfig::default(),
            phases: vec![Phase {
                name: "P1".into(),
                order: 1,
                tasks: vec![TaskDef {
                    slug: "alpha".into(),
                    title: "Alpha".into(),
                    goal: "Do alpha".into(),
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

    fn empty_score() -> PlanScore {
        PlanScore {
            dimensions: vec![],
            overall: 10,
            suggestions: vec![],
            estimated_tokens: 0,
        }
    }

    #[test]
    fn healthy_plan_suggests_orchestrator() {
        let plan = empty_plan();
        let score = empty_score();
        let hint = first_command_hint(&plan, &score);
        assert!(hint.starts_with("Next:"), "got: {hint}");
        assert!(hint.contains("ORCHESTRATOR.md"), "got: {hint}");
        assert!(hint.len() <= MAX_HINT_LEN, "len {}: {hint}", hint.len());
    }

    #[test]
    fn gated_plan_suggests_review() {
        let mut plan = empty_plan();
        if let Some(phase) = plan.phases.first_mut()
            && let Some(task) = phase.tasks.first_mut()
        {
            task.gate = Some("auth".into());
        }
        let score = empty_score();
        let hint = first_command_hint(&plan, &score);
        assert!(hint.starts_with("Next:"), "got: {hint}");
        assert!(
            hint.contains("⚠") || hint.contains("PROGRESS"),
            "got: {hint}"
        );
    }

    #[test]
    fn unhealthy_plan_suggests_scorecard_fix() {
        let plan = empty_plan();
        let mut score = empty_score();
        score.overall = 5;
        let hint = first_command_hint(&plan, &score);
        assert!(
            hint.contains("scorecard") || hint.contains("flagged"),
            "got: {hint}"
        );
    }

    #[test]
    fn hint_always_within_length_limit() {
        // Build a pathologically long keyword by stuffing it into a title
        // — the formatter doesn't read titles but the truncation logic
        // still has to keep the hint short.
        let mut plan = empty_plan();
        if let Some(phase) = plan.phases.first_mut()
            && let Some(task) = phase.tasks.first_mut()
        {
            task.title = "x".repeat(1000);
        }
        let score = empty_score();
        let hint = first_command_hint(&plan, &score);
        assert!(
            hint.len() <= MAX_HINT_LEN,
            "len {} > {MAX_HINT_LEN}",
            hint.len()
        );
    }
}
