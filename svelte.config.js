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
    // Bundle all JS and CSS into a single entry per route instead of code-splitting.
    //
    // With the default "split" strategy, Vite emits separate .css chunks and loads
    // them via a modulepreload polyfill that creates <link crossOrigin=""> elements
    // and awaits their load/error events. Under tauri://localhost (WebKitGTK custom
    // protocol + WebDriver), these events are never fired — the Promise hangs
    // forever, blocking kit.start() and preventing the Svelte app from mounting.
    //
    // "inline" bundles CSS into JS (injected at runtime) and removes the preload
    // mechanism entirely. Trade-off: slightly larger initial JS payload and a brief
    // flash before CSS is injected. Both are negligible for a bundled desktop app
    // where all assets are embedded in the binary and served locally.
    output: {
      bundleStrategy: "inline",
    },
  },
};

export default config;
