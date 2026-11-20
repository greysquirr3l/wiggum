#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::PathBuf;
use tempfile::TempDir;

use wiggum::adapters::fs::FsAdapter;
use wiggum::adapters::retro::{RetroSuggestion, RetroSummary, Severity};
use wiggum::adapters::{patterns, replan};
use wiggum::domain::check::score_plan;
use wiggum::domain::dag::validate_dag;
use wiggum::domain::plan::Plan;
use wiggum::domain::targets::TargetSet;
use wiggum::generation;
use wiggum::ports::PlanReader;

fn load_example_plan() -> Plan {
    let fs = FsAdapter;
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/example-plan.toml");
    let toml_content = fs.read_plan(&fixture).expect("Failed to read fixture");
    Plan::from_toml(&toml_content).expect("Failed to parse plan")
}

#[test]
fn parse_example_plan() {
    let plan = load_example_plan();
    assert_eq!(plan.project.name, "example-project");
    assert_eq!(plan.phases.len(), 3);
    assert_eq!(plan.preflight.build, "cargo build --workspace");
}

#[test]
fn resolve_tasks_numbering() {
    let plan = load_example_plan();
    let resolved = plan.resolve_tasks().expect("Failed to resolve tasks");
    // 5 explicit tasks + 3 auto-injected tasks:
    // - security-hardening (http-client triggers web surface detection)
    // - integration-wiring (3+ tasks triggers complexity threshold)
    // - stub-cleanup (3+ tasks triggers complexity threshold)
    assert_eq!(resolved.len(), 8);
    assert_eq!(resolved[0].number, 1);
    assert_eq!(resolved[0].slug, "workspace-scaffold");
    assert_eq!(resolved[4].number, 5);
    assert_eq!(resolved[4].slug, "persistence");
    assert_eq!(resolved[5].slug, "security-hardening");
    assert_eq!(resolved[6].slug, "integration-wiring");
    assert_eq!(resolved[7].slug, "stub-cleanup");
}

#[test]
fn validate_example_dag() {
    let plan = load_example_plan();
    let resolved = plan.resolve_tasks().unwrap();
    let sorted = validate_dag(&resolved).expect("DAG should be valid");
    // workspace-scaffold must come first
    assert_eq!(sorted[0], "workspace-scaffold");
}

#[test]
fn generate_all_artifacts() {
    let plan = load_example_plan();
    let artifacts = generation::generate_all(&plan).expect("Generation failed");

    // Check PROGRESS.md
    assert!(artifacts.progress.contains("example-project"));
    assert!(artifacts.progress.contains("Phase 1"));
    assert!(artifacts.progress.contains("Workspace Scaffold"));
    assert!(artifacts.progress.contains("`[ ]`"));

    // Check VSCode orchestrator
    assert!(artifacts.orchestrator_vscode.contains("example-project"));
    assert!(artifacts.orchestrator_vscode.contains("runSubagent"));

    // Check opencode orchestrator (single-file pattern: embeds SUBAGENT_PROMPT)
    assert!(artifacts.orchestrator_opencode.contains("example-project"));
    assert!(artifacts.orchestrator_opencode.contains("subagent_type"));
    assert!(!artifacts.orchestrator_opencode.contains("runSubagent"));
    assert!(
        artifacts
            .orchestrator_opencode
            .contains("<SUBAGENT_PROMPT>"),
        "opencode orchestrator must embed SUBAGENT_PROMPT"
    );

    // Check ORCHESTRATOR.md root workflow doc
    assert!(artifacts.orchestrator_root.starts_with("# ORCHESTRATOR"));
    assert!(artifacts.orchestrator_root.contains("Task state machine"));

    // Check plan doc
    assert!(artifacts.plan_doc.contains("example-project"));
    assert!(artifacts.plan_doc.contains("hexagonal"));

    // Check task files (5 explicit + 3 auto-injected: security-hardening, integration-wiring, stub-cleanup)
    assert_eq!(artifacts.tasks.len(), 8);
    let (filename, content) = &artifacts.tasks[0];
    assert_eq!(filename, "T01-workspace-scaffold.md");
    assert!(content.contains("# T01"));
    assert!(content.contains("Depends on"));
    assert!(content.contains("Preflight"));

    // Task with dependencies
    let (_, content) = &artifacts.tasks[1];
    assert!(content.contains("T02"));
    assert!(content.contains("domain-model"));
}

#[test]
fn write_artifacts_to_disk() {
    let mut plan = load_example_plan();
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let project_path = tmp.path().to_path_buf();
    plan.project.path = project_path.to_string_lossy().to_string();

    let artifacts = generation::generate_all(&plan).expect("Generation failed");
    let fs = FsAdapter;
    let targets = TargetSet::vscode_only();
    generation::write_artifacts(&fs, &project_path, &artifacts, &targets)
        .expect("Failed to write artifacts");

    // Verify files exist
    assert!(project_path.join("PROGRESS.md").exists());
    assert!(project_path.join("IMPLEMENTATION_PLAN.md").exists());
    assert!(project_path.join(".vscode/orchestrator.prompt.md").exists());
    assert!(!project_path.join(".opencode").exists());
    assert!(!project_path.join(".claude").exists());
    assert!(
        project_path
            .join("tasks/T01-workspace-scaffold.md")
            .exists()
    );
    assert!(project_path.join("tasks/T05-persistence.md").exists());
}

#[test]
fn write_artifacts_opencode_target_writes_opencode_agents() {
    let mut plan = load_example_plan();
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let project_path = tmp.path().to_path_buf();
    plan.project.path = project_path.to_string_lossy().to_string();

    let artifacts = generation::generate_all(&plan).expect("Generation failed");
    let fs = FsAdapter;
    let targets = TargetSet {
        vscode: false,
        opencode: true,
        claude: false,
        agent_rules: false,
    };
    generation::write_artifacts(&fs, &project_path, &artifacts, &targets)
        .expect("Failed to write artifacts");

    assert!(project_path.join("PROGRESS.md").exists());
    assert!(project_path.join("AGENTS.md").exists());
    assert!(!project_path.join(".vscode").exists());
    assert!(
        project_path
            .join(".opencode/agents/orchestrator.md")
            .exists()
    );
    assert!(project_path.join(".opencode/agents/planner.md").exists());
    assert!(
        project_path
            .join(".opencode/agents/background-auditor.md")
            .exists()
    );
    assert!(!project_path.join(".opencode/package.json").exists());
    assert!(!project_path.join(".opencode/.gitignore").exists());

    // ORCHESTRATOR.md at the root is a real workflow reference doc.
    let root_orch = project_path.join("ORCHESTRATOR.md");
    assert!(
        root_orch.exists(),
        "opencode target must emit a real ORCHESTRATOR.md (not a symlink)"
    );
    let body = std::fs::read_to_string(&root_orch).expect("read ORCHESTRATOR.md");
    assert!(
        body.starts_with("# ORCHESTRATOR"),
        "ORCHESTRATOR.md must be the workflow reference doc"
    );
}

