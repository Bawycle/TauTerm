// SPDX-License-Identifier: MPL-2.0

//! Integration tests for SSH authentication — require a live OpenSSH server.
//!
//! Run with: cargo nextest run --profile ssh --test ssh_integration
//! In CI: use the Podman environment defined in Containerfile.ssh-test
//!
//! ## Environment variables (set by run-ssh-tests-inner.sh)
//!
//! | Variable                      | Purpose                                       |
//! |-------------------------------|-----------------------------------------------|
//! | TAUTERM_SSH_TEST_HOST         | SSH server hostname (127.0.0.1)               |
//! | TAUTERM_SSH_TEST_PORT         | SSH server port (2222)                        |
//! | TAUTERM_SSH_TEST_USER         | Username with password + pubkey auth ("tauterm") |
//! | TAUTERM_SSH_TEST_PASSWORD     | Password for the test user                    |
//! | TAUTERM_SSH_TEST_NOAUTH_USER  | Username with no valid credentials            |
//! | TAUTERM_TEST_PUBKEY_PATH      | Path to ED25519 private key (authorized)      |
//! | TAUTERM_SSH_TEST_HOST_KEY_LINE| Full known_hosts line for the server key      |
//!
//! ## Coverage
//!
//! | Test ID              | FS reference     | Scenario                                  |
//! |----------------------|------------------|-------------------------------------------|
//! | SSH-INT-001          | FS-SSH-012       | Password auth succeeds                    |
//! | SSH-INT-002          | FS-SSH-012       | Pubkey auth succeeds                      |
//! | SSH-INT-003          | FS-SSH-012       | Keyboard-interactive auth succeeds        |
//! | SSH-INT-004          | FS-SSH-012       | Wrong password returns Ok(false)          |
//! | SSH-INT-005          | FS-SSH-011       | First connection → Unknown (TOFU trigger) |
//! | SSH-INT-006          | FS-SSH-011       | Trusted host → Ok(true) from check_server_key |
//! | SSH-INT-007          | FS-SSH-011       | Mismatch key → Mismatch (MITM detection)  |
//! | SSH-INT-008          | FS-SSH-011       | accept_host_key stores key; reconnect succeeds |
//! | SSH-INT-009          | FS-CRED-006      | Path traversal in identity_file is rejected |
//! | SSH-INT-010          | FS-CRED-006      | Non-regular-file identity_file is rejected |
//! | SSH-INT-011          | FS-SSH-020       | keepalive constants match FS-SSH-020      |
//! | SSH-INT-012          | FS-SSH-013       | PTY request negotiation succeeds          |

#[cfg(target_os = "linux")]
mod linux {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use tau_term_lib::ssh::auth::{
        authenticate_keyboard_interactive, authenticate_password, authenticate_pubkey,
    };
    use tau_term_lib::ssh::keepalive::{SSH_KEEPALIVE_INTERVAL, SSH_KEEPALIVE_MAX_MISSES};
    use tau_term_lib::ssh::known_hosts::{KnownHostLookup, KnownHostsStore};

    // -----------------------------------------------------------------------
    // Test environment helpers
    // -----------------------------------------------------------------------

    struct SshTestEnv {
        host: String,
        port: u16,
        user: String,
        password: String,
        pubkey_path: PathBuf,
        host_key_line: String,
    }

    impl SshTestEnv {
        /// Load test environment variables set by run-ssh-tests-inner.sh.
        ///
        /// Returns `None` and prints a skip message when a variable is missing,
        /// allowing the test suite to run gracefully outside the container.
        fn load() -> Option<Self> {
            fn env(key: &str) -> Option<String> {
                std::env::var(key).ok()
            }

            let host = env("TAUTERM_SSH_TEST_HOST")?;
            let port_str = env("TAUTERM_SSH_TEST_PORT")?;
            let port = port_str.parse::<u16>().ok()?;
            let user = env("TAUTERM_SSH_TEST_USER")?;
            let password = env("TAUTERM_SSH_TEST_PASSWORD")?;
            let pubkey_path_str = env("TAUTERM_TEST_PUBKEY_PATH")?;
            let pubkey_path = PathBuf::from(pubkey_path_str);
            let host_key_line = env("TAUTERM_SSH_TEST_HOST_KEY_LINE")?;

            Some(Self {
                host,
                port,
                user,
                password,
                pubkey_path,
                host_key_line,
            })
        }

