use serde::{Deserialize, Serialize};

use crate::error::{Result, WiggumError};

use super::languages::LanguageProfile;
use super::targets::TargetSet;

/// Top-level plan definition, parsed from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub project: Project,
    #[serde(default)]
    pub preflight: Preflight,
    #[serde(default)]
    pub orchestrator: Orchestrator,
    #[serde(default)]
    pub evaluator: Option<EvaluatorConfig>,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub integration: IntegrationConfig,
    #[serde(default)]
    pub style: StyleConfig,
    /// Per-target output control. When unset, only the `vscode` target is
    /// generated (back-compat). The `--target` CLI flag overrides this.
    #[serde(default)]
    pub targets: TargetConfig,
    pub phases: Vec<Phase>,
}

/// Plan-level target configuration.
///
/// Mirrors the `TargetSet` bit-set but uses `Option<bool>` per target so a
/// `[targets]` section can selectively enable or disable individual targets
/// without unsetting the others.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TargetConfig {
    /// Emit `.vscode/*.prompt.md` files. Defaults to `true` if the section
    /// is present, otherwise `TargetSet::vscode_only()` is used.
    #[serde(default)]
    pub vscode: Option<bool>,
    /// Emit `.opencode/agents/wiggum-*.md` agent files. Defaults to `false`.
    #[serde(default)]
    pub opencode: Option<bool>,
    /// Emit `.claude/settings.json` hooks plus `CLAUDE.md` project memory.
    /// Defaults to `false`.
    #[serde(default)]
    pub claude: Option<bool>,
    /// Emit fork-neutral rules files (`.cursorrules`, `.windsurfrules`,
    /// `.github/copilot-instructions.md`) for VSCode-family forks that
    /// don't speak the Copilot `runSubagent` or opencode `task` protocols
    /// (Cursor, Windsurf, etc.). Defaults to `false`.
    #[serde(default, alias = "agent-rules")]
    pub agent_rules: Option<bool>,
}

impl TargetConfig {
    /// Resolve to a `TargetSet`. When all options are `None`, returns the
    /// back-compat default (`vscode` only) via `TargetSet::vscode_only()`.
    /// Otherwise, the absent fields are treated as `false`.
    #[must_use]
    pub fn resolve(self) -> TargetSet {
        if self.vscode.is_none()
            && self.opencode.is_none()
            && self.claude.is_none()
            && self.agent_rules.is_none()
        {
            return TargetSet::vscode_only();
        }
        TargetSet {
            vscode: self.vscode.unwrap_or(false),
            opencode: self.opencode.unwrap_or(false),
            claude: self.claude.unwrap_or(false),
            agent_rules: self.agent_rules.unwrap_or(false),
        }
    }
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
    /// Supply-chain / vulnerability audit command appended after lint.
    /// Defaults to the language profile's `audit_cmd`.
    /// Set to an empty string to disable auditing for this plan.
    #[serde(default)]
    pub audit: Option<String>,
}

/// A single weighted evaluation criterion for the QA evaluator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCriterion {
    /// Short name identifying the criterion (e.g. "tests-pass").
    pub name: String,
    /// Relative weight in the 0–100 range. All weights in the `criteria` vec
    /// must sum to 100 when the vec is non-empty.
    pub weight: u8,
    /// Human-readable description of what must be true for this criterion to pass.
    pub description: String,
}

/// Whether the evaluator operates as a hard gate or a non-blocking advisor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvalMode {
    /// Evaluator verdict blocks the orchestrator from proceeding (default).
    #[default]
    Blocking,
    /// Evaluator runs and reports findings but does not block task progression.
    Advisor,
}

/// Configuration for the evaluator/QA agent generated alongside the subagent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatorConfig {
    /// Persona injected into the evaluator system prompt.
    #[serde(default = "default_evaluator_persona")]
    pub persona: String,
    /// Minimum score (0–10) the evaluator must assign before a task passes.
    #[serde(default = "default_pass_threshold")]
    pub pass_threshold: u8,
    /// When true, any single failing criterion immediately fails the task,
    /// regardless of the overall score.
    #[serde(default)]
    pub hard_fail: bool,
    /// Shell command the evaluator runs to verify the implementation
    /// (e.g. `"cargo test --workspace"`). Falls back to preflight.test when absent.
    #[serde(default)]
    pub test_tool: Option<String>,
    /// Weighted evaluation criteria. When non-empty, weights must sum to 100.
    /// When empty, the evaluator uses a flat criterion list derived from the task.
    #[serde(default)]
    pub criteria: Vec<EvalCriterion>,
    /// Whether the evaluator blocks task progression (Blocking) or advises only (Advisor).
    #[serde(default)]
    pub mode: EvalMode,
    /// When true, the orchestrator runs a two-phase contract-review loop before
    /// dispatching the implementation subagent. See the `contract_review` template section.
    #[serde(default)]
    pub contract_review: bool,
    /// Model identifier for the evaluator agent (e.g. `"claude-sonnet-4.5"`).
    /// Rendered as a header note in `evaluator.prompt.md` and passed as the
    /// `model:` argument when the orchestrator dispatches the evaluator as a
    /// subagent. When unset, the evaluator inherits the dispatcher's model.
    #[serde(default)]
    pub model: Option<String>,
}

impl EvaluatorConfig {
    /// Validate the evaluator configuration.
    ///
    /// # Errors
    ///
    /// Returns `WiggumError::Validation` if the criteria weights are non-empty
    /// but do not sum to exactly 100.
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.criteria.is_empty() {
            return Ok(());
        }
        let total: u32 = self.criteria.iter().map(|c| u32::from(c.weight)).sum();
        if total != 100 {
            return Err(WiggumError::Validation(format!(
                "evaluator criteria weights must sum to 100, got {total}"
            )));
        }
        Ok(())
    }
}

fn default_evaluator_persona() -> String {
    "You are a skeptical senior engineer acting as a QA evaluator. \
     Your job is to verify that the implementation actually meets the stated criteria — \
     not just that it compiles or that the author says it's done."
        .to_string()
}

const fn default_pass_threshold() -> u8 {
    7
}

impl Default for EvaluatorConfig {
    fn default() -> Self {
        Self {
            persona: default_evaluator_persona(),
            pass_threshold: default_pass_threshold(),
            hard_fail: false,
            test_tool: None,
            criteria: Vec::new(),
            mode: EvalMode::Blocking,
            contract_review: false,
            model: None,
        }
    }
}

