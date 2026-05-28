/**
 * Library A default story — elohim-imagodei-portal-shell
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * The portal-shell is the wizard's outer wrapper for the peer OAuth portal.
 * Authority is host-provided — the element does NOT fetch. Hosts pre-fetch
 * `/auth/me` (the doorway endpoint) and bind the result to the `authority`
 * property. Stories bind `authority` directly as a fixture to demonstrate
 * the host-pre-fetch contract and each rendering state.
 *
 * IMPORTANT: No story may issue a real HTTP request. Authority is always
 * provided via the `.authority` property binding.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';
import type { AuthorityResolution } from 'elohim-imagodei';

// ---------------------------------------------------------------------------
// Authority fixtures — realistic protocol vocabulary, no invented identity
// ---------------------------------------------------------------------------

const doorwayResolution: AuthorityResolution = {
  trustMode: 'doorway-host',
  authority: { label: 'alpha.elohim.host', id: 'alpha' },
  flywheelHint: true,
  attestors: [
    { eprRef: 'epr:attestor:susan-elder', displayName: 'Susan Miller', role: 'qahal-elder' },
    { eprRef: 'epr:attestor:james-circle', displayName: 'James Dowell', role: 'intimate-circle' },
    { eprRef: 'epr:attestor:marta-witness', displayName: 'Marta Reyes', role: 'recovery-witness' },
  ],
};

const peerResolution: AuthorityResolution = {
  trustMode: 'peer-conductor',
  authority: { label: 'your conductor on this device' },
  flywheelHint: false,
  attestors: [
    { eprRef: 'epr:attestor:james-circle', displayName: 'James Dowell', role: 'intimate-circle' },
  ],
};

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Imagodei/elohim-imagodei-portal-shell',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-imagodei-portal-shell>\` — wizard outer wrapper for the peer OAuth portal.

**Authority is host-provided.** The element does NOT fetch. Hosts pre-fetch \`/auth/me\`
(the doorway endpoint) and bind the result to the \`authority\` property. When \`authority\`
is null, the element emits \`authority-needed\` giving the host a signal to fetch and bind.

Renders persistent chrome (trust-indicator + attestor-row in the \`header\` slot).
The active step element slots via \`primary\`.

**Override surface:** bound via \`--elohim-portal-*\` CSS custom properties with CSS system
color defaults (\`Canvas\`, \`CanvasText\`).
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
  decorators: [story => html`<div style="all: initial; display: block; height: 100vh;">${story()}</div>`],
  render: () => html`
    <elohim-imagodei-portal-shell
      .authority=${doorwayResolution}
    >
      <p slot="primary">Primary slot content.</p>
      <p slot="footer">Protocol · Privacy · Help</p>
    </elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Wrapped in `style="all: initial;"`. Shell renders with Canvas/CanvasText defaults — no brand tokens. Authority provided directly via `.authority` property (host-pre-fetch pattern).',
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
        font-family: ui-monospace, 'Courier New', monospace;
        --elohim-portal-bg: #0d1117;
        --elohim-portal-fg: #c9d1d9;
        --elohim-portal-panel-bg: #161b22;
        --elohim-portal-panel-radius: 0;
        --elohim-portal-grid-gap: 1.5rem;
        --elohim-portal-padding: 2rem;
        --elohim-portal-footer-size: 0.7rem;
        display: block;
        height: 100vh;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-portal-shell
      .authority=${peerResolution}
    >
      <p slot="primary" style="margin:0;">Primary step content in custom theme.</p>
      <p slot="footer" style="margin:0;">Custom theme footer · Help</p>
    </elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Override-surface proof: GitHub-dark palette via `--elohim-portal-*` properties. Demonstrates the CSS override surface is honest. Authority (peer-conductor) bound via `.authority` property.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// State stories per spec §5.2
// ---------------------------------------------------------------------------

export const EmptyShell: Story = {
  name: 'EmptyShell (authority: null — loading state)',
  render: () => html`
    <elohim-imagodei-portal-shell></elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Shell with `authority` null (not yet provided). Renders placeholder chrome and emits `authority-needed`. In real use the host responds to that event and binds authority.',
      },
    },
  },
};

export const WithLoginCard: Story = {
  name: 'WithLoginCard',
  render: () => html`
    <elohim-imagodei-portal-shell
      .authority=${doorwayResolution}
      step="login"
    >
      <elohim-imagodei-login-card
        slot="primary"
        remembered-identifier="matthew@alpha.elohim.host"
        allow-password
      ></elohim-imagodei-login-card>
      <span slot="footer">Sign in to continue · Help</span>
    </elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Login step: `elohim-imagodei-login-card` slotted into the `primary` slot. Authority pre-provided by host (doorway-host mode with flywheel hint). The shell provides outer chrome; the card provides the credential form.',
      },
    },
  },
};

export const WithConsentCard: Story = {
  name: 'WithConsentCard',
  render: () => html`
    <elohim-imagodei-portal-shell
      .authority=${doorwayResolution}
      step="consent"
    >
      <elohim-imagodei-consent-card
        slot="primary"
        .requestingClient=${{ id: 'graphos-designer', displayName: 'Graphos Designer' }}
        .requestedClaims=${[
          { id: 'imagodei.displayName', label: 'Display name', description: 'Your public-facing name' },
          { id: 'qahal.standing', label: 'Community standing', description: 'Your attested standing in your qahal' },
        ]}
        .requiredClaims=${['imagodei.displayName']}
      ></elohim-imagodei-consent-card>
      <span slot="footer">Review and approve the access request · Help</span>
    </elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Consent step: `elohim-imagodei-consent-card` slotted into `primary`. Graphos Designer (the lamad pattern library) requesting displayName (required) + qahal.standing (optional). Authority pre-provided by host.',
      },
    },
  },
};

export const ErrorBoundary: Story = {
  name: 'ErrorBoundary',
  render: () => html`
    <elohim-imagodei-portal-shell
      .authority=${doorwayResolution}
    >
      <p slot="primary">Login step content.</p>
      <div slot="error-region" role="alert">
        <strong>Authority discovery failed.</strong>
        The portal could not contact the doorway endpoint. Check your connection and try again.
      </div>
      <span slot="footer">Help · Protocol</span>
    </elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Error boundary: content provided in the `error-region` slot is rendered below the primary panel. The consumer drives error surfacing; the shell provides the layout slot.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Trust-mode variants — authority bound directly via property
// ---------------------------------------------------------------------------

export const DoorwayHostAuthority: Story = {
  name: 'DoorwayHostAuthority',
  render: () => html`
    <elohim-imagodei-portal-shell
      .authority=${doorwayResolution}
    >
      <p slot="primary">Shell with doorway-host authority bound via .authority property.</p>
      <span slot="footer">Hosted via alpha.elohim.host · Flywheel available</span>
    </elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Doorway-host authority bound directly via `.authority` property — the correct host-pre-fetch pattern. Trust-indicator shows doorway-host mode with flywheel hint. Three attestors shown.',
      },
    },
  },
};

export const PeerConductorAuthority: Story = {
  name: 'PeerConductorAuthority',
  render: () => html`
    <elohim-imagodei-portal-shell
      .authority=${peerResolution}
    >
      <p slot="primary">Shell with peer-conductor authority bound via .authority property.</p>
      <span slot="footer">Your conductor on this device · Help</span>
    </elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Peer-conductor authority bound directly via `.authority` property. Trust-indicator shows peer-conductor mode. One attestor shown.',
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
      <div style="background: #111; color-scheme: dark; height: 100vh;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-portal-shell
      .authority=${doorwayResolution}
    >
      <p slot="primary">Primary step content — dark theme.</p>
      <span slot="footer">Protocol · Privacy</span>
    </elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Dark theme canary: Canvas/CanvasText system colors adapt; panel-bg color-mix adjusts. Authority pre-provided.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// RTL canary (he locale)
// ---------------------------------------------------------------------------

export const RTL: Story = {
  name: 'RTL (he locale canary)',
  decorators: [
    story => html`
      <div dir="rtl" lang="he" style="height: 100vh;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-portal-shell
      .authority=${doorwayResolution}
    >
      <p slot="primary">תוכן שלב ראשי — בדיקת RTL.</p>
      <span slot="footer">פרוטוקול · פרטיות · עזרה</span>
    </elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story: 'RTL locale canary (he). Shell layout uses logical properties; the frame centres correctly in right-to-left document flow.',
      },
    },
  },
};
