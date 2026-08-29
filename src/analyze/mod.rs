//! Analysis pipeline: tier0 (tools) → tier1 (heuristics) → tier2 (LLM).
pub mod tier0;
pub mod tier1;
pub mod tier2;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Author {
    Agent,
    Human,
    Unknown,
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
}
