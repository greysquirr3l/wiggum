use std::path::{Path, PathBuf};

use dialoguer::{Confirm, Input, MultiSelect, Select};

use crate::domain::plan::{
    Language, Orchestrator, Phase, Plan, Preflight, Project, Strategy, TaskDef,
};
use crate::error::{Result, WiggumError};

/// Run the interactive plan creation wizard.
/// Returns the created Plan and the path where the TOML was saved.
///
/// # Errors
///
/// Returns an error if interactive prompts fail or the plan file cannot be written.
pub fn run_init(output_plan: Option<&Path>) -> Result<(Plan, PathBuf)> {
    println!("🦝 wiggum init — interactive plan builder\n");

    let project = prompt_project()?;
    let language = project.language;
    let (persona, rules) = prompt_orchestrator(language)?;
    let strategy = prompt_strategy()?;

    let use_coraline = Confirm::new()
        .with_prompt("Enable Coraline code intelligence for subagents?")
        .default(true)
        .interact()
        .map_err(|e| WiggumError::Validation(e.to_string()))?;

    let phases = prompt_phases()?;

    // ── Build plan ──────────────────────────────────────────────────
    let mut plan = Plan {
        project,
        preflight: Preflight::default(),
        orchestrator: Orchestrator {
            persona,
            strategy,
            rules,
        },
        phases,
    };
    plan.preflight = plan.preflight.with_defaults(language);

    if use_coraline {
        plan.orchestrator
            .rules
            .push("Use coraline_* MCP tools for code navigation when available".to_string());
    }

    let plan_path = output_plan.map_or_else(|| PathBuf::from("plan.toml"), Path::to_path_buf);

    let toml_content = serialize_plan_toml(&plan)?;
    std::fs::write(&plan_path, &toml_content)?;

    println!("\n✅ Plan saved to {}", plan_path.display());

    if use_coraline {
        maybe_install_coraline();
    }

    Ok((plan, plan_path))
}

fn prompt_project() -> Result<Project> {
    let name: String = Input::new()
        .with_prompt("Project name")
        .interact_text()
        .map_err(|e| WiggumError::Validation(e.to_string()))?;

    let description: String = Input::new()
        .with_prompt("Project description")
        .interact_text()
        .map_err(|e| WiggumError::Validation(e.to_string()))?;

    let languages: Vec<String> = Language::ALL.iter().map(ToString::to_string).collect();
    let lang_items: Vec<&str> = languages.iter().map(String::as_str).collect();
    let lang_idx = Select::new()
        .with_prompt("Language")
        .items(&lang_items)
        .default(0)
        .interact()
        .map_err(|e| WiggumError::Validation(e.to_string()))?;
    let language = Language::ALL
        .get(lang_idx)
        .copied()
        .unwrap_or(Language::Rust);

    let path: String = Input::new()
        .with_prompt("Project path (absolute)")
        .default(
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .to_string_lossy()
                .to_string(),
        )
        .interact_text()
        .map_err(|e| WiggumError::Validation(e.to_string()))?;

    let architectures = ["hexagonal", "layered", "modular", "flat", "none"];
    let arch_idx = Select::new()
        .with_prompt("Architecture style")
        .items(architectures)
        .default(0)
        .interact()
        .map_err(|e| WiggumError::Validation(e.to_string()))?;
    let architecture = match architectures.get(arch_idx) {
        Some(&"none") | None => None,
        Some(val) => Some((*val).to_string()),
    };

    Ok(Project {
        name,
        description,
        language,
        path,
        architecture,
    })
}

fn prompt_orchestrator(language: Language) -> Result<(String, Vec<String>)> {
    let persona: String = Input::new()
        .with_prompt("Subagent persona")
        .default(format!("You are a senior {language} software engineer"))
        .interact_text()
        .map_err(|e| WiggumError::Validation(e.to_string()))?;

    let mut rules: Vec<String> = Vec::new();
    println!("\nAdd project rules (empty line to finish):");
    loop {
        let rule: String = Input::new()
            .with_prompt("  Rule")
            .allow_empty(true)
            .interact_text()
            .map_err(|e| WiggumError::Validation(e.to_string()))?;
        if rule.is_empty() {
            break;
        }
        rules.push(rule);
    }

    Ok((persona, rules))
}

fn prompt_strategy() -> Result<Strategy> {
    let strategies = ["standard", "tdd", "gsd"];
    let idx = Select::new()
        .with_prompt("Prompt strategy")
        .items(strategies)
        .default(0)
        .interact()
        .map_err(|e| WiggumError::Validation(e.to_string()))?;
    Ok(match strategies.get(idx) {
        Some(&"tdd") => Strategy::Tdd,
        Some(&"gsd") => Strategy::Gsd,
        _ => Strategy::Standard,
    })
}

