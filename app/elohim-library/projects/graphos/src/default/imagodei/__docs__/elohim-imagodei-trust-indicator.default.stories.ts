/**
 * Library A default story — elohim-imagodei-trust-indicator
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * The trust-indicator chip signals which conductor is signing for the viewer.
 * Two operational modes: 'doorway-host' (conductor runs on a shared host) and
 * 'peer-conductor' (conductor runs on the viewer's own device). Security framing
 * per spec §0 + §1.1 — both modes carry community-attested security; the difference
 * is where conductor-compute lives, not who holds philosophical trust.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Imagodei/elohim-imagodei-trust-indicator',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-imagodei-trust-indicator>\` — small chip showing where the conductor lives.

Two operational modes:
- \`doorway-host\` — conductor runs on a shared doorway host (glyph: ⌂)
- \`peer-conductor\` — conductor runs on the viewer's own device (glyph: ◇)

Tap emits \`trust-indicator-tap\` for consumers to surface a details panel.

**Override surface:** all visual properties are bound via \`--elohim-trust-*\` CSS custom
properties with neutral CSS system color and color-mix defaults.
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
    <elohim-imagodei-trust-indicator
      trust-mode="doorway-host"
      authority-label="alpha.elohim.host"
    ></elohim-imagodei-trust-indicator>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Wrapped in `style="all: initial;"`. Proves the element renders with zero external CSS. CSS system colors and color-mix() provide legible defaults.',
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
        font-family: Courier, monospace;
        color: rebeccapurple;
        --elohim-trust-bg: papayawhip;
        --elohim-trust-host-accent: navy;
        --elohim-trust-peer-accent: maroon;
        --elohim-trust-radius: 0;
        --elohim-trust-padding-block: 0.5rem;
        --elohim-trust-padding-inline: 1rem;
        padding: 1rem;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-trust-indicator
      trust-mode="doorway-host"
      authority-label="alpha.elohim.host"
    ></elohim-imagodei-trust-indicator>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Override-surface proof: papayawhip background, navy host accent, Courier typeface — deliberately non-Elohim. Demonstrates the `--elohim-trust-*` properties are the honest override surface.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Mode stories
// ---------------------------------------------------------------------------

export const DoorwayHost: Story = {
  name: 'DoorwayHost',
  render: () => html`
    <elohim-imagodei-trust-indicator
      trust-mode="doorway-host"
      authority-label="alpha.elohim.host"
    ></elohim-imagodei-trust-indicator>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Doorway-host mode: glyph ⌂, label "Hosted via alpha.elohim.host". Default border uses host-accent (currentColor mix).',
      },
    },
  },
};

export const PeerConductor: Story = {
  name: 'PeerConductor',
  render: () => html`
    <elohim-imagodei-trust-indicator
      trust-mode="peer-conductor"
      authority-label="your conductor on this device"
    ></elohim-imagodei-trust-indicator>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Peer-conductor mode: glyph ◇, label "Your conductor — your conductor on this device". Border uses peer-accent.',
      },
    },
  },
};

export const WithFlywheelHint: Story = {
  name: 'WithFlywheelHint',
  render: () => html`
    <elohim-imagodei-trust-indicator
      trust-mode="doorway-host"
      authority-label="alpha.elohim.host"
      flywheel-hint
    ></elohim-imagodei-trust-indicator>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Flywheel hint shown (doorway-host mode only): the "(flywheel)" affordance surfaces the graduation path to a peer conductor without requiring the user to read docs.',
      },
    },
  },
};

export const DisabledOffline: Story = {
  name: 'DisabledOffline',
  render: () => html`
    <div style="opacity: 0.4; pointer-events: none;">
      <elohim-imagodei-trust-indicator
        trust-mode="peer-conductor"
        authority-label="conductor — offline"
      ></elohim-imagodei-trust-indicator>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Offline affordance: the consumer wraps the chip in a reduced-opacity, pointer-events-none container. The element itself has no internal offline state; the portal-shell drives this.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Dark theme canary
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
    <elohim-imagodei-trust-indicator
      trust-mode="doorway-host"
      authority-label="alpha.elohim.host"
      flywheel-hint
    ></elohim-imagodei-trust-indicator>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Dark theme canary: chip on dark background. CSS color-mix() defaults adapt to the dark context.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// RTL canary
// ---------------------------------------------------------------------------

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
    <elohim-imagodei-trust-indicator
      trust-mode="doorway-host"
      authority-label="alpha.elohim.host"
    ></elohim-imagodei-trust-indicator>
  `,
  parameters: {
    docs: {
      description: {
        story: 'RTL canary: he locale in an RTL container. Logical CSS properties (padding-block, padding-inline) ensure correct layout mirroring.',
      },
    },
  },
};
