# revd — MVP Specification

**Thesis:** a grounding ledger. `revd` runs in the background, reviews every commit
silently, records what was shipped and who wrote it (you or an agent), and only
interrupts when something must be fixed. It describes events; it never grades people.

**Non-goals for v1:** no skill scores, no team views, no quizzes, no refresher
recommendations, no chat-window output. Anything that could be read as a judgment
about a person is out. It can be layered on later as a view over the same data.

---

## 1. Core invariants

1. **Zero latency in the authoring loop.** Hooks enqueue and exit in <10ms. All
   analysis is async.
2. **Silent by default.** The only push channels are: `pre-push` block, one OS
   notification per commit for `must-fix`, and a statusline count. Everything else
   is pull.
3. **Nothing enters the agent's context unless asked.** Agent access is via a CLI
   command / MCP tool it calls deliberately.
4. **Every number has a denominator and a link to a diff.** No adjectives, no
   ratings, no comparisons.
5. **Local only.** One SQLite file per user. No network except LLM calls.

---

## 2. Architecture

```
git post-commit ──┐
Claude Code hooks ┤──▶ revd hook (enqueue, exit) ──▶ unix socket ──▶ revd daemon
                  │                                                    │
                  │                     ┌──────────────────────────────┤
                  │                     ▼                              ▼
                  │              analysis pipeline               SQLite (~/.revd/revd.db)
                  │              T0 linters → T1 heuristics → T2 LLM  │
                  │                     │                              │
                  │                     ▼                              ▼
                  │              surfacing policy              revd show / revd ledger
                  │              notify · statusline · block   revd mcp (pull)
```

Single Rust binary, subcommands:

| Command | Role |
|---|---|
| `revd daemon` | long-running worker; owns socket + DB writes |
| `revd hook <event>` | called by git / Claude Code; enqueues JSON, exits |
| `revd install` | writes git hooks (global `core.hooksPath` shim) + Claude Code hook config |
| `revd show [sha]` | findings for a commit (default HEAD) |
| `revd ledger [--lang go] [--days 30]` | the counts-with-denominators table |
| `revd status` | one-line statusline output, e.g. `revd ⚠2` |
| `revd mute <rule>` / `revd dismiss <id>` | feedback |
| `revd mcp` | stdio MCP server exposing `get_findings`, `get_ledger` |

---

## 3. Events captured

### 3.1 Git (via `post-commit`, `pre-push`)
- `commit`: repo, sha, parent, branch, author, timestamp, files, +/- lines, message
- `push`: repo, range → triggers block check on unresolved `block` findings

### 3.2 Claude Code hooks (via `PostToolUse` on Edit/Write/MultiEdit, `Stop`)
- `agent_edit`: repo, file, timestamp, session id, line ranges written (from tool
  input: old/new string → compute range post-write), tool name
- `agent_bash`: command string (used to detect test runs before commit)
- `session_end`: session id, timestamp

Hook config installed into `~/.claude/settings.json` by `revd install`. The hook
command is `revd hook claude` reading the hook JSON from stdin; it must exit 0
always and never print to stdout.

### 3.3 Attribution (derived, computed at commit time)
For each hunk in the commit diff, match against `agent_edit` events for the same
file since the parent commit:
- `author = agent` if ≥80% of hunk lines fall in agent-written ranges and no
  human edit touched them after
- `author = human` otherwise
- `touched_after_agent = true` if the file mtime / a non-agent write occurred
  between the last agent edit and the commit (detected via fs watcher on the
  daemon side, or, simpler for v1: any change to the file not accounted for by
  agent_edit ranges)

v1 accepts approximate attribution. Precision improves later.

---

## 4. Analysis pipeline

Runs per commit, per changed file, in order. Each tier emits `Finding` records.
Later tiers are skipped for files where earlier tiers already produced a `block`.

### Tier 0 — existing tools (free)
Run only if the tool is on PATH and the language matches.
- Go: `go vet ./...`, `staticcheck ./...` (if present), `gofmt -l`
- TS/JS: `tsc --noEmit` (if tsconfig), `eslint --format json` (if config)
- Both: `gitleaks detect --no-git` on the diff (if present), else built-in regex
  secret scan

Outputs parsed to Findings with `source=tool:<name>`, `confidence=1.0`.
Restricted to lines in the diff (baseline noise is not surfaced).

