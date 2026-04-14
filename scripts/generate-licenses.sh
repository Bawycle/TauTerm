#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# generate-licenses.sh — Generate third-party license notices at build time.
#
# Output: THIRD-PARTY-NOTICES.md (Markdown, human-readable)
# Sorted by name, deduplicated by name@version@source.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_FILE="${REPO_ROOT}/THIRD-PARTY-NOTICES.md"

# ---------------------------------------------------------------------------
# Rust (Cargo) metadata — empty string on failure
# ---------------------------------------------------------------------------
CARGO_JSON=""
if command -v cargo >/dev/null 2>&1 && [ -f "${REPO_ROOT}/src-tauri/Cargo.toml" ]; then
  CARGO_JSON=$(cd "${REPO_ROOT}/src-tauri" && cargo metadata --format-version 1 2>/dev/null) || CARGO_JSON=""
fi

# ---------------------------------------------------------------------------
# Write cargo metadata to a temp file (too large for argv)
# ---------------------------------------------------------------------------
CARGO_TMP=$(mktemp)
trap 'rm -f "${CARGO_TMP}"' EXIT
printf '%s' "${CARGO_JSON}" > "${CARGO_TMP}"

# ---------------------------------------------------------------------------
# npm pattern arguments for the Python script
# ---------------------------------------------------------------------------
NPM_PATTERN_A="${REPO_ROOT}/node_modules/.pnpm/*/node_modules/*/package.json"
NPM_PATTERN_B="${REPO_ROOT}/node_modules/.pnpm/*/node_modules/@*/*/package.json"

# ---------------------------------------------------------------------------
# Python 3: merge + deduplicate + sort → write Markdown
# ---------------------------------------------------------------------------
python3 - "${CARGO_TMP}" "${NPM_PATTERN_A}" "${NPM_PATTERN_B}" "${OUTPUT_FILE}" <<'PYEOF'
import sys
import json
import glob
import os



cargo_file  = sys.argv[1]
npm_pat_a   = sys.argv[2]
npm_pat_b   = sys.argv[3]
output_file = sys.argv[4]

# Read cargo metadata from temp file
cargo_raw = ""
try:
    with open(cargo_file, "r", encoding="utf-8") as f:
        cargo_raw = f.read()
except OSError:
    pass

# --- Cargo ---
cargo_entries = []
if cargo_raw:
    try:
        metadata = json.loads(cargo_raw)
        for pkg in metadata.get("packages", []):
            if pkg.get("source") is None:
                continue
            name    = pkg.get("name", "")
            version = pkg.get("version", "")
            license = pkg.get("license") or "unknown"
            if name:
                cargo_entries.append({
                    "name": name,
                    "version": version,
                    "license": license,
                    "source": "cargo",
                })
    except (json.JSONDecodeError, KeyError):
        pass

# --- npm ---
npm_entries = []
seen_files  = set()

for pattern in (npm_pat_a, npm_pat_b):
    for path in glob.glob(pattern, recursive=False):
        real = os.path.realpath(path)
        if real in seen_files:
            continue
        seen_files.add(real)
        try:
            with open(path, "r", encoding="utf-8", errors="replace") as f:
                pkg = json.load(f)
            name    = pkg.get("name", "")
            version = pkg.get("version", "")
            lic     = pkg.get("license")
            if isinstance(lic, dict):
                lic = lic.get("type", "unknown")
            lic = lic or "unknown"
            if name:
                npm_entries.append({
                    "name": name,
                    "version": version,
                    "license": lic,
                    "source": "npm",
                })
        except (json.JSONDecodeError, OSError):
            pass

# --- Merge, deduplicate, sort ---
seen_keys = set()
unique    = []
for entry in cargo_entries + npm_entries:
    key = f"{entry['name']}@{entry['version']}@{entry['source']}"
    if key not in seen_keys:
        seen_keys.add(key)
        unique.append(entry)

unique.sort(key=lambda e: e["name"].lower())

# --- Write Markdown ---
cargo_deps = [e for e in unique if e["source"] == "cargo"]
npm_deps   = [e for e in unique if e["source"] == "npm"]

with open(output_file, "w", encoding="utf-8") as out:
    out.write("# Third-Party Notices\n\n")
    out.write("This file lists the third-party dependencies used by TauTerm and their licenses.\n\n")

    if cargo_deps:
        out.write(f"## Rust Crates ({len(cargo_deps)})\n\n")
        out.write("| Crate | Version | License |\n")
        out.write("|---|---|---|\n")
        for dep in cargo_deps:
            out.write(f"| {dep['name']} | {dep['version']} | {dep['license']} |\n")
        out.write("\n")

    if npm_deps:
        out.write(f"## npm Packages ({len(npm_deps)})\n\n")
        out.write("| Package | Version | License |\n")
        out.write("|---|---|---|\n")
        for dep in npm_deps:
            out.write(f"| {dep['name']} | {dep['version']} | {dep['license']} |\n")
        out.write("\n")

print(f"[generate-licenses] wrote {len(unique)} entries ({len(cargo_deps)} cargo + {len(npm_deps)} npm) to {output_file}")
PYEOF
