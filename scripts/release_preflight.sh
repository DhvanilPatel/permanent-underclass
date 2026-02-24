#!/usr/bin/env bash
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

FAILURES=0
STEPS_RUN=0

run_step() {
  local name="$1"
  shift
  STEPS_RUN=$((STEPS_RUN + 1))
  echo
  echo "==> ${name}"
  if "$@"; then
    echo "[PASS] ${name}"
  else
    echo "[FAIL] ${name}"
    FAILURES=$((FAILURES + 1))
  fi
}

require_cmd() {
  local cmd="$1"
  if command -v "$cmd" >/dev/null 2>&1; then
    return 0
  fi
  echo "[MISSING] Required command not found: ${cmd}"
  FAILURES=$((FAILURES + 1))
  return 1
}

require_cargo_subcommand() {
  local subcommand="$1"
  if cargo "$subcommand" -V >/dev/null 2>&1; then
    return 0
  fi
  echo "[MISSING] Required cargo subcommand not found: cargo ${subcommand}"
  FAILURES=$((FAILURES + 1))
  return 1
}

run_secret_pattern_scan() {
  local pattern='(AKIA[0-9A-Z]{16}|ASIA[0-9A-Z]{16}|ghp_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{20,}|sk-[A-Za-z0-9]{20,}|-----BEGIN (RSA|EC|OPENSSH|PGP) PRIVATE KEY-----)'
  if rg -n --hidden -S "$pattern" . \
    -g '!.git/**' \
    -g '!target/**' \
    -g '!node_modules/**' \
    -g '!.venv/**' \
    -g '!.venv-tools/**'; then
    echo "Potential secret pattern detected"
    return 1
  fi
}

run_gitleaks_scan() {
  if gitleaks help 2>&1 | rg -q '\bdetect\b'; then
    gitleaks detect \
      --source . \
      --no-git \
      --config .gitleaks.toml \
      --max-target-megabytes 2 \
      --timeout 180 \
      --redact \
      --no-banner \
      --exit-code 1
  else
    gitleaks dir . \
      --config .gitleaks.toml \
      --max-target-megabytes 2 \
      --timeout 180 \
      --redact \
      --no-banner \
      --exit-code 1
  fi
}

echo "Release preflight started at $(date -u +"%Y-%m-%dT%H:%M:%SZ")"

if require_cmd cargo; then
  run_step "cargo fmt --check" cargo fmt --check
  run_step "cargo clippy --all-targets --all-features -- -D warnings" \
    cargo clippy --all-targets --all-features -- -D warnings
  run_step "cargo test --lib" cargo test --lib
  run_step "cargo test --test sim" cargo test --test sim

  if require_cargo_subcommand audit; then
    run_step "cargo audit" cargo audit
  fi
  if require_cargo_subcommand deny; then
    run_step "cargo deny check advisories" cargo deny check advisories
  fi
fi

if require_cmd npm; then
  run_step "npm audit --omit=dev" npm audit --omit=dev
  run_step "npm audit" npm audit
fi

if require_cmd rg; then
  run_step "secret pattern regex scan" run_secret_pattern_scan
fi

if require_cmd gitleaks; then
  run_step "gitleaks secret scan" run_gitleaks_scan
fi

echo
echo "Steps run: ${STEPS_RUN}"
if (( FAILURES == 0 )); then
  echo "PRE-FLIGHT RESULT: PASS"
  exit 0
fi

echo "PRE-FLIGHT RESULT: FAIL (${FAILURES} issue(s))"
exit 1
