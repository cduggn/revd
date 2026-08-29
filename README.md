# revd

A grounding ledger. `revd` reviews every commit silently in the background,
records what shipped and whether you or an agent wrote it, and only interrupts
when something must be fixed. It describes events; it never grades people.

- Zero latency in the authoring loop — hooks enqueue and exit
- Silent by default — one notification per commit at most, everything else is pull
- Nothing enters the agent's context unless the agent asks (`revd mcp`)
- Every number has a denominator and a link to a diff

See [docs/SPEC.md](docs/SPEC.md), [docs/TAXONOMY.md](docs/TAXONOMY.md), [docs/ROADMAP.md](docs/ROADMAP.md).

## Status
Pre-alpha scaffold. `cargo build` works; nothing is wired yet.
