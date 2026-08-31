//! Terminal rendering for `plan`, `init` and `doctor`.
use crate::plan::{Plan, Status};
use crate::tools::Role;

pub fn languages(plan: &Plan) {
    if plan.detected.is_empty() {
        println!("no supported languages detected (go, typescript, python, rust, java)");
        return;
    }
    let list: Vec<String> = plan
        .detected
        .iter()
        .map(|d| format!("{} ({})", d.lang.as_str(), d.marker))
        .collect();
    println!("detected: {}", list.join(", "));
}

pub fn tools(plan: &Plan) {
    let mut role = None;
    for t in &plan.tools {
        if role != Some(t.spec.role) {
            println!("\n{}", t.spec.role.as_str());
            role = Some(t.spec.role);
        }
        let installed = match &t.installed {
            Some(_) => "installed",
            None => "missing",
        };
        let cfg = match &t.config {
            Status::Create => "will write config".to_string(),
            Status::Exists(f) => format!("config exists ({f})"),
            Status::NoTemplate => "no config needed".to_string(),
        };
        println!("  {:<16} {:<10} {}", t.spec.id, installed, cfg);
        if t.installed.is_none() {
            println!("      install: {}", t.spec.install.command());
        }
    }
}

pub fn hook(plan: &Plan) {
    use crate::hooks::HookPlan;
    let msg = match &plan.hook {
        HookPlan::Install => "will install pre-commit hook".to_string(),
        HookPlan::Refresh => "will refresh existing revd pre-commit hook".to_string(),
        HookPlan::Foreign(p) => {
            format!("pre-commit hook already exists and is not revd's — leaving it ({})", p.display())
        }
        HookPlan::NotGit => "not a git repository — no hook".to_string(),
    };
    println!("\nhook\n  {msg}");
}

pub fn notes(plan: &Plan) {
    let noted: Vec<&crate::plan::ToolPlan> =
        plan.tools.iter().filter(|t| !t.spec.note.is_empty()).collect();
    if noted.is_empty() {
        return;
    }
    println!("\nnotes");
    for t in noted {
        println!("  {:<16} {}", t.spec.id, t.spec.note);
    }
}

pub fn summary(plan: &Plan) {
    let missing = plan.missing_tools().len();
    let writes = plan.files_to_write().len();
    println!(
        "\n{} tool(s) for {} language(s); {} config file(s) to write, {} tool(s) not installed",
        plan.tools.len(),
        plan.detected.len(),
        writes,
        missing
    );
    if missing > 0 {
        println!("revd never installs anything itself — run the install commands above.");
    }
}

/// `revd tools` — the registry, so the extension point is visible.
pub fn registry() {
    let mut role = None;
    for t in crate::tools::registry::ALL {
        if role != Some(t.role) {
            println!("\n{}", t.role.as_str());
            role = Some(t.role);
        }
        let langs = if t.langs.is_empty() {
            "all".to_string()
        } else {
            t.langs.iter().map(|l| l.as_str()).collect::<Vec<_>>().join(",")
        };
        println!("  {:<16} {:<28} {:?}  {}", t.id, langs, t.output, t.why);
    }
    println!("\nadd a tool: define a ToolSpec in src/tools/registry.rs and list it in ALL");
}

pub fn role_coverage(plan: &Plan) {
    println!("\ncoverage");
    for role in [
        Role::Secrets,
        Role::Sast,
        Role::Lint,
        Role::TypeCheck,
        Role::Deps,
        Role::Format,
    ] {
        let have = plan
            .tools
            .iter()
            .filter(|t| t.spec.role == role && t.installed.is_some())
            .count();
        let total = plan.tools.iter().filter(|t| t.spec.role == role).count();
        let mark = if total == 0 {
            "—"
        } else if have == 0 {
            "gap"
        } else {
            "ok"
        };
        println!("  {:<12} {}/{} {}", role.as_str(), have, total, mark);
    }
}
