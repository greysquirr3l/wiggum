//! End-to-end tests for `wiggum reverse`.
//!
//! Builds a tiny local git repo + writes hints + invokes the CLI via
//! `assert_cmd`. Network-free: uses a `file://` URL to point at the local
//! repo so no GitHub auth or rate limits are involved.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::PathBuf;
use std::process::Command as StdCommand;

use assert_cmd::Command;
use tempfile::TempDir;

const CARGO_TOML: &str = r#"[package]
name = "fixture-crate"
description = "A fixture repo for reverse tests"
version = "0.1.0"
edition = "2021"
"#;

const HINTS_TOML: &str = r#"
[project]
language = "go"
architecture = "ddd"
extra-rules = ["Use the latest stable Go"]

[orchestrator]
persona = "Senior Go engineer"
strategy = "complete"
max-retries = 3
on-failure = "pause"
extra-rules = ["Run `go vet ./...`"]

[[phase]]
name = "Domain Core"

[[phase.tasks]]
slug = "aggregates"
title = "Define aggregate roots"
goal = "Identify the core aggregates"
hints = ["Start with Account"]

[[phase]]
name = "Adapters"

[[phase.tasks]]
slug = "http-adapter"
title = "HTTP adapter"
goal = "Inbound HTTP layer"
depends-on = ["aggregates"]
"#;

/// Build a tiny git repo in `dir` with a Cargo.toml and one commit.
fn make_git_fixture(dir: &std::path::Path) {
    std::fs::write(dir.join("Cargo.toml"), CARGO_TOML).expect("write Cargo.toml");
    std::fs::create_dir_all(dir.join("src")).expect("mkdir src");
    std::fs::write(dir.join("src/lib.rs"), "// placeholder\n").expect("write lib.rs");

    let run = |args: &[&str]| {
        let status = StdCommand::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .status()
            .expect("git command");
        assert!(status.success(), "git {args:?} failed");
    };

    run(&["init", "--initial-branch=main"]);
    run(&["add", "."]);
    run(&["commit", "-m", "initial"]);
}

#[test]
fn reverse_against_local_repo_with_hints() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path().join("fixture");
    std::fs::create_dir(&repo).expect("mkdir fixture");
    make_git_fixture(&repo);

    let hints_path = tmp.path().join("hints.toml");
    std::fs::write(&hints_path, HINTS_TOML).expect("write hints.toml");

    let out_path = tmp.path().join("plan.toml");

    let url = format!("file://{}", repo.display());
    Command::cargo_bin("wiggum")
        .expect("cargo_bin")
        .env("RUST_LOG", "error")
        .arg("reverse")
        .arg(&url)
        .arg("--hints")
        .arg(&hints_path)
        .arg("--output")
        .arg(&out_path)
        .assert()
        .success();

    let toml = std::fs::read_to_string(&out_path).expect("read plan.toml");

    // Sanity: the generated plan should be parseable by wiggum itself.
    let plan: toml::Value = toml::from_str(&toml).expect("parse generated TOML");
    let project = plan.get("project").expect("project section");
    assert_eq!(
        project.get("language").and_then(|v| v.as_str()),
        Some("go"),
        "hints.toml should override detected language (rust) with go"
    );
    assert_eq!(
        project.get("architecture").and_then(|v| v.as_str()),
        Some("ddd")
    );

    let orchestrator = plan.get("orchestrator").expect("orchestrator section");
    assert_eq!(
        orchestrator.get("persona").and_then(|v| v.as_str()),
        Some("Senior Go engineer")
    );
    assert_eq!(
        orchestrator.get("strategy").and_then(|v| v.as_str()),
        Some("complete")
    );
    assert_eq!(
        orchestrator
            .get("max_retries")
            .and_then(toml::Value::as_integer),
        Some(3)
    );
    assert_eq!(
        orchestrator.get("on_failure").and_then(|v| v.as_str()),
        Some("pause")
    );

    let rules = orchestrator
        .get("rules")
        .and_then(|v| v.as_array())
        .expect("rules array");
    let rule_strs: Vec<&str> = rules.iter().filter_map(|v| v.as_str()).collect();
    assert!(rule_strs.contains(&"Use the latest stable Go"));
    assert!(rule_strs.contains(&"Run `go vet ./...`"));

    let phases = plan
        .get("phases")
        .and_then(|v| v.as_array())
        .expect("phases");
    assert_eq!(phases.len(), 2, "two [[phase]] blocks in hints.toml");

    let first_phase = &phases[0];
    let first_tasks = first_phase
        .get("tasks")
        .and_then(|v| v.as_array())
        .expect("first phase tasks");
    assert_eq!(first_tasks.len(), 1);
    assert_eq!(
        first_tasks[0].get("slug").and_then(|v| v.as_str()),
        Some("aggregates")
    );

    let second_phase = &phases[1];
    let second_tasks = second_phase
        .get("tasks")
        .and_then(|v| v.as_array())
        .expect("second phase tasks");
    assert_eq!(second_tasks.len(), 1);
    assert_eq!(
        second_tasks[0].get("slug").and_then(|v| v.as_str()),
        Some("http-adapter")
    );
    let deps = second_tasks[0]
        .get("depends_on")
        .and_then(|v| v.as_array())
        .expect("depends_on array");
    let dep_strs: Vec<&str> = deps.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(dep_strs, vec!["aggregates"]);
}