#[test]
fn write_artifacts_claude_target_writes_hooks_and_claude_md() {
    let mut plan = load_example_plan();
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let project_path = tmp.path().to_path_buf();
    plan.project.path = project_path.to_string_lossy().to_string();

    let artifacts = generation::generate_all(&plan).expect("Generation failed");
    let fs = FsAdapter;
    let targets = TargetSet {
        vscode: false,
        opencode: false,
        claude: true,
        agent_rules: false,
    };
    generation::write_artifacts(&fs, &project_path, &artifacts, &targets)
        .expect("Failed to write artifacts");

    assert!(
        project_path.join(".claude/settings.json").exists(),
        "Claude target must emit the hooks JSON"
    );
    assert!(
        project_path.join("CLAUDE.md").exists(),
        "Claude target must emit CLAUDE.md at the repo root for Claude Code project memory"
    );
    let claude_md =
        std::fs::read_to_string(project_path.join("CLAUDE.md")).expect("read CLAUDE.md");
    assert!(
        claude_md.starts_with("# CLAUDE.md"),
        "CLAUDE.md must start with the `# CLAUDE.md` header so Claude Code recognises it"
    );
    assert!(!project_path.join(".vscode").exists());
    assert!(!project_path.join(".opencode").exists());
    assert!(!project_path.join(".cursorrules").exists());
    assert!(!project_path.join(".windsurfrules").exists());
}

#[test]
fn write_artifacts_agent_rules_target_writes_three_rules_files() {
    let mut plan = load_example_plan();
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let project_path = tmp.path().to_path_buf();
    plan.project.path = project_path.to_string_lossy().to_string();

    let artifacts = generation::generate_all(&plan).expect("Generation failed");
    let fs = FsAdapter;
    let targets = TargetSet {
        vscode: false,
        opencode: false,
        claude: false,
        agent_rules: true,
    };
    generation::write_artifacts(&fs, &project_path, &artifacts, &targets)
        .expect("Failed to write artifacts");

    assert!(
        project_path.join(".cursorrules").exists(),
        "agent-rules target must emit .cursorrules (Cursor)"
    );
    assert!(
        project_path.join(".windsurfrules").exists(),
        "agent-rules target must emit .windsurfrules (Windsurf)"
    );
    assert!(
        project_path
            .join(".github/copilot-instructions.md")
            .exists(),
        "agent-rules target must emit .github/copilot-instructions.md (GitHub Copilot)"
    );
    assert!(
        project_path.join(".github").is_dir(),
        ".github/ directory must exist after writing copilot-instructions.md"
    );
    assert!(
        !project_path.join(".vscode").exists(),
        "agent-rules target must NOT emit VSCode artifacts"
    );
    assert!(
        !project_path.join(".opencode").exists(),
        "agent-rules target must NOT emit opencode artifacts"
    );
    assert!(
        !project_path.join(".claude").exists(),
        "agent-rules target must NOT emit Claude artifacts"
    );
    assert!(
        !project_path.join("CLAUDE.md").exists(),
        "agent-rules target must NOT emit CLAUDE.md (that's part of the `claude` target)"
    );

    // The three rules files should have identical content (one template, three destinations).
    let cursorrules =
        std::fs::read_to_string(project_path.join(".cursorrules")).expect("read .cursorrules");
    let windsurfrules =
        std::fs::read_to_string(project_path.join(".windsurfrules")).expect("read .windsurfrules");
    let copilot_instructions =
        std::fs::read_to_string(project_path.join(".github/copilot-instructions.md"))
            .expect("read copilot-instructions.md");
    assert_eq!(
        cursorrules, windsurfrules,
        ".cursorrules and .windsurfrules must have identical content"
    );
    assert_eq!(
        cursorrules, copilot_instructions,
        ".cursorrules and .github/copilot-instructions.md must have identical content"
    );
    // Sanity: the rules content must reference the project name and language.
    assert!(cursorrules.contains("example-project"));
    assert!(cursorrules.contains("rust"));
    // And it must NOT reference the orchestrator loop — the receiving IDE drives that.
    assert!(
        !cursorrules.contains("runSubagent"),
        "agent rules must not reference runSubagent"
    );
}

#[test]
fn write_artifacts_all_targets_writes_everything() {
    let mut plan = load_example_plan();
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let project_path = tmp.path().to_path_buf();
    plan.project.path = project_path.to_string_lossy().to_string();

    let artifacts = generation::generate_all(&plan).expect("Generation failed");
    let fs = FsAdapter;
    generation::write_artifacts(&fs, &project_path, &artifacts, &TargetSet::all())
        .expect("Failed to write artifacts");

    assert!(project_path.join(".vscode/orchestrator.prompt.md").exists());
    assert!(
        project_path
            .join(".opencode/agents/orchestrator.md")
            .exists()
    );
    assert!(project_path.join(".claude/settings.json").exists());
}

#[test]
fn opencode_orchestrator_does_not_contain_runsubagent() {
    let plan = load_example_plan();
    let artifacts = generation::generate_all(&plan).expect("Generation failed");
    assert!(
        !artifacts.orchestrator_opencode.contains("runSubagent"),
        "opencode orchestrator must not contain runSubagent"
    );
    assert!(
        artifacts.orchestrator_opencode.contains("`task` tool"),
        "opencode orchestrator should reference the `task` tool"
    );
}

#[test]
fn plan_targets_field_overrides_default() {
    let toml = r#"
[project]
name = "opencode-app"
description = "Test opencode target from plan"
language = "rust"
path = "/tmp/opencode-app"

[targets]
vscode = false
opencode = true

[[phases]]
name = "Phase 1"
order = 1

[[phases.tasks]]
slug = "t01"
title = "T01"
goal = "Test goal"
"#;
    let plan = Plan::from_toml(toml).expect("parse plan");
    let resolved = plan.targets.resolve();
    assert!(!resolved.vscode, "vscode should be disabled in plan");
    assert!(resolved.opencode, "opencode should be enabled in plan");
}

#[test]
fn plan_targets_field_empty_resolves_to_vscode_default() {
    let toml = r#"
[project]
name = "default-app"
description = "Test default targets"
language = "rust"
path = "/tmp/default-app"

[[phases]]
name = "Phase 1"
order = 1

[[phases.tasks]]
slug = "t01"
title = "T01"
goal = "Test goal"
"#;
    let plan = Plan::from_toml(toml).expect("parse plan");
    let resolved = plan.targets.resolve();
    // Empty/absent [targets] should yield the back-compat default (vscode only).
    assert!(resolved.vscode);
    assert!(!resolved.opencode);
    assert!(!resolved.claude);
}

#[test]
fn detect_duplicate_slugs() {
    let toml = r#"
[project]
name = "test"
description = "test"
path = "/tmp/test"

[[phases]]
name = "Phase 1"
order = 1

[[phases.tasks]]
slug = "duplicate"
title = "First"
goal = "First task"

[[phases.tasks]]
slug = "duplicate"
title = "Second"
goal = "Second task with same slug"
"#;
    let plan = Plan::from_toml(toml).unwrap();
    let result = plan.resolve_tasks();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("duplicate"), "Expected duplicate error: {err}");
}

