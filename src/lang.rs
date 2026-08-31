//! Deterministic language detection from marker files. No AI, no network:
//! a project's build files say what it is with certainty.
use serde::Serialize;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Go,
    TypeScript,
    Python,
    Rust,
    Java,
}

impl Lang {
    pub fn as_str(&self) -> &'static str {
        match self {
            Lang::Go => "go",
            Lang::TypeScript => "typescript",
            Lang::Python => "python",
            Lang::Rust => "rust",
            Lang::Java => "java",
        }
    }

    pub const ALL: &'static [Lang] = &[
        Lang::Go,
        Lang::TypeScript,
        Lang::Python,
        Lang::Rust,
        Lang::Java,
    ];

    /// Marker files that prove the language is present, strongest first.
    fn markers(&self) -> &'static [&'static str] {
        match self {
            Lang::Go => &["go.mod", "go.work"],
            Lang::TypeScript => &["tsconfig.json", "package.json"],
            Lang::Python => &["pyproject.toml", "setup.py", "requirements.txt", "Pipfile"],
            Lang::Rust => &["Cargo.toml"],
            Lang::Java => &["pom.xml", "build.gradle", "build.gradle.kts"],
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Detected {
    pub lang: Lang,
    /// The marker that proved it, relative to the project root.
    pub marker: String,
}

/// Directories never worth descending into.
fn skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | "node_modules"
            | "vendor"
            | "target"
            | "dist"
            | "build"
            | ".venv"
            | "venv"
            | "__pycache__"
            | ".gradle"
            | ".idea"
    )
}

/// Walk up to `max_depth` levels looking for marker files. Depth 2 covers the
/// common monorepo shape (`services/api/go.mod`) without walking the world.
pub fn detect(root: &Path, max_depth: usize) -> Vec<Detected> {
    let mut found: Vec<Detected> = Vec::new();
    walk(root, root, 0, max_depth, &mut found);
    found.sort_by_key(|d| (d.lang, d.marker.clone()));
    found.dedup_by_key(|d| d.lang);
    found
}

fn walk(root: &Path, dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<Detected>) {
    for lang in Lang::ALL {
        for marker in lang.markers() {
            if dir.join(marker).is_file() {
                let rel = dir
                    .strip_prefix(root)
                    .unwrap_or(Path::new(""))
                    .join(marker)
                    .to_string_lossy()
                    .to_string();
                out.push(Detected {
                    lang: *lang,
                    marker: rel,
                });
                break; // one marker per language per directory is enough
            }
        }
    }
    if depth >= max_depth {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let path = e.path();
        if !path.is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        if skip_dir(&name) || name.starts_with('.') {
            continue;
        }
        walk(root, &path, depth + 1, max_depth, out);
    }
}

/// A `package.json` without a `tsconfig.json` is JS, not TS. Callers that care
/// about the distinction can use this; detection stays permissive.
#[allow(dead_code)] // for callers that must distinguish TS from plain JS
pub fn has_typescript_config(root: &Path) -> bool {
    root.join("tsconfig.json").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(files: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("revd-lang-{}", ulid::Ulid::new()));
        for f in files {
            let p = dir.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "").unwrap();
        }
        dir
    }

    #[test]
    fn detects_each_language_from_its_marker() {
        let d = scratch(&["go.mod", "Cargo.toml", "pyproject.toml", "pom.xml"]);
        let langs: Vec<Lang> = detect(&d, 2).into_iter().map(|x| x.lang).collect();
        assert!(langs.contains(&Lang::Go));
        assert!(langs.contains(&Lang::Rust));
        assert!(langs.contains(&Lang::Python));
        assert!(langs.contains(&Lang::Java));
    }

    #[test]
    fn finds_languages_nested_in_a_monorepo() {
        let d = scratch(&["services/api/go.mod", "web/package.json"]);
        let langs: Vec<Lang> = detect(&d, 2).into_iter().map(|x| x.lang).collect();
        assert!(langs.contains(&Lang::Go));
        assert!(langs.contains(&Lang::TypeScript));
    }

    #[test]
    fn ignores_vendored_and_build_directories() {
        let d = scratch(&[
            "node_modules/x/package.json",
            "vendor/y/go.mod",
            "target/z/Cargo.toml",
        ]);
        assert!(detect(&d, 3).is_empty());
    }

    #[test]
    fn reports_each_language_once() {
        let d = scratch(&["go.mod", "services/a/go.mod", "services/b/go.mod"]);
        assert_eq!(detect(&d, 2).len(), 1);
    }
}
