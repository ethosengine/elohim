/// <reference types="vitest" />
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    root: './src',
    include: ['**/*.spec.ts'],
    exclude: ['node_modules', 'dist', 'resilience/**'],
    pool: 'forks',
    maxForks: 8,
    reporters: ['default'],
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
