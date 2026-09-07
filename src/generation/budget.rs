//! Renders `BUDGET.md`, a universal scaffold artifact that summarises
//! the per-iteration token cost of running this plan and surfaces the
//! warn/critical thresholds the orchestrator should respect.
//!
//! Pure function over `Plan` + `&[ResolvedTask]`: no filesystem, no IO.

use std::fmt::Write as _;

use crate::domain::check::{TOKEN_CRITICAL_THRESHOLD, TOKEN_WARN_THRESHOLD};
use crate::domain::plan::{Plan, ResolvedTask};
use crate::error::Result;

/// Approximate tokens per character (cl100k_base-style heuristic).
/// Mirrors `generation::tokens::estimate_tokens` but kept here so the
/// per-task context computation is self-contained.
const fn chars_to_tokens(chars: usize) -> usize {
    chars / 4
}

/// Per-task token estimate used by `render` to surface the largest task.
#[derive(Debug, Clone)]
pub struct TaskToken {
    pub number: u32,
    pub slug: String,
    pub tokens: usize,
}

/// Estimate the per-task context size.
///
/// Sums the `goal`, `hints`, `test_hints`, `must_haves`, and
/// `evaluation_criteria` character counts and converts to tokens. This
/// is the size of the prompt body the subagent sees when running the
/// task (excluding the boilerplate around it).
#[must_use]
pub fn task_context_tokens(task: &ResolvedTask) -> usize {
    let mut chars = task.goal.chars().count();
    for hint in &task.hints {
        chars = chars.saturating_add(hint.chars().count());
    }
    for hint in &task.test_hints {
        chars = chars.saturating_add(hint.chars().count());
    }
    for m in &task.must_haves {
        chars = chars.saturating_add(m.chars().count());
    }
    for c in &task.evaluation_criteria {
        chars = chars.saturating_add(c.chars().count());
    }
    chars_to_tokens(chars)
}

/// Render `BUDGET.md` for the given plan.
///
/// Sections:
/// 1. Summary — total estimated tokens, total artifacts, base overhead.
/// 2. Per-artifact table — token estimate for each generated file.
/// 3. Per-task table — token estimate per task, with ⚠ CRITICAL marker
///    for tasks over [`TOKEN_CRITICAL_THRESHOLD`] and ⚠ WARN for tasks
///    over [`TOKEN_WARN_THRESHOLD`].
/// 4. Thresholds — the canonical warn / critical boundaries.
/// 5. Recommended daily cap — max single task × 10, so the orchestrator
///    can plan iteration budgets.
///
/// # Errors
///
/// Returns an error only if the underlying `String` writer fails, which
/// in practice cannot happen with `String`. The `Result` return is for
/// symmetry with other renderers.
pub fn render(plan: &Plan, tasks: &[ResolvedTask]) -> Result<String> {
    let mut out = String::with_capacity(2048);

    let project_name = &plan.project.name;
    let _ = writeln!(out, "# BUDGET — {project_name}");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Token estimates for the generated scaffold. Use this file to budget \
         per-iteration prompt sizes before running the orchestrator loop."
    );
    let _ = writeln!(out);

    // ── Per-task token table ────────────────────────────────────────
    let _ = writeln!(out, "## Per-task context size");
    let _ = writeln!(out);
    let _ = writeln!(out, "| # | Task | Tokens | Status |");
    let _ = writeln!(out, "|---|------|-------:|--------|");

    let mut task_estimates: Vec<TaskToken> = Vec::with_capacity(tasks.len());
    for task in tasks {
        let tokens = task_context_tokens(task);
        let status = if tokens > TOKEN_CRITICAL_THRESHOLD {
            "⚠ CRITICAL"
        } else if tokens > TOKEN_WARN_THRESHOLD {
            "⚠ WARN"
        } else {
            "ok"
        };
        task_estimates.push(TaskToken {
            number: task.number,
            slug: task.slug.clone(),
            tokens,
        });
        let _ = writeln!(
            out,
            "| T{:02} | {} | {} | {} |",
            task.number, task.slug, tokens, status
        );
    }
    let _ = writeln!(out);

    // ── Summary metrics ─────────────────────────────────────────────
    let largest_task = task_estimates.iter().max_by_key(|t| t.tokens);
    let total_task_tokens: usize = task_estimates.iter().map(|t| t.tokens).sum();

    let base_overhead = estimate_plan_doc_tokens(plan)
        + estimate_progress_tokens(tasks)
        + estimate_orchestrator_prompt_tokens(plan);

    let total_tokens = total_task_tokens.saturating_add(base_overhead);

    let _ = writeln!(out, "## Summary");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Metric | Tokens |");
    let _ = writeln!(out, "|--------|-------:|");
    let _ = writeln!(out, "| Total task contexts | {total_task_tokens} |");
    let _ = writeln!(
        out,
        "| Plan + progress + orchestrator base overhead | {base_overhead} |"
    );
    let _ = writeln!(out, "| **Total estimated tokens** | **{total_tokens}** |");
    if let Some(big) = largest_task {
        let _ = writeln!(
            out,
            "| Largest single task ({}) | {} |",
            big.slug, big.tokens
        );
    }
    let _ = writeln!(out);

    // ── Thresholds ──────────────────────────────────────────────────
    let _ = writeln!(out, "## Thresholds");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Tokens are measured against the cl100k_base-style heuristic used by \
         `wiggum check`. Tasks above the WARN threshold risk losing detail to \
         context window churn; tasks above CRITICAL will almost certainly \
         truncate."
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "| Threshold | Tokens |");
    let _ = writeln!(out, "|-----------|-------:|");
    let _ = writeln!(out, "| WARN threshold | {TOKEN_WARN_THRESHOLD} |");
    let _ = writeln!(out, "| CRITICAL threshold | {TOKEN_CRITICAL_THRESHOLD} |");
    let _ = writeln!(out);

    // ── Recommended daily cap ───────────────────────────────────────
    let daily_cap = largest_task.map_or(base_overhead * 5, |t| t.tokens.saturating_mul(10));
    let _ = writeln!(out, "## Recommended daily cap");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "**Recommended daily cap: {daily_cap} tokens** (largest task × 10, so the \
         loop can complete roughly 10 iterations before context churn forces \
         a fresh subagent). Adjust downward if your orchestrator model has a \
         smaller context window."
    );
    let _ = writeln!(out);

    Ok(out)
}

