#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# run-ssh-tests.sh
#
# Host-side orchestration script for SSH integration tests.
# Builds the Podman image (if needed) and runs the SSH integration tests
# in an isolated container with a real OpenSSH server.
#
# Usage:
#   ./scripts/run-ssh-tests.sh             # build image + run tests
#   ./scripts/run-ssh-tests.sh --no-build  # skip image build (use existing)
#   ./scripts/run-ssh-tests.sh --dry-run   # print commands without executing
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
IMAGE_NAME="tauterm-ssh-test"
CONTAINERFILE="$PROJECT_ROOT/Containerfile.ssh-test"
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

echo "[ssh-test] Project root: $PROJECT_ROOT"
echo "[ssh-test] Image: $IMAGE_NAME"

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

if "$BUILD"; then
    echo "[ssh-test] Building image $IMAGE_NAME ..."
    run podman build \
        --tag "$IMAGE_NAME" \
        --file "$CONTAINERFILE" \
        --memory 12g \
        "$PROJECT_ROOT"
    echo "[ssh-test] Image built successfully."
else
    echo "[ssh-test] Skipping image build (--no-build)."
fi

# ---------------------------------------------------------------------------
# Run tests
# ---------------------------------------------------------------------------
# --rm                    remove container after exit
# --security-opt label=disable  disable SELinux/AppArmor labels
# --cap-drop ALL          drop non-required capabilities
# --cap-add SETUID/SETGID/AUDIT_WRITE  sshd needs these to switch to the
#                         authenticated user and write PAM audit records

echo "[ssh-test] Running SSH integration tests..."
run podman run \
    --rm \
    --memory 4g \
    --security-opt label=disable \
    --cap-drop ALL \
    --cap-add SETUID \
    --cap-add SETGID \
    --cap-add AUDIT_WRITE \
    --cap-add SYS_CHROOT \
    --cap-add CHOWN \
    "$IMAGE_NAME"

echo "[ssh-test] All tests passed."