/// What the orchestrator should do when a task exhausts its retry budget.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FailureAction {
    /// Emit a GATE banner and stop — human must restart to proceed.
    #[default]
    Pause,
    /// Mark the task `[!]` (blocked) and continue to the next available task.
    Skip,
    /// Emit a structured failure block into PROGRESS.md with diagnosis summary,
    /// then continue to the next available task.
    Escalate,
}

impl std::fmt::Display for FailureAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pause => write!(f, "pause"),
            Self::Skip => write!(f, "skip"),
            Self::Escalate => write!(f, "escalate"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Orchestrator {
    #[serde(default = "default_persona")]
    pub persona: String,
    #[serde(default)]
    pub strategy: Strategy,
    #[serde(default)]
    pub rules: Vec<String>,
    /// Maximum number of preflight-fail/retry cycles before the orchestrator
    /// applies `on_failure`. Defaults to `2`.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Action taken when a task exhausts `max_retries`. Defaults to `pause`.
    #[serde(default)]
    pub on_failure: FailureAction,
    /// Recommended model identifier for the orchestrator agent itself
    /// (e.g. `"claude-opus-4.7"`, `"gpt-5"`, `"gemini-2.5-pro"`). Rendered as
    /// a header note in `orchestrator.prompt.md` so the human knows which model
    /// to select in the chat picker before running the prompt.
    #[serde(default)]
    pub model: Option<String>,
    /// Model identifier passed as the `model:` argument to every `runSubagent`
    /// call the orchestrator dispatches for implementation work. When unset,
    /// subagents inherit the orchestrator's model.
    #[serde(default)]
    pub subagent_model: Option<String>,
    /// Whether the plan requires an `[evaluator]` block.
    ///
    /// Tri-state, parsed as `Option<bool>`:
    /// - `None` (omitted in TOML): auto-derive — `true` when the resolved
    ///   plan has ≥4 tasks OR any task slug contains a security-sensitive
    ///   keyword (`auth`, `payment`, `billing`, `crypto`, `credential`,
    ///   `webhook`, `secret`, `key`, `token`, `sign`, `signature`, `kdf`,
    ///   `hash`). See [`auto_derive_require_evaluator`].
    /// - `Some(true)`: evaluator is mandatory — `wiggum validate`/`generate`
    ///   error if the `[evaluator]` section is absent.
    /// - `Some(false)`: evaluator explicitly skipped. `wiggum validate`
    ///   accepts the plan, and `wiggum generate` prints a visible warning
    ///   when the auto-derive rule would have required one.
    #[serde(default)]
    pub require_evaluator: Option<bool>,
    /// Categories of task that require an explicit human gate before the
    /// subagent can begin implementation. When empty (the default), wiggum
    /// auto-derives a list from the resolved task slugs/titles using
    /// [`GATE_KEYWORDS`]. When non-empty, only those categories are
    /// considered gates and every task whose slug or title matches must
    /// declare `gate = "<category>"` in its `TaskDef`; otherwise
    /// `wiggum validate`/`generate` errors with a message naming the
    /// missing gate and the affected task.
    #[serde(default)]
    pub gates: Vec<String>,
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
    /// Complete end-to-end delivery: implement root fix + tests + docs + preflight.
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub name: String,
    pub order: u32,
    pub tasks: Vec<TaskDef>,
}

/// Task archetype — selects a template variant with role-appropriate
/// sections and exit criteria pre-populated for that kind of work.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskKind {
    /// New functionality: emphasises implementation + tests.
    #[default]
    Feature,
    /// Code quality change: emphasises behavioural equivalence before/after.
    Refactor,
    /// CI, config, tooling, infrastructure-as-code work.
    Infrastructure,
    /// Exploratory / spike: produces a document or recommendation, not code.
    Research,
    /// Security or quality audit: produces findings, not new behaviour.
    Audit,
}

impl std::fmt::Display for TaskKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Feature => write!(f, "feature"),
            Self::Refactor => write!(f, "refactor"),
            Self::Infrastructure => write!(f, "infrastructure"),
            Self::Research => write!(f, "research"),
            Self::Audit => write!(f, "audit"),
        }
    }
}

impl TaskKind {
    /// All supported task kinds.
    pub const ALL: &[Self] = &[
        Self::Feature,
        Self::Refactor,
        Self::Infrastructure,
        Self::Research,
        Self::Audit,
    ];
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
    /// Evaluator exit criteria — each item is a verifiable condition that
    /// must pass before the task can be marked complete.
    #[serde(default)]
    pub evaluation_criteria: Vec<String>,
    /// Task archetype — influences template variant and exit criteria.
    /// Defaults to `feature` when omitted.
    #[serde(default)]
    pub kind: TaskKind,
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
    /// Evaluator exit criteria carried over from `TaskDef`.
    pub evaluation_criteria: Vec<String>,
    pub phase_name: String,
    pub phase_order: u32,
    /// Task archetype, propagated from `TaskDef`.
    pub kind: TaskKind,
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
    #[serde(rename = "php")]
    Php,
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
            Self::Php => write!(f, "php"),
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
        Self::Php,
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

const fn default_max_retries() -> u32 {
    2
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self {
            persona: default_persona(),
            strategy: Strategy::default(),
            rules: Vec::new(),
            max_retries: default_max_retries(),
            on_failure: FailureAction::default(),
            model: None,
            subagent_model: None,
            require_evaluator: None,
            gates: Vec::new(),
        }
    }
}

impl std::fmt::Display for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Standard => write!(f, "standard"),
            Self::Tdd => write!(f, "tdd"),
            Self::Gsd => write!(f, "gsd"),
            Self::Complete => write!(f, "complete"),
        }
    }
}

/// Plan-level security configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// When `true`, suppress the automatic injection of the `security-hardening`
    /// task even if web-surface slugs are detected.
    #[serde(default)]
    pub skip_hardening_task: bool,
}

/// Plan-level integration audit configuration.
///
/// Integration audits run as the final tasks before project completion to catch
/// common AI-generated code issues: unwired components and stub implementations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntegrationConfig {
    /// When `true`, suppress the automatic injection of the `integration-wiring`
    /// task that verifies all components are properly connected.
    #[serde(default)]
    pub skip_wiring_audit: bool,

    /// When `true`, suppress the automatic injection of the `stub-cleanup`
    /// task that finds and replaces placeholder implementations.
    #[serde(default)]
    pub skip_stub_audit: bool,
}

