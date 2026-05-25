/**
 * Library A default story — elohim-content-analytics
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * <elohim-content-analytics> shows protocol-native attention metrics
 * (views, completions, completion rate) for a content node.
 * Service dependency is abstracted via the `loader` property.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';
import type { ContentAnalyticsLoader, ContentAnalyticsMetrics } from 'elohim-core';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const RICH_METRICS: ContentAnalyticsMetrics = {
  viewCount: 1_247,
  completionCount: 834,
  completionRate: 67,
};

const SPARSE_METRICS: ContentAnalyticsMetrics = {
  viewCount: 3,
  completionCount: 1,
  completionRate: 33,
};

function makeLoader(metrics: ContentAnalyticsMetrics | null, delayMs = 0): ContentAnalyticsLoader {
  return {
    load: () => new Promise(resolve => setTimeout(() => resolve(metrics), delayMs)),
  };
}

function makeFailingLoader(): ContentAnalyticsLoader {
  return {
    load: async () => { throw new Error('network failure'); },
  };
}

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-content-analytics',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-content-analytics>\` — protocol-native attention metrics display.

Shows view count, completion count, and completion rate for a content node.
Metrics are protocol-native economic events — not external analytics systems.

Data can be provided pre-loaded via \`.metrics\` or fetched via \`.loader\`.
The element manages loading and error states internally.

**Note:** Requires \`steward | elohim-support\` standing — not shown to general pilots.

**Override surface:** \`--elohim-analytics-*\`.
**Parts:** \`container\`, \`title\`, \`loading\`, \`grid\`, \`metric\`, \`metric-value\`, \`metric-label\`, \`note\`.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Content states
// ---------------------------------------------------------------------------

export const Loaded: Story = {
  name: 'Loaded (pre-set metrics)',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 400px;">
      <elohim-content-analytics .metrics=${RICH_METRICS}></elohim-content-analytics>
    </div>
  `,
};

export const Sparse: Story = {
  name: 'Sparse (low counts)',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 400px;">
      <elohim-content-analytics .metrics=${SPARSE_METRICS}></elohim-content-analytics>
    </div>
  `,
};

export const Loading: Story = {
  name: 'Loading',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 400px;">
      <elohim-content-analytics
        content-id="test-content-1"
        .loader=${makeLoader(RICH_METRICS, 60_000)}
      ></elohim-content-analytics>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Loading state — loader has not resolved yet (very long delay keeps it in loading state).',
      },
    },
  },
};

export const Error: Story = {
  name: 'Error (load failure)',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 400px;">
      <elohim-content-analytics
        content-id="test-content-2"
        .loader=${makeFailingLoader()}
      ></elohim-content-analytics>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Error state — loader throws, metrics marked as unavailable.',
      },
    },
  },
};

export const Empty: Story = {
  name: 'Empty (no contentId, no metrics)',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 400px;">
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
        No contentId or metrics prop set — default empty state.
      </p>
      <elohim-content-analytics></elohim-content-analytics>
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
      <div style="background: #1a1a1a; color: #e8e8e8; color-scheme: dark; padding: 1.5rem; max-inline-size: 400px;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-content-analytics .metrics=${RICH_METRICS}></elohim-content-analytics>`,
};

export const RTLCanary: Story = {
  name: 'RTLCanary',
  decorators: [
    story => html`
      <div dir="rtl" lang="he" style="padding: 1rem; max-inline-size: 400px;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-content-analytics .metrics=${RICH_METRICS}></elohim-content-analytics>`,
};

// ---------------------------------------------------------------------------
// Blank-slate and override-surface proofs
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial; display: block; padding: 1rem;">${story()}</div>`],
  render: () => html`<elohim-content-analytics .metrics=${RICH_METRICS}></elohim-content-analytics>`,
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
          --elohim-analytics-padding: 1.5rem;
          --elohim-analytics-bg: #0a1628;
          --elohim-analytics-fg: #e0f2fe;
          --elohim-analytics-title-size: 1.125rem;
          --elohim-analytics-value-size: 2rem;
          --elohim-analytics-value-weight: 800;
          --elohim-analytics-label-size: 0.7rem;
          --elohim-analytics-label-color: #93c5fd;
          --elohim-analytics-grid-gap: 1.25rem;
          --elohim-analytics-note-size: 0.7rem;
          --elohim-analytics-note-color: #60a5fa;
          font-family: ui-monospace, monospace;
          max-inline-size: 400px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-content-analytics .metrics=${RICH_METRICS}></elohim-content-analytics>`,
  parameters: {
    docs: {
      description: {
        story: 'Deliberately non-Elohim ocean-blue theme with monospace font. Proves the override surface is honest.',
      },
    },
  },
};