        fn addr(&self) -> String {
            format!("{}:{}", self.host, self.port)
        }
    }

    // -----------------------------------------------------------------------
    // Minimal russh Handler — accepts all host keys unconditionally.
    //
    // Used for tests that exercise auth, not host key verification.
    // A separate TOFU-aware handler is used for FS-SSH-011 tests.
    // -----------------------------------------------------------------------

    /// Shared state type for the TOFU checking handler's captured key.
    type CapturedKeyArc = Arc<tokio::sync::Mutex<Option<(String, Vec<u8>)>>>;

    struct AcceptAllHandler;

    impl russh::client::Handler for AcceptAllHandler {
        type Error = russh::Error;

        async fn check_server_key(
            &mut self,
            _server_public_key: &russh::keys::PublicKey,
        ) -> Result<bool, Self::Error> {
            Ok(true)
        }
    }

    // -----------------------------------------------------------------------
    // Handler that performs the TOFU check via KnownHostsStore.
    // -----------------------------------------------------------------------

    struct TofuCheckingHandler {
        host: String,
        known_hosts_path: PathBuf,
        /// Set to Some(lookup_result) after check_server_key completes.
        result: Arc<tokio::sync::Mutex<Option<KnownHostLookup>>>,
        /// Captured raw key bytes — used by SSH-INT-008 to build the known_hosts entry.
        captured_key: CapturedKeyArc,
    }

    impl TofuCheckingHandler {
        fn new(
            host: String,
            known_hosts_path: PathBuf,
            result: Arc<tokio::sync::Mutex<Option<KnownHostLookup>>>,
        ) -> Self {
            Self {
                host,
                known_hosts_path,
                result,
                captured_key: Arc::new(tokio::sync::Mutex::new(None)),
            }
        }
    }

    impl russh::client::Handler for TofuCheckingHandler {
        type Error = russh::Error;

        async fn check_server_key(
            &mut self,
            server_public_key: &russh::keys::PublicKey,
        ) -> Result<bool, Self::Error> {
            use russh::keys::PublicKeyBase64;

            let key_type = server_public_key.algorithm().as_str().to_string();
            let key_bytes: Vec<u8> = server_public_key.public_key_bytes();

            // Always capture the key for tests that need it (e.g. SSH-INT-008).
            *self.captured_key.lock().await = Some((key_type.clone(), key_bytes.clone()));

            let store = KnownHostsStore::new(self.known_hosts_path.clone());
            let lookup = store
                .lookup(&self.host, &key_type, &key_bytes)
                .map_err(russh::Error::IO)?;

            let accept = matches!(lookup, KnownHostLookup::Trusted(_));

            // Store the lookup result for the test to inspect.
            *self.result.lock().await = Some(lookup);

            Ok(accept)
        }
    }

    // -----------------------------------------------------------------------
    // Helper: open a minimal russh client session with AcceptAllHandler.
    // -----------------------------------------------------------------------

    async fn connect_accept_all(
        env: &SshTestEnv,
    ) -> Result<russh::client::Handle<AcceptAllHandler>, russh::Error> {
        let config = Arc::new(russh::client::Config {
            ..Default::default()
        });
        russh::client::connect(config, env.addr().as_str(), AcceptAllHandler).await
    }

    // -----------------------------------------------------------------------
    // SSH-INT-001 — Password auth succeeds (FS-SSH-012)
    // -----------------------------------------------------------------------

