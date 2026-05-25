/**
 * Library B designed story — elohim-reaction-bar
 *
 * Binds the Elohim brand tokens to `<elohim-reaction-bar>` via story-decorator
 * overrides. The primitive itself is NEVER modified — binding happens via
 * `--elohim-reaction-*` CSS custom properties mapped to `--el-*`.
 *
 * Sources of truth:
 *   1. Types — `ReactionConstraints`, `ReactionCount` from `elohim-core`.
 *   2. Manifest vocabulary — reaction types are protocol vocabulary.
 *   3. Brand tokens — `--el-*` palette from design spec §14.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

// ---------------------------------------------------------------------------
// Brand token declaration
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
  --el-radius-sm: 4px;
  --el-radius-pill: 999px;
`;

const REACTION_TOKENS_LIGHT = `
  --elohim-reaction-btn-bg:          color-mix(in oklch, var(--el-stone) 6%, var(--el-cream));
  --elohim-reaction-btn-fg:          var(--el-stone);
  --elohim-reaction-btn-border:      1px solid color-mix(in oklch, var(--el-stone) 20%, transparent);
  --elohim-reaction-btn-radius:      var(--el-radius-pill);
  --elohim-reaction-btn-active-bg:   color-mix(in oklch, var(--el-green-deep) 15%, var(--el-cream));
  --elohim-reaction-btn-active-fg:   var(--el-green-deep);
  --elohim-reaction-btn-active-border: 1px solid var(--el-green-light);
  --elohim-reaction-count-color:     var(--el-stone);
`;

const REACTION_TOKENS_DARK = `
  --elohim-reaction-btn-bg:          color-mix(in oklch, var(--el-starlight) 8%, var(--el-night));
  --elohim-reaction-btn-fg:          var(--el-starlight);
  --elohim-reaction-btn-border:      1px solid color-mix(in oklch, var(--el-starlight) 15%, transparent);
  --elohim-reaction-btn-radius:      var(--el-radius-pill);
  --elohim-reaction-btn-active-bg:   color-mix(in oklch, var(--el-green-light) 20%, var(--el-night));
  --elohim-reaction-btn-active-fg:   var(--el-green-light);
  --elohim-reaction-btn-active-border: 1px solid var(--el-green-light);
  --elohim-reaction-count-color:     var(--el-starlight);
`;

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Core/elohim-reaction-bar',
  parameters: {
    docs: {
      description: {
        component:
          'Library B — brand-bound view of `<elohim-reaction-bar>`. ' +
          'Tokens bound via decorator; primitive not modified.',
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Stories
// ---------------------------------------------------------------------------

export const Light: Story = {
  name: 'Light (Linen)',
  decorators: [
    story => html`
      <div style="${EL_TOKENS}${REACTION_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        padding: 1.5rem;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-reaction-bar
      .showCounts=${true}
      .counts=${[{ type: 'moved', count: 14 }, { type: 'grateful', count: 7 }, { type: 'inspired', count: 22 }]}
      .userReactions=${['moved']}
    ></elohim-reaction-bar>
  `,
};

export const Dark: Story = {
  name: 'Dark (Night)',
  decorators: [
    story => html`
      <div style="${EL_TOKENS}${REACTION_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        color: var(--el-starlight);
        color-scheme: dark;
        padding: 1.5rem;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-reaction-bar
      .showCounts=${true}
      .counts=${[{ type: 'moved', count: 14 }, { type: 'grateful', count: 7 }]}
      .userReactions=${['grateful']}
    ></elohim-reaction-bar>
  `,
};
