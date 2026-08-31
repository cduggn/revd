//! Terminal rendering for `plan`, `init` and `doctor`.
//!
//! One layout for all three commands: a header, a table grouped by role, then
//! a summary. Only what differs between them is the verb.
use crate::hooks::HookPlan;
use crate::plan::{Plan, Status, ToolPlan};
use crate::tools::Role;
use crate::ui;

const ROLES: &[Role] = &[
    Role::Secrets,
    Role::Sast,
    Role::Lint,
    Role::TypeCheck,
    Role::Deps,
    Role::Format,
];

const ROLE_W: usize = 11;
const TOOL_W: usize = 18;

fn project_name(plan: &Plan) -> String {
    plan.root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| plan.root.display().to_string())
}

/// `revd · myproject                      go · typescript`
pub fn header(plan: &Plan) {
    let langs: Vec<&str> = plan.detected.iter().map(|d| d.lang.as_str()).collect();
    let left = format!("{} {}", ui::bold("revd"), ui::dim(&project_name(plan)));
    if langs.is_empty() {
        println!("\n{left}  {}", ui::dim("no languages detected"));
    } else {
        println!("\n{left}  {}", ui::cyan(&langs.join(" · ")));
    }
}

/// The config cell: filename, dimmed when it already exists.
fn config_cell(t: &ToolPlan) -> String {
    match &t.config {
        Status::Create => t
            .spec
            .template
            .map(|tpl| ui::yellow(tpl.path))
            .unwrap_or_default(),
        Status::Exists(f) => ui::dim(f),
        Status::NoTemplate => String::new(),
    }
}

pub fn table(plan: &Plan) {
    println!();
    for role in ROLES {
        let tools: Vec<&ToolPlan> = plan.tools.iter().filter(|t| t.spec.role == *role).collect();
        if tools.is_empty() {
            continue;
        }
        for (i, t) in tools.iter().enumerate() {
            // the role label appears once, against its first tool
            let label = if i == 0 { role.as_str() } else { "" };
            let mark = if t.installed.is_some() {
                ui::ok()
            } else {
                ui::missing()
            };
            let name = if t.installed.is_some() {
                t.spec.id.to_string()
            } else {
                ui::dim(t.spec.id)
            };
            let cfg = config_cell(t);
            let line = format!(
                "  {}{} {} {}",
                ui::pad(&ui::dim(label), ROLE_W),
                mark,
                ui::pad(&name, TOOL_W),
                cfg
            );
            println!("{}", line.trim_end());
        }
    }
}

pub fn hook(plan: &Plan) {
    let (mark, text) = match &plan.hook {
        HookPlan::Install => (ui::yellow("→"), "pre-commit hook".to_string()),
        HookPlan::Refresh => (ui::ok(), "pre-commit hook".to_string()),
        HookPlan::Managed(m, _) => (
            ui::dim("·"),
            format!("managed by {} — revd defers to it", ui::bold(m.name())),
        ),
        HookPlan::Foreign(_) => (
            ui::dim("·"),
            "existing hook is not revd's — left alone".to_string(),
        ),
        HookPlan::NotGit => (ui::dim("·"), ui::dim("not a git repository").to_string()),
    };
    println!("  {}{} {}", ui::pad(&ui::dim("hook"), ROLE_W), mark, text);
}

/// `● 6 installed   ○ 7 missing   4 configs to write`
pub fn summary(plan: &Plan, applied: bool) {
    let installed = plan.tools.len() - plan.missing_tools().len();
    let missing = plan.missing_tools().len();
    let writes = plan.files_to_write().len();
    let verb = if applied { "written" } else { "to write" };
    let noun = if writes == 1 { "config" } else { "configs" };
    let mut parts = vec![
        format!("{} {installed} installed", ui::ok()),
        format!("{} {missing} missing", ui::missing()),
    ];
    if writes > 0 || applied {
        parts.push(ui::yellow(&format!("{writes} {noun} {verb}")));
    }
    println!("\n  {}", parts.join(ui::dim("   ").as_str()));
}