/// Plan-level style configuration for AI pattern avoidance.
///
/// When enabled, injects guidance into generated prompts to avoid common
/// When enabled, injects guidance into the orchestrator prompt to avoid
/// common AI-generated code tells: slop vocabulary, obvious comments, and
/// structural patterns that reveal machine authorship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleConfig {
    /// When `true` (default), inject AI pattern avoidance rules into the
    /// orchestrator prompt. These rules guide subagents to write code that
    /// reads as human-authored: avoiding "slop" vocabulary, tutorial-style
    /// comments, and cookie-cutter structure.
    #[serde(default = "default_avoid_ai_patterns")]
    pub avoid_ai_patterns: bool,

    /// When `true` (default), inject guidance that discourages creating
    /// "God" files (single files accumulating unrelated responsibilities).
    /// Encourages focused modules and splitting by concern.
    #[serde(default = "default_avoid_god_files")]
    pub avoid_god_files: bool,

    /// Standing directive on the bar for shipped work. Injected into the
    /// orchestrator prompt and propagated verbatim into every subagent
    /// dispatch alongside `Accumulated Learnings` and `Codebase State`.
    ///
    /// The completion standard is the project's definition of done. It is
    /// edited rarely (typically once, when the project is bootstrapped) and
    /// then travels unchanged with every dispatch so that every fresh
    /// subagent — including ones spun up mid-stream after compaction —
    /// receives the same load-bearing bar.
    ///
    /// Defaults to a generic completion standard appropriate for any
    /// language. Override per project in the plan TOML:
    ///
    /// ```toml
    /// [style]
    /// completion_standard = "All tasks must pass preflight (build + test +
    /// lint) with no warnings, no `#[allow(...)]` suppressions, and no
    /// placeholder implementations remaining in production code."
    /// ```
    #[serde(default)]
    pub completion_standard: Option<String>,

    /// When `true`, inject the active language's strict rule set into the
    /// orchestrator, implementer, evaluator, and per-task prompts. Rules are
    /// language-specific — for Rust they mirror `~/Projects/nick-v2.md`
    /// (DDD-lite hexagonal layout, narrow port traits, `AuthContext` +
    /// idempotency keys, `with_tx` boundaries, object-safe async port
    /// traits via `async-trait`, `LazyCell` for single-threaded contexts,
    /// the strengthened `#[expect]` rule, web security defaults, Rust
    /// 1.95+ syntax — match arm `if let` guards, let chains, `std::io::pipe`,
    /// the 1.97 `pin!` deref coercion fix, `deny(dead_code_pub_in_binary)`,
    /// `#[cfg(unix)]` cross-platform hygiene) plus the original
    /// panic-free baseline (no `.unwrap()` / `.expect()` in production,
    /// no index slicing, no `#[allow(clippy::...)]` suppressions, prefer
    /// `.is_multiple_of(n)`, etc.). The Rust 1.78→1.98 catchup guide at
    /// `~/Projects/rust/rust-docs/rust-catchup-1.78-1.98.md` is the source
    /// of truth for the toolchain version that ships these features. For
    /// Go, TypeScript, Python, Java, C#, Kotlin, Swift, Ruby, Elixir,
    /// and PHP the rulesets follow the profiles in
    /// `docs/strict-lints.md`. Cross-language baselines (fail-secure,
    /// parse-don't-validate, CSPRNG, no weak crypto, parameterised
    /// queries, no untrusted deserialisation, etc.) are folded into each
    /// language's profile.
    ///
    /// Defaults to `false` — back-compat for existing plans that may rely
    /// on ordinary lint passes without the pedantic / strict profile. Set
    /// to `true` in the plan TOML:
    ///
    /// ```toml
    /// [style]
    /// strict = true
    /// ```
    #[serde(default)]
    pub strict: bool,
}

const fn default_avoid_ai_patterns() -> bool {
    true
}

const fn default_avoid_god_files() -> bool {
    true
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            avoid_ai_patterns: default_avoid_ai_patterns(),
            avoid_god_files: default_avoid_god_files(),
            strict: false,
            completion_standard: None,
        }
    }
}

impl StyleConfig {
    /// Resolve the completion standard, falling back to a generic default
    /// when the plan does not set one. The standard is a multi-line string
    /// that is propagated verbatim into every subagent dispatch alongside
    /// `Accumulated Learnings` and `Codebase State`.
    #[must_use]
    pub fn resolved_completion_standard(&self) -> String {
        self.completion_standard
            .clone()
            .unwrap_or_else(default_completion_standard)
    }
}

/// Default completion standard used when the plan does not set one.
/// Generic enough to apply to any language; project-specific overrides
/// should narrow it (e.g. add Rust-specific "no `unwrap()` outside tests").
fn default_completion_standard() -> String {
    "Every task ships when all of the following hold:\n\
     - Preflight (build + test + lint) passes with zero errors and zero warnings.\n\
     - Every exit criterion in the task file has been met, not just the ones that were convenient.\n\
     - No placeholder implementations remain in production code: no `todo!()`, no `unimplemented!()`, \
     no functions that return a hard-coded default value as a stand-in for real logic.\n\
     - No lint suppressions (`#[allow(...)]` / `// nolint` / `# noqa` / etc.) have been added to silence errors — \
     the underlying issue was fixed in code.\n\
     - No new dependency was added without justification in the task Notes column.\n\
     - Public functions added in this task have at least one unit or integration test that exercises them.\n\
     - The Codebase State and Accumulated Learnings sections of PROGRESS.md were updated."
        .to_string()
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
        // Inherit audit_cmd from profile if the user hasn't overridden it.
        // An explicit `audit = ""` in TOML leaves the field as Some(""), which
        // signals "disabled" and will be rendered as None below.
        if self.audit.is_none() && !profile.audit_cmd.is_empty() {
            self.audit = Some(profile.audit_cmd.to_string());
        }
        // Normalise empty-string override to None so templates can use `if audit_cmd`.
        if self.audit.as_deref() == Some("") {
            self.audit = None;
        }
        self
    }
}

