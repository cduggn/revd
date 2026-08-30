//! SQLite storage. See docs/SPEC.md §8.
use crate::analyze::Finding;
use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::PathBuf;

const SCHEMA: &str = r#"
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS repos (
  id INTEGER PRIMARY KEY,
  path TEXT UNIQUE NOT NULL,
  name TEXT NOT NULL,
  first_seen TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS commits (
  sha TEXT PRIMARY KEY,
  repo TEXT NOT NULL,
  parent TEXT, branch TEXT, ts TEXT, author TEXT, msg TEXT,
  files INTEGER, adds INTEGER, dels INTEGER,
  seen_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS findings (
  id TEXT PRIMARY KEY,
  repo TEXT NOT NULL, sha TEXT NOT NULL, file TEXT NOT NULL,
  line_start INTEGER NOT NULL, line_end INTEGER NOT NULL,
  lang TEXT NOT NULL, category TEXT NOT NULL, severity TEXT NOT NULL,
  confidence REAL NOT NULL, source TEXT NOT NULL, title TEXT NOT NULL,
  evidence TEXT NOT NULL, fix_hint TEXT NOT NULL,
  author TEXT NOT NULL, touched_after_agent INTEGER NOT NULL,
  status TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_findings_sha ON findings(sha);
CREATE TABLE IF NOT EXISTS notified (
  sha TEXT PRIMARY KEY,
  ts TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS events (
  id INTEGER PRIMARY KEY,
  ts TEXT NOT NULL, kind TEXT NOT NULL, payload TEXT NOT NULL
);
"#;

/// ~/.revd, created on demand.
pub fn revd_dir() -> Result<PathBuf> {
    let base = directories::BaseDirs::new().context("no home directory")?;
    let dir = base.home_dir().join(".revd");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn socket_path() -> Result<PathBuf> {
    Ok(revd_dir()?.join("revd.sock"))
}

pub fn open() -> Result<Connection> {
    let conn = Connection::open(revd_dir()?.join("revd.db"))?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn log_event(conn: &Connection, kind: &str, payload: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO events (ts, kind, payload) VALUES (?1, ?2, ?3)",
        params![now(), kind, payload],
    )?;
    Ok(())
}

pub fn upsert_repo(conn: &Connection, path: &str) -> Result<()> {
    let name = std::path::Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    conn.execute(
        "INSERT OR IGNORE INTO repos (path, name, first_seen) VALUES (?1, ?2, ?3)",
        params![path, name, now()],
    )?;
    Ok(())
}

pub fn insert_commit(conn: &Connection, c: &crate::git::CommitInfo, repo: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO commits
         (sha, repo, parent, branch, ts, author, msg, files, adds, dels, seen_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            c.sha,
            repo,
            c.parent,
            c.branch,
            c.ts,
            c.author,
            c.msg,
            c.files,
            c.adds,
            c.dels,
            now()
        ],
    )?;
    Ok(())
}

pub fn insert_findings(conn: &Connection, fs: &[Finding]) -> Result<()> {
    let mut stmt = conn.prepare(
        "INSERT OR REPLACE INTO findings
         (id, repo, sha, file, line_start, line_end, lang, category, severity,
          confidence, source, title, evidence, fix_hint, author,
          touched_after_agent, status, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
    )?;
    for f in fs {
        stmt.execute(params![
            f.id,
            f.repo,
            f.sha,
            f.file,
            f.line_start,
            f.line_end,
            f.lang,
            f.category,
            f.severity.as_str(),
            f.confidence,
            f.source,
            f.title,
            f.evidence,
            f.fix_hint,
            f.author.as_str(),
            f.touched_after_agent as i32,
            f.status.as_str(),
            f.created_at,
        ])?;
    }
    Ok(())
}

/// (file, line_start, category, severity, title, evidence, fix_hint) for a commit.
pub type FindingRow = (String, u32, String, String, String, String, String);

pub fn findings_for_sha(conn: &Connection, sha: &str) -> Result<Vec<FindingRow>> {
    let mut stmt = conn.prepare(
        "SELECT file, line_start, category, severity, title, evidence, fix_hint
         FROM findings WHERE sha = ?1 ORDER BY file, line_start",
    )?;
    let rows = stmt
        .query_map(params![sha], |r| {
            Ok((
                r.get(0)?,
                r.get::<_, i64>(1)? as u32,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// True the first time it is called for a sha. Enforces "one notification per
/// commit" across daemon restarts and repeated analysis.
pub fn claim_notification(conn: &Connection, sha: &str) -> Result<bool> {
    let n = conn.execute(
        "INSERT OR IGNORE INTO notified (sha, ts) VALUES (?1, ?2)",
        params![sha, now()],
    )?;
    Ok(n == 1)
}

/// Drop stored findings for a commit before re-inserting, so rules that no
/// longer fire leave nothing behind.
pub fn clear_findings(conn: &Connection, sha: &str) -> Result<()> {
    conn.execute("DELETE FROM findings WHERE sha = ?1", params![sha])?;
    Ok(())
}

pub fn alert_count(conn: &Connection, sha: &str) -> Result<i64> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM findings
         WHERE sha = ?1 AND status = 'open' AND severity IN ('high','block')",
        params![sha],
        |r| r.get(0),
    )?;
    Ok(n)
}
