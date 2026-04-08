use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::adapters::fs::FsAdapter;
use crate::domain::dag::validate_dag;
use crate::domain::lint;
use crate::domain::plan::Plan;
use crate::error::{Result, WiggumError};
use crate::generation;
use crate::ports::{PlanReader, ProgressStore};

// ─── JSON-RPC types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

// ─── MCP protocol types ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ServerInfo {
    name: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    protocol_version: String,
    capabilities: Capabilities,
    #[serde(rename = "serverInfo")]
    server_info: ServerInfo,
}

#[derive(Debug, Serialize)]
struct Capabilities {
    tools: ToolsCapability,
}

#[derive(Debug, Serialize)]
struct ToolsCapability {
    #[serde(rename = "listChanged")]
    list_changed: bool,
}

#[derive(Debug, Serialize)]
struct ToolDefinition {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

#[derive(Debug, Serialize)]
struct ToolsListResult {
    tools: Vec<ToolDefinition>,
}

#[derive(Debug, Serialize)]
struct ToolResult {
    content: Vec<ContentBlock>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

// ─── MCP Server ─────────────────────────────────────────────────────

/// # Errors
///
/// Returns an error if stdin/stdout I/O fails.
pub fn run_mcp_server() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = stdin.lock();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {e}"),
                        data: None,
                    }),
                };
                write_response(&mut stdout, &response)?;
                continue;
            }
        };

        if request.jsonrpc != "2.0" {
            let response = error_response(
                request.id.unwrap_or(Value::Null),
                -32600,
                "Invalid JSON-RPC version",
            );
            write_response(&mut stdout, &response)?;
            continue;
        }

        let response = handle_request(&request);
        // Notifications (no id) don't get responses
        if request.id.is_some() {
            write_response(&mut stdout, &response)?;
        }
    }

    Ok(())
}

fn write_response(stdout: &mut io::Stdout, response: &JsonRpcResponse) -> Result<()> {
    let json = serde_json::to_string(response)?;
    writeln!(stdout, "{json}")?;
    stdout.flush()?;
    Ok(())
}

fn handle_request(request: &JsonRpcRequest) -> JsonRpcResponse {
    let id = request.id.clone().unwrap_or(Value::Null);

    match request.method.as_str() {
        "initialize" => {
            match serde_json::to_value(InitializeResult {
                protocol_version: "2025-06-18".to_string(),
                capabilities: Capabilities {
                    tools: ToolsCapability {
                        list_changed: false,
                    },
                },
                server_info: ServerInfo {
                    name: "wiggum".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
            }) {
                Ok(val) => success_response(id, val),
                Err(e) => error_response(id, -32603, &format!("Internal error: {e}")),
            }
        }
        "notifications/initialized" => success_response(id, Value::Null),
        "tools/list" => handle_tools_list(id),
        "tools/call" => handle_tool_call(id, &request.params),
        _ => error_response(id, -32601, &format!("Unknown method: {}", request.method)),
    }
}

fn plan_path_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "plan_path": {
                "type": "string",
                "description": "Absolute path to the plan TOML file"
            }
        },
        "required": ["plan_path"]
    })
}

fn progress_path_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "progress_path": {
                "type": "string",
                "description": "Absolute path to the PROGRESS.md file"
            }
        },
        "required": ["progress_path"]
    })
}

fn handle_tools_list(id: Value) -> JsonRpcResponse {
    let tools = tool_definitions();

    success_response(
        id.clone(),
        match serde_json::to_value(ToolsListResult { tools }) {
            Ok(val) => val,
            Err(e) => return error_response(id, -32603, &format!("Internal error: {e}")),
        },
    )
}

fn tool_definitions() -> Vec<ToolDefinition> {
    let mut tools = core_tool_definitions();
    tools.extend(extended_tool_definitions());
    tools
}

