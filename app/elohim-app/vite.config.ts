/// <reference types="vitest" />
import { defineConfig } from 'vite';
import angular from '@analogjs/vite-plugin-angular';
import tsconfigPaths from 'vite-tsconfig-paths';

export default defineConfig({
  plugins: [angular({ tsconfig: 'tsconfig.spec.json' }), tsconfigPaths()],
  resolve: {
    alias: [
      // elohim-imagodei/federated-identifier is a workspace ESM subpath export
      // that Vite's module runner does not resolve in pool:forks test workers.
      // Point directly at the TypeScript source so the Vitest transform pipeline
      // handles it the same way it handles @app/* aliases via tsconfigPaths.
      // The production Angular build (webpack/esbuild) uses the workspace
      // node_modules symlink directly, unaffected by this alias.
      {
        find: 'elohim-imagodei/federated-identifier',
        replacement: new URL(
          '../elohim-elements/elohim-imagodei/src/federated-identifier.ts',
          import.meta.url,
        ).pathname,
      },
    ],
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['src/test-setup.ts'],
    include: ['src/**/*.spec.ts'],
    exclude: ['node_modules', 'dist'],
    pool: 'forks',
    maxForks: 8,
    testTimeout: 10000,
    hookTimeout: 10000,
    reporters: ['default'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html', 'lcov'],
      reportsDirectory: 'coverage/vitest',
      include: ['src/**/*.ts'],
      exclude: [
        'src/**/*.spec.ts',
        'src/**/*.d.ts',
        'src/main.ts',
        'src/test-setup.ts',
      ],
    },
  },
});
