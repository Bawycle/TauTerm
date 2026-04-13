// SPDX-License-Identifier: MPL-2.0
import { readFileSync } from 'node:fs';
import { defineConfig } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { licenseGeneratorPlugin } from './vite-plugin-licenses';

/** Read the app version from the single source of truth: src-tauri/Cargo.toml. */
function readCargoVersion() {
  const cargo = readFileSync('src-tauri/Cargo.toml', 'utf-8');
  const match = cargo.match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) throw new Error('Cannot find version in src-tauri/Cargo.toml');
  return match[1];
}

const host = process.env.TAURI_DEV_HOST;

/**
 * Wrap @tailwindcss/vite plugins to exclude Svelte virtual CSS modules.
 *
 * Root cause: when the Svelte CSS compilation cache is not yet populated on
 * initial request, `vite-plugin-svelte:load-compiled-css` returns undefined.
 * Vite then falls back to reading the raw .svelte file from disk, which
 * includes <script> blocks. Tailwind's `generate:serve` transformer receives
 * this mixed content and Lightning CSS fails parsing JavaScript identifiers
 * (e.g. `onMount`) as CSS declarations.
 *
 * No Svelte component in this project uses @apply or other Tailwind CSS
 * directives — utility classes are applied via class attributes, scanned from
 * app.css. Excluding Svelte virtual modules from Tailwind processing is
 * therefore lossless.
 */
function tailwindExcludingSvelteVirtual() {
  // Matches ?svelte&type=style&lang.css (and similar Svelte virtual IDs)
  const svelteVirtual = /[?&]svelte[=&]/;
  return tailwindcss().map((plugin) => {
    if (!plugin?.transform || typeof plugin.transform !== 'object') return plugin;
    const t = plugin.transform;
    // filter.id may be a string, RegExp, array, or an include/exclude object.
    // Only augment the include/exclude object form — other forms have no exclude list.
    const idFilter = t.filter?.id;
    if (
      !idFilter ||
      typeof idFilter === 'string' ||
      idFilter instanceof RegExp ||
      Array.isArray(idFilter)
    ) {
      return plugin;
    }
    return {
      ...plugin,
      transform: {
        ...t,
        filter: {
          ...t.filter,
          id: {
            ...idFilter,
            exclude: [
              ...(Array.isArray(idFilter.exclude)
                ? idFilter.exclude
                : idFilter.exclude != null
                  ? [idFilter.exclude]
                  : []),
              svelteVirtual,
            ],
          },
        },
      },
    };
  });
}

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    licenseGeneratorPlugin(),
    sveltekit(),
    tailwindExcludingSvelteVirtual(),
    paraglideVitePlugin({
      project: './project.inlang',
      outdir: './src/lib/paraglide',
      strategy: ['globalVariable', 'baseLocale'],
    }),
  ],

  define: {
    'import.meta.env.VITE_APP_VERSION': JSON.stringify(readCargoVersion()),
  },

  test: {
    environment: 'jsdom',
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },

  build: {
    // Assets are embedded in the Tauri binary and loaded from memory — no
    // network transfer cost. 2 MB is a reasonable ceiling before investigating
    // code-splitting (which adds complexity with no UX benefit in this context).
    chunkSizeWarningLimit: 2048,
    rolldownOptions: {
      // Suppress EMPTY_IMPORT_META: `bundleStrategy: "inline"` forces an iife-style
      // bundle where import.meta is not natively supported. Rolldown already replaces
      // import.meta with {} automatically; this makes the intent explicit and silences
      // the warning. Vite has already replaced all import.meta.env.* before this point.
      transform: {
        define: { 'import.meta': '{}' },
      },
      // Suppress PLUGIN_TIMINGS: informational-only performance report, not actionable.
      checks: {
        pluginTimings: false,
      },
    },
  },
}));
