//! Hints file parser — supports both TOML (structured) and Markdown (freeform).
//!
//! Detection is by file extension: `.toml` → TOML, `.md`/`.markdown` → Markdown.
//! Anything else returns a clear error.
//!
//! # TOML schema (`hints.toml`)
//!
//! Field names are kebab-case (TOML convention). The full set of supported
//! fields:
//!
//! ```toml
//! [project]
//! name = "my-app"
//! description = "..."
//! language = "go"          # overrides detected language
//! architecture = "ddd"     # overrides detected architecture
//! extra-rules = [
//!   "Use the latest stable Go",
//!   "No globals — wire deps through constructors",
//! ]
//!
//! [orchestrator]
//! persona = "..."
//! strategy = "complete"    # standard | tdd | gsd | complete
//! max-retries = 3
//! on-failure = "pause"     # pause | skip | escalate
//! extra-rules = [
//!   "Run `go vet ./...` before committing",
//! ]
//!
//! [[phase]]
//! name = "Domain Core"
//!
//! [[phase.tasks]]
//! slug = "aggregate-roots"
//! title = "Define aggregate roots"
//! goal = "Identify and implement the core aggregates"
//! depends-on = []
//! hints = ["Start with the Account aggregate", "..."]
//! gate = "approve-domain-model"
//! ```
//!
//! # Markdown (`hints.md`)
//!
//! Lines under `## Rules` (or `## Code style`, `## Orchestrator Rules`) become
//! `orchestrator.extra_rules`. Everything else is ignored — Markdown is the
//! brain-dump format.

use std::path::Path;

use serde::Deserialize;

use crate::domain::plan::{FailureAction, Strategy};
use crate::error::{Result, WiggumError};

/// Top-level hints container.
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)] // fine for a config struct
pub struct Hints {
    pub project: ProjectHints,
    pub orchestrator: OrchestratorHints,
    pub phases: Vec<PhaseHints>,
}

/// Project-level hint overrides.
#[derive(Debug, Clone, Default)]
pub struct ProjectHints {
    pub name: Option<String>,
    pub description: Option<String>,
    pub language: Option<crate::domain::plan::Language>,
    pub architecture: Option<String>,
    pub extra_rules: Vec<String>,
}

/// Orchestrator-level hint overrides.
#[derive(Debug, Clone, Default)]
pub struct OrchestratorHints {
    pub persona: Option<String>,
    pub strategy: Option<Strategy>,
    pub max_retries: Option<u32>,
    pub on_failure: Option<FailureAction>,
    pub extra_rules: Vec<String>,
}

/// One declared phase + its tasks.
#[derive(Debug, Clone, Default)]
pub struct PhaseHints {
    pub name: String,
    pub tasks: Vec<TaskHints>,
}

/// One declared task.
#[derive(Debug, Clone, Default)]
pub struct TaskHints {
    pub slug: String,
    pub title: String,
    pub goal: String,
    pub depends_on: Vec<String>,
    pub hints: Vec<String>,
    pub gate: Option<String>,
}

