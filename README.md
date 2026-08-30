# revd

A grounding ledger. `revd` reviews every commit silently in the background,
records what shipped and whether you or an agent wrote it, and only interrupts
when something must be fixed. It describes events; it never grades people.

- Zero latency in the authoring loop — hooks enqueue and exit
- Silent by default — one notification per commit at most, everything else is pull
- Nothing enters the agent's context unless the agent asks (`revd mcp`)
- Every number has a denominator and a link to a diff

See [docs/SPEC.md](docs/SPEC.md), [docs/TAXONOMY.md](docs/TAXONOMY.md), [docs/ROADMAP.md](docs/ROADMAP.md).

## Try it

```sh
cargo build --release
./target/release/revd install     # writes .git/hooks/post-commit in the current repo
./target/release/revd daemon &    # background worker
git commit -m "..."               # analysed asynchronously
./target/release/revd show        # findings for HEAD
./target/release/revd status      # statusline: "revd ⚠3" or nothing
```

## Status

End-to-end slice works: commit → hook → socket → daemon → tier1 heuristics →
SQLite → `revd show`. Hook overhead is ~11ms on top of `git commit`.

Implemented: git hook install, daemon, diff parsing, 10 tier1 rules (Go/TS/common),
storage, `show`, `status`, one-notification-per-commit.

Not yet: tier0 linters, tier2 LLM, Claude Code hook capture, attribution,
`ledger`, `mcp`, `mute`/`dismiss`, `pre-push` blocking. See docs/ROADMAP.md.