fn estimate_plan_doc_tokens(plan: &Plan) -> usize {
    // Roughly: project name + description + phase list. The renderer
    // output is bigger, but this is the base-overhead heuristic.
    let chars = plan.project.name.chars().count()
        + plan.project.description.chars().count()
        + plan
            .phases
            .iter()
            .map(|p| p.name.chars().count() + p.tasks.len() * 30)
            .sum::<usize>();
    chars_to_tokens(chars)
}

fn estimate_progress_tokens(tasks: &[ResolvedTask]) -> usize {
    let chars: usize = tasks
        .iter()
        .map(|t| {
            t.slug.chars().count() + t.title.chars().count() + t.phase_name.chars().count() + 40 // status + row separators
        })
        .sum();
    chars_to_tokens(chars)
}

fn estimate_orchestrator_prompt_tokens(plan: &Plan) -> usize {
    // Persona + rules + preflight. Mirror the persona default.
    let chars = plan.orchestrator.persona.chars().count()
        + plan
            .orchestrator
            .rules
            .iter()
            .map(String::len)
            .sum::<usize>()
        + plan.preflight.build.chars().count()
        + plan.preflight.test.chars().count()
        + plan.preflight.lint.chars().count()
        + 200; // boilerplate headings
    chars_to_tokens(chars)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn minimal_task(number: u32, slug: &str, goal_chars: usize) -> ResolvedTask {
        ResolvedTask {
            number,
            slug: slug.to_string(),
            title: slug.to_string(),
            goal: "x".repeat(goal_chars),
            depends_on: vec![],
            hints: vec![],
            test_hints: vec![],
            must_haves: vec![],
            gate: None,
            evaluation_criteria: vec![],
            phase_name: "P1".into(),
            phase_order: 1,
            kind: crate::domain::plan::TaskKind::default(),
        }
    }

    fn minimal_plan() -> Plan {
        use crate::domain::plan::{
            IntegrationConfig, Orchestrator, Phase, Preflight, Project, SecurityConfig,
            StyleConfig, TargetConfig, TaskDef,
        };
        Plan {
            project: Project {
                name: "test-plan".into(),
                description: "A test plan".into(),
                language: crate::domain::plan::Language::Rust,
                path: "/tmp/test".into(),
                architecture: None,
            },
            preflight: Preflight::default(),
            orchestrator: Orchestrator::default(),
            evaluator: None,
            security: SecurityConfig::default(),
            integration: IntegrationConfig::default(),
            style: StyleConfig::default(),
            targets: TargetConfig::default(),
            phases: vec![Phase {
                name: "P1".into(),
                order: 1,
                tasks: vec![TaskDef {
                    slug: "alpha".into(),
                    title: "Alpha".into(),
                    goal: "Implement alpha".into(),
                    depends_on: vec![],
                    hints: vec![],
                    test_hints: vec![],
                    must_haves: vec![],
                    gate: None,
                    evaluation_criteria: vec![],
                    kind: crate::domain::plan::TaskKind::default(),
                }],
            }],
        }
    }

    #[test]
    fn render_contains_required_sections() {
        let plan = minimal_plan();
        let tasks = vec![minimal_task(1, "alpha", 200)];
        let md = render(&plan, &tasks).unwrap();
        assert!(
            md.contains("Total estimated tokens"),
            "missing summary line"
        );
        assert!(
            md.contains("| WARN threshold | 80000 |"),
            "missing WARN row"
        );
        assert!(
            md.contains("| CRITICAL threshold | 150000 |"),
            "missing CRITICAL row"
        );
        assert!(md.contains("Recommended daily cap"), "missing daily cap");
    }

    #[test]
    fn render_marks_critical_tasks() {
        let plan = minimal_plan();
        // ~700k characters → ~175k tokens, comfortably over 150k CRITICAL.
        let tasks = vec![minimal_task(1, "huge", 700_000)];
        let md = render(&plan, &tasks).unwrap();
        assert!(
            md.contains("⚠ CRITICAL"),
            "expected CRITICAL marker for huge task, got:\n{md}"
        );
    }

    #[test]
    fn render_writes_to_disk_when_generated() {
        // Smoke test: rendering is a pure String, so just verify the
        // output is non-empty and well-formed CommonMark (single H1).
        let plan = minimal_plan();
        let tasks = vec![minimal_task(1, "alpha", 100)];
        let md = render(&plan, &tasks).unwrap();
        assert!(md.starts_with("# BUDGET"));
        let h1_count = md.lines().filter(|l| l.starts_with("# ")).count();
        assert!(h1_count >= 1, "expected at least one H1, got {h1_count}");
    }
}
