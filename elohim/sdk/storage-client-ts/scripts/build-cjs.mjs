import { build } from 'esbuild';

// Cucumber/TSX loads this package through require(). Bundle the ESM-only EPR
// dependency chain for that entry, while keeping the existing ESM output intact.
// Automerge must retain its own Node conditional export and adjacent WASM asset.
await build({
  entryPoints: ['src/index.ts'],
  outfile: 'dist/index.cjs',
  bundle: true,
  format: 'cjs',
  platform: 'node',
  target: 'node18',
  external: ['@automerge/automerge', 'zod'],
  sourcemap: true,
});
