//! Git access: shell out to `git`, parse diffs into added-line hunks. See SPEC.md §3.1.
use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub sha: String,
    pub parent: String,
    pub branch: String,
    pub ts: String,
    pub author: String,
    pub msg: String,
    pub files: u32,
    pub adds: u32,
    pub dels: u32,
}

/// One added line, with its line number in the new file.
#[derive(Debug, Clone)]
pub struct AddedLine {
    pub line: u32,
    pub text: String,
}

/// The added lines of one file in a commit.
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub lang: String,
    pub added: Vec<AddedLine>,
}

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

pub fn head(repo: &Path) -> Result<String> {
    Ok(git(repo, &["rev-parse", "HEAD"])?.trim().to_string())
}

pub fn lang_of(path: &str) -> String {
    match path.rsplit('.').next().unwrap_or("") {
        "go" => "go",
        "ts" | "tsx" => "ts",
        "js" | "jsx" | "mjs" => "js",
        "rs" => "rust",
        "py" => "python",
        _ => "other",
    }
    .to_string()
}

pub fn commit_info(repo: &Path, sha: &str) -> Result<CommitInfo> {
    // %x1f = unit separator, keeps fields unambiguous
    let raw = git(
        repo,
        &["show", "-s", "--format=%H%x1f%P%x1f%cI%x1f%an%x1f%s", sha],
    )?;
    let f: Vec<&str> = raw.trim_end().split('\u{1f}').collect();
    if f.len() < 5 {
        bail!("unexpected git show output for {sha}");
    }
    let branch = git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .unwrap_or_default()
        .trim()
        .to_string();

    // numstat gives per-file adds/dels; sum them
    let stat = git(repo, &["show", "--numstat", "--format=", sha])?;
    let (mut files, mut adds, mut dels) = (0u32, 0u32, 0u32);
    for line in stat.lines().filter(|l| !l.trim().is_empty()) {
        let mut parts = line.split('\t');
        let a = parts.next().unwrap_or("0");
        let d = parts.next().unwrap_or("0");
        files += 1;
        adds += a.parse::<u32>().unwrap_or(0); // "-" for binary
        dels += d.parse::<u32>().unwrap_or(0);
    }

    Ok(CommitInfo {
        sha: f[0].to_string(),
        parent: f[1].split(' ').next().unwrap_or("").to_string(),
        branch,
        ts: f[2].to_string(),
        author: f[3].to_string(),
        msg: f[4].to_string(),
        files,
        adds,
        dels,
    })
}

/// Added lines per file for a commit, via `git show --unified=0`.
pub fn added_lines(repo: &Path, sha: &str) -> Result<Vec<FileDiff>> {
    let diff = git(
        repo,
        &["show", "--unified=0", "--format=", "--no-color", sha],
    )?;
    Ok(parse_unified0(&diff))
}

/// Parse a `--unified=0` diff. Only additions are tracked; line numbers come
/// from the `@@ -a,b +c,d @@` hunk headers.
pub fn parse_unified0(diff: &str) -> Vec<FileDiff> {
    let mut out: Vec<FileDiff> = Vec::new();
    let mut cur: Option<FileDiff> = None;
    let mut next_line = 0u32;

    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("+++ b/") {
            if let Some(f) = cur.take() {
                out.push(f);
            }
            cur = Some(FileDiff {
                path: rest.to_string(),
                lang: lang_of(rest),
                added: Vec::new(),
            });
        } else if line.starts_with("+++ /dev/null") {
            // file deleted; drop whatever we were building
            cur = None;
        } else if let Some(rest) = line.strip_prefix("@@ ") {
            // "-1,0 +2,3 @@ optional heading"
            if let Some(plus) = rest.split_whitespace().find(|t| t.starts_with('+')) {
                let num = plus.trim_start_matches('+');
                let start = num.split(',').next().unwrap_or("0");
                next_line = start.parse().unwrap_or(0);
            }
        } else if let Some(text) = line.strip_prefix('+') {
            if let Some(f) = cur.as_mut() {
                f.added.push(AddedLine {
                    line: next_line,
                    text: text.to_string(),
                });
            }
            next_line += 1;
        }
        // '-' lines and everything else are ignored at unified=0
    }
    if let Some(f) = cur.take() {
        out.push(f);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_added_lines_with_numbers() {
        let diff = "diff --git a/x.go b/x.go\n--- a/x.go\n+++ b/x.go\n@@ -0,0 +3,2 @@\n+one\n+two\n@@ -10,1 +12,1 @@\n-old\n+three\n";
        let files = parse_unified0(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "x.go");
        assert_eq!(files[0].lang, "go");
        let got: Vec<(u32, &str)> = files[0]
            .added
            .iter()
            .map(|a| (a.line, a.text.as_str()))
            .collect();
        assert_eq!(got, vec![(3, "one"), (4, "two"), (12, "three")]);
    }
}
