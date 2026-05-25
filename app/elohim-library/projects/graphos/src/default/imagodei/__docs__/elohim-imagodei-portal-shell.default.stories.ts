/**
 * Library A default story — elohim-imagodei-portal-shell
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * The portal-shell is the wizard's outer wrapper for the peer OAuth portal. It
 * discovers trustMode on mount via authorityEndpoint, propagates it to slotted
 * children, and renders persistent chrome (trust-indicator + attestor-row by
 * default). The `primary` slot receives the active step element.
 *
 * IMPORTANT: Stories set `authority-endpoint` to a non-existent path so the
 * element never fires a real HTTP request. The _setAuthority test seam is used
 * via a ref callback to inject authority state directly where needed.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';
import type { AuthorityResolution } from 'elohim-imagodei';

// ---------------------------------------------------------------------------
// Authority fixture
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

Discovers \`trustMode\` on mount via \`authorityEndpoint\` (default \`/auth/me\`); propagates it
to slotted children; renders persistent chrome (trust-indicator + attestor-row in the \`header\`
slot). The active step element slots via \`primary\`.

**Override surface:** bound via \`--elohim-portal-*\` CSS custom properties with CSS system color
defaults (\`Canvas\`, \`CanvasText\`).
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
      authority-endpoint="/nonexistent-url-story-safe"
    >
      <p slot="primary">Primary slot content.</p>
      <p slot="footer">Protocol · Privacy · Help</p>
    </elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Wrapped in `style="all: initial;"`. Shell renders with Canvas/CanvasText defaults — no brand tokens.',
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
      authority-endpoint="/nonexistent-url-story-safe"
    >
      <p slot="primary" style="margin:0;">Primary step content in custom theme.</p>
      <p slot="footer" style="margin:0;">Custom theme footer · Help</p>
    </elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Override-surface proof: GitHub-dark palette via `--elohim-portal-*` properties. Demonstrates the CSS override surface is honest.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// State stories per spec §5.2
// ---------------------------------------------------------------------------

export const EmptyShell: Story = {
  name: 'EmptyShell',
  render: () => html`
    <elohim-imagodei-portal-shell
      authority-endpoint="/nonexistent-url-story-safe"
    ></elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Empty shell with no slotted children and no authority resolved. Shows default chrome with empty trust-indicator and attestor-row placeholder.',
      },
    },
  },
};

export const WithLoginCard: Story = {
  name: 'WithLoginCard',
  render: () => html`
    <elohim-imagodei-portal-shell
      authority-endpoint="/nonexistent-url-story-safe"
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
        story: 'Login step: `elohim-imagodei-login-card` slotted into the `primary` slot. The shell provides outer chrome; the card provides the credential form.',
      },
    },
  },
};

export const WithConsentCard: Story = {
  name: 'WithConsentCard',
  render: () => html`
    <elohim-imagodei-portal-shell
      authority-endpoint="/nonexistent-url-story-safe"
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
        story: 'Consent step: `elohim-imagodei-consent-card` slotted into `primary`. Graphos Designer (the lamad pattern library) requesting displayName (required) + qahal.standing (optional).',
      },
    },
  },
};

export const ErrorBoundary: Story = {
  name: 'ErrorBoundary',
  render: () => html`
    <elohim-imagodei-portal-shell
      authority-endpoint="/nonexistent-url-story-safe"
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
// Trust-mode variants
// ---------------------------------------------------------------------------

export const DoorwayHostAuthority: Story = {
  name: 'DoorwayHostAuthority',
  render: () => {
    const el = html`
      <elohim-imagodei-portal-shell
        authority-endpoint="/nonexistent-url-story-safe"
        flywheel-hint
        id="shell-doorway"
      >
        <p slot="primary">Shell with doorway-host authority injected via _setAuthority.</p>
        <span slot="footer">Hosted via alpha.elohim.host · Flywheel available</span>
      </elohim-imagodei-portal-shell>
    `;
    // Inject authority after render via macrotask to use the test seam.
    setTimeout(() => {
      const shell = document.getElementById('shell-doorway');
      if (shell && '_setAuthority' in shell) {
        (shell as unknown as { _setAuthority: (r: AuthorityResolution) => void })._setAuthority(doorwayResolution);
      }
    }, 0);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story: 'Doorway-host authority injected via the `_setAuthority` test seam. Trust-indicator shows ⌂ alpha.elohim.host with flywheel hint. Three attestors shown.',
      },
    },
  },
};

export const PeerConductorAuthority: Story = {
  name: 'PeerConductorAuthority',
  render: () => {
    const el = html`
      <elohim-imagodei-portal-shell
        authority-endpoint="/nonexistent-url-story-safe"
        id="shell-peer"
      >
        <p slot="primary">Shell with peer-conductor authority injected via _setAuthority.</p>
        <span slot="footer">Your conductor on this device · Help</span>
      </elohim-imagodei-portal-shell>
    `;
    setTimeout(() => {
      const shell = document.getElementById('shell-peer');
      if (shell && '_setAuthority' in shell) {
        (shell as unknown as { _setAuthority: (r: AuthorityResolution) => void })._setAuthority(peerResolution);
      }
    }, 0);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story: 'Peer-conductor authority injected via `_setAuthority`. Trust-indicator shows ◇ "your conductor on this device". One attestor shown.',
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
      authority-endpoint="/nonexistent-url-story-safe"
    >
      <p slot="primary">Primary step content — dark theme.</p>
      <span slot="footer">Protocol · Privacy</span>
    </elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Dark theme canary: Canvas/CanvasText system colors adapt; panel-bg color-mix adjusts.',
      },
    },
  },
};
