#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# check-all.sh
#
# Local CI equivalent — runs the full verification suite in logical order.
# Mirrors what CI would do; intended to be run manually before pushing,
# or automatically via the git pre-push hook.
#
# Usage:
#   ./scripts/check-all.sh [options]
#
# Options:
#   --skip-containers   Skip Podman integration tests (step 4)
#   --skip-e2e          Skip E2E tests via WebdriverIO + tauri-driver (step 6)
#   --skip-audit        Skip cargo audit + cargo deny (step 5)
#   --no-build          Pass --no-build to Podman scripts (reuse existing images)
#   --fast              Alias for --skip-containers --skip-e2e --skip-audit
#   --check-version     Also check that version strings are in sync (opt-in)
#   --dry-run           Print commands without executing them
#   --install-hooks     Install the git pre-push hook and exit
#   --help, -h          Show this help message
#
# Installing the git hook:
#   Run:  ./scripts/check-all.sh --install-hooks
#   Or manually:  ln -sf ../../scripts/pre-push .git/hooks/pre-push
#
# Exit codes:
#   0  all active steps passed
#   1  one or more steps failed or preflight check failed

set -euo pipefail

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------

SKIP_CONTAINERS=false
SKIP_E2E=false
SKIP_AUDIT=false
CHECK_VERSION=false
NO_BUILD=false
DRY_RUN=false

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------

for arg in "$@"; do
    case "$arg" in
        --skip-containers) SKIP_CONTAINERS=true ;;
        --skip-e2e)        SKIP_E2E=true ;;
        --skip-audit)      SKIP_AUDIT=true ;;
        --no-build)        NO_BUILD=true ;;
        --check-version)   CHECK_VERSION=true ;;
        --fast)
            SKIP_CONTAINERS=true
            SKIP_E2E=true
            SKIP_AUDIT=true
            ;;
        --dry-run) DRY_RUN=true ;;
        --install-hooks)
            target="$PROJECT_ROOT/.git/hooks/pre-push"
            source_rel="../../scripts/pre-push"
            ln -sf "$source_rel" "$target"
            echo "[check-all] Hook installed: $target -> $source_rel"
            exit 0
            ;;
        --help|-h)
            sed -n '2,/^set -euo pipefail/{ /^set -euo pipefail/d; s/^# \{0,1\}//; p }' "$0"
            exit 0
            ;;
        *)
            echo "Unknown argument: $arg" >&2
            echo "Run '$0 --help' for usage." >&2
            exit 1
            ;;
    esac
done

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

run() {
    if "$DRY_RUN"; then
        echo "[dry-run] $*"
    else
        "$@"
    fi
}

step_header() {
    echo ""
    echo "[check] === Step $1: $2 ==="
}

step_skipped() {
    echo "[check] (skipped)"
    SKIPPED_STEPS+=("$1")
}

passed_steps=()
SKIPPED_STEPS=()

step_passed() {
    passed_steps+=("$1")
}

# ---------------------------------------------------------------------------
# Helpers — disk space
# ---------------------------------------------------------------------------

# check_disk_space <required_gb> <path> <context>
# Exits with code 1 if available space on the partition containing <path>
# is below <required_gb>. No-ops in dry-run mode (space cannot be consumed).
check_disk_space() {
    local required_gb="$1"
    local check_path="$2"
    local context="$3"

    if "$DRY_RUN"; then
        echo "[dry-run] check_disk_space ${required_gb}G on $(df -P "$check_path" | awk 'NR==2{print $6}') for: $context"
        return
    fi

    # df -P: POSIX portable output; column 4 = Available (KiB)
    local available_kb
    available_kb=$(df -P "$check_path" | awk 'NR==2{print $4}')
    local available_gb=$(( available_kb / 1024 / 1024 ))

    if [ "$available_gb" -lt "$required_gb" ]; then
        echo "[check] ERROR: Insufficient disk space for: $context" >&2
        echo "[check]        Partition: $(df -P "$check_path" | awk 'NR==2{print $1}')" >&2
        echo "[check]        Required : ${required_gb} GB" >&2
        echo "[check]        Available: ${available_gb} GB" >&2
        echo "[check]        Free up space and retry. Suggestions:" >&2
        echo "[check]          cargo clean (in src-tauri/)            — removes Rust build artifacts" >&2
        echo "[check]          podman image prune                     — removes dangling images" >&2
        echo "[check]          podman container prune                 — removes stopped containers" >&2
        echo "[check]          podman volume prune                    — removes unused volumes" >&2
        echo "[check]          podman system prune                    — removes all of the above at once" >&2
        exit 1
    fi

    echo "[check] Disk space OK (${available_gb} GB available >= ${required_gb} GB required) for: $context"
}

# ---------------------------------------------------------------------------
# Pre-flight checks
# ---------------------------------------------------------------------------

