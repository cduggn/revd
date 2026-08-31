//! The tool registry.
//!
//! To add a tool: write a `ToolSpec` const below and add it to [`ALL`].
//! That is the whole extension process — the rest of revd is generic over it.
use super::{Install, Output, Role, Template, ToolSpec};
use crate::lang::Lang;

// ---------------------------------------------------------------- secrets --

pub const GITLEAKS: ToolSpec = ToolSpec {
    id: "gitleaks",
    role: Role::Secrets,
    langs: &[], // every project has secrets to leak
    binary: "gitleaks",
    probe: None,
    version_args: &["version"],
    install: Install::Brew("gitleaks"),
    config_files: &[".gitleaks.toml", "gitleaks.toml"],
    template: Some(Template {
        path: ".gitleaks.toml",
        contents: include_str!("../../templates/gitleaks.toml"),
    }),
    scan_args: &["git", "--staged", "--no-banner", "--redact"],
    output: Output::Sarif,
    why: "catches credentials before they reach history",
    note: "MIT. Last release 2026-03-21 — maintenance has slowed.",
};

// ------------------------------------------------------------------- sast --

pub const OPENGREP: ToolSpec = ToolSpec {
    id: "opengrep",
    role: Role::Sast,
    langs: &[],
    binary: "opengrep",
    probe: None,
    version_args: &["--version"],
    install: Install::Manual("see github.com/opengrep/opengrep releases (signed static binaries)"),
    config_files: &[".opengrep.yml", "opengrep.yml"],
    template: None, // rules are project-specific; we do not ship a corpus
    scan_args: &["scan", "--sarif"],
    output: Output::Sarif,
    why: "cross-language structural security rules",
    note: "LGPL-2.1 fork of Semgrep. Preferred over Semgrep: no telemetry, no \
           account, static binaries. Bring your own rules — Semgrep's registry \
           rules are no longer open source.",
};

// ------------------------------------------------------------------- lint --

pub const GOLANGCI_LINT: ToolSpec = ToolSpec {
    id: "golangci-lint",
    role: Role::Lint,
    langs: &[Lang::Go],
    binary: "golangci-lint",
    probe: None,
    version_args: &["--version"],
    install: Install::Go("github.com/golangci/golangci-lint/v2/cmd/golangci-lint@latest"),
    config_files: &[".golangci.yml", ".golangci.yaml", ".golangci.toml", ".golangci.json"],
    template: Some(Template {
        path: ".golangci.yml",
        contents: include_str!("../../templates/golangci.yml"),
    }),
    // built-in diff scoping: no need to filter its output ourselves
    scan_args: &["run", "--new-from-rev=HEAD~"],
    output: Output::Sarif,
    why: "bundles errcheck, staticcheck, go vet and ~100 more behind one binary",
    note: "GPL-3.0 — safe to shell out to, do not vendor or redistribute.",
};

pub const OXLINT: ToolSpec = ToolSpec {
    id: "oxlint",
    role: Role::Lint,
    langs: &[Lang::TypeScript],
    binary: "oxlint",
    probe: None,
    version_args: &["--version"],
    install: Install::Npm("oxlint"),
    config_files: &[".oxlintrc.json"],
    template: Some(Template {
        path: ".oxlintrc.json",
        contents: include_str!("../../templates/oxlintrc.json"),
    }),
    scan_args: &["--format=sarif"],
    output: Output::Sarif,
    why: "Rust-based, ~50x faster than ESLint, no node_modules needed to run",
    note: "MIT. Type-aware mode needs TypeScript 7+.",
};

pub const RUFF: ToolSpec = ToolSpec {
    id: "ruff",
    role: Role::Lint,
    langs: &[Lang::Python],
    binary: "ruff",
    probe: None,
    version_args: &["--version"],
    install: Install::Brew("ruff"),
    config_files: &["ruff.toml", ".ruff.toml"],
    template: Some(Template {
        path: "ruff.toml",
        contents: include_str!("../../templates/ruff.toml"),
    }),
    scan_args: &["check", "--output-format=sarif"],
    output: Output::Sarif,
    why: "replaces flake8, isort, bandit and pyupgrade in one fast binary",
    note: "MIT. Also covers formatting via `ruff format`.",
};

pub const CLIPPY: ToolSpec = ToolSpec {
    id: "clippy",
    role: Role::Lint,
    langs: &[Lang::Rust],
    binary: "cargo",
    probe: Some("cargo-clippy"),
    version_args: &["clippy", "--version"],
    install: Install::Manual("rustup component add clippy"),
    config_files: &["clippy.toml", ".clippy.toml"],
    template: Some(Template {
        path: "clippy.toml",
        contents: include_str!("../../templates/clippy.toml"),
    }),
    scan_args: &["clippy", "--all-targets", "--message-format=json"],
    output: Output::Json,
    why: "the Rust linter; catches correctness and idiom problems rustc allows",
    note: "Ships with rustup. No SARIF — emits cargo JSON diagnostics.",
};

pub const PMD: ToolSpec = ToolSpec {
    id: "pmd",
    role: Role::Lint,
    langs: &[Lang::Java],
    binary: "pmd",
    probe: None,
    version_args: &["--version"],
    install: Install::Brew("pmd"),
    config_files: &["pmd-ruleset.xml", "ruleset.xml"],
    template: None, // ruleset choice depends heavily on the project
    scan_args: &["check", "-f", "sarif"],
    output: Output::Sarif,
    why: "Java static analysis: unused code, complexity, error-prone patterns",
    note: "BSD-style. SpotBugs is the bytecode-level complement.",
};

// -------------------------------------------------------------- typecheck --

