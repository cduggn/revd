//! Git hook installation.
//!
//! The hook is deliberately conservative: only secret detection blocks a
//! commit, every tool is optional, and a missing binary is skipped rather than
//! treated as failure. Friction is what kills these tools.
use crate::plan::Plan;
use crate::tools::Role;
use anyhow::Result;
use std::path::{Path, PathBuf};

const TEMPLATE: &str = include_str!("../templates/pre-commit.sh");
const MARKER: &str = "Installed by revd";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookPlan {
    /// No hook present; revd will install one.
    Install,
    /// A revd hook is already there; it will be refreshed.
    Refresh,
    /// Someone else's hook is there; revd will not touch it.
    Foreign(PathBuf),
    /// Not a git repository.
    NotGit,
}

pub fn hook_path(root: &Path) -> PathBuf {
    root.join(".git/hooks/pre-commit")
}

pub fn plan(root: &Path) -> Result<HookPlan> {
    if !root.join(".git").exists() {
        return Ok(HookPlan::NotGit);
    }
    let path = hook_path(root);
    if !path.exists() {
        return Ok(HookPlan::Install);
    }
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    if existing.contains(MARKER) {
        Ok(HookPlan::Refresh)
    } else {
        Ok(HookPlan::Foreign(path))
    }
}

/// Build the advisory section from the tools this project actually uses.
fn advisory_block(plan: &Plan) -> String {
    let mut lines = Vec::new();
    for t in &plan.tools {
        // formatting and type-checking are too slow or too noisy for a hook
        if !matches!(t.spec.role, Role::Lint) {
            continue;
        }
        let args = t.spec.scan_args.join(" ");
        lines.push(format!(
            "if command -v {bin} >/dev/null 2>&1; then\n  \
             {bin} {args} >/dev/null 2>&1 || say \"revd: {id} reported issues (advisory) — run: {bin} {args}\"\nfi",
            bin = t.spec.binary,
            args = args,
            id = t.spec.id
        ));
    }
    if lines.is_empty() {
        "# no advisory linters configured".to_string()
    } else {
        lines.join("\n")
    }
}

pub fn render(plan: &Plan) -> String {
    TEMPLATE.replace("__REVD_ADVISORY__", &advisory_block(plan))
}

/// Write the hook. Returns the path if one was written.
pub fn apply(plan: &Plan, force: bool) -> Result<Option<PathBuf>> {
    match &plan.hook {
        HookPlan::NotGit => Ok(None),
        HookPlan::Foreign(_) if !force => Ok(None),
        _ => {
            let path = hook_path(&plan.root);
            let next = render(plan);
            // Re-running init must be a genuine no-op, not a silent rewrite.
            if std::fs::read_to_string(&path).is_ok_and(|cur| cur == next) {
                return Ok(None);
            }
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(&path, next)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
            }
            Ok(Some(path))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_a_foreign_hook_and_leaves_it_alone() {
        let d = std::env::temp_dir().join(format!("revd-hook-{}", ulid::Ulid::new()));
        std::fs::create_dir_all(d.join(".git/hooks")).unwrap();
        std::fs::write(d.join("go.mod"), "module x").unwrap();
        std::fs::write(hook_path(&d), "#!/bin/sh\necho mine\n").unwrap();

        let p = crate::plan::build(&d).unwrap();
        assert!(matches!(p.hook, HookPlan::Foreign(_)));
        crate::plan::apply(&p, false).unwrap();
        assert_eq!(
            std::fs::read_to_string(hook_path(&d)).unwrap(),
            "#!/bin/sh\necho mine\n",
            "a foreign hook must survive untouched"
        );
    }

    #[test]
    fn rerunning_init_writes_nothing() {
        let d = std::env::temp_dir().join(format!("revd-hook-{}", ulid::Ulid::new()));
        std::fs::create_dir_all(d.join(".git/hooks")).unwrap();
        std::fs::write(d.join("go.mod"), "module x").unwrap();
        let p = crate::plan::build(&d).unwrap();
        assert!(!crate::plan::apply(&p, false).unwrap().is_empty());
        let p2 = crate::plan::build(&d).unwrap();
        assert!(
            crate::plan::apply(&p2, false).unwrap().is_empty(),
            "a second init must write nothing"
        );
    }

    #[test]
    fn generated_hook_only_blocks_on_secrets() {
        let d = std::env::temp_dir().join(format!("revd-hook-{}", ulid::Ulid::new()));
        std::fs::create_dir_all(d.join(".git/hooks")).unwrap();
        std::fs::write(d.join("go.mod"), "module x").unwrap();
        let p = crate::plan::build(&d).unwrap();
        let script = render(&p);
        assert!(script.contains("gitleaks"));
        assert!(script.contains("fail=1"), "secrets must be able to fail the commit");
        assert!(script.contains("advisory"), "linters must be advisory");
        assert!(!script.contains("__REVD_ADVISORY__"), "placeholder must be substituted");
    }
}
