// SPDX-License-Identifier: MPL-2.0
// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      fallback: "index.html",
    }),
    // ⚠️  DO NOT change bundleStrategy without rebuilding and running `pnpm wdio`.
    //
    // Root cause: Vite's modulepreload polyfill emits <link rel="stylesheet"
    // crossOrigin=""> elements and awaits their load/error events. Under
    // tauri://localhost (WebKitGTK custom protocol), the tauri:// handler does not
    // return Access-Control-Allow-Origin headers, so WebKitGTK fires neither load
    // nor error on these CORS-mode requests. The Promise hangs forever, blocking
    // kit.start() and preventing the Svelte app from mounting.
    //
    // "inline" bundles CSS into JS (injected at runtime via <style> tags) and
    // removes the modulepreload mechanism entirely, eliminating the CORS-mode links.
    //
    // This is a workaround for a WebKitGTK / tauri:// protocol limitation.
    // If a future version of Tauri or WebKitGTK fixes CORS handling on custom
    // protocols, "split" could be reconsidered for better code-splitting.
    //
    // Trade-off: slightly larger initial JS payload. Negligible for a desktop app
    // where all assets are embedded in the binary and served locally.
    output: {
      bundleStrategy: "inline",
    },
  },
};

export default config;
