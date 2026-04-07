// SPDX-License-Identifier: MPL-2.0

//! Shell path resolution for PTY session creation (FS-PTY-014).

use crate::error::SessionError;
use crate::platform::validation::validate_shell_path;

// ---------------------------------------------------------------------------
// Shell resolution (FS-PTY-014)
// ---------------------------------------------------------------------------

/// Resolve the shell executable path.
///
/// Priority:
/// 1. `explicit` — the caller's explicit shell path (from `CreateTabConfig.shell`)
/// 2. `$SHELL` — from the environment
/// 3. `/bin/sh` — unconditional fallback
///
/// Each candidate is validated by `validate_shell_path()`. The first valid
/// candidate is returned. If all candidates fail, `/bin/sh` is returned as a
/// last resort (it is always present on Linux).
pub(super) fn resolve_shell_path(explicit: Option<&str>) -> Result<String, SessionError> {
    // 1. Explicit override
    if let Some(raw) = explicit {
        return validate_shell_path(raw)
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| SessionError::InvalidShellPath(e.message));
    }

    // 2. $SHELL from environment
    if let Ok(shell_env) = std::env::var("SHELL") {
        if let Ok(path) = validate_shell_path(&shell_env) {
            return Ok(path.to_string_lossy().to_string());
        }
        // $SHELL was set but invalid — fall through to /bin/sh.
        tracing::warn!("$SHELL={shell_env} is invalid; falling back to /bin/sh");
    }

    // 3. Unconditional fallback
    Ok("/bin/sh".to_string())
}
