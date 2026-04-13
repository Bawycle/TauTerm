#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# sign-release.sh
#
# Collects build artifacts and generates SHA256SUMS.
# Run after `pnpm tauri build`.
#
# Usage:
#   ./scripts/sign-release.sh [options]
#
# Options:
#   --output <DIR>    Output directory for release artifacts (default: release/)
#   --help, -h        Show this help message
#
# Prerequisites:
#   - A successful `pnpm tauri build` (artifacts in src-tauri/target/release/bundle/)
#
# Output:
#   <output_dir>/
#     ├── tau-term_<version>_amd64.AppImage
#     └── SHA256SUMS

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUNDLE_DIR="$PROJECT_ROOT/src-tauri/target/release/bundle"

# ── Defaults ──────────────────────────────────────────────────────────
OUTPUT_DIR="$PROJECT_ROOT/release"

# ── Helpers ───────────────────────────────────────────────────────────
usage() {
    sed -n '3,/^$/s/^# \?//p' "$0"
    exit 0
}

die() {
    echo "error: $1" >&2
    exit 1
}

info() {
    echo ":: $1"
}

# ── Parse arguments ──────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --output)
            [[ -n "${2:-}" ]] || die "--output requires a value"
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -h|--help)
            usage
            ;;
        *)
            die "unknown option: $1"
            ;;
    esac
done

# ── Verify bundle directory exists ───────────────────────────────────
[[ -d "$BUNDLE_DIR" ]] || die "bundle directory not found at $BUNDLE_DIR. Run 'pnpm tauri build' first."

# ── Collect artifacts ────────────────────────────────────────────────
mapfile -t ARTIFACTS < <(find "$BUNDLE_DIR" -type f -name "*.AppImage" | sort)

[[ ${#ARTIFACTS[@]} -gt 0 ]] || die "no AppImage found in $BUNDLE_DIR. Run 'pnpm tauri build' first."

info "Found ${#ARTIFACTS[@]} artifact(s):"
for f in "${ARTIFACTS[@]}"; do
    echo "   $(basename "$f")"
done

# ── Prepare output directory ─────────────────────────────────────────
mkdir -p "$OUTPUT_DIR"

# ── Copy artifacts ───────────────────────────────────────────────────
info "Copying artifacts to $OUTPUT_DIR/"
for f in "${ARTIFACTS[@]}"; do
    cp "$f" "$OUTPUT_DIR/"
done

# ── Generate SHA256SUMS ──────────────────────────────────────────────
info "Generating SHA256SUMS"
CHECKSUMS_FILE="$OUTPUT_DIR/SHA256SUMS"

(
    cd "$OUTPUT_DIR"
    sha256sum -- *.AppImage | sort -k2 > SHA256SUMS
)

echo "   $(wc -l < "$CHECKSUMS_FILE") checksum(s) written"

# ── Summary ──────────────────────────────────────────────────────────
echo ""
info "Release artifacts ready in $OUTPUT_DIR/:"
(cd "$OUTPUT_DIR" && ls -1)
echo ""
info "Users can verify with:"
echo "   sha256sum --check SHA256SUMS"
