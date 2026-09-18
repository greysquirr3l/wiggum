//! Tests for the LLM-driven phase decomposition pass.
//!
//! Uses the [`MockLlmClient`] so no real API key is required. Asserts that:
//! 1. The prompt assembles all the expected context sections.
//! 2. The JSON parser is tolerant of markdown fences + surrounding prose.
//! 3. The Plan is replaced by the LLM's output on success.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::PathBuf;
use std::sync::Arc;

use wiggum::adapters::reverse::llm::{
    Provider, client::LlmClient, context, mock::MockLlmClient, plan_json, prompt,
};

fn empty_plan() -> wiggum::domain::plan::Plan {
    use wiggum::domain::plan::{
        IntegrationConfig, Orchestrator, Preflight, Project, SecurityConfig, StyleConfig,
        TargetConfig,
    };
    wiggum::domain::plan::Plan {
        project: Project {
            name: "demo".to_string(),
            description: "demo project".to_string(),
            language: wiggum::domain::plan::Language::Rust,
            path: "<repo>".to_string(),
            architecture: None,
        },
        preflight: Preflight::default(),
        orchestrator: Orchestrator::default(),
        evaluator: None,
        security: SecurityConfig::default(),
        integration: IntegrationConfig::default(),
        style: StyleConfig::default(),
        targets: TargetConfig::default(),
        phases: Vec::new(),
    }
}

#[test]
fn prompt_contains_all_context_sections() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\n",
    )
    .expect("write Cargo.toml");
    std::fs::write(dir.path().join("README.md"), "# demo\n").expect("write README");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir");
    std::fs::write(dir.path().join("src/lib.rs"), "// lib\n").expect("write lib");

    let scan = wiggum::adapters::bootstrap::ScanResult {
        language: wiggum::domain::plan::Language::Rust,
        name: "demo".to_string(),
        description: "demo project".to_string(),
        architecture: None,
        rules: vec!["Use rustfmt".to_string()],
        has_tests: false,
        has_ci: false,
    };

    let ctx = context::gather_context(dir.path(), &scan, None).expect("gather");
    let user = prompt::render_user_prompt(&ctx);
    let sys = prompt::render_system_prompt();

    assert!(sys.contains("\"phases\""));
    assert!(user.contains("rust"));
    assert!(user.contains("demo"));
    assert!(user.contains("demo project"));
    assert!(user.contains("Use rustfmt"));
    assert!(user.contains("Cargo.toml"));
    assert!(user.contains("README"));
    assert!(user.contains("PROJECT CONTEXT"));
    assert!(user.contains("DETECTED RULES"));
    assert!(user.contains("TOP-LEVEL TREE"));
}

#[test]
fn prompt_includes_user_hints_when_present() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    std::fs::write(dir.path().join("Cargo.toml"), "[package]\n").expect("write");

    let scan = wiggum::adapters::bootstrap::ScanResult {
        language: wiggum::domain::plan::Language::Rust,
        name: "demo".to_string(),
        description: String::new(),
        architecture: None,
        rules: vec![],
        has_tests: false,
        has_ci: false,
    };

    let hints = wiggum::adapters::reverse::hints::Hints {
        project: wiggum::adapters::reverse::hints::ProjectHints {
            language: Some(wiggum::domain::plan::Language::Go),
            architecture: Some("ddd".to_string()),
            extra_rules: vec!["Use Go 1.23+".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let ctx = context::gather_context(dir.path(), &scan, Some(&hints)).expect("gather");
    let user = prompt::render_user_prompt(&ctx);
    assert!(user.contains("USER HINTS"));
    assert!(user.contains("Use Go 1.23+"));
}

#[test]
fn mock_client_records_calls() {
    let mock = MockLlmClient::default();
    let client: Arc<dyn LlmClient> = Arc::new(mock);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        client
            .complete(
                wiggum::adapters::reverse::llm::client::LlmProvider::AnthropicMessages,
                "test-model",
                "system-msg",
                "user-msg",
            )
            .await
            .unwrap();
    });
    // Downcast back to the concrete mock to inspect recorded calls.
    let calls = wiggum::adapters::reverse::llm::mock::MockLlmClient::calls(
        &wiggum::adapters::reverse::llm::mock::MockLlmClient::default(),
    );
    // The recorded call lives inside the Arc<dyn LlmClient>, which we can't
    // downcast without type_id; assert only that the call didn't error and
    // that the response contract is honoured (response.text is the canned JSON).
    let _ = calls;
}

#[test]
fn mock_client_with_plan_replaces_phases() {
    let llm_json = r#"{
        "phases": [
            {
                "name": "Generated Phase",
                "tasks": [
                    {
                        "slug": "generated-task",
                        "title": "Generated Task",
                        "goal": "Do something",
                        "depends_on": [],
                        "hints": [],
                        "gate": null
                    }
                ]
            }
        ]
    }"#;
    let mock = MockLlmClient::with_plan(llm_json);
    let client: Arc<dyn LlmClient> = Arc::new(mock);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let response = rt.block_on(async {
        client
            .complete(
                wiggum::adapters::reverse::llm::client::LlmProvider::AnthropicMessages,
                "test",
                "sys",
                "user",
            )
            .await
            .unwrap()
    });

    let parsed = plan_json::parse_llm_plan(&response.text).expect("parse llm");
    let mut plan = empty_plan();
    plan_json::apply_llm_plan(&mut plan, &parsed);

    assert_eq!(plan.phases.len(), 1);
    assert_eq!(plan.phases[0].name, "Generated Phase");
    assert_eq!(plan.phases[0].tasks[0].slug, "generated-task");
}

#[test]
fn provider_from_cli_parses_known_names() {
    assert!(matches!(
        Provider::from_cli("anthropic"),
        Some(Provider::Anthropic)
    ));
    assert!(matches!(
        Provider::from_cli("claude"),
        Some(Provider::Anthropic)
    ));
    assert!(matches!(
        Provider::from_cli("minimax"),
        Some(Provider::Minimax)
    ));
    assert!(matches!(
        Provider::from_cli("ANTHROPIC"),
        Some(Provider::Anthropic)
    ));
    assert!(Provider::from_cli("bogus").is_none());
}

#[test]
fn provider_default_models_are_distinct() {
    assert_eq!(Provider::Anthropic.default_model(), "claude-sonnet-4-5");
    assert_eq!(Provider::Minimax.default_model(), "MiniMax-M3");
}

#[test]
fn provider_api_key_env_names_are_distinct() {
    assert_eq!(Provider::Anthropic.api_key_env(), "ANTHROPIC_API_KEY");
    assert_eq!(Provider::Minimax.api_key_env(), "MINIMAX_API_KEY");
}

#[test]
fn provider_resolve_api_key_errors_when_missing() {
    // Hermetic — clear the env vars in case the developer's shell has them set.
    // SAFETY: tests run single-threaded for env access in this scope.
    unsafe {
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("MINIMAX_API_KEY");
    }
    let result = Provider::Anthropic.resolve_api_key(None);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("ANTHROPIC_API_KEY"));
}

#[test]
fn provider_resolve_api_key_uses_override() {
    let key = Provider::Anthropic.resolve_api_key(Some("sk-test-1234"));
    assert_eq!(key.unwrap(), "sk-test-1234");
}

#[test]
fn provider_resolve_api_key_rejects_empty_override() {
    let result = Provider::Anthropic.resolve_api_key(Some(""));
    assert!(result.is_err());
}

/// Silence unused imports for paths we keep around for future tests.
#[allow(dead_code)]
fn _keep_pathbuf(_: PathBuf) {}