    /// Connecting with the correct password must return Ok(true).
    #[tokio::test]
    async fn ssh_int_001_password_auth_success() {
        let Some(env) = SshTestEnv::load() else {
            eprintln!("SKIP: SSH test environment variables not set");
            return;
        };

        let mut session = connect_accept_all(&env)
            .await
            .expect("TCP/SSH connect must succeed");

        let result = authenticate_password(&mut session, &env.user, &env.password)
            .await
            .expect("authenticate_password must not return a transport error");

        assert!(
            result,
            "SSH-INT-001: password authentication with correct credentials must succeed (FS-SSH-012)"
        );

        // Disconnect cleanly.
        let _ = session
            .disconnect(russh::Disconnect::ByApplication, "", "en")
            .await;
    }

    // -----------------------------------------------------------------------
    // SSH-INT-002 — Pubkey auth succeeds (FS-SSH-012)
    // -----------------------------------------------------------------------

    /// Connecting with the pre-authorized ED25519 key must return Ok(true).
    #[tokio::test]
    async fn ssh_int_002_pubkey_auth_success() {
        let Some(env) = SshTestEnv::load() else {
            eprintln!("SKIP: SSH test environment variables not set");
            return;
        };

        let mut session = connect_accept_all(&env)
            .await
            .expect("TCP/SSH connect must succeed");

        let result = authenticate_pubkey(&mut session, &env.user, &env.pubkey_path)
            .await
            .expect("authenticate_pubkey must not return a transport error");

        assert!(
            result,
            "SSH-INT-002: pubkey authentication with pre-authorized key must succeed (FS-SSH-012)"
        );

        let _ = session
            .disconnect(russh::Disconnect::ByApplication, "", "en")
            .await;
    }

    // -----------------------------------------------------------------------
    // SSH-INT-003 — Keyboard-interactive auth succeeds (FS-SSH-012)
    // -----------------------------------------------------------------------

    /// Keyboard-interactive auth with the correct password must return Ok(true).
    ///
    /// The OpenSSH server in test mode maps keyboard-interactive to PAM which
    /// accepts the password for the test user.
    #[tokio::test]
    async fn ssh_int_003_keyboard_interactive_auth_success() {
        let Some(env) = SshTestEnv::load() else {
            eprintln!("SKIP: SSH test environment variables not set");
            return;
        };

        let mut session = connect_accept_all(&env)
            .await
            .expect("TCP/SSH connect must succeed");

        let result = authenticate_keyboard_interactive(&mut session, &env.user, &env.password)
            .await
            .expect("authenticate_keyboard_interactive must not return a transport error");

        assert!(
            result,
            "SSH-INT-003: keyboard-interactive authentication with correct password must succeed (FS-SSH-012)"
        );

        let _ = session
            .disconnect(russh::Disconnect::ByApplication, "", "en")
            .await;
    }

    // -----------------------------------------------------------------------
    // SSH-INT-004 — Wrong password returns Ok(false) (FS-SSH-012)
    // -----------------------------------------------------------------------

    /// Providing a wrong password must return Ok(false), not an error.
    /// This verifies that credential rejection is not confused with a transport failure.
    #[tokio::test]
    async fn ssh_int_004_wrong_password_returns_ok_false() {
        let Some(env) = SshTestEnv::load() else {
            eprintln!("SKIP: SSH test environment variables not set");
            return;
        };

        let mut session = connect_accept_all(&env)
            .await
            .expect("TCP/SSH connect must succeed");

        let result = authenticate_password(
            &mut session,
            &env.user,
            "definitely-wrong-password-xyz",
        )
        .await
        .expect("authenticate_password must not return a transport error on credential rejection");

        assert!(
            !result,
            "SSH-INT-004: wrong password must return Ok(false), not Ok(true) (FS-SSH-012)"
        );

        // Connection may be closed by server after auth failure — that is acceptable.
    }

    // -----------------------------------------------------------------------
    // SSH-INT-005 — First connection produces Unknown lookup (TOFU trigger)
    //              (FS-SSH-011)
    // -----------------------------------------------------------------------

