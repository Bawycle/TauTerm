// SPDX-License-Identifier: MPL-2.0

/**
 * IPC error discriminators for TauTerm.
 *
 * All IPC errors emitted by the backend are `#[derive(Serialize)]` types
 * (see src-tauri/src/errors.rs). The frontend must discriminate by `code`
 * rather than by string message content.
 *
 * Source of truth: src-tauri/src/errors.rs TauTermError.
 */

/**
 * Structured error envelope returned by the backend on IPC command failure.
 * Mirrors Rust TauTermError serialised via #[serde(tag = "code", rename_all = "camelCase")].
 */
export interface TauTermError {
  /** Machine-readable error code (maps to Rust variant name). */
  code: TauTermErrorCode;
  /** Human-readable detail (never shown directly in UI — use code for dispatch). */
  message: string;
}

/**
 * All error codes the backend can emit.
 * Mirrors the Rust TauTermError enum variants.
 */
export type TauTermErrorCode =
  | 'sessionNotFound'
  | 'paneNotFound'
  | 'tabNotFound'
  | 'connectionNotFound'
  | 'ptySpawnFailed'
  | 'ptyIoError'
  | 'sshAuthFailed'
  | 'sshConnectionFailed'
  | 'sshNotConnected'
  | 'credentialStoreError'
  | 'preferencesIoError'
  | 'preferencesParseError'
  | 'invalidInput'
  | 'clipboardError'
  | 'urlRejected'
  | 'themeNotFound'
  | 'internal';

/**
 * Type guard: returns true if the thrown value is a TauTermError envelope.
 *
 * Usage:
 * ```ts
 * try { await someCommand(); }
 * catch (err) {
 *   if (isTauTermError(err)) {
 *     if (err.code === 'sshAuthFailed') { ... }
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
