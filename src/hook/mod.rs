//! Hook entry points. These run in the authoring loop, so they do the minimum
//! possible and always exit 0. See docs/SPEC.md §1.
use crate::daemon::Event;
use crate::{git, store};
use anyhow::Result;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::Path;

/// Send an event to the daemon. Never fails loudly: a dead daemon must not
/// break the user's commit.
fn send(ev: &Event) {
    let Ok(sock) = store::socket_path() else {
        return;
    };
    let Ok(mut stream) = UnixStream::connect(&sock) else {
        tracing::debug!("no daemon listening; dropping event");
        return;
    };
    if let Ok(json) = serde_json::to_string(ev) {
        let _ = writeln!(stream, "{json}");
        let _ = stream.flush();
    }
}

pub fn dispatch(event: &str) -> Result<()> {
    match event {
        "post-commit" => post_commit(),
        other => {
            tracing::debug!("unknown hook event {other}");
            Ok(())
        }
    }
}

fn post_commit() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let Ok(repo) = git::toplevel(&cwd) else {
        return Ok(()); // not a git repo; nothing to do
    };
    let Ok(sha) = git::head(Path::new(&repo)) else {
        return Ok(()); // no commits yet
    };
    send(&Event::Commit { repo, sha });
    Ok(())
}
