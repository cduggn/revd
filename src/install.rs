//! `revd install` — wire the current repo's git hooks to this binary.
//! See docs/SPEC.md §2. Claude Code hooks and MCP registration come later.
use anyhow::{Context, Result};
use std::path::Path;

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo = crate::git::toplevel(&cwd).context("not inside a git repository")?;
    let exe = std::env::current_exe()?;
    let hooks = Path::new(&repo).join(".git/hooks");
    std::fs::create_dir_all(&hooks)?;

    let path = hooks.join("post-commit");
    if path.exists() {
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        if !existing.contains("revd hook") {
            anyhow::bail!(
                "{} already exists and is not a revd hook — add this line yourself:\n  {} hook post-commit >/dev/null 2>&1 || true",
                path.display(),
                exe.display()
            );
        }
    }
    // `|| true` and the redirect keep a broken revd from ever failing a commit.
    let script = format!(
        "#!/bin/sh\n# installed by revd\n{} hook post-commit >/dev/null 2>&1 || true\n",
        exe.display()
    );
    std::fs::write(&path, script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    println!("installed post-commit hook in {}", path.display());
    println!("start the worker with:  revd daemon");
    Ok(())
}
