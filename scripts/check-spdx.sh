#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# check-spdx.sh
#
# Verifies that all source files under src/ and src-tauri/src/ have an
# SPDX-License-Identifier header on their first line.
#
# Checked extensions: .rs, .ts, .js, .svelte, .html, .css
# Excluded: JSON, lock files, binaries (per project convention).
#
# Exit codes:
#   0  all files have a valid SPDX header
#   1  one or more files are missing the header

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

missing=0

# Only check files tracked by git — excludes generated code (paraglide, etc.)
while IFS= read -r file; do
    case "$file" in
        *.rs|*.ts|*.js|*.svelte|*.html|*.css) ;;
        *) continue ;;
    esac

    # Skip generated files (committed but auto-generated — no manual SPDX header).
    case "$file" in
        src/lib/ipc/bindings.ts) continue ;;
    esac

    if ! head -1 "$file" | grep -q 'SPDX-License-Identifier: MPL-2.0'; then
        echo "Missing SPDX header: $file"
        missing=$((missing + 1))
    fi
done < <(git -C "$PROJECT_ROOT" ls-files -- 'src/**/*.rs' 'src/**/*.ts' 'src/**/*.js' 'src/**/*.svelte' 'src/**/*.html' 'src/**/*.css' \
    'src-tauri/src/**/*.rs' | sort)

if [ "$missing" -gt 0 ]; then
    echo ""
    echo "ERROR: $missing file(s) missing SPDX-License-Identifier header."
    echo "Expected first line: // SPDX-License-Identifier: MPL-2.0 (or <!-- --> / /* */ variant)"
    exit 1
fi

echo "All source files have SPDX headers."
