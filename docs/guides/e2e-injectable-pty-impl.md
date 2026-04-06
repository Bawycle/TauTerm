<!-- SPDX-License-Identifier: MPL-2.0 -->

> This implementation guide was extracted from `docs/adr/`. It is not an ADR — it is a technical blueprint. The corresponding architectural decision is [ADR-0015-e2e-injectable-pty.md](../adr/ADR-0015-e2e-injectable-pty.md).

# ADR-0015 — Implementation Notes: Injectable PTY Backend

**For:** rust-dev, frontend-dev
**Status:** Authoritative — implement exactly as described
**Companion ADR:** ADR-0015-e2e-injectable-pty.md

This document is a precise technical blueprint for implementing the `e2e-testing` feature. Read the ADR first for rationale. Read the existing source files listed in each section before writing code.

---

## 1. Cargo.toml changes

File: `src-tauri/Cargo.toml`

Add one line under `[features]` (create the section if it does not exist — as of this writing the file has no `[features]` section):

```toml
[features]
e2e-testing = []
```

No new dependencies are needed. `tokio::sync::mpsc` is already available via `tokio = { version = "1", features = ["full"] }`. `dashmap` is also already a dependency.

---

## 2. New file: `src-tauri/src/platform/pty_injectable.rs`

This file is only compiled when `cfg(feature = "e2e-testing")` is active (enforced by the module declaration in `platform.rs` — see section 4). The file does not need its own `#[cfg(...)]` attribute on every item because the module itself is conditionally compiled.

Read `src-tauri/src/platform/pty_linux.rs` and `src-tauri/src/platform.rs` before writing this file. The patterns mirror `LinuxPtySession` exactly.

### 2.1 Imports

The file needs:
- `std::io::{self, Read}` — for the `Read` impl on the receiver adapter
- `std::sync::{Arc, Mutex}` — to wrap the receiver in `Arc<Mutex<...>>` matching the signature of `PtySession::reader_handle()`
- `dashmap::DashMap` — for the pane-to-sender map in `InjectablePtyBackend`
- `tokio::sync::mpsc` — for the channel
- `crate::error::PtyError` — for `PtyBackend`/`PtySession` error types
- `crate::platform::{PtyBackend, PtySession}` — the traits being implemented
- `crate::session::ids::PaneId` — used as the map key

### 2.2 `InjectableRegistry`

```
pub struct InjectableRegistry {
    senders: DashMap<PaneId, mpsc::UnboundedSender<Vec<u8>>>,
}
```

This is a separate public struct, not part of `InjectablePtyBackend`. It is the piece that `inject_pty_output` will access via Tauri state. It must implement `Send + Sync` (which `DashMap` already provides).

