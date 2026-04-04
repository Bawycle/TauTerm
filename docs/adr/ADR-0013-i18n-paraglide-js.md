<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-0013 — i18n library: Paraglide JS (Inlang)

**Date:** 2026-04-04
**Status:** Accepted

## Context

FS-I18N requires all TauTerm UI strings to be externalised and looked up from a locale catalogue, with v1 supporting English (fallback) and French. The i18n solution must integrate cleanly with Svelte 5 / SvelteKit and satisfy the following constraints:

- Immediate locale switching without a page reload or application restart (FS-I18N-004).
- No modification of PTY environment variables (`LANG`, `LC_*`) — i18n is strictly a UI concern (FS-I18N-007).
- Static asset delivery: locale files are bundled with the frontend, not fetched from a server at runtime (consistent with TauTerm's static SPA adapter).
- TypeScript type safety: accessing a non-existent message key should be a compile-time error.
- Minimal runtime overhead: i18n must not add perceptible latency to string resolution in a high-frequency render path (terminal row rendering does not use UI strings, but Preferences panel and status bar do).

## Decision

Use **Paraglide JS** (`@inlang/paraglide-sveltekit`) as the i18n library.

Paraglide JS is the idiomatic i18n solution for SvelteKit, maintained by Inlang. It works by extracting messages from JSON catalogues at compile time and generating fully tree-shakeable TypeScript accessor functions. There is no runtime lookup table — calling `m.some_key()` compiles to a direct string return for the active locale. This is the only i18n library for SvelteKit that provides compile-time exhaustiveness checking (a missing key is a TypeScript error) without requiring a custom Vite plugin or code generator.

**Locale files:** `src/lib/i18n/messages/en.json` (source, fallback) and `src/lib/i18n/messages/fr.json`. JSON objects mapping snake_case namespaced keys to string values.

**Runtime locale state:** A Svelte 5 reactive value in `src/lib/state/locale.svelte.ts`. Locale switching updates this value; all components re-render via fine-grained reactivity. The active locale is persisted to `preferences.json` under `appearance.language` via `update_preferences`.

**Tauri integration:** Locale files are static Vite-bundled assets. No Rust-side i18n is required. IPC error codes and status codes are locale-agnostic keys; the frontend maps them to strings via the message catalogue.

## Alternatives considered

**svelte-i18n**
The most widely used Svelte i18n library. It uses a store-based runtime dictionary: all locale strings are loaded into a writable store, and `$_('key')` looks up the key at render time. This works well for SSR applications where locale detection happens server-side. For TauTerm (static SPA, Tauri WebView, no server), it adds runtime overhead for a lookup that Paraglide eliminates at compile time. It also provides no compile-time key exhaustiveness: a typo in a key is a silent runtime miss (returning the key string itself). Rejected: inferior type safety, unnecessary runtime overhead.

**i18next / react-i18next (adapted)**
i18next is the most feature-complete i18n framework in the JavaScript ecosystem (plurals, namespaces, interpolation, backend plugins). It is designed for large-scale applications with many locale variants and dynamic loading from a CDN. TauTerm v1 has two locales, a small catalogue (< 500 keys), and a static delivery model. i18next's plugin architecture and configuration surface are over-engineered for this scale. The SvelteKit adapter (`i18next-svelte`) is less mature than Paraglide's first-party integration. Rejected: over-engineered for scale.

**Manual key lookup (plain TypeScript object)**
A hand-rolled solution: a TypeScript object `const messages = { en: {...}, fr: {...} }` with a function `t(key)` that returns the string for the active locale. This is trivially simple for small catalogues but does not scale: no tooling for extracting untranslated keys, no compile-time exhaustiveness, no standard format for translators. Rejected: lacks tooling, not maintainable as the catalogue grows.

**Typesafe-i18n**
A compile-time-first i18n library similar to Paraglide. It generates typed accessor functions from locale files and provides exhaustiveness checking. It is framework-agnostic (good) but lacks a first-party SvelteKit adapter (requires manual integration). Paraglide's `@inlang/paraglide-sveltekit` package provides a tighter integration (Vite plugin, automatic route handling if needed). Rejected: Paraglide is better integrated for SvelteKit.

## Consequences

**Positive:**
- Compile-time exhaustiveness: missing or misspelled message keys are TypeScript errors. No untranslated key string can reach production.
- Zero runtime overhead for string lookup: accessor functions return strings directly, with no dictionary traversal.
- Tree-shaking: unused message keys are eliminated from the production bundle.
- Standard JSON catalogue format: translators can work with the files directly without tooling.
- Immediate locale switching: updating the reactive locale value triggers Svelte 5 fine-grained re-render of all string consumers (FS-I18N-004).

**Negative / risks:**
- Paraglide requires a build step (`pnpm exec paraglide-js compile`) whenever locale files change. The generated `src/lib/paraglide/` directory is a build artefact and must not be hand-edited. This is mitigated by running the compile step automatically via the Vite plugin integration in `pnpm dev` and `pnpm tauri build`.
- Paraglide JS is a relatively young library (2023–2024). Its API surface is stable for the core use case (static SPA, small catalogue) but less battle-tested than svelte-i18n or i18next. For TauTerm's v1 scope (two locales, < 500 keys, static delivery), this risk is acceptable.
- Adding a new locale in a future version requires creating a new JSON file and recompiling. This is the expected workflow; there is no dynamic locale loading.

## Notes

The `src/lib/paraglide/` generated directory should be added to `.gitignore` if the team prefers not to commit generated code, or committed if reproducibility is preferred. Either approach is acceptable; the convention should be established at project setup time.

The `language` field is added to `AppearancePrefs` in `preferences/schema.rs` with `#[serde(default)]` defaulting to `"en"`. Unknown values on load are treated as `"en"` (FS-I18N-006).
