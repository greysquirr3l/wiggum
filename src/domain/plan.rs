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
    #[serde(default)]
    pub evaluator: Option<EvaluatorConfig>,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub integration: IntegrationConfig,
    #[serde(default)]
    pub style: StyleConfig,
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
    /// Supply-chain / vulnerability audit command appended after lint.
    /// Defaults to the language profile's `audit_cmd`.
    /// Set to an empty string to disable auditing for this plan.
    #[serde(default)]
    pub audit: Option<String>,
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
    /// Evaluator exit criteria — each item is a verifiable condition that
    /// must pass before the task can be marked complete.
    #[serde(default)]
    pub evaluation_criteria: Vec<String>,
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
}

const fn default_avoid_ai_patterns() -> bool {
    true
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            avoid_ai_patterns: default_avoid_ai_patterns(),
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
                    evaluation_criteria: task.evaluation_criteria.clone(),
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
    }
}

/// Returns `true` if the plan warrants integration audits.
/// Triggered when there are 3+ explicit (user-defined) tasks.
const fn needs_integration_audit(explicit_task_count: usize) -> bool {
    explicit_task_count >= 3
}