impl Plan {
    /// Parse a plan from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns an error if the TOML is malformed, missing required fields, or
    /// fails post-parse validation (gates + evaluator coverage).
    pub fn from_toml(input: &str) -> Result<Self> {
        let mut plan: Self = toml::from_str(input)?;
        plan.preflight = plan.preflight.with_defaults(plan.project.language);
        if let Some(evaluator) = &plan.evaluator {
            evaluator.validate()?;
        }
        plan.orchestrator.gates = auto_derive_gates(&plan.orchestrator.gates, &plan.phases);
        Ok(plan)
    }

    /// Compute the effective `require_evaluator` value after honoring any
    /// explicit user override in the plan TOML.
    ///
    /// - If the plan set `require_evaluator = true|false`, that wins.
    /// - Otherwise, [`auto_derive_require_evaluator`] decides based on the
    ///   resolved task list.
    #[must_use]
    pub fn effective_require_evaluator(&self, resolved: &[ResolvedTask]) -> bool {
        self.orchestrator
            .require_evaluator
            .unwrap_or_else(|| auto_derive_require_evaluator(resolved))
    }

    /// Validate gate and evaluator policies against the resolved task list.
    ///
    /// # Errors
    ///
    /// Returns `WiggumError::Validation` when the plan's effective
    /// `require_evaluator` is true but the `[evaluator]` section is missing.
    /// The message names the trigger (task count or security-sensitive slug)
    /// and points at the two ways to resolve it: add an `[evaluator]` block
    /// or opt out with `require_evaluator = false`.
    pub fn validate_gates_and_evaluator(&self, resolved: &[ResolvedTask]) -> Result<()> {
        let required = self.effective_require_evaluator(resolved);
        if required && self.evaluator.is_none() {
            let reason = if resolved.len() >= 4 {
                format!(
                    "this plan resolves to {} tasks (threshold: 4)",
                    resolved.len()
                )
            } else {
                let sensitive: Vec<&str> = resolved
                    .iter()
                    .filter(|t| has_security_sensitive_slug(&t.slug))
                    .map(|t| t.slug.as_str())
                    .collect();
                format!(
                    "this plan has security-sensitive task slug(s): {}",
                    sensitive.join(", ")
                )
            };
            return Err(WiggumError::Validation(format!(
                "evaluator required: {reason}, but no [evaluator] section is configured. \
                 Either add an [evaluator] block to the plan, or set `require_evaluator = false` \
                 in [orchestrator] to opt out (wiggum generate will print a warning)."
            )));
        }
        Ok(())
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
                    evaluation_criteria: task.evaluation_criteria.clone(),
                    phase_name: phase.name.clone(),
                    phase_order: phase.order,
                    kind: task.kind,
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

        // Get language profile for integration audit patterns
        let profile = self.project.language.profile();

        // Capture explicit (user-defined) task count before any auto-injection.
        // This is used to decide whether to inject integration audit tasks.
        let explicit_task_count = resolved.len();

        // Auto-inject a security-hardening task when the plan has
        // web-facing surface (detected from task slugs/titles).
        if !self.security.skip_hardening_task
            && !resolved.iter().any(|t| t.slug == "security-hardening")
            && has_web_surface(&resolved)
        {
            let last_slug = resolved.last().map(|t| t.slug.clone());
            let last_phase = resolved.last().map_or_else(
                || ("Security".to_string(), 999),
                |t| (t.phase_name.clone(), t.phase_order),
            );

            resolved.push(security_hardening_task(
                number,
                last_slug,
                last_phase.0,
                last_phase.1,
            ));
            number += 1;
        }

        // Auto-inject integration wiring audit when the plan has enough complexity.
        // This catches the common AI failure mode of creating modules that compile
        // but aren't actually wired into the application.
        if !self.integration.skip_wiring_audit
            && !resolved.iter().any(|t| t.slug == "integration-wiring")
            && needs_integration_audit(explicit_task_count)
        {
            let last_slug = resolved.last().map(|t| t.slug.clone());
            let last_phase = resolved.last().map_or_else(
                || ("Integration".to_string(), 999),
                |t| (t.phase_name.clone(), t.phase_order),
            );

            resolved.push(integration_wiring_task(
                number,
                last_slug,
                last_phase.0,
                last_phase.1,
                profile.wiring_hints,
            ));
            number += 1;
        }

        // Auto-inject stub cleanup audit to find and fix placeholder implementations.
        // This catches the common AI failure mode of leaving todo!() / NotImplementedError
        // stubs that compile but crash at runtime.
        if !self.integration.skip_stub_audit
            && !resolved.iter().any(|t| t.slug == "stub-cleanup")
            && needs_integration_audit(explicit_task_count)
        {
            let last_slug = resolved.last().map(|t| t.slug.clone());
            let last_phase = resolved.last().map_or_else(
                || ("Integration".to_string(), 999),
                |t| (t.phase_name.clone(), t.phase_order),
            );

            resolved.push(stub_cleanup_task(
                number,
                last_slug,
                last_phase.0,
                last_phase.1,
                profile.stub_patterns,
            ));
            number += 1;
            let _ = number; // silence unused warning; keeps numbering correct for future auto-injections
        }

        Ok(resolved)
    }
}

// ─── Security helpers ────────────────────────────────────────────────────────

/// Keywords in task slugs or titles that suggest the plan has web-facing surface.
const WEB_SURFACE_KEYWORDS: &[&str] = &[
    "http",
    "api",
    "server",
    "router",
    "route",
    "endpoint",
    "handler",
    "webhook",
    "upload",
    "auth",
    "login",
    "session",
    "request",
    "response",
    "middleware",
    "web",
    "rest",
    "grpc",
    "graphql",
];

/// Returns `true` if any resolved task looks like it introduces web-facing code.
fn has_web_surface(tasks: &[ResolvedTask]) -> bool {
    tasks.iter().any(|t| {
        let haystack = format!("{} {}", t.slug, t.title).to_lowercase();
        WEB_SURFACE_KEYWORDS.iter().any(|kw| haystack.contains(kw))
    })
}

/// Keywords in task slugs that flag the plan as security-sensitive.
///
/// Matching is a case-insensitive substring test against the slug. If any
/// resolved task slug contains one of these words, the plan auto-derives
/// `require_evaluator = true` so a QA evaluator is wired in.
const SECURITY_SENSITIVE_KEYWORDS: &[&str] = &[
    "auth",
    "payment",
    "billing",
    "crypto",
    "credential",
    "webhook",
    "secret",
    "key",
    "token",
    "sign",
    "signature",
    "kdf",
    "hash",
];

