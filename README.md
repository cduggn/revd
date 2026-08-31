# revd

Detect what a project is written in, then set up the checks those languages
deserve — linters, secret scanning, dependency audits and a pre-commit hook —
in one command.

Supports **Go, TypeScript, Python, Rust and Java**.

```
$ revd plan
detected: go (services/api/go.mod), typescript (web/tsconfig.json)

secrets
  gitleaks         missing    will write config
      install: brew install gitleaks
lint
  golangci-lint    missing    will write config
      install: go install github.com/golangci/golangci-lint/v2/cmd/golangci-lint@latest
  oxlint           installed  will write config

hook
  will install pre-commit hook
```

## Install

Needs a Rust toolchain ([rustup.rs](https://rustup.rs)). Builds in a few
seconds — revd has three dependencies.

```sh
cargo install --locked --git https://github.com/cduggn/revd
```

To update, run the same command again. To uninstall: `cargo uninstall revd`.

### Without a Rust toolchain

> **Not available yet** — these need a tagged release. Until one is cut, use
> the `cargo install` command above.

Once `v0.1.0` is tagged:

```sh
# macOS / Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/cduggn/revd/releases/latest/download/revd-installer.sh | sh

# Windows
irm https://github.com/cduggn/revd/releases/latest/download/revd-installer.ps1 | iex

# or, fetching the prebuilt binary instead of compiling
cargo binstall revd
```

Targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
`x86_64-pc-windows-msvc`. Archives contain a top-level directory, so extract
with `--strip-components=1`. Each ships a `.sha256`, and each release carries a
combined `sha256.sum`.

### Build locally

```sh
git clone https://github.com/cduggn/revd && cd revd
cargo build --release      # ./target/release/revd
```

## Releasing

Packaging is handled by [cargo-dist](https://github.com/axodotdev/cargo-dist);
`.github/workflows/release.yml` is generated and should not be hand-edited.

```sh
cargo install cargo-dist --locked
dist plan                  # what a release would produce
dist init --yes            # regenerate CI after editing [workspace.metadata.dist]
git tag v0.1.0 && git push --tags
```

To add a Homebrew tap later: create a `cduggn/homebrew-tap` repo, add
`"homebrew"` to `installers` and `tap = "cduggn/homebrew-tap"` in
`[workspace.metadata.dist]`, set a `HOMEBREW_TAP_TOKEN` secret, then re-run
`dist init --yes`. It is left off until that repo exists, because a missing tap
fails the release job.

## Use

```sh
cd your-project

revd plan     # what would change — writes nothing
revd init     # write missing configs + install the pre-commit hook
revd doctor   # what's installed, what's missing, where the gaps are
revd tools    # every tool revd knows about
```

`revd init` is safe to re-run: it never overwrites a config you already have,
never touches a pre-commit hook it didn't write, and does nothing at all
outside a recognised project. Use `--force` to override, `--root <path>` to
point it elsewhere.

Then install whichever tools you actually want — revd prints the commands but
never installs anything itself.

## What it writes

| File | When |
|---|---|
| `.gitleaks.toml` | always (secrets apply to every project) |
| `.golangci.yml` | Go detected |
| `.oxlintrc.json` | TypeScript detected |
| `ruff.toml` | Python detected |
| `clippy.toml` | Rust detected |
| `.git/hooks/pre-commit` | in a git repo, if no foreign hook exists |

These are **your** files. Commit them; edit them freely. revd will not
overwrite them on a later run.

## The pre-commit hook

- **Only secrets block a commit.** Everything else warns.
- **Every tool is optional** — a missing binary is skipped, never an error.
- Bypass with `git commit --no-verify`.

## Principles

- **Detection is deterministic.** `go.mod`, `Cargo.toml`, `pyproject.toml`,
  `tsconfig.json`, `pom.xml`. No AI, no network, no account, no API key.
- **revd never installs anything.** It prints the command; you stay in control.
- **Your configs are yours.** Never overwritten, never managed invisibly.
- **Friction is what kills these tools**, so the defaults are quiet.

## Adding a tool

A tool is data, not code. Define one `ToolSpec` in `src/tools/registry.rs` and
add it to `ALL` — no trait to implement, no dispatch to wire:

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

Known gaps: Java has no lint ruleset or typecheck step; the SAST row
(opengrep) has no rules yet; there is no way to baseline an existing codebase,
so a large repo will show a big backlog on first run.

The commit-analysis layer — AI review of what tools can't check, plus an
activity ledger — is not built. See [docs/ROADMAP.md](docs/ROADMAP.md).

## License

MIT
