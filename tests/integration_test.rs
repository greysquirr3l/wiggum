#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::PathBuf;
use tempfile::TempDir;

use wiggum::adapters::fs::FsAdapter;
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
    // 5 explicit tasks + 1 auto-injected security-hardening task (http-client triggers detection)
    assert_eq!(resolved.len(), 6);
    assert_eq!(resolved[0].number, 1);
    assert_eq!(resolved[0].slug, "workspace-scaffold");
    assert_eq!(resolved[4].number, 5);
    assert_eq!(resolved[4].slug, "persistence");
    assert_eq!(resolved[5].slug, "security-hardening");
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

    // Check task files (5 explicit + 1 auto-injected security-hardening)
    assert_eq!(artifacts.tasks.len(), 6);
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
