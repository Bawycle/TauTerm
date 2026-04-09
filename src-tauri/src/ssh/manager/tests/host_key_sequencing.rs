// SPDX-License-Identifier: MPL-2.0

//! SEC-HOSTKEY-SEQ-001/002 — Verifies that authentication is never attempted
//! before host key validation, and never attempted when host key is rejected.
//!
//! ## Why an in-process mock russh server
//!
//! The invariant is guaranteed by russh::client::connect() internally: when
//! Handler::check_server_key() returns Ok(false), russh returns Err(Error::UnknownKey)
//! from connect() without constructing a Handle. Auth methods can only be called on a
//! Handle, so no auth is possible. A future russh version that broke this would cause
//! SEC-HOSTKEY-SEQ-001 to receive Ok(handle), failing the is_err() assertion.
//!
//! A pure unit test on check_server_key() alone would not detect that regression.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use russh::server::Server as _;
use tokio::net::TcpListener;

// ---------------------------------------------------------------------------
// Mock server — minimal russh server, handles one connection per instance.
// ---------------------------------------------------------------------------

struct MockServer;

impl russh::server::Server for MockServer {
    type Handler = MockServerHandler;

    fn new_client(&mut self, _peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        MockServerHandler
    }
}

struct MockServerHandler;

impl russh::server::Handler for MockServerHandler {
    type Error = russh::Error;

    async fn auth_none(&mut self, _user: &str) -> Result<russh::server::Auth, Self::Error> {
        Ok(russh::server::Auth::Accept)
    }

    async fn auth_password(
        &mut self,
        _user: &str,
        _password: &str,
    ) -> Result<russh::server::Auth, Self::Error> {
        Ok(russh::server::Auth::Accept)
    }

    async fn auth_publickey(
        &mut self,
        _user: &str,
        _public_key: &russh::keys::PublicKey,
    ) -> Result<russh::server::Auth, Self::Error> {
        Ok(russh::server::Auth::Accept)
    }
}

// ---------------------------------------------------------------------------
// Client handler — rejects the host key (Unknown/Mismatch simulation).
// ---------------------------------------------------------------------------

struct RejectingKeyClient {
    key_check_called: Arc<AtomicBool>,
}

impl russh::client::Handler for RejectingKeyClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        self.key_check_called.store(true, Ordering::SeqCst);
        Ok(false) // Simulate Unknown or Mismatch host key → reject
    }
}

// ---------------------------------------------------------------------------
// Client handler — accepts the host key (Trusted simulation).
// ---------------------------------------------------------------------------

struct AcceptingKeyClient;

impl russh::client::Handler for AcceptingKeyClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true) // Simulate Trusted host key → accept
    }
}

// ---------------------------------------------------------------------------
// Fixture — spins up a mock server on an OS-assigned loopback port.
// The server task handles exactly one connection then exits.
// Using port 0 avoids port conflicts when tests run in parallel.
// ---------------------------------------------------------------------------

async fn spawn_mock_server() -> std::net::SocketAddr {
    let server_key = russh::keys::ssh_key::PrivateKey::random(
        &mut rand::rng(),
        russh::keys::ssh_key::Algorithm::Ed25519,
    )
    .expect("ephemeral Ed25519 server key generation must succeed");

    let mut server_config = russh::server::Config::default();
    server_config.keys.push(server_key);
    // Disable inactivity timeout — prevents the server from dropping the
    // connection before the handshake completes in a busy CI environment.
    server_config.inactivity_timeout = None;
    // Zero auth_rejection_time — avoids 1-second delay per auth attempt
    // in SEC-HOSTKEY-SEQ-002 (positive control test).
    server_config.auth_rejection_time = std::time::Duration::ZERO;
    server_config.auth_rejection_time_initial = Some(std::time::Duration::ZERO);
    let server_config = Arc::new(server_config);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind to loopback must succeed");
    let addr = listener.local_addr().expect("must have local addr");

    tokio::spawn(async move {
        let (stream, peer_addr) = listener.accept().await.expect("accept one connection");
        let handler = MockServer.new_client(Some(peer_addr));
        let _ = russh::server::run_stream(server_config, stream, handler).await;
    });

    addr
}

// ---------------------------------------------------------------------------
// SEC-HOSTKEY-SEQ-001
// ---------------------------------------------------------------------------

/// When check_server_key returns Ok(false), russh::client::connect() MUST return
/// Err(Error::UnknownKey). No Handle is constructed; auth methods are unreachable.
///
/// Regression contract: if a future russh version called auth methods despite
/// Ok(false), connect() would return Ok(handle) here and is_err() would fail,
/// making the regression immediately visible.
#[tokio::test]
async fn sec_hostkey_seq_001_rejected_key_prevents_connect_and_auth() {
    let addr = spawn_mock_server().await;
    let key_check_called = Arc::new(AtomicBool::new(false));
    let client = RejectingKeyClient {
        key_check_called: Arc::clone(&key_check_called),
    };

    let result =
        russh::client::connect(Arc::new(russh::client::Config::default()), addr, client).await;

    // Verify check_server_key was actually called (not a TCP-level failure).
    assert!(
        key_check_called.load(Ordering::SeqCst),
        "SEC-HOSTKEY-SEQ-001: check_server_key must be called during the SSH handshake"
    );

    // connect() must fail — the Handle must not be returned.
    assert!(
        result.is_err(),
        "SEC-HOSTKEY-SEQ-001: connect() must return Err when check_server_key returns Ok(false); \
         got Ok(handle) — russh no longer enforces host key rejection before auth"
    );

    // The error must be UnknownKey specifically (not a TCP or parse error),
    // confirming the host-key-rejection path was taken.
    match result {
        Err(russh::Error::UnknownKey) => {}
        Err(other) => panic!(
            "SEC-HOSTKEY-SEQ-001: error must be Error::UnknownKey when host key is rejected; \
             a different error variant indicates the rejection path may have changed in russh. \
             Got: {other:?}"
        ),
        Ok(_) => unreachable!("already asserted result.is_err() above"),
    }
}

// ---------------------------------------------------------------------------
// SEC-HOSTKEY-SEQ-002
// ---------------------------------------------------------------------------

/// Positive control: when check_server_key returns Ok(true), connect() MUST succeed
/// and return a Handle. Validates that the mock server fixture is functional,
/// making the rejection assertion in SEC-HOSTKEY-SEQ-001 meaningful.
#[tokio::test]
async fn sec_hostkey_seq_002_accepted_key_allows_connect() {
    let addr = spawn_mock_server().await;

    let result = russh::client::connect(
        Arc::new(russh::client::Config::default()),
        addr,
        AcceptingKeyClient,
    )
    .await;

    assert!(
        result.is_ok(),
        "SEC-HOSTKEY-SEQ-002: connect() must succeed when check_server_key returns Ok(true); \
         got Err({:?}) — mock server fixture may be broken",
        result.err()
    );

    if let Ok(handle) = result {
        let _ = handle
            .disconnect(russh::Disconnect::ByApplication, "", "en")
            .await;
    }
}
