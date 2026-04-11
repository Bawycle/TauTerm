#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# bench-vtebench.sh
#
# Run vtebench benchmarks against TauTerm and produce a Markdown report.
#
# TauTerm does not expose a `-e <command>` CLI flag.  vtebench is instead
# launched as the PTY shell by passing a wrapper script via the SHELL
# environment variable — TauTerm reads SHELL to determine which program to
# spawn inside the PTY (src-tauri/src/platform/pty_linux/backend.rs:69).
#
# vtebench measures terminal throughput using CPR (cursor position report,
# ESC[6n → ESC[row;colR) as a synchronisation primitive.  TauTerm implements
# the CPR response in its Rust PTY read task (session/pty_task/reader.rs),
# so the benchmark runs entirely in the Rust backend without requiring any
# frontend interaction.
#
# Usage:
#   ./scripts/bench-vtebench.sh         # run vtebench benchmarks against TauTerm
#   ./scripts/bench-vtebench.sh --help  # show usage and exit
#
# Prerequisites:
#   - Xvfb: virtual framebuffer (sudo apt install xvfb)
#   - xdotool: X11 automation tool (sudo apt install xdotool)
#   - vtebench: auto-installed from https://github.com/alacritty/vtebench if absent
#   - TauTerm release binary: auto-built via pnpm tauri build --no-bundle if absent
#
# Exit codes:
#   0  report generated successfully
#   1  missing prerequisite or benchmark failure

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TAUTERM_BIN="$PROJECT_ROOT/src-tauri/target/release/tau-term"
VTEBENCH_SRC="/tmp/vtebench-src"
REPORT_DIR="$PROJECT_ROOT/target/bench-reports"
TIMEOUT_SECS=120
WINDOW_TITLE="tau-term"

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------

for arg in "$@"; do
    case "$arg" in
        --help|-h)
            echo "Usage: $0"
            echo ""
            echo "Run vtebench benchmarks against TauTerm and produce a Markdown report."
            echo ""
            echo "Prerequisites (auto-installed if absent):"
            echo "  vtebench   cloned from https://github.com/alacritty/vtebench"
            echo "             and installed via cargo install"
            echo "  tau-term   built via pnpm tauri build --no-bundle if not found"
            echo "             at src-tauri/target/release/tau-term"
            echo ""
            echo "Required system tools (must be installed manually):"
            echo "  Xvfb       virtual framebuffer (sudo apt install xvfb)"
            echo "  xdotool    X11 automation (sudo apt install xdotool)"
            echo ""
            echo "Output: target/bench-reports/vtebench-<date>-<commit>.md"
            exit 0
            ;;
        *)
            echo "Unknown argument: $arg" >&2
            echo "Run $0 --help for usage." >&2
            exit 1
            ;;
    esac
done

# ---------------------------------------------------------------------------
# Pre-flight: required system tools
# ---------------------------------------------------------------------------

if ! command -v Xvfb &>/dev/null; then
    echo "ERROR: 'Xvfb' is not installed or not in PATH." >&2
    echo "Install it with: sudo apt install xvfb" >&2
    exit 1
fi

if ! command -v xdotool &>/dev/null; then
    echo "ERROR: 'xdotool' is not installed or not in PATH." >&2
    echo "Install it with: sudo apt install xdotool" >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# vtebench: install if absent
# ---------------------------------------------------------------------------

if ! command -v vtebench &>/dev/null; then
    echo "[bench-vtebench] vtebench not found — cloning and installing..."
    if [ ! -d "$VTEBENCH_SRC" ]; then
        git clone https://github.com/alacritty/vtebench "$VTEBENCH_SRC"
    fi
    cargo install --path "$VTEBENCH_SRC"
    echo "[bench-vtebench] vtebench installed."
fi

VTEBENCH_BIN="$(command -v vtebench)"
# vtebench does not support --version; derive version from the git commit of
# the cloned source so the report has a stable identifier.
VTEBENCH_VERSION="$(git -C "$VTEBENCH_SRC" rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
echo "[bench-vtebench] vtebench: git@${VTEBENCH_VERSION} ($VTEBENCH_BIN)"

