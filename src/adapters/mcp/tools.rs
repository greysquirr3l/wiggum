//! The wiggum MCP tools.
//!
//! Each tool is an associated function on [`WiggumTools`], marked with
//! `#[protocol_droid::tool]`. The macro derives the `inputSchema` from the
//! Rust signature, so the schema a peer sees and the arguments the handler
//! reads cannot drift apart — the previous hand-written `input_schema` JSON
//! was a second description of the same thing, and it could disagree with the
//! code that read the arguments.
//!
//! Every tool returns [`Result<String>`]. protocol-droid lowers the `Ok` into
//! a text content block and the `Err` into a failed tool result, so an error
//! reaches the peer as tool output rather than as a protocol-level failure —
//! which is what a client can act on.
//!
//! # Guardrails
//!
//! A tool body is the domain work and nothing else. The three guardrails
//! live in [`super::guardrail`] and are applied around every call by
//! [`build_server`], so a tool cannot forget one and no tool body has to
//! carry the boilerplate.
//!
//! # Why `build_server` lives here
//!
//! `#[protocol_droid::server]` expands into module-scope items — the schema
//! factory and the handler factory for each tool — in the module the `impl`
//! block is written in. Those items are private, so a sibling module cannot
//! name them. Building the server here rather than in the parent keeps the
//! generated names reachable, and keeps the one list of tools next to the
//! tools themselves.

use std::path::PathBuf;

use protocol_droid::application::server::Server;

use crate::adapters::fs::FsAdapter;
use crate::domain::dag::validate_dag;
use crate::domain::lint;
use crate::domain::plan::Plan;
use crate::domain::targets::TargetSet;
use crate::error::{Result, WiggumError};
use crate::generation;
use crate::ports::{PlanReader, ProgressStore};

/// The wiggum MCP server.
///
/// The struct exists to give `#[protocol_droid::server]` an `impl` block to
/// hang the tools off; it carries no state, because every tool is a
/// function of its arguments.
pub struct WiggumTools;

/// Build the wiggum MCP server, with every tool behind the guardrails.
///
/// The server is assembled by hand rather than through
/// `WiggumTools::into_server()` for one reason: `into_server()` never
/// reaches `ServerBuilder::version`, so a peer would be told the server's
/// version is whichever version of protocol-droid happens to be linked
/// against rather than wiggum's own. A peer asking "which wiggum am I
/// talking to" is asking about wiggum.
///
/// # Errors
///
/// When a tool's input schema cannot be built. The schema is derived from
/// the Rust signature, so this is a mistake in an argument type rather than
/// anything a peer can cause.
pub fn build_server() -> std::result::Result<Server, protocol_droid::domain::ir::JsonObjectError> {
    let mut builder = Server::builder("wiggum").version(env!("CARGO_PKG_VERSION"));

    builder = builder.tool(
        __WiggumTools_wiggum_version_TOOL()?,
        super::guarded(
            "wiggum_version",
            __WiggumTools_wiggum_version_HANDLER(WiggumTools::wiggum_version),
        ),
    );
    builder = builder.tool(
        __WiggumTools_wiggum_generate_plan_TOOL()?,
        super::guarded(
            "wiggum_generate_plan",
            __WiggumTools_wiggum_generate_plan_HANDLER(WiggumTools::wiggum_generate_plan),
        ),
    );
    builder = builder.tool(
        __WiggumTools_wiggum_validate_plan_TOOL()?,
        super::guarded(
            "wiggum_validate_plan",
            __WiggumTools_wiggum_validate_plan_HANDLER(WiggumTools::wiggum_validate_plan),
        ),
    );
    builder = builder.tool(
        __WiggumTools_wiggum_lint_plan_TOOL()?,
        super::guarded(
            "wiggum_lint_plan",
            __WiggumTools_wiggum_lint_plan_HANDLER(WiggumTools::wiggum_lint_plan),
        ),
    );
    builder = builder.tool(
        __WiggumTools_wiggum_check_plan_TOOL()?,
        super::guarded(
            "wiggum_check_plan",
            __WiggumTools_wiggum_check_plan_HANDLER(WiggumTools::wiggum_check_plan),
        ),
    );
    builder = builder.tool(
        __WiggumTools_wiggum_read_progress_TOOL()?,
        super::guarded(
            "wiggum_read_progress",
            __WiggumTools_wiggum_read_progress_HANDLER(WiggumTools::wiggum_read_progress),
        ),
    );
    builder = builder.tool(
        __WiggumTools_wiggum_update_progress_TOOL()?,
        super::guarded(
            "wiggum_update_progress",
            __WiggumTools_wiggum_update_progress_HANDLER(WiggumTools::wiggum_update_progress),
        ),
    );
    builder = builder.tool(
        __WiggumTools_wiggum_list_templates_TOOL()?,
        super::guarded(
            "wiggum_list_templates",
            __WiggumTools_wiggum_list_templates_HANDLER(WiggumTools::wiggum_list_templates),
        ),
    );
    builder = builder.tool(
        __WiggumTools_wiggum_report_TOOL()?,
        super::guarded(
            "wiggum_report",
            __WiggumTools_wiggum_report_HANDLER(WiggumTools::wiggum_report),
        ),
    );
    builder = builder.tool(
        __WiggumTools_wiggum_generate_agents_md_TOOL()?,
        super::guarded(
            "wiggum_generate_agents_md",
            __WiggumTools_wiggum_generate_agents_md_HANDLER(WiggumTools::wiggum_generate_agents_md),
        ),
    );
    builder = builder.tool(
        __WiggumTools_wiggum_bootstrap_TOOL()?,
        super::guarded(
            "wiggum_bootstrap",
            __WiggumTools_wiggum_bootstrap_HANDLER(WiggumTools::wiggum_bootstrap),
        ),
    );
    builder = builder.tool(
        __WiggumTools_wiggum_draft_plan_TOOL()?,
        super::guarded(
            "wiggum_draft_plan",
            __WiggumTools_wiggum_draft_plan_HANDLER(WiggumTools::wiggum_draft_plan),
        ),
    );

    Ok(builder.build())
}

