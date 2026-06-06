/// <reference types="vitest" />
import { defineConfig } from 'vite';
import angular from '@analogjs/vite-plugin-angular';
import tsconfigPaths from 'vite-tsconfig-paths';

export default defineConfig({
  plugins: [angular({ tsconfig: 'tsconfig.spec.json' }), tsconfigPaths()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['src/test-setup.ts'],
    include: [
      'src/**/*.spec.ts',
      'projects/elohim-identity/src/**/*.spec.ts',
      'projects/elohim-rea-runtime/src/**/*.spec.ts',
      'projects/elohim-service/src/resilience/**/*.spec.ts',
      'projects/elohim-service/src/distribution/**/*.spec.ts',
      'projects/graphos/src/**/*.spec.ts',
    ],
    exclude: ['node_modules', 'dist'],
    pool: 'forks',
    // Vitest 4 removed `poolOptions`; `poolOptions.forks.maxForks` is now the
    // top-level `maxWorkers`. See migration guide #pool-rework.
    maxWorkers: 8,
    reporters: ['default'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html', 'lcov'],
      reportsDirectory: 'coverage/vitest',
      include: ['src/**/*.ts', 'projects/graphos/src/**/*.ts'],
      exclude: [
        'src/**/*.spec.ts',
        'src/**/*.d.ts',
        'src/main.ts',
        'src/test-setup.ts',
        'projects/graphos/src/**/*.spec.ts',
      ],
    },
  },
});