### Tier 1 — heuristics (built-in, regex/AST-light)
Go:
- `err` assigned and not checked (`_ = err`, `err` shadowed/unused)
- `context.Background()` / `TODO()` inside non-main, non-test function
- `go func` with no ctx / done channel param
- `defer` missing after `os.Open`, `http.Get` body, `sql` rows
- map write without prior make (simple flow within function)
- `panic(` in non-main, non-test package

TS:
- `as any`, `as unknown as`, `: any` added (count, not per-line)
- `@ts-ignore` / `@ts-expect-error` without comment
- non-null `!` added
- `.catch(() => {})` / empty catch
- `forEach(async`
- floating promise: call to known-async fn without await/return/void (name list)

Both:
- debug leftovers: `console.log`, `fmt.Println`, `debugger`, `.only(`, `spew`
- `TODO|FIXME|HACK` without `[A-Z]+-\d+|#\d+` ref
- file >2k lines touched; single commit >800 changed lines (process finding)
- exported/public symbol changed with no test file in commit

`source=heuristic:<rule>`, `confidence` fixed per rule (0.6–0.95).

### Tier 2 — LLM (Haiku via `claude -p`)
Runs only for files where T0/T1 produced nothing at `high`, and only if the
diff for that file is 10–600 lines. Skipped entirely if `REVD_LLM=off`.

Invocation: `claude -p --model haiku --output-format json` with a prompt
containing: language, file path, the diff hunk(s), up to 60 lines of surrounding
context per hunk. Prompt requests **at most 3** findings as strict JSON matching
the Finding schema, restricted to correctness/security/robustness categories.
Style feedback is explicitly forbidden in the prompt.

Cached by `sha256(model + prompt)` in `llm_cache` table.
`source=llm:haiku`, `confidence` from model (0–1), floor-clamped to 0.5.

Budget: per-commit cap of 6 file calls; daily cap configurable (default 200).

### Tier 3 — on-demand deep review
`revd review [sha|range] --model sonnet|opus` — same pipeline, bigger model, no
caps, no size limit. Never automatic.

---

## 5. Finding schema

```json
{
  "id": "f_01J...",
  "repo": "/abs/path",
  "sha": "abc123",
  "file": "internal/worker/pool.go",
  "line_start": 41, "line_end": 47,
  "lang": "go",
  "category": "go.concurrency.goroutine_lifecycle",
  "severity": "block|high|medium|low",
  "confidence": 0.85,
  "source": "heuristic:go_func_no_ctx",
  "title": "goroutine started with no cancellation path",
  "evidence": "go func() { for { ... } }()  — no ctx/done param",
  "fix_hint": "accept ctx and select on ctx.Done()",
  "author": "agent|human|unknown",
  "touched_after_agent": false,
  "status": "open|fixed_human|fixed_agent|dismissed|muted",
  "created_at": "...", "resolved_at": null, "resolved_sha": null
}
```

`category` is a dotted path into the taxonomy (`docs/TAXONOMY.md`). Only
correctness / security / robustness / tests / types nodes exist in v1. Style nodes
are not defined; T0 style output (gofmt, eslint style rules) is dropped.

### Resolution detection
On each new commit, open findings for files in the diff are re-checked: if the
finding's line range no longer matches the pattern (T0/T1 re-run; T2 findings
re-checked by re-running the same heuristic, or marked `fixed_unknown` if the
lines were rewritten), status becomes `fixed_human` or `fixed_agent` based on
attribution of the resolving hunk.

---

## 6. Surfacing policy

| Level | Trigger | Channel |
|---|---|---|
| **block** | secret / credential / CVE dep (T0 only) | `pre-push` exits 1 with list; notification |
| **alert** | severity=high AND (source=tool OR confidence≥0.9) | one OS notification per commit (`osascript` on macOS, `notify-send` on Linux) + statusline |
| **quiet** | everything else | DB only; `revd show` |
| **pull** | any | `revd show`, `revd ledger`, MCP |

Rules:
- Max 1 notification per commit; body lists ≤3 findings, one line each.
- `revd mute <rule>` suppresses a rule from alert (still recorded as quiet).
- A rule dismissed 5× in 30 days auto-demotes to quiet and prints a one-time notice.
- Notifications carry the `revd show <sha>` command to copy.

Statusline: `revd status` prints `revd ⚠2` (open alert-level findings on HEAD)
or empty string. Integrates with Claude Code statusline via `revd install`.

---

## 7. Ledger (the dashboard, v1 = terminal table)

`revd ledger --days 30 [--lang go] [--repo .]`

Columns per `(lang, category)`:

