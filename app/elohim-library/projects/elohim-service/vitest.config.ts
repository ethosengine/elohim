/// <reference types="vitest" />
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    root: './src',
    include: ['**/*.spec.ts'],
    exclude: ['node_modules', 'dist', 'resilience/**', 'distribution/**'],
    pool: 'forks',
    maxForks: 8,
    reporters: ['default'],
    // Absolute: vitest >=4.1 resolves setupFiles against `root` (./src), not this file's directory.
    setupFiles: [fileURLToPath(new URL('./vitest.setup.ts', import.meta.url))],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov', 'html'],
      reportsDirectory: '../coverage',
      include: ['**/*.ts'],
      exclude: [
        '**/*.d.ts',
        '**/index.ts',
        '**/*.spec.ts',
      ],
    },
    testTimeout: 10000,
  },
});
