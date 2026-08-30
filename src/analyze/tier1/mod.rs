//! Tier 1 — built-in heuristics over added lines. See docs/SPEC.md §4.
//! Deliberately a small starter set; breadth comes later.
use crate::analyze::{Author, Finding, Status};
use crate::git::FileDiff;
use crate::store::now;
use crate::taxonomy;
use regex::Regex;
use std::sync::OnceLock;

struct Rule {
    /// rule id, used as `source` and for muting
    id: &'static str,
    /// taxonomy path; severity is looked up from it
    category: &'static str,
    /// languages this applies to; empty = all
    langs: &'static [&'static str],
    pattern: &'static str,
    /// If this also matches the line, the finding is suppressed.
    /// (Rust's regex crate has no lookahead — it guarantees linear time.)
    exclude: &'static str,
    title: &'static str,
    fix_hint: &'static str,
    confidence: f32,
}

const RULES: &[Rule] = &[
    Rule {
        id: "debug_leftover",
        category: "common.debug.leftover",
        langs: &[],
        pattern: r"(?:console\.log\(|fmt\.Println\(|\bdebugger\b|\bit\.only\(|\bdescribe\.only\()",
        exclude: "",
        title: "debug statement left in",
        fix_hint: "remove before shipping, or use the logger",
        confidence: 0.9,
    },
    Rule {
        id: "todo_unreferenced",
        category: "common.todo.unreferenced",
        langs: &[],
        pattern: r"(?://|#|/\*)\s*(?:TODO|FIXME|HACK)\b",
        exclude: r"[A-Z]{2,}-\d+|#\d+",
        title: "TODO with no ticket reference",
        fix_hint: "link an issue, or do it now",
        confidence: 0.8,
    },
    Rule {
        id: "secret_literal",
        category: "common.secrets.credential",
        langs: &[],
        pattern: r#"(?i)(?:api[_-]?key|secret|password|passwd|token)\s*[:=]\s*["'][^"']{12,}["']"#,
        exclude: "",
        title: "possible credential literal",
        fix_hint: "move to env/secret store and rotate it",
        confidence: 0.7,
    },
    Rule {
        id: "aws_key",
        category: "common.secrets.credential",
        langs: &[],
        pattern: r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b",
        exclude: "",
        title: "AWS access key id",
        fix_hint: "revoke immediately and remove from history",
        confidence: 0.99,
    },
    Rule {
        id: "go_err_discarded",
        category: "go.errors.unchecked",
        langs: &["go"],
        pattern: r"^\s*_\s*(?:,\s*_\s*)*=\s*\w+.*\berr\b|^\s*_\s*=\s*\w+\(",
        exclude: "",
        title: "error discarded",
        fix_hint: "handle or wrap the error",
        confidence: 0.75,
    },
    Rule {
        id: "go_context_background",
        category: "go.context.background",
        langs: &["go"],
        pattern: r"context\.(?:Background|TODO)\(\)",
        exclude: "",
        title: "context.Background() outside an entry point",
        fix_hint: "accept a ctx parameter and propagate it",
        confidence: 0.6,
    },
    Rule {
        id: "go_panic_in_lib",
        category: "go.errors.panic_in_lib",
        langs: &["go"],
        pattern: r"^\s*panic\(",
        exclude: "",
        title: "panic in library code",
        fix_hint: "return an error instead",
        confidence: 0.7,
    },
    Rule {
        id: "ts_any",
        category: "ts.types.any",
        langs: &["ts"],
        pattern: r"\bas\s+any\b|:\s*any\b|\bas\s+unknown\s+as\b",
        exclude: "",
        title: "`any` introduced",
        fix_hint: "narrow the type, or model it as a union",
        confidence: 0.9,
    },
    Rule {
        id: "ts_empty_catch",
        category: "ts.async.empty_catch",
        langs: &["ts", "js"],
        pattern: r"\.catch\(\s*\(\s*\)\s*=>\s*\{\s*\}\s*\)|catch\s*\([^)]*\)\s*\{\s*\}",
        exclude: "",
        title: "swallowed error",
        fix_hint: "log it or rethrow; empty catch hides failures",
        confidence: 0.9,
    },
    Rule {
        id: "ts_foreach_async",
        category: "ts.async.foreach_async",
        langs: &["ts", "js"],
        pattern: r"\.forEach\(\s*async\b",
        exclude: "",
        title: "async callback in forEach",
        fix_hint: "use a for..of loop, or Promise.all(map(...))",
        confidence: 0.95,
    },
];

/// (match, optional suppress) per rule, compiled once.
type Compiled = Vec<(Regex, Option<Regex>)>;

fn compiled() -> &'static Compiled {
    static RE: OnceLock<Compiled> = OnceLock::new();
    RE.get_or_init(|| {
        RULES
            .iter()
            .map(|r| {
                let m = Regex::new(r.pattern).expect("tier1 rule regex must compile");
                let x = (!r.exclude.is_empty())
                    .then(|| Regex::new(r.exclude).expect("tier1 exclude regex must compile"));
                (m, x)
            })
            .collect()
    })
}