```
Go · 30d · 2,140 lines changed (agent 61% / human 39%)

category                  findings  agent-wrote  human-wrote  fixed-human  fixed-agent  open  dismissed
go.errors.unchecked            0           —            —           —            —        —       —
go.context.background          4           4            0           0            4        0       0
go.concurrency.lifecycle       2           2            0           1            1        0       0
touch-after-agent: 12% of agent-written lines edited before commit
tests run before commit: 18 / 41 commits
```

Descriptive only. No sorting by "worst", no coloring by judgment; sort by category
name. Every row expands with `revd ledger --category go.context.background` to
list the findings with `sha:file:line`.

---

## 8. Storage (SQLite, `~/.revd/revd.db`, WAL mode)

```sql
repos(id, path, name, first_seen)
commits(sha PK, repo_id, parent, branch, ts, author, msg, files, adds, dels,
        agent_lines, human_lines, tests_ran_before INT)
agent_edits(id, repo_id, file, session_id, ts, line_start, line_end, tool)
findings(... as schema §5 ...)
rule_state(rule PK, muted INT, dismiss_count, last_dismissed)
llm_cache(key PK, response, ts)
events(id, ts, kind, payload JSON)   -- raw hook payloads, for replay/debug
```

Retention: `events` pruned at 90 days; everything else kept.

---

## 9. Agent integration (pull only)

`revd mcp` — stdio MCP server with two tools:
- `get_findings(sha?, path?, min_severity?)` → JSON list
- `get_ledger(days?, lang?)` → JSON table

Registered via `revd install` into `~/.claude.json` mcpServers. Nothing in
Claude Code's hooks ever prints findings; the `PostToolUse` hook is
capture-only.

---

## 10. Config (`~/.revd/config.toml`)

```toml
llm = "haiku"          # or "off"
daily_llm_cap = 200
notify = true
window_days = 30
[muted]
rules = []
```

Per-repo overrides in `.revd.toml` (e.g. `llm = "off"` for vendored repos).

---

## 11. Repo layout (Rust)

```
src/
  main.rs          clap entry, subcommand dispatch
  daemon/          unix socket server, queue, worker loop (tokio)
  hook/            git + claude hook handlers (stdin → enqueue, exit 0)
  git/             shell out to `git`, diff → hunk model
  attrib/          agent_edit ↔ hunk matching
  analyze/
    mod.rs         pipeline orchestrator, Finding type
    tier0/         tool runners + output parsers (govet, staticcheck, tsc, eslint, gitleaks)
    tier1/         heuristic rules: go.rs, ts.rs, common.rs
    tier2/         `claude -p` runner, prompt template, cache
  taxonomy.rs      category tree as const table
  store/           rusqlite, migrations (embedded SQL)
  surface/         policy, notify (osascript / notify-send), statusline
  ledger/          aggregation queries + table render
  mcp/             stdio JSON-RPC server
docs/SPEC.md  docs/TAXONOMY.md  docs/ROADMAP.md
```

Crates: `clap` (derive), `tokio` (rt, net, process), `rusqlite` (bundled),
`serde`/`serde_json`, `regex`, `anyhow`, `thiserror`, `toml`, `directories`,
`ulid`, `sha2`, `tracing`/`tracing-subscriber`, `comfy-table` (ledger render),
`notify` (fs watcher, later). Git via `std::process::Command` — not `git2`, to
keep the build simple and behaviour identical to the user's git.

Toolchain: stable (1.98+), `edition = "2024"`. `cargo clippy -D warnings`,
`cargo fmt --check` in CI. Release profile: `lto = "thin"`, `strip = true`.

## 12. MVP acceptance

- [ ] `revd install` in a repo; commit; `revd show` lists T0+T1 findings within 5s
- [ ] `post-commit` hook adds <10ms to commit wall time
- [ ] agent-written hunks attributed correctly on a scripted Claude Code session
- [ ] a committed fake secret blocks `git push` with a clear message
- [ ] one notification per commit, none when there's nothing at alert level
- [ ] `revd ledger` renders the table over 30 days of real use
- [ ] `revd mcp` answers `get_findings` from Claude Code
- [ ] `REVD_LLM=off` yields a fully working tool with no network

## 13. Build order

1. store + schema + `revd hook` enqueue + daemon loop (no analysis) — events land in DB
2. git diff/hunk model + commits table
3. tier1 heuristics for Go + TS, `revd show`
4. surfacing: statusline, notification, mute/dismiss
5. tier0 tool runners
6. Claude Code hook capture + attribution
7. `revd ledger`
8. tier2 LLM + cache + caps
9. `revd mcp`
10. `pre-push` block, `revd install` polish
