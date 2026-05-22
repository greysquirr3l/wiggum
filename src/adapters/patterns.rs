//! Pattern store — save and re-apply reusable plan patterns from retros.
//!
//! Patterns are saved as TOML files in `~/.wiggum/patterns/`.
//! IDs are derived from the system clock (no `uuid` crate required).

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::adapters::retro::RetroSummary;
use crate::domain::plan::Plan;
use crate::error::{Result, WiggumError};

/// A reusable pattern extracted from a retrospective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub project_name: String,
    pub created_at_unix: u64,
    pub language: String,
    pub suggestions: Vec<String>,
    pub source: PatternSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternSource {
    Retro,
    Progress,
}

impl std::fmt::Display for PatternSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Retro => write!(f, "retro"),
            Self::Progress => write!(f, "progress"),
        }
    }
}

/// Generate a timestamp-based pattern ID.
fn new_pattern_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    format!("pat-{ts}")
}

/// Return the default patterns directory: `~/.wiggum/patterns/`.
#[must_use]
pub fn default_patterns_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".wiggum").join("patterns"))
}

/// List all patterns in `patterns_dir`.
///
/// # Errors
///
/// Returns an error if the directory cannot be read or a file cannot be parsed.
pub fn list(patterns_dir: &Path) -> Result<Vec<Pattern>> {
    if !patterns_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut patterns = Vec::new();
    let entries = std::fs::read_dir(patterns_dir).map_err(|e| {
        WiggumError::Io(std::io::Error::new(
            e.kind(),
            format!("Cannot read patterns dir {}: {e}", patterns_dir.display()),
        ))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            WiggumError::Io(std::io::Error::new(
                e.kind(),
                format!("Cannot read dir entry: {e}"),
            ))
        })?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            let content = std::fs::read_to_string(&path).map_err(|e| {
                WiggumError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Cannot read {}: {e}", path.display()),
                ))
            })?;
            let pattern: Pattern = toml::from_str(&content).map_err(|e| {
                WiggumError::Validation(format!("Invalid pattern file {}: {e}", path.display()))
            })?;
            patterns.push(pattern);
        }
    }

    patterns.sort_by_key(|a| a.created_at_unix);
    Ok(patterns)
}

/// Save a pattern derived from a `RetroSummary`.
///
/// Returns the path of the written pattern file.
///
/// # Errors
///
/// Returns an error if the patterns directory cannot be created or the file
/// cannot be written.
pub fn save_from_retro(
    summary: &RetroSummary,
    plan: &Plan,
    patterns_dir: &Path,
) -> Result<PathBuf> {
    let suggestions: Vec<String> = summary
        .suggestions
        .iter()
        .map(|s| s.suggestion.clone())
        .collect();

    let pattern = Pattern {
        id: new_pattern_id(),
        project_name: summary.project_name.clone(),
        created_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
        language: format!("{:?}", plan.project.language).to_lowercase(),
        suggestions,
        source: PatternSource::Retro,
    };

    write_pattern(&pattern, patterns_dir)
}

/// Save a pattern derived from PROGRESS.md.
///
/// Returns the path of the written pattern file.
///
/// # Errors
///
/// Returns an error if parsing or writing fails.
pub fn save_from_progress(
    progress_path: &Path,
    plan: &Plan,
    patterns_dir: &Path,
) -> Result<PathBuf> {
    let content = std::fs::read_to_string(progress_path).map_err(|e| {
        WiggumError::Io(std::io::Error::new(
            e.kind(),
            format!("Cannot read {}: {e}", progress_path.display()),
        ))
    })?;

    // Extract lines tagged with [~] (in-progress / blocked) as learning material
    let suggestions: Vec<String> = content
        .lines()
        .filter(|l| l.contains("[~]") || l.to_lowercase().contains("required fix"))
        .map(|l| l.trim().to_string())
        .collect();

    let pattern = Pattern {
        id: new_pattern_id(),
        project_name: plan.project.name.clone(),
        created_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
        language: format!("{:?}", plan.project.language).to_lowercase(),
        suggestions,
        source: PatternSource::Progress,
    };

    write_pattern(&pattern, patterns_dir)
}