pub const GO_VET: ToolSpec = ToolSpec {
    id: "go-vet",
    role: Role::TypeCheck,
    langs: &[Lang::Go],
    binary: "go",
    probe: None,
    version_args: &["version"],
    install: Install::Builtin,
    config_files: &[],
    template: None,
    scan_args: &["vet", "-json", "./..."],
    output: Output::Json,
    why: "type-aware checks the compiler does not run",
    note: "Included in golangci-lint; listed separately as a zero-install fallback.",
};

pub const TSC: ToolSpec = ToolSpec {
    id: "tsc",
    role: Role::TypeCheck,
    langs: &[Lang::TypeScript],
    binary: "tsc",
    probe: None,
    version_args: &["--version"],
    install: Install::Npm("typescript"),
    config_files: &["tsconfig.json"],
    template: None, // never generate a tsconfig; it defines the project
    scan_args: &["--noEmit", "--pretty", "false"],
    output: Output::Text,
    why: "the only source of real type errors",
    note: "Must run whole-project: passing changed files gives a false green. \
           Text output only — needs a bespoke parser.",
};

pub const MYPY: ToolSpec = ToolSpec {
    id: "mypy",
    role: Role::TypeCheck,
    langs: &[Lang::Python],
    binary: "mypy",
    probe: None,
    version_args: &["--version"],
    install: Install::Pipx("mypy"),
    config_files: &["mypy.ini", ".mypy.ini"],
    template: None,
    scan_args: &["--no-error-summary"],
    output: Output::Text,
    why: "static types for Python, where annotations exist",
    note: "MIT. Configure in pyproject.toml if you already have one.",
};

pub const CARGO_CHECK: ToolSpec = ToolSpec {
    id: "cargo-check",
    role: Role::TypeCheck,
    langs: &[Lang::Rust],
    binary: "cargo",
    probe: None,
    version_args: &["--version"],
    install: Install::Builtin,
    config_files: &[],
    template: None,
    scan_args: &["check", "--all-targets", "--message-format=json"],
    output: Output::Json,
    why: "compiler errors without producing a binary",
    note: "Ships with Rust.",
};

// ------------------------------------------------------------------- deps --

pub const OSV_SCANNER: ToolSpec = ToolSpec {
    id: "osv-scanner",
    role: Role::Deps,
    langs: &[],
    binary: "osv-scanner",
    probe: None,
    version_args: &["--version"],
    install: Install::Brew("osv-scanner"),
    config_files: &["osv-scanner.toml"],
    template: None,
    scan_args: &["--format", "sarif", "."],
    output: Output::Sarif,
    why: "known vulnerabilities in your dependency tree, every ecosystem",
    note: "Apache-2.0. Genuine offline mode via --offline with a local database.",
};

// ----------------------------------------------------------------- format --

pub const GOFMT: ToolSpec = ToolSpec {
    id: "gofmt",
    role: Role::Format,
    langs: &[Lang::Go],
    binary: "gofmt",
    probe: None,
    version_args: &["-h"],
    install: Install::Builtin,
    config_files: &[],
    template: None,
    scan_args: &["-l", "."],
    output: Output::Text,
    why: "canonical Go formatting; no configuration, no debate",
    note: "Advisory only — formatting never blocks a commit.",
};

pub const RUSTFMT: ToolSpec = ToolSpec {
    id: "rustfmt",
    role: Role::Format,
    langs: &[Lang::Rust],
    binary: "rustfmt",
    probe: None,
    version_args: &["--version"],
    install: Install::Manual("rustup component add rustfmt"),
    config_files: &["rustfmt.toml", ".rustfmt.toml"],
    template: None,
    scan_args: &["--check"],
    output: Output::Text,
    why: "canonical Rust formatting",
    note: "Advisory only.",
};

pub const RUFF_FORMAT: ToolSpec = ToolSpec {
    id: "ruff-format",
    role: Role::Format,
    langs: &[Lang::Python],
    binary: "ruff",
    probe: None,
    version_args: &["--version"],
    install: Install::Brew("ruff"),
    config_files: &["ruff.toml", ".ruff.toml"],
    template: None, // shares ruff.toml with the lint spec
    scan_args: &["format", "--check"],
    output: Output::Text,
    why: "Black-compatible formatting, same binary as the linter",
    note: "Advisory only.",
};

pub const PRETTIER: ToolSpec = ToolSpec {
    id: "prettier",
    role: Role::Format,
    langs: &[Lang::TypeScript],
    binary: "prettier",
    probe: None,
    version_args: &["--version"],
    install: Install::Npm("prettier"),
    config_files: &[".prettierrc", ".prettierrc.json", "prettier.config.js"],
    template: None,
    scan_args: &["--check", "."],
    output: Output::Text,
    why: "formatting for the JS/TS ecosystem",
    note: "Advisory only. Biome is a faster single-binary alternative.",
};

pub const SPOTLESS: ToolSpec = ToolSpec {
    id: "spotless",
    role: Role::Format,
    langs: &[Lang::Java],
    binary: "mvn",
    probe: None,
    version_args: &["--version"],
    install: Install::Manual("add the spotless plugin to pom.xml or build.gradle"),
    config_files: &[],
    template: None,
    scan_args: &["spotless:check"],
    output: Output::Text,
    why: "formatting via your existing build tool",
    note: "Advisory only. Configured in the build file, not standalone.",
};

/// Every tool revd knows about. Add new specs here.
pub const ALL: &[ToolSpec] = &[
    GITLEAKS,
    OPENGREP,
    GOLANGCI_LINT,
    OXLINT,
    RUFF,
    CLIPPY,
    PMD,
    GO_VET,
    TSC,
    MYPY,
    CARGO_CHECK,
    OSV_SCANNER,
    GOFMT,
    RUSTFMT,
    RUFF_FORMAT,
    PRETTIER,
    SPOTLESS,
];
