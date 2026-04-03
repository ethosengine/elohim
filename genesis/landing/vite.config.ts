import { defineConfig } from 'vite';

export default defineConfig({
  root: '.',
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    // Minimal output — no framework, no code splitting
    rollupOptions: {
      input: 'index.html',
    },
  },
});
