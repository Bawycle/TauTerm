// SPDX-License-Identifier: MPL-2.0
import { execSync } from 'node:child_process';
import type { Plugin } from 'vite';

export function licenseGeneratorPlugin(): Plugin {
  return {
    name: 'license-generator',
    buildStart() {
      try {
        execSync('./scripts/generate-licenses.sh', { stdio: 'pipe' });
      } catch {
        console.warn('[license-generator] Script failed — THIRD-PARTY-NOTICES.json not updated');
      }
    },
  };
}