#[test]
fn detect_unknown_dependency() {
    let toml = r#"
[project]
name = "test"
description = "test"
path = "/tmp/test"

[[phases]]
name = "Phase 1"
order = 1

[[phases.tasks]]
slug = "task-a"
title = "Task A"
goal = "Do A"
depends_on = ["nonexistent"]
"#;
    let plan = Plan::from_toml(toml).unwrap();
    let result = plan.resolve_tasks();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("nonexistent"),
        "Expected unknown dep error: {err}"
    );
}

#[test]
fn language_defaults_applied() {
    let toml = r#"
[project]
name = "go-project"
description = "A Go project"
language = "go"
path = "/tmp/go-project"

[[phases]]
name = "Setup"
order = 1

[[phases.tasks]]
slug = "setup"
title = "Setup"
goal = "Set up"
"#;
    let plan = Plan::from_toml(toml).unwrap();
    assert_eq!(plan.preflight.build, "go build ./...");
    assert_eq!(plan.preflight.test, "go test -v ./...");
}

#[test]
fn task_hints_appear_in_output() {
    let plan = load_example_plan();
    let artifacts = generation::generate_all(&plan).unwrap();
    let (_, content) = &artifacts.tasks[0]; // workspace-scaffold has hints
    assert!(content.contains("Create a 3-crate workspace"));
    assert!(content.contains("Verify cargo build"));
}

// ─── New artifacts (wiggum 3.0) ──────────────────────────────────────────────

#[test]
fn generate_all_includes_new_3_0_artifacts() {
    let plan = load_example_plan();
    let artifacts = generation::generate_all(&plan).expect("Generation failed");

    assert!(
        !artifacts.hooks_json.is_empty(),
        "hooks.json should be generated"
    );
    assert!(
        !artifacts.planner_vscode.is_empty(),
        "planner prompt should be generated"
    );
    assert!(
        !artifacts.background_auditor_vscode.is_empty(),
        "background auditor prompt should be generated"
    );
    assert!(
        artifacts.hooks_json.contains("PreToolUse") || artifacts.hooks_json.contains("hooks"),
        "hooks.json should contain Claude hook structure"
    );
    assert!(
        artifacts.planner_vscode.contains("planner") || artifacts.planner_vscode.contains("plan"),
        "planner prompt should reference planning"
    );
}

// ─── replan ───────────────────────────────────────────────────────────────────

fn fixture_plan_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/example-plan.toml")
}

#[test]
fn replan_dry_run_succeeds_for_known_slug() {
    let tmp = TempDir::new().expect("temp dir");
    let plan_path = tmp.path().join("plan.toml");
    std::fs::copy(fixture_plan_path(), &plan_path).expect("copy fixture");

    let result = replan::run_replan(&plan_path, "workspace-scaffold", true);
    assert!(result.is_ok(), "dry-run replan failed: {result:?}");
}

#[test]
fn replan_unknown_slug_returns_error() {
    let result = replan::run_replan(&fixture_plan_path(), "no-such-task", true);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("no-such-task"),
        "Error should mention the missing slug: {msg}"
    );
}

#[test]
fn replan_writes_task_file_to_disk() {
    let tmp = TempDir::new().expect("temp dir");
    let plan_path = tmp.path().join("plan.toml");
    std::fs::copy(fixture_plan_path(), &plan_path).expect("copy fixture");
    std::fs::create_dir(tmp.path().join("tasks")).expect("create tasks dir");

    let result = replan::run_replan(&plan_path, "workspace-scaffold", false);
    assert!(result.is_ok(), "replan write failed: {result:?}");
    assert!(
        tmp.path().join("tasks/T01-workspace-scaffold.md").exists(),
        "task file should be written to tasks/"
    );
}

#[test]
fn replan_task_file_contains_slug_and_preflight() {
    let tmp = TempDir::new().expect("temp dir");
    let plan_path = tmp.path().join("plan.toml");
    std::fs::copy(fixture_plan_path(), &plan_path).expect("copy fixture");
    std::fs::create_dir(tmp.path().join("tasks")).expect("create tasks dir");

    replan::run_replan(&plan_path, "workspace-scaffold", false).expect("replan");
    let content =
        std::fs::read_to_string(tmp.path().join("tasks/T01-workspace-scaffold.md")).unwrap();
    assert!(content.contains("workspace-scaffold"));
    assert!(content.contains("Preflight"));
}

// ─── patterns ─────────────────────────────────────────────────────────────────

#[test]
fn patterns_list_empty_for_empty_dir() {
    let tmp = TempDir::new().expect("temp dir");
    let result = patterns::list(tmp.path()).expect("list");
    assert!(result.is_empty());
}

#[test]
fn patterns_list_nonexistent_dir_returns_empty() {
    let tmp = TempDir::new().expect("temp dir");
    let result = patterns::list(&tmp.path().join("does-not-exist")).expect("list");
    assert!(result.is_empty());
}

#[test]
fn patterns_save_from_retro_and_list() {
    let tmp = TempDir::new().expect("temp dir");
    let plan = load_example_plan();

    let summary = RetroSummary {
        project_name: "test-project".to_string(),
        total_tasks: 5,
        completed_tasks: 4,
        retry_count: 1,
        gate_count: 0,
        suggestions: vec![RetroSuggestion {
            pattern: "retries".to_string(),
            suggestion: "Add more test hints to reduce retries".to_string(),
            task_slug: None,
            severity: Severity::Info,
        }],
    };

    let path = patterns::save_from_retro(&summary, &plan, tmp.path()).expect("save pattern");
    assert!(path.exists(), "pattern file should be written to disk");

    let listed = patterns::list(tmp.path()).expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].project_name, "test-project");
    assert_eq!(listed[0].suggestions.len(), 1);
    assert!(listed[0].suggestions[0].contains("test hints"));
}

#[test]
fn patterns_save_multiple_sorted_by_timestamp() {
    use wiggum::adapters::patterns::{Pattern, PatternSource};

    let tmp = TempDir::new().expect("temp dir");
    // Write three pattern files directly with known, distinct timestamps
    // so the test is not sensitive to sub-millisecond clock resolution.
    for (i, ts) in [1_000_000_u64, 2_000_000, 3_000_000].iter().enumerate() {
        let pat = Pattern {
            id: format!("pat-{ts}"),
            project_name: format!("proj-{i}"),
            created_at_unix: *ts,
            language: "rust".to_string(),
            suggestions: vec![],
            source: PatternSource::Retro,
        };
        let content = toml::to_string_pretty(&pat).expect("serialize");
        std::fs::write(tmp.path().join(format!("pat-{ts}.toml")), content).expect("write");
    }

    let listed = patterns::list(tmp.path()).expect("list");
    assert_eq!(listed.len(), 3);
    // list() sorts by created_at_unix ascending
    let timestamps: Vec<u64> = listed.iter().map(|p| p.created_at_unix).collect();
    assert!(
        timestamps.windows(2).all(|w| w[0] <= w[1]),
        "Patterns should be returned sorted by timestamp ascending: {timestamps:?}"
    );
}

// ─── check / score_plan ───────────────────────────────────────────────────────

