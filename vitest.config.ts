// SPDX-License-Identifier: MPL-2.0

import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';

export default defineConfig({
  plugins: [
    // Svelte plugin is required to transform .svelte.ts rune files ($state, etc.).
    // No options needed for test-only usage.
    svelte(),
  ],

  resolve: {
    // 'browser' condition ensures Svelte resolves its client-side exports
    // (mount, unmount, etc.) rather than the server-side stubs when running
    // under jsdom. Without this, `import { mount } from 'svelte'` resolves
    // to index-server.js which throws lifecycle_function_unavailable.
    conditions: ['browser'],
    alias: {
      // Mirror the SvelteKit $lib alias so tests resolve the same paths.
      $lib: path.resolve(__dirname, 'src/lib'),
    },
  },

  test: {
    // jsdom provides the DOM APIs needed for Svelte component tests.
    environment: 'jsdom',

    // Global setup: polyfills for jsdom gaps (ResizeObserver, etc.).
    setupFiles: ['src/__mocks__/vitest-setup.ts'],

    // Glob covering all unit/component tests in src/.
    // E2E tests in tests/ are handled by WebdriverIO separately.
    include: ['src/**/*.{test,spec}.{ts,js}'],

    // Module name mapping: replace generated and Tauri runtime modules
    // with hand-written stubs so unit tests run without a real backend
    // or Paraglide build step.
    alias: [
      {
        find: /^\$lib\/paraglide\/runtime$/,
        replacement: path.resolve(__dirname, 'src/__mocks__/paraglide-runtime.ts'),
      },
      {
        find: /^@tauri-apps\/api\/core$/,
        replacement: path.resolve(__dirname, 'src/__mocks__/tauri-core.ts'),
      },
      {
        find: /^@tauri-apps\/api\/event$/,
        replacement: path.resolve(__dirname, 'src/__mocks__/tauri-event.ts'),
      },
      {
        find: /^@tauri-apps\/api\/window$/,
        replacement: path.resolve(__dirname, 'src/__mocks__/tauri-window.ts'),
      },
    ],
  },
});
