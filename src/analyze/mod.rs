//! Analysis pipeline: tier0 (tools) → tier1 (heuristics) → tier2 (LLM).
//! Only tier1 is implemented in the first slice. See docs/SPEC.md §4.
pub mod tier0;
pub mod tier1;
pub mod tier2;

use crate::git::FileDiff;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Block,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
            Severity::Block => "block",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Author {
    Agent,
    Human,
    Unknown,
}

impl Author {
    pub fn as_str(&self) -> &'static str {
        match self {
            Author::Agent => "agent",
            Author::Human => "human",
            Author::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Open,
    FixedHuman,
    FixedAgent,
    Dismissed,
    Muted,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::FixedHuman => "fixed_human",
            Status::FixedAgent => "fixed_agent",
            Status::Dismissed => "dismissed",
            Status::Muted => "muted",
        }
    }
}

/// One review finding. See SPEC.md §5.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub repo: String,
    pub sha: String,
    pub file: String,
    pub line_start: u32,
    pub line_end: u32,
    pub lang: String,
    pub category: String,
    pub severity: Severity,
    pub confidence: f32,
    pub source: String,
    pub title: String,
    pub evidence: String,
    pub fix_hint: String,
    pub author: Author,
    pub touched_after_agent: bool,
    pub status: Status,
    pub created_at: String,
}

/// A finding's identity is *where and what*, not when it was found. Re-analysing
/// a commit must produce the same ids so results replace rather than accumulate.
pub fn finding_id(sha: &str, file: &str, line: u32, rule: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(sha.as_bytes());
    h.update(b"\x1f");
    h.update(file.as_bytes());
    h.update(b"\x1f");
    h.update(line.to_string().as_bytes());
    h.update(b"\x1f");
    h.update(rule.as_bytes());
    format!("f_{:x}", h.finalize())[..18].to_string()
}

/// Run the pipeline over a commit's added lines.
/// Tier 0 and tier 2 are not wired yet; tier 1 carries the first slice.
pub fn run(repo: &str, sha: &str, diffs: &[FileDiff]) -> Vec<Finding> {
    tier1::run(repo, sha, diffs)
}
