import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { esbuildPlugin } from '@web/dev-server-esbuild';
import { playwrightLauncher } from '@web/test-runner-playwright';

// axe-core 4.x ships as a CJS-only UMD IIFE. In a browser ESM environment it
// has no `default` export, so the spec's `import axe from 'axe-core'` would
// fail at module-load time. The canonical pattern (used by chai-a11y-axe) is
// to import axe.min.js for side effects (it attaches itself to window.axe),
// then read window.axe.
//
// We redirect bare `axe-core` (and the resolved file paths wtr's nodeResolve
// produces) to a virtual ESM shim that does exactly this. Scoped to axe-core;
// no impact on other modules.
const SHIM_URL = '/__elohim__/axe-core-shim.js';
const AXE_RESOLVED_PATTERN = /[/\\]axe-core(?:@[^/\\]+)?[/\\]node_modules[/\\]axe-core[/\\]axe(?:\.min)?\.js$/;

function axeCoreShim() {
  return {
    name: 'axe-core-shim',
    serve(context) {
      if (context.path === SHIM_URL) {
        return {
          body: `
// Auto-generated ESM shim for axe-core 4.x (CJS-only UMD).
// chai-a11y-axe uses this exact pattern.
import 'axe-core/axe.min.js';
if (!window.axe) {
  throw new Error('axe-core failed to register on window.axe');
}
const axe = window.axe;
export default axe;
`,
          type: 'js',
        };
      }
      return undefined;
    },
    resolveImport({ source }) {
      if (source === 'axe-core') {
        return SHIM_URL;
      }
      if (typeof source === 'string' && AXE_RESOLVED_PATTERN.test(source)) {
        // Match either the unminified entry that wtr's nodeResolve picked, OR
        // chai-a11y-axe's bare `axe-core/axe.min.js`. Only redirect the
        // unminified `axe.js` since `axe.min.js` is what the shim itself
        // imports for side effects.
        if (source.endsWith('axe.min.js')) return undefined;
        return SHIM_URL;
      }
      return undefined;
    },
  };
}

// The theme-contrast gate injects the REAL token + binding sources as its
// fixture (theme-authority spec §4.1: never a copied fixture that can drift).
// Both files are plain CSS in .scss clothing (no SCSS syntax). The binding
// file lives outside the wtr rootDir (app/lamad/src/), so serve both at
// virtual URLs — same precedent as axeCoreShim's SHIM_URL.
const PKG_DIR = dirname(fileURLToPath(import.meta.url));
const THEME_FIXTURE_FILES = {
  '/__elohim__/tokens.css': resolve(PKG_DIR, 'tokens.scss'),
  '/__elohim__/chrome-binding.css': resolve(PKG_DIR, '../../lamad/src/_chrome-binding.scss'),
};

function themeFixtureFiles() {
  return {
    name: 'theme-fixture-files',
    serve(context) {
      const src = THEME_FIXTURE_FILES[context.path];
      if (src) {
        return { body: readFileSync(src, 'utf8'), type: 'css' };
      }
      return undefined;
    },
  };
}

export default {
  files: 'src/**/*.spec.ts',
  nodeResolve: true,
  browsers: [playwrightLauncher({ product: 'chromium' })],
  plugins: [
    axeCoreShim(),
    themeFixtureFiles(),
    esbuildPlugin({
      ts: true,
      target: 'es2022',
      tsconfig: './tsconfig.json',
    }),
  ],
  testFramework: {
    config: {
      ui: 'bdd',
      timeout: 5000,
    },
  },
};
