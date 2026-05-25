/**
 * Library A default story — elohim-epr-popover
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * <elohim-epr-popover> renders a positioned three-pillar metadata popover.
 * Shows lamad (title, type, format, tags, description), shefa (steward count),
 * and qahal (reach, layer) from an EprHead.
 *
 * Slice 2.6 canary addition.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';
import type { EprHead } from 'elohim-core';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const RICH_HEAD: EprHead = {
  version: 1,
  id: 'epr:shefa-stewardship-101',
  content: 'bafybeigdyrztm7vexample0000000000000000000000000000000000000000',
  lamad: {
    title: 'Stewardship 101 — Foundations',
    contentType: 'lesson',
    contentFormat: 'sophia-quiz-json',
    description:
      'A foundational lesson on economic stewardship in the Elohim Protocol. Covers REA commitments, provisioning flows, and household budgeting.',
    tags: ['shefa', 'stewardship', 'rea', 'foundations'],
  },
  shefa: {
    stewards: ['agent-alice-hash', 'agent-bob-hash'],
  },
  qahal: {
    reach: 'household',
    layer: 'constitutional',
  },
  relationships: [],
};

const MINIMAL_HEAD: EprHead = {
  version: 1,
  id: 'epr:lamad-intro',
  content: 'bafybeigdyrztm7vexample0000000000000000000000000000000000000001',
  lamad: {
    title: 'Introduction to Learning',
    contentType: 'path',
  },
  shefa: {},
  qahal: {},
  relationships: [],
};

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-epr-popover',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-epr-popover>\` — three-pillar metadata popover for EPR anchors.

Displays EPR Head metadata in a positioned popover:
- Lamad: content type badge, format, title, description, tags
- Shefa: steward count
- Qahal: reach badge, constitutional layer badge
- Footer: "Open resource" button (when \`route\` is set)

Controlled via \`visible\` (reflects as attribute) and \`position: { top, left }\`.
Emits \`epr-popover-enter\`, \`epr-popover-dismiss\`, \`epr-popover-navigate\`.

**Override surface:** \`--elohim-epr-popover-*\`, \`--elohim-epr-type-*\`, \`--elohim-epr-tag-*\`, \`--elohim-epr-reach-*\`, \`--elohim-epr-layer-*\`.
**Parts:** \`popover\`, \`header\`, \`type-badge\`, \`format\`, \`title\`, \`description\`, \`tags\`, \`tag\`, \`shefa-section\`, \`qahal-section\`, \`reach-badge\`, \`layer-badge\`, \`link\`.
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

export const Rich: Story = {
  name: 'Rich (all pillar sections)',
  render: () => html`
    <div style="padding: 2rem; position: relative; inline-size: 340px;">
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 1rem;">
        All three pillar sections — lamad (full), shefa (2 stewards), qahal (reach + layer).
      </p>
      <elohim-epr-popover
        .head=${RICH_HEAD}
        .visible=${true}
        .position=${{ top: 80, left: 20 }}
        route="/lamad/stewardship-101"
        style="position: relative; top: auto; left: auto;"
      ></elohim-epr-popover>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Statically positioned for the story (uses `position: relative` override). In production, `position: fixed` is used with `top`/`left` set via the `.position` prop.',
      },
    },
  },
};

export const Minimal: Story = {
  name: 'Minimal (lamad-only)',
  render: () => html`
    <div style="padding: 2rem; position: relative; inline-size: 340px;">
      <elohim-epr-popover
        .head=${MINIMAL_HEAD}
        .visible=${true}
        style="position: relative; top: auto; left: auto;"
      ></elohim-epr-popover>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'No shefa stewards, no qahal reach/layer, no route — only the lamad section.',
      },
    },
  },
};

export const Empty: Story = {
  name: 'Empty (not visible)',
  render: () => html`
    <div style="padding: 1rem;">
      <p style="font-size: 0.875rem; opacity: 0.8;">
        <code>visible=false</code> — popover renders nothing (display: none).
      </p>
      <elohim-epr-popover .head=${RICH_HEAD} .visible=${false}></elohim-epr-popover>
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-start: 0.5rem;">
        (The element is in the DOM but invisible — correct blank-slate behavior.)
      </p>
    </div>
  `,
};

export const WithLink: Story = {
  name: 'WithLink ("Open resource" button)',
  render: () => {
    const onNavigate = (e: Event) =>
      // eslint-disable-next-line no-console
      console.log('epr-popover-navigate:', (e as CustomEvent<{ route: string }>).detail);
    return html`
      <div style="padding: 2rem; position: relative; inline-size: 340px;">
        <elohim-epr-popover
          .head=${RICH_HEAD}
          .visible=${true}
          route="/lamad/stewardship-101"
          style="position: relative; top: auto; left: auto;"
          @epr-popover-navigate=${onNavigate}
        ></elohim-epr-popover>
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
      <div style="background: #1a1a1a; color: #e8e8e8; color-scheme: dark; padding: 2rem; inline-size: 360px;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-epr-popover
      .head=${RICH_HEAD}
      .visible=${true}
      route="/lamad/stewardship-101"
      style="position: relative; top: auto; left: auto;"
    ></elohim-epr-popover>
  `,
};

export const RTLCanary: Story = {
  name: 'RTLCanary',
  decorators: [
    story => html`
      <div dir="rtl" lang="he" style="padding: 2rem; inline-size: 360px;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-epr-popover
      .head=${{
        ...RICH_HEAD,
        lamad: { ...RICH_HEAD.lamad, title: 'מבוא לניהול משק בית', description: 'שיעור יסודי בניהול כלכלי.' },
      }}
      .visible=${true}
      style="position: relative; top: auto; left: auto;"
    ></elohim-epr-popover>
  `,
};

// ---------------------------------------------------------------------------
// Blank-slate and override-surface proofs
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial; display: block; padding: 2rem; inline-size: 360px;">${story()}</div>`],
  render: () => html`
    <elohim-epr-popover
      .head=${RICH_HEAD}
      .visible=${true}
      style="position: relative; top: auto; left: auto;"
    ></elohim-epr-popover>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Wrapped in `style="all: initial;"`. Proves the popover renders with zero external tokens.',
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
          --elohim-epr-popover-width: 300px;
          --elohim-epr-popover-bg: #1a0a2e;
          --elohim-epr-popover-fg: #e9d5ff;
          --elohim-epr-popover-border: 1px solid #7c3aed;
          --elohim-epr-popover-radius: 0.75rem;
          --elohim-epr-popover-shadow: 0 8px 32px rgba(124, 58, 237, 0.4);
          --elohim-epr-type-bg: #4c1d95;
          --elohim-epr-type-fg: #ddd6fe;
          --elohim-epr-tag-bg: #3b0764;
          --elohim-epr-tag-fg: #f5d0fe;
          --elohim-epr-reach-bg: #4c1d95;
          --elohim-epr-reach-fg: #ddd6fe;
          --elohim-epr-layer-bg: #4c1d95;
          --elohim-epr-layer-fg: #ddd6fe;
          --elohim-epr-link-color: #c084fc;
          --elohim-epr-divider-color: #5b21b6;
          background: #0d0020;
          padding: 2rem;
          inline-size: 360px;
          font-family: Georgia, serif;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-epr-popover
      .head=${RICH_HEAD}
      .visible=${true}
      route="/lamad/stewardship-101"
      style="position: relative; top: auto; left: auto;"
    ></elohim-epr-popover>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Deliberately non-Elohim violet/purple theme with serif font. Proves the override surface is honest.',
      },
    },
  },
};