/// Returns `true` if `slug` contains any security-sensitive keyword
/// (case-insensitive substring match).
fn has_security_sensitive_slug(slug: &str) -> bool {
    let lower = slug.to_lowercase();
    SECURITY_SENSITIVE_KEYWORDS
        .iter()
        .any(|kw| lower.contains(kw))
}

/// Auto-derive `require_evaluator` from the resolved task list.
///
/// Returns `true` when the plan has ≥4 tasks or any task slug contains a
/// security-sensitive keyword (`auth`, `payment`, `billing`, `crypto`,
/// `credential`, `webhook`, `secret`, `key`, `token`, `sign`, `signature`,
/// `kdf`, `hash`).
///
/// Used by [`Plan::effective_require_evaluator`] when the plan does not
/// explicitly set `require_evaluator` in the TOML.
#[must_use]
pub fn auto_derive_require_evaluator(resolved: &[ResolvedTask]) -> bool {
    resolved.len() >= 4
        || resolved
            .iter()
            .any(|t| has_security_sensitive_slug(&t.slug))
}

/// Keywords that flag a task as requiring an explicit human gate.
///
/// When matched in a task slug or title, the task must declare a matching
/// `gate = "<category>"` in its `TaskDef`. Mirrors
/// [`SECURITY_SENSITIVE_KEYWORDS`] but is its own constant so gate
/// semantics can evolve independently of the evaluator auto-derive rule.
pub const GATE_KEYWORDS: &[&str] = &[
    "auth",
    "payment",
    "billing",
    "crypto",
    "credential",
    "webhook",
    "secret",
    "key",
    "token",
    "sign",
    "signature",
    "kdf",
    "hash",
];

/// Auto-derive the orchestrator's gate list from the plan's task slugs
/// and titles.
///
/// - When `explicit_gates` is non-empty, it is returned verbatim
///   (the user has chosen which categories are gates).
/// - When `explicit_gates` is empty, the function scans every task's
///   slug and title (case-insensitive substring match) and returns the
///   deduplicated, alphabetically-sorted set of matched keywords.
///
/// The return value replaces `plan.orchestrator.gates` after parsing so
/// downstream validation and rendering see the resolved list.
#[must_use]
pub fn auto_derive_gates(explicit_gates: &[String], phases: &[Phase]) -> Vec<String> {
    if !explicit_gates.is_empty() {
        return explicit_gates.to_vec();
    }

    let mut matched: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for phase in phases {
        for task in &phase.tasks {
            let haystack = format!("{} {}", task.slug, task.title).to_lowercase();
            for kw in GATE_KEYWORDS {
                if haystack.contains(kw) {
                    matched.insert((*kw).to_string());
                }
            }
        }
    }
    matched.into_iter().collect()
}

/// Validate gate declarations against the resolved task list.
///
/// Returns `WiggumError::Validation` listing every task whose slug or
/// title matches a gate category but does not declare a matching
/// `gate = "<category>"` field. Returns `Ok(())` when no gate categories
/// are configured, or when every gated task carries a matching
/// declaration.
///
/// Note: this is distinct from `validate_gates_and_evaluator` which
/// checks the evaluator policy. Gate validation is independent and may
/// be run even when evaluator validation is opted out.
///
/// # Errors
///
/// Returns `WiggumError::Validation` with one line per offending task.
pub fn validate_gates(plan: &Plan, resolved: &[ResolvedTask]) -> Result<()> {
    let gates = &plan.orchestrator.gates;
    if gates.is_empty() {
        return Ok(());
    }

    let gates_lower: Vec<String> = gates.iter().map(|g| g.to_lowercase()).collect();
    let mut offenders: Vec<String> = Vec::new();

    for task in resolved {
        let haystack = format!("{} {}", task.slug, task.title).to_lowercase();
        let matched: Vec<&str> = gates_lower
            .iter()
            .filter(|kw| haystack.contains(kw.as_str()))
            .map(String::as_str)
            .collect();
        if matched.is_empty() {
            continue;
        }
        let declared = task.gate.as_deref().map(str::to_lowercase);
        let has_match = declared
            .as_ref()
            .is_some_and(|d| matched.iter().any(|m| m == &d.as_str()));
        if !has_match {
            offenders.push(format!(
                "T{:02}-{}: slug/title matches gate(s) {:?} but no matching `gate = \"...\"` declared",
                task.number, task.slug, matched
            ));
        }
    }

    if !offenders.is_empty() {
        let listed = match gates.first() {
            Some(single) if gates.len() == 1 => format!("\"{single}\""),
            _ => format!("[{}]", gates.join(", ")),
        };
        return Err(WiggumError::Validation(format!(
            "gate coverage: {} task(s) require an explicit `gate` declaration matching one of {listed}:\n  - {}",
            offenders.len(),
            offenders.join("\n  - ")
        )));
    }

    Ok(())
}

