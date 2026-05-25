/**
 * Library B designed story — elohim-graduated-feedback
 *
 * Binds the Elohim brand tokens to `<elohim-graduated-feedback>` via
 * story-decorator overrides. The primitive itself is NEVER modified — binding
 * happens via `--elohim-feedback-*` CSS custom properties mapped to `--el-*`.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

// ---------------------------------------------------------------------------
// Brand tokens
// ---------------------------------------------------------------------------

const EL_TOKENS = `
  --el-green-deep:  #2D5F3B;
  --el-green-light: #7FB069;
  --el-amber:       #D4A03E;
  --el-cream:       #F5F0E8;
  --el-stone:       #6B6157;
  --el-starlight:   #E8E4D9;
  --el-night:       #0F1A12;
  --el-font-ui:     'DM Sans', system-ui, sans-serif;
  --el-radius-sm:   4px;
`;

const FEEDBACK_TOKENS_LIGHT = `
  --elohim-feedback-label-color:     var(--el-stone);
  --elohim-feedback-position-bg:     color-mix(in oklch, var(--el-stone) 6%, var(--el-cream));
  --elohim-feedback-position-fg:     var(--el-stone);
  --elohim-feedback-position-border: 1px solid color-mix(in oklch, var(--el-stone) 20%, transparent);
  --elohim-feedback-position-radius: var(--el-radius-sm);
  --elohim-feedback-position-active-bg:   color-mix(in oklch, var(--el-green-deep) 15%, var(--el-cream));
  --elohim-feedback-position-active-fg:   var(--el-green-deep);
  --elohim-feedback-submit-bg:       var(--el-green-deep);
  --elohim-feedback-submit-fg:       var(--el-cream);
`;

const FEEDBACK_TOKENS_DARK = `
  --elohim-feedback-label-color:     var(--el-starlight);
  --elohim-feedback-position-bg:     color-mix(in oklch, var(--el-starlight) 8%, var(--el-night));
  --elohim-feedback-position-fg:     var(--el-starlight);
  --elohim-feedback-position-border: 1px solid color-mix(in oklch, var(--el-starlight) 15%, transparent);
  --elohim-feedback-position-radius: var(--el-radius-sm);
  --elohim-feedback-position-active-bg:   color-mix(in oklch, var(--el-green-light) 20%, var(--el-night));
  --elohim-feedback-position-active-fg:   var(--el-green-light);
  --elohim-feedback-submit-bg:       var(--el-green-light);
  --elohim-feedback-submit-fg:       var(--el-night);
`;

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Core/elohim-graduated-feedback',
  parameters: {
    docs: {
      description: {
        component:
          'Library B — brand-bound view of `<elohim-graduated-feedback>`. ' +
          'Tokens bound via decorator; primitive not modified.',
      },
    },
  },
};

export default meta;
type Story = StoryObj;

export const Light: Story = {
  name: 'Light (Linen)',
  decorators: [
    story => html`
      <div style="${EL_TOKENS}${FEEDBACK_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        padding: 1.5rem;
        max-inline-size: 500px;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-graduated-feedback context="usefulness"></elohim-graduated-feedback>`,
};

export const Dark: Story = {
  name: 'Dark (Night)',
  decorators: [
    story => html`
      <div style="${EL_TOKENS}${FEEDBACK_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        color: var(--el-starlight);
        color-scheme: dark;
        padding: 1.5rem;
        max-inline-size: 500px;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-graduated-feedback context="usefulness"></elohim-graduated-feedback>`,
};