fn prompt_phases() -> Result<Vec<Phase>> {
    let mut phases: Vec<Phase> = Vec::new();
    let mut order = 1u32;

    println!("\n── Define phases and tasks ──");
    println!("  (Add at least one phase with at least one task)\n");

    loop {
        let phase_name: String = Input::new()
            .with_prompt(format!("Phase {order} name"))
            .interact_text()
            .map_err(|e| WiggumError::Validation(e.to_string()))?;

        let tasks = prompt_tasks_for_phase(&phase_name, &phases)?;

        phases.push(Phase {
            name: phase_name,
            order,
            tasks,
        });
        order += 1;

        let more_phases = Confirm::new()
            .with_prompt("Add another phase?")
            .default(false)
            .interact()
            .map_err(|e| WiggumError::Validation(e.to_string()))?;
        if !more_phases {
            break;
        }
    }

    Ok(phases)
}

fn prompt_tasks_for_phase(phase_name: &str, existing_phases: &[Phase]) -> Result<Vec<TaskDef>> {
    let mut tasks: Vec<TaskDef> = Vec::new();
    let mut task_num = 1u32;

    loop {
        println!("  Task {task_num} in phase \"{phase_name}\":");

        let slug: String = Input::new()
            .with_prompt("    Slug (kebab-case)")
            .interact_text()
            .map_err(|e| WiggumError::Validation(e.to_string()))?;

        let title: String = Input::new()
            .with_prompt("    Title")
            .interact_text()
            .map_err(|e| WiggumError::Validation(e.to_string()))?;

        let goal: String = Input::new()
            .with_prompt("    Goal")
            .interact_text()
            .map_err(|e| WiggumError::Validation(e.to_string()))?;

        let all_slugs: Vec<String> = existing_phases
            .iter()
            .flat_map(|p| p.tasks.iter().map(|t| t.slug.clone()))
            .chain(tasks.iter().map(|t| t.slug.clone()))
            .collect();

        let depends_on = select_dependencies(&all_slugs)?;

        tasks.push(TaskDef {
            slug,
            title,
            goal,
            depends_on,
            hints: Vec::new(),
            test_hints: Vec::new(),
            must_haves: Vec::new(),
            gate: None,
        });

        task_num += 1;

        let more = Confirm::new()
            .with_prompt("    Add another task to this phase?")
            .default(false)
            .interact()
            .map_err(|e| WiggumError::Validation(e.to_string()))?;
        if !more {
            break;
        }
    }

    Ok(tasks)
}

fn select_dependencies(all_slugs: &[String]) -> Result<Vec<String>> {
    if all_slugs.is_empty() {
        return Ok(Vec::new());
    }

    let selected = MultiSelect::new()
        .with_prompt("    Dependencies (space to toggle, enter to confirm)")
        .items(all_slugs)
        .interact()
        .map_err(|e| WiggumError::Validation(e.to_string()))?;

    Ok(selected
        .into_iter()
        .filter_map(|i| all_slugs.get(i).cloned())
        .collect())
}

/// Serialize a Plan to pretty TOML string.
fn serialize_plan_toml(plan: &Plan) -> Result<String> {
    toml::to_string_pretty(plan).map_err(|e| WiggumError::Validation(e.to_string()))
}

/// Check if coraline is installed; offer to install from crates.io if not.
fn maybe_install_coraline() {
    let coraline_path = dirs_home().map_or_else(
        || PathBuf::from("coraline"),
        |h| h.join(".cargo/bin/coraline"),
    );

    if coraline_path.exists() {
        println!("✅ Coraline found at {}", coraline_path.display());
        return;
    }

    if which_coraline().is_some() {
        println!("✅ Coraline found in PATH");
        return;
    }

    println!("\n⚠️  Coraline not found.");
    let install = Confirm::new()
        .with_prompt("Install coraline from crates.io? (cargo install coraline)")
        .default(true)
        .interact()
        .unwrap_or(false);

    if install {
        println!("📦 Installing coraline...");
        let status = std::process::Command::new("cargo")
            .args(["install", "coraline"])
            .status();
        match status {
            Ok(s) if s.success() => println!("✅ Coraline installed successfully"),
            Ok(s) => println!("⚠️  cargo install exited with: {s}"),
            Err(e) => println!("⚠️  Failed to run cargo install: {e}"),
        }
    } else {
        println!("  Skipping. Install later with: cargo install coraline");
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn which_coraline() -> Option<PathBuf> {
    let cmd = if cfg!(windows) { "where" } else { "which" };
    std::process::Command::new(cmd)
        .arg("coraline")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let p = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if p.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(p))
                }
            } else {
                None
            }
        })
}
