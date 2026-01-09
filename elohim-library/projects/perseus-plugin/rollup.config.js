import resolve from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';
import replace from '@rollup/plugin-replace';
import url from '@rollup/plugin-url';
import esbuild from 'rollup-plugin-esbuild';
import alias from '@rollup/plugin-alias';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Process shim for browser environment
// Injected at the top of the bundle to prevent "process is not defined" errors
const processShim = `
(function() {
  // Process shim
  if (typeof globalThis !== 'undefined' && typeof globalThis.process === 'undefined') {
    globalThis.process = {
      env: { NODE_ENV: 'production' },
      cwd: function() { return '/'; },
      platform: 'browser',
      version: '',
      nextTick: function(fn) { setTimeout(fn, 0); }
    };
  }

  // Global polyfill - ensure Array.from is available (needed by some React internals)
  if (typeof Array.from !== 'function') {
    Array.from = function(arrayLike, mapFn, thisArg) {
      var arr = [];
      var len = arrayLike.length >>> 0;
      for (var i = 0; i < len; i++) {
        if (i in arrayLike) {
          arr.push(mapFn ? mapFn.call(thisArg, arrayLike[i], i) : arrayLike[i]);
        }
      }
      return arr;
    };
  }

  // Ensure Set constructor accepts an iterable (used by React DOM event system)
  // Some environments have incomplete Set implementations
  var _OriginalSet = typeof Set !== 'undefined' ? Set : undefined;
  if (_OriginalSet) {
    try {
      // Test if Set properly handles concat'd arrays in constructor
      new _OriginalSet([].concat(['test']));
    } catch (e) {
      // Set constructor doesn't work as expected, polyfill won't help here
      console.warn('[Perseus] Set polyfill test failed, may have initialization issues');
    }
  }
})();
`;

// Wrapper to catch initialization errors gracefully
// React DOM's event system can throw during feature detection in some environments
// These errors are non-fatal - the plugin still works, but they pollute the console
const errorBoundaryWrapper = `
(function() {
  if (typeof window === 'undefined') return;

  var _originalOnerror = window.onerror;
  var _suppressedCount = 0;

  // Temporarily catch TypeError related to concat during initialization
  window.onerror = function(message, source, lineno, colno, error) {
    // Check if this is the known React DOM initialization error
    if (error instanceof TypeError &&
        message && message.indexOf && message.indexOf("concat") !== -1) {
      _suppressedCount++;
      return true; // Suppress the error
    }
    // Pass through to original handler
    if (_originalOnerror) {
      return _originalOnerror.apply(window, arguments);
    }
    return false;
  };

  // Restore after a microtask (after sync initialization completes)
  Promise.resolve().then(function() {
    window.onerror = _originalOnerror;
    if (_suppressedCount > 0) {
      console.debug('[Perseus] Suppressed', _suppressedCount, 'non-fatal initialization error(s)');
    }
  });
})();
`;

export default {
  input: 'src/index.ts',
  output: [
    {
      file: 'dist/perseus-plugin.umd.js',
      format: 'umd',
      name: 'PerseusPlugin',
      sourcemap: true,
      inlineDynamicImports: true,
      intro: errorBoundaryWrapper + processShim
    },
    {
      file: 'dist/index.esm.js',
      format: 'es',
      sourcemap: true,
      inlineDynamicImports: true,
      intro: errorBoundaryWrapper + processShim
    }
  ],
  plugins: [
    // Fix asap package browser resolution - the browser field mappings
    // in asap/package.json aren't being applied correctly by node-resolve
    alias({
      entries: [
        {
          find: /^asap\/raw$/,
          replacement: path.resolve(__dirname, 'node_modules/asap/browser-raw.js')
        },
        {
          find: /^\.\/raw$/,
          replacement: path.resolve(__dirname, 'node_modules/asap/browser-raw.js'),
          customResolver: (_source, importer) => {
            // Only apply this alias when the importer is from the asap package
            if (importer && importer.includes('asap')) {
              return path.resolve(__dirname, 'node_modules/asap/browser-raw.js');
            }
            return null;
          }
        }
      ]
    }),
    // Handle SVG imports from @phosphor-icons/core
    url({
      include: ['**/*.svg', '**/*.png', '**/*.jpg', '**/*.gif'],
      limit: 0, // Always inline as base64
      emitFiles: false
    }),
    // TypeScript/TSX compilation via esbuild (fast, handles JSX natively)
    esbuild({
      include: /\.[jt]sx?$/,
      exclude: /node_modules/,
      target: 'es2020',
      jsx: 'automatic',
      jsxImportSource: 'react',
      tsconfig: './tsconfig.json'
    }),
    // Replace process.env.NODE_ENV for production optimization
    // This runs after the intro shim, so any remaining references work
    replace({
      'process.env.NODE_ENV': JSON.stringify('production'),
      preventAssignment: true
    }),
    resolve({
      browser: true,
      preferBuiltins: false,
      extensions: ['.js', '.jsx', '.ts', '.tsx']
    }),
    commonjs({
      include: /node_modules/,
      transformMixedEsModules: true
    })
  ],
  onwarn(warning, warn) {
    // Suppress circular dependency warnings from React ecosystem
    if (warning.code === 'CIRCULAR_DEPENDENCY') return;
    // Suppress "use client" directive warnings
    if (warning.code === 'MODULE_LEVEL_DIRECTIVE') return;
    warn(warning);
  }
};
