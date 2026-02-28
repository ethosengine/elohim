/// <reference types="vitest" />
import { defineConfig } from 'vite';
import angular from '@analogjs/vite-plugin-angular';
import tsconfigPaths from 'vite-tsconfig-paths';

export default defineConfig({
  plugins: [angular({ tsconfig: 'tsconfig.vitest.json' }), tsconfigPaths()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['src/test-setup.vitest.ts'],
    include: ['src/**/*.vitest.spec.ts'],
    exclude: ['node_modules', 'dist'],
    reporters: ['default'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html', 'lcov'],
      reportsDirectory: 'coverage/vitest',
      include: ['src/**/*.ts'],
      exclude: [
        'src/**/*.spec.ts',
        'src/**/*.vitest.spec.ts',
        'src/**/*.d.ts',
        'src/main.ts',
        'src/test-setup.vitest.ts',
      ],
    },
  },
});
