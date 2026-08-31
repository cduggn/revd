//! Git access: shell out to `git`. Kept minimal — the commit-analysis layer
//! will extend this. See docs/SPEC.md.
use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .context("failed to run git")?;
    if !out.status.success() {
        bail!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn toplevel(from: &Path) -> Result<String> {
    Ok(git(from, &["rev-parse", "--show-toplevel"])?
        .trim()
        .to_string())
}

#[allow(dead_code)] // used by the commit-analysis layer
pub fn head(repo: &Path) -> Result<String> {
    Ok(git(repo, &["rev-parse", "HEAD"])?.trim().to_string())
}

