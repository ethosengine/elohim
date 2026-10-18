// Shared analyzer defaults for every element package.
export default {
  globs: ['src/**/*.ts'],
  exclude: ['src/**/*.spec.ts'],
  outdir: 'dist',
  litelement: true,
  packagejson: false,
  plugins: [
    {
      name: 'deterministic-module-order',
      packageLinkPhase({ customElementsManifest }) {
        // Glob enumeration varies by filesystem. Use locale-independent path order,
        // preserving declarations and all other arrays inside each module.
        customElementsManifest.modules.sort((a, b) =>
          a.path < b.path ? -1 : a.path > b.path ? 1 : 0,
        );
      },
    },
  ],
};