#[test]
fn check_score_plan_has_seven_dimensions() {
    let plan = load_example_plan();
    let resolved = plan.resolve_tasks().expect("resolve");
    let score = score_plan(&plan, &resolved);
    assert_eq!(
        score.dimensions.len(),
        7,
        "Expected exactly 7 scoring dimensions"
    );
}

#[test]
fn check_estimated_tokens_nonzero_for_example_plan() {
    let plan = load_example_plan();
    let resolved = plan.resolve_tasks().expect("resolve");
    let score = score_plan(&plan, &resolved);
    assert!(score.estimated_tokens > 0, "Token estimate should be > 0");
}

#[test]
fn check_rich_plan_with_evaluator_is_healthy() {
    let toml = r#"
[project]
name = "rich-app"
description = "A well-specified hexagonal service"
language = "rust"
path = "/tmp/rich-app"
architecture = "hexagonal"

[evaluator]
pass_threshold = 7
contract_review = true
criteria = [
    { name = "tests-pass",  weight = 40, description = "All tests pass" },
    { name = "build-clean", weight = 35, description = "Cargo build succeeds" },
    { name = "lint-clean",  weight = 25, description = "Clippy clean" },
]

[security]
skip_hardening_task = true

[integration]
skip_wiring_audit = true

[[phases]]
name = "Foundation"
order = 1

[[phases.tasks]]
slug = "workspace-scaffold"
title = "Cargo workspace + CI"
goal = "Set up workspace with CI, formatting, and linting gates"
hints = ["3-crate workspace: domain/, infra/, api/", "GitHub Actions CI"]
test_hints = ["cargo build --workspace", "cargo test --workspace"]
evaluation_criteria = ["build succeeds", "CI green"]

[[phases.tasks]]
slug = "domain-model"
title = "Domain entities and port traits"
goal = "Define pure domain layer with no I/O dependencies"
depends_on = ["workspace-scaffold"]
hints = ["no std::io in domain crate", "derive-based value objects"]
test_hints = ["domain unit tests pass", "no forbidden imports"]
evaluation_criteria = ["domain compiles without infra deps", "unit tests pass"]

[[phases]]
name = "Application"
order = 2

[[phases.tasks]]
slug = "api-adapter"
title = "HTTP API adapter (axum)"
goal = "Implement REST endpoints wired to domain services"
depends_on = ["domain-model"]
hints = ["axum Router with typed extractors", "error → status-code mapping"]
test_hints = ["integration tests with TestClient", "all routes 200/404/422"]
evaluation_criteria = ["API tests pass", "no panics in handlers"]

[[phases.tasks]]
slug = "persistence-adapter"
title = "SQLite persistence adapter"
goal = "Implement repository traits using sqlx with migrations"
depends_on = ["domain-model"]
hints = ["sqlx::migrate! macro", "named bind params only — no string interpolation"]
test_hints = ["run migrations in test setup", "CRUD round-trip test"]
evaluation_criteria = ["migrations apply cleanly", "round-trip test passes"]
"#;

    let plan = Plan::from_toml(toml).expect("parse plan");
    let resolved = plan.resolve_tasks().expect("resolve");
    let score = score_plan(&plan, &resolved);
    assert!(
        score.is_healthy(),
        "Rich plan should score >= 7, got {}",
        score.overall
    );
}

#[test]
fn check_harness_dimension_max_when_all_tasks_have_criteria() {
    let toml = r#"
[project]
name = "criteria-test"
description = "All tasks have evaluation criteria"
language = "rust"
path = "/tmp/criteria-test"

[evaluator]
pass_threshold = 7
criteria = [
    { name = "tests-pass",  weight = 60, description = "Tests pass" },
    { name = "build-clean", weight = 40, description = "Build clean" },
]

[[phases]]
name = "Phase 1"
order = 1

[[phases.tasks]]
slug = "task-one"
title = "Task One"
goal = "Implement feature one"
evaluation_criteria = ["tests pass", "build clean"]

[[phases.tasks]]
slug = "task-two"
title = "Task Two"
goal = "Implement feature two"
evaluation_criteria = ["tests pass"]

[[phases.tasks]]
slug = "task-three"
title = "Task Three"
goal = "Implement feature three"
evaluation_criteria = ["build clean"]
"#;

    let plan = Plan::from_toml(toml).expect("parse plan");
    let resolved = plan.resolve_tasks().expect("resolve");
    let score = score_plan(&plan, &resolved);

    let harness = score
        .dimensions
        .iter()
        .find(|d| d.name == "Harness complexity")
        .expect("harness dimension must exist");
    // All 3 tasks have criteria (100% ≥ 50% threshold) → bonus → penalty −1 → clamped to 0 → score 10
    assert_eq!(
        harness.score, 10,
        "All tasks with criteria should yield max harness score"
    );
}

// ── T09: --thin flag ────────────────────────────────────────────────────────

/// Load the thin-plan fixture, mutating its project.path to a temp dir so
/// the test can write real artifacts without polluting the fixture file.
fn load_thin_plan_into(project_path: &std::path::Path) -> Plan {
    let fs = FsAdapter;
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/thin-plan.toml");
    let toml_content = fs
        .read_plan(&fixture)
        .expect("Failed to read thin-plan.toml");
    let mut plan = Plan::from_toml(&toml_content).expect("Failed to parse thin-plan.toml");
    plan.project.path = project_path.to_string_lossy().to_string();
    plan
}

#[test]
fn thin_mode_writes_universal_artifacts_only() {
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let project_path = tmp.path().to_path_buf();
    let plan = load_thin_plan_into(&project_path);

    let artifacts = generation::generate_all(&plan).expect("Generation failed");
    let fs = FsAdapter;
    // --thin is modelled by passing TargetSet::none() to write_artifacts.
    generation::write_artifacts(&fs, &project_path, &artifacts, &TargetSet::none())
        .expect("Failed to write artifacts");

    // Universal artifacts present.
    assert!(
        project_path.join("PROGRESS.md").exists(),
        "PROGRESS.md missing"
    );
    assert!(
        project_path.join("IMPLEMENTATION_PLAN.md").exists(),
        "IMPLEMENTATION_PLAN.md missing"
    );
    assert!(project_path.join("BUDGET.md").exists(), "BUDGET.md missing");
    assert!(project_path.join("AGENTS.md").exists(), "AGENTS.md missing");
    assert!(
        project_path.join("features.json").exists(),
        "features.json missing"
    );
    assert!(
        project_path.join("tasks").exists(),
        "tasks/ directory missing"
    );

    // Per-tool artifacts absent.
    assert!(
        !project_path.join(".vscode").exists(),
        ".vscode/ should NOT exist in thin mode"
    );
    assert!(
        !project_path.join(".opencode").exists(),
        ".opencode/ should NOT exist in thin mode"
    );
    assert!(
        !project_path.join(".claude").exists(),
        ".claude/ should NOT exist in thin mode"
    );
}

