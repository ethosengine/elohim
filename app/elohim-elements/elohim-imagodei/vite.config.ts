import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import dts from 'vite-plugin-dts';

export default defineConfig({
  build: {
    lib: {
      entry: {
        index: resolve(__dirname, 'src/index.ts'),
        register: resolve(__dirname, 'src/register.ts'),
        'federated-identifier': resolve(__dirname, 'src/federated-identifier.ts'),
        'witness-step': resolve(__dirname, 'src/witness-step.ts'),
        'device-consent': resolve(__dirname, 'src/device-consent/index.ts'),
        'identity-standing': resolve(__dirname, 'src/identity-standing/index.ts'),
        'session-key': resolve(__dirname, 'src/session-key/index.ts'),
        'node-sign-in': resolve(__dirname, 'src/node-sign-in/index.ts'),
        'pending-asks': resolve(__dirname, 'src/pending-asks/index.ts'),
      },
      formats: ['es'],
      fileName: (_format, entry) => `${entry}.js`,
    },
    rollupOptions: {
      external: [/^lit($|\/)/, /^@open-wc\//, 'axe-core', /^@lit\//, /^elohim-core/],
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
