#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# run-keyring-tests.sh
#
# Host-side orchestration script for SecretService integration tests.
# Builds the Podman image (if needed) and runs the keyring integration tests
# in an isolated container with a live GNOME Keyring daemon.
#
# Usage:
#   ./scripts/run-keyring-tests.sh             # build image + run tests
#   ./scripts/run-keyring-tests.sh --no-build  # skip image build (use existing)
#   ./scripts/run-keyring-tests.sh --dry-run   # print commands without executing
#
# Exit codes:
#   0  all tests passed
#   1  one or more tests failed or container error

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
IMAGE_NAME="tauterm-keyring-test"
CONTAINERFILE="$PROJECT_ROOT/Containerfile.keyring-test"
BUILD=true
DRY_RUN=false

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------

for arg in "$@"; do
    case "$arg" in
        --no-build) BUILD=false ;;
        --dry-run)  DRY_RUN=true ;;
        --help|-h)
            echo "Usage: $0 [--no-build] [--dry-run]"
            echo ""
            echo "  --no-build  Skip image build; use existing '$IMAGE_NAME' image."
            echo "  --dry-run   Print commands without executing them."
            exit 0
            ;;
        *)
            echo "Unknown argument: $arg" >&2
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

# ---------------------------------------------------------------------------
# Pre-flight checks
# ---------------------------------------------------------------------------

if ! command -v podman &>/dev/null; then
    echo "ERROR: 'podman' is not installed or not in PATH." >&2
    echo "Install podman: https://podman.io/docs/installation" >&2
    exit 1
fi

echo "[keyring-test] Project root: $PROJECT_ROOT"
echo "[keyring-test] Image: $IMAGE_NAME"

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

if "$BUILD"; then
    echo "[keyring-test] Building image $IMAGE_NAME ..."
    run podman build \
        --tag "$IMAGE_NAME" \
        --file "$CONTAINERFILE" \
        "$PROJECT_ROOT"
    echo "[keyring-test] Image built successfully."
else
    echo "[keyring-test] Skipping image build (--no-build)."
fi

# ---------------------------------------------------------------------------
# Run tests
# ---------------------------------------------------------------------------
# --rm            remove container after exit
# --security-opt label=disable  disable SELinux/AppArmor labels so D-Bus Unix
#                               sockets work correctly in rootless Podman
# --cap-drop ALL  drop all capabilities (we don't need any elevated privileges)

echo "[keyring-test] Running SecretService integration tests..."
run podman run \
    --rm \
    --security-opt label=disable \
    --cap-drop ALL \
    "$IMAGE_NAME"

echo "[keyring-test] All tests passed."