#[test]
fn thin_mode_target_set_none_equals_no_targets() {
    // Sanity: TargetSet::none() is the canonical empty target set.
    let t = TargetSet::none();
    assert!(!t.vscode);
    assert!(!t.opencode);
    assert!(!t.claude);
    assert!(!t.agent_rules);
    assert!(t.is_empty(), "TargetSet::none() must be empty");
}

#[test]
fn thin_flag_appears_in_cli_help() {
    use assert_cmd::Command;
    let help = Command::cargo_bin("wiggum")
        .unwrap()
        // suppress stderr noise from tracing init
        .env("RUST_LOG", "error")
        .arg("generate")
        .arg("--help")
        .output()
        .expect("Failed to invoke wiggum generate --help");
    let stdout = String::from_utf8_lossy(&help.stdout);
    assert!(
        stdout.contains("--thin"),
        "--thin flag must appear in `wiggum generate --help` output, got:\n{stdout}"
    );
}

#[test]
fn capabilities_render_to_separate_files() {
    let toml = r#"
[project]
name = "kestrel"
path = "/tmp/kestrel"
description = "Webhook relay"
language = "rust"
architecture = "hexagonal"

[[capabilities]]
name = "webhook-reception"
title = "Inbound Webhook Reception"
description = "Accepts and validates inbound webhooks."
[[capabilities.scenarios]]
name = "valid-signature"
when = "POST /webhook receives a valid HMAC signature"
then = "the server returns 202 Accepted"

[[phases]]
name = "Inbound"
order = 1
[[phases.tasks]]
slug = "router"
title = "Router"
goal = "Stand up the inbound router."
implements = ["webhook-reception"]
"#;
    let plan = Plan::from_toml(toml).expect("plan parses");
    let resolved = plan.resolve_tasks().expect("resolves");
    plan.validate_capabilities(&resolved).expect("validates");

    let artifacts = generation::generate_all(&plan).expect("generation");

    // One capability file per capability.
    assert_eq!(artifacts.capabilities.len(), 1);
    let (filename, body) = &artifacts.capabilities[0];
    assert_eq!(filename, "webhook-reception.md");
    assert!(body.contains("Inbound Webhook Reception"));
    assert!(body.contains("Accepts and validates inbound webhooks."));
    assert!(body.contains("**WHEN**"));
    assert!(body.contains("**THEN**"));
    assert!(body.contains("202 Accepted"));

    // IMPLEMENTATION_PLAN.md gains a Capabilities section.
    assert!(artifacts.plan_doc.contains("## Capabilities"));
    assert!(artifacts.plan_doc.contains("webhook-reception.md"));

    // The referencing task file inlines scenarios under ## Implements.
    let router = artifacts
        .tasks
        .iter()
        .find(|(_, body)| body.contains("# T01 — Router"))
        .expect("T01 router task exists");
    assert!(router.1.contains("## Implements"));
    assert!(router.1.contains("valid-signature"));
    assert!(router.1.contains("202 Accepted"));

    // The orchestrator learns about capabilities on Setup.
    assert!(artifacts.orchestrator_vscode.contains("capabilities/"));
    assert!(artifacts.orchestrator_vscode.contains("1 capability"));

    // write_artifacts emits capabilities/<name>.md to disk.
    let tmp = TempDir::new().expect("temp dir");
    let project_path = tmp.path().to_path_buf();
    generation::write_artifacts(
        &FsAdapter,
        &project_path,
        &artifacts,
        &TargetSet::vscode_only(),
    )
    .expect("write");

    let cap_file = project_path
        .join("capabilities")
        .join("webhook-reception.md");
    assert!(cap_file.exists(), "capability file must be written");
    let on_disk = std::fs::read_to_string(&cap_file).expect("read");
    assert!(on_disk.contains("202 Accepted"));
}

#[test]
fn validate_capabilities_rejects_unknown_reference() {
    let toml = r#"
[project]
name = "kestrel"
path = "/tmp/kestrel"
description = "Webhook relay"
language = "rust"

[[phases]]
name = "Inbound"
order = 1
[[phases.tasks]]
slug = "router"
title = "Router"
goal = "Stand up the inbound router."
implements = ["missing-capability"]
"#;
    let plan = Plan::from_toml(toml).expect("plan parses");
    let resolved = plan.resolve_tasks().expect("resolves");
    let err = plan
        .validate_capabilities(&resolved)
        .expect_err("must reject");
    assert!(err.to_string().contains("missing-capability"));
}

#[test]
fn capabilities_omitted_when_plan_has_none() {
    // A plan without `[[capabilities]]` must not create a capabilities/
    // directory and must not reference one from IMPLEMENTATION_PLAN.md.
    let plan = load_example_plan();
    let artifacts = generation::generate_all(&plan).expect("generation");
    assert!(
        artifacts.capabilities.is_empty(),
        "expected no capability artifacts, got {:?}",
        artifacts.capabilities
    );
    assert!(!artifacts.plan_doc.contains("## Capabilities"));

    let tmp = TempDir::new().expect("temp dir");
    let project_path = tmp.path().to_path_buf();
    generation::write_artifacts(
        &FsAdapter,
        &project_path,
        &artifacts,
        &TargetSet::vscode_only(),
    )
    .expect("write");
    assert!(
        !project_path.join("capabilities").exists(),
        "capabilities/ must not be created when plan has no capabilities"
    );
}

// ─── Task anchors ───────────────────────────────────────────────────────────

/// Build a minimal single-task plan, optionally with anchors.
fn anchor_plan(anchors_toml: &str, root: &std::path::Path) -> Plan {
    // Serialise the path through the `toml` crate rather than interpolating
    // it raw: on Windows a temp path contains backslashes, and `\U` is an
    // invalid escape in a TOML basic string.
    let path = toml::Value::String(root.display().to_string()).to_string();
    let toml = format!(
        r#"
[project]
name = "anchor-test"
description = "Anchor fixture."
language = "rust"
path = {path}

[preflight]
build = "cargo build"
test = "cargo test"
lint = "cargo clippy"

[[phases]]
name = "Phase 1"
order = 1

[[phases.tasks]]
slug = "wire-thing"
title = "Wire the thing"
goal = "Connect the thing."
hints = ["Follow the existing wiring."]
test_hints = ["Add a test."]
{anchors_toml}
"#,
    );
    Plan::from_toml(&toml).expect("parse")
}

#[test]
fn anchor_plan_toml_handles_windows_paths() {
    // On Windows a temp path contains backslashes, and `\U` is not a valid
    // escape inside a TOML basic string — so a raw-interpolated path makes
    // the whole plan unparseable. Backslashes are legal filename characters
    // on Unix, so this reproduces the Windows failure on every platform.
    let root = PathBuf::from(r"C:\Users\RUNNER~1\AppData\Local\Temp\.tmpX");
    let plan = anchor_plan("", &root);
    assert_eq!(
        plan.project.path,
        r"C:\Users\RUNNER~1\AppData\Local\Temp\.tmpX"
    );
}