#[test]
fn reverse_against_local_repo_no_hints() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path().join("fixture");
    std::fs::create_dir(&repo).expect("mkdir fixture");
    make_git_fixture(&repo);

    let out_path = tmp.path().join("plan.toml");
    let url = format!("file://{}", repo.display());

    Command::cargo_bin("wiggum")
        .expect("cargo_bin")
        .env("RUST_LOG", "error")
        .arg("reverse")
        .arg(&url)
        .arg("--output")
        .arg(&out_path)
        .assert()
        .success();

    let toml = std::fs::read_to_string(&out_path).expect("read plan.toml");
    let plan: toml::Value = toml::from_str(&toml).expect("parse generated TOML");
    let project = plan.get("project").expect("project section");
    // No hints → language should match what the local fixture actually has.
    assert_eq!(
        project.get("language").and_then(|v| v.as_str()),
        Some("rust")
    );
    assert_eq!(
        project.get("name").and_then(|v| v.as_str()),
        Some("fixture-crate")
    );
    assert_eq!(
        project.get("description").and_then(|v| v.as_str()),
        Some("A fixture repo for reverse tests")
    );
}

#[test]
fn reverse_errors_on_existing_output_without_force() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path().join("fixture");
    std::fs::create_dir(&repo).expect("mkdir fixture");
    make_git_fixture(&repo);

    let out_path = tmp.path().join("plan.toml");
    std::fs::write(&out_path, "existing content").expect("write plan.toml");

    let url = format!("file://{}", repo.display());
    Command::cargo_bin("wiggum")
        .expect("cargo_bin")
        .env("RUST_LOG", "error")
        .arg("reverse")
        .arg(&url)
        .arg("--output")
        .arg(&out_path)
        .assert()
        .failure()
        .stderr(predicates::str::contains("already exists"));

    // The original file must be untouched.
    let content = std::fs::read_to_string(&out_path).expect("read plan.toml");
    assert_eq!(content, "existing content");
}

#[test]
fn reverse_with_force_overwrites_existing_output() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path().join("fixture");
    std::fs::create_dir(&repo).expect("mkdir fixture");
    make_git_fixture(&repo);

    let out_path = tmp.path().join("plan.toml");
    std::fs::write(&out_path, "stale content").expect("write plan.toml");

    let url = format!("file://{}", repo.display());
    Command::cargo_bin("wiggum")
        .expect("cargo_bin")
        .env("RUST_LOG", "error")
        .arg("reverse")
        .arg(&url)
        .arg("--output")
        .arg(&out_path)
        .arg("--force")
        .assert()
        .success();

    let content = std::fs::read_to_string(&out_path).expect("read plan.toml");
    assert_ne!(content, "stale content");
    assert!(content.contains("[project]"));
}

#[test]
fn reverse_appears_in_cli_help() {
    // `wiggum --help` lists the `reverse` subcommand.
    let top_help = Command::cargo_bin("wiggum")
        .expect("cargo_bin")
        .env("RUST_LOG", "error")
        .arg("--help")
        .output()
        .expect("invoke wiggum --help");
    let top_stdout = String::from_utf8_lossy(&top_help.stdout);
    assert!(
        top_stdout.contains("reverse"),
        "reverse subcommand must appear in `wiggum --help`, got:\n{top_stdout}"
    );

    // `wiggum reverse --help` lists the per-subcommand flags.
    let sub_help = Command::cargo_bin("wiggum")
        .expect("cargo_bin")
        .env("RUST_LOG", "error")
        .arg("reverse")
        .arg("--help")
        .output()
        .expect("invoke wiggum reverse --help");
    let sub_stdout = String::from_utf8_lossy(&sub_help.stdout);
    assert!(
        sub_stdout.contains("--hints"),
        "--hints flag must appear in `wiggum reverse --help`, got:\n{sub_stdout}"
    );
    assert!(
        sub_stdout.contains("--keep-tmp"),
        "--keep-tmp flag must appear in `wiggum reverse --help`, got:\n{sub_stdout}"
    );
}

#[test]
fn reverse_rejects_bad_hints_extension() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path().join("fixture");
    std::fs::create_dir(&repo).expect("mkdir fixture");
    make_git_fixture(&repo);

    let hints_path = tmp.path().join("hints.json");
    std::fs::write(&hints_path, "{}").expect("write hints.json");

    let out_path = tmp.path().join("plan.toml");
    let url = format!("file://{}", repo.display());

    Command::cargo_bin("wiggum")
        .expect("cargo_bin")
        .env("RUST_LOG", "error")
        .arg("reverse")
        .arg(&url)
        .arg("--hints")
        .arg(&hints_path)
        .arg("--output")
        .arg(&out_path)
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "unrecognized hints file extension",
        ));
}

/// Silence unused-import for `PathBuf` in case future edits drop it.
#[allow(dead_code)]
fn _keep_pathbuf(_: PathBuf) {}
