/**
 * Library B designed story — elohim-gate-feedback-trigger
 *
 * Binds the Elohim brand tokens to `<elohim-gate-feedback-trigger>` via
 * story-decorator overrides. The primitive itself is NEVER modified — binding
 * happens via `--elohim-gate-trigger-*` and `--elohim-gate-modal-*` CSS custom
 * properties mapped to `--el-*`.
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
  --el-radius-md:   8px;
`;

const TRIGGER_TOKENS_LIGHT = `
  --elohim-gate-trigger-bg:          color-mix(in oklch, var(--el-stone) 8%, var(--el-cream));
  --elohim-gate-trigger-fg:          var(--el-stone);
  --elohim-gate-trigger-border:      1px solid color-mix(in oklch, var(--el-stone) 20%, transparent);
  --elohim-gate-trigger-radius:      var(--el-radius-sm);
  --elohim-gate-menu-bg:             var(--el-cream);
  --elohim-gate-menu-border:         1px solid color-mix(in oklch, var(--el-stone) 20%, transparent);
  --elohim-gate-menu-item-hover-bg:  color-mix(in oklch, var(--el-stone) 8%, transparent);
  --elohim-gate-modal-bg:            var(--el-cream);
  --elohim-gate-modal-fg:            var(--el-stone);
  --elohim-gate-modal-border:        1px solid color-mix(in oklch, var(--el-stone) 20%, transparent);
  --elohim-gate-submit-bg:           var(--el-green-deep);
  --elohim-gate-submit-fg:           var(--el-cream);
`;

const TRIGGER_TOKENS_DARK = `
  --elohim-gate-trigger-bg:          color-mix(in oklch, var(--el-starlight) 10%, var(--el-night));
  --elohim-gate-trigger-fg:          var(--el-starlight);
  --elohim-gate-trigger-border:      1px solid color-mix(in oklch, var(--el-starlight) 15%, transparent);
  --elohim-gate-trigger-radius:      var(--el-radius-sm);
  --elohim-gate-menu-bg:             color-mix(in oklch, var(--el-starlight) 5%, var(--el-night));
  --elohim-gate-menu-border:         1px solid color-mix(in oklch, var(--el-starlight) 15%, transparent);
  --elohim-gate-menu-item-hover-bg:  color-mix(in oklch, var(--el-starlight) 8%, transparent);
  --elohim-gate-modal-bg:            color-mix(in oklch, var(--el-starlight) 5%, var(--el-night));
  --elohim-gate-modal-fg:            var(--el-starlight);
  --elohim-gate-modal-border:        1px solid color-mix(in oklch, var(--el-starlight) 15%, transparent);
  --elohim-gate-submit-bg:           var(--el-green-light);
  --elohim-gate-submit-fg:           var(--el-night);
`;

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Core/elohim-gate-feedback-trigger',
  parameters: {
    docs: {
      description: {
        component:
          'Library B — brand-bound view of `<elohim-gate-feedback-trigger>`. ' +
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
      <div style="${EL_TOKENS}${TRIGGER_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        padding: 1.5rem;
        display: flex;
        justify-content: flex-end;
        min-block-size: 200px;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-gate-feedback-trigger content-id="designed-content-1"></elohim-gate-feedback-trigger>`,
};

export const Dark: Story = {
  name: 'Dark (Night)',
  decorators: [
    story => html`
      <div style="${EL_TOKENS}${TRIGGER_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        color: var(--el-starlight);
        color-scheme: dark;
        padding: 1.5rem;
        display: flex;
        justify-content: flex-end;
        min-block-size: 200px;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-gate-feedback-trigger content-id="designed-content-2"></elohim-gate-feedback-trigger>`,
};