#[test]
fn plan_without_anchors_parses_and_renders_unchanged() {
    // Back-compat: a plan that never mentions anchors must be fully valid and
    // must not gain a Code Anchors section.
    let tmp = TempDir::new().expect("temp dir");
    let plan = anchor_plan("", tmp.path());
    let resolved = plan.resolve_tasks().expect("resolve");
    assert!(resolved[0].anchors.is_empty());

    let diags = wiggum::domain::anchors::lint_anchors(&resolved, tmp.path());
    assert!(
        diags.is_empty(),
        "expected no anchor diagnostics, got {diags:?}"
    );

    let artifacts = generation::generate_all(&plan).expect("generation");
    let task_body = artifacts
        .tasks
        .iter()
        .find(|(_, body)| body.contains("Wire the thing"))
        .map(|(_, body)| body.clone())
        .expect("task file for wire-thing");
    assert!(
        !task_body.contains("## Code Anchors"),
        "no anchors declared means no Code Anchors section"
    );
}

#[test]
fn anchors_parse_from_toml() {
    let tmp = TempDir::new().expect("temp dir");
    let plan = anchor_plan(
        r#"
[[phases.tasks.anchors]]
path = "src/main.rs"
lines = "1-3"
role = "modify"
note = "the entry point"
[[phases.tasks.anchors]]
path = "src/lib.rs"
role = "read"
"#,
        tmp.path(),
    );
    let resolved = plan.resolve_tasks().expect("resolve");
    let anchors = &resolved[0].anchors;
    assert_eq!(anchors.len(), 2);
    assert_eq!(anchors[0].path, "src/main.rs");
    assert_eq!(anchors[0].citation(), "src/main.rs:1-3");
    assert_eq!(anchors[0].role.to_string(), "modify");
    // role defaults to `read` when omitted
    assert_eq!(anchors[1].role.to_string(), "read");
    assert_eq!(anchors[1].citation(), "src/lib.rs");
}

#[test]
fn anchors_render_into_task_file() {
    let tmp = TempDir::new().expect("temp dir");
    let plan = anchor_plan(
        r#"
[[phases.tasks.anchors]]
path = "src/main.rs"
lines = "1-3"
role = "modify"
note = "the entry point"
[[phases.tasks.anchors]]
path = "src/lib.rs"
role = "create"
note = "new library module"
"#,
        tmp.path(),
    );
    let artifacts = generation::generate_all(&plan).expect("generation");
    let task_body = artifacts
        .tasks
        .iter()
        .find(|(_, body)| body.contains("Wire the thing"))
        .map(|(_, body)| body.clone())
        .expect("task file");

    assert!(task_body.contains("## Code Anchors"));
    assert!(task_body.contains("`src/main.rs:1-3`"));
    assert!(task_body.contains("**modify**"));
    assert!(task_body.contains("the entry point"));
    // A file-level anchor renders without a line range.
    assert!(task_body.contains("`src/lib.rs`"));
    assert!(task_body.contains("**create**"));
}

#[test]
fn anchor_drift_is_reported_when_file_moves() {
    let tmp = TempDir::new().expect("temp dir");
    std::fs::create_dir_all(tmp.path().join("src")).expect("mkdir");
    std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}\n").expect("write");

    let plan = anchor_plan(
        r#"
[[phases.tasks.anchors]]
path = "src/main.rs"
lines = "400-500"
role = "modify"
note = "region that no longer exists"
"#,
        tmp.path(),
    );
    let resolved = plan.resolve_tasks().expect("resolve");
    let diags = wiggum::domain::anchors::lint_anchors(&resolved, tmp.path());

    assert_eq!(
        diags
            .iter()
            .filter(|d| d.rule == "anchor-line-drift")
            .count(),
        1,
        "expected one drift warning, got {diags:?}"
    );
}

#[test]
fn anchor_on_deleted_file_is_an_error() {
    let tmp = TempDir::new().expect("temp dir");
    let plan = anchor_plan(
        r#"
[[phases.tasks.anchors]]
path = "src/vanished.rs"
role = "modify"
note = "file was deleted"
"#,
        tmp.path(),
    );
    let resolved = plan.resolve_tasks().expect("resolve");
    let diags = wiggum::domain::anchors::lint_anchors(&resolved, tmp.path());

    assert_eq!(
        diags
            .iter()
            .filter(|d| d.rule == "anchor-file-missing")
            .count(),
        1,
        "expected one missing-file error, got {diags:?}"
    );
}

#[test]
fn task_verify_commands_reach_task_file_and_orchestrators() {
    let mut plan = load_example_plan();
    let task = plan
        .phases
        .iter_mut()
        .flat_map(|p| p.tasks.iter_mut())
        .find(|t| t.slug == "domain-model")
        .expect("fixture has domain-model");
    task.verify = vec!["cargo test --test foo".to_string()];

    let artifacts = generation::generate_all(&plan).expect("Generation failed");

    let (_, task_file) = artifacts
        .tasks
        .iter()
        .find(|(name, _)| name == "T02-domain-model.md")
        .expect("T02 task file");
    assert!(
        task_file.contains("- [ ] Verify: `cargo test --test foo` exits with status 0"),
        "task file must list the verify command in Exit Criteria:\n{task_file}"
    );

    for (name, orchestrator) in [
        ("vscode", &artifacts.orchestrator_vscode),
        ("opencode", &artifacts.orchestrator_opencode),
        ("root", &artifacts.orchestrator_root),
    ] {
        assert!(
            orchestrator.contains("`cargo test --test foo`"),
            "{name} orchestrator verify step must name the command"
        );
    }
}

#[test]
fn stop_and_ask_section_in_every_orchestrator_variant() {
    let mut plan = load_example_plan();
    let default_plan = generation::generate_all(&plan).expect("Generation failed");
    for (name, text) in [
        ("vscode", &default_plan.orchestrator_vscode),
        ("opencode", &default_plan.orchestrator_opencode),
        ("root", &default_plan.orchestrator_root),
    ] {
        assert!(text.contains("## Stop and ask"), "{name}: missing section");
        assert!(
            text.contains("before force-pushing"),
            "{name}: missing default"
        );
        assert!(
            text.contains("Keep working when the next step does not need the human's input."),
            "{name}: missing closing sentence"
        );
        assert!(!text.contains("before rotating the signing key"));
    }

    plan.orchestrator.stop_when = vec!["before rotating the signing key".to_string()];
    let artifacts = generation::generate_all(&plan).expect("Generation failed");
    let claude = wiggum::generation::claude::render(&plan).expect("claude render");
    for (name, text) in [
        ("vscode", &artifacts.orchestrator_vscode),
        ("opencode", &artifacts.orchestrator_opencode),
        ("root", &artifacts.orchestrator_root),
        ("claude", &claude),
    ] {
        assert!(text.contains("## Stop and ask"), "{name}: missing section");
        let default_at = text.find("before force-pushing").expect("default entry");
        let custom_at = text
            .find("before rotating the signing key")
            .expect("custom entry");
        assert!(
            custom_at > default_at,
            "{name}: custom entry must follow defaults"
        );
    }
}