/// Files we never analyse (vendored, generated, lockfiles).
fn skip_file(path: &str) -> bool {
    const SKIP: &[&str] = &["vendor/", "node_modules/", "/testdata/", ".min.js", ".lock"];
    SKIP.iter().any(|s| path.contains(s))
}

/// Rules that only make sense outside test/entry files.
fn rule_applies_to_file(rule_id: &str, path: &str) -> bool {
    let is_test = path.ends_with("_test.go")
        || path.contains(".test.")
        || path.contains(".spec.")
        || path.contains("/test/");
    let is_main = path.ends_with("main.go") || path.contains("/cmd/");
    match rule_id {
        "go_context_background" | "go_panic_in_lib" => !is_test && !is_main,
        "debug_leftover" => !is_test,
        _ => true,
    }
}

pub fn run(repo: &str, sha: &str, diffs: &[FileDiff]) -> Vec<Finding> {
    let res = compiled();
    let mut out = Vec::new();

    for fd in diffs {
        if skip_file(&fd.path) {
            continue;
        }
        for (rule, (re, exclude)) in RULES.iter().zip(res.iter()) {
            if !rule.langs.is_empty() && !rule.langs.contains(&fd.lang.as_str()) {
                continue;
            }
            if !rule_applies_to_file(rule.id, &fd.path) {
                continue;
            }
            for add in &fd.added {
                if !re.is_match(&add.text) {
                    continue;
                }
                if exclude.as_ref().is_some_and(|x| x.is_match(&add.text)) {
                    continue;
                }
                let severity = taxonomy::lookup(rule.category)
                    .map(|n| n.default_severity)
                    .unwrap_or(crate::analyze::Severity::Low);
                out.push(Finding {
                    id: crate::analyze::finding_id(sha, &fd.path, add.line, rule.id),
                    repo: repo.to_string(),
                    sha: sha.to_string(),
                    file: fd.path.clone(),
                    line_start: add.line,
                    line_end: add.line,
                    lang: fd.lang.clone(),
                    category: rule.category.to_string(),
                    severity,
                    confidence: rule.confidence,
                    source: format!("heuristic:{}", rule.id),
                    title: rule.title.to_string(),
                    evidence: add.text.trim().chars().take(160).collect(),
                    fix_hint: rule.fix_hint.to_string(),
                    author: Author::Unknown,
                    touched_after_agent: false,
                    status: Status::Open,
                    created_at: now(),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{AddedLine, FileDiff};

    fn diff(path: &str, lines: &[&str]) -> Vec<FileDiff> {
        vec![FileDiff {
            path: path.to_string(),
            lang: crate::git::lang_of(path),
            added: lines
                .iter()
                .enumerate()
                .map(|(i, t)| AddedLine {
                    line: i as u32 + 1,
                    text: t.to_string(),
                })
                .collect(),
        }]
    }

    fn cats(fs: &[Finding]) -> Vec<&str> {
        fs.iter().map(|f| f.category.as_str()).collect()
    }

    #[test]
    fn reanalysis_is_idempotent() {
        let d = diff("internal/w/pool.go", &["\tctx := context.Background()"]);
        let a = run("r", "s", &d);
        let b = run("r", "s", &d);
        assert_eq!(a.len(), 1);
        assert_eq!(
            a[0].id, b[0].id,
            "same input must yield the same finding id"
        );
    }

    #[test]
    fn flags_go_issues() {
        let d = diff("internal/w/pool.go", &["\tctx := context.Background()"]);
        assert_eq!(cats(&run("r", "s", &d)), vec!["go.context.background"]);
    }

    #[test]
    fn skips_entry_points_and_tests() {
        let d = diff("cmd/app/main.go", &["\tctx := context.Background()"]);
        assert!(run("r", "s", &d).is_empty());
        let d = diff(
            "internal/w/pool_test.go",
            &["\tctx := context.Background()"],
        );
        assert!(run("r", "s", &d).is_empty());
    }

    #[test]
    fn flags_ts_issues() {
        let d = diff(
            "src/a.ts",
            &["  const x = y as any;", "  list.forEach(async (i) => {"],
        );
        assert_eq!(
            cats(&run("r", "s", &d)),
            vec!["ts.types.any", "ts.async.foreach_async"]
        );
    }

    #[test]
    fn flags_secrets_anywhere() {
        let d = diff(
            "src/cfg.ts",
            &["const apiKey = \"sk-liveabcdefghijklmno\";"],
        );
        let f = run("r", "s", &d);
        assert_eq!(cats(&f), vec!["common.secrets.credential"]);
        assert_eq!(f[0].severity, crate::analyze::Severity::Block);
    }

    #[test]
    fn ignores_vendored_paths() {
        let d = diff("vendor/x/pool.go", &["\tpanic(\"x\")"]);
        assert!(run("r", "s", &d).is_empty());
    }

    #[test]
    fn todo_with_ticket_is_fine() {
        let d = diff("a.go", &["// TODO(ENG-123): later", "// TODO: later"]);
        assert_eq!(cats(&run("r", "s", &d)), vec!["common.todo.unreferenced"]);
    }
}

#[cfg(test)]
mod rule_tests {
    #[test]
    fn all_rule_regexes_compile() {
        super::compiled();
    }
}