/// Build the auto-injected security hardening task.
fn security_hardening_task(
    number: u32,
    last_slug: Option<String>,
    phase_name: String,
    phase_order: u32,
) -> ResolvedTask {
    let depends_on = last_slug.map(|s| vec![s]).unwrap_or_default();

    ResolvedTask {
        number,
        slug: "security-hardening".to_string(),
        title: "Security hardening and vulnerability review".to_string(),
        goal: "Verify and enforce the six OWASP baseline security properties \
               across the entire codebase before declaring the project complete."
            .to_string(),
        depends_on,
        hints: vec![
            "Audit all source files for hardcoded credentials, API keys, or secrets. \
             Everything sensitive must be read from environment variables or a secrets manager."
                .to_string(),
            "Verify every SQL query uses parameterised inputs — grep for string interpolation \
             into query strings and replace any found."
                .to_string(),
            "Confirm HTTP security headers are set on all responses: \
             Content-Security-Policy, Strict-Transport-Security, X-Frame-Options, \
             X-Content-Type-Options."
                .to_string(),
            "Verify rate-limiting middleware is applied to the router/mux, not just defined — \
             write a smoke test that sends >N requests and asserts 429."
                .to_string(),
            "Inspect every file-upload handler: validate MIME type server-side, \
             block executable extensions, enforce a maximum file size."
                .to_string(),
            "Find every place the code fetches a URL on behalf of a user. \
             Confirm the target is validated against an explicit allowlist \
             (SSRF prevention)."
                .to_string(),
        ],
        test_hints: vec![
            "Rate-limiting smoke test: send N+1 requests to a rate-limited endpoint and assert \
             the final response is HTTP 429."
                .to_string(),
            "Upload rejection test: submit a file with an executable extension and assert the \
             server returns an error, not a successful upload."
                .to_string(),
            "SSRF test: attempt to fetch an internal metadata URL (e.g. 169.254.169.254) \
             and assert the server rejects it."
                .to_string(),
        ],
        must_haves: vec![
            "No hardcoded secrets in any source file".to_string(),
            "All database queries use parameterised inputs".to_string(),
            "HTTP security headers present on all responses".to_string(),
            "Rate-limiting middleware wired to router and verified by test".to_string(),
            "File upload handler validates MIME type and rejects executable extensions".to_string(),
            "URL-fetching code validates target against an allowlist".to_string(),
        ],
        gate: None,
        evaluation_criteria: vec![
            "No secrets found by grep -r for hardcoded keys, passwords, or tokens".to_string(),
            "Security response headers verified by an integration test or curl assertion"
                .to_string(),
            "Rate-limit test sends N+1 requests and receives HTTP 429".to_string(),
            "File upload test rejects .exe/.sh/.php and oversized files".to_string(),
            "SSRF test confirms internal metadata URLs are blocked".to_string(),
        ],
        phase_name,
        phase_order,
        kind: TaskKind::Audit,
    }
}

// ─── Integration audit helpers ───────────────────────────────────────────────

/// Build the auto-injected integration wiring audit task.
/// This task verifies all components are properly connected and wired together.
fn integration_wiring_task(
    number: u32,
    last_slug: Option<String>,
    phase_name: String,
    phase_order: u32,
    wiring_hints: &[&str],
) -> ResolvedTask {
    let depends_on = last_slug.map(|s| vec![s]).unwrap_or_default();

    let hints: Vec<String> = wiring_hints.iter().map(|s| (*s).to_string()).collect();

    ResolvedTask {
        number,
        slug: "integration-wiring".to_string(),
        title: "Integration wiring audit".to_string(),
        goal: "Verify all components are properly connected and wired together. \
               AI-generated code often creates modules that compile but aren't actually \
               integrated into the application — this task catches those gaps."
            .to_string(),
        depends_on,
        hints,
        test_hints: vec![
            "Write an integration test that exercises the full request/response path from \
             entry point to exit."
                .to_string(),
            "For each major feature, trace the call chain from the public API to the \
             underlying implementation and verify nothing is disconnected."
                .to_string(),
        ],
        must_haves: vec![
            "All public exports from library modules are imported and used somewhere".to_string(),
            "All route handlers/controllers are registered with the router/framework".to_string(),
            "All service/repository interfaces have implementations that are instantiated"
                .to_string(),
            "All background tasks/workers are spawned in the application startup".to_string(),
            "All middleware/interceptors are mounted on the request pipeline".to_string(),
            "Configuration values are read and passed to components that need them".to_string(),
        ],
        gate: None,
        evaluation_criteria: vec![
            "No dead code: every public function/type is reachable from main or tests".to_string(),
            "Integration test passes exercising the primary user flow end-to-end".to_string(),
            "Manual trace confirms each feature's wiring from entry to implementation".to_string(),
        ],
        phase_name,
        phase_order,
        kind: TaskKind::Audit,
    }
}

/// Build the auto-injected stub cleanup audit task.
/// This task finds and replaces all placeholder/stub implementations.
fn stub_cleanup_task(
    number: u32,
    last_slug: Option<String>,
    phase_name: String,
    phase_order: u32,
    stub_patterns: &[&str],
) -> ResolvedTask {
    let depends_on = last_slug.map(|s| vec![s]).unwrap_or_default();

    let pattern_hints: Vec<String> = stub_patterns
        .iter()
        .map(|p| format!("Search for: `{p}`"))
        .collect();

    let mut hints = vec![
        "Run a grep search for each stub pattern across the entire codebase.".to_string(),
        "For each match, either implement the functionality or remove the dead code.".to_string(),
        "Resolve or remove TODO/FIXME comments in `src/` before completing this task.".to_string(),
    ];
    hints.extend(pattern_hints);

    ResolvedTask {
        number,
        slug: "stub-cleanup".to_string(),
        title: "Stub and placeholder cleanup".to_string(),
        goal: "Find and replace all stub implementations, placeholder code, TODO markers, \
               and unimplemented functions. AI-generated code frequently leaves behind \
               placeholder implementations that compile but don't actually work."
            .to_string(),
        depends_on,
        hints,
        test_hints: vec![
            "After cleanup, run the full test suite to confirm no test was relying on \
             stub behavior."
                .to_string(),
            "Add tests for any functions that were previously stubbed but are now implemented."
                .to_string(),
        ],
        must_haves: vec![
            "No todo!() / unimplemented!() / NotImplementedError remaining in production code"
                .to_string(),
            "No functions that just return default/dummy values as placeholders".to_string(),
            "No TODO/FIXME comments for work that should have been done in earlier tasks"
                .to_string(),
            "All code paths are reachable and functional".to_string(),
        ],
        gate: None,
        evaluation_criteria: vec![
            "grep for stub patterns returns zero matches in src/ (excluding tests)".to_string(),
            "All previously-stubbed functions now have real implementations with tests".to_string(),
            "Test suite passes with full coverage of formerly-stubbed code paths".to_string(),
        ],
        phase_name,
        phase_order,
        kind: TaskKind::Audit,
    }
}

