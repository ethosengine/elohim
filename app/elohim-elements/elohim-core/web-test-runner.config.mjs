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
export { axe };
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

export default {
  files: 'src/**/*.spec.ts',
  nodeResolve: true,
  browsers: [playwrightLauncher({ product: 'chromium' })],
  plugins: [
    axeCoreShim(),
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
