/// Estimate tokens for a text string using chars/4 approximation.
///
/// This is a rough estimate suitable for English text with code blocks,
/// approximating `cl100k_base` tokenization without requiring a tokenizer library.
#[must_use]
pub const fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Token estimate for a named artifact.
pub struct TokenEstimate {
    pub name: String,
    pub tokens: usize,
}

/// Estimate tokens for all generated artifacts, returning per-artifact and
/// per-task context estimates. Only the active targets in `targets` are
/// included.
#[must_use]
pub fn estimate_all(
    artifacts: &super::GeneratedArtifacts,
    targets: &crate::domain::targets::TargetSet,
) -> Vec<TokenEstimate> {
    use crate::domain::targets::Target;

    let mut estimates = vec![
        TokenEstimate {
            name: "PROGRESS.md".to_string(),
            tokens: estimate_tokens(&artifacts.progress),
        },
        TokenEstimate {
            name: "IMPLEMENTATION_PLAN.md".to_string(),
            tokens: estimate_tokens(&artifacts.plan_doc),
        },
    ];

    if targets.contains(Target::Vscode) {
        estimates.push(TokenEstimate {
            name: ".vscode/orchestrator.prompt.md".to_string(),
            tokens: estimate_tokens(&artifacts.orchestrator_vscode),
        });
        if let Some(eval) = &artifacts.evaluator_vscode {
            estimates.push(TokenEstimate {
                name: ".vscode/evaluator.prompt.md".to_string(),
                tokens: estimate_tokens(eval),
            });
        }
        estimates.push(TokenEstimate {
            name: ".vscode/planner.prompt.md".to_string(),
            tokens: estimate_tokens(&artifacts.planner_vscode),
        });
        estimates.push(TokenEstimate {
            name: ".vscode/background-auditor.prompt.md".to_string(),
            tokens: estimate_tokens(&artifacts.background_auditor_vscode),
        });
    }

    if targets.contains(Target::Opencode) {
        estimates.push(TokenEstimate {
            name: ".opencode/agents/wiggum-orchestrator.md".to_string(),
            tokens: estimate_tokens(&artifacts.orchestrator_opencode),
        });
        estimates.push(TokenEstimate {
            name: ".opencode/agents/wiggum-implementer.md".to_string(),
            tokens: estimate_tokens(&artifacts.implementer),
        });
        if let Some(eval) = &artifacts.evaluator_opencode {
            estimates.push(TokenEstimate {
                name: ".opencode/agents/wiggum-evaluator.md".to_string(),
                tokens: estimate_tokens(eval),
            });
        }
        estimates.push(TokenEstimate {
            name: ".opencode/agents/wiggum-planner.md".to_string(),
            tokens: estimate_tokens(&artifacts.planner_opencode),
        });
        estimates.push(TokenEstimate {
            name: ".opencode/agents/wiggum-auditor.md".to_string(),
            tokens: estimate_tokens(&artifacts.background_auditor_opencode),
        });
    }

    if targets.contains(Target::Claude) {
        estimates.push(TokenEstimate {
            name: ".claude/settings.json".to_string(),
            tokens: estimate_tokens(&artifacts.hooks_json),
        });
    }

    for (filename, content) in &artifacts.tasks {
        estimates.push(TokenEstimate {
            name: filename.clone(),
            tokens: estimate_tokens(content),
        });
    }

    estimates
}

/// Format token estimates as a human-readable report string with cost estimates.
#[must_use]
pub fn format_report(
    artifacts: &super::GeneratedArtifacts,
    targets: &crate::domain::targets::TargetSet,
) -> String {
    use crate::domain::pricing::PricingData;

    let estimates = estimate_all(artifacts, targets);
    let mut lines = vec!["Token estimates (approx, chars/4):\n".to_string()];

    // Per-task token counts — skip the per-target headers (everything before
    // the first Txx file) and emit the per-task rows. Task files are always
    // lowercase `.md` per the writer, so case-insensitive comparison is
    // purely defensive.
    let is_task_file = |name: &str| -> bool {
        name.starts_with('T')
            && name.len() > 3
            && name[name.len() - 3..].eq_ignore_ascii_case(".md")
    };
    let first_task_idx = estimates.iter().position(|e| is_task_file(&e.name));
    let task_start = first_task_idx.unwrap_or(estimates.len());
    for est in estimates.iter().skip(task_start) {
        lines.push(format!("  {:<30} ~{} tokens", est.name, est.tokens));
    }

    // Total tokens
    let total_tokens: usize = estimates.iter().map(|e| e.tokens).sum();
    lines.push(String::new());
    lines.push(format!("  Total: ~{total_tokens} tokens"));

    // Cost estimates
    let pricing = PricingData::bundled();
    let costs = pricing.estimate_cost(total_tokens);

    lines.push(String::new());
    lines.push("Estimated cost (prompt + completion, rough):".to_string());
    for cost in &costs {
        lines.push(format!("  {:<24} ~${:.2}", cost.model, cost.cost_usd));
    }

    lines.push(String::new());
    lines.push("  Prices are estimates. Actual cost depends on completion length.".to_string());
    lines.push(format!(
        "  Last updated: {}. Run `wiggum prices` to see rates.",
        pricing.last_updated
    ));

    lines.join("\n")
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::domain::targets::TargetSet;

    fn sample_artifacts() -> super::super::GeneratedArtifacts {
        super::super::GeneratedArtifacts {
            progress: "x".repeat(400),
            plan_doc: "z".repeat(200),
            tasks: vec![
                ("T01-foo.md".to_string(), "a".repeat(1200)),
                ("T02-bar.md".to_string(), "b".repeat(2000)),
            ],
            agents_md: None,
            features_json: String::new(),
            orchestrator_vscode: "y".repeat(800),
            evaluator_vscode: None,
            planner_vscode: String::new(),
            background_auditor_vscode: String::new(),
            orchestrator_opencode: String::new(),
            implementer: String::new(),
            evaluator_opencode: None,
            planner_opencode: String::new(),
            background_auditor_opencode: String::new(),
            hooks_json: String::new(),
            claude_md: String::new(),
            agent_rules_cursorrules: String::new(),
            agent_rules_windsurfrules: String::new(),
            agent_rules_copilot_instructions: String::new(),
        }
    }

    #[test]
    fn estimate_tokens_basic() {
        // 100 bytes / 4 = 25
        let text = "a".repeat(100);
        assert_eq!(estimate_tokens(&text), 25);
    }

    #[test]
    fn estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn estimate_all_vscode_only() {
        let artifacts = sample_artifacts();
        let estimates = estimate_all(&artifacts, &TargetSet::vscode_only());
        // 2 universal (progress, plan_doc) + 3 vscode (orchestrator, planner, auditor — no evaluator) + 2 tasks = 7
        assert_eq!(estimates.len(), 7);
        assert_eq!(estimates[0].tokens, 100); // progress 400/4
        assert_eq!(estimates[1].tokens, 50); // plan_doc 200/4
        assert_eq!(estimates[2].tokens, 200); // orchestrator 800/4
    }

    #[test]
    fn format_report_output() {
        let artifacts = sample_artifacts();
        let report = format_report(&artifacts, &TargetSet::vscode_only());
        assert!(report.contains("Token estimates"));
        assert!(report.contains("T01-foo.md"));
        assert!(report.contains("Total:"));
        assert!(report.contains("Estimated cost"));
        assert!(report.contains("claude-sonnet-4"));
    }
}
