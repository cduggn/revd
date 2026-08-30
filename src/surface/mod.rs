//! Surfacing policy: what interrupts the user and what stays in the DB.
//! See docs/SPEC.md §6.
use crate::analyze::{Finding, Severity, Status};
use crate::store;
use anyhow::Result;
use rusqlite::Connection;

/// Findings that earn a notification: high-confidence and serious.
fn is_alert(f: &Finding) -> bool {
    f.status == Status::Open
        && matches!(f.severity, Severity::High | Severity::Block)
        && f.confidence >= 0.9
}

pub fn apply(conn: &Connection, findings: &[Finding]) -> Result<()> {
    let alerts: Vec<&Finding> = findings.iter().filter(|f| is_alert(f)).collect();
    if alerts.is_empty() {
        return Ok(()); // silence is the default
    }
    let sha = &alerts[0].sha;
    if !store::claim_notification(conn, sha)? {
        return Ok(()); // already told them about this commit
    }
    let short = &sha[..7.min(sha.len())];
    let mut body: Vec<String> = alerts
        .iter()
        .take(3)
        .map(|f| format!("{}:{} {}", f.file, f.line_start, f.title))
        .collect();
    if alerts.len() > 3 {
        body.push(format!("+{} more", alerts.len() - 3));
    }
    notify(
        &format!("revd — {} finding(s) in {short}", alerts.len()),
        &body.join("\n"),
    );
    Ok(())
}

/// Best-effort desktop notification. Never fails the pipeline.
fn notify(title: &str, body: &str) {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification {} with title {}",
            applescript_quote(body),
            applescript_quote(title)
        );
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("notify-send")
            .arg(title)
            .arg(body)
            .output();
    }
    tracing::info!("notify: {title} — {body}");
}

#[cfg(target_os = "macos")]
fn applescript_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}
