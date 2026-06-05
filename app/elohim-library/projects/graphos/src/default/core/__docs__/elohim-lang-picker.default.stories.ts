/**
 * Library A default story — elohim-lang-picker
 *
 * Blank-slate proof for the language selector. The element drives the shared
 * LocaleStore (lit-localize setLocale + document lang/dir + localStorage
 * persistence) — the same contract the Angular LocaleService speaks, so any
 * picker anywhere stays in sync.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

const meta: Meta = {
  title: 'Default/Core/elohim-lang-picker',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-lang-picker>\` — native \`<select>\` language override driven by the shared LocaleStore.

**Override surface:** \`--elohim-lang-picker-bg\`, \`--elohim-lang-picker-fg\`, \`--elohim-lang-picker-border\`.
**Parts:** \`label\`, \`select\`. **Events:** \`locale-changed\`.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

export const Default: Story = {
  render: () => html`<elohim-lang-picker></elohim-lang-picker>`,
};

export const Dark: Story = {
  render: () => html`
    <div style="background:#101218;color:#e8eaf0;padding:2rem;">
      <elohim-lang-picker></elohim-lang-picker>
    </div>
  `,
};

// ---------------------------------------------------------------------------
// RTL canary
// ---------------------------------------------------------------------------

export const RTLCanary: Story = {
  name: 'RTLCanary',
  decorators: [
    (story) => html`
      <div dir="rtl" lang="he" style="padding: 0;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-lang-picker></elohim-lang-picker>`,
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary: lang-picker in a right-to-left container. `padding-inline` and logical properties ensure the select renders correctly under RTL.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Blank-slate proof
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  render: () => html`
    <div style="all: initial;">
      <elohim-lang-picker></elohim-lang-picker>
    </div>
  `,
};

// ---------------------------------------------------------------------------
// CustomTheme — override-surface proof
// ---------------------------------------------------------------------------

export const CustomTheme: Story = {
  name: 'CustomTheme (override-surface proof)',
  render: () => html`
    <div
      style="
        --elohim-lang-picker-bg: #062a06;
        --elohim-lang-picker-fg: #6cff6c;
        --elohim-lang-picker-border: #0c4d0c;
        padding: 2rem;
        background: #021202;
      "
    >
      <elohim-lang-picker></elohim-lang-picker>
    </div>
  `,
};