    /// On first connection to a host, KnownHostsStore::lookup must return Unknown.
    /// The TofuCheckingHandler captures the lookup result; the connection is
    /// rejected (check_server_key returns false) which is the correct TOFU behavior.
    ///
    /// Note: this test exercises the TOFU lookup path in KnownHostsStore, not the
    /// full `TauTermSshHandler::check_server_key` (which requires an AppHandle).
    /// The business logic being tested is identical — only the event emission differs.
    #[tokio::test]
    async fn ssh_int_005_first_connection_tofu_unknown() {
        let Some(env) = SshTestEnv::load() else {
            eprintln!("SKIP: SSH test environment variables not set");
            return;
        };

        let tmp = tempfile::tempdir().expect("tempdir");
        let known_hosts_path = tmp.path().join("known_hosts");

        let lookup_result: Arc<tokio::sync::Mutex<Option<KnownHostLookup>>> =
            Arc::new(tokio::sync::Mutex::new(None));

        let handler = TofuCheckingHandler::new(
            env.host.clone(),
            known_hosts_path,
            Arc::clone(&lookup_result),
        );

        let config = Arc::new(russh::client::Config {
            ..Default::default()
        });

        // The connection will be rejected by check_server_key (returns Ok(false)).
        // russh returns an error when the handler rejects the key.
        let _ = russh::client::connect(config, env.addr().as_str(), handler).await;

        let lookup = lookup_result.lock().await;
        assert!(
            lookup.is_some(),
            "SSH-INT-005: check_server_key must have been called — lookup result is None"
        );

        assert!(
            matches!(lookup.as_ref().unwrap(), KnownHostLookup::Unknown),
            "SSH-INT-005: first connection to unknown host must produce KnownHostLookup::Unknown (FS-SSH-011), \
             got: {:?}",
            lookup
        );
    }

    // -----------------------------------------------------------------------
    // SSH-INT-006 — Trusted host key: check_server_key accepts connection
    //              (FS-SSH-011)
    // -----------------------------------------------------------------------

    /// When the server's host key is present and matches the known-hosts file,
    /// KnownHostsStore::lookup must return Trusted and the connection must proceed.
    #[tokio::test]
    async fn ssh_int_006_trusted_host_key_accepts_connection() {
        let Some(env) = SshTestEnv::load() else {
            eprintln!("SKIP: SSH test environment variables not set");
            return;
        };

        // Populate a known-hosts file with the server's actual key.
        // TAUTERM_SSH_TEST_HOST_KEY_LINE is set by run-ssh-tests-inner.sh using ssh-keyscan.
        let tmp = tempfile::tempdir().expect("tempdir");
        let known_hosts_path = tmp.path().join("known_hosts");
        std::fs::write(&known_hosts_path, format!("{}\n", env.host_key_line))
            .expect("write known_hosts");

        // Rewrite the hostname in the known_hosts line to match what we connect to
        // (ssh-keyscan may write "127.0.0.1" whereas we pass env.host — they should match,
        // but we normalise explicitly to be safe).
        let normalized_line = if let Some(rest) = env.host_key_line.split_once(' ') {
            format!("{} {}\n", env.host, rest.1)
        } else {
            format!("{}\n", env.host_key_line)
        };
        std::fs::write(&known_hosts_path, normalized_line).expect("write normalized known_hosts");

        let lookup_result: Arc<tokio::sync::Mutex<Option<KnownHostLookup>>> =
            Arc::new(tokio::sync::Mutex::new(None));

        let handler = TofuCheckingHandler::new(
            env.host.clone(),
            known_hosts_path,
            Arc::clone(&lookup_result),
        );

        let config = Arc::new(russh::client::Config {
            ..Default::default()
        });

        let session_result = russh::client::connect(config, env.addr().as_str(), handler).await;

        let lookup = lookup_result.lock().await;
        assert!(
            lookup.is_some(),
            "SSH-INT-006: check_server_key must have been called"
        );

        assert!(
            matches!(lookup.as_ref().unwrap(), KnownHostLookup::Trusted(_)),
            "SSH-INT-006: known-matching key must produce KnownHostLookup::Trusted (FS-SSH-011), \
             got: {:?}",
            lookup
        );

        // The connection must have been accepted (no error from host key check).
        assert!(
            session_result.is_ok(),
            "SSH-INT-006: connection must succeed when host key matches known-hosts (FS-SSH-011), \
             error: {:?}",
            session_result.err()
        );

        if let Ok(session) = session_result {
            let _ = session
                .disconnect(russh::Disconnect::ByApplication, "", "en")
                .await;
        }
    }

