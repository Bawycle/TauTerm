import os from "os";
import path from "path";
import { spawn } from "child_process";
import { fileURLToPath } from "url";
import type { Options } from "@wdio/types";

const __dirname = fileURLToPath(new URL(".", import.meta.url));

// Respect TAURI_BUILD_TYPE env var — defaults to "debug" for faster iteration.
// Set TAURI_BUILD_TYPE=release to test the release binary.
const buildType = (process.env.TAURI_BUILD_TYPE ?? "debug") as
  | "debug"
  | "release";

const binaryPath = path.resolve(
  __dirname,
  "src-tauri",
  "target",
  buildType,
  "tau-term"
);

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
  beforeSession: () => {
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
