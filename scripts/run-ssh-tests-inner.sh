#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
#
# run-ssh-tests-inner.sh
#
# Entrypoint executed INSIDE the Podman container.
# Starts sshd with the test configuration, waits for it to be ready,
# then runs the SSH integration tests via cargo-nextest.
#
# Not intended to be run directly on the host — use scripts/run-ssh-tests.sh.

set -euo pipefail

# ---------------------------------------------------------------------------
# 1. Start sshd
# ---------------------------------------------------------------------------

echo "[ssh-test] Starting sshd on port 2222..."
/usr/sbin/sshd -f /etc/ssh/sshd_config_test -D &
SSHD_PID=$!

# ---------------------------------------------------------------------------
# 2. Wait for sshd to be ready (up to 15 s)
# ---------------------------------------------------------------------------

echo "[ssh-test] Waiting for sshd to accept connections..."
MAX_WAIT=15
ELAPSED=0
until ssh \
        -o StrictHostKeyChecking=no \
        -o UserKnownHostsFile=/dev/null \
        -o ConnectTimeout=1 \
        -o PasswordAuthentication=no \
        -i /root/.ssh-test-keys/id_ed25519_test \
        -p 2222 \
        tauterm@127.0.0.1 \
        "exit 0" \
        2>/dev/null; do
    if [ "$ELAPSED" -ge "$MAX_WAIT" ]; then
        echo "[ssh-test] ERROR: sshd did not become ready after ${MAX_WAIT}s" >&2
        kill "$SSHD_PID" 2>/dev/null || true
        exit 1
    fi
    sleep 0.5
    ELAPSED=$((ELAPSED + 1))
done
echo "[ssh-test] sshd ready after ${ELAPSED}s."

# ---------------------------------------------------------------------------
# 3. Run the integration tests
# ---------------------------------------------------------------------------

cd /app

# Export environment variables that tests read to locate the test server and
# the pre-generated test key pair.
export TAUTERM_SSH_TEST_HOST=127.0.0.1
export TAUTERM_SSH_TEST_PORT=2222
export TAUTERM_SSH_TEST_USER=tauterm
export TAUTERM_SSH_TEST_PASSWORD=tauterm-test-pw
export TAUTERM_SSH_TEST_NOAUTH_USER=tauterm-noauth
# TAUTERM_TEST_PUBKEY_PATH is already set by the Containerfile ENV instruction.

# Capture the server's host key fingerprint for TOFU tests.
# ssh-keyscan emits "host key_type base64key" lines; we extract the ed25519 entry.
TAUTERM_SSH_TEST_HOST_KEY_LINE=$(ssh-keyscan -p 2222 -t ed25519 127.0.0.1 2>/dev/null | head -n 1)
export TAUTERM_SSH_TEST_HOST_KEY_LINE

echo "[ssh-test] Host key line: ${TAUTERM_SSH_TEST_HOST_KEY_LINE}"

exec cargo-nextest nextest run \
    --profile ssh \
    --test ssh_integration \
    "$@"
