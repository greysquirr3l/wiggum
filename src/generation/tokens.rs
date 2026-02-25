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

/// Format token estimates as a human-readable report string.
#[must_use]
pub fn format_report(artifacts: &super::GeneratedArtifacts) -> String {
    let estimates = estimate_all(artifacts);
    let mut lines = vec!["Token estimates (approx, chars/4):\n".to_string()];

    // Shared artifacts
    for est in estimates.iter().take(3) {
        lines.push(format!("  {:<30} ~{} tokens", est.name, est.tokens));
    }

    // Base overhead = progress + plan_doc + orchestrator
    let base_overhead: usize = estimates.iter().take(3).map(|e| e.tokens).sum();

    // Per-task context
    if estimates.len() > 3 {
        lines.push(String::new());
        lines.push("  Per-task context (plan + progress + task file):".to_string());

        let mut largest: usize = 0;
        for est in estimates.iter().skip(3) {
            let task_context = base_overhead + est.tokens;
            if task_context > largest {
                largest = task_context;
            }
            let flag = if task_context > 4000 {
                "  ⚠️"
            } else {
                "  ✅"
            };
            lines.push(format!(
                "    {:<28} ~{} tokens{flag}",
                est.name, task_context
            ));
        }

        lines.push(String::new());
        lines.push(format!(
            "  Base overhead per iteration: ~{base_overhead} tokens"
        ));
        lines.push(format!("  Largest single task context: ~{largest} tokens"));
    }

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
        };
        let report = format_report(&artifacts);
        assert!(report.contains("Token estimates"));
        assert!(report.contains("PROGRESS.md"));
        assert!(report.contains("Base overhead"));
        assert!(report.contains("T01-foo.md"));
    }
}
