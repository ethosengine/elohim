/**
 * Library B designed story — elohim-epr-relationships-panel
 *
 * Binds the Elohim brand tokens to `<elohim-epr-relationships-panel>` via
 * story-decorator overrides. The primitive itself is NEVER modified — binding
 * happens via `--elohim-epr-panel-*` CSS custom properties mapped to `--el-*`.
 *
 * Sources of truth:
 *   1. Types — `EprRelationship` from `elohim-core`.
 *   2. Manifest vocabulary — relationship types PREREQUISITE, TEACHES, CONTAINS,
 *      REFERENCES are the protocol vocabulary declared in the lamad manifest.
 *   3. Brand tokens — `--el-*` palette from design spec §14.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';
import type { EprRelationship } from 'elohim-core';

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
  --el-night-alt:   #1A1A2E;
  --el-font-ui:     'DM Sans', system-ui, sans-serif;
  --el-font-body:   'Source Serif 4', Georgia, serif;
  --el-space-sm:  16px;
  --el-space-md:  24px;
  --el-radius-sm: 4px;
  --el-radius-md: 8px;
  --el-shadow-soft: 0 2px 8px rgba(107, 97, 87, 0.08);
`;

const PANEL_TOKENS_LIGHT = `
  --elohim-epr-panel-heading-color: var(--el-stone);
  --elohim-epr-panel-card-bg:       color-mix(in oklch, var(--el-stone) 6%, var(--el-cream));
  --elohim-epr-panel-card-border:   1px solid color-mix(in oklch, var(--el-stone) 15%, transparent);
  --elohim-epr-panel-card-fg:       var(--el-green-deep);
  --elohim-epr-panel-card-radius:   var(--el-radius-sm);
  --elohim-epr-panel-gap:           var(--el-space-sm);
`;

const PANEL_TOKENS_DARK = `
  --elohim-epr-panel-heading-color: var(--el-starlight);
  --elohim-epr-panel-card-bg:       color-mix(in oklch, var(--el-starlight) 8%, var(--el-night));
  --elohim-epr-panel-card-border:   1px solid color-mix(in oklch, var(--el-starlight) 15%, transparent);
  --elohim-epr-panel-card-fg:       var(--el-green-light);
  --elohim-epr-panel-card-radius:   var(--el-radius-sm);
  --elohim-epr-panel-gap:           var(--el-space-sm);
`;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const RELATIONSHIPS: EprRelationship[] = [
  { type: 'PREREQUISITE', target: 'epr:foundations-of-shefa', label: 'Foundations of Shefa' },
  { type: 'PREREQUISITE', target: 'epr:intro-to-rea', label: 'Introduction to REA' },
  { type: 'TEACHES', target: 'epr:household-budgeting', label: 'Household Budgeting' },
  { type: 'TEACHES', target: 'epr:stewardship-basics', label: 'Stewardship Basics' },
  { type: 'CONTAINS', target: 'epr:quiz-shefa-1', label: 'Shefa Quiz #1' },
  { type: 'REFERENCES', target: 'epr:glossary-rea', label: 'REA Glossary' },
];

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Core/elohim-epr-relationships-panel',
  parameters: {
    docs: {
      description: {
        component:
          'Library B — brand-bound view of `<elohim-epr-relationships-panel>`. ' +
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
      <div style="${EL_TOKENS}${PANEL_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        padding: var(--el-space-md);
        max-inline-size: 640px;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-epr-relationships-panel .relationships=${RELATIONSHIPS}></elohim-epr-relationships-panel>
  `,
};

export const Dark: Story = {
  name: 'Dark (Night)',
  decorators: [
    story => html`
      <div style="${EL_TOKENS}${PANEL_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        color: var(--el-starlight);
        color-scheme: dark;
        padding: var(--el-space-md);
        max-inline-size: 640px;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-epr-relationships-panel .relationships=${RELATIONSHIPS}></elohim-epr-relationships-panel>
  `,
};
