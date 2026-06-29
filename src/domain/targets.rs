//! Target selection for generated artifacts.
//!
//! Wiggum supports emitting scaffold files for multiple AI coding tools
//! simultaneously. The `Target` enum identifies a single supported tool,
//! and `TargetSet` is the set of tools a given generate run should write
//! artifacts for.
//!
//! ## Selection precedence
//!
//! 1. The `--target` CLI flag (if provided) always wins.
//! 2. Otherwise, the `[targets]` section of the plan TOML.
//! 3. Otherwise, the default (`vscode`) — preserves back-compat.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// The set of AI coding tools wiggum can emit artifacts for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    /// VS Code + GitHub Copilot. Emits `.vscode/*.prompt.md` files using
    /// the `runSubagent` tool.
    Vscode,
    /// opencode. Emits `.opencode/agents/wiggum-*.md` agent files using
    /// the `task` tool for subagent dispatch.
    Opencode,
    /// Claude Code. Emits `.claude/settings.json` hooks plus `CLAUDE.md`
    /// project memory at the repository root.
    Claude,
    /// Fork-neutral rules files for `VSCode` forks that don't speak the
    /// Copilot `runSubagent` or opencode `task` protocols. Emits
    /// `.cursorrules` (Cursor), `.windsurfrules` (Windsurf), and
    /// `.github/copilot-instructions.md` (GitHub Copilot / forks that read
    /// it). The receiving IDE is responsible for its own agent loop;
    /// wiggum only provides the rules + project context.
    AgentRules,
}

impl Target {
    /// All targets, in stable display order.
    pub const ALL: &[Self] = &[Self::Vscode, Self::Opencode, Self::Claude, Self::AgentRules];

    /// Stable identifier used in CLI flags and TOML.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vscode => "vscode",
            Self::Opencode => "opencode",
            Self::Claude => "claude",
            Self::AgentRules => "agent-rules",
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Target {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "vscode" => Ok(Self::Vscode),
            "opencode" => Ok(Self::Opencode),
            "claude" => Ok(Self::Claude),
            "agent-rules" | "agent_rules" | "rules" => Ok(Self::AgentRules),
            other => Err(format!(
                "unknown target '{other}'; expected one of: vscode, opencode, claude, agent-rules"
            )),
        }
    }
}

/// A set of AI coding tools to emit artifacts for.
///
/// `TargetSet` is a bit-set style wrapper. Construct one via [`TargetSet::default`]
/// (yields `vscode` for back-compat), [`TargetSet::from_iter`], or
/// [`crate::domain::plan::TargetConfig::resolve`] which applies the CLI/plan/default precedence.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct TargetSet {
    pub vscode: bool,
    pub opencode: bool,
    pub claude: bool,
    pub agent_rules: bool,
}

impl TargetSet {
    /// `vscode` only — preserves pre-`[targets]` behavior. Use this when no
    /// `[targets]` section is present in the plan TOML and no CLI override.
    #[must_use]
    pub const fn vscode_only() -> Self {
        Self {
            vscode: true,
            opencode: false,
            claude: false,
            agent_rules: false,
        }
    }

    /// All targets enabled.
    #[must_use]
    pub const fn all() -> Self {
        Self {
            vscode: true,
            opencode: true,
            claude: true,
            agent_rules: true,
        }
    }

    /// Parse from a CLI string. Accepts `vscode`, `opencode`, `claude`,
    /// `agent-rules`, or `all` (enables every target).
    ///
    /// # Errors
    ///
    /// Returns an error string if `s` is not a recognised target.
    pub fn from_cli_str(s: &str) -> Result<Self, String> {
        if s.eq_ignore_ascii_case("all") {
            return Ok(Self::all());
        }
        let target: Target = s.parse()?;
        Ok(Self::from_iter([target]))
    }