/// The range of MCP revisions this server can be negotiated onto.
///
/// The upper bound is the newest revision with a handshake defined, not the
/// newest revision protocol-droid knows: SEP-2575 took `initialize` out of
/// `2026-07-28`, so that revision is reached by a peer naming its own
/// version rather than by the handshake, and it is not one this server
/// negotiates.
///
/// The lower bound is the oldest revision protocol-droid has wire types for,
/// which is also the oldest a peer can be answered with.
///
/// The set is never empty, so this cannot fail; the `Result` on
/// `SupportedVersions::all` is a formality the crate keeps so callers do not
/// have to spell the list out.
fn negotiable_protocol_range() -> String {
    let Ok(versions) = protocol_droid::domain::SupportedVersions::all() else {
        return "unknown".to_string();
    };
    format!(
        "{} through {}",
        versions.earliest().as_wire(),
        versions.latest_handshake().as_wire()
    )
}

// Every tool is an `async fn` because `#[protocol_droid::tool]` requires
// one: the generated handler awaits the future the tool returns, and a plain
// `fn` answers with a value that has nothing to await. Most of these bodies
// do their work synchronously, so clippy sees an `async` with no `.await`
// in it — which is the macro's requirement, not an oversight here.
#[expect(
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    reason = "the #[tool] macro requires an `async fn`; the bodies are synchronous"
)]
#[protocol_droid::server(name = "wiggum")]
impl WiggumTools {
    /// Return wiggum version metadata (package, git SHA, MCP protocol).
    ///
    /// The protocol line names the *range* of revisions this server can be
    /// negotiated onto, not one revision, because the revision in use is
    /// settled between the server and the peer and the server does not get to
    /// choose it. A peer that wants to know whether wiggum can talk to it
    /// needs the range; a peer that wants to know which revision it settled
    /// on already has it — it is the `protocolVersion` in the handshake
    /// result.
    ///
    /// The range is read from protocol-droid rather than written here, so it
    /// moves when protocol-droid's set of revisions moves.
    #[protocol_droid::tool]
    async fn wiggum_version() -> Result<String> {
        let git_sha = option_env!("WIGGUM_GIT_SHA").unwrap_or("unknown");
        Ok(format!(
            "wiggum {}\nGit SHA: {git_sha}\nMCP protocol: {}",
            env!("CARGO_PKG_VERSION"),
            negotiable_protocol_range()
        ))
    }

