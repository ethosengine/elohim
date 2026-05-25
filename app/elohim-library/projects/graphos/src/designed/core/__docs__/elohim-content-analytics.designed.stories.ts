/**
 * Library B designed story — elohim-content-analytics
 *
 * Binds the Elohim brand tokens to `<elohim-content-analytics>` via
 * story-decorator overrides. The primitive itself is NEVER modified — binding
 * happens via `--elohim-analytics-*` CSS custom properties mapped to `--el-*`.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';
import type { ContentAnalyticsMetrics } from 'elohim-core';

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
  --el-font-display: 'Fraunces', Georgia, serif;
`;

const ANALYTICS_TOKENS_LIGHT = `
  --elohim-analytics-padding:      1.25rem;
  --elohim-analytics-bg:           color-mix(in oklch, var(--el-stone) 5%, var(--el-cream));
  --elohim-analytics-fg:           var(--el-stone);
  --elohim-analytics-title-size:   1rem;
  --elohim-analytics-value-size:   1.75rem;
  --elohim-analytics-value-weight: 700;
  --elohim-analytics-label-size:   0.75rem;
  --elohim-analytics-label-color:  var(--el-stone);
  --elohim-analytics-grid-gap:     1rem;
  --elohim-analytics-note-size:    0.7rem;
  --elohim-analytics-note-color:   color-mix(in oklch, var(--el-stone) 70%, transparent);
`;

const ANALYTICS_TOKENS_DARK = `
  --elohim-analytics-padding:      1.25rem;
  --elohim-analytics-bg:           color-mix(in oklch, var(--el-starlight) 6%, var(--el-night));
  --elohim-analytics-fg:           var(--el-starlight);
  --elohim-analytics-title-size:   1rem;
  --elohim-analytics-value-size:   1.75rem;
  --elohim-analytics-value-weight: 700;
  --elohim-analytics-label-size:   0.75rem;
  --elohim-analytics-label-color:  color-mix(in oklch, var(--el-starlight) 70%, transparent);
  --elohim-analytics-grid-gap:     1rem;
  --elohim-analytics-note-size:    0.7rem;
  --elohim-analytics-note-color:   color-mix(in oklch, var(--el-starlight) 50%, transparent);
`;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const RICH_METRICS: ContentAnalyticsMetrics = {
  viewCount: 1_247,
  completionCount: 834,
  completionRate: 67,
};

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Core/elohim-content-analytics',
  parameters: {
    docs: {
      description: {
        component:
          'Library B — brand-bound view of `<elohim-content-analytics>`. ' +
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
      <div style="${EL_TOKENS}${ANALYTICS_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        padding: 1.5rem;
        max-inline-size: 420px;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-content-analytics .metrics=${RICH_METRICS}></elohim-content-analytics>`,
};

export const Dark: Story = {
  name: 'Dark (Night)',
  decorators: [
    story => html`
      <div style="${EL_TOKENS}${ANALYTICS_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        color: var(--el-starlight);
        color-scheme: dark;
        padding: 1.5rem;
        max-inline-size: 420px;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-content-analytics .metrics=${RICH_METRICS}></elohim-content-analytics>`,
};