    // -----------------------------------------------------------------------
    // SSH-INT-007 — Mismatched host key: Mismatch lookup, connection rejected
    //              (FS-SSH-011 — MITM detection)
    // -----------------------------------------------------------------------

    /// When the known-hosts file contains a different key for the host,
    /// KnownHostsStore::lookup must return Mismatch, and check_server_key
    /// must reject the connection.
    #[tokio::test]
    async fn ssh_int_007_mismatched_host_key_rejected() {
        let Some(env) = SshTestEnv::load() else {
            eprintln!("SKIP: SSH test environment variables not set");
            return;
        };

        // Write a known-hosts entry with a deliberately wrong (fabricated) key.
        // The key bytes must be valid base64 and decode to valid-looking data,
        // but must not match the server's real key.
        // Fake key in SSH wire format: correct structure (name-length + "ssh-ed25519" +
        // key-length + 32 fake key bytes), but with bytes 0x00..0x1f as the key data.
        // base64::STANDARD requires canonical padding (last char before '=' must have
        // low 2 bits = 00); this pre-computed key satisfies that constraint.
        let fake_key_b64 = "AAAAC3NzaC1lZDI1NTE5AAAAIAABAgMEBQYHCAkKCwwNDg8QERITFBUWFxgZGhscHR4f";
        let tmp = tempfile::tempdir().expect("tempdir");
        let known_hosts_path = tmp.path().join("known_hosts");
        std::fs::write(
            &known_hosts_path,
            format!("{} ssh-ed25519 {}\n", env.host, fake_key_b64),
        )
        .expect("write known_hosts with fake key");

        let lookup_result: Arc<tokio::sync::Mutex<Option<KnownHostLookup>>> =
            Arc::new(tokio::sync::Mutex::new(None));

        let handler = TofuCheckingHandler::new(
            env.host.clone(),
            known_hosts_path,
            Arc::clone(&lookup_result),
        );

        let config = Arc::new(russh::client::Config {
            ..Default::default()
        });

        // Connection must be rejected (check_server_key returns Ok(false) for Mismatch).
        let _ = russh::client::connect(config, env.addr().as_str(), handler).await;

        let lookup = lookup_result.lock().await;
        assert!(
            lookup.is_some(),
            "SSH-INT-007: check_server_key must have been called"
        );

        assert!(
            matches!(lookup.as_ref().unwrap(), KnownHostLookup::Mismatch { .. }),
            "SSH-INT-007: mismatched key must produce KnownHostLookup::Mismatch (FS-SSH-011), \
             got: {:?}",
            lookup
        );
    }

    // -----------------------------------------------------------------------
    // SSH-INT-008 — Accept host key, store it, reconnect succeeds (FS-SSH-011)
    // -----------------------------------------------------------------------

