# Finding taxonomy (v1)

Dotted paths. Only these nodes exist in v1. No style nodes. A rule that cannot be
mapped to a node here is not surfaced.

## common
- `common.secrets.credential`          — token/key/password literal          [block]
- `common.secrets.env_file`             — .env or similar committed            [block]
- `common.deps.vulnerable`              — new dep with known CVE (T0 only)     [block]
- `common.debug.leftover`               — console.log / fmt.Println / debugger / .only
- `common.todo.unreferenced`            — TODO/FIXME without ticket/issue ref
- `common.process.large_commit`         — >800 changed lines
- `common.process.large_file`           — touched file >2k lines
- `common.tests.untested_public_change` — public symbol changed, no test in commit
- `common.tests.assertionless`          — test added with no assertions
- `common.tests.sleep`                  — time-based sleeps in tests

## go
- `go.errors.unchecked`         — err assigned, not checked / `_ = err`
- `go.errors.shadowed`          — err shadowed in nested scope
- `go.errors.panic_in_lib`      — panic() outside main/test
- `go.context.background`       — context.Background()/TODO() in non-entry code
- `go.context.not_propagated`   — func makes network/db call, no ctx param
- `go.concurrency.lifecycle`    — goroutine with no cancellation path
- `go.concurrency.shared_state` — shared map/slice write without sync
- `go.concurrency.unbounded`    — goroutine per item with no limiter
- `go.resources.unclosed`       — Open/Get/Query with no defer Close
- `go.nil.map_write`            — write to nil map
- `go.nil.type_assert`          — unchecked type assertion
- `go.http.no_timeout`          — http.Client{} / http.Get without timeout

## ts
- `ts.types.any`                — `any` / `as any` / `as unknown as` added
- `ts.types.non_null`           — `!` assertion added
- `ts.types.suppression`        — @ts-ignore / @ts-expect-error without note
- `ts.types.exhaustiveness`     — switch on union without exhaustive default
- `ts.async.floating_promise`   — un-awaited promise
- `ts.async.foreach_async`      — forEach with async callback
- `ts.async.empty_catch`        — swallowed rejection
- `ts.async.no_timeout`         — fetch/axios without timeout/signal
- `ts.runtime.unvalidated_input`— JSON.parse / req.body used without schema

Severity defaults per node live in `src/taxonomy.rs`. Tier-2 (LLM) findings must
cite one of these paths or are discarded.
