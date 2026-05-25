/**
 * Library B designed story — elohim-feedback-mechanism-gateway
 *
 * Binds the Elohim brand tokens to `<elohim-feedback-mechanism-gateway>` via
 * story-decorator overrides. The primitive itself is NEVER modified — binding
 * happens via `--elohim-gateway-*` CSS custom properties mapped to `--el-*`.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';
import type { AccumulationStatus } from 'elohim-core';

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
`;

const GATEWAY_TOKENS_LIGHT = `
  --elohim-gateway-loading-color:      var(--el-stone);
  --elohim-gateway-sensemaking-bg:     color-mix(in oklch, var(--el-amber) 15%, var(--el-cream));
  --elohim-gateway-sensemaking-fg:     var(--el-green-deep);
  --elohim-gateway-controversy-bg:     color-mix(in oklch, #B8664F 15%, var(--el-cream));
  --elohim-gateway-controversy-fg:     #7a2020;
`;

const GATEWAY_TOKENS_DARK = `
  --elohim-gateway-loading-color:      var(--el-starlight);
  --elohim-gateway-sensemaking-bg:     color-mix(in oklch, var(--el-amber) 20%, var(--el-night));
  --elohim-gateway-sensemaking-fg:     var(--el-amber);
  --elohim-gateway-controversy-bg:     color-mix(in oklch, #B8664F 20%, var(--el-night));
  --elohim-gateway-controversy-fg:     #fca5a5;
`;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const SENSEMAKING: AccumulationStatus = {
  readyForSensemaking: true,
  controversyDetected: false,
  settled: false,
};

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Core/elohim-feedback-mechanism-gateway',
  parameters: {
    docs: {
      description: {
        component:
          'Library B — brand-bound view of `<elohim-feedback-mechanism-gateway>`. ' +
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
      <div style="${EL_TOKENS}${GATEWAY_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        padding: 1.5rem;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-feedback-mechanism-gateway
      .selection=${{ level: 1, renderTarget: 'angular' }}
      .accumulationStatus=${SENSEMAKING}
    >
      <div slot="reactions" style="padding: 0.5rem; font-size: 0.875rem; color: var(--el-stone);">
        [reactions slot]
      </div>
    </elohim-feedback-mechanism-gateway>
  `,
};

export const Dark: Story = {
  name: 'Dark (Night)',
  decorators: [
    story => html`
      <div style="${EL_TOKENS}${GATEWAY_TOKENS_DARK}
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
    <elohim-feedback-mechanism-gateway
      .selection=${{ level: 1, renderTarget: 'angular' }}
      .accumulationStatus=${SENSEMAKING}
    >
      <div slot="reactions" style="padding: 0.5rem; font-size: 0.875rem; color: var(--el-starlight);">
        [reactions slot]
      </div>
    </elohim-feedback-mechanism-gateway>
  `,
};