    /// After accepting a host key (storing it in known_hosts), a subsequent
    /// connection to the same server must proceed without rejection.
    ///
    /// This tests the TOFU acceptance path: Unknown on first contact → store →
    /// Trusted on subsequent connection.
    #[tokio::test]
    async fn ssh_int_008_accept_host_key_then_reconnect_succeeds() {
        let Some(env) = SshTestEnv::load() else {
            eprintln!("SKIP: SSH test environment variables not set");
            return;
        };

        let tmp = tempfile::tempdir().expect("tempdir");
        let known_hosts_path = tmp.path().join("known_hosts");

        // --- Phase 1: first connection — capture the server's key via TofuCheckingHandler.
        //
        // We use an empty known-hosts file so check_server_key returns Unknown.
        // The handler captures the raw key bytes in `captured_key`.
        // The connection is rejected (check_server_key returns false for Unknown),
        // but we have the key bytes we need to call add_entry.

        let first_lookup_result: Arc<tokio::sync::Mutex<Option<KnownHostLookup>>> =
            Arc::new(tokio::sync::Mutex::new(None));

        let first_handler = TofuCheckingHandler::new(
            env.host.clone(),
            known_hosts_path.clone(),
            Arc::clone(&first_lookup_result),
        );
        let captured_key_arc = Arc::clone(&first_handler.captured_key);

        let config = Arc::new(russh::client::Config::default());

        // Connection will be rejected (Unknown key) — that is expected.
        let _ = russh::client::connect(config.clone(), env.addr().as_str(), first_handler).await;

        let first_lookup = first_lookup_result.lock().await;
        assert!(
            matches!(first_lookup.as_ref(), Some(KnownHostLookup::Unknown)),
            "SSH-INT-008: first connection must see Unknown (FS-SSH-011), got: {:?}",
            first_lookup
        );
        drop(first_lookup);

        // Retrieve the captured key.
        let (key_type, key_bytes) = captured_key_arc
            .lock()
            .await
            .take()
            .expect("SSH-INT-008: check_server_key must have captured the key");

        // --- Phase 2: store the key (simulating the user accepting the TOFU prompt).

        let store = KnownHostsStore::new(known_hosts_path.clone());
        store
            .add_entry(&env.host, &key_type, &key_bytes)
            .expect("add_entry must succeed");

        // Verify the key is now Trusted.
        let verify_lookup = store
            .lookup(&env.host, &key_type, &key_bytes)
            .expect("lookup must succeed");
        assert!(
            matches!(verify_lookup, KnownHostLookup::Trusted(_)),
            "SSH-INT-008: stored key must be Trusted after add_entry (FS-SSH-011)"
        );

        // --- Phase 3: second connection — TofuCheckingHandler must see Trusted.

        let lookup_result: Arc<tokio::sync::Mutex<Option<KnownHostLookup>>> =
            Arc::new(tokio::sync::Mutex::new(None));

        let handler = TofuCheckingHandler::new(
            env.host.clone(),
            known_hosts_path,
            Arc::clone(&lookup_result),
        );

        let reconnect = russh::client::connect(config, env.addr().as_str(), handler).await;

        let lookup = lookup_result.lock().await;
        assert!(
            matches!(lookup.as_ref(), Some(KnownHostLookup::Trusted(_))),
            "SSH-INT-008: second connection must see Trusted lookup (FS-SSH-011), got: {:?}",
            lookup
        );

        assert!(
            reconnect.is_ok(),
            "SSH-INT-008: reconnect must succeed after accepting host key (FS-SSH-011), \
             error: {:?}",
            reconnect.err()
        );

        if let Ok(session) = reconnect {
            let _ = session
                .disconnect(russh::Disconnect::ByApplication, "", "en")
                .await;
        }
    }

    // -----------------------------------------------------------------------
    // SSH-INT-009 — Path traversal in identity_file is rejected (FS-CRED-006)
    // -----------------------------------------------------------------------

    /// A path containing `../` components must fail at key-loading time,
    /// not at connection time (the guard runs before any network I/O).
    ///
    /// This directly tests the `authenticate_pubkey` error path for a
    /// path-traversal value — the underlying `load_secret_key` must reject it
    /// because the path does not point to a valid key file.
    ///
    /// Note: FS-CRED-006 also requires that the IPC command layer validates
    /// the path before passing it to auth. That guard is tested in
    /// ipc_command_handlers.rs. This test validates the auth function itself
    /// handles the bad path gracefully (returns Err rather than panicking).
    #[tokio::test]
    async fn ssh_int_009_path_traversal_in_identity_file_rejected() {
        let Some(env) = SshTestEnv::load() else {
            eprintln!("SKIP: SSH test environment variables not set");
            return;
        };

        let mut session = connect_accept_all(&env)
            .await
            .expect("TCP/SSH connect must succeed");

        // A path containing `../` that resolves to a non-key file.
        let traversal_path = Path::new("/tmp/../../etc/passwd");

        let result = authenticate_pubkey(&mut session, &env.user, traversal_path).await;

        assert!(
            result.is_err(),
            "SSH-INT-009: identity_file with path traversal must produce an error, not Ok (FS-CRED-006)"
        );
    }

