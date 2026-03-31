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
/// per-task context estimates.
#[must_use]
pub fn estimate_all(artifacts: &super::GeneratedArtifacts) -> Vec<TokenEstimate> {
    let mut estimates = vec![
        TokenEstimate {
            name: "PROGRESS.md".to_string(),
            tokens: estimate_tokens(&artifacts.progress),
        },
        TokenEstimate {
            name: "IMPLEMENTATION_PLAN.md".to_string(),
            tokens: estimate_tokens(&artifacts.plan_doc),
        },
        TokenEstimate {
            name: "orchestrator.prompt.md".to_string(),
            tokens: estimate_tokens(&artifacts.orchestrator),
        },
    ];

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
pub fn format_report(artifacts: &super::GeneratedArtifacts) -> String {
    use crate::domain::pricing::PricingData;

    let estimates = estimate_all(artifacts);
    let mut lines = vec!["Token estimates (approx, chars/4):\n".to_string()];

    // Per-task token counts
    for est in estimates.iter().skip(3) {
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
    fn estimate_all_counts_artifacts() {
        let artifacts = super::super::GeneratedArtifacts {
            progress: "x".repeat(400),
            orchestrator: "y".repeat(800),
            plan_doc: "z".repeat(200),
            tasks: vec![
                ("T01-foo.md".to_string(), "a".repeat(1200)),
                ("T02-bar.md".to_string(), "b".repeat(2000)),
            ],
            agents_md: None,
            features_json: String::new(),
            evaluator_prompt: None,
        };
        let estimates = estimate_all(&artifacts);
        assert_eq!(estimates.len(), 5);
        assert_eq!(estimates[0].tokens, 100); // progress 400/4
        assert_eq!(estimates[1].tokens, 50); // plan_doc 200/4
        assert_eq!(estimates[2].tokens, 200); // orchestrator 800/4
        assert_eq!(estimates[3].tokens, 300); // T01 1200/4
        assert_eq!(estimates[4].tokens, 500); // T02 2000/4
    }

    #[test]
    fn format_report_output() {
        let artifacts = super::super::GeneratedArtifacts {
            progress: "x".repeat(400),
            orchestrator: "y".repeat(800),
            plan_doc: "z".repeat(200),
            tasks: vec![("T01-foo.md".to_string(), "a".repeat(1200))],
            agents_md: None,
            features_json: String::new(),
            evaluator_prompt: None,
        };
        let report = format_report(&artifacts);
        assert!(report.contains("Token estimates"));
        assert!(report.contains("T01-foo.md"));
        assert!(report.contains("Total:"));
        assert!(report.contains("Estimated cost"));
        assert!(report.contains("claude-sonnet-4"));
    }
}
