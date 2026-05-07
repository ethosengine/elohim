import type { StorybookConfig } from '@storybook/web-components-vite';

// Build output dir is set in package.json's `build-storybook` script
// (`--output-dir dist/storybook`) so the Jenkinsfile + nginx Dockerfile
// can find index.html / index.json. Storybook 10's CLI default is
// ./storybook-static which does NOT match that contract.
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