    /// Generate full scaffold from a plan TOML file path.
    #[protocol_droid::tool]
    async fn wiggum_generate_plan(plan_path: String, target: Option<String>) -> Result<String> {
        let target = target.unwrap_or_else(|| "vscode".to_string());

        let fs = FsAdapter;
        let toml_content = fs.read_plan(&PathBuf::from(&plan_path))?;
        let plan = Plan::from_toml(&toml_content)?;
        let targets = if target.eq_ignore_ascii_case("all") {
            TargetSet::all()
        } else {
            TargetSet::from_cli_str(&target)
                .map_err(|e| WiggumError::Validation(format!("target: {e}")))?
        };
        let artifacts = generation::generate_all(&plan)?;

        let project_path = PathBuf::from(&plan.project.path);

        let vcs_warning = match crate::adapters::vcs::check_vcs_status(&project_path) {
            crate::adapters::vcs::VcsStatus::Dirty(_) => {
                "\n⚠️  Warning: target directory has uncommitted changes."
            }
            _ => "",
        };

        generation::write_artifacts(&fs, &project_path, &artifacts, &targets)?;

        Ok(format!(
            "Generated {} task files, PROGRESS.md, IMPLEMENTATION_PLAN.md, and target-specific agents in {} (targets: {}{vcs_warning})",
            artifacts.tasks.len(),
            plan.project.path,
            targets
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }

    /// Validate a plan TOML file: parse it, resolve its tasks, and check
    /// the dependency graph for cycles.
    #[protocol_droid::tool]
    async fn wiggum_validate_plan(plan_path: String) -> Result<String> {
        let fs = FsAdapter;
        let toml_content = fs.read_plan(&PathBuf::from(&plan_path))?;
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

    /// Lint a plan TOML file and report every diagnostic.
    #[protocol_droid::tool]
    async fn wiggum_lint_plan(plan_path: String) -> Result<String> {
        let fs = FsAdapter;
        let toml_content = fs.read_plan(&PathBuf::from(&plan_path))?;
        let plan = Plan::from_toml(&toml_content)?;
        let resolved = plan.resolve_tasks()?;
        let _ = validate_dag(&resolved)?;

        let diagnostics = lint::lint_plan(&plan, &resolved);
        // Anchor drift checks need filesystem access; no-op when no anchors exist.
        let mut all = diagnostics;
        let root = std::path::PathBuf::from(&plan.project.path);
        all.extend(crate::domain::anchors::lint_anchors(&resolved, &root));

        if all.is_empty() {
            return Ok("Lint: no issues found.".to_string());
        }

        let summary = lint::summarize(&all);
        let mut lines = vec![format!("Lint: {}", lint::format_summary(&summary))];
        for d in &all {
            lines.push(format!("  [{}] {}: {}", d.severity, d.rule, d.message));
        }
        Ok(lines.join("\n"))
    }

    /// Score a plan's quality on five dimensions — granularity, dependency
    /// health, coverage, richness, and token budget — and return an overall
    /// score with actionable suggestions.
    #[protocol_droid::tool]
    async fn wiggum_check_plan(plan_path: String) -> Result<String> {
        let fs = FsAdapter;
        let toml_content = fs.read_plan(&PathBuf::from(&plan_path))?;
        let plan = Plan::from_toml(&toml_content)?;
        let resolved = plan.resolve_tasks()?;

        let score = crate::domain::check::score_plan(&plan, &resolved);
        Ok(crate::domain::check::format_scorecard(&score))
    }

    /// Read a PROGRESS.md file and report each task's status.
    #[protocol_droid::tool]
    async fn wiggum_read_progress(progress_path: String) -> Result<String> {
        let fs = FsAdapter;
        let content = fs.read_progress(&PathBuf::from(&progress_path))?;

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
                if let Some(title_end) = rest.find(" | ") {
                    let task_id = &rest[..title_end];
                    statuses.push(format!("T{task_id}: {status}"));
                }
            }
        }

        Ok(statuses.join("\n"))
    }

    /// Set a task's status in a PROGRESS.md file.
    #[protocol_droid::tool]
    async fn wiggum_update_progress(
        progress_path: String,
        task_number: u32,
        status: String,
        notes: Option<String>,
    ) -> Result<String> {
        let notes = notes.unwrap_or_default();

        let status_marker = match status.as_str() {
            "not-started" => "`[ ]`",
            "in-progress" => "`[~]`",
            "completed" => "`[x]`",
            "blocked" => "`[!]`",
            other => {
                return Err(WiggumError::Validation(format!("Invalid status: {other}")));
            }
        };

        let task_prefix = format!("T{task_number:02}");
        let fs = FsAdapter;
        let path = PathBuf::from(&progress_path);
        let content = fs.read_progress(&path)?;

        let mut updated = false;
        let new_content: String = content
            .lines()
            .map(|line| {
                if line.contains(&format!("| {task_prefix} "))
                    || line.contains(&format!("| {task_prefix} —"))
                {
                    updated = true;
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

    /// List the available language defaults and architecture styles.
    #[protocol_droid::tool]
    async fn wiggum_list_templates() -> Result<String> {
        Ok("Available language defaults:\n  \
            - rust: cargo build/test/clippy\n  \
            - go: go build/test/vet + golangci-lint\n  \
            - typescript: tsc/vitest/eslint\n\n\
            Architecture styles: hexagonal, layered, modular, flat"
            .to_string())
    }

    /// Generate a progress report from a PROGRESS.md file.
    #[protocol_droid::tool]
    async fn wiggum_report(progress_path: String, project_dir: Option<String>) -> Result<String> {
        let content = std::fs::read_to_string(&progress_path)?;
        let project_dir = project_dir.map(PathBuf::from);

        let report = crate::domain::report::generate_report(&content, project_dir.as_deref());
        Ok(crate::domain::report::format_report(&report))
    }

    /// Generate an AGENTS.md file from a plan TOML.
    #[protocol_droid::tool]
    async fn wiggum_generate_agents_md(plan_path: String) -> Result<String> {
        let fs = FsAdapter;
        let toml_content = fs.read_plan(&PathBuf::from(&plan_path))?;
        let plan = Plan::from_toml(&toml_content)?;

        let content = generation::agents_md::render(&plan)?;
        let project_path = PathBuf::from(&plan.project.path);
        let output_path = project_path.join("AGENTS.md");

        std::fs::write(&output_path, &content)?;

        Ok(format!("Generated AGENTS.md at {}", output_path.display()))
    }

    /// Scan an existing project directory and generate a skeleton plan TOML.
    #[protocol_droid::tool]
    async fn wiggum_bootstrap(
        project_path: String,
        output_path: Option<String>,
        force: Option<bool>,
    ) -> Result<String> {
        let force = force.unwrap_or(false);
        let project_path = PathBuf::from(&project_path);
        let output_path = output_path.map_or_else(|| project_path.join("plan.toml"), PathBuf::from);

        let written =
            crate::adapters::bootstrap::run_bootstrap(&project_path, Some(&output_path), force)?;

        Ok(format!(
            "Scanned {} and wrote a skeleton plan to {}",
            project_path.display(),
            written.display()
        ))
    }

    /// Generate a skeleton plan.toml from a natural-language description.
    /// The calling agent fills in the task details; this tool provides the
    /// correctly-structured scaffold.
    #[protocol_droid::tool]
    async fn wiggum_draft_plan(
        project_name: String,
        description: String,
        language: Option<String>,
        task_slugs: Option<Vec<String>>,
    ) -> Result<String> {
        let language = language.unwrap_or_else(|| "rust".to_string());

        let default_slugs = [
            "scaffold",
            "domain-model",
            "core-logic",
            "api-layer",
            "persistence",
            "integration",
            "security-hardening",
            "documentation",
        ];
        let task_slugs =
            task_slugs.unwrap_or_else(|| default_slugs.iter().map(|s| (*s).to_string()).collect());

        let plan_toml = crate::domain::workspace::skeleton_plan_toml(
            &project_name,
            &description,
            &language,
            &task_slugs,
        );

        Ok(format!(
            "# wiggum skeleton plan — fill in goals, hints, and evaluation_criteria before running `wiggum generate`\n\n{plan_toml}"
        ))
    }
}
