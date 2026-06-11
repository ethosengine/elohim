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
import type { ReactionConstraints } from 'elohim-core';

// ---------------------------------------------------------------------------
// Brand token declaration
// ---------------------------------------------------------------------------

const EL_TOKENS = `
  --el-green-deep:  #2D5F3B;
  --el-green-light: #7FB069;
  --el-amber:       #D4A03E;
  --el-clay:        #B8664F;
  --el-cream:       #F5F0E8;
  --el-stone:       #6B6157;
  --el-starlight:   #E8E4D9;
  --el-night:       #0F1A12;
  --el-font-ui:     'DM Sans', system-ui, sans-serif;
  --el-radius-sm: 4px;
  --el-radius-pill: 999px;
`;

/**
 * Active reactions glow Harvest Gold (spec §6: "Active/selected nodes use
 * --el-amber") — the golden-hour element in both themes. Mediation surfaces
 * carry the terracotta warning register (never aggressive red).
 */
const REACTION_TOKENS_LIGHT = `
  --elohim-reaction-btn-bg:          color-mix(in oklch, var(--el-stone) 6%, var(--el-cream));
  --elohim-reaction-btn-fg:          var(--el-stone);
  --elohim-reaction-btn-border:      1px solid color-mix(in oklch, var(--el-stone) 20%, transparent);
  --elohim-reaction-btn-radius:      var(--el-radius-pill);
  --elohim-reaction-btn-active-bg:   color-mix(in oklch, var(--el-amber) 22%, var(--el-cream));
  --elohim-reaction-btn-active-fg:   color-mix(in oklch, var(--el-stone) 60%, var(--el-night));
  --elohim-reaction-btn-active-border: 1px solid var(--el-amber);
  --elohim-reaction-count-fg:        var(--el-stone);
  --elohim-reaction-mediation-bg:    var(--el-cream);
  --elohim-reaction-mediation-fg:    var(--el-stone);
  --elohim-reaction-warning-color:   var(--el-clay);
  --elohim-reaction-scrim:           color-mix(in oklch, var(--el-night) 45%, transparent);
`;

const REACTION_TOKENS_DARK = `
  --elohim-reaction-btn-bg:          color-mix(in oklch, var(--el-starlight) 8%, var(--el-night));
  --elohim-reaction-btn-fg:          var(--el-starlight);
  --elohim-reaction-btn-border:      1px solid color-mix(in oklch, var(--el-starlight) 15%, transparent);
  --elohim-reaction-btn-radius:      var(--el-radius-pill);
  --elohim-reaction-btn-active-bg:   color-mix(in oklch, var(--el-amber) 25%, var(--el-night));
  --elohim-reaction-btn-active-fg:   var(--el-amber);
  --elohim-reaction-btn-active-border: 1px solid var(--el-amber);
  --elohim-reaction-count-fg:        var(--el-starlight);
  --elohim-reaction-mediation-bg:    color-mix(in oklch, var(--el-starlight) 6%, var(--el-night));
  --elohim-reaction-mediation-fg:    var(--el-starlight);
  --elohim-reaction-warning-color:   color-mix(in oklch, var(--el-clay) 70%, var(--el-amber));
  --elohim-reaction-scrim:           rgb(5 10 6 / 60%);
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

const MEDIATED_CONSTRAINTS: ReactionConstraints = {
  permittedTypes: ['moved', 'uncomfortable'],
  mediatedTypes: [
    {
      type: 'uncomfortable',
      constitutionalReasoning:
        'Expressing discomfort shapes the governance of this content. Proceed only if ' +
        'you can speak to why this content creates discomfort for you or others.',
      proceedBehavior: { visibleToOthers: true },
    },
  ],
};

export const MediatedLight: Story = {
  name: 'Mediated (terracotta register, Linen)',
  decorators: [
    story => html`
      <div style="${EL_TOKENS}${REACTION_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        padding: 1.5rem;
        max-inline-size: 500px;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <p style="font-size: 0.875rem; color: var(--el-stone); margin: 0 0 0.5rem;">
      The mediated reaction carries the terracotta indicator — earthy caution, never
      alarm-red. Click "uncomfortable" to open the constitutional-reasoning dialog
      (mediation surface bound to linen/stone with a warm night scrim).
    </p>
    <elohim-reaction-bar .constraints=${MEDIATED_CONSTRAINTS}></elohim-reaction-bar>
  `,
};
