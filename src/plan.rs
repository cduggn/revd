//! The repeatable process: detect → plan → (show | apply).
//!
//! `revd plan` and `revd init` run the *same* planner; the only difference is
//! whether the actions are executed. That keeps the dry run honest.
use crate::hooks;
use crate::lang::{self, Detected, Lang};
use crate::tools::{self, ToolSpec};
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Why an action is or is not going to happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// revd will create this file.
    Create,
    /// Already present; revd leaves it alone.
    Exists(String),
    /// Nothing to generate for this tool.
    NoTemplate,
}

#[derive(Debug, Clone)]
pub struct ToolPlan {
    pub spec: &'static ToolSpec,
    pub installed: Option<String>,
    pub config: Status,
    pub target: Option<PathBuf>,
}

impl ToolPlan {
    pub fn will_write(&self) -> bool {
        self.config == Status::Create
    }
}

#[derive(Debug)]
pub struct Plan {
    pub root: PathBuf,
    pub detected: Vec<Detected>,
    pub tools: Vec<ToolPlan>,
    pub hook: hooks::HookPlan,
}

impl Plan {
    #[allow(dead_code)]
    pub fn languages(&self) -> Vec<Lang> {
        self.detected.iter().map(|d| d.lang).collect()
    }

    pub fn missing_tools(&self) -> Vec<&ToolPlan> {
        self.tools.iter().filter(|t| t.installed.is_none()).collect()
    }

    pub fn files_to_write(&self) -> Vec<&ToolPlan> {
        self.tools.iter().filter(|t| t.will_write()).collect()
    }
}

/// Build a plan for a project. Pure inspection — writes nothing.
pub fn build(root: &Path) -> Result<Plan> {
    let detected = lang::detect(root, 2);
    let langs: Vec<Lang> = detected.iter().map(|d| d.lang).collect();

    let tools = tools::for_languages(&langs)
        .into_iter()
        .map(|spec| {
            let installed = spec.installed_version();
            let (config, target) = match (spec.existing_config(root), spec.template) {
                (Some(found), _) => (Status::Exists(found), None),
                (None, Some(tpl)) => (Status::Create, Some(root.join(tpl.path))),
                (None, None) => (Status::NoTemplate, None),
            };
            ToolPlan {
                spec,
                installed,
                config,
                target,
            }
        })
        .collect();

    let hook = hooks::plan(root)?;

    Ok(Plan {
        root: root.to_path_buf(),
        detected,
        tools,
        hook,
    })
}

/// Execute a plan. Never overwrites an existing file unless `force`.
pub fn apply(plan: &Plan, force: bool) -> Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    for t in &plan.tools {
        let (Some(target), Some(tpl)) = (&t.target, t.spec.template) else {
            continue;
        };
        if target.exists() && !force {
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(target, tpl.contents)?;
        written.push(target.clone());
    }
    if let Some(p) = hooks::apply(plan, force)? {
        written.push(p);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(files: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("revd-plan-{}", ulid::Ulid::new()));
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        for (f, c) in files {
            let p = dir.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, c).unwrap();
        }
        dir
    }

    #[test]
    fn plans_go_tools_for_a_go_project() {
        let d = scratch(&[("go.mod", "module x")]);
        let p = build(&d).unwrap();
        assert_eq!(p.languages(), vec![Lang::Go]);
        let ids: Vec<&str> = p.tools.iter().map(|t| t.spec.id).collect();
        assert!(ids.contains(&"golangci-lint"));
        assert!(ids.contains(&"gitleaks"), "secrets tool applies to every project");
        assert!(!ids.contains(&"ruff"), "python tools must not appear");
    }

    #[test]
    fn respects_an_existing_config() {
        let d = scratch(&[("go.mod", "module x"), (".golangci.yml", "# mine")]);
        let p = build(&d).unwrap();
        let gcl = p.tools.iter().find(|t| t.spec.id == "golangci-lint").unwrap();
        assert!(matches!(gcl.config, Status::Exists(_)));
        assert!(!gcl.will_write());
    }

    #[test]
    fn apply_writes_configs_and_never_clobbers() {
        let d = scratch(&[("go.mod", "module x")]);
        let p = build(&d).unwrap();
        let written = apply(&p, false).unwrap();
        assert!(written.iter().any(|w| w.ends_with(".golangci.yml")));

        // user edits the generated file; a second run must not undo that
        let cfg = d.join(".golangci.yml");
        std::fs::write(&cfg, "# hand edited").unwrap();
        let p2 = build(&d).unwrap();
        apply(&p2, false).unwrap();
        assert_eq!(std::fs::read_to_string(&cfg).unwrap(), "# hand edited");
    }

    #[test]
    fn polyglot_project_gets_tools_for_every_language() {
        let d = scratch(&[("go.mod", "module x"), ("web/package.json", "{}"), ("Cargo.toml", "[package]")]);
        let p = build(&d).unwrap();
        let ids: Vec<&str> = p.tools.iter().map(|t| t.spec.id).collect();
        assert!(ids.contains(&"golangci-lint"));
        assert!(ids.contains(&"oxlint"));
        assert!(ids.contains(&"clippy"));
    }
}
