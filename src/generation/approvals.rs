//! Render the **Approvals** rules shared by every orchestrator and subagent prompt.
//!
//! The rules live here once so the variants cannot drift: an approval given for
//! one scope (a task, staging) must never be read as permission for another
//! (a different task, production).

const RULES: [&str; 5] = [
    "An approval applies only to its recorded scope (the task and the environment it names).",
    "An approval for a test or staging action never authorizes a production action.",
    "Never reuse an approval for a different task or a different environment.",
    "If the scope of an approval is unclear or missing, stop and ask the human.",
    "An approval expires at the end of the task it was given for, unless its row says otherwise.",
];

/// Markdown body (heading included) for the Approvals section.
#[must_use]
pub fn render_approvals_section() -> String {
    let mut out = String::from("## Approvals\n\n");
    for rule in RULES {
        out.push_str("- ");
        out.push_str(rule);
        out.push('\n');
    }
    out.push_str(
        "- Record every human approval as a row in the **Approvals** table in `PROGRESS.md` \
         (scope, action, approved by, expires) before acting on it.\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_has_heading_and_one_bullet_per_rule_plus_record_line() {
        let s = render_approvals_section();
        assert!(s.starts_with("## Approvals\n\n- "));
        assert_eq!(
            s.lines().filter(|l| l.starts_with("- ")).count(),
            RULES.len() + 1
        );
        assert!(s.ends_with('\n'));
    }
}
