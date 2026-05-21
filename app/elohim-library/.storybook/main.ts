import type { StorybookConfig } from '@storybook/web-components-vite';

// Build output dir is set in package.json's `build-storybook` script
// (`--output-dir dist/storybook`) so the Jenkinsfile + nginx Dockerfile
// can find index.html / index.json. Storybook 10's CLI default is
// ./storybook-static which does NOT match that contract.
//
// Heads-up: empty `[build:storybook]` retriggers do NOT dispatch this
// pipeline today — orchestrator's buildTagAliases map (genesis/orchestrator/
// Jenkinsfile) lacks a `storybook` entry, so the tag is logged "Unknown"
// and only changeset analysis runs. Touch a real source path under
// app/elohim-library/ to force a rebuild.
const config: StorybookConfig = {
  // Allow-list the projects that ship web-components stories. `lamad-ui` is
  // intentionally omitted — its story files import from `@storybook/angular`
  // and render Angular components, which can't run under
  // `@storybook/web-components-vite` (the Angular Linker isn't loaded). Those
  // stories were broken in the deployed bundle as well, alongside the wider
  // CSF-export-stripping bug now fixed by the viteFinal hook below. When the
  // lamad-ui surface migrates to Lit web components (project memory: Lit/WC
  // pivot), add it back to this list.
  stories: ['../projects/graphos/**/__docs__/**/*.@(stories.ts|mdx)'],
  addons: [
    '@storybook/addon-a11y',
    '@storybook/addon-docs',
    '@storybook/addon-links',
  ],
  framework: {
    name: '@storybook/web-components-vite',
    options: {},
  },
  // The project root `vite.config.ts` registers `@analogjs/vite-plugin-angular`
  // for the Angular library + vitest harness. Storybook's vite builder picks up
  // that config automatically, and the Angular compiler then transforms every
  // `.ts` file (including web-components `.stories.ts`) — stripping the named
  // exports it doesn't recognize. The result is every story chunk reduces to
  // `export{r as __namedExportsOrder};` with no `default` and no story exports,
  // which crashes `processCSFFileWithCache` at runtime with
  // "Cannot destructure property 'id' of 'e' as it is undefined".
  //
  // Filter the Angular plugin out of Storybook's pipeline. The Angular library
  // and vitest still pick it up via the root vite.config.ts directly.
  async viteFinal(viteConfig) {
    // The Angular plugin (registered by the project's root vite.config.ts for
    // the Angular library + vitest harness) bundles itself as an array of
    // sub-plugins, so we have to flatten before filtering.
    const isAngularPlugin = (plugin: unknown): boolean => {
      if (!plugin || typeof plugin !== 'object' || Array.isArray(plugin)) return false;
      const name = (plugin as { name?: string }).name ?? '';
      return name.startsWith('analogjs-') || name.includes('vite-plugin-angular');
    };
    const flat = (viteConfig.plugins ?? []).flat(Infinity);
    viteConfig.plugins = flat.filter((p) => !isAngularPlugin(p));
    return viteConfig;
  },
};

export default config;