Public methods:
- `pub fn new() -> Self` — returns an empty registry
- `pub fn register(&self, pane_id: PaneId, tx: mpsc::UnboundedSender<Vec<u8>>)` — inserts or replaces the sender for the given pane
- `pub fn send(&self, pane_id: &PaneId, data: Vec<u8>) -> Result<(), String>` — retrieves the sender and calls `tx.send(data)`. Returns `Err("pane not found: {pane_id}")` if the sender is absent. Returns `Err("channel closed for pane {pane_id}")` if the send fails (receiver dropped, meaning the pane's read task has exited).
- `pub fn remove(&self, pane_id: &PaneId)` — removes the sender (called when the injectable pane is closed)

### 2.3 `MpscReaderAdapter`

The PTY read task calls `reader.lock().read(&mut buf)` in a blocking loop inside `tokio::task::spawn_blocking`. The read must block until data is available, and must return `Ok(0)` (EOF) when the sender is dropped.

`tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>` is an async type; it cannot be used directly as `std::io::Read`. The adapter bridges them:

```
struct MpscReaderAdapter {
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
    /// Bytes buffered from the last received chunk that have not yet been
    /// consumed by a `read()` call.
    leftover: Vec<u8>,
}
```

`impl std::io::Read for MpscReaderAdapter`:

The `read(&mut self, buf: &mut [u8]) -> io::Result<usize>` method must:
1. If `self.leftover` is non-empty, copy as many bytes as fit into `buf`, advance `leftover`, return the count. This handles the case where a previous chunk was larger than `buf`.
2. Otherwise, call `self.rx.blocking_recv()`. This is the blocking call. It blocks the thread until a message arrives or the sender is dropped.
   - `Some(chunk)` — copy as many bytes as fit into `buf`, store the remainder in `self.leftover`, return the count copied.
   - `None` — the sender was dropped (pane closed). Return `Ok(0)` to signal EOF to the read task, which will exit its loop cleanly (see `pty_task.rs` line 77: `Ok(0) => { tracing::debug!("PTY EOF..."); return; }`).

The `blocking_recv()` method is the correct call here because `MpscReaderAdapter` is used exclusively inside `spawn_blocking`, which runs on a dedicated thread pool where blocking is safe. Do not use `try_recv` (would require spinning) and do not create a new Tokio runtime inside the adapter.

Note: `MpscReaderAdapter` holds an `UnboundedReceiver`, which is `!Send`. This means it cannot be placed directly inside `Arc<Mutex<Box<dyn Read + Send>>>`. The solution is to move the adapter into the `Box<dyn Read + Send>` before wrapping it in the `Arc<Mutex<...>>` — the `Box` erases the concrete type, and the `Send` bound is satisfied because `blocking_recv` only needs to be called from a single thread (the `spawn_blocking` thread). However, `UnboundedReceiver` is in fact `Send` in tokio — verify this. If it is `Send`, the wrapping is straightforward. If not, wrap the receiver in a `std::sync::Mutex` before boxing it, and acquire the mutex inside the `read()` call.

As of tokio 1.x, `mpsc::UnboundedReceiver<T>` is `Send` when `T: Send`. Since `T = Vec<u8>` and `Vec<u8>: Send`, the receiver is `Send`. The adapter is therefore `Send` and can be boxed directly.

### 2.4 `InjectablePtySession`

```
pub struct InjectablePtySession {
    pane_id: PaneId,
    reader: Arc<Mutex<Box<dyn Read + Send>>>,
    /// Shared reference to the registry so the session can deregister on close.
    registry: Arc<InjectableRegistry>,
}
```

`impl PtySession for InjectablePtySession`:

- `reader_handle(&self) -> Option<Arc<Mutex<Box<dyn Read + Send>>>>` — returns `Some(self.reader.clone())`. This is the method `SessionRegistry::create_tab` calls (via `get_reader_handle`) to wire the read task. The injectable session must implement this; without it, no read task is spawned and `inject_pty_output` bytes would never be processed.
- `write(&mut self, _data: &[u8]) -> Result<(), PtyError>` — returns `Ok(())`. Input from `send_input` is silently discarded. E2E tests control output via `inject_pty_output`, not via `send_input`.
- `resize(&mut self, _cols: u16, _rows: u16, _pixel_width: u16, _pixel_height: u16) -> Result<(), PtyError>` — returns `Ok(())`. No-op.
- `close(self: Box<Self>)` — calls `self.registry.remove(&self.pane_id)`. Dropping `self` drops `self.reader`, which drops the `Arc<Mutex<Box<dyn Read + Send>>>`. When the refcount reaches zero, `MpscReaderAdapter` is dropped, which drops the `UnboundedReceiver`. The read task is holding the `Arc` independently; when the task next calls `rx.blocking_recv()` and gets `None` (because the sender in `InjectableRegistry` has also been removed and dropped), it returns `Ok(0)` and exits.

### 2.5 `InjectablePtyBackend`

```
pub struct InjectablePtyBackend {
    registry: Arc<InjectableRegistry>,
}
```

The backend holds a shared reference to the `InjectableRegistry` so that `open_session` can register senders.

`impl PtyBackend for InjectablePtyBackend`:

`open_session(&self, cols: u16, rows: u16, _command: &str, _args: &[&str], _env: &[(&str, &str)]) -> Result<Box<dyn PtySession>, PtyError>`:

1. Generate a `PaneId` — but wait: `open_session` does not receive a `PaneId`. The caller (`SessionRegistry::create_tab`) generates the `PaneId` *after* calling `open_session`. This is a design mismatch. Resolution: `InjectablePtyBackend::open_session` generates its own `PaneId` internally (via `PaneId::new()`), creates the channel, registers the sender in the registry under that pane ID, and stores the pane ID in the returned `InjectablePtySession`. The `inject_pty_output` command will look up bytes by the pane ID known to the frontend (which comes from the `TabState` returned by `create_tab`). But the pane ID in `TabState` is generated by `SessionRegistry::create_tab`, not by `open_session`. These would be two different IDs — a bug.

Correct resolution: the `PaneId` used in `SessionRegistry::create_tab` is generated *inside* the registry, after `open_session` returns. `InjectablePtyBackend` cannot know it in advance. The injectable backend therefore cannot pre-register the sender under the correct pane ID.

The cleanest fix is to add a post-registration step. Two options:

**Option A — `InjectablePtySession` holds the sender, and `SessionRegistry` calls a post-hook after creating the pane ID.**
This requires changing `SessionRegistry`, which is not acceptable.

**Option B — `InjectablePtyBackend::open_session` uses a temporary placeholder ID; `InjectableRegistry` exposes a `remap(old_id, new_id)` method; `SessionRegistry` calls remap after assigning the real pane ID.**
This requires `SessionRegistry` to know about `InjectableRegistry`, which is coupling that we want to avoid.

**Option C (chosen) — `InjectablePtySession` exposes the sender, and `InjectableRegistry` is populated by `inject_pty_output`'s caller, not by `open_session`.**
Instead: `open_session` returns the `UnboundedSender` as a field on `InjectablePtySession`. `SessionRegistry::create_tab` calls `open_session`, receives the session, extracts the sender via a method on the trait (impossible — `PtySession` is a trait object) or via a separate accessor.

**Option D (chosen — cleanest) — `InjectablePtyBackend` registers the sender lazily, keyed by a temporary UUID. `InjectablePtySession` exposes its temporary ID. A dedicated `lib.rs` setup step extracts the real pane ID from the returned `TabState` and remaps.**
This is complex and fragile.

**Option E (correct — no registry pre-population) — `InjectablePtyBackend::open_session` creates the channel and stores the `UnboundedSender` in `InjectablePtySession`. `InjectablePtySession` adds a `pub fn take_sender(&mut self) -> Option<mpsc::UnboundedSender<Vec<u8>>>` method. The `inject_pty_output` command retrieves the sender not from `InjectableRegistry` but from the pane session stored in `SessionRegistry`.**

This is also problematic: `PtySession` is a trait object and `PaneSession.pty_session` is `Option<Box<dyn PtySession>>`. Accessing the sender requires downcasting, which violates the architecture rules (CLAUDE.md: "Never downcast a trait object via raw pointer").

**Definitive resolution (Option F — add accessor to `PtySession` trait, feature-gated):**

Add a feature-gated method to the `PtySession` trait in `platform.rs`:

```
#[cfg(feature = "e2e-testing")]
fn injectable_sender(&self) -> Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>> {
    None
}
```

`InjectablePtySession` overrides this to return `Some(self.tx.clone())`, where `self.tx` is the `UnboundedSender` created in `open_session`. All other `PtySession` implementors inherit the default `None`.

In `SessionRegistry::create_tab` (feature-gated block), after creating the pane ID:

```
#[cfg(feature = "e2e-testing")]
if let Some(tx) = pty_box.injectable_sender() {
    injectable_registry.register(pane_id.clone(), tx);
}
```

where `injectable_registry` is `Arc<InjectableRegistry>` retrieved from a parameter or stored in `SessionRegistry`.

To avoid storing `InjectableRegistry` in `SessionRegistry` (which would couple a test concern into a production struct), retrieve it from the `AppHandle` in `lib.rs`'s `setup` closure and pass it to a feature-gated helper function that `create_tab` calls. This is still coupling.

**Final definitive resolution:** Store `Arc<InjectableRegistry>` as a field on `SessionRegistry` under `#[cfg(feature = "e2e-testing")]`. In production builds the field does not exist. In test builds, `SessionRegistry::new` receives it as an extra parameter and stores it. `create_tab` uses it to register the sender after the pane ID is known.

Concretely:

`SessionRegistry` gets a feature-gated field:

```
#[cfg(feature = "e2e-testing")]
injectable_registry: Arc<crate::platform::pty_injectable::InjectableRegistry>,
```

`SessionRegistry::new` gets a feature-gated extra parameter:

```
#[cfg(feature = "e2e-testing")]
injectable_registry: Arc<crate::platform::pty_injectable::InjectableRegistry>,
```

`create_tab` gets a feature-gated block after the pane ID is generated:

```
#[cfg(feature = "e2e-testing")]
if let Some(tx) = pty_box.injectable_sender() {
    self.injectable_registry.register(pane_id.clone(), tx);
}
```

`split_pane` gets the same block for the `new_pane_id`.

`close_pane` gets a feature-gated call to `self.injectable_registry.remove(&pane_id)`.

The `injectable_sender` method is added to the `PtySession` trait (feature-gated, default `None`). `InjectablePtySession` overrides it.

This approach is self-contained: the coupling is entirely within feature-gated blocks that disappear in production. It requires changes to `SessionRegistry` and `PtySession`, but those changes are invisible outside the `e2e-testing` feature.

`InjectablePtySession` therefore holds:
- `tx: mpsc::UnboundedSender<Vec<u8>>` — for cloning in `injectable_sender()`
- `reader: Arc<Mutex<Box<dyn Read + Send>>>` — the adapter, for `reader_handle()`

It no longer needs a reference back to `InjectableRegistry` (removal is handled by `SessionRegistry::close_pane`).

### 2.6 Factory function

```
pub fn create_injectable_pty_backend(
    registry: Arc<InjectableRegistry>,
) -> InjectablePtyBackend {
    InjectablePtyBackend { registry }
}
```

---

## 3. New file: `src-tauri/src/commands/testing.rs` (feature-gated)

Read `src-tauri/src/commands/input_cmds.rs` before writing this file, to follow the exact command handler pattern used in this project.

```
// SPDX-License-Identifier: MPL-2.0

//! E2E testing commands — only compiled with the `e2e-testing` feature.
//!
//! These commands must never appear in production builds.

use std::sync::Arc;

use tauri::State;

use crate::platform::pty_injectable::InjectableRegistry;
use crate::session::ids::PaneId;

/// Push synthetic bytes directly into the VT pipeline for a pane.
///
/// The bytes bypass the real PTY and are delivered to the pane's VtProcessor
/// through the injectable mpsc channel. This is the primary mechanism for
/// E2E test determinism (ADR-0015).
#[cfg(feature = "e2e-testing")]
#[tauri::command]
pub async fn inject_pty_output(
    pane_id: PaneId,
    data: Vec<u8>,
    registry: State<'_, Arc<InjectableRegistry>>,
) -> Result<(), String> {
    registry.send(&pane_id, data)
}
```

The return type is `Result<(), String>` rather than a typed error struct. Rationale: this is a testing command, not a production API. The frontend test code checks for success/failure but does not need to discriminate error variants. A plain `String` is acceptable here as a deliberate exception to the IPC error typing rule.

---

## 4. Changes to `src-tauri/src/platform.rs`

Read `src-tauri/src/platform.rs` before editing.

### 4.1 Module declaration

Add after the existing platform module declarations (after `pub mod validation;`):

```rust
#[cfg(feature = "e2e-testing")]
pub mod pty_injectable;
```

### 4.2 Re-exports

Add after the module declarations:

```rust
#[cfg(feature = "e2e-testing")]
pub use pty_injectable::{
    InjectablePtyBackend,
    InjectableRegistry,
    create_injectable_pty_backend,
};
```

### 4.3 Feature-gated method on `PtySession`

Add to the `PtySession` trait, after `reader_handle`:

```rust
/// Return the injectable sender for this session, if this is an `InjectablePtySession`.
///
/// The default returns `None`. Only `InjectablePtySession` returns `Some`.
/// This method only exists when the `e2e-testing` feature is active.
#[cfg(feature = "e2e-testing")]
fn injectable_sender(
    &self,
) -> Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>> {
    None
}
```

---

## 5. Changes to `src-tauri/src/session/registry.rs`

Read `src-tauri/src/session/registry.rs` in full before editing.

### 5.1 Feature-gated field on `SessionRegistry`

Add to the `SessionRegistry` struct:

```rust
/// Injectable output registry, present only in e2e-testing builds.
#[cfg(feature = "e2e-testing")]
injectable_registry: std::sync::Arc<crate::platform::pty_injectable::InjectableRegistry>,
```

### 5.2 Feature-gated parameter on `SessionRegistry::new`

The signature changes under the feature flag. The cleanest approach is to use a helper macro or, simpler, add the parameter unconditionally as `Option<...>` under the feature flag. Prefer a clean approach: make `new` accept the extra argument only when the feature is active by using a conditional compilation block:

```rust
pub fn new(
    pty_backend: Arc<dyn PtyBackend>,
    app: AppHandle,
    #[cfg(feature = "e2e-testing")]
    injectable_registry: std::sync::Arc<crate::platform::pty_injectable::InjectableRegistry>,
) -> Arc<Self> {
    Arc::new_cyclic(|weak| Self {
        inner: RwLock::new(RegistryInner::new()),
        pty_backend,
        app,
        self_ref: weak.clone(),
        #[cfg(feature = "e2e-testing")]
        injectable_registry,
    })
}
```

Note: `#[cfg(...)]` on a function parameter is valid Rust syntax in edition 2024.

### 5.3 Registration in `create_tab`

After the line `pane.pty_session = Some(pty_box);` (where `pty_box` is still accessible via `pane`), add:

```rust
#[cfg(feature = "e2e-testing")]
if let Some(tx) = pane.pty_session.as_ref()
    .and_then(|s| s.injectable_sender())
{
    self.injectable_registry.register(pane_id.clone(), tx);
}
```

Wait — at the point `pane.pty_session = Some(pty_box)` is assigned, `pty_box` has moved. Access `pane.pty_session.as_ref()` afterwards, which returns `Option<&Box<dyn PtySession>>`. Call `injectable_sender()` on the inner `&dyn PtySession` via deref.

Actually the sequence in `create_tab` is:
1. `let pty_box: Box<dyn PtySession> = ...`
2. `let reader_handle = get_reader_handle(&*pty_box);`
3. Extract sender before moving: `#[cfg(feature = "e2e-testing")] let injectable_tx = pty_box.injectable_sender();`
4. `pane.pty_session = Some(pty_box);`
5. `#[cfg(feature = "e2e-testing")] if let Some(tx) = injectable_tx { self.injectable_registry.register(pane_id.clone(), tx); }`

This is the correct ordering: extract the sender from `pty_box` before it moves into `pane.pty_session`.

Apply the same pattern in `split_pane` for `new_pane_id`.

### 5.4 Deregistration in `close_pane`

In `close_pane`, after `entry.panes.remove(&pane_id)` and before the layout tree rebuild:

```rust
#[cfg(feature = "e2e-testing")]
self.injectable_registry.remove(&pane_id);
```

---

## 6. Changes to `src-tauri/src/commands.rs`

Add the feature-gated submodule:

```rust
#[cfg(feature = "e2e-testing")]
pub mod testing;

#[cfg(feature = "e2e-testing")]
pub use testing::*;
```

---

## 7. Changes to `src-tauri/src/lib.rs`

Read `src-tauri/src/lib.rs` before editing.

### 7.1 Feature-gated `InjectableRegistry` state and backend selection

In the `setup` closure, replace the unconditional backend creation with a feature-gated branch:

```rust
.setup(|app| {
    #[cfg(not(feature = "e2e-testing"))]
    let pty_backend: Arc<dyn PtyBackend> = Arc::from(platform::create_pty_backend());

    #[cfg(feature = "e2e-testing")]
    let injectable_registry = Arc::new(platform::pty_injectable::InjectableRegistry::new());

    #[cfg(feature = "e2e-testing")]
    let pty_backend: Arc<dyn PtyBackend> = Arc::new(
        platform::create_injectable_pty_backend(injectable_registry.clone())
    );

    #[cfg(feature = "e2e-testing")]
    app.manage(injectable_registry.clone());

    let registry = SessionRegistry::new(
        pty_backend,
        app.handle().clone(),
        #[cfg(feature = "e2e-testing")]
        injectable_registry,
    );
    app.manage(registry);
    Ok(())
})
```

Note: `platform::pty_injectable` and `platform::create_injectable_pty_backend` are only accessible under the feature flag (they are gated in `platform.rs` — see section 4). The `#[cfg(feature = "e2e-testing")]` attributes on the `use` statements ensure the non-feature build does not see these symbols.

You may need to add `use crate::platform::PtyBackend;` (already present via the existing `Arc::from(platform::create_pty_backend())` path — check that the trait import is visible).

### 7.2 Feature-gated handler registration

In `generate_handler![]`, add at the end:

```rust
// E2E testing commands — only compiled and registered when e2e-testing feature is active.
#[cfg(feature = "e2e-testing")]
commands::testing::inject_pty_output,
```

Note: `generate_handler![]` is a macro. The `#[cfg(...)]` attribute on a macro argument may or may not work depending on the macro's expansion. If the macro does not support `#[cfg]` on individual entries, use a conditional compilation block at the `invoke_handler` call site by splitting into two calls — but Tauri's `Builder::invoke_handler` only accepts a single call. The correct approach is to use a macro-level conditional. Tauri's `generate_handler![]` macro does support `#[cfg(...)]` on its entries as of Tauri 2. Verify this compiles; if not, use a workaround such as defining a no-op handler for non-test builds.

If `generate_handler![]` does not support `#[cfg]` on entries, the alternative is:

```rust
#[cfg(not(feature = "e2e-testing"))]
let handler = tauri::generate_handler![
    /* all production handlers */
];

#[cfg(feature = "e2e-testing")]
let handler = tauri::generate_handler![
    /* all production handlers */
    commands::testing::inject_pty_output,
];

tauri::Builder::default()
    // ...
    .invoke_handler(handler)
```

This approach duplicates the handler list but is guaranteed to compile correctly. Prefer it if `#[cfg]` inside `generate_handler![]` does not work.

---

## 8. Changes to `wdio.conf.ts`

Read the existing `wdio.conf.ts` before editing.

### 8.1 Binary path

The `application` capability must point to the debug binary built with the `e2e-testing` feature:

```typescript
const binaryPath = process.env.TAUTERM_BINARY_PATH
  ?? path.resolve(__dirname, 'src-tauri/target/debug/tau-term');
```

Using an environment variable allows CI to override the path without changing the config file. The build command that produces this binary is:

```bash
cd src-tauri && cargo build --features e2e-testing
```

This produces `src-tauri/target/debug/tau-term`. It is a debug build (no `--release`), which is correct for E2E: test builds do not need optimisation, and debug symbols aid diagnosis.

### 8.2 `beforeSession` hook

Add a `beforeSession` hook that validates the binary exists and prints an actionable error if not:

```typescript
beforeSession: async (_config, _capabilities) => {
  const fs = await import('fs/promises');
  try {
    await fs.access(binaryPath);
  } catch {
    throw new Error(
      `E2E binary not found at: ${binaryPath}\n` +
      `Build it with: cd src-tauri && cargo build --features e2e-testing\n` +
      `Or set TAUTERM_BINARY_PATH to point to an existing binary.`
    );
  }
},
```

This ensures the test run fails immediately with a clear message rather than a cryptic tauri-driver spawn error.

---

## 9. Changes to E2E specs: `pty-roundtrip.spec.ts`

### 9.1 Getting `paneId` from the DOM

The frontend must render the active pane's ID on the pane's root element as a `data-pane-id` attribute. For example:

```html
<div class="terminal-pane" data-pane-id="550e8400-e29b-41d4-a716-446655440000">
  ...
</div>
```

If this attribute is not yet present, it must be added by `frontend-dev` as part of this feature. The spec retrieves it as:

```typescript
const paneId = await $('.terminal-pane').getAttribute('data-pane-id');
expect(paneId).toBeTruthy();
```

### 9.2 `TEST-PTY-RT-002` — using `inject_pty_output`

Replace the existing keyboard-based injection with a direct IPC call:

```typescript
it('TEST-PTY-RT-002: injected bytes appear on the terminal grid', async () => {
  // Get the active pane's ID from the DOM.
  const paneId = await $('.terminal-pane[data-active="true"]').getAttribute('data-pane-id');

  // Prepare the byte sequence: "tauterm-e2e-marker\r\n"
  const marker = 'tauterm-e2e-marker';
  const bytes = [...new TextEncoder().encode(marker + '\r\n')];

  // Inject bytes directly into the VT pipeline, bypassing the real PTY.
  await browser.execute(
    async (paneIdArg: string, dataArg: number[]) => {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('inject_pty_output', {
        paneId: paneIdArg,
        data: dataArg,
      });
    },
    paneId,
    bytes,
  );

  // Wait for the marker to appear somewhere in the terminal grid.
  await browser.waitUntil(
    async () => {
      const text = await $('.terminal-grid').getText();
      return text.includes(marker);
    },
    {
      timeout: 3000,
      timeoutMsg: `"${marker}" did not appear on the terminal grid within 3 s`,
    },
  );
});
```

Key points:
- The `invoke` call goes through `browser.execute`, which runs in the WebView context. This is how Tauri commands are called from WebdriverIO.
- `data` must be a plain `number[]` (array of byte values 0–255), not a `Uint8Array`, because WebdriverIO serialises arguments via JSON and `Uint8Array` does not serialise cleanly. The Rust side receives `Vec<u8>` via serde's JSON deserialiser, which accepts a JSON array of integers.
- The `paneId` serialisation: `PaneId` is a newtype over `String` with `#[derive(Serialize, Deserialize)]`. It serialises as a plain JSON string. Pass `paneId` (the DOM attribute string) directly — Tauri's IPC layer will deserialise it into `PaneId`.
- Do not use `await browser.pause(...)` before the `waitUntil` call. The VT pipeline is synchronous from the perspective of the read task: bytes injected into the channel are processed before the next `screen-update` event is emitted. The `waitUntil` polling covers the async event propagation to the frontend.

---

## 10. Risks and mitigations

### 10.1 `blocking_recv()` inside `spawn_blocking`

`tokio::sync::mpsc::UnboundedReceiver::blocking_recv()` blocks the OS thread until a message is available or the sender is dropped. This is the intended behaviour inside `spawn_blocking`. The Tokio documentation explicitly supports this pattern. The PTY read task already uses `spawn_blocking` for the production reader, so this is consistent.

Risk: if the read task is aborted via `PtyTaskHandle::abort()` while `blocking_recv()` is in progress, the `spawn_blocking` thread is not automatically interrupted (unlike `spawn` tasks, `spawn_blocking` tasks are not cancellation-aware at the OS thread level). The thread will remain blocked until the next message arrives or the sender is dropped.

Mitigation: `close_pane` calls `self.injectable_registry.remove(&pane_id)`, which drops the `UnboundedSender`. With the sender dropped, `blocking_recv()` returns `None` (EOF), and the read task exits its loop on the next iteration. The thread is then released back to the Tokio blocking thread pool. This is correct behaviour: the thread is not leaked, it exits cleanly after the next EOF signal.

In practice, `close_pane` is called by the test teardown, which always runs after the test body. The sender is dropped as part of teardown, unblocking any lingering read task.

### 10.2 `reader_handle` is called once

`SessionRegistry::create_tab` calls `get_reader_handle(&*pty_box)` once, before `pane.pty_session = Some(pty_box)`. The reader is an `Arc<Mutex<Box<dyn Read + Send>>>`. The read task holds one clone, and `InjectablePtySession::reader` holds the other. When `InjectablePtySession` is dropped (via `close_pane`), the `Arc` refcount drops by one, but the read task's clone keeps the `Arc` alive until the task exits. This is safe and mirrors the production `LinuxPtySession` pattern exactly.

### 10.3 EOF / close ordering

The correct close sequence for an injectable pane is:

1. `close_pane` is called → `InjectableRegistry::remove(&pane_id)` drops the `UnboundedSender`.
2. `PaneSession` is dropped → `PtyTaskHandle` is dropped → `abort_handle.abort()` is called (see `pty_task.rs`).
3. The read task is inside `blocking_recv()`. The sender is already dropped, so `blocking_recv()` returns `None` on the next wakeup. The task returns from the read loop.
4. Alternatively, if `abort()` races with `blocking_recv()` returning `None`, the task may exit via cancellation instead. Either path is clean.

No data corruption or panic can occur because the receiver is only accessed from the single `spawn_blocking` thread, protected by the `Arc<Mutex<...>>` (the mutex is always locked by one thread at a time).

### 10.4 Sender registration race in `create_tab`

`create_tab` extracts the injectable sender before spawning the read task. The read task is spawned after the sender is registered in `InjectableRegistry`. Therefore, by the time the read task starts blocking on `blocking_recv()`, the sender is already in the registry. Tests can call `inject_pty_output` as soon as `create_tab` returns a `TabState`. There is no race.

### 10.5 `split_pane` — new pane ID vs sender

`split_pane` spawns a second PTY session for the new pane. The same injectable sender extraction pattern must be applied in `split_pane` as in `create_tab` (see section 5.3). The new pane ID (`new_pane_id`) is available at the point where `pty_box` is created. The extraction must happen before `new_pane.pty_session = Some(pty_box)` (same ordering constraint as in `create_tab`).

### 10.6 Production build safety

The entire injectable path — `pty_injectable.rs`, `InjectableRegistry`, `inject_pty_output`, the field on `SessionRegistry`, the method on `PtySession` — is wrapped in `#[cfg(feature = "e2e-testing")]`. The standard production build command (`pnpm tauri build`, which invokes `cargo build --release` without feature flags) will not compile any of this code. Run `cargo clippy -- -D warnings` without the feature to verify that the production build is clean. Also run with `--features e2e-testing` to verify the test build.
