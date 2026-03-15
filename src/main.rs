use std::path::{Path, PathBuf};
use std::process;

use clap::Parser;
use tracing::{error, info};

use wiggum::adapters::cli::{Cli, Command, TemplatesCmd};
use wiggum::adapters::fs::FsAdapter;
use wiggum::adapters::mcp;
use wiggum::adapters::vcs;
use wiggum::adapters::{bootstrap, diff, init, resume, retro, split, templates};
use wiggum::domain::dag::{parallel_groups, validate_dag};
use wiggum::domain::lint;
use wiggum::domain::plan::Plan;
use wiggum::domain::pricing::PricingData;
use wiggum::generation;
use wiggum::ports::PlanReader;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Command::Init { plan } => cmd_init(plan.as_deref()),
        Command::AddTask { plan } => cmd_add_task(&plan),
        Command::Generate {
            plan,
            output,
            force: _force,
            dry_run,
            estimate_tokens,
            skip_agents_md,
        } => cmd_generate(
            &plan,
            output.as_deref(),
            dry_run,
            estimate_tokens,
            skip_agents_md,
        ),
        Command::Validate { plan, lint } => cmd_validate(&plan, lint),
        Command::Serve { mcp: true } => {
            info!("Starting wiggum MCP server (stdio)");
            mcp::run_mcp_server()
        }
        Command::Serve { mcp: false } => {
            error!("Use --mcp flag to start the MCP server");
            process::exit(1);
        }
        Command::Report {
            progress,
            project_dir,
        } => cmd_report(&progress, project_dir.as_deref()),
        Command::Watch { progress, poll_ms } => cmd_watch(&progress, poll_ms),
        Command::Bootstrap {
            path,
            output,
            force,
        } => cmd_bootstrap(&path, output.as_deref(), force),
        Command::Clean {
            plan,
            output,
            dry_run,
        } => cmd_clean(&plan, output.as_deref(), dry_run),
        Command::Resume {
            progress,
            plan,
            task,
            dry_run,
        } => cmd_resume(&progress, &plan, task.as_deref(), dry_run),
        Command::Diff { old, new } => cmd_diff(&old, &new),
        Command::Retro {
            progress,
            apply,
            plan,
        } => cmd_retro(&progress, apply, &plan),
        Command::Split { plan, task, into } => cmd_split(&plan, &task, into),
        Command::Templates(sub) => cmd_templates(sub),
        Command::Prices { update } => cmd_prices(update),
    };

    if let Err(e) = result {
        error!("{e}");
        process::exit(1);
    }
}

fn cmd_init(plan_path: Option<&Path>) -> wiggum::error::Result<()> {
    let (_plan, path) = init::run_init(plan_path)?;
    println!(
        "\nRun `wiggum generate {}` to generate scaffold artifacts.",
        path.display()
    );
    Ok(())
}

fn cmd_add_task(plan_path: &Path) -> wiggum::error::Result<()> {
    wiggum::adapters::add_task::run_add_task(plan_path)
}

