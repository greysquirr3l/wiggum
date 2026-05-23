#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::PathBuf;
use tempfile::TempDir;

use wiggum::adapters::fs::FsAdapter;
use wiggum::adapters::retro::{RetroSuggestion, RetroSummary, Severity};
use wiggum::adapters::{patterns, replan};
use wiggum::domain::check::score_plan;
use wiggum::domain::dag::validate_dag;
use wiggum::domain::plan::Plan;
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

    // Check orchestrator
    assert!(artifacts.orchestrator.contains("example-project"));
    assert!(artifacts.orchestrator.contains("runSubagent"));

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
    generation::write_artifacts(&fs, &project_path, &artifacts).expect("Failed to write artifacts");

    // Verify files exist
    assert!(project_path.join("PROGRESS.md").exists());
    assert!(project_path.join("IMPLEMENTATION_PLAN.md").exists());
    assert!(project_path.join(".vscode/orchestrator.prompt.md").exists());
    assert!(
        project_path
            .join("tasks/T01-workspace-scaffold.md")
            .exists()
    );
    assert!(project_path.join("tasks/T05-persistence.md").exists());
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
        !artifacts.planner_prompt.is_empty(),
        "planner prompt should be generated"
    );
    assert!(
        !artifacts.background_auditor_prompt.is_empty(),
        "background auditor prompt should be generated"
    );
    assert!(
        artifacts.hooks_json.contains("PreToolUse") || artifacts.hooks_json.contains("hooks"),
        "hooks.json should contain Claude hook structure"
    );
    assert!(
        artifacts.planner_prompt.contains("planner") || artifacts.planner_prompt.contains("plan"),
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
fn check_score_plan_has_six_dimensions() {
    let plan = load_example_plan();
    let resolved = plan.resolve_tasks().expect("resolve");
    let score = score_plan(&plan, &resolved);
    assert_eq!(
        score.dimensions.len(),
        6,
        "Expected exactly 6 scoring dimensions"
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
