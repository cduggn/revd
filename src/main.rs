//! revd — a grounding ledger. See docs/SPEC.md.
mod analyze;
mod attrib;
mod daemon;
mod git;
mod hook;
mod ledger;
mod mcp;
mod store;
mod surface;
mod taxonomy;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "revd",
    version,
    about = "Silent background code review with agent attribution"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the background worker
    Daemon,
    /// Called by git / Claude Code hooks; enqueues and exits
    Hook { event: String },
    /// Install git hooks, Claude Code hooks, MCP server, statusline
    Install,
    /// Findings for a commit (default HEAD)
    Show { sha: Option<String> },
    /// Counts-with-denominators table
    Ledger {
        #[arg(long, default_value_t = 30)]
        days: u32,
        #[arg(long)]
        lang: Option<String>,
        #[arg(long)]
        category: Option<String>,
    },
    /// One-line statusline output
    Status,
    /// Suppress a rule from alert level
    Mute { rule: String },
    /// Dismiss a finding by id
    Dismiss { id: String },
    /// On-demand deep review
    Review {
        target: Option<String>,
        #[arg(long, default_value = "sonnet")]
        model: String,
    },
    /// stdio MCP server (pull-only agent access)
    Mcp,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Status => {
            print!("");
            Ok(())
        }
        other => anyhow::bail!(
            "not implemented yet: {}",
            match other {
                Cmd::Daemon => "daemon",
                Cmd::Hook { .. } => "hook",
                Cmd::Install => "install",
                Cmd::Show { .. } => "show",
                Cmd::Ledger { .. } => "ledger",
                Cmd::Mute { .. } => "mute",
                Cmd::Dismiss { .. } => "dismiss",
                Cmd::Review { .. } => "review",
                Cmd::Mcp => "mcp",
                Cmd::Status => unreachable!(),
            }
        ),
    }
}
