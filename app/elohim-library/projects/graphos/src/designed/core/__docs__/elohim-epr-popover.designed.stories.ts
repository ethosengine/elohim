/**
 * Library B designed story — elohim-epr-popover
 *
 * Binds the Elohim brand tokens to `<elohim-epr-popover>` via story-decorator
 * overrides. The primitive itself is NEVER modified — binding happens via
 * `--elohim-epr-popover-*` and `--elohim-epr-*` CSS custom properties mapped
 * to `--el-*`.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';
import type { EprHead } from 'elohim-core';

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
  --el-night-alt:   #1A1A2E;
  --el-font-ui:     'DM Sans', system-ui, sans-serif;
  --el-radius-md:   8px;
  --el-shadow-medium: 0 4px 16px rgba(107, 97, 87, 0.12);
`;

const POPOVER_TOKENS_LIGHT = `
  --elohim-epr-popover-bg:     var(--el-cream);
  --elohim-epr-popover-fg:     var(--el-stone);
  --elohim-epr-popover-border: 1px solid color-mix(in oklch, var(--el-stone) 20%, transparent);
  --elohim-epr-popover-radius: var(--el-radius-md);
  --elohim-epr-popover-shadow: var(--el-shadow-medium);
  --elohim-epr-type-bg:        color-mix(in oklch, var(--el-green-deep) 15%, var(--el-cream));
  --elohim-epr-type-fg:        var(--el-green-deep);
  --elohim-epr-tag-bg:         color-mix(in oklch, var(--el-amber) 12%, var(--el-cream));
  --elohim-epr-tag-fg:         var(--el-stone);
  --elohim-epr-reach-bg:       color-mix(in oklch, var(--el-stone) 8%, var(--el-cream));
  --elohim-epr-reach-fg:       var(--el-stone);
  --elohim-epr-layer-bg:       color-mix(in oklch, var(--el-stone) 8%, var(--el-cream));
  --elohim-epr-layer-fg:       var(--el-stone);
  --elohim-epr-link-color:     var(--el-green-deep);
  --elohim-epr-divider-color:  color-mix(in oklch, var(--el-stone) 15%, transparent);
`;

const POPOVER_TOKENS_DARK = `
  --elohim-epr-popover-bg:     var(--el-night-alt);
  --elohim-epr-popover-fg:     var(--el-starlight);
  --elohim-epr-popover-border: 1px solid color-mix(in oklch, var(--el-starlight) 15%, transparent);
  --elohim-epr-popover-radius: var(--el-radius-md);
  --elohim-epr-popover-shadow: 0 4px 24px rgba(0, 0, 0, 0.4);
  --elohim-epr-type-bg:        color-mix(in oklch, var(--el-green-light) 20%, var(--el-night-alt));
  --elohim-epr-type-fg:        var(--el-green-light);
  --elohim-epr-tag-bg:         color-mix(in oklch, var(--el-starlight) 8%, var(--el-night-alt));
  --elohim-epr-tag-fg:         var(--el-starlight);
  --elohim-epr-reach-bg:       color-mix(in oklch, var(--el-starlight) 8%, var(--el-night-alt));
  --elohim-epr-reach-fg:       var(--el-starlight);
  --elohim-epr-layer-bg:       color-mix(in oklch, var(--el-starlight) 8%, var(--el-night-alt));
  --elohim-epr-layer-fg:       var(--el-starlight);
  --elohim-epr-link-color:     var(--el-green-light);
  --elohim-epr-divider-color:  color-mix(in oklch, var(--el-starlight) 12%, transparent);
`;

// ---------------------------------------------------------------------------
// Fixture
// ---------------------------------------------------------------------------

const RICH_HEAD: EprHead = {
  version: 1,
  id: 'epr:shefa-stewardship-101',
  content: 'bafybeigdyrztm7vexample0000000000000000000000000000000000000000',
  lamad: {
    title: 'Stewardship 101 — Foundations',
    contentType: 'lesson',
    contentFormat: 'sophia-quiz-json',
    description: 'A foundational lesson on economic stewardship in the Elohim Protocol.',
    tags: ['shefa', 'stewardship', 'rea', 'foundations'],
  },
  shefa: {
    stewards: ['agent-alice-hash', 'agent-bob-hash'],
  },
  qahal: {
    reach: 'household',
    layer: 'constitutional',
  },
  relationships: [],
};

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Core/elohim-epr-popover',
  parameters: {
    docs: {
      description: {
        component:
          'Library B — brand-bound view of `<elohim-epr-popover>`. ' +
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
      <div style="${EL_TOKENS}${POPOVER_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        padding: 2rem;
        inline-size: 360px;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-epr-popover
      .head=${RICH_HEAD}
      .visible=${true}
      route="/lamad/stewardship-101"
      style="position: relative; top: auto; left: auto;"
    ></elohim-epr-popover>
  `,
};

export const Dark: Story = {
  name: 'Dark (Night)',
  decorators: [
    story => html`
      <div style="${EL_TOKENS}${POPOVER_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        color: var(--el-starlight);
        color-scheme: dark;
        padding: 2rem;
        inline-size: 360px;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-epr-popover
      .head=${RICH_HEAD}
      .visible=${true}
      route="/lamad/stewardship-101"
      style="position: relative; top: auto; left: auto;"
    ></elohim-epr-popover>
  `,
};