fn core_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "wiggum_generate_plan".to_string(),
            description: "Generate full scaffold from a plan TOML file path".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "plan_path": {
                        "type": "string",
                        "description": "Absolute path to the plan TOML file"
                    },
                    "force": {
                        "type": "boolean",
                        "description": "Overwrite existing files",
                        "default": false
                    }
                },
                "required": ["plan_path"]
            }),
        },
        ToolDefinition {
            name: "wiggum_validate_plan".to_string(),
            description: "Validate a plan TOML file (dependency DAG check, missing fields)"
                .to_string(),
            input_schema: plan_path_schema(),
        },
        ToolDefinition {
            name: "wiggum_read_progress".to_string(),
            description: "Parse PROGRESS.md and return structured status".to_string(),
            input_schema: progress_path_schema(),
        },
        ToolDefinition {
            name: "wiggum_update_progress".to_string(),
            description: "Update a task's status in PROGRESS.md".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "progress_path": {
                        "type": "string",
                        "description": "Absolute path to the PROGRESS.md file"
                    },
                    "task_number": {
                        "type": "integer",
                        "description": "Task number (e.g. 1 for T01)"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["not-started", "in-progress", "completed", "blocked"],
                        "description": "New status for the task"
                    },
                    "notes": {
                        "type": "string",
                        "description": "Optional notes to add"
                    }
                },
                "required": ["progress_path", "task_number", "status"]
            }),
        },
        ToolDefinition {
            name: "wiggum_list_templates".to_string(),
            description: "List available language/architecture templates".to_string(),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
        },
    ]
}

fn extended_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "wiggum_lint_plan".to_string(),
            description: "Run quality lint rules against a plan TOML file".to_string(),
            input_schema: plan_path_schema(),
        },
        ToolDefinition {
            name: "wiggum_report".to_string(),
            description: "Generate a post-execution report from PROGRESS.md".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "progress_path": {
                        "type": "string",
                        "description": "Path to the PROGRESS.md file"
                    },
                    "project_dir": {
                        "type": "string",
                        "description": "Project directory for git timeline (optional)"
                    }
                },
                "required": ["progress_path"]
            }),
        },
        ToolDefinition {
            name: "wiggum_generate_agents_md".to_string(),
            description:
                "Generate an AGENTS.md file from a plan TOML (universal agent instructions)"
                    .to_string(),
            input_schema: plan_path_schema(),
        },
        ToolDefinition {
            name: "wiggum_bootstrap".to_string(),
            description: "Scan an existing project directory and generate a skeleton plan TOML"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "project_path": {
                        "type": "string",
                        "description": "Absolute path to the project directory to scan"
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Path to write the generated plan TOML (defaults to <project_path>/plan.toml)"
                    },
                    "force": {
                        "type": "boolean",
                        "description": "Overwrite existing plan file",
                        "default": false
                    }
                },
                "required": ["project_path"]
            }),
        },
    ]
}

fn handle_tool_call(id: Value, params: &Value) -> JsonRpcResponse {
    let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

    let result = match tool_name {
        "wiggum_generate_plan" => tool_generate_plan(&arguments),
        "wiggum_validate_plan" => tool_validate_plan(&arguments),
        "wiggum_read_progress" => tool_read_progress(&arguments),
        "wiggum_update_progress" => tool_update_progress(&arguments),
        "wiggum_list_templates" => Ok(tool_list_templates()),
        "wiggum_lint_plan" => tool_lint_plan(&arguments),
        "wiggum_report" => tool_report(&arguments),
        "wiggum_generate_agents_md" => tool_generate_agents_md(&arguments),
        "wiggum_bootstrap" => tool_bootstrap(&arguments),
        _ => Err(WiggumError::Validation(format!(
            "Unknown tool: {tool_name}"
        ))),
    };

    match result {
        Ok(text) => {
            let tool_result = ToolResult {
                content: vec![ContentBlock {
                    content_type: "text".to_string(),
                    text,
                }],
                is_error: None,
            };
            match serde_json::to_value(tool_result) {
                Ok(val) => success_response(id, val),
                Err(e) => error_response(id, -32603, &format!("Internal error: {e}")),
            }
        }
        Err(e) => {
            let tool_result = ToolResult {
                content: vec![ContentBlock {
                    content_type: "text".to_string(),
                    text: format!("Error: {e}"),
                }],
                is_error: Some(true),
            };
            match serde_json::to_value(tool_result) {
                Ok(val) => success_response(id, val),
                Err(e) => error_response(id, -32603, &format!("Internal error: {e}")),
            }
        }
    }
}

