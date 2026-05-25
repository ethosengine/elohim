/**
 * Library A default story — elohim-epr-relationships-panel
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * <elohim-epr-relationships-panel> displays an EPR's relationships grouped by
 * type in protocol-defined order (PREREQUISITE, TEACHES, CONTAINS, REFERENCES).
 * Relationship types with no entries are omitted.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';
import type { EprRelationship } from 'elohim-core';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const SAMPLE_RELATIONSHIPS: EprRelationship[] = [
  { type: 'PREREQUISITE', target: 'epr:foundations-of-shefa', label: 'Foundations of Shefa' },
  { type: 'PREREQUISITE', target: 'epr:intro-to-rea', label: 'Introduction to REA' },
  { type: 'TEACHES', target: 'epr:household-budgeting', label: 'Household Budgeting' },
  { type: 'TEACHES', target: 'epr:stewardship-basics', label: 'Stewardship Basics' },
  { type: 'CONTAINS', target: 'epr:quiz-shefa-1', label: 'Shefa Quiz #1' },
  { type: 'REFERENCES', target: 'epr:glossary-rea', label: 'REA Glossary' },
];

const PREREQUISITE_ONLY: EprRelationship[] = [
  { type: 'PREREQUISITE', target: 'epr:lamad-intro', label: 'Introduction to Learning' },
];

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-epr-relationships-panel',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-epr-relationships-panel>\` — groups EPR relationships by type in protocol order.

Displays PREREQUISITE, TEACHES, CONTAINS, REFERENCES groups. Empty groups are omitted.
Clicking a relationship card emits \`epr-navigate\` or calls the \`.navigator\` callback.

**Override surface:** \`--elohim-epr-panel-*\`, \`--elohim-epr-panel-card-*\`.
**Parts:** \`panel\`, \`group\`, \`group-heading\`, \`group-grid\`, \`card\`, \`card-type\`, \`card-target\`.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Core stories
// ---------------------------------------------------------------------------

export const Default: Story = {
  name: 'Default (all relationship types)',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 600px;">
      <elohim-epr-relationships-panel
        .relationships=${SAMPLE_RELATIONSHIPS}
      ></elohim-epr-relationships-panel>
    </div>
  `,
};

export const PrerequisiteOnly: Story = {
  name: 'PrerequisiteOnly',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 400px;">
      <elohim-epr-relationships-panel
        .relationships=${PREREQUISITE_ONLY}
      ></elohim-epr-relationships-panel>
    </div>
  `,
  parameters: {
    docs: {
      description: { story: 'Only PREREQUISITE relationships present — other groups are omitted.' },
    },
  },
};

export const Empty: Story = {
  name: 'Empty (no relationships)',
  render: () => html`
    <div style="padding: 1rem;">
      <elohim-epr-relationships-panel .relationships=${[]}></elohim-epr-relationships-panel>
    </div>
  `,
  parameters: {
    docs: {
      description: { story: 'Empty state — no relationship groups rendered.' },
    },
  },
};

export const WithNavigatorCallback: Story = {
  name: 'WithNavigatorCallback (interactive)',
  render: () => {
    const onNavigate = (target: string) =>
      // eslint-disable-next-line no-console
      console.log('navigator callback:', target);
    return html`
      <div style="padding: 1rem; max-inline-size: 600px;">
        <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
          Click any card — navigator callback fires (logged to console).
        </p>
        <elohim-epr-relationships-panel
          .relationships=${SAMPLE_RELATIONSHIPS}
          .navigator=${onNavigate}
        ></elohim-epr-relationships-panel>
      </div>
    `;
  },
};

// ---------------------------------------------------------------------------
// Theme canaries
// ---------------------------------------------------------------------------

export const Dark: Story = {
  name: 'Dark',
  decorators: [
    story => html`
      <div style="background: #1a1a1a; color: #e8e8e8; color-scheme: dark; padding: 1.5rem;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-epr-relationships-panel
      .relationships=${SAMPLE_RELATIONSHIPS}
    ></elohim-epr-relationships-panel>
  `,
};

export const RTLCanary: Story = {
  name: 'RTLCanary',
  decorators: [
    story => html`
      <div dir="rtl" lang="he" style="padding: 1rem; max-inline-size: 600px;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-epr-relationships-panel
      .relationships=${SAMPLE_RELATIONSHIPS}
    ></elohim-epr-relationships-panel>
  `,
};

// ---------------------------------------------------------------------------
// Blank-slate and override-surface proofs
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial; display: block; padding: 1rem;">${story()}</div>`],
  render: () => html`
    <elohim-epr-relationships-panel
      .relationships=${SAMPLE_RELATIONSHIPS}
    ></elohim-epr-relationships-panel>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Wrapped in `style="all: initial;"`. Proves the element renders with zero external tokens.',
      },
    },
  },
};

export const CustomTheme: Story = {
  name: 'CustomTheme (override-surface proof)',
  decorators: [
    story => html`
      <div
        style="
          --elohim-epr-panel-heading-color: #c084fc;
          --elohim-epr-panel-card-bg: #1e0a3c;
          --elohim-epr-panel-card-border: 1px solid #7c3aed;
          --elohim-epr-panel-card-fg: #e9d5ff;
          --elohim-epr-panel-card-radius: 0.75rem;
          --elohim-epr-panel-gap: 0.625rem;
          background: #0d0020;
          color: #e9d5ff;
          font-family: ui-monospace, monospace;
          padding: 1.5rem;
          max-inline-size: 600px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-epr-relationships-panel
      .relationships=${SAMPLE_RELATIONSHIPS}
    ></elohim-epr-relationships-panel>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Deliberately non-Elohim purple theme via CSS custom properties. Proves the override surface is honest.',
      },
    },
  },
};
