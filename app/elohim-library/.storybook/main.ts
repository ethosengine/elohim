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
  stories: ['../projects/**/__docs__/**/*.@(stories.ts|mdx)'],
  addons: [
    '@storybook/addon-a11y',
    '@storybook/addon-docs',
    '@storybook/addon-links',
  ],
  framework: {
    name: '@storybook/web-components-vite',
    options: {},
  },
};

export default config;