# ---------------------------------------------------------------------------
# TauTerm binary: build if absent
# ---------------------------------------------------------------------------

if [ ! -f "$TAUTERM_BIN" ]; then
    echo "[bench-vtebench] TauTerm release binary not found — building..."
    echo "[bench-vtebench] Running: pnpm tauri build --no-bundle"
    cd "$PROJECT_ROOT"
    pnpm tauri build --no-bundle
    echo "[bench-vtebench] TauTerm built."
fi

echo "[bench-vtebench] TauTerm binary: $TAUTERM_BIN"

# ---------------------------------------------------------------------------
# Prepare output files and SHELL wrapper
# ---------------------------------------------------------------------------

DATFILE="/tmp/tau-vtebench-dat-$$.txt"
DONEFILE="/tmp/tau-vtebench-done-$$.txt"
SHELL_WRAPPER="/tmp/tau-vtebench-shell-$$.sh"

rm -f "$DATFILE" "$DONEFILE" "$SHELL_WRAPPER"

# Write the wrapper script that TauTerm will spawn as its PTY shell.
# vtebench is passed the dense_cells suite — the most representative single
# benchmark.  --max-secs 5 caps each sample; --max-samples 20 limits the run
# to ≤100 s even on a slow machine, well within the ${TIMEOUT_SECS} s budget.
#
# --silent   suppresses human-readable results on stdout (VT sequences still
#            go to the PTY master as intended — CPR timing is unaffected)
# --dat FILE writes Gnuplot-compatible timing data to a separate file so the
#            report contains only the benchmark numbers, not the raw VT data
#            (which would bloat the report to tens of MiB)
cat > "$SHELL_WRAPPER" <<'WRAPPER_EOF'
#!/usr/bin/env bash
WRAPPER_EOF

# Expand variables here (not in the heredoc) so paths are embedded literally.
cat >> "$SHELL_WRAPPER" <<WRAPPER_EOF
"$VTEBENCH_BIN" --benchmarks "$VTEBENCH_SRC/benchmarks/dense_cells" \
    --max-secs 5 --max-samples 20 --silent --dat "$DATFILE"
echo DONE > "$DONEFILE"
WRAPPER_EOF

chmod +x "$SHELL_WRAPPER"
echo "[bench-vtebench] SHELL wrapper written: $SHELL_WRAPPER"

# ---------------------------------------------------------------------------
# Choose a free virtual display number
# ---------------------------------------------------------------------------

DISPLAY_NUM="$(shuf -i 99-199 -n 1)"
while [ -e "/tmp/.X${DISPLAY_NUM}-lock" ]; do
    echo "[bench-vtebench] Display :${DISPLAY_NUM} is in use, trying another..."
    DISPLAY_NUM="$(shuf -i 99-199 -n 1)"
done

echo "[bench-vtebench] Using virtual display :${DISPLAY_NUM}"

# ---------------------------------------------------------------------------
# Start Xvfb
# ---------------------------------------------------------------------------

Xvfb ":${DISPLAY_NUM}" -screen 0 1280x800x24 -nolisten tcp &
XVFB_PID=$!
export DISPLAY=":${DISPLAY_NUM}"
sleep 0.5   # give Xvfb time to bind the socket
echo "[bench-vtebench] Xvfb started (pid $XVFB_PID, DISPLAY=$DISPLAY)"

trap 'echo "[bench-vtebench] Cleaning up..."; kill "$XVFB_PID" 2>/dev/null || true; rm -f "${DATFILE:-}" "${DONEFILE:-}" "${SHELL_WRAPPER:-}" 2>/dev/null || true' EXIT

# ---------------------------------------------------------------------------
# Launch TauTerm with the vtebench wrapper as the PTY shell
# ---------------------------------------------------------------------------

# Override SHELL so TauTerm spawns the vtebench wrapper instead of bash.
# TauTerm reads SHELL via std::env::var("SHELL") at PTY open time.
SHELL="$SHELL_WRAPPER" "$TAUTERM_BIN" &
TAUTERM_PID=$!
echo "[bench-vtebench] TauTerm started (pid $TAUTERM_PID, SHELL=$SHELL_WRAPPER)"

