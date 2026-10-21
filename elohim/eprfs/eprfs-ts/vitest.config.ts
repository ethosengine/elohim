import { defineConfig } from 'vitest/config';

// Resolve workspace dependencies through their built public exports, as consumers do.
export default defineConfig({
  test: {
    environment: 'node',
    include: ['test/**/*.test.ts'],
  },
});