/// Apply matching patterns to augment hints in a plan file.
///
/// Returns the list of hint strings added.
///
/// # Errors
///
/// Returns an error if the plan cannot be parsed or the pattern directory
/// cannot be read.
pub fn apply(plan_path: &Path, patterns_dir: &Path) -> Result<Vec<String>> {
    let toml_content = std::fs::read_to_string(plan_path).map_err(|e| {
        WiggumError::Io(std::io::Error::new(
            e.kind(),
            format!("Cannot read {}: {e}", plan_path.display()),
        ))
    })?;
    let plan = Plan::from_toml(&toml_content)?;
    let language = format!("{:?}", plan.project.language).to_lowercase();

    let patterns = list(patterns_dir)?;
    let matching: Vec<&Pattern> = patterns.iter().filter(|p| p.language == language).collect();

    if matching.is_empty() {
        return Ok(Vec::new());
    }

    let mut applied = Vec::new();
    for p in &matching {
        for suggestion in &p.suggestions {
            applied.push(format!("[Pattern {}] {suggestion}", p.id));
        }
    }

    Ok(applied)
}

fn write_pattern(pattern: &Pattern, patterns_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(patterns_dir).map_err(|e| {
        WiggumError::Io(std::io::Error::new(
            e.kind(),
            format!("Cannot create patterns dir {}: {e}", patterns_dir.display()),
        ))
    })?;

    let filename = format!("{}.toml", pattern.id);
    let path = patterns_dir.join(&filename);

    let serialized = toml::to_string_pretty(&pattern)
        .map_err(|e| WiggumError::Validation(format!("Failed to serialize pattern: {e}")))?;

    std::fs::write(&path, &serialized).map_err(|e| {
        WiggumError::Io(std::io::Error::new(
            e.kind(),
            format!("Cannot write {}: {e}", path.display()),
        ))
    })?;

    Ok(path)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn list_returns_empty_for_missing_dir() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("nope");
        let result = list(&missing).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn write_and_list_roundtrip() {
        let dir = TempDir::new().unwrap();
        let pat = Pattern {
            id: "pat-123".to_string(),
            project_name: "test".to_string(),
            created_at_unix: 1_000_000,
            language: "rust".to_string(),
            suggestions: vec!["Use tokio".to_string()],
            source: PatternSource::Retro,
        };
        write_pattern(&pat, dir.path()).unwrap();

        let patterns = list(dir.path()).unwrap();
        assert_eq!(patterns.len(), 1);
        let p = patterns.first().unwrap();
        assert_eq!(p.project_name, "test");
        assert_eq!(p.suggestions.first().unwrap(), "Use tokio");
    }

    #[test]
    fn apply_returns_matching_language_suggestions() {
        let dir = TempDir::new().unwrap();
        let pat = Pattern {
            id: "pat-456".to_string(),
            project_name: "other-project".to_string(),
            created_at_unix: 1_000_001,
            language: "rust".to_string(),
            suggestions: vec!["Prefer ? over unwrap".to_string()],
            source: PatternSource::Retro,
        };
        write_pattern(&pat, dir.path()).unwrap();

        let plan_toml = r#"
[project]
name = "apply-test"
path = "/tmp/apply-test"
description = "Apply test"
language = "rust"

[[phases]]
name = "Phase 1"
order = 1

[[phases.tasks]]
slug = "task-a"
title = "Task A"
goal = "Do a thing"
"#;
        let plan_dir = TempDir::new().unwrap();
        let plan_path = plan_dir.path().join("plan.toml");
        std::fs::write(&plan_path, plan_toml).unwrap();

        let hints = apply(&plan_path, dir.path()).unwrap();
        assert_eq!(hints.len(), 1);
        assert!(hints.first().unwrap().contains("Prefer ? over unwrap"));
    }
}