    // -----------------------------------------------------------------------
    // SSH-INT-010 — Non-regular-file identity_file is rejected (FS-CRED-006)
    // -----------------------------------------------------------------------

    /// A path pointing to a directory (not a regular file) must be rejected
    /// at key-loading time with an error, not a panic.
    #[tokio::test]
    async fn ssh_int_010_directory_identity_file_rejected() {
        let Some(env) = SshTestEnv::load() else {
            eprintln!("SKIP: SSH test environment variables not set");
            return;
        };

        let mut session = connect_accept_all(&env)
            .await
            .expect("TCP/SSH connect must succeed");

        // /tmp is a directory — definitely not a valid SSH private key.
        let dir_path = Path::new("/tmp");

        let result = authenticate_pubkey(&mut session, &env.user, dir_path).await;

        assert!(
            result.is_err(),
            "SSH-INT-010: identity_file pointing to a directory must produce an error (FS-CRED-006)"
        );
    }

    // -----------------------------------------------------------------------
    // SSH-INT-011 — Keepalive constants match FS-SSH-020 specification
    // -----------------------------------------------------------------------

    /// The keepalive interval and max-misses constants must match the values
    /// specified in FS-SSH-020. This is a compile-time structural test that
    /// verifies the constants used when building `russh::client::Config` are
    /// not accidentally changed.
    ///
    /// The actual keepalive behavior (timeout after 3 missed probes) is
    /// exercised by classify_disconnect_reason unit tests in connection.rs,
    /// since russh owns the keepalive timer and exposes only the final
    /// `disconnected(DisconnectReason::Error)` callback.
    #[test]
    fn ssh_int_011_keepalive_constants_match_fs_ssh_020() {
        assert_eq!(
            SSH_KEEPALIVE_INTERVAL.as_secs(),
            30,
            "SSH-INT-011: keepalive interval must be 30 seconds (FS-SSH-020)"
        );
        assert_eq!(
            SSH_KEEPALIVE_MAX_MISSES, 3,
            "SSH-INT-011: keepalive max misses must be 3 (FS-SSH-020)"
        );
    }

    // -----------------------------------------------------------------------
    // SSH-INT-012 — PTY request negotiation succeeds (FS-SSH-013)
    // -----------------------------------------------------------------------

    /// After authenticating, requesting a PTY with `xterm-256color` and the
    /// RFC 4254 terminal modes defined in TERMINAL_MODES must succeed.
    /// The server must confirm the PTY request.
    ///
    /// This validates FS-SSH-013: the PTY request includes TERM, dimensions, and
    /// the correct terminal mode opcodes.
    #[tokio::test]
    async fn ssh_int_012_pty_request_succeeds() {
        let Some(env) = SshTestEnv::load() else {
            eprintln!("SKIP: SSH test environment variables not set");
            return;
        };

        let mut session = connect_accept_all(&env)
            .await
            .expect("TCP/SSH connect must succeed");

        // Authenticate first.
        let authed = authenticate_password(&mut session, &env.user, &env.password)
            .await
            .expect("password auth must not produce a transport error");
        assert!(
            authed,
            "SSH-INT-012: authentication must succeed before PTY test"
        );

        // Open a session channel.
        let mut channel = session
            .channel_open_session()
            .await
            .expect("channel_open_session must succeed");

        // Request a PTY with the modes from FS-SSH-013.
        // These are the exact opcodes from TERMINAL_MODES in manager.rs.
        let terminal_modes: &[(russh::Pty, u32)] = &[
            (russh::Pty::VINTR, 3),
            (russh::Pty::VQUIT, 28),
            (russh::Pty::VERASE, 127),
            (russh::Pty::VKILL, 21),
            (russh::Pty::VEOF, 4),
            (russh::Pty::VSUSP, 26),
            (russh::Pty::ISIG, 1),
            (russh::Pty::ICANON, 1),
            (russh::Pty::ECHO, 1),
        ];

        let pty_result = channel
            .request_pty(
                true, // want_reply
                "xterm-256color",
                80, // cols
                24, // rows
                0,  // pixel width
                0,  // pixel height
                terminal_modes,
            )
            .await;

        assert!(
            pty_result.is_ok(),
            "SSH-INT-012: request_pty must not produce a transport error (FS-SSH-013), \
             error: {:?}",
            pty_result.err()
        );

        // Wait for the server's Success/Failure confirmation.
        let mut pty_confirmed = false;
        for _ in 0..20 {
            match channel.wait().await {
                Some(russh::ChannelMsg::Success) => {
                    pty_confirmed = true;
                    break;
                }
                Some(russh::ChannelMsg::Failure) => {
                    break; // PTY rejected — assertion below will fail with context
                }
                None => break,
                Some(_) => continue,
            }
        }

        assert!(
            pty_confirmed,
            "SSH-INT-012: PTY request must be confirmed by the server (FS-SSH-013: TERM=xterm-256color, \
             RFC 4254 terminal modes)"
        );

        let _ = session
            .disconnect(russh::Disconnect::ByApplication, "", "en")
            .await;
    }

