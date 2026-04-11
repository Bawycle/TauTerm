// SPDX-License-Identifier: MPL-2.0

/**
 * IPC error discriminators for TauTerm.
 *
 * All IPC errors emitted by the backend are `#[derive(Serialize)]` types
 * (see src-tauri/src/error.rs). The frontend must discriminate by `code`
 * rather than by string message content.
 *
 * Source of truth: src-tauri/src/error.rs TauTermError.
 */

/**
 * Structured error envelope returned by the backend on IPC command failure.
 * Mirrors Rust TauTermError serialised as `{ "code": "...", "message": "...", "detail"?: "..." }`.
 *
 * Note: `code` is UPPER_SNAKE_CASE as emitted by the Rust backend — not camelCase.
 */
export interface TauTermError {
  /** Machine-readable error code (maps to Rust error string, UPPER_SNAKE_CASE). */
  code: TauTermErrorCode;
  /** Human-readable detail (never shown directly in UI — use code for dispatch). */
  message: string;
  /** Optional technical detail: raw OS error, exit code, system message. */
  detail?: string;
}

/**
 * All error codes the backend can emit.
 * Mirrors all `TauTermError::new` / `TauTermError::with_detail` code strings
 * in src-tauri/src/error.rs (UPPER_SNAKE_CASE).
 */
export type TauTermErrorCode =
  // PTY errors (PtyError → TauTermError)
  | 'PTY_OPEN_FAILED'
  | 'PTY_SPAWN_FAILED'
  | 'PTY_IO_ERROR'
  | 'PTY_RESIZE_FAILED'
  // Session errors (SessionError → TauTermError)
  | 'INVALID_TAB_ID'
  | 'INVALID_PANE_ID'
  | 'PANE_NOT_RUNNING'
  | 'INVALID_SHELL_PATH'
  // SSH errors (SshError → TauTermError)
  | 'SSH_CONNECTION_FAILED'
  | 'SSH_AUTH_FAILED'
  | 'SSH_HOST_KEY_REJECTED'
  | 'SSH_IO_ERROR'
  | 'NO_PENDING_CREDENTIALS'
  | 'SSH_TRANSPORT_ERROR'
  | 'INVALID_SSH_IDENTITY_PATH'
  // Preferences errors (PreferencesError → TauTermError)
  | 'PREF_IO_ERROR'
  | 'PREF_PARSE_ERROR'
  | 'PREF_INVALID_VALUE'
  // Credential store errors (CredentialError → TauTermError)
  | 'CRED_STORE_UNAVAILABLE'
  | 'CRED_NOT_FOUND'
  | 'CRED_IO_ERROR'
  // Fallback from anyhow::Error
  | 'INTERNAL_ERROR';

/**
 * Type guard: returns true if the thrown value is a TauTermError envelope.
 *
 * Usage:
 * ```ts
 * try { await someCommand(); }
 * catch (err) {
 *   if (isTauTermError(err)) {
 *     if (err.code === 'SSH_AUTH_FAILED') { ... }
 *   }
 * }
 * ```
 */
export function isTauTermError(value: unknown): value is TauTermError {
  return (
    typeof value === 'object' &&
    value !== null &&
    'code' in value &&
    'message' in value &&
    typeof (value as TauTermError).code === 'string' &&
    typeof (value as TauTermError).message === 'string'
  );
}
