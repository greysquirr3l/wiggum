//! Generate `.claude/settings.json` — Claude hooks configuration.
//!
//! Registers a `PreCompact` hook that blocks context compression while any
//! task is in the `[~]` (in-progress) state.  This prevents Claude from
//! discarding mid-task context during a long implementation session.

/// The static JSON content for `.claude/settings.json`.
pub const SETTINGS_JSON: &str = r#"{
  "hooks": {
    "PreCompact": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "grep -q '\\[~\\]' PROGRESS.md 2>/dev/null && echo 'BLOCKED: task in progress — mark [~] task as [x] or [!] before compacting' && exit 1 || exit 0"
          }
        ]
      }
    ]
  }
}
"#;

/// Return the `.claude/settings.json` content.
///
/// This is a pure function returning a static string — no generation needed.
#[must_use]
pub const fn render() -> &'static str {
    SETTINGS_JSON
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_is_valid_json() {
        let result: Result<serde_json::Value, _> = serde_json::from_str(render());
        assert!(result.is_ok(), "hooks JSON must be valid: {result:?}");
    }

    #[test]
    fn contains_precompact_hook() {
        let output = render();
        assert!(
            output.contains("PreCompact"),
            "must register a PreCompact hook"
        );
    }

    #[test]
    fn contains_progress_check() {
        let output = render();
        assert!(
            output.contains("PROGRESS.md"),
            "hook must check PROGRESS.md"
        );
    }
}
