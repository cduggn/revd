//! revd — set a project up with the checks its languages deserve, then keep
//! an eye on what gets committed. See docs/SPEC.md.
mod git;
mod hooks;
mod lang;
mod plan;
mod report;
mod tools;
mod ui;

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
            report::header(&p);
            if p.detected.is_empty() {
                println!("\n  {}\n", ui::dim("run revd from a project root"));
                return Ok(());
            }
            report::table(&p);
            report::hook(&p);
            report::summary(&p, false);
            report::install_hints(&p);
            report::hook_footnote(&p);
            println!(
                "\n  {}\n",
                ui::dim("nothing written — run `revd init` to apply")
            );
            Ok(())
        }
        Cmd::Doctor => {
            let p = plan::build(&root)?;
            report::header(&p);
            if p.detected.is_empty() {
                println!("\n  {}\n", ui::dim("run revd from a project root"));
                return Ok(());
            }
            report::table(&p);
            report::hook(&p);
            report::coverage(&p);
            report::notes(&p);
            report::summary(&p, false);
            report::install_hints(&p);
            println!();
            Ok(())
        }
        Cmd::Init { force } => {
            let p = plan::build(&root)?;
            report::header(&p);
            if p.detected.is_empty() {
                println!(
                    "\n  {}\n",
                    ui::dim("nothing to set up — run revd from a project root")
                );
                return Ok(());
            }
            let written = plan::apply(&p, force)?;
            report::table(&p);
            report::hook(&p);
            report::wrote(&p, &written);
            report::summary(&p, true);
            report::install_hints(&p);
            report::hook_footnote(&p);
            println!();
            Ok(())
        }
    }
}
