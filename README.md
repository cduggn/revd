# revd

Detect what a project is written in, then set up the checks those languages
deserve — linters, secret scanning, dependency audits, and a pre-commit hook —
in one command.

```sh
revd plan     # what would change, writes nothing
revd init     # write missing configs + install the pre-commit hook
revd doctor   # what's installed, what's missing, where the gaps are
revd tools    # every tool revd knows about
```

Languages: Go, TypeScript, Python, Rust, Java.

## Principles

- **Detection is deterministic.** `go.mod`, `Cargo.toml`, `pyproject.toml`,
  `package.json`, `pom.xml`. No AI, no network, no account, no API key.
- **revd never installs anything.** It prints the command; you stay in control.
- **Your configs are yours.** Generated files are committed to your repo and
  never overwritten. An existing config is left alone.
- **Only secrets block a commit.** Everything else is advisory. A missing tool
  is skipped, never an error.
- **A foreign pre-commit hook is never touched** without `--force`.

## Adding a tool

Define one `ToolSpec` in `src/tools/registry.rs` and add it to `ALL`. There is
no trait to implement and no dispatch to wire — a tool is data:

```rust
pub const RUFF: ToolSpec = ToolSpec {
    id: "ruff",
    role: Role::Lint,
    langs: &[Lang::Python],
    binary: "ruff",
    probe: None,
    version_args: &["--version"],
    install: Install::Brew("ruff"),
    config_files: &["ruff.toml", ".ruff.toml"],
    template: Some(Template { path: "ruff.toml", contents: include_str!("../../templates/ruff.toml") }),
    scan_args: &["check", "--output-format=sarif"],
    output: Output::Sarif,
    why: "replaces flake8, isort, bandit and pyupgrade in one fast binary",
    note: "MIT. Also covers formatting via `ruff format`.",
};
```

`revd tools` prints the registry, so the extension point is visible from the CLI.

## Status

`init` / `plan` / `doctor` / `tools` work. 17 tools across 5 languages.

The commit-analysis layer — AI review of what tools can't check, and the
activity ledger — is not built yet. See `docs/ROADMAP.md`.