echo "[check] Project root: $PROJECT_ROOT"
echo "[check] DRY_RUN=$DRY_RUN  SKIP_CONTAINERS=$SKIP_CONTAINERS  SKIP_E2E=$SKIP_E2E  SKIP_AUDIT=$SKIP_AUDIT"

missing_tools=()

if ! command -v cargo &>/dev/null; then
    missing_tools+=("cargo (Rust toolchain — https://rustup.rs)")
fi

if ! command -v pnpm &>/dev/null; then
    missing_tools+=("pnpm (Node package manager — https://pnpm.io/installation)")
fi

if ! "$SKIP_CONTAINERS" && ! command -v podman &>/dev/null; then
    missing_tools+=("podman (container runtime — https://podman.io/docs/installation)")
fi

if [ ${#missing_tools[@]} -gt 0 ]; then
    echo "[check] ERROR: The following required tools are missing:" >&2
    for tool in "${missing_tools[@]}"; do
        echo "  - $tool" >&2
    done
    exit 1
fi

# Global minimum — enough to run fmt, lint, and unit tests
check_disk_space 2 "$PROJECT_ROOT" "global (fmt + lint + unit tests)"

# Optional tools — warn but do not fail
if ! "$SKIP_AUDIT"; then
    if ! command -v cargo-audit &>/dev/null; then
        echo "[check] WARNING: 'cargo audit' not found. Install with: cargo install cargo-audit"
        echo "[check]          cargo audit step will be skipped."
        _CARGO_AUDIT_MISSING=true
    else
        _CARGO_AUDIT_MISSING=false
    fi

    if ! cargo deny --version &>/dev/null 2>&1; then
        echo "[check] WARNING: 'cargo deny' not found. Install with: cargo install cargo-deny"
        echo "[check]          cargo deny step will be skipped."
        _CARGO_DENY_MISSING=true
    else
        _CARGO_DENY_MISSING=false
    fi
fi

# ---------------------------------------------------------------------------
# Working directory — all pnpm commands expect to run from the project root
# ---------------------------------------------------------------------------

cd "$PROJECT_ROOT"

# ---------------------------------------------------------------------------
# Step 1 — Formatting (fail-fast)
# ---------------------------------------------------------------------------

step_header 1 "Formatting"

echo "[check] cargo fmt -- --check"
run cargo fmt --manifest-path "$PROJECT_ROOT/src-tauri/Cargo.toml" -- --check

echo "[check] pnpm prettier --check src/"
run pnpm prettier --check src/

echo "[check] check-spdx.sh"
run "$SCRIPT_DIR/check-spdx.sh"

step_passed "1: Formatting + SPDX"

# ---------------------------------------------------------------------------
# Step 2 — Lint
# ---------------------------------------------------------------------------

step_header 2 "Lint"

echo "[check] cargo clippy -- -D warnings"
run cargo clippy --manifest-path "$PROJECT_ROOT/src-tauri/Cargo.toml" -- -D warnings

echo "[check] pnpm check (TypeScript/Svelte)"
run pnpm check

step_passed "2: Lint"

# ---------------------------------------------------------------------------
# Step 3 — Unit tests
# ---------------------------------------------------------------------------

step_header 3 "Unit tests"

echo "[check] cargo nextest run"
run cargo nextest run --manifest-path "$PROJECT_ROOT/src-tauri/Cargo.toml"

echo "[check] pnpm vitest run"
run pnpm vitest run

step_passed "3: Unit tests"

# ---------------------------------------------------------------------------
# Step 4 — Podman integration tests
# ---------------------------------------------------------------------------

step_header 4 "Podman integration tests (SecretService + SSH)"

if "$SKIP_CONTAINERS"; then
    step_skipped "4: Podman integration tests"
else
    # Images tauterm-keyring-test + tauterm-ssh-test can reach ~1–2 GB total;
    # build layers may temporarily double that. 4 GB is a safe lower bound.
    # Podman stores images under ~/.local/share/containers/ by default.
    _PODMAN_STORAGE="${XDG_DATA_HOME:-$HOME/.local/share}/containers"
    # If the storage dir doesn't exist yet, check the home partition instead.
    _PODMAN_CHECK_PATH="$( [ -d "$_PODMAN_STORAGE" ] && echo "$_PODMAN_STORAGE" || echo "$HOME" )"
    check_disk_space 4 "$_PODMAN_CHECK_PATH" "Podman container images (step 4)"

    PODMAN_EXTRA_ARGS=()
    if "$NO_BUILD"; then
        PODMAN_EXTRA_ARGS+=("--no-build")
    fi
    if "$DRY_RUN"; then
        PODMAN_EXTRA_ARGS+=("--dry-run")
    fi

    echo "[check] run-keyring-tests.sh"
    run "$SCRIPT_DIR/run-keyring-tests.sh" "${PODMAN_EXTRA_ARGS[@]+"${PODMAN_EXTRA_ARGS[@]}"}"

    echo "[check] run-ssh-tests.sh"
    run "$SCRIPT_DIR/run-ssh-tests.sh" "${PODMAN_EXTRA_ARGS[@]+"${PODMAN_EXTRA_ARGS[@]}"}"

    step_passed "4: Podman integration tests"
fi

# ---------------------------------------------------------------------------
# Step 5 — Security / Licenses
# ---------------------------------------------------------------------------

step_header 5 "Security / Licenses (cargo audit + cargo deny)"

if "$SKIP_AUDIT"; then
    step_skipped "5: Security / Licenses"
else
    if "${_CARGO_AUDIT_MISSING:-false}"; then
        echo "[check] Skipping cargo audit (not installed)."
    else
        echo "[check] cargo audit (in src-tauri/)"
        # cargo audit reads Cargo.lock from the working directory; must run from src-tauri/
        if "$DRY_RUN"; then
            echo "[dry-run] (cd $PROJECT_ROOT/src-tauri && cargo audit)"
        else
            (cd "$PROJECT_ROOT/src-tauri" && cargo audit)
        fi
    fi

    if "${_CARGO_DENY_MISSING:-false}"; then
        echo "[check] Skipping cargo deny (not installed)."
    else
        echo "[check] cargo deny check"
        # cargo deny requires --manifest-path or a working directory with Cargo.toml
        run cargo deny --manifest-path "$PROJECT_ROOT/src-tauri/Cargo.toml" check
    fi

    step_passed "5: Security / Licenses"
fi

# ---------------------------------------------------------------------------
# Step 6 — E2E tests (WebdriverIO + tauri-driver)
# ---------------------------------------------------------------------------

step_header 6 "E2E tests (WebdriverIO + tauri-driver)"

if "$SKIP_E2E"; then
    step_skipped "6: E2E tests"
else
    # A Rust release build writes 4–6 GB into src-tauri/target/.
    # Combined with an existing debug target/, 8 GB is a conservative minimum.
    check_disk_space 8 "$PROJECT_ROOT" "E2E build — Rust release target/ (step 6)"

    echo "[check] VITE_PERF_INSTRUMENTATION=1 pnpm tauri build --no-bundle -- --features e2e-testing"
    echo "[check] NOTE: --features e2e-testing is mandatory (enables inject_pty_output)."
    run env VITE_PERF_INSTRUMENTATION=1 pnpm tauri build --no-bundle -- --features e2e-testing

    echo "[check] pnpm wdio"
    run pnpm wdio

    step_passed "6: E2E tests"
fi

# ---------------------------------------------------------------------------
# Step 7 — Version consistency (opt-in via --check-version)
# ---------------------------------------------------------------------------

step_header 7 "Version consistency (--check-version)"

if "$CHECK_VERSION"; then
    CARGO_VERSION=$(grep -m1 '^version\s*=' "$PROJECT_ROOT/src-tauri/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')
    PKG_VERSION=$(python3 -c "import json; print(json.load(open('$PROJECT_ROOT/package.json'))['version'])")
    README_VERSION=$(grep -oP 'Version: \K[^)]+(?=\])' "$PROJECT_ROOT/README.md" | head -1)

    VERSION_OK=true

    if [[ "$PKG_VERSION" != "$CARGO_VERSION" ]]; then
        echo "[check] MISMATCH: package.json version ($PKG_VERSION) != Cargo.toml ($CARGO_VERSION)"
        VERSION_OK=false
    fi

    if [[ -n "$README_VERSION" && "$README_VERSION" != "$CARGO_VERSION" ]]; then
        echo "[check] MISMATCH: README.md badge version ($README_VERSION) != Cargo.toml ($CARGO_VERSION)"
        VERSION_OK=false
    fi

    if "$VERSION_OK"; then
        echo "[check] All version strings match: $CARGO_VERSION"
        step_passed "7: Version consistency"
    else
        echo "[check] Run: ./scripts/bump-version.sh $CARGO_VERSION"
        exit 1
    fi
else
    step_skipped "7: Version consistency"
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

echo ""
echo "[check] ======================================="
echo "[check] Summary"
echo "[check] ======================================="

if [ ${#passed_steps[@]} -gt 0 ]; then
    echo "[check] Passed:"
    for s in "${passed_steps[@]}"; do
        echo "[check]   OK  Step $s"
    done
fi

if [ ${#SKIPPED_STEPS[@]} -gt 0 ]; then
    echo "[check] Skipped:"
    for s in "${SKIPPED_STEPS[@]}"; do
        echo "[check]   --  Step $s"
    done
fi

echo "[check] ======================================="
echo "[check] All active checks passed."