fn cmd_generate(
    plan_path: &Path,
    output_override: Option<&std::path::Path>,
    dry_run: bool,
    estimate_tokens: bool,
    skip_agents_md: bool,
) -> wiggum::error::Result<()> {
    let fs = FsAdapter;
    let toml_content = fs.read_plan(plan_path)?;
    let plan = Plan::from_toml(&toml_content)?;

    // Validate first
    let resolved = plan.resolve_tasks()?;
    let sorted = validate_dag(&resolved)?;
    info!(
        "Plan validated: {} phases, {} tasks",
        plan.phases.len(),
        resolved.len()
    );
    info!("Execution order: {}", sorted.join(" → "));

    // Generate (with user template overrides if present)
    let project_path =
        output_override.map_or_else(|| PathBuf::from(&plan.project.path), Path::to_path_buf);

    // VCS pre-check: warn if target has uncommitted changes
    if !dry_run && let vcs::VcsStatus::Dirty(status) = vcs::check_vcs_status(&project_path) {
        println!("⚠️  Target directory has uncommitted changes:");
        for line in status.lines().take(5) {
            println!("   {line}");
        }
        println!("   Consider committing first. (use --force to skip this warning)");
        println!();
    }

    let mut artifacts = generation::generate_all_with_overrides(&plan, &project_path)?;

    if skip_agents_md {
        artifacts.agents_md = None;
    }

    if dry_run {
        println!("Dry run — would generate:\n");
        #[allow(clippy::cast_precision_loss)]
        {
            println!(
                "  PROGRESS.md             ({:.1} KB)",
                artifacts.progress.len() as f64 / 1024.0
            );
            println!(
                "  IMPLEMENTATION_PLAN.md  ({:.1} KB)",
                artifacts.plan_doc.len() as f64 / 1024.0
            );
            println!(
                "  .vscode/orchestrator.prompt.md  ({:.1} KB)",
                artifacts.orchestrator.len() as f64 / 1024.0
            );
            println!("  tasks/");
            for (filename, content) in &artifacts.tasks {
                println!("    {filename}  ({:.1} KB)", content.len() as f64 / 1024.0);
            }
            if let Some(agents) = &artifacts.agents_md {
                println!(
                    "  AGENTS.md               ({:.1} KB)",
                    agents.len() as f64 / 1024.0
                );
            }
            let total_size = artifacts.progress.len()
                + artifacts.plan_doc.len()
                + artifacts.orchestrator.len()
                + artifacts.tasks.iter().map(|(_, c)| c.len()).sum::<usize>()
                + artifacts.agents_md.as_ref().map_or(0, String::len);
            let total_files =
                3 + artifacts.tasks.len() + usize::from(artifacts.agents_md.is_some());
            println!(
                "\n  Total: {} files, {:.1} KB",
                total_files,
                total_size as f64 / 1024.0
            );
        }
        println!("  Target: {}", project_path.display());
        println!("  DAG: valid (no cycles)");
        let groups = parallel_groups(&resolved)?;
        println!("  Parallelizable groups: {}", groups.len());
        for (i, group) in groups.iter().enumerate() {
            println!("    Group {}: {}", i + 1, group.join(", "));
        }
        if estimate_tokens {
            println!();
            println!("{}", generation::tokens::format_report(&artifacts));
        }
        return Ok(());
    }

    if estimate_tokens {
        println!();
        println!("{}", generation::tokens::format_report(&artifacts));
        println!();
    }

    generation::write_artifacts(&fs, &project_path, &artifacts)?;

    println!("✅ Generated scaffold in {}", project_path.display());
    println!("   📋 PROGRESS.md");
    println!("   📄 IMPLEMENTATION_PLAN.md");
    println!("   🤖 .vscode/orchestrator.prompt.md");
    println!("   📁 tasks/ ({} task files)", artifacts.tasks.len());
    if artifacts.agents_md.is_some() {
        println!("   📝 AGENTS.md");
    }
    println!();
    println!("Next steps:");
    println!("  1. Review and enrich the task files in tasks/");
    println!("     - Add implementation guidance, code snippets, test specs");
    println!("     - Fill in the <!-- TODO --> sections");
    println!("  2. Open the project in VS Code with Copilot Agent mode");
    println!("  3. Run the orchestrator prompt to start the loop");

    Ok(())
}

fn cmd_validate(plan_path: &Path, run_lint: bool) -> wiggum::error::Result<()> {
    let fs = FsAdapter;
    let toml_content = fs.read_plan(plan_path)?;
    let plan = Plan::from_toml(&toml_content)?;

    let resolved = plan.resolve_tasks()?;
    let sorted = validate_dag(&resolved)?;

    println!("✅ Plan is valid");
    println!("   Phases: {}", plan.phases.len());
    println!("   Tasks:  {}", resolved.len());
    println!("   Order:  {}", sorted.join(" → "));

    let groups = parallel_groups(&resolved)?;
    if groups.len() < resolved.len() {
        println!("   Parallelizable groups: {}", groups.len());
        for (i, group) in groups.iter().enumerate() {
            if group.len() > 1 {
                println!("     Group {}: {} (concurrent)", i + 1, group.join(", "));
            }
        }
    }

    if run_lint {
        let diagnostics = lint::lint_plan(&plan, &resolved);
        if diagnostics.is_empty() {
            println!("\n✅ Lint: no issues");
        } else {
            let summary = lint::summarize(&diagnostics);
            println!("\nLint: {}", lint::format_summary(&summary));
            for d in &diagnostics {
                println!("{d}");
            }
        }
    }

    Ok(())
}

fn cmd_report(progress_path: &Path, project_dir: Option<&Path>) -> wiggum::error::Result<()> {
    let content = std::fs::read_to_string(progress_path)?;

    let report = wiggum::domain::report::generate_report(&content, project_dir);
    println!("{}", wiggum::domain::report::format_report(&report));

    Ok(())
}

fn cmd_watch(progress_path: &Path, poll_ms: u64) -> wiggum::error::Result<()> {
    wiggum::adapters::watch::run_watch(progress_path, poll_ms)
}

fn cmd_bootstrap(
    project_path: &Path,
    output: Option<&Path>,
    force: bool,
) -> wiggum::error::Result<()> {
    bootstrap::run_bootstrap(project_path, output, force)?;
    Ok(())
}