#[test]
fn known_baseline_failures_in_progress_and_orchestrators() {
    let plan = load_example_plan();
    let artifacts = generation::generate_all(&plan).expect("Generation failed");
    assert!(artifacts.progress.contains("## Known baseline failures"));
    for (name, text) in [
        ("vscode", &artifacts.orchestrator_vscode),
        ("opencode", &artifacts.orchestrator_opencode),
        ("root", &artifacts.orchestrator_root),
    ] {
        assert!(
            text.contains("Known baseline failures"),
            "{name}: baseline heading not mentioned"
        );
        assert!(
            text.to_lowercase().contains("the preflight once"),
            "{name}: baseline step missing"
        );
        assert!(
            text.contains("never add"),
            "{name}: rule that tasks never add baseline entries missing"
        );
    }
    assert!(
        artifacts
            .orchestrator_vscode
            .contains("must not show any failure that is not in")
    );
}

const BASELINE_PLAN_NO_CAPS: &str = r#"
[project]
name = "kestrel"
path = "/tmp/kestrel"
description = "Webhook relay"
language = "rust"

[[phases]]
name = "Inbound"
order = 1
[[phases.tasks]]
slug = "router"
title = "Router"
goal = "Stand up the inbound router."
verify = ["cargo test --test foo"]
"#;

const BASELINE_PLAN_WITH_CAPS: &str = r#"
[project]
name = "kestrel"
path = "/tmp/kestrel"
description = "Webhook relay"
language = "rust"

[[capabilities]]
name = "webhook-reception"
title = "Inbound Webhook Reception"
description = "Accepts and validates inbound webhooks."
[[capabilities.scenarios]]
name = "valid-signature"
when = "POST /webhook receives a valid HMAC signature"
then = "the server returns 202 Accepted"

[[phases]]
name = "Inbound"
order = 1
[[phases.tasks]]
slug = "router"
title = "Router"
goal = "Stand up the inbound router."
implements = ["webhook-reception"]
verify = ["cargo test --test foo"]
"#;

fn assert_baseline_step_on_own_line(text: &str, label: &str, marker: &str) {
    let lines: Vec<&str> = text.lines().collect();
    let pos = lines.iter().position(|l| l.starts_with(marker));
    assert!(pos.is_some(), "{label}: no line starts with {marker:?}");
    let idx = pos.expect("checked above");
    assert!(idx > 0, "{label}: baseline step is the first line");
    let prev = lines[idx - 1].trim_end();
    let before_step = if prev.is_empty() {
        lines[idx - 2].trim_end()
    } else {
        prev
    };
    assert!(
        before_step.ends_with("should have been created.")
            || before_step.ends_with("capability(ies)."),
        "{label}: text glued before baseline step: {before_step:?}"
    );
    assert!(
        !prev.contains("**Record the baseline**"),
        "{label}: duplicate baseline step"
    );
}

#[test]
fn baseline_step_renders_on_its_own_line_with_and_without_capabilities() {
    let plain = Plan::from_toml(BASELINE_PLAN_NO_CAPS).expect("plan parses");
    let plain_art = generation::generate_all(&plain).expect("generation");
    let vscode = &plain_art.orchestrator_vscode;
    assert_baseline_step_on_own_line(vscode, "vscode/no-caps", "3. **Record the baseline**");
    let before = vscode
        .lines()
        .take_while(|l| !l.starts_with("3. **Record the baseline**"))
        .last()
        .expect("line before step");
    assert!(
        before.ends_with("should have been created."),
        "no-caps: preceding line is {before:?}"
    );

    let caps = Plan::from_toml(BASELINE_PLAN_WITH_CAPS).expect("plan parses");
    let caps_art = generation::generate_all(&caps).expect("generation");
    assert_baseline_step_on_own_line(
        &caps_art.orchestrator_vscode,
        "vscode/caps",
        "4. **Record the baseline**",
    );
    assert!(
        caps_art
            .orchestrator_vscode
            .lines()
            .any(|l| l.starts_with("3. Read every file in `capabilities/`")),
        "caps: step 3 must start its own line"
    );
    assert!(
        caps_art
            .orchestrator_opencode
            .lines()
            .any(|l| l.starts_with("3. **Record the baseline**")),
        "opencode baseline step must start a line"
    );
}

#[test]
fn baseline_resume_rule_and_subagent_wording() {
    let plan = Plan::from_toml(BASELINE_PLAN_NO_CAPS).expect("plan parses");
    let art = generation::generate_all(&plan).expect("generation");
    let claude = wiggum::generation::claude::render(&plan).expect("claude render");
    for (name, text) in [
        ("vscode", &art.orchestrator_vscode),
        ("opencode", &art.orchestrator_opencode),
    ] {
        assert!(
            text.contains("`_None recorded._`"),
            "{name}: placeholder rule"
        );
        assert!(
            text.contains("`None (clean, recorded before T01)`"),
            "{name}: clean-baseline marker"
        );
        assert!(
            !text.contains("until preflight passes"),
            "{name}: stale subagent preflight wording"
        );
        assert!(
            text.contains("never add to the baseline"),
            "{name}: subagent baseline rule"
        );
    }
    assert!(!art.orchestrator_root.contains("until preflight passes"));

    for (name, text) in [
        ("vscode", &art.orchestrator_vscode),
        ("opencode", &art.orchestrator_opencode),
        ("root", &art.orchestrator_root),
        ("claude", &claude),
    ] {
        assert!(!text.contains("`Verify:'"), "{name}: mismatched backtick");
        for line in text.lines().filter(|l| l.contains("Verify:")) {
            assert_eq!(
                line.matches('`').count() % 2,
                0,
                "{name}: odd backticks on line {line:?}"
            );
        }
    }
    assert!(
        claude.contains("\n\n## Strategy"),
        "claude: blank line before Strategy"
    );
    assert!(claude.contains("not already listed under **Known baseline failures**"));
}

const APPROVAL_RULE_LINES: [&str; 6] = [
    "## Approvals",
    "- An approval applies only to its recorded scope (the task and the environment it names).",
    "- An approval for a test or staging action never authorizes a production action.",
    "- Never reuse an approval for a different task or a different environment.",
    "- If the scope of an approval is unclear or missing, stop and ask the human.",
    "- An approval expires at the end of the task it was given for, unless its row says otherwise.",
];
const APPROVAL_RECORD_LINE: &str = "- Record every human approval as a row in the **Approvals** table in `PROGRESS.md` (scope, action, approved by, expires) before acting on it.";

fn assert_lines_present(name: &str, text: &str, lines: &[&str]) {
    let have: Vec<&str> = text.lines().collect();
    for want in lines {
        assert!(have.contains(want), "{name}: missing whole line `{want}`");
    }
}

#[test]
fn approval_rules_in_every_orchestrator_variant_and_claude() {
    let plan = load_example_plan();
    let artifacts = generation::generate_all(&plan).expect("Generation failed");
    let claude = wiggum::generation::claude::render(&plan).expect("claude render");
    for (name, text) in [
        ("vscode", &artifacts.orchestrator_vscode),
        ("opencode", &artifacts.orchestrator_opencode),
        ("root", &artifacts.orchestrator_root),
        ("claude", &claude),
    ] {
        assert_lines_present(name, text, &APPROVAL_RULE_LINES);
        assert_lines_present(name, text, &[APPROVAL_RECORD_LINE]);
    }
    // The vscode and opencode files also carry the subagent prompt, which
    // repeats the rules so a subagent dispatched alone still sees them.
    for (name, text) in [
        ("vscode", &artifacts.orchestrator_vscode),
        ("opencode", &artifacts.orchestrator_opencode),
    ] {
        let count = text.lines().filter(|l| *l == "## Approvals").count();
        assert_eq!(count, 2, "{name}: orchestrator and subagent sections");
    }
}

