import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import dts from 'vite-plugin-dts';

export default defineConfig({
  build: {
    lib: {
      entry: {
        index: resolve(__dirname, 'src/index.ts'),
        register: resolve(__dirname, 'src/register.ts'),
        'testing/index': resolve(__dirname, 'src/testing/index.ts'),
      },
      formats: ['es'],
      fileName: (_format, entry) => `${entry}.js`,
    },
    rollupOptions: {
      // @web/test-runner-commands only resolves inside the web-test-runner
      // (its browser module imports the runner's websocket virtual module) —
      // it stays external in the library build like the other harness deps.
      external: [
        /^lit($|\/)/,
        /^@open-wc\//,
        'axe-core',
        /^@lit\//,
        /^@web\/test-runner-commands/,
      ],
    },
    sourcemap: true,
    target: 'es2022',
  },
  plugins: [
    dts({
      entryRoot: 'src',
      include: ['src/**/*.ts'],
      exclude: ['src/**/*.spec.ts'],
    }),
  ],
});
