# Roadmap

## v0.1 — MVP (see SPEC.md §12/§13)
Ledger + silent review + attribution + pull-only agent access.

## v0.2 — Fidelity
- fs watcher for precise touch-after-agent detection
- resolution detection for T2 findings (re-ask model "is this still present?")
- eslint/tsc/staticcheck parsers hardened; gitleaks bundled config
- `revd review --model sonnet` deep on-demand review
- `.revd.toml` per-repo config; monorepo lang detection
- Rust and Python heuristics (dogfood)

## v0.3 — Ledger UX
- `revd web` — local single-page dashboard over the same queries
- per-category drill-down with diff rendering
- weekly digest (`revd digest`, optional Monday notification) — counts only
- export (JSON/CSV)

## v0.4 — Opt-in grounding tools (all user-initiated, off by default)
Only after v0.1–0.3 have been used for real and the descriptive ledger feels fair.
- `revd quiz <category>` — explain-back on your own merged agent-written code
- `revd drill <category>` — mutated diffs from your own history
- per-category reading links (curated, not generated)
Everything here is a view over existing data; no new capture, no scores.

## Explicitly not planned
- team/manager views, per-person comparison, percentiles, any global score
- pushing findings into the agent's context automatically
- cloud sync