trap 'echo "[bench-vtebench] Cleaning up..."; kill "$XVFB_PID" "$TAUTERM_PID" 2>/dev/null || true; rm -f "${DATFILE:-}" "${DONEFILE:-}" "${SHELL_WRAPPER:-}" 2>/dev/null || true' EXIT

# ---------------------------------------------------------------------------
# Wait for TauTerm window to appear (confirms the PTY task has started)
# ---------------------------------------------------------------------------

echo "[bench-vtebench] Waiting for TauTerm window..."
WINDOW_WAIT=0
WINDOW_TIMEOUT=30
WINDOW_ID=""
until WINDOW_ID="$(xdotool search --onlyvisible --name "$WINDOW_TITLE" 2>/dev/null | head -1)" && [ -n "$WINDOW_ID" ]; do
    sleep 1
    WINDOW_WAIT=$((WINDOW_WAIT + 1))
    if [ "$WINDOW_WAIT" -ge "$WINDOW_TIMEOUT" ]; then
        echo "ERROR: TauTerm window did not appear within ${WINDOW_TIMEOUT}s." >&2
        exit 1
    fi
done
echo "[bench-vtebench] Window found (id $WINDOW_ID) — PTY task running."

# ---------------------------------------------------------------------------
# Wait for benchmark to complete (poll DONEFILE)
# ---------------------------------------------------------------------------

echo "[bench-vtebench] Waiting for vtebench to complete (timeout ${TIMEOUT_SECS}s)..."
ELAPSED=0
while [ ! -f "$DONEFILE" ]; do
    sleep 2
    ELAPSED=$((ELAPSED + 2))
    if [ "$ELAPSED" -ge "$TIMEOUT_SECS" ]; then
        echo "ERROR: vtebench did not complete within ${TIMEOUT_SECS} seconds." >&2
        echo "  Results file: $DATFILE" >&2
        if [ -f "$DATFILE" ]; then
            echo "  Results file contents:" >&2
            cat "$DATFILE" >&2
        else
            echo "  Results file was NOT created (vtebench may have failed to start)." >&2
        fi
        exit 1
    fi
done

echo "[bench-vtebench] Benchmark completed."

# ---------------------------------------------------------------------------
# Read results
# ---------------------------------------------------------------------------

RESULTS=""
if [ -f "$DATFILE" ]; then
    RESULTS="$(cat "$DATFILE")"
else
    RESULTS="(no output captured)"
fi

# ---------------------------------------------------------------------------
# Generate Markdown report
# ---------------------------------------------------------------------------

mkdir -p "$REPORT_DIR"

GIT_COMMIT="$(git -C "$PROJECT_ROOT" rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
ISO_DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
REPORT_FILE="$REPORT_DIR/vtebench-${TIMESTAMP}-${GIT_COMMIT}.md"

cat > "$REPORT_FILE" <<REPORT
# TauTerm vtebench Report

- **Date**: ${ISO_DATE}
- **Git commit**: ${GIT_COMMIT}
- **vtebench commit**: ${VTEBENCH_VERSION}

## Benchmarks run

- Suite: \`dense_cells\` from \`${VTEBENCH_SRC}/benchmarks/dense_cells\`
- Options: \`--max-secs 5 --max-samples 20\`

## Results (Gnuplot DAT format: benchmark_name sample_ms…)

\`\`\`
${RESULTS}
\`\`\`

## How to run Alacritty/foot for comparison

\`\`\`bash
# vtebench must run inside the terminal being measured.
# Replace VTEBENCH_SRC with the path to the alacritty/vtebench clone.
alacritty -e vtebench --benchmarks \$VTEBENCH_SRC/benchmarks/dense_cells
foot vtebench --benchmarks \$VTEBENCH_SRC/benchmarks/dense_cells
\`\`\`
REPORT

echo "[bench-vtebench] Report written to: $REPORT_FILE"

# Cleanup temp files.
rm -f "$DATFILE" "$DONEFILE" "$SHELL_WRAPPER"
