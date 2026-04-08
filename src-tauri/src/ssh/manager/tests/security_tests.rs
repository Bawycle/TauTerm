// SPDX-License-Identifier: MPL-2.0

use super::super::Credentials;

// -----------------------------------------------------------------------
// SEC-CRED-003 — Credentials::Debug redacts password and private_key_path
// -----------------------------------------------------------------------

#[test]
fn sec_cred_003_password_redacted_in_debug_output() {
    let creds = Credentials {
        username: "alice".to_string(),
        password: Some("hunter2".to_string()),
        private_key_path: None,
        save_in_keychain: false,
    };
    let debug_str = format!("{:?}", creds);
    assert!(
        !debug_str.contains("hunter2"),
        "Password must not appear in Debug output (SEC-CRED-003). Got: {}",
        debug_str
    );
    assert!(
        debug_str.contains("<redacted>"),
        "Debug output must contain '<redacted>' for password (SEC-CRED-003). Got: {}",
        debug_str
    );
}

#[test]
fn sec_cred_003_none_password_debug_output_safe() {
    let creds = Credentials {
        username: "alice".to_string(),
        password: None,
        private_key_path: None,
        save_in_keychain: false,
    };
    let debug_str = format!("{:?}", creds);
    assert!(
        debug_str.contains("None"),
        "None password should appear as None in Debug"
    );
}

#[test]
fn sec_cred_003_private_key_path_redacted_in_debug() {
    let creds = Credentials {
        username: "alice".to_string(),
        password: None,
        private_key_path: Some("/home/alice/.ssh/id_ed25519".to_string()),
        save_in_keychain: false,
    };
    let debug_str = format!("{:?}", creds);
    assert!(
        !debug_str.contains("/home/alice/.ssh/id_ed25519"),
        "private_key_path must NOT appear in Debug output (SEC-CRED-003 / FINDING-001). Got: {}",
        debug_str
    );
    assert!(
        debug_str.contains("<redacted>"),
        "Debug output must contain '<redacted>' for private_key_path (SEC-CRED-003). Got: {}",
        debug_str
    );
}

// -----------------------------------------------------------------------
// SEC-CRED-004 — SshConnectionConfig does not contain password field
// -----------------------------------------------------------------------

#[test]
fn sec_cred_004_ssh_connection_config_no_password_in_json() {
    use crate::session::ids::ConnectionId;
    use crate::ssh::SshConnectionConfig;

    let config = SshConnectionConfig {
        id: ConnectionId::new(),
        label: "My Server".to_string(),
        host: "example.com".to_string(),
        port: 22,
        username: "alice".to_string(),
        identity_file: Some("/home/alice/.ssh/id_ed25519".to_string()),
        allow_osc52_write: false,
        keepalive_interval_secs: None,
        keepalive_max_failures: None,
    };

    let json = serde_json::to_string(&config).expect("serialize failed");
    assert!(
        !json.contains("password"),
        "SshConnectionConfig JSON must not contain a 'password' field (SEC-CRED-004). Got: {}",
        json
    );
    assert!(
        json.contains("/home/alice/.ssh/id_ed25519"),
        "identity_file must store the path, not key content (SEC-CRED-004)"
    );
    assert!(
        json.contains("identityFile"),
        "Field must serialize as identityFile (camelCase)"
    );
}

#[test]
fn sec_cred_004_ssh_connection_config_identity_file_skipped_when_none() {
    use crate::session::ids::ConnectionId;
    use crate::ssh::SshConnectionConfig;

    let config = SshConnectionConfig {
        id: ConnectionId::new(),
        label: "Password server".to_string(),
        host: "example.com".to_string(),
        port: 22,
        username: "bob".to_string(),
        identity_file: None,
        allow_osc52_write: false,
        keepalive_interval_secs: None,
        keepalive_max_failures: None,
    };

    let json = serde_json::to_string(&config).expect("serialize failed");
    assert!(
        !json.contains("identityFile"),
        "identityFile field must be omitted when None (skip_serializing_if)"
    );
}
