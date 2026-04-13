#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# bump-version.sh — Bump the app version across all files that carry it.
#
# Usage:
#   ./scripts/bump-version.sh <version>
#
# Example:
#   ./scripts/bump-version.sh 0.2.0
#   ./scripts/bump-version.sh 0.2.0-beta
#
# Updates:
#   - src-tauri/Cargo.toml  (SSOT — [package].version)
#   - package.json          (cosmetic sync)
#   - README.md             (badge + status line)
#   - CHANGELOG.md          ([Unreleased] → [version] - date)
#
# Does NOT commit — the caller decides when to commit.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ── Helpers ───────────────────────────────────────────────────────────
die() {
    echo "error: $1" >&2
    exit 1
}

info() {
    echo ":: $1"
}

# ── Parse arguments ──────────────────────────────────────────────────
if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.2.0"
    exit 1
fi

NEW_VERSION="$1"

# Basic SemVer validation (with optional pre-release)
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    die "invalid version format: '$NEW_VERSION' (expected SemVer, e.g. 1.2.3 or 1.2.3-beta)"
fi

# ── Read current version from Cargo.toml (SSOT) ─────────────────────
CARGO_TOML="$PROJECT_ROOT/src-tauri/Cargo.toml"
CURRENT_VERSION=$(grep -m1 '^version\s*=' "$CARGO_TOML" | sed 's/.*"\(.*\)".*/\1/')

if [[ -z "$CURRENT_VERSION" ]]; then
    die "cannot read current version from $CARGO_TOML"
fi

if [[ "$CURRENT_VERSION" == "$NEW_VERSION" ]]; then
    die "version is already $NEW_VERSION — nothing to do"
fi

info "Bumping version: $CURRENT_VERSION → $NEW_VERSION"

# ── 1. Cargo.toml (SSOT) ────────────────────────────────────────────
info "Updating src-tauri/Cargo.toml"
sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" "$CARGO_TOML"

# ── 2. package.json ─────────────────────────────────────────────────
PACKAGE_JSON="$PROJECT_ROOT/package.json"
info "Updating package.json"
sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$NEW_VERSION\"/" "$PACKAGE_JSON"

# ── 3. README.md — badge + status line ──────────────────────────────
README="$PROJECT_ROOT/README.md"
info "Updating README.md"

# Badge: ![Version: X.Y.Z](https://img.shields.io/badge/version-X.Y.Z-yellow)
# shields.io requires -- for hyphens in the badge text
BADGE_VERSION=$(echo "$NEW_VERSION" | sed 's/-/--/g')
OLD_BADGE_VERSION=$(echo "$CURRENT_VERSION" | sed 's/-/--/g')
sed -i "s|version-${OLD_BADGE_VERSION}-yellow|version-${BADGE_VERSION}-yellow|" "$README"
sed -i "s|Version: ${CURRENT_VERSION}|Version: ${NEW_VERSION}|" "$README"

# Status line: **Status: beta (vX.Y.Z)** or similar
sed -i "s|v${CURRENT_VERSION}|v${NEW_VERSION}|g" "$README"

# ── 4. CHANGELOG.md — [Unreleased] → [version] - date ───────────────
CHANGELOG="$PROJECT_ROOT/CHANGELOG.md"
if [[ -f "$CHANGELOG" ]]; then
    TODAY=$(date +%Y-%m-%d)
    if grep -q '^\## \[Unreleased\]' "$CHANGELOG"; then
        info "Updating CHANGELOG.md: [Unreleased] → [$NEW_VERSION] - $TODAY"
        sed -i "s/^## \[Unreleased\]/## [$NEW_VERSION] - $TODAY/" "$CHANGELOG"
    else
        info "CHANGELOG.md: no [Unreleased] section found — skipping"
    fi
else
    info "CHANGELOG.md not found — skipping"
fi

# ── Summary ──────────────────────────────────────────────────────────
echo ""
info "Version bumped to $NEW_VERSION in:"
echo "   src-tauri/Cargo.toml"
echo "   package.json"
echo "   README.md"
[[ -f "$CHANGELOG" ]] && echo "   CHANGELOG.md"
echo ""
info "Review the changes, then commit:"
echo "   git add -A && git commit -m 'chore(release): bump version to $NEW_VERSION'"
