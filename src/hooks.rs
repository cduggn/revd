//! Git hook installation.
//!
//! The hook is deliberately conservative: only secret detection blocks a
//! commit, every tool is optional, and a missing binary is skipped rather than
//! treated as failure. Friction is what kills these tools.
use crate::plan::Plan;
use crate::tools::Role;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// A tool that owns this repository's git hooks. revd defers to these rather
/// than fighting them: whatever it wrote would be overwritten on their next
/// `install`, or ignored outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manager {
    PreCommit,
    Husky,
    Lefthook,
    Overcommit,
}

impl Manager {
    pub fn name(&self) -> &'static str {
        match self {
            Manager::PreCommit => "pre-commit",
            Manager::Husky => "husky",
            Manager::Lefthook => "lefthook",
            Manager::Overcommit => "overcommit",
        }
    }

    /// How the user adds revd's checks to that manager's own config.
    pub fn advice(&self) -> &'static str {
        match self {
            Manager::PreCommit => {
                "add a `repo: local` hook to .pre-commit-config.yaml that runs your linters"
            }
            Manager::Husky => "add the commands to .husky/pre-commit",
            Manager::Lefthook => "add them under `pre-commit.commands` in lefthook.yml",
            Manager::Overcommit => "add them under `PreCommit` in .overcommit.yml",
        }
    }

    fn config_files(&self) -> &'static [&'static str] {
        match self {
            Manager::PreCommit => &[".pre-commit-config.yaml", ".pre-commit-config.yml"],
            Manager::Husky => &[".husky"],
            Manager::Lefthook => &["lefthook.yml", "lefthook.yaml", ".lefthook.yml"],
            Manager::Overcommit => &[".overcommit.yml"],
        }
    }

    /// A fingerprint left in the generated hook shim, if the manager writes one.
    fn shim_marker(&self) -> &'static str {
        match self {
            Manager::PreCommit => "pre-commit.com",
            Manager::Husky => "husky",
            Manager::Lefthook => "lefthook",
            Manager::Overcommit => "overcommit",
        }
    }

    const ALL: &'static [Manager] = &[
        Manager::PreCommit,
        Manager::Husky,
        Manager::Lefthook,
        Manager::Overcommit,
    ];
}

/// Detect a hook manager from its config file, or from the shim it installed.
fn detect_manager(root: &Path, existing_hook: Option<&str>) -> Option<Manager> {
    for m in Manager::ALL {
        if m.config_files().iter().any(|f| root.join(f).exists()) {
            return Some(*m);
        }
    }
    let hook = existing_hook?;
    Manager::ALL
        .iter()
        .find(|m| hook.contains(m.shim_marker()))
        .copied()
}

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
    /// A hook manager owns this repo. revd defers to it entirely.
    Managed(Manager, PathBuf),
    /// Not a git repository.
    NotGit,
}

/// Where git will actually look for hooks. Falls back to `.git/hooks` only
/// when git cannot be consulted.
pub fn hook_path(root: &Path) -> PathBuf {
    crate::git::hooks_dir(root)
        .unwrap_or_else(|_| root.join(".git/hooks"))
        .join("pre-commit")
}

pub fn plan(root: &Path) -> Result<HookPlan> {
    if !root.join(".git").exists() {
        return Ok(HookPlan::NotGit);
    }
    let path = hook_path(root);
    let existing = std::fs::read_to_string(&path).ok();

    // A revd hook we wrote ourselves takes precedence: re-running init should
    // refresh it rather than suddenly defer to a manager added since.
    if existing.as_deref().is_some_and(|e| e.contains(MARKER)) {
        return Ok(HookPlan::Refresh);
    }
    if let Some(m) = detect_manager(root, existing.as_deref()) {
        return Ok(HookPlan::Managed(m, path));
    }
    match existing {
        None => Ok(HookPlan::Install),
        Some(_) => Ok(HookPlan::Foreign(path)),
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
    if plan.detected.is_empty() {
        return Ok(None);
    }
    match &plan.hook {
        HookPlan::NotGit => Ok(None),
        // Never fight a hook manager: it would overwrite us on its next install.
        HookPlan::Managed(..) => Ok(None),
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

    fn repo(files: &[(&str, &str)]) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("revd-hm-{}", ulid::Ulid::new()));
        std::fs::create_dir_all(&d).unwrap();
        std::process::Command::new("git")
            .arg("init")
            .arg("-q")
            .current_dir(&d)
            .output()
            .unwrap();
        std::fs::write(d.join("go.mod"), "module x").unwrap();
        for (f, c) in files {
            let p = d.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, c).unwrap();
        }
        d
    }

    #[test]
    fn honours_core_hooks_path() {
        let d = repo(&[(".husky/.keep", "")]);
        std::process::Command::new("git")
            .args(["config", "core.hooksPath", ".husky"])
            .current_dir(&d)
            .output()
            .unwrap();
        // macOS resolves /var to /private/var, so compare the tail, not the prefix.
        let got = hook_path(&d);
        assert!(
            got.ends_with(".husky/pre-commit"),
            "writing to .git/hooks when core.hooksPath is set produces a file git ignores; got {}",
            got.display()
        );
    }

    #[test]
    fn defers_to_the_pre_commit_framework() {
        let d = repo(&[(".pre-commit-config.yaml", "repos: []")]);
        let p = crate::plan::build(&d).unwrap();
        assert!(matches!(p.hook, HookPlan::Managed(Manager::PreCommit, _)));
        assert!(
            crate::plan::apply(&p, false)
                .unwrap()
                .iter()
                .all(|w| !w.ends_with("pre-commit"))
        );
    }

    #[test]
    fn defers_to_lefthook_before_it_has_installed_its_hook() {
        let d = repo(&[("lefthook.yml", "pre-commit:\n  commands: {}")]);
        let p = crate::plan::build(&d).unwrap();
        assert!(matches!(p.hook, HookPlan::Managed(Manager::Lefthook, _)));
    }

    #[test]
    fn recognises_a_manager_from_its_shim_alone() {
        let d = repo(&[]);
        std::fs::create_dir_all(d.join(".git/hooks")).unwrap();
        std::fs::write(
            d.join(".git/hooks/pre-commit"),
            "#!/usr/bin/env bash\n# File generated by pre-commit: https://pre-commit.com\n",
        )
        .unwrap();
        let p = crate::plan::build(&d).unwrap();
        assert!(matches!(p.hook, HookPlan::Managed(Manager::PreCommit, _)));
    }

    #[test]
    fn generated_hook_only_blocks_on_secrets() {
        let d = std::env::temp_dir().join(format!("revd-hook-{}", ulid::Ulid::new()));
        std::fs::create_dir_all(d.join(".git/hooks")).unwrap();
        std::fs::write(d.join("go.mod"), "module x").unwrap();
        let p = crate::plan::build(&d).unwrap();
        let script = render(&p);
        assert!(script.contains("gitleaks"));
        assert!(
            script.contains("fail=1"),
            "secrets must be able to fail the commit"
        );
        assert!(script.contains("advisory"), "linters must be advisory");
        assert!(
            !script.contains("__REVD_ADVISORY__"),
            "placeholder must be substituted"
        );
    }
}