// ─── Tool implementations ───────────────────────────────────────────

fn tool_generate_plan(args: &Value) -> Result<String> {
    let plan_path = args
        .get("plan_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WiggumError::Validation("plan_path is required".to_string()))?;

    let fs = FsAdapter;
    let toml_content = fs.read_plan(&PathBuf::from(plan_path))?;
    let plan = Plan::from_toml(&toml_content)?;
    let artifacts = generation::generate_all(&plan)?;

    let project_path = PathBuf::from(&plan.project.path);

    let vcs_warning = match super::vcs::check_vcs_status(&project_path) {
        super::vcs::VcsStatus::Dirty(_) => {
            "\n⚠️  Warning: target directory has uncommitted changes."
        }
        _ => "",
    };

    generation::write_artifacts(&fs, &project_path, &artifacts)?;

    Ok(format!(
        "Generated {} task files, PROGRESS.md, IMPLEMENTATION_PLAN.md, and orchestrator.prompt.md in {}{vcs_warning}",
        artifacts.tasks.len(),
        plan.project.path
    ))
}

fn tool_validate_plan(args: &Value) -> Result<String> {
    let plan_path = args
        .get("plan_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WiggumError::Validation("plan_path is required".to_string()))?;

    let fs = FsAdapter;
    let toml_content = fs.read_plan(&PathBuf::from(plan_path))?;
    let plan = Plan::from_toml(&toml_content)?;
    let resolved = plan.resolve_tasks()?;
    let sorted = validate_dag(&resolved)?;

    Ok(format!(
        "Plan is valid.\n  Phases: {}\n  Tasks: {}\n  Execution order: {}",
        plan.phases.len(),
        resolved.len(),
        sorted.join(" → ")
    ))
}

fn tool_lint_plan(args: &Value) -> Result<String> {
    let plan_path = args
        .get("plan_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WiggumError::Validation("plan_path is required".to_string()))?;

    let fs = FsAdapter;
    let toml_content = fs.read_plan(&PathBuf::from(plan_path))?;
    let plan = Plan::from_toml(&toml_content)?;
    let resolved = plan.resolve_tasks()?;
    let _ = validate_dag(&resolved)?;

    let diagnostics = lint::lint_plan(&plan, &resolved);
    if diagnostics.is_empty() {
        return Ok("Lint: no issues found.".to_string());
    }

    let summary = lint::summarize(&diagnostics);
    let mut lines = vec![format!("Lint: {}", lint::format_summary(&summary))];
    for d in &diagnostics {
        lines.push(format!("  [{}] {}: {}", d.severity, d.rule, d.message));
    }
    Ok(lines.join("\n"))
}

fn tool_read_progress(args: &Value) -> Result<String> {
    let progress_path = args
        .get("progress_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WiggumError::Validation("progress_path is required".to_string()))?;

    let fs = FsAdapter;
    let content = fs.read_progress(&PathBuf::from(progress_path))?;

    // Parse progress status from markdown
    let mut statuses = Vec::new();
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("| T") {
            let status = if line.contains("`[x]`") {
                "completed"
            } else if line.contains("`[~]`") {
                "in-progress"
            } else if line.contains("`[!]`") {
                "blocked"
            } else {
                "not-started"
            };
            // Extract task identifier
            if let Some(title_end) = rest.find(" | ") {
                let task_id = &rest[..title_end];
                statuses.push(format!("T{task_id}: {status}"));
            }
        }
    }

    Ok(statuses.join("\n"))
}

fn tool_update_progress(args: &Value) -> Result<String> {
    let progress_path = args
        .get("progress_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WiggumError::Validation("progress_path is required".to_string()))?;

    let task_number_u64 = args
        .get("task_number")
        .and_then(Value::as_u64)
        .ok_or_else(|| WiggumError::Validation("task_number is required".to_string()))?;
    let task_number = u32::try_from(task_number_u64)
        .map_err(|_| WiggumError::Validation("task_number too large".to_string()))?;

    let status = args
        .get("status")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WiggumError::Validation("status is required".to_string()))?;

    let notes = args.get("notes").and_then(|v| v.as_str()).unwrap_or("");

    let status_marker = match status {
        "not-started" => "`[ ]`",
        "in-progress" => "`[~]`",
        "completed" => "`[x]`",
        "blocked" => "`[!]`",
        _ => return Err(WiggumError::Validation(format!("Invalid status: {status}"))),
    };

    let task_prefix = format!("T{task_number:02}");
    let fs = FsAdapter;
    let path = PathBuf::from(progress_path);
    let content = fs.read_progress(&path)?;

    let mut updated = false;
    let new_content: String = content
        .lines()
        .map(|line| {
            if line.contains(&format!("| {task_prefix} "))
                || line.contains(&format!("| {task_prefix} —"))
            {
                updated = true;
                // Replace the status marker and notes
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 4 {
                    parts.get(1).map_or_else(
                        || line.to_string(),
                        |task_col| {
                            let notes_text = if notes.is_empty() {
                                String::new()
                            } else {
                                format!(" {notes}")
                            };
                            format!("|{task_col}| {status_marker} |{notes_text} |")
                        },
                    )
                } else {
                    line.to_string()
                }
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !updated {
        return Err(WiggumError::Validation(format!(
            "Task {task_prefix} not found in progress file"
        )));
    }

    fs.write_progress(&path, &new_content)?;
    Ok(format!("Updated {task_prefix} to {status}"))
}

fn tool_list_templates() -> String {
    "Available language defaults:\n  \
        - rust: cargo build/test/clippy\n  \
        - go: go build/test/vet + golangci-lint\n  \
        - typescript: tsc/vitest/eslint\n\n\
        Architecture styles: hexagonal, layered, modular, flat"
        .to_string()
}

fn tool_report(args: &Value) -> Result<String> {
    let progress_path = args
        .get("progress_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WiggumError::Validation("progress_path is required".to_string()))?;

    let content = std::fs::read_to_string(progress_path)?;
    let project_dir = args
        .get("project_dir")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);

    let report = crate::domain::report::generate_report(&content, project_dir.as_deref());
    Ok(crate::domain::report::format_report(&report))
}

fn tool_generate_agents_md(args: &Value) -> Result<String> {
    let plan_path = args
        .get("plan_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WiggumError::Validation("plan_path is required".to_string()))?;

    let fs = FsAdapter;
    let toml_content = fs.read_plan(&PathBuf::from(plan_path))?;
    let plan = Plan::from_toml(&toml_content)?;

    let content = generation::agents_md::render(&plan)?;
    let project_path = PathBuf::from(&plan.project.path);
    let output_path = project_path.join("AGENTS.md");

    std::fs::write(&output_path, &content)?;

    Ok(format!("Generated AGENTS.md at {}", output_path.display()))
}

fn tool_bootstrap(args: &Value) -> Result<String> {
    let project_path = args
        .get("project_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WiggumError::Validation("project_path is required".to_string()))?;

    let output_path = args.get("output_path").and_then(|v| v.as_str());
    let force = args.get("force").and_then(Value::as_bool).unwrap_or(false);

    let project = PathBuf::from(project_path);
    let output = output_path.map(PathBuf::from);

    let scan = super::bootstrap::scan_project(&project)?;
    let plan = super::bootstrap::build_plan_from_scan(&scan, &project);
    let toml_content =
        toml::to_string_pretty(&plan).map_err(|e| WiggumError::Validation(e.to_string()))?;

    let plan_path = output.unwrap_or_else(|| project.join("plan.toml"));

    if plan_path.exists() && !force {
        return Err(WiggumError::Validation(format!(
            "{} already exists (set force=true to overwrite)",
            plan_path.display()
        )));
    }

    let content = format!(
        "# Generated by wiggum bootstrap — fill in phases and tasks below\n\n{toml_content}"
    );
    std::fs::write(&plan_path, content)?;

    Ok(format!(
        "Bootstrapped plan from {} → {}\n  Language: {}\n  Name: {}\n  Architecture: {}",
        project_path,
        plan_path.display(),
        scan.language,
        scan.name,
        scan.architecture.as_deref().unwrap_or("flat")
    ))
}

// ─── Response helpers ───────────────────────────────────────────────

fn success_response(id: Value, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: Value, code: i32, message: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_string(),
            data: None,
        }),
    }
}
