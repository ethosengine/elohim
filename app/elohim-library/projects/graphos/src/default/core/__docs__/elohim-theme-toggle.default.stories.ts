/**
 * Library A default story — elohim-theme-toggle
 *
 * Blank-slate proof for the theme cycle button. The element drives the shared
 * ThemeStore (localStorage 'elohim-theme' + body[data-theme]) — the same
 * contract the Angular ThemeService speaks, so any toggle anywhere stays in sync.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

const meta: Meta = {
  title: 'Default/Core/elohim-theme-toggle',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-theme-toggle>\` — cycles device → light → dark through the shared ThemeStore.

**Override surface:** \`--elohim-theme-toggle-bg\`, \`--elohim-theme-toggle-fg\`,
\`--elohim-theme-toggle-border\`, \`--elohim-theme-toggle-badge-bg\`, \`--elohim-theme-toggle-badge-fg\`.
**Parts:** \`button\`, \`icon\`, \`auto-indicator\`. **Events:** \`theme-changed\`.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

export const Default: Story = {
  render: () => html`<elohim-theme-toggle></elohim-theme-toggle>`,
};

export const Dark: Story = {
  render: () => html`
    <div style="background:#101218;color:#e8eaf0;padding:2rem;">
      <elohim-theme-toggle></elohim-theme-toggle>
    </div>
  `,
};

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  render: () => html`
    <div style="all: initial;">
      <elohim-theme-toggle></elohim-theme-toggle>
    </div>
  `,
};

export const CustomTheme: Story = {
  name: 'CustomTheme (override-surface proof)',
  render: () => html`
    <div
      style="
        --elohim-theme-toggle-bg: #062a06;
        --elohim-theme-toggle-fg: #6cff6c;
        --elohim-theme-toggle-border: #0c4d0c;
        --elohim-theme-toggle-badge-bg: #0c4d0c;
        --elohim-theme-toggle-badge-fg: #6cff6c;
        padding: 2rem;
        background: #021202;
      "
    >
      <elohim-theme-toggle></elohim-theme-toggle>
    </div>
  `,
};
