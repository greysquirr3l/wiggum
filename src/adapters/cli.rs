use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "wiggum",
    about = "AI orchestration scaffold generator for the Ralph Wiggum loop",
    version
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

        /// Overwrite existing files without prompting
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
