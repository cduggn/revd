//! The tool registry — revd's extension point.
//!
//! Adding support for a new tool means adding one [`ToolSpec`] const to
//! `registry.rs` and listing it in [`registry::ALL`]. No trait to implement,
//! no dispatch to wire: a tool is data.
pub mod registry;

use crate::lang::Lang;
use serde::Serialize;
use std::process::Command;

/// What job a tool does. One tool may only fill one role; use two specs if it
/// genuinely does two (e.g. ruff lint vs ruff format).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Secrets,
    Sast,
    Lint,
    TypeCheck,
    Deps,
    Format,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Secrets => "secrets",
            Role::Sast => "sast",
            Role::Lint => "lint",
            Role::TypeCheck => "typecheck",
            Role::Deps => "deps",
            Role::Format => "format",
        }
    }
}

/// How the user installs the tool. revd never installs anything itself — it
/// prints the command, so the user stays in control of their machine.
#[derive(Debug, Clone, Copy)]
pub enum Install {
    Brew(&'static str),
    Go(&'static str),
    #[allow(dead_code)] // for tools distributed via crates.io
    Cargo(&'static str),
    Npm(&'static str),
    Pipx(&'static str),
    /// Ships with the language toolchain; nothing to install.
    Builtin,
    Manual(&'static str),
}

impl Install {
    pub fn command(&self) -> String {
        match self {
            Install::Brew(p) => format!("brew install {p}"),
            Install::Go(p) => format!("go install {p}"),
            Install::Cargo(p) => format!("cargo install {p}"),
            Install::Npm(p) => format!("npm i -g {p}"),
            Install::Pipx(p) => format!("pipx install {p}"),
            Install::Builtin => "(ships with the toolchain)".into(),
            Install::Manual(s) => (*s).to_string(),
        }
    }
}

/// What the tool can emit. Drives how findings are parsed later; for `init`
/// it is documentation of what we would get.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Output {
    /// SARIF 2.1.0 — one parser serves all of these.
    Sarif,
    /// Structured but tool-specific.
    Json,
    /// Needs a bespoke parser.
    Text,
}

/// A config file revd can generate for a tool.
#[derive(Debug, Clone, Copy)]
pub struct Template {
    /// Where it goes, relative to the project root.
    pub path: &'static str,
    pub contents: &'static str,
}

/// Everything revd knows about one tool.
#[derive(Debug, Clone, Copy)]
pub struct ToolSpec {
    pub id: &'static str,
    pub role: Role,
    /// Languages this serves. Empty means language-agnostic.
    pub langs: &'static [Lang],
    /// Executable used to invoke the tool.
    pub binary: &'static str,
    /// Executable to probe for existence, when it differs from `binary`.
    /// Cargo subcommands install as `cargo-clippy` but are invoked as `cargo clippy`.
    pub probe: Option<&'static str>,
    pub version_args: &'static [&'static str],
    pub install: Install,
    /// Existing config files that mean "already set up — don't touch".
    pub config_files: &'static [&'static str],
    /// Config revd offers to write. None = tool needs no config.
    pub template: Option<Template>,
    /// How to run it over changed code only, for the hook.
    pub scan_args: &'static [&'static str],
    pub output: Output,
    /// One line shown to the user explaining why this tool.
    pub why: &'static str,
    /// Caveats worth surfacing (licence, maintenance, gotchas).
    pub note: &'static str,
}

impl ToolSpec {
    /// Does this tool apply to a project containing these languages?
    pub fn applies_to(&self, langs: &[Lang]) -> bool {
        self.langs.is_empty() || self.langs.iter().any(|l| langs.contains(l))
    }

    /// Version string if the binary is on PATH, else None.
    pub fn installed_version(&self) -> Option<String> {
        let bin = self.probe.unwrap_or(self.binary);
        let out = Command::new(bin).args(self.version_args).output().ok()?;
        if !out.status.success() && out.stdout.is_empty() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let text = if text.trim().is_empty() {
            String::from_utf8_lossy(&out.stderr).to_string()
        } else {
            text.to_string()
        };
        text.lines().next().map(|l| l.trim().to_string())
    }

    /// The first existing config file found, if any.
    pub fn existing_config(&self, root: &std::path::Path) -> Option<String> {
        self.config_files
            .iter()
            .find(|f| root.join(f).exists())
            .map(|f| (*f).to_string())
    }
}

/// Specs applicable to a set of detected languages, in a stable order.
pub fn for_languages(langs: &[Lang]) -> Vec<&'static ToolSpec> {
    let mut v: Vec<&'static ToolSpec> = registry::ALL
        .iter()
        .filter(|t| t.applies_to(langs))
        .collect();
    v.sort_by_key(|t| (t.role, t.id));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_ids_are_unique() {
        let mut ids: Vec<&str> = registry::ALL.iter().map(|t| t.id).collect();
        let before = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(before, ids.len(), "duplicate tool id in registry");
    }

    #[test]
    fn every_language_has_a_linter_and_a_secrets_scanner() {
        for lang in Lang::ALL {
            let tools = for_languages(&[*lang]);
            assert!(
                tools.iter().any(|t| t.role == Role::Lint),
                "{} has no lint tool",
                lang.as_str()
            );
            assert!(
                tools.iter().any(|t| t.role == Role::Secrets),
                "{} has no secrets tool",
                lang.as_str()
            );
        }
    }

    #[test]
    fn template_paths_are_relative() {
        for t in registry::ALL {
            if let Some(tpl) = t.template {
                assert!(!tpl.path.starts_with('/'), "{}: absolute template path", t.id);
                assert!(!tpl.contents.is_empty(), "{}: empty template", t.id);
            }
        }
    }

    #[test]
    fn language_agnostic_tools_apply_everywhere() {
        let gitleaks = registry::ALL.iter().find(|t| t.id == "gitleaks").unwrap();
        assert!(gitleaks.applies_to(&[Lang::Java]));
        assert!(gitleaks.applies_to(&[Lang::Rust]));
    }
}
