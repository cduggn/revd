//! Background worker: owns the socket and all DB writes. See docs/SPEC.md §2.
use crate::{analyze, git, store, surface};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

/// A message from a hook. One JSON object per line.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    Commit { repo: String, sha: String },
    Ping,
}

pub fn serve() -> Result<()> {
    let sock = store::socket_path()?;
    // A stale socket file from a killed daemon would block bind.
    if sock.exists() {
        if UnixStream::connect(&sock).is_ok() {
            anyhow::bail!("daemon already running at {}", sock.display());
        }
        std::fs::remove_file(&sock)?;
    }
    let listener = UnixListener::bind(&sock)?;
    tracing::info!("revd daemon listening on {}", sock.display());

    let conn = store::open()?;
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("accept failed: {e}");
                continue;
            }
        };
        for line in BufReader::new(stream).lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    tracing::warn!("read failed: {e}");
                    break;
                }
            };
            if line.trim().is_empty() {
                continue;
            }
            store::log_event(&conn, "hook", &line).ok();
            match serde_json::from_str::<Event>(&line) {
                // One bad event must never take the daemon down.
                Ok(ev) => {
                    if let Err(e) = handle(&conn, ev) {
                        tracing::warn!("event failed: {e:#}");
                    }
                }
                Err(e) => tracing::warn!("bad event {line:?}: {e}"),
            }
        }
    }
    Ok(())
}

fn handle(conn: &rusqlite::Connection, ev: Event) -> Result<()> {
    match ev {
        Event::Ping => Ok(()),
        Event::Commit { repo, sha } => process_commit(conn, &repo, &sha),
    }
}

/// Analyse one commit and record what we found. Also callable synchronously,
/// which is how `revd show` works when no daemon is running.
pub fn process_commit(conn: &rusqlite::Connection, repo: &str, sha: &str) -> Result<()> {
    let path = Path::new(repo);
    let info = git::commit_info(path, sha)?;
    store::upsert_repo(conn, repo)?;
    store::insert_commit(conn, &info, repo)?;

    let diffs = git::added_lines(path, sha)?;
    let findings = analyze::run(repo, sha, &diffs);
    tracing::info!(
        "commit {} — {} files, {} findings",
        &sha[..7.min(sha.len())],
        diffs.len(),
        findings.len()
    );
    store::clear_findings(conn, sha)?;
    store::insert_findings(conn, &findings)?;
    surface::apply(conn, &findings)?;
    Ok(())
}
