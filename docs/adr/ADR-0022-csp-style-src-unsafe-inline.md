<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0022 — CSP `style-src 'unsafe-inline'`: permanent v1 constraint

**Status:** Accepted
**Date:** 2026-04-10
**Author:** Security Expert — TauTerm team

---

## Context

TauTerm's Content Security Policy, configured in `src-tauri/tauri.conf.json`, includes:

```
style-src 'self' 'unsafe-inline'
```

The CLAUDE.md constraint note states: "`style-src 'unsafe-inline'` is retained for Tailwind 4 dev mode and must be reviewed when production build is validated." The TODO item requested either a concrete exit criterion or documentation of the constraint as permanent.

This ADR records the result of that investigation.

---

## Analysis

### Layer 1 — SvelteKit `bundleStrategy: "inline"`

`svelte.config.js` sets `output.bundleStrategy: "inline"`. A detailed comment in that file explains the root cause: Vite's modulepreload polyfill emits `<link rel="stylesheet" crossOrigin="">` elements that require CORS headers. Under WebKitGTK's `tauri://localhost` custom protocol, no `Access-Control-Allow-Origin` header is returned, so WebKitGTK fires neither `load` nor `error` on those requests. The resulting Promise never resolves, blocking `kit.start()` and preventing the Svelte application from mounting.

The `"inline"` strategy eliminates the modulepreload mechanism. It does so by bundling all CSS into JavaScript and injecting it at runtime as dynamically created `<style>` elements. This applies to every CSS file processed by Vite, including the Tailwind 4 output — even in production builds.

Consequence: in the TauTerm production build, style injection is unconditionally dynamic. There is no static `<link>` stylesheet that could be covered by `style-src 'self'` alone.

### Layer 2 — Tailwind 4 `@theme` compilation

`src/app.css` uses a large `@theme {}` block defining design tokens as CSS custom properties. Tailwind 4 (via `@tailwindcss/vite` 4.2.2) processes this at build time and produces a compiled CSS bundle. Under `bundleStrategy: "inline"`, that bundle is embedded in the JavaScript output and injected into the document as a `<style>` element at runtime by SvelteKit's JS initialization code.

This is not a dev-mode-only behavior: the `"inline"` strategy applies identically in development and production.

### Layer 3 — Tauri 2 CSP nonce support

Tauri 2's CSP is configured as a static string in `tauri.conf.json` under `app.security.csp`. Tauri 2.10.3 does not provide a built-in mechanism for generating a per-request nonce and injecting it into both the CSP header and the HTML document at render time. The `tauri-utils` crate applies the CSP string as-is when the WebView loads the application.

A nonce-based approach would require:
1. Generating a cryptographically random nonce per page load in the Rust backend.
2. Injecting `'nonce-<value>'` into the CSP header for `style-src`.
3. Injecting the same nonce value as the `nonce` attribute on every `<style>` element emitted by SvelteKit's runtime injection code.

Step 3 is not feasible with the current SvelteKit `"inline"` strategy: SvelteKit's bundle injects `<style>` elements via JavaScript without any hook point for adding a nonce attribute. Modifying this behavior would require either patching SvelteKit's internal DOM injection code or replacing `bundleStrategy: "inline"` entirely.

### Layer 4 — Can `bundleStrategy: "inline"` be replaced?

The comment in `svelte.config.js` explicitly states: "If a future version of Tauri or WebKitGTK fixes CORS handling on custom protocols, `split` could be reconsidered." The `"inline"` strategy is a workaround for a WebKitGTK protocol-level limitation, not a deliberate CSP trade-off. Until WebKitGTK or the Tauri custom protocol handler adds proper CORS support for `tauri://localhost`, switching away from `"inline"` would break application startup.

### Risk assessment of `style-src 'unsafe-inline'`

`'unsafe-inline'` for `style-src` permits an attacker who can inject arbitrary HTML into the WebView to apply arbitrary styles. The concrete impact in TauTerm's threat model is:

- **CSS-based data exfiltration**: Stylesheet injection can be used to exfiltrate attribute values via `background-image: url(...)` requests. TauTerm's `connect-src` is restricted to `ipc:` and `http://ipc.localhost` only — outbound network requests from injected stylesheets to arbitrary origins are blocked. This substantially limits the exfiltration surface.
- **UI redress / clickjacking via CSS**: Malicious styles could overlay UI elements to mislead the user. This requires first achieving HTML injection into the Svelte WebView, which is a separate, higher-order prerequisite.
- **No script execution**: `'unsafe-inline'` on `style-src` does not affect `script-src`. Inline scripts remain blocked. CSS cannot execute code in this context.

The residual risk is rated **Low** given the `connect-src` restriction and the requirement for prior HTML injection. However, it is not zero and must be acknowledged as a permanent accepted risk for v1.

---

## Decision

`style-src 'unsafe-inline'` is confirmed as a **permanent v1 constraint**. It cannot be removed without first resolving the upstream `bundleStrategy: "inline"` dependency, which in turn depends on WebKitGTK CORS support for the `tauri://` custom protocol.

No nonce-based alternative is viable under the current architecture. The exit path is defined in the Consequences section below.

---

## Alternatives Considered

### Alternative A — Remove `'unsafe-inline'` immediately

Requires replacing `bundleStrategy: "inline"` with `"split"`, which breaks application startup under WebKitGTK due to the CORS issue documented in `svelte.config.js`. Not viable.

### Alternative B — Nonce injection via Tauri middleware

Would require a custom Tauri plugin intercepting the WebView's page load, generating a nonce, patching the HTML, and synchronizing with SvelteKit's CSS injection code. This is a significant engineering effort with no current support in Tauri 2's plugin API for modifying outgoing HTML before WebView rendering. Experimental and not viable for v1.

### Alternative C — Hash-based `style-src`

CSP `'sha256-...'` hashes for `style-src` require knowing the exact content of each `<style>` element at build time. With `bundleStrategy: "inline"`, the injected style content is deterministic per build but would require a post-build step to compute and embed hashes in the CSP string. Tauri 2 has no built-in mechanism for this. Additionally, if any `<style>` content changes (e.g., after a token update), the hash must be recomputed and committed — creating fragile coordination between the build artifact and the configuration file. Not viable for v1 without dedicated tooling.

---

## Consequences

### Accepted risk (v1)

`'unsafe-inline'` on `style-src` is accepted as a residual risk for v1 with the following mitigations already in place:

- `connect-src` is restricted to `ipc: http://ipc.localhost` — outbound CSS exfiltration to external origins is blocked.
- `script-src` has no `'unsafe-inline'` and no `'unsafe-eval'` — CSS injection cannot escalate to script execution.
- The existing test `sec_csp_002_script_src_unsafe_inline_absent_or_csp_null` in `security_static_checks.rs` guards `script-src` and explicitly carves out the `style-src` allowance (comment: "Style-src is allowed to have it temporarily — see SEC-CSP-004").

### Exit criterion (post-v1)

The constraint can be lifted when **all three** of the following conditions are met:

1. WebKitGTK or Tauri's `tauri://localhost` custom protocol handler adds proper CORS header support for `<link rel="stylesheet" crossOrigin="">` requests, making `bundleStrategy: "split"` viable.
2. `svelte.config.js` is updated to `bundleStrategy: "split"` and application startup is verified end-to-end via `pnpm wdio`.
3. `tauri.conf.json` `style-src` is updated to `'self'` only, and the static check in `security_static_checks.rs` is extended to verify that `'unsafe-inline'` is absent from `style-src` as well.

### Documentation updates

- `docs/arch/06-appendix.md` §8.4: the vague "future tightening" note is replaced with a reference to this ADR and the concrete exit criterion above.
- `security_static_checks.rs`: the comment referencing "SEC-CSP-004" and "temporarily" should be updated to reference ADR-0022 when this record is committed.

### Debt

This ADR records an acknowledged architectural constraint, not ignorance. The trade-off is: working application on all supported Linux distributions vs. elimination of a low-risk CSP permission. The decision prioritizes correctness over incremental hardening until the upstream limitation is resolved.