    // -----------------------------------------------------------------------
    // SSH-INT-013 — try_authenticate with wrong credentials returns Ok(false)
    //              (FS-SSH-012, regression guard for credential prompt loop)
    //
    // This test exercises the FULL authentication sequence used by connect_task:
    //   keyboard-interactive (with wrong password) → Ok(false)
    //   then password          (with wrong password) → Ok(false)
    //
    // Both methods must return Ok(false) — NOT a transport error — so that the
    // connect_task auth loop can emit a credential-prompt event instead of
    // propagating an error via `?` and skipping the prompt entirely.
    //
    // If either method returns Err on a wrong-password rejection (as opposed to
    // a genuine transport failure), this test catches the regression.
    // -----------------------------------------------------------------------

    /// SSH-INT-013: both keyboard-interactive and password auth return Ok(false)
    /// when called in sequence with wrong credentials on the same session.
    #[tokio::test]
    async fn ssh_int_013_try_authenticate_wrong_credentials_returns_ok_false() {
        let Some(env) = SshTestEnv::load() else {
            eprintln!("SKIP: SSH test environment variables not set");
            return;
        };

        let mut session = connect_accept_all(&env)
            .await
            .expect("TCP/SSH connect must succeed");

        const WRONG_PASSWORD: &str = "definitely-wrong-password-xyz-regression";

        // Step 1: keyboard-interactive with wrong password — must be Ok(false).
        let kbd_result = authenticate_keyboard_interactive(&mut session, &env.user, WRONG_PASSWORD)
            .await
            .expect(
                "SSH-INT-013: authenticate_keyboard_interactive with wrong password must return \
                 Ok(false), not Err (transport error). If this fails, the connect_task auth loop \
                 will exit via `?` before emitting a credential-prompt event.",
            );
        assert!(
            !kbd_result,
            "SSH-INT-013: keyboard-interactive with wrong password must return Ok(false), not Ok(true)"
        );

        // Step 2: password auth with wrong password on the SAME session — must also be Ok(false).
        // This simulates the sequence in try_authenticate: kbd-interactive then password.
        let pw_result = authenticate_password(&mut session, &env.user, WRONG_PASSWORD)
            .await
            .expect(
                "SSH-INT-013: authenticate_password with wrong password (called after a failed \
                 keyboard-interactive on the same session) must return Ok(false), not Err. \
                 If this fails, the server may be disconnecting after MaxAuthTries is exhausted \
                 after only two attempts, preventing the credential-prompt from being emitted.",
            );
        assert!(
            !pw_result,
            "SSH-INT-013: password auth with wrong password (after kbd-interactive) must return Ok(false)"
        );

        // Connection may be closed by server after auth failure — acceptable.
        let _ = session
            .disconnect(russh::Disconnect::ByApplication, "", "en")
            .await;
    }
}