/// Returns `true` if the plan warrants integration audits.
/// Triggered when there are 3+ explicit (user-defined) tasks.
const fn needs_integration_audit(explicit_task_count: usize) -> bool {
    explicit_task_count >= 3
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod evaluator_policy_tests {
    use super::*;
    use std::fmt::Write as _;

    /// Build a minimal plan TOML with `n` tasks and an optional
    /// `require_evaluator` override, plus an optional `[evaluator]` section.
    ///
    /// Disables the auto-injected `security-hardening` and
    /// `integration-wiring` tasks so the resolved task count equals the
    /// number of slugs passed in — tests assert on exact counts.
    fn build_plan(
        task_slugs: &[&str],
        require_evaluator: Option<&str>,
        with_evaluator: bool,
    ) -> String {
        let mut plan = String::from(
            r#"
[project]
name = "test"
description = "test"
language = "rust"
path = "/tmp/test"

[orchestrator]
"#,
        );
        if let Some(req) = require_evaluator {
            let _ = writeln!(plan, "require_evaluator = {req}");
        }

        if with_evaluator {
            plan.push_str(
                r#"
[evaluator]
persona = "QA"
pass_threshold = 7
hard_fail = false
mode = "blocking"
"#,
            );
        }

        plan.push_str(
            r"
[security]
skip_hardening_task = true

[integration]
skip_wiring_audit = true
skip_stub_audit = true
",
        );

        plan.push_str("\n[[phases]]\nname = \"Phase 1\"\norder = 1\n");
        for slug in task_slugs {
            let _ = write!(
                plan,
                r#"
[[phases.tasks]]
slug = "{slug}"
title = "{slug}"
goal = "Implement {slug} with proper tests"
depends_on = []
"#
            );
        }
        plan
    }

    /// Helper: parse the plan and run the evaluator/gates validator explicitly.
    /// `Plan::from_toml` does NOT auto-validate the evaluator policy (so that
    /// unit tests and integration tests can load sample plans without needing
    /// to add `[evaluator]` blocks), but the production path in
    /// `cmd_generate` does call this helper. Tests assert on the error
    /// produced by `validate_gates_and_evaluator`.
    fn parse_and_validate_or_err(toml: &str, context: &str) -> WiggumError {
        let plan = match Plan::from_toml(toml) {
            Ok(p) => p,
            Err(e) => panic!("{context}: parse failed: {e}"),
        };
        let resolved = match plan.resolve_tasks() {
            Ok(r) => r,
            Err(e) => panic!("{context}: resolve failed: {e}"),
        };
        match plan.validate_gates_and_evaluator(&resolved) {
            Err(e) => e,
            Ok(()) => panic!("{context}: expected validator Err, got Ok"),
        }
    }

    /// Helper: panic with the given message if `result` is not an Ok.
    fn unwrap_ok_or_panic<T>(result: Result<T>, context: &str) -> T {
        match result {
            Ok(v) => v,
            Err(e) => panic!("{context}: expected Ok, got Err({e})"),
        }
    }

    #[test]
    fn five_tasks_no_evaluator_require_true_errors() {
        let slugs = ["alpha", "beta", "gamma", "delta", "epsilon"];
        let toml = build_plan(&slugs, Some("true"), false);
        let err = parse_and_validate_or_err(&toml, "should error when required but missing");
        let msg = err.to_string();
        assert!(
            msg.contains("evaluator required"),
            "error should mention evaluator required: {msg}"
        );
        assert!(
            msg.contains("require_evaluator"),
            "error should mention the fix (require_evaluator): {msg}"
        );
    }

    #[test]
    fn five_tasks_no_evaluator_require_false_validates_and_warns() {
        let slugs = ["alpha", "beta", "gamma", "delta", "epsilon"];
        let toml = build_plan(&slugs, Some("false"), false);
        let plan = unwrap_ok_or_panic(Plan::from_toml(&toml), "validate should accept opt-out");
        assert!(plan.evaluator.is_none());
        let resolved = unwrap_ok_or_panic(plan.resolve_tasks(), "resolve tasks");
        assert!(!plan.effective_require_evaluator(&resolved));
    }

    #[test]
    fn three_tasks_no_evaluator_no_require_does_not_require() {
        let slugs = ["alpha", "beta", "gamma"];
        let toml = build_plan(&slugs, None, false);
        let plan = unwrap_ok_or_panic(Plan::from_toml(&toml), "non-sensitive plan should pass");
        let resolved = unwrap_ok_or_panic(plan.resolve_tasks(), "resolve tasks");
        assert!(!auto_derive_require_evaluator(&resolved));
        assert!(!plan.effective_require_evaluator(&resolved));
    }

    #[test]
    fn explicit_require_true_with_evaluator_validates() {
        let slugs = ["alpha", "beta", "gamma", "delta", "epsilon"];
        let toml = build_plan(&slugs, Some("true"), true);
        let plan = unwrap_ok_or_panic(
            Plan::from_toml(&toml),
            "explicit true with evaluator should pass",
        );
        assert!(plan.evaluator.is_some());
        let resolved = unwrap_ok_or_panic(plan.resolve_tasks(), "resolve tasks");
        assert!(plan.effective_require_evaluator(&resolved));
    }

    #[test]
    fn auto_derive_keyword_matching_is_case_insensitive() {
        let slugs = ["Payment-Processor"];
        let toml = build_plan(&slugs, None, false);
        let err = parse_and_validate_or_err(&toml, "PAYMENT slug should match regardless of case");
        assert!(err.to_string().contains("evaluator required"));
    }

    #[test]
    fn auto_derive_recognises_all_listed_keywords() {
        let cases = [
            "auth-login",
            "stripe-payment",
            "billing-export",
            "crypto-signer",
            "credential-store",
            "inbound-webhook",
            "secret-rotation",
            "key-derivation",
            "token-refresh",
            "sign-verify",
            "signature-validation",
            "argon2-kdf",
            "checksum-hash",
        ];
        for slug in cases {
            let toml = build_plan(&[slug], None, false);
            let err = parse_and_validate_or_err(
                &toml,
                &format!("slug `{slug}` should auto-derive require_evaluator=true"),
            );
            assert!(
                err.to_string().contains("evaluator required"),
                "slug `{slug}` error should mention evaluator required"
            );
        }
    }

    #[test]
    fn four_tasks_hit_count_threshold_without_keywords() {
        // 4 tasks (threshold edge), no security-sensitive slugs → derived true.
        let slugs = ["alpha", "beta", "gamma", "delta"];
        let toml = build_plan(&slugs, None, false);
        let err = parse_and_validate_or_err(&toml, "4 tasks should trigger threshold");
        assert!(err.to_string().contains("threshold: 4"));
    }

    #[test]
    fn five_tasks_explicit_override_accepts_plan() {
        let slugs = ["alpha", "beta", "gamma", "delta", "epsilon"];
        let toml = build_plan(&slugs, Some("false"), false);
        let plan = unwrap_ok_or_panic(Plan::from_toml(&toml), "explicit override should accept");
        assert!(plan.evaluator.is_none());
        let resolved = unwrap_ok_or_panic(plan.resolve_tasks(), "resolve");
        assert!(!plan.effective_require_evaluator(&resolved));
        assert!(auto_derive_require_evaluator(&resolved));
    }

    #[test]
    fn two_tasks_no_evaluator_no_explicit_require_no_error() {
        let slugs = ["alpha", "beta"];
        let toml = build_plan(&slugs, None, false);
        let plan = unwrap_ok_or_panic(
            Plan::from_toml(&toml),
            "two-task plan should validate without evaluator",
        );
        let resolved = unwrap_ok_or_panic(plan.resolve_tasks(), "resolve");
        assert!(!plan.effective_require_evaluator(&resolved));
        assert!(!auto_derive_require_evaluator(&resolved));
    }

    #[test]
    fn one_task_auth_slug_auto_derives_true_and_errors() {
        let slugs = ["auth-handler"];
        let toml = build_plan(&slugs, None, false);
        let err = parse_and_validate_or_err(&toml, "auth-handler should auto-derive required");
        let msg = err.to_string();
        assert!(msg.contains("evaluator required"), "msg: {msg}");
        assert!(
            msg.contains("auth-handler"),
            "error should name the sensitive slug: {msg}"
        );
    }

    #[test]
    fn one_task_unrelated_slug_auto_derives_false() {
        let slugs = ["data-loader"];
        let toml = build_plan(&slugs, None, false);
        let plan = unwrap_ok_or_panic(
            Plan::from_toml(&toml),
            "non-sensitive slug should not require evaluator",
        );
        let resolved = unwrap_ok_or_panic(plan.resolve_tasks(), "resolve");
        assert!(!auto_derive_require_evaluator(&resolved));
        assert!(!plan.effective_require_evaluator(&resolved));
    }

    // ── T03: gate auto-derive + validation ───────────────────────────

    /// Build a plan TOML where the [orchestrator] section can include a
    /// custom `gates = [...]` override and individual tasks can declare
    /// `gate = "..."`. Mirrors `build_plan` but exposes the extra knobs.
    fn build_plan_with_gates(
        task_specs: &[(&str, Option<&str>)], // (slug, optional gate declaration)
        explicit_gates: Option<&[&str]>,
    ) -> String {
        use std::fmt::Write as _;
        let mut plan = String::from(
            r#"
[project]
name = "test"
description = "test"
language = "rust"
path = "/tmp/test"

[orchestrator]
"#,
        );

        if let Some(gates) = explicit_gates {
            let _ = writeln!(
                plan,
                "gates = [{}]",
                gates
                    .iter()
                    .map(|g| format!("\"{g}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        plan.push_str(
            r"
[security]
skip_hardening_task = true

[integration]
skip_wiring_audit = true
skip_stub_audit = true
",
        );

        plan.push_str("\n[[phases]]\nname = \"Phase 1\"\norder = 1\n");
        for (slug, gate) in task_specs {
            let _ = write!(
                plan,
                r#"
[[phases.tasks]]
slug = "{slug}"
title = "{slug}"
goal = "Implement {slug} with proper tests"
depends_on = []
"#,
            );
            if let Some(g) = gate {
                let _ = writeln!(plan, "gate = \"{g}\"");
            }
        }
        plan
    }

    #[test]
    fn gates_auto_derive_when_unset_and_slug_matches() {
        let toml = build_plan_with_gates(&[("auth-handler", None)], None);
        let plan = unwrap_ok_or_panic(Plan::from_toml(&toml), "parse plan");
        assert!(
            plan.orchestrator.gates.iter().any(|g| g == "auth"),
            "auto-derive should populate `auth` for slug auth-handler, got {:?}",
            plan.orchestrator.gates
        );
    }

    #[test]
    fn gates_explicit_custom_requires_declaration() {
        // Explicit gates = ["custom"] + slug auth-handler (which matches no
        // listed gate) but no `gate = "..."` on the task. Since the slug
        // matches neither "custom" nor any default gate keyword, this should
        // actually pass — but if we instead pick a slug that matches the
        // explicit gate, it errors.
        let toml = build_plan_with_gates(&[("auth-handler", None)], Some(&["auth"]));
        let plan = unwrap_ok_or_panic(Plan::from_toml(&toml), "parse plan");
        assert_eq!(plan.orchestrator.gates, vec!["auth".to_string()]);
        let resolved = unwrap_ok_or_panic(plan.resolve_tasks(), "resolve");
        let Err(err) = validate_gates(&plan, &resolved) else {
            panic!("validate_gates should error: auth-handler matches gate 'auth' but declares no gate");
        };
        let msg = err.to_string();
        assert!(msg.contains("gate coverage"), "msg: {msg}");
        assert!(msg.contains("auth-handler"), "msg: {msg}");
        assert!(msg.contains("auth"), "msg: {msg}");
    }

    #[test]
    fn gates_explicit_with_matching_declaration_passes() {
        let toml = build_plan_with_gates(&[("auth-handler", Some("auth"))], Some(&["auth"]));
        let plan = unwrap_ok_or_panic(Plan::from_toml(&toml), "parse plan");
        let resolved = unwrap_ok_or_panic(plan.resolve_tasks(), "resolve");
        if let Err(e) = validate_gates(&plan, &resolved) {
            panic!("validate_gates should pass when gate is declared: {e}");
        }
    }

    #[test]
    fn gates_non_matching_slug_with_empty_gates_auto_derives_but_no_validation_error() {
        // 'fetch-events' doesn't match any keyword → auto-derive produces
        // an empty gates list → validate_gates is a no-op.
        let toml = build_plan_with_gates(&[("fetch-events", None)], None);
        let plan = unwrap_ok_or_panic(Plan::from_toml(&toml), "parse plan");
        assert!(
            plan.orchestrator.gates.is_empty(),
            "fetch-events should not trigger gate auto-derive, got {:?}",
            plan.orchestrator.gates
        );
        let resolved = unwrap_ok_or_panic(plan.resolve_tasks(), "resolve");
        if let Err(e) = validate_gates(&plan, &resolved) {
            panic!("validate_gates should be a no-op with empty gates: {e}");
        }
    }
}
