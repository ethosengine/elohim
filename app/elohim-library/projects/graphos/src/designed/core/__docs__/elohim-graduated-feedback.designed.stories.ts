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
  --el-clay:        #B8664F;
  --el-cream:       #F5F0E8;
  --el-stone:       #6B6157;
  --el-starlight:   #E8E4D9;
  --el-night:       #0F1A12;
  --el-font-ui:     'DM Sans', system-ui, sans-serif;
  --el-radius-sm:   4px;
`;

/**
 * The Gradients-of-Agreement scale in the protocol's moral-signal palette:
 * terracotta caution → harvest-gold uncertainty → green growth. Per-slot
 * vars drive resting border accents, the selected fill, and the community
 * distribution bar; fg vars are bound in pairs per the primitive's @cssprop
 * contract. Harvest Gold at the midpoint + the amber slider accent satisfy
 * the golden-hour rule in both themes.
 */
const FEEDBACK_SCALE_TOKENS = `
  --elohim-feedback-scale-0:      var(--el-clay);
  --elohim-feedback-scale-0-fg:   var(--el-cream);
  --elohim-feedback-scale-25:     color-mix(in oklch, var(--el-clay) 45%, var(--el-amber));
  --elohim-feedback-scale-25-fg:  var(--el-night);
  --elohim-feedback-scale-50:     var(--el-amber);
  --elohim-feedback-scale-50-fg:  var(--el-night);
  --elohim-feedback-scale-75:     var(--el-green-light);
  --elohim-feedback-scale-75-fg:  var(--el-night);
  --elohim-feedback-scale-100:    var(--el-green-deep);
  --elohim-feedback-scale-100-fg: var(--el-cream);
`;

const FEEDBACK_TOKENS_LIGHT = `
  ${FEEDBACK_SCALE_TOKENS}
  --elohim-feedback-label-color:     var(--el-stone);
  --elohim-feedback-position-bg:     color-mix(in oklch, var(--el-stone) 6%, var(--el-cream));
  --elohim-feedback-position-fg:     var(--el-stone);
  --elohim-feedback-position-radius: var(--el-radius-sm);
  --elohim-feedback-accent:          var(--el-amber);
  --elohim-feedback-submit-bg:       var(--el-green-deep);
  --elohim-feedback-submit-fg:       var(--el-cream);
`;

const FEEDBACK_TOKENS_DARK = `
  ${FEEDBACK_SCALE_TOKENS}
  --elohim-feedback-label-color:     var(--el-starlight);
  --elohim-feedback-position-bg:     color-mix(in oklch, var(--el-starlight) 8%, var(--el-night));
  --elohim-feedback-position-fg:     var(--el-starlight);
  --elohim-feedback-position-radius: var(--el-radius-sm);
  --elohim-feedback-accent:          var(--el-amber);
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

export const ProposalWithAggregates: Story = {
  name: 'Proposal + community distribution (Linen)',
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
  render: () => html`
    <elohim-graduated-feedback
      context="proposal"
      .totalResponses=${23}
      .distribution=${{
        Block: 1,
        Disagree: 3,
        Abstain: 4,
        Agree: 9,
        'Strongly Agree': 6,
      }}
    ></elohim-graduated-feedback>
  `,
};
