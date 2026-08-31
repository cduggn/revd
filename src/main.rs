//! revd — set a project up with the checks its languages deserve, then keep
//! an eye on what gets committed. See docs/SPEC.md.
mod git;
mod hooks;
mod lang;
mod plan;
mod report;
mod tools;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "revd",
    version,
    about = "Detect a project's languages and set up the checks they deserve"
)]
struct Cli {
    /// Project root (defaults to the git toplevel, else the current directory)
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Write missing configs and install the pre-commit hook
    Init {
        /// Overwrite existing configs and a foreign pre-commit hook
        #[arg(long)]
        force: bool,
    },
    /// Show what `init` would do, changing nothing
    Plan,
    /// Report which tools and configs are present, and what is missing
    Doctor,
    /// List every tool revd knows about
    Tools,
}

fn resolve_root(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    let cwd = std::env::current_dir()?;
    Ok(git::toplevel(&cwd).map(PathBuf::from).unwrap_or(cwd))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = resolve_root(cli.root)?;

    match cli.cmd {
        Cmd::Tools => {
            report::registry();
            Ok(())
        }
        Cmd::Plan => {
            let p = plan::build(&root)?;
            report::languages(&p);
            report::tools(&p);
            report::hook(&p);
            report::notes(&p);
            report::summary(&p);
            println!("\nnothing was written — run `revd init` to apply");
            Ok(())
        }
        Cmd::Doctor => {
            let p = plan::build(&root)?;
            report::languages(&p);
            report::tools(&p);
            report::role_coverage(&p);
            report::hook(&p);
            Ok(())
        }
        Cmd::Init { force } => {
            let p = plan::build(&root)?;
            report::languages(&p);
            if p.detected.is_empty() {
                println!("nothing to set up — run revd from a project root");
                return Ok(());
            }
            let written = plan::apply(&p, force)?;
            if written.is_empty() {
                println!("\nnothing to do — already configured");
            } else {
                println!("\nwrote:");
                for w in &written {
                    let shown = w.strip_prefix(&root).unwrap_or(w);
                    println!("  {}", shown.display());
                }
            }
            let missing = p.missing_tools();
            if !missing.is_empty() {
                println!("\ninstall the tools you want (revd never installs anything itself):");
                for t in missing {
                    println!("  {:<16} {}", t.spec.id, t.spec.install.command());
                }
            }
            match &p.hook {
                crate::hooks::HookPlan::Foreign(path) => println!(
                    "\nleft your existing pre-commit hook alone: {}\n  re-run with --force to replace it",
                    path.display()
                ),
                crate::hooks::HookPlan::Managed(m, path) => println!(
                    "\n{} manages this repo's hooks ({}) — revd installed no hook.\n  to add these checks: {}",
                    m.name(),
                    path.display(),
                    m.advice()
                ),
                _ => {}
            }
            Ok(())
        }
    }
}
