/**
 * Library A default story — elohim-reaction-bar
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * <elohim-reaction-bar> renders the protocol's emotional reaction surface.
 * Default reactions: moved, grateful, inspired, hopeful, grieving, challenged.
 * Mediated reactions require constitutional reasoning before proceeding.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';
import type { ReactionConstraints } from 'elohim-core';

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-reaction-bar',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-reaction-bar>\` — the protocol's emotional reaction surface.

Renders reaction buttons (moved, grateful, inspired, hopeful, grieving, challenged by default).
Clicking a reaction emits \`reaction-submit\`. Clicking an already-submitted reaction emits \`reaction-remove\`.
Mediated reactions open a constitutional-reasoning dialog before proceeding.

**Override surface:** \`--elohim-reaction-*\`.
**Parts:** \`bar\`, \`reaction-btn\`, \`count\`, \`label\`, \`mediation-dialog\`.
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
  name: 'Default (all permitted reactions)',
  render: () => html`
    <div style="padding: 1rem;">
      <elohim-reaction-bar entity-type="content" entity-id="test-content-1"></elohim-reaction-bar>
    </div>
  `,
};

export const WithCounts: Story = {
  name: 'WithCounts',
  render: () => html`
    <div style="padding: 1rem;">
      <elohim-reaction-bar
        entity-type="content"
        entity-id="test-content-2"
        .counts=${[
          { type: 'moved', count: 12 },
          { type: 'grateful', count: 5 },
          { type: 'inspired', count: 23 },
        ]}
        .showCounts=${true}
      ></elohim-reaction-bar>
    </div>
  `,
};

export const WithUserReactions: Story = {
  name: 'WithUserReactions (aria-pressed)',
  render: () => html`
    <div style="padding: 1rem;">
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
        "moved" and "inspired" are user-submitted (aria-pressed=true).
      </p>
      <elohim-reaction-bar
        .userReactions=${['moved', 'inspired']}
        .showCounts=${true}
        .counts=${[
          { type: 'moved', count: 3 },
          { type: 'inspired', count: 7 },
        ]}
      ></elohim-reaction-bar>
    </div>
  `,
};

export const Compact: Story = {
  name: 'Compact (no labels)',
  render: () => html`
    <div style="padding: 1rem;">
      <elohim-reaction-bar compact .showCounts=${true} .counts=${[{ type: 'moved', count: 8 }]}></elohim-reaction-bar>
    </div>
  `,
};

export const CustomConstraints: Story = {
  name: 'CustomConstraints (subset)',
  render: () => {
    const constraints: ReactionConstraints = {
      permittedTypes: ['grateful', 'inspired'],
    };
    return html`
      <div style="padding: 1rem;">
        <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
          Only "grateful" and "inspired" permitted.
        </p>
        <elohim-reaction-bar .constraints=${constraints}></elohim-reaction-bar>
      </div>
    `;
  },
};

export const MediatedReaction: Story = {
  name: 'MediatedReaction (uncomfortable)',
  render: () => {
    const constraints: ReactionConstraints = {
      permittedTypes: ['moved', 'uncomfortable'],
      mediatedTypes: [
        {
          type: 'uncomfortable',
          constitutionalReasoning:
            'Expressing discomfort shapes the governance of this content. Proceed only if you can speak to why this content creates discomfort for you or others.',
          proceedBehavior: { visibleToOthers: true },
        },
      ],
    };
    return html`
      <div style="padding: 1rem; max-inline-size: 500px;">
        <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
          Click "uncomfortable" to see the constitutional reasoning dialog.
        </p>
        <elohim-reaction-bar .constraints=${constraints}></elohim-reaction-bar>
      </div>
    `;
  },
};

// ---------------------------------------------------------------------------
// Content states
// ---------------------------------------------------------------------------

export const Empty: Story = {
  name: 'Empty (no reactions permitted)',
  render: () => html`
    <div style="padding: 1rem;">
      <elohim-reaction-bar .constraints=${{ permittedTypes: [] }}></elohim-reaction-bar>
    </div>
  `,
  parameters: {
    docs: {
      description: { story: 'No permitted types — bar renders empty.' },
    },
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
    <elohim-reaction-bar
      .showCounts=${true}
      .counts=${[{ type: 'moved', count: 5 }, { type: 'grateful', count: 2 }]}
    ></elohim-reaction-bar>
  `,
};

export const RTLCanary: Story = {
  name: 'RTLCanary',
  decorators: [
    story => html`
      <div dir="rtl" lang="he" style="padding: 1rem;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-reaction-bar .showCounts=${true} .counts=${[{ type: 'moved', count: 5 }]}></elohim-reaction-bar>
  `,
};

// ---------------------------------------------------------------------------
// Blank-slate and override-surface proofs
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial; display: block; padding: 1rem;">${story()}</div>`],
  render: () => html`<elohim-reaction-bar></elohim-reaction-bar>`,
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
          --elohim-reaction-btn-bg: #1a3a1a;
          --elohim-reaction-btn-fg: #86efac;
          --elohim-reaction-btn-border: 1px solid #22c55e;
          --elohim-reaction-btn-active-bg: #14532d;
          --elohim-reaction-btn-radius: 0;
          background: #0a1a0a;
          color: #86efac;
          font-family: ui-monospace, monospace;
          padding: 1.5rem;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-reaction-bar
      .showCounts=${true}
      .counts=${[{ type: 'moved', count: 8 }, { type: 'inspired', count: 3 }]}
    ></elohim-reaction-bar>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Deliberately non-Elohim terminal-green theme. Proves the override surface is honest.',
      },
    },
  },
};