/// Install commands for the missing tools, collapsed one line per package
/// manager so each is copy-pasteable.
pub fn install_hints(plan: &Plan) {
    use std::collections::BTreeMap;

    let missing = plan.missing_tools();
    if missing.is_empty() {
        return;
    }
    let mut grouped: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut ungrouped: Vec<(&str, String)> = Vec::new();
    let mut toolchain_missing: Vec<&str> = Vec::new();
    for t in &missing {
        match t.spec.install {
            // "ships with the toolchain" is not an install command; a missing
            // builtin means the language toolchain itself is absent.
            crate::tools::Install::Builtin => toolchain_missing.push(t.spec.id),
            _ => match t.spec.install.group() {
                Some((prefix, pkg)) => grouped.entry(prefix).or_default().push(pkg),
                None => ungrouped.push((t.spec.id, t.spec.install.command())),
            },
        }
    }
    if grouped.is_empty() && ungrouped.is_empty() && toolchain_missing.is_empty() {
        return;
    }
    println!("\n  {}", ui::bold("install what you need"));
    for (prefix, mut pkgs) in grouped {
        pkgs.sort_unstable();
        pkgs.dedup();
        println!("    {} {}", prefix, pkgs.join(" "));
    }
    for (id, cmd) in ungrouped {
        println!("    {} {}", ui::pad(id, TOOL_W), ui::dim(&cmd));
    }
    if !toolchain_missing.is_empty() {
        println!(
            "    {}",
            ui::dim(&format!(
                "not on PATH: {} — install the language toolchain",
                toolchain_missing.join(", ")
            ))
        );
    }
}

/// Caveats — licence, maintenance, gotchas. `doctor` only; too noisy elsewhere.
pub fn notes(plan: &Plan) {
    // Tools you are actually relying on: installed, or carrying a config
    // (whether revd wrote it or it was already there). A caveat about a tool
    // you have configured still matters even before you install it.
    let noted: Vec<&ToolPlan> = plan
        .tools
        .iter()
        .filter(|t| {
            !t.spec.note.is_empty()
                && (t.installed.is_some() || !matches!(t.config, Status::NoTemplate))
        })
        .collect();
    if noted.is_empty() {
        return;
    }
    println!("\n  {}", ui::bold("notes"));
    for t in noted {
        println!(
            "    {} {}",
            ui::pad(t.spec.id, TOOL_W),
            ui::dim(t.spec.note)
        );
    }
}

/// Per-role gaps, for `doctor`.
pub fn coverage(plan: &Plan) {
    println!("\n  {}", ui::bold("coverage"));
    for role in ROLES {
        let total = plan.tools.iter().filter(|t| t.spec.role == *role).count();
        if total == 0 {
            continue;
        }
        let have = plan
            .tools
            .iter()
            .filter(|t| t.spec.role == *role && t.installed.is_some())
            .count();
        let state = if have == 0 {
            ui::yellow("gap")
        } else {
            ui::green("ok")
        };
        println!(
            "    {}{}  {}",
            ui::pad(&ui::dim(role.as_str()), ROLE_W),
            ui::dim(&format!("{have}/{total}")),
            state
        );
    }
}

pub fn wrote(plan: &Plan, written: &[std::path::PathBuf]) {
    if written.is_empty() {
        return;
    }
    println!();
    for w in written {
        let shown = w.strip_prefix(&plan.root).unwrap_or(w);
        println!("  {} {}", ui::green("+"), shown.display());
    }
}

/// A path relative to the project root where possible; absolute paths are
/// noise in a terminal.
fn rel(plan: &Plan, p: &std::path::Path) -> String {
    // canonicalize both sides: macOS reports /var where the root says /private/var
    let root = plan
        .root
        .canonicalize()
        .unwrap_or_else(|_| plan.root.clone());
    let full = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    full.strip_prefix(&root).unwrap_or(p).display().to_string()
}

pub fn hook_footnote(plan: &Plan) {
    match &plan.hook {
        HookPlan::Managed(m, path) => println!(
            "\n  {} manages {}\n    to add these checks: {}",
            ui::bold(m.name()),
            ui::dim(&rel(plan, path)),
            ui::dim(m.advice())
        ),
        HookPlan::Foreign(path) => println!(
            "\n  {} {}\n    {}",
            ui::dim("existing hook left alone:"),
            ui::dim(&rel(plan, path)),
            ui::dim("re-run with --force to replace it")
        ),
        _ => {}
    }
}

/// `revd tools` — the registry, so the extension point is visible.
pub fn registry() {
    for role in ROLES {
        let tools: Vec<_> = crate::tools::registry::ALL
            .iter()
            .filter(|t| t.role == *role)
            .collect();
        for (i, t) in tools.iter().enumerate() {
            let label = if i == 0 { role.as_str() } else { "" };
            let langs = if t.langs.is_empty() {
                "all".to_string()
            } else {
                t.langs
                    .iter()
                    .map(|l| l.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            };
            let fmt = format!("{:?}", t.output).to_lowercase();
            println!(
                "  {}{} {} {} {}",
                ui::pad(&ui::dim(label), ROLE_W),
                ui::pad(t.id, TOOL_W),
                ui::pad(&ui::dim(&langs), 12),
                ui::pad(&ui::dim(&fmt), 6),
                ui::dim(t.why)
            );
        }
    }
    println!(
        "\n  {}",
        ui::dim("add a tool: define a ToolSpec in src/tools/registry.rs and list it in ALL")
    );
}
