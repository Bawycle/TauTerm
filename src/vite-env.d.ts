// SPDX-License-Identifier: MPL-2.0
/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_PERF_INSTRUMENTATION?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
