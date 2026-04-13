// SPDX-License-Identifier: MPL-2.0
/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_PERF_INSTRUMENTATION?: string;
  readonly VITE_APP_VERSION: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