fn cmd_clean(
    plan_path: &Path,
    output_override: Option<&Path>,
    dry_run: bool,
) -> wiggum::error::Result<()> {
    let fs = FsAdapter;
    let toml_content = fs.read_plan(plan_path)?;
    let plan = Plan::from_toml(&toml_content)?;

    let project_path =
        output_override.map_or_else(|| PathBuf::from(&plan.project.path), Path::to_path_buf);

    if dry_run {
        let targets = generation::clean::collect_targets(&plan, &project_path)?;
        let existing: Vec<_> = targets.iter().filter(|p| p.exists()).collect();
        if existing.is_empty() {
            println!("Nothing to clean in {}", project_path.display());
        } else {
            println!("Dry run — would remove:\n");
            for path in &existing {
                let relative = path.strip_prefix(&project_path).unwrap_or(path);
                if path.is_dir() {
                    println!("  📁 {}/", relative.display());
                } else {
                    println!("  🗑  {}", relative.display());
                }
            }
            println!(
                "\n  Total: {} item(s) in {}",
                existing.len(),
                project_path.display()
            );
        }
        return Ok(());
    }

    let removed = generation::clean::remove_artifacts(&plan, &project_path)?;

    if removed.is_empty() {
        println!("Nothing to clean in {}", project_path.display());
    } else {
        println!(
            "🧹 Cleaned {} item(s) from {}",
            removed.len(),
            project_path.display()
        );
        for path in &removed {
            let relative = path.strip_prefix(&project_path).unwrap_or(path);
            println!("   ✕ {}", relative.display());
        }
    }

    Ok(())
}

fn cmd_resume(
    progress_path: &Path,
    plan_path: &Path,
    task_override: Option<&str>,
    dry_run: bool,
) -> wiggum::error::Result<()> {
    let ctx = resume::find_resume_task(progress_path, plan_path, task_override)?;

    println!("{}", resume::format_resume_info(&ctx, dry_run));

    if !dry_run {
        println!();
        println!("{}", "─".repeat(60));
        println!("{}", ctx.prompt);
    }

    Ok(())
}

fn cmd_diff(old_path: &Path, new_path: &Path) -> wiggum::error::Result<()> {
    let changes = diff::diff_plans(old_path, new_path)?;
    println!("{}", diff::format_diff(&changes));
    Ok(())
}

fn cmd_retro(progress_path: &Path, _apply: bool, _plan_path: &Path) -> wiggum::error::Result<()> {
    let summary = retro::analyze_progress(progress_path)?;
    println!("{}", retro::format_retro(&summary));

    // TODO: Implement --apply flag to patch plan.toml
    // if apply && !summary.suggestions.is_empty() {
    //     println!("\nSave suggestions to plan-retro.toml? [Y/n]:");
    // }

    Ok(())
}

fn cmd_split(plan_path: &Path, task_slug: &str, into: Option<u32>) -> wiggum::error::Result<()> {
    let analysis = split::analyze_task(plan_path, task_slug)?;

    if let Some(n) = into {
        // Non-interactive mode
        println!("{}", split::format_split_preview(&analysis));
        if n < 2 {
            println!("\n⚠️  Cannot split into fewer than 2 tasks");
            return Ok(());
        }
        println!("\nNon-interactive split into {n} tasks not yet implemented.");
        println!("Use interactive mode: `wiggum split --task {task_slug}`");
    } else {
        // Interactive mode
        let split_plan = split::run_interactive_split(plan_path, task_slug)?;

        println!("\nPreview changes? [Y/n]: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().to_lowercase().starts_with('n') {
            println!("\nWould create:");
            for (i, part) in split_plan.parts.iter().enumerate() {
                println!("  {}. {} — {}", i + 1, part.slug, part.goal);
            }
            if split_plan.rewire_dependents {
                println!("  (Would rewire dependents to last task)");
            }
        }

        println!("\nApply? [Y/n]: ");
        input.clear();
        std::io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase().starts_with('n') {
            println!("Cancelled.");
        } else {
            split::apply_split(plan_path, &split_plan)?;
            println!(
                "✅ plan.toml updated. Run `wiggum validate {}` to verify.",
                plan_path.display()
            );
        }
    }

    Ok(())
}

fn cmd_templates(cmd: TemplatesCmd) -> wiggum::error::Result<()> {
    match cmd {
        TemplatesCmd::List => {
            let list = templates::list_templates()?;
            println!("{}", templates::format_template_list(&list));
        }
        TemplatesCmd::Show { name } => {
            let tmpl = templates::load_template(&name)?;
            println!("{}", templates::format_template_show(&tmpl));
        }
        TemplatesCmd::Save { plan, task, name } => {
            let path = templates::save_template(&plan, &task, name.as_deref())?;
            println!("✅ Template saved to {}", path.display());
        }
    }
    Ok(())
}

#[allow(clippy::unnecessary_wraps)] // May add error cases in the future
fn cmd_prices(update: bool) -> wiggum::error::Result<()> {
    let data = PricingData::bundled();

    if update {
        println!("⚠️  Online price updates not yet implemented.");
        println!(
            "Using bundled prices (last updated: {}).",
            data.last_updated
        );
        println!();
    }

    println!("Model pricing (per 1M tokens):\n");
    println!("  {:<24} {:>10} {:>10}", "Model", "Input", "Output");
    println!("  {}", "─".repeat(46));
    for model in &data.models {
        println!(
            "  {:<24} {:>9.2}$ {:>9.2}$",
            model.name, model.input_per_m, model.output_per_m
        );
    }
    println!("\n  Last updated: {}", data.last_updated);

    Ok(())
}