    /// True if at least one target is enabled.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        !(self.vscode || self.opencode || self.claude || self.agent_rules)
    }

    /// True if `target` is enabled in this set.
    #[must_use]
    pub const fn contains(self, target: Target) -> bool {
        match target {
            Target::Vscode => self.vscode,
            Target::Opencode => self.opencode,
            Target::Claude => self.claude,
            Target::AgentRules => self.agent_rules,
        }
    }

    /// Enable a single target.
    pub const fn enable(&mut self, target: Target) {
        match target {
            Target::Vscode => self.vscode = true,
            Target::Opencode => self.opencode = true,
            Target::Claude => self.claude = true,
            Target::AgentRules => self.agent_rules = true,
        }
    }

    /// Returns an iterator over enabled targets in stable order.
    pub fn iter(self) -> impl Iterator<Item = Target> {
        let mut out = Vec::with_capacity(4);
        if self.vscode {
            out.push(Target::Vscode);
        }
        if self.opencode {
            out.push(Target::Opencode);
        }
        if self.claude {
            out.push(Target::Claude);
        }
        if self.agent_rules {
            out.push(Target::AgentRules);
        }
        out.into_iter()
    }
}

impl std::iter::FromIterator<Target> for TargetSet {
    fn from_iter<I: IntoIterator<Item = Target>>(iter: I) -> Self {
        let mut set = Self::default();
        for t in iter {
            set.enable(t);
        }
        set
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::needless_update)]
mod tests {
    use super::*;

    #[test]
    fn vscode_only_constructor() {
        let s = TargetSet::vscode_only();
        assert!(s.contains(Target::Vscode));
        assert!(!s.contains(Target::Opencode));
        assert!(!s.contains(Target::Claude));
    }

    #[test]
    fn from_cli_str_parses_known_values() {
        assert_eq!(
            TargetSet::from_cli_str("vscode").unwrap(),
            TargetSet::from_iter([Target::Vscode])
        );
        assert_eq!(
            TargetSet::from_cli_str("opencode").unwrap(),
            TargetSet::from_iter([Target::Opencode])
        );
        assert_eq!(
            TargetSet::from_cli_str("claude").unwrap(),
            TargetSet::from_iter([Target::Claude])
        );
    }

    #[test]
    fn from_cli_str_all_enables_everything() {
        let s = TargetSet::from_cli_str("all").unwrap();
        assert!(s.vscode && s.opencode && s.claude);
    }

    #[test]
    fn from_cli_str_is_case_insensitive() {
        assert_eq!(
            TargetSet::from_cli_str("OpenCode").unwrap(),
            TargetSet::from_iter([Target::Opencode])
        );
    }

    #[test]
    fn from_cli_str_rejects_unknown() {
        assert!(TargetSet::from_cli_str("cursor").is_err());
    }

    #[test]
    fn iter_yields_enabled_in_stable_order() {
        let s = TargetSet {
            claude: true,
            vscode: true,
            opencode: true,
            agent_rules: true,
        };
        let v: Vec<_> = s.iter().collect();
        assert_eq!(
            v,
            vec![
                Target::Vscode,
                Target::Opencode,
                Target::Claude,
                Target::AgentRules,
            ]
        );
    }

    #[test]
    fn empty_set_reports_empty() {
        let s = TargetSet {
            vscode: false,
            opencode: false,
            claude: false,
            agent_rules: false,
        };
        assert!(s.is_empty());
        assert_eq!(s.iter().count(), 0);
    }

    #[test]
    fn target_display_round_trips() {
        for t in Target::ALL {
            assert_eq!(t.to_string().parse::<Target>().unwrap(), *t);
        }
    }

    #[test]
    fn agent_rules_parses_variants_and_aliases() {
        for s in ["agent-rules", "agent_rules", "rules", "Agent-Rules"] {
            assert_eq!(
                Target::from_str(s).unwrap(),
                Target::AgentRules,
                "input `{s}` should parse as AgentRules"
            );
        }
    }
}