// ── TOML schema ────────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct TomlHints {
    #[serde(default)]
    project: TomlProject,
    #[serde(default)]
    orchestrator: TomlOrchestrator,
    #[serde(default)]
    phase: Vec<TomlPhase>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct TomlProject {
    name: Option<String>,
    description: Option<String>,
    language: Option<String>,
    architecture: Option<String>,
    #[serde(default)]
    extra_rules: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct TomlOrchestrator {
    persona: Option<String>,
    strategy: Option<String>,
    max_retries: Option<u32>,
    on_failure: Option<String>,
    #[serde(default)]
    extra_rules: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct TomlPhase {
    name: String,
    #[serde(default)]
    tasks: Vec<TomlTask>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct TomlTask {
    slug: String,
    title: String,
    goal: String,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    hints: Vec<String>,
    gate: Option<String>,
}

// ── Public entry ──────────────────────────────────────────────────────────

/// Parse a hints file. Dispatches on extension.
///
/// # Errors
///
/// Returns an error if the file extension is not `.toml`/`.md`/`.markdown`,
/// or if parsing fails.
pub fn parse_hints(text: &str, path: &Path) -> Result<Hints> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "toml" => parse_toml(text),
        "md" | "markdown" => Ok(parse_markdown(text)),
        other => Err(WiggumError::Validation(format!(
            "unrecognized hints file extension '{other}' (expected .toml, .md, or .markdown)"
        ))),
    }
}

fn parse_toml(text: &str) -> Result<Hints> {
    let raw: TomlHints = toml::from_str(text)
        .map_err(|e| WiggumError::Validation(format!("hints TOML parse error: {e}")))?;

    let language = raw
        .project
        .language
        .as_deref()
        .map(parse_language)
        .transpose()?;

    let strategy = raw
        .orchestrator
        .strategy
        .as_deref()
        .map(parse_strategy)
        .transpose()?;

    let on_failure = raw
        .orchestrator
        .on_failure
        .as_deref()
        .map(parse_failure_action)
        .transpose()?;

    let phases = raw
        .phase
        .into_iter()
        .map(|p| PhaseHints {
            name: p.name,
            tasks: p
                .tasks
                .into_iter()
                .map(|t| TaskHints {
                    slug: t.slug,
                    title: t.title,
                    goal: t.goal,
                    depends_on: t.depends_on,
                    hints: t.hints,
                    gate: t.gate,
                })
                .collect(),
        })
        .collect();

    Ok(Hints {
        project: ProjectHints {
            name: raw.project.name,
            description: raw.project.description,
            language,
            architecture: raw.project.architecture,
            extra_rules: raw.project.extra_rules,
        },
        orchestrator: OrchestratorHints {
            persona: raw.orchestrator.persona,
            strategy,
            max_retries: raw.orchestrator.max_retries,
            on_failure,
            extra_rules: raw.orchestrator.extra_rules,
        },
        phases,
    })
}

fn parse_markdown(text: &str) -> Hints {
    // Freeform Markdown: every non-empty bullet under `## Rules` becomes a rule.
    // Everything else is silently ignored — Markdown is the brain-dump format.
    let mut rules: Vec<String> = Vec::new();
    let mut in_rules = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(header) = trimmed.strip_prefix("## ") {
            in_rules = header.eq_ignore_ascii_case("Rules")
                || header.eq_ignore_ascii_case("Code style")
                || header.eq_ignore_ascii_case("Orchestrator Rules");
            continue;
        }
        if in_rules
            && let Some(rule) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
        {
            let rule = rule.trim();
            if !rule.is_empty() {
                rules.push(rule.to_string());
            }
        }
    }

    Hints {
        orchestrator: OrchestratorHints {
            extra_rules: rules,
            ..Default::default()
        },
        ..Default::default()
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn parse_language(s: &str) -> Result<crate::domain::plan::Language> {
    use crate::domain::plan::Language;
    let normalized = s.trim().to_ascii_lowercase().replace(['_', '-'], "");
    let lang = match normalized.as_str() {
        "rust" => Language::Rust,
        "go" | "golang" => Language::Go,
        "typescript" | "ts" | "javascript" | "js" => Language::TypeScript,
        "python" | "py" => Language::Python,
        "java" => Language::Java,
        "csharp" | "cs" | "c#" => Language::CSharp,
        "kotlin" | "kt" => Language::Kotlin,
        "swift" => Language::Swift,
        "ruby" => Language::Ruby,
        "elixir" => Language::Elixir,
        "php" => Language::Php,
        other => {
            return Err(WiggumError::Validation(format!(
                "unknown language '{other}' in hints.toml"
            )));
        }
    };
    Ok(lang)
}

fn parse_strategy(s: &str) -> Result<Strategy> {
    match s.trim().to_ascii_lowercase().as_str() {
        "standard" | "default" => Ok(Strategy::Standard),
        "tdd" => Ok(Strategy::Tdd),
        "gsd" => Ok(Strategy::Gsd),
        "complete" => Ok(Strategy::Complete),
        other => Err(WiggumError::Validation(format!(
            "unknown strategy '{other}' in hints.toml (expected standard|tdd|gsd|complete)"
        ))),
    }
}

fn parse_failure_action(s: &str) -> Result<FailureAction> {
    match s.trim().to_ascii_lowercase().as_str() {
        "pause" => Ok(FailureAction::Pause),
        "skip" => Ok(FailureAction::Skip),
        "escalate" => Ok(FailureAction::Escalate),
        other => Err(WiggumError::Validation(format!(
            "unknown on_failure action '{other}' in hints.toml (expected pause|skip|escalate)"
        ))),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn toml_path() -> PathBuf {
        PathBuf::from("hints.toml")
    }
    fn md_path() -> PathBuf {
        PathBuf::from("hints.md")
    }

    #[test]
    fn parse_minimal_toml() {
        let text = r#"
[project]
language = "go"
architecture = "ddd"
"#;
        let h = parse_hints(text, &toml_path()).unwrap();
        assert!(matches!(
            h.project.language,
            Some(crate::domain::plan::Language::Go)
        ));
        assert_eq!(h.project.architecture.as_deref(), Some("ddd"));
    }

    #[test]
    fn parse_full_toml_with_phases() {
        let text = r#"
[project]
name = "my-app"
description = "An example app"
language = "go"
architecture = "ddd"
extra-rules = ["Use latest stable Go", "No globals"]

[orchestrator]
persona = "Senior Go engineer"
strategy = "complete"
max-retries = 3
on-failure = "pause"
extra-rules = ["Run `go vet ./...`"]

[[phase]]
name = "Domain Core"

[[phase.tasks]]
slug = "aggregate-roots"
title = "Define aggregates"
goal = "Identify and implement the core aggregates"
hints = ["Start with Account", "Use value objects"]
gate = "approve-domain"

[[phase.tasks]]
slug = "domain-services"
title = "Implement domain services"
goal = "Wire aggregates through services"
depends-on = ["aggregate-roots"]
"#;
        let h = parse_hints(text, &toml_path()).unwrap();
        assert_eq!(h.project.name.as_deref(), Some("my-app"));
        assert_eq!(h.project.extra_rules.len(), 2);
        assert_eq!(
            h.orchestrator.persona.as_deref(),
            Some("Senior Go engineer")
        );
        assert!(matches!(h.orchestrator.strategy, Some(Strategy::Complete)));
        assert_eq!(h.orchestrator.max_retries, Some(3));
        assert!(matches!(
            h.orchestrator.on_failure,
            Some(FailureAction::Pause)
        ));
        assert_eq!(h.phases.len(), 1);
        let p = &h.phases[0];
        assert_eq!(p.name, "Domain Core");
        assert_eq!(p.tasks.len(), 2);
        assert_eq!(p.tasks[0].slug, "aggregate-roots");
        assert_eq!(p.tasks[0].gate.as_deref(), Some("approve-domain"));
        assert_eq!(p.tasks[1].depends_on, vec!["aggregate-roots".to_string()]);
    }

    #[test]
    fn parse_unknown_language_errors() {
        let text = r#"
[project]
language = "cobol"
"#;
        assert!(parse_hints(text, &toml_path()).is_err());
    }

    #[test]
    fn parse_unknown_strategy_errors() {
        let text = r#"
[orchestrator]
strategy = "vibes"
"#;
        assert!(parse_hints(text, &toml_path()).is_err());
    }

    #[test]
    fn parse_unknown_on_failure_errors() {
        let text = r#"
[orchestrator]
on-failure = "explode"
"#;
        assert!(parse_hints(text, &toml_path()).is_err());
    }

    #[test]
    fn parse_unknown_extension_errors() {
        let text = "anything";
        let err = parse_hints(text, &PathBuf::from("hints.json")).unwrap_err();
        assert!(format!("{err}").contains("unrecognized hints file extension"));
    }

    #[test]
    fn parse_markdown_rules_only() {
        let text = r"
# Project notes

Some preamble here that should be ignored.

## Rules

- Use the latest stable Go
- No globals — wire deps through constructors
- Run `go vet ./...` before committing

## Other section

- This should NOT be picked up
";
        let h = parse_hints(text, &md_path()).unwrap();
        assert_eq!(h.orchestrator.extra_rules.len(), 3);
        assert_eq!(h.orchestrator.extra_rules[0], "Use the latest stable Go");
        assert_eq!(
            h.orchestrator.extra_rules[1],
            "No globals — wire deps through constructors"
        );
        assert_eq!(
            h.orchestrator.extra_rules[2],
            "Run `go vet ./...` before committing"
        );
        assert!(h.project.name.is_none());
        assert!(h.phases.is_empty());
    }

    #[test]
    fn parse_markdown_with_code_style_header() {
        let text = r"
## Code style

- Run `cargo fmt` before committing
- Use rustfmt defaults
";
        let h = parse_hints(text, &md_path()).unwrap();
        assert_eq!(h.orchestrator.extra_rules.len(), 2);
    }

    #[test]
    fn parse_markdown_star_bullets() {
        let text = r"
## Rules

* Bullet with star
* Another bullet
";
        let h = parse_hints(text, &md_path()).unwrap();
        assert_eq!(h.orchestrator.extra_rules.len(), 2);
    }
}
