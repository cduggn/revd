//! revd — a grounding ledger. See docs/SPEC.md.
mod analyze;
mod attrib;
mod daemon;
mod git;
mod hook;
mod install;
mod ledger;
mod mcp;
mod store;
mod surface;
mod taxonomy;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::Path;

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
    /// Install git hooks into the current repository
    Install,
    /// Findings for a commit (default HEAD)
    Show {
        sha: Option<String>,
        /// Analyse now instead of reading stored results
        #[arg(long)]
        rerun: bool,
    },
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

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "revd=info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    match Cli::parse().cmd {
        Cmd::Daemon => daemon::serve(),
        Cmd::Hook { event } => hook::dispatch(&event),
        Cmd::Install => install::run(),
        Cmd::Show { sha, rerun } => show(sha, rerun),
        Cmd::Status => status(),
        _ => anyhow::bail!("not implemented yet"),
    }
}

/// Resolve a sha argument against the current repo, defaulting to HEAD.
fn resolve(sha: Option<String>) -> Result<(String, String)> {
    let cwd = std::env::current_dir()?;
    let repo = git::toplevel(&cwd)?;
    let sha = match sha {
        Some(s) => s,
        None => git::head(Path::new(&repo))?,
    };
    Ok((repo, sha))
}

fn show(sha: Option<String>, rerun: bool) -> Result<()> {
    let (repo, sha) = resolve(sha)?;
    let conn = store::open()?;
    if rerun {
        daemon::process_commit(&conn, &repo, &sha)?;
    }
    let rows = store::findings_for_sha(&conn, &sha)?;
    let short = &sha[..7.min(sha.len())];
    if rows.is_empty() {
        println!("{short}: no findings");
        return Ok(());
    }
    println!("{short} — {} finding(s)\n", rows.len());
    for (file, line, category, severity, title, evidence, fix_hint) in rows {
        println!("{file}:{line}  [{severity}] {category}");
        println!("  {title}");
        println!("  > {evidence}");
        println!("  fix: {fix_hint}\n");
    }
    Ok(())
}

/// Statusline: a count, or nothing at all. Must never error into the prompt.
fn status() -> Result<()> {
    let Ok((_, sha)) = resolve(None) else {
        return Ok(());
    };
    let Ok(conn) = store::open() else {
        return Ok(());
    };
    match store::alert_count(&conn, &sha) {
        Ok(n) if n > 0 => println!("revd ⚠{n}"),
        _ => {}
    }
    Ok(())
}
