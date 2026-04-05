#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# run-keyring-tests-inner.sh
#
# Entrypoint executed INSIDE the Podman container.
# Starts a D-Bus session bus + virtual framebuffer + GNOME Keyring daemon,
# then runs the SecretService integration tests via cargo-nextest.
#
# Not intended to be run directly on the host — use scripts/run-keyring-tests.sh.

set -euo pipefail

# ---------------------------------------------------------------------------
# 1. Start Xvfb BEFORE dbus-run-session
# ---------------------------------------------------------------------------
# gnome-keyring's gcr-prompter is a D-Bus-activated service: dbus-daemon
# spawns it with dbus-daemon's own environment, NOT the calling shell's
# environment.  Xvfb and DISPLAY must be in the environment that dbus-daemon
# inherits at start-up, which means before dbus-run-session is invoked.

Xvfb :99 -screen 0 800x600x24 -nolisten tcp &
export DISPLAY=:99
sleep 0.5   # give Xvfb time to bind the socket

# ---------------------------------------------------------------------------
# 2. Start D-Bus session bus (DISPLAY already set in its environment)
# ---------------------------------------------------------------------------

exec dbus-run-session -- bash -euo pipefail -c '

# ---------------------------------------------------------------------------
# 3. Start GNOME Keyring daemon (secrets component only)
# ---------------------------------------------------------------------------

eval "$(echo -n "" | gnome-keyring-daemon \
    --unlock \
    --start \
    --daemonize \
    --components=secrets \
    2>/dev/null)" || true
export GNOME_KEYRING_CONTROL GNOME_KEYRING_PID SSH_AUTH_SOCK 2>/dev/null || true

# ---------------------------------------------------------------------------
# 4. Wait for SecretService to register on D-Bus (up to 10 s)
# ---------------------------------------------------------------------------

echo "[keyring-test] Waiting for org.freedesktop.secrets to appear on D-Bus..."
MAX_WAIT=10
ELAPSED=0
until dbus-send \
        --session \
        --dest=org.freedesktop.secrets \
        --print-reply \
        /org/freedesktop/secrets \
        org.freedesktop.DBus.Peer.Ping \
        >/dev/null 2>&1; do
    if [ "$ELAPSED" -ge "$MAX_WAIT" ]; then
        echo "[keyring-test] ERROR: org.freedesktop.secrets did not appear after ${MAX_WAIT}s" >&2
        exit 1
    fi
    sleep 0.5
    ELAPSED=$((ELAPSED + 1))
done
echo "[keyring-test] SecretService available after ${ELAPSED}s."

# ---------------------------------------------------------------------------
# 5. Force-initialise the default collection
# ---------------------------------------------------------------------------
# The first write to gnome-keyring triggers gcr-prompter to ask the user for
# a new keyring password.  With Xvfb providing a display, gcr-prompter now
# opens a GTK dialog on display :99.  As the only window on an unmanaged
# virtual display it receives keyboard focus automatically.  We send two
# Return keypresses (empty password + confirm) via xdotool in a background
# job while secret-tool store drives the D-Bus interaction in the foreground.
# A 30-second timeout prevents an indefinite hang if something goes wrong.

echo "[keyring-test] Initialising default collection (auto-dismissing prompt)..."
(
    # Give gcr-prompter time to open the window and receive focus.
    sleep 3
    # Empty password — first Return accepts the password field.
    xdotool key --clearmodifiers Return 2>/dev/null || true
    sleep 0.8
    # Second Return dismisses the "weak password" confirmation, if present.
    xdotool key --clearmodifiers Return 2>/dev/null || true
    sleep 0.5
    # Third Return for any additional confirmation step.
    xdotool key --clearmodifiers Return 2>/dev/null || true
) &
AUTO_DISMISS_PID=$!

timeout 30 secret-tool store \
    --label="TauTerm bootstrap" \
    service tauterm-bootstrap key init <<< "" \
    2>/dev/null || true

wait "$AUTO_DISMISS_PID" 2>/dev/null || true
secret-tool clear service tauterm-bootstrap key init >/dev/null 2>&1 || true
echo "[keyring-test] Default collection ready."

# ---------------------------------------------------------------------------
# 6. Run the integration tests
# ---------------------------------------------------------------------------

cd /app
exec cargo-nextest nextest run \
    --profile keyring \
    --test credentials_integration \
    "$@"
'
