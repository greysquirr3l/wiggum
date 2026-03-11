use serde::{Deserialize, Serialize};

use crate::error::{Result, WiggumError};

use super::languages::LanguageProfile;

/// Top-level plan definition, parsed from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub project: Project,
    #[serde(default)]
    pub preflight: Preflight,
    #[serde(default)]
    pub orchestrator: Orchestrator,
    pub phases: Vec<Phase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub description: String,
    #[serde(default = "default_language")]
    pub language: Language,
    /// Absolute path to the target project root.
    pub path: String,
    #[serde(default)]
    pub architecture: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Preflight {
    pub build: String,
    pub test: String,
    pub lint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Orchestrator {
    #[serde(default = "default_persona")]
    pub persona: String,
    #[serde(default)]
    pub strategy: Strategy,
    #[serde(default)]
    pub rules: Vec<String>,
}

/// Prompt strategy mode controlling task and orchestrator template styles.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Strategy {
    /// Goal → implement → test → preflight (default).
    #[default]
    Standard,
    /// Red test first → implement to green → refactor → preflight.
    Tdd,
    /// Must-haves checklist → implement → verify all must-haves.
    Gsd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub name: String,
    pub order: u32,
    pub tasks: Vec<TaskDef>,
}

/// A task definition as written in the plan TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDef {
    pub slug: String,
    pub title: String,
    pub goal: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Optional implementation hints the user can provide.
    #[serde(default)]
    pub hints: Vec<String>,
    /// Optional test requirements the user can describe.
    #[serde(default)]
    pub test_hints: Vec<String>,
    /// Must-have deliverables — used by the GSD strategy.
    #[serde(default)]
    pub must_haves: Vec<String>,
    /// Human-in-the-loop gate: when set, the orchestrator must emit this
    /// message and pause for human confirmation before starting this task.
    #[serde(default)]
    pub gate: Option<String>,
}

/// A resolved task with its assigned number (T01, T02, ...).
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedTask {
    pub number: u32,
    pub slug: String,
    pub title: String,
    pub goal: String,
    pub depends_on: Vec<String>,
    pub hints: Vec<String>,
    pub test_hints: Vec<String>,
    pub must_haves: Vec<String>,
    /// Human-in-the-loop gate message, if any.
    pub gate: Option<String>,
    pub phase_name: String,
    pub phase_order: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Go,
    #[serde(rename = "typescript")]
    TypeScript,
    Python,
    Java,
    #[serde(rename = "csharp")]
    CSharp,
    Kotlin,
    Swift,
    Ruby,
    Elixir,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rust => write!(f, "rust"),
            Self::Go => write!(f, "go"),
            Self::TypeScript => write!(f, "typescript"),
            Self::Python => write!(f, "python"),
            Self::Java => write!(f, "java"),
            Self::CSharp => write!(f, "csharp"),
            Self::Kotlin => write!(f, "kotlin"),
            Self::Swift => write!(f, "swift"),
            Self::Ruby => write!(f, "ruby"),
            Self::Elixir => write!(f, "elixir"),
        }
    }
}

impl Language {
    /// All supported languages.
    pub const ALL: &[Self] = &[
        Self::Rust,
        Self::Go,
        Self::TypeScript,
        Self::Python,
        Self::Java,
        Self::CSharp,
        Self::Kotlin,
        Self::Swift,
        Self::Ruby,
        Self::Elixir,
    ];

    /// Get the language profile containing best practices and tool defaults.
    #[must_use]
    pub fn profile(self) -> &'static LanguageProfile {
        super::languages::profile(self)
    }
}

const fn default_language() -> Language {
    Language::Rust
}

fn default_persona() -> String {
    "You are a senior software engineer".to_string()
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self {
            persona: default_persona(),
            strategy: Strategy::default(),
            rules: Vec::new(),
        }
    }
}

impl std::fmt::Display for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Standard => write!(f, "standard"),
            Self::Tdd => write!(f, "tdd"),
            Self::Gsd => write!(f, "gsd"),
        }
    }
}

impl Preflight {
    /// Returns preflight commands with language-specific defaults
    /// for any field left empty.
    #[must_use]
    pub fn with_defaults(mut self, language: Language) -> Self {
        let profile = language.profile();

        if self.build.is_empty() {
            self.build = profile.build_cmd.to_string();
        }
        if self.test.is_empty() {
            self.test = profile.test_cmd.to_string();
        }
        if self.lint.is_empty() {
            self.lint = profile.lint_cmd.to_string();
        }
        self
    }
}

impl Plan {
    /// Parse a plan from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns an error if the TOML is malformed or missing required fields.
    pub fn from_toml(input: &str) -> Result<Self> {
        let mut plan: Self = toml::from_str(input)?;
        plan.preflight = plan.preflight.with_defaults(plan.project.language);
        Ok(plan)
    }

    /// Resolve phases into a flat, numbered task list.
    /// Tasks are numbered sequentially across all phases, ordered by phase order.
    ///
    /// # Errors
    ///
    /// Returns an error if duplicate task slugs are found.
    pub fn resolve_tasks(&self) -> Result<Vec<ResolvedTask>> {
        let mut phases = self.phases.clone();
        phases.sort_by_key(|p| p.order);

        let mut resolved = Vec::new();
        let mut number = 1u32;

        for phase in &phases {
            for task in &phase.tasks {
                resolved.push(ResolvedTask {
                    number,
                    slug: task.slug.clone(),
                    title: task.title.clone(),
                    goal: task.goal.clone(),
                    depends_on: task.depends_on.clone(),
                    hints: task.hints.clone(),
                    test_hints: task.test_hints.clone(),
                    must_haves: task.must_haves.clone(),
                    gate: task.gate.clone(),
                    phase_name: phase.name.clone(),
                    phase_order: phase.order,
                });
                number += 1;
            }
        }

        // Validate: no duplicate slugs
        let mut seen = std::collections::HashSet::new();
        for t in &resolved {
            if !seen.insert(&t.slug) {
                return Err(WiggumError::DuplicateSlug(t.slug.clone()));
            }
        }

        // Validate: all dependencies reference existing slugs
        let all_slugs: std::collections::HashSet<&str> =
            resolved.iter().map(|t| t.slug.as_str()).collect();
        for t in &resolved {
            for dep in &t.depends_on {
                if !all_slugs.contains(dep.as_str()) {
                    return Err(WiggumError::UnknownDependency {
                        referenced: dep.clone(),
                        referencing: t.slug.clone(),
                    });
                }
            }
        }

        Ok(resolved)
    }
}
