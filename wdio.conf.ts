// SPDX-License-Identifier: MPL-2.0
import os from "os";
import path from "path";
import { spawn } from "child_process";
import { fileURLToPath } from "url";
import type { Options } from "@wdio/types";

const __dirname = fileURLToPath(new URL(".", import.meta.url));

// Path to the Tauri binary to test.
// Override with TAUTERM_BINARY_PATH to point at a custom build location.
// Default: the debug binary produced by `cargo build --features e2e-testing`
// (run from src-tauri/).
const binaryPath =
  process.env.TAUTERM_BINARY_PATH ??
  path.resolve(__dirname, "src-tauri", "target", "debug", "tau-term");

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

  // Kill tauri-driver after the session to avoid zombie processes.
  afterSession: () => {
    tauriDriver?.kill();
    tauriDriver = undefined;
  },
};