#[test]
fn gate_step_records_confirmation_in_approvals_table() {
    let plan = load_example_plan();
    let artifacts = generation::generate_all(&plan).expect("Generation failed");
    let vscode_lines = [
        "   Record that confirmation as a row in the **Approvals** table in PROGRESS.md, using the",
        "   gate's `Scope:` line as the scope when the banner has one.",
    ];
    let opencode_lines = [
        "   and **stop**. The human must confirm before you proceed. Record that confirmation as a row",
        "   in the **Approvals** table in `PROGRESS.md`, using the gate's `Scope:` line as the scope",
        "   when the banner has one.",
    ];
    let root_line = "1. **Gate check.** If the task file begins with `⛔ GATE`, emit the banner verbatim and stop. The human must confirm; record that confirmation as a row in the **Approvals** table in `PROGRESS.md` (scope from the banner's `Scope:` line when present).";
    assert_lines_present(
        "vscode gate step (confirmation recorded in Approvals)",
        &artifacts.orchestrator_vscode,
        &vscode_lines,
    );
    assert_lines_present(
        "opencode gate step (confirmation recorded in Approvals)",
        &artifacts.orchestrator_opencode,
        &opencode_lines,
    );
    assert_lines_present(
        "ORCHESTRATOR.md gate step (confirmation recorded in Approvals)",
        &artifacts.orchestrator_root,
        &[root_line],
    );
}

#[test]
fn progress_approvals_note_names_prompts_for_every_target() {
    let plan = load_example_plan();
    let artifacts = generation::generate_all(&plan).expect("Generation failed");
    assert!(
        artifacts
            .progress
            .contains("generated orchestrator or CLAUDE.md prompt says what an approval covers"),
        "PROGRESS approvals note must name CLAUDE.md as well as the orchestrator"
    );
}

#[test]
fn progress_has_approvals_table_and_still_parses() {
    let plan = load_example_plan();
    let artifacts = generation::generate_all(&plan).expect("Generation failed");
    let lines: Vec<&str> = artifacts.progress.lines().collect();
    let header = lines
        .iter()
        .position(|l| *l == "| Scope | Action | Approved by | Expires |")
        .expect("approvals table header line");
    assert_eq!(lines[header + 1], "|---|---|---|---|");
    let approvals = lines.iter().position(|l| *l == "## Approvals").unwrap();
    let baseline = lines
        .iter()
        .position(|l| *l == "## Known baseline failures")
        .unwrap();
    let learnings = lines
        .iter()
        .position(|l| *l == "## Accumulated Learnings")
        .unwrap();
    assert!(baseline < approvals && approvals < header && header < learnings);
    assert!(lines.contains(&"_None recorded._"));

    let summary = wiggum::domain::report::parse_progress(&artifacts.progress);
    let resolved = plan.resolve_tasks().expect("resolve");
    assert_eq!(summary.tasks.len(), resolved.len());
    assert!(
        summary
            .tasks
            .iter()
            .all(|t| !t.title.contains("Approved by"))
    );
}

#[test]
fn gate_scope_renders_in_task_banner_only_when_set() {
    let mut plan = load_example_plan();
    plan.phases[0].tasks[0].gate = Some("deploy".to_string());
    let without = generation::generate_all(&plan).expect("Generation failed");
    assert!(
        !without.tasks[0]
            .1
            .lines()
            .any(|l| l.starts_with("> Scope:"))
    );

    plan.phases[0].tasks[0].gate_scope = Some("T01, staging only".to_string());
    let with = generation::generate_all(&plan).expect("Generation failed");
    let lines: Vec<&str> = with.tasks[0].1.lines().collect();
    let cat = lines
        .iter()
        .position(|l| l.starts_with("> Category: `deploy`"))
        .expect("category line");
    assert_eq!(lines[cat + 1], "> Scope: T01, staging only");
    assert_eq!(lines[cat + 2], ">");
}

#[test]
fn blank_gate_scope_renders_no_scope_line() {
    let mut plan = load_example_plan();
    plan.phases[0].tasks[0].gate = Some("deploy".to_string());
    plan.phases[0].tasks[0].gate_scope = Some("   ".to_string());
    let art = generation::generate_all(&plan).expect("Generation failed");
    assert!(!art.tasks[0].1.lines().any(|l| l.starts_with("> Scope:")));
}

#[test]
fn approval_row_in_generated_progress_is_not_counted_as_a_task() {
    let plan = load_example_plan();
    let art = generation::generate_all(&plan).expect("Generation failed");
    let expected = plan.resolve_tasks().expect("resolve").len();
    let row =
        "| T02 — staging payment sandbox | run capture against sandbox | alice | end of T02 |";
    let progress = art.progress.replace(
        "|---|---|---|---|\n",
        &format!("|---|---|---|---|\n{row}\n"),
    );
    assert!(progress.lines().any(|l| l == row), "row was inserted");
    let summary = wiggum::domain::report::parse_progress(&progress);
    assert_eq!(summary.tasks.len(), expected);
    assert!(
        summary
            .tasks
            .iter()
            .all(|t| t.title != "staging payment sandbox")
    );
}

/// Collect every `*.toml` file under `dir`, recursively.
fn collect_toml_files(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).expect("read shipped plan dir");
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_toml_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "toml") {
            out.push(path);
        }
    }
}

/// Every shipped plan that configures an `[evaluator]` must pin a model that
/// differs from the implementer's, so the `evaluator-not-independent` lint
/// stays quiet on the fixtures, reference plans (including the example
/// workspace) and golden plans. Plans are found by a recursive directory scan;
/// TOML files that do not parse as a `Plan` (hints, workspace manifests) are
/// skipped. All offenders are reported together.
#[test]
fn shipped_plans_have_independent_evaluators() {
    const EXPECTED_PLANS: usize = 15;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in ["tests/fixtures", "tests/golden", "reference"] {
        collect_toml_files(&root.join(dir), &mut files);
    }
    files.sort();

    let mut checked = Vec::new();
    let mut offenders = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path).expect("read toml");
        let Ok(plan) = Plan::from_toml(&text) else {
            continue;
        };
        let resolved = plan.resolve_tasks().expect("resolve");
        let diags = wiggum::domain::lint::lint_plan(&plan, &resolved);
        checked.push(path.display().to_string());
        if diags.iter().any(|d| d.rule == "evaluator-not-independent") {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        checked.len() >= EXPECTED_PLANS,
        "expected at least {EXPECTED_PLANS} shipped plans, checked {}: {checked:#?}",
        checked.len()
    );
    assert!(
        offenders.is_empty(),
        "plans raising evaluator-not-independent: {offenders:#?}"
    );
}
