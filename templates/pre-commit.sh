#!/bin/sh
# Installed by revd. Fast, deterministic checks on staged changes only.
#
# Design rules:
#   - only secrets BLOCK a commit; everything else warns
#   - every tool is optional: a missing binary is skipped, never an error
#   - bypass with:  git commit --no-verify
set -u

fail=0
say() { printf '%s\n' "$*" >&2; }

# --- blocking: secrets -----------------------------------------------------
if command -v gitleaks >/dev/null 2>&1; then
  if ! gitleaks git --staged --no-banner --redact -v >/dev/null 2>&1; then
    say "revd: secret detected in staged changes — commit blocked."
    say "      review with: gitleaks git --staged -v"
    say "      bypass with: git commit --no-verify"
    fail=1
  fi
fi

# --- advisory: everything else ---------------------------------------------
__REVD_ADVISORY__

exit $fail
