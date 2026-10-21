import { defineConfig } from 'vitest/config';

// Resolve workspace dependencies through their built public exports, as consumers do.
export default defineConfig({
  test: {
    include: ['tests/**/*.test.ts'],
    environment: 'node',
  },
});
