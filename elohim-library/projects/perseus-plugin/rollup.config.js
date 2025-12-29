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
if (typeof globalThis !== 'undefined' && typeof globalThis.process === 'undefined') {
  globalThis.process = {
    env: { NODE_ENV: 'production' },
    cwd: function() { return '/'; },
    platform: 'browser',
    version: '',
    nextTick: function(fn) { setTimeout(fn, 0); }
  };
}
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
      intro: processShim
    },
    {
      file: 'dist/index.esm.js',
      format: 'es',
      sourcemap: true,
      inlineDynamicImports: true,
      intro: processShim
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
