/**
 * Library A default story — elohim-imagodei-attestor-row
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * The attestor-row makes the community-attestation security model VISIBLE — the
 * user's qahal-elders, intimate-circle, and recovery-witnesses are shown as an
 * avatar chip row. "Your people have your back." Per spec §0 + §1.1 + §2.3.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';
import type { AttestorRef } from 'elohim-imagodei';

// ---------------------------------------------------------------------------
// Fixtures — realistic protocol vocabulary
// ---------------------------------------------------------------------------

const susanElder: AttestorRef = {
  eprRef: 'epr:attestor:susan-qahal-elder',
  displayName: 'Susan Miller',
  role: 'qahal-elder',
};

const jamesCircle: AttestorRef = {
  eprRef: 'epr:attestor:james-intimate',
  displayName: 'James Dowell',
  role: 'intimate-circle',
};

const martaWitness: AttestorRef = {
  eprRef: 'epr:attestor:marta-witness',
  displayName: 'Marta Reyes',
  role: 'recovery-witness',
};

const elohimAgent: AttestorRef = {
  eprRef: 'epr:attestor:elohim-agent-household',
  displayName: 'Household Elohim',
  role: 'elohim-agent',
};

const extra1: AttestorRef = {
  eprRef: 'epr:attestor:david-circle',
  displayName: 'David Kim',
  role: 'intimate-circle',
};

const extra2: AttestorRef = {
  eprRef: 'epr:attestor:priya-witness',
  displayName: 'Priya Nair',
  role: 'recovery-witness',
};

const sixAttestors: AttestorRef[] = [susanElder, jamesCircle, martaWitness, elohimAgent, extra1, extra2];

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Imagodei/elohim-imagodei-attestor-row',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-imagodei-attestor-row>\` — avatar chip row of qahal-elders, intimate-circle, and recovery-witnesses.

Foregrounds the social-trust model: your community attests you in both trust modes.
Overflow after \`maxVisible\` (default 5) shows +N. Empty state uses the named \`empty\` slot.

Tap on an avatar emits \`attestor-tap\` with \`{ eprRef }\` for consumers to surface a detail panel.

**Override surface:** all visual properties bound via \`--elohim-attestor-*\` CSS custom properties
with neutral CSS system color defaults (\`Canvas\`, \`CanvasText\`).
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Blank-slate proof
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial;">${story()}</div>`],
  render: () => html`
    <elohim-imagodei-attestor-row
      .attestors=${[susanElder, jamesCircle, martaWitness]}
    ></elohim-imagodei-attestor-row>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Wrapped in `style="all: initial;"`. Proves the element renders with zero external CSS — canvas/system colors provide legible avatar backgrounds.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Override-surface proof
// ---------------------------------------------------------------------------

export const CustomTheme: Story = {
  name: 'CustomTheme (override-surface proof)',
  decorators: [
    story => html`
      <div style="
        font-family: 'Georgia', serif;
        color: #2c1a0e;
        --elohim-attestor-avatar-size: 36px;
        --elohim-attestor-bg: #f5deb3;
        --elohim-attestor-ring: #8b4513;
        --elohim-attestor-gap: 0.5rem;
        --elohim-attestor-overflow-opacity: 0.9;
        padding: 1rem;
        background: #fdf6e3;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-attestor-row
      .attestors=${sixAttestors}
    ></elohim-imagodei-attestor-row>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Override-surface proof: warm wheat palette with saddle-brown ring — deliberately non-Elohim. Demonstrates `--elohim-attestor-*` properties are the honest override surface.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// State stories per spec §5.2
// ---------------------------------------------------------------------------

export const OneAttestor: Story = {
  name: 'OneAttestor',
  render: () => html`
    <elohim-imagodei-attestor-row
      .attestors=${[susanElder]}
    ></elohim-imagodei-attestor-row>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Single attestor: Susan Miller (qahal-elder). The row renders a single avatar with no overflow.',
      },
    },
  },
};

export const FivePlusOverflow: Story = {
  name: 'FivePlusOverflow',
  render: () => html`
    <elohim-imagodei-attestor-row
      .attestors=${sixAttestors}
      max-visible="5"
    ></elohim-imagodei-attestor-row>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Six attestors with maxVisible=5: first five avatars shown, "+1" overflow label renders for the sixth.',
      },
    },
  },
};

export const EmptyState: Story = {
  name: 'EmptyState',
  render: () => html`
    <elohim-imagodei-attestor-row .attestors=${[]}>
      <span slot="empty">No witnesses recorded yet — invite your community to attest you.</span>
    </elohim-imagodei-attestor-row>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Empty attestors array: the `empty` slot content renders. Default slot text is "No witnesses recorded yet."',
      },
    },
  },
};

export const RTLLayout: Story = {
  name: 'RTLLayout',
  decorators: [
    story => html`
      <div dir="rtl" lang="he" style="padding: 1rem;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-attestor-row
      .attestors=${[susanElder, jamesCircle, martaWitness]}
    ></elohim-imagodei-attestor-row>
  `,
  parameters: {
    docs: {
      description: {
        story: 'RTL canary: he locale in an RTL container. The flex row (logical properties) mirrors correctly.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Compact density
// ---------------------------------------------------------------------------

export const CompactDensity: Story = {
  name: 'CompactDensity',
  render: () => html`
    <elohim-imagodei-attestor-row
      .attestors=${[susanElder, jamesCircle, martaWitness, elohimAgent]}
      density="compact"
    ></elohim-imagodei-attestor-row>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Compact density variant: tighter gap between avatars for inline / header contexts.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Dark canary
// ---------------------------------------------------------------------------

export const Dark: Story = {
  name: 'Dark',
  decorators: [
    story => html`
      <div style="background: #111; padding: 1.5rem; color-scheme: dark; color: #f0f0f0;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-attestor-row
      .attestors=${[susanElder, jamesCircle, martaWitness]}
    ></elohim-imagodei-attestor-row>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Dark theme canary: avatar rings use Canvas system color, which adapts to dark color-scheme.',
      },
    },
  },
};
