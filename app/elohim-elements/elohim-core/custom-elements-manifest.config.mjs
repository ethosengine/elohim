// Forward-affordance: when component-CID federation tooling lands,
// add a plugin here that hashes each declaration and writes a
// `componentCid` field per entry. Until then, the field is reserved
// but not populated. See:
// genesis/docs/superpowers/specs/2026-05-06-elohim-lit-component-pivot-design.md (D8)
export default {
  globs: ['src/**/*.ts'],
  exclude: ['src/**/*.spec.ts'],
  outdir: 'dist',
  litelement: true,
  packagejson: false,
};
