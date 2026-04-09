// SPDX-License-Identifier: MPL-2.0
import os from "os";
import path from "path";
import { spawn } from "child_process";
import { fileURLToPath } from "url";
import type { Options } from "@wdio/types";

const __dirname = fileURLToPath(new URL(".", import.meta.url));

// Path to the Tauri binary to test.
// Override with TAUTERM_BINARY_PATH to point at a custom build location.
// Default: the release binary produced by:
//   pnpm tauri build --no-bundle -- --features e2e-testing
// Must go through the Tauri CLI (not bare `cargo build`) so that the frontend
// assets are embedded. `--no-bundle` skips AppImage/deb packaging.
const binaryPath =
  process.env.TAUTERM_BINARY_PATH ??
  path.resolve(__dirname, "src-tauri", "target", "release", "tau-term");

const tauriDriverPath = path.resolve(
  os.homedir(),
  ".cargo",
  "bin",
  "tauri-driver"
);

let tauriDriver: ReturnType<typeof spawn> | undefined;

export const config: Options.Testrunner = {
  // WebDriver endpoint served by tauri-driver
  hostname: "127.0.0.1",
  port: 4444,

  specs: ["./tests/e2e/**/*.spec.ts"],
  exclude: [],

  maxInstances: 1,

  capabilities: [
    {
      maxInstances: 1,
      "tauri:options": {
        application: binaryPath,
      },
    },
  ],

  logLevel: "warn",
  bail: 0,
  waitforTimeout: 10_000,
  connectionRetryTimeout: 30_000,
  connectionRetryCount: 3,

  framework: "mocha",
  reporters: ["spec"],

  mochaOpts: {
    ui: "bdd",
    timeout: 60_000,
  },

  // Start tauri-driver before the session; it in turn starts WebKitWebDriver.
  // Also validates that the E2E binary exists, giving an actionable error if not.
  beforeSession: async (_config, _capabilities) => {
    const fs = await import("fs/promises");
    try {
      await fs.access(binaryPath);
    } catch {
      throw new Error(
        `E2E binary not found at: ${binaryPath}\n` +
          `Build it with: cd src-tauri && cargo build --features e2e-testing\n` +
          `Or set TAUTERM_BINARY_PATH to point to an existing binary.`
      );
    }
    tauriDriver = spawn(tauriDriverPath, [], {
      stdio: [null, process.stdout, process.stderr],
    });
  },

  // Wait for the app shell to be fully rendered before any spec runs.
  // tauri-driver connects as soon as the process starts, but the WebView
  // takes time to load the embedded assets and mount the Svelte app.
  before: async () => {
    await browser.waitUntil(
      async () => {
        try {
          const el = await $(".app-shell");
          return await el.isExisting();
        } catch {
          return false;
        }
      },
      {
        timeout: 30_000,
        timeoutMsg: "App shell (.app-shell) did not appear within 30 s — app may have failed to start",
        interval: 500,
      }
    );

    // Reset locale to English so all specs see locale-independent strings.
    await browser.execute((): void => {
      (window as any).__TAURI_INTERNALS__.invoke('update_preferences', {
        patch: { appearance: { language: 'en' } },
      });
    });
    await browser.pause(200); // allow backend to persist + frontend to react
  },

  // Kill tauri-driver after the session to avoid zombie processes.
  afterSession: () => {
    tauriDriver?.kill();
    tauriDriver = undefined;
  },
};
