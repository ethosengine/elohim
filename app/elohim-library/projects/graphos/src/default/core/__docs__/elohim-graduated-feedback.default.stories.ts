/**
 * Library A default story — elohim-graduated-feedback
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * <elohim-graduated-feedback> renders the protocol's graduated feedback widget.
 * Scale contexts: usefulness (default), accuracy, proposal, clarity, relevance.
 * Negative positions require reasoning. Aggregate distribution bar optional.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-graduated-feedback',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-graduated-feedback>\` — the protocol's graduated feedback widget.

Renders a scale of positions appropriate to the context (accuracy, usefulness, proposal, clarity, relevance).
Selecting a position shows an intensity slider (1–10) and optional reasoning field.
Negative positions (index ≤ 0.25) require reasoning before submission.
Emits \`feedback-submit\` on submission.

**Override surface:** \`--elohim-feedback-*\`.
**Parts:** \`container\`, \`label\`, \`scale\`, \`position-btn\`, \`intensity\`, \`reasoning\`, \`submit-btn\`, \`distribution\`.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Context variants
// ---------------------------------------------------------------------------

export const UsefulnessContext: Story = {
  name: 'Usefulness (default context)',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 500px;">
      <elohim-graduated-feedback context="usefulness"></elohim-graduated-feedback>
    </div>
  `,
};

export const AccuracyContext: Story = {
  name: 'Accuracy context',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 500px;">
      <elohim-graduated-feedback context="accuracy"></elohim-graduated-feedback>
    </div>
  `,
};

export const ProposalContext: Story = {
  name: 'Proposal context',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 500px;">
      <elohim-graduated-feedback context="proposal"></elohim-graduated-feedback>
    </div>
  `,
};

export const ClarityContext: Story = {
  name: 'Clarity context',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 500px;">
      <elohim-graduated-feedback context="clarity"></elohim-graduated-feedback>
    </div>
  `,
};

export const RelevanceContext: Story = {
  name: 'Relevance context',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 500px;">
      <elohim-graduated-feedback context="relevance"></elohim-graduated-feedback>
    </div>
  `,
};

// ---------------------------------------------------------------------------
// Aggregate distribution bar
// ---------------------------------------------------------------------------

export const WithAggregateDistribution: Story = {
  name: 'WithAggregateDistribution',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 500px;">
      <elohim-graduated-feedback
        context="usefulness"
        .showAggregates=${true}
        .totalResponses=${42}
        .distribution=${{
          'Not Useful': 2,
          'Slightly Useful': 8,
          'Useful': 18,
          'Very Useful': 10,
          'Transformative': 4,
        }}
      ></elohim-graduated-feedback>
    </div>
  `,
};

// ---------------------------------------------------------------------------
// Content states
// ---------------------------------------------------------------------------

export const Empty: Story = {
  name: 'Empty (nothing selected)',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 500px;">
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
        Initial state — no position selected, no intensity shown.
      </p>
      <elohim-graduated-feedback></elohim-graduated-feedback>
    </div>
  `,
};

export const Compact: Story = {
  name: 'Compact',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 320px;">
      <elohim-graduated-feedback compact></elohim-graduated-feedback>
    </div>
  `,
};

// ---------------------------------------------------------------------------
// Theme canaries
// ---------------------------------------------------------------------------

export const Dark: Story = {
  name: 'Dark',
  decorators: [
    story => html`
      <div style="background: #1a1a1a; color: #e8e8e8; color-scheme: dark; padding: 1.5rem; max-inline-size: 500px;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-graduated-feedback></elohim-graduated-feedback>`,
};

export const RTLCanary: Story = {
  name: 'RTLCanary',
  decorators: [
    story => html`
      <div dir="rtl" lang="he" style="padding: 1rem; max-inline-size: 500px;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-graduated-feedback></elohim-graduated-feedback>`,
};

// ---------------------------------------------------------------------------
// Blank-slate and override-surface proofs
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial; display: block; padding: 1rem;">${story()}</div>`],
  render: () => html`<elohim-graduated-feedback></elohim-graduated-feedback>`,
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
          --elohim-feedback-label-color: #f59e0b;
          --elohim-feedback-position-bg: #1c1917;
          --elohim-feedback-position-fg: #fde68a;
          --elohim-feedback-position-border: 1px solid #d97706;
          --elohim-feedback-position-active-bg: #92400e;
          --elohim-feedback-position-active-fg: #fffbeb;
          --elohim-feedback-accent: #f59e0b;
          --elohim-feedback-submit-bg: #b45309;
          --elohim-feedback-submit-fg: #fffbeb;
          background: #0c0a06;
          color: #fde68a;
          font-family: Georgia, serif;
          padding: 1.5rem;
          max-inline-size: 500px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-graduated-feedback context="usefulness"></elohim-graduated-feedback>`,
  parameters: {
    docs: {
      description: {
        story: 'Deliberately non-Elohim amber/sepia theme. Proves the override surface is honest.',
      },
    },
  },
};
