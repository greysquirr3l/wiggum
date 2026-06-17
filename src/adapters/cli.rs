use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "wiggum",
    about = "Agentic implementation scaffold generator for dependency-aware AI coding workflows"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate all artifacts from a plan file
    Generate {
        /// Path to the plan TOML file
        plan: PathBuf,

        /// Override the output directory (defaults to project.path from the plan)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Skip VCS-dirty warning and continue generating (does not affect file overwrite behaviour)
        #[arg(long)]
        force: bool,

        /// Preview what would be generated without writing files
        #[arg(long)]
        dry_run: bool,

        /// Show estimated token counts for generated artifacts
        #[arg(long)]
        estimate_tokens: bool,

        /// Skip AGENTS.md generation
        #[arg(long)]
        skip_agents_md: bool,

        /// Override the target tool(s) to generate artifacts for.
        /// `vscode` (default), `opencode`, `claude`, or `all`.
        /// Overrides the plan-level `[targets]` section.
        #[arg(long, value_parser = ["vscode", "opencode", "claude", "all"])]
        target: Option<String>,
    },

    /// Validate a plan file without generating artifacts
    Validate {
        /// Path to the plan TOML file
        plan: PathBuf,

        /// Run lint rules to check plan quality
        #[arg(long)]
        lint: bool,
    },

    /// Interactively create a new plan file
    Init {
        /// Path to write the generated plan TOML
        #[arg(short, long)]
        plan: Option<PathBuf>,
    },

    /// Add a task to an existing plan file
    AddTask {
        /// Path to the plan TOML file
        plan: PathBuf,
    },

    /// Start the MCP server (stdio transport)
    Serve {
        /// Start in MCP mode
        #[arg(long)]
        mcp: bool,
    },

    /// Generate a post-execution report from PROGRESS.md
    Report {
        /// Path to PROGRESS.md (default: ./PROGRESS.md)
        #[arg(long, default_value = "PROGRESS.md")]
        progress: PathBuf,

        /// Project directory for git timeline (optional)
        #[arg(long)]
        project_dir: Option<PathBuf>,
    },

    /// Watch PROGRESS.md for live progress updates
    Watch {
        /// Path to PROGRESS.md (default: ./PROGRESS.md)
        #[arg(long, default_value = "PROGRESS.md")]
        progress: PathBuf,

        /// Poll interval in milliseconds
        #[arg(long, default_value = "1000")]
        poll_ms: u64,

        /// Seconds before an in-progress task triggers a stall warning (0 = disabled)
        #[arg(long, default_value = "1800")]
        stall_secs: u64,
    },

    /// Bootstrap a plan from an existing project directory
    Bootstrap {
        /// Path to the project directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Path to write the generated plan TOML (defaults to `<path>/plan.toml`)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Overwrite existing plan file without prompting
        #[arg(long)]
        force: bool,
    },

    /// Remove all wiggum-generated artifacts from a project
    Clean {
        /// Path to the plan TOML file
        plan: PathBuf,

        /// Override the target directory (defaults to project.path from the plan)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Preview what would be removed without deleting anything
        #[arg(long)]
        dry_run: bool,
    },

    /// Resume an interrupted orchestrator loop from the last in-progress task
    Resume {
        /// Path to PROGRESS.md (default: ./PROGRESS.md)
        #[arg(long, default_value = "PROGRESS.md")]
        progress: PathBuf,

        /// Path to the plan TOML file (default: ./plan.toml)
        #[arg(long, default_value = "plan.toml")]
        plan: PathBuf,

        /// Resume from a specific task instead of auto-detecting
        #[arg(long)]
        task: Option<String>,

        /// Show what would be resumed without emitting prompt
        #[arg(long)]
        dry_run: bool,
    },

    /// Compare two plan.toml files and show what changed
    Diff {
        /// Path to the original plan TOML file
        old: PathBuf,

        /// Path to the new plan TOML file
        new: PathBuf,
    },

    /// Generate retrospective suggestions from learnings in PROGRESS.md
    Retro {
        /// Path to PROGRESS.md (default: ./PROGRESS.md)
        #[arg(long, default_value = "PROGRESS.md")]
        progress: PathBuf,

        /// Auto-apply non-destructive suggestions to plan.toml
        #[arg(long)]
        apply: bool,

        /// Path to the plan TOML file (for --apply)
        #[arg(long, default_value = "plan.toml")]
        plan: PathBuf,

        /// Save extracted patterns to the global patterns store (~/.wiggum/patterns/)
        #[arg(long)]
        save: bool,
    },

    /// Split an oversized task into multiple smaller tasks
    Split {
        /// Path to the plan TOML file
        plan: PathBuf,

        /// Slug of the task to split
        #[arg(long)]
        task: String,

        /// Non-interactive mode: split into N tasks
        #[arg(long)]
        into: Option<u32>,
    },

    /// Manage reusable task templates
    #[command(subcommand)]
    Templates(TemplatesCmd),

    /// Update model pricing data
    Prices {
        /// Update prices from the latest source
        #[arg(long)]
        update: bool,
    },

    /// Score the quality of a plan file before generating
    Check {
        /// Path to the plan TOML file
        plan: PathBuf,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show version information
    Version,

    /// Re-render a single task file after a failure, injecting failure evidence as hints
    Replan {
        /// Path to the plan TOML file
        plan: PathBuf,

        /// Slug of the task to re-render
        #[arg(short, long)]
        task: String,

        /// Preview output without writing files
        #[arg(long)]
        dry_run: bool,
    },

    /// Manage reusable plan patterns from past retrospectives
    Patterns {
        #[command(subcommand)]
        action: PatternsAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum TemplatesCmd {
    /// List available templates
    List,

    /// Show details of a specific template
    Show {
        /// Template name
        name: String,
    },

    /// Save a task from the current plan as a template
    Save {
        /// Path to the plan TOML file
        #[arg(long)]
        plan: PathBuf,

        /// Slug of the task to save
        #[arg(long)]
        task: String,

        /// Template name (defaults to task slug)
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum PatternsAction {
    /// List saved patterns
    List,

    /// Save patterns from PROGRESS.md into the global store
    Save {
        /// Path to PROGRESS.md
        from: PathBuf,

        /// Path to the plan TOML file (for language detection)
        #[arg(long, default_value = "plan.toml")]
        plan: PathBuf,
    },

    /// Apply matching patterns to a plan (shows suggested hints)
    Apply {
        /// Path to the plan TOML file
        plan: PathBuf,
    },
}
