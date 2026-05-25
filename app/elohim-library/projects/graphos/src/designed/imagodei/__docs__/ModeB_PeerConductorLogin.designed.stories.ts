/**
 * Library B designed story — ModeB_PeerConductorLogin
 *
 * Scene: Matthew has graduated from doorway-host to peer-conductor. Alpha.elohim.host
 * is still present as the network ingress point — it routes requests to his conductor
 * — but his keys live on his own device. The trust-indicator reflects this: peer-
 * conductor mode, with the addendum "alpha.elohim.host is helping with ingress."
 *
 * The attestor-row is populated with his community witnesses: Susan (qahal-elder),
 * James (intimate-circle), and Marta (recovery-witness). These neighbors have his
 * back — the social trust model is visible, not buried in settings.
 *
 * The login-card renders the peer-conductor copy: "Unlock your conductor."
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — AuthorityResolution, AttestorRef, OAuthProviderRef from elohim-imagodei.
 *   2. Manifest vocabulary — imagodei domain; peer-conductor trust mode.
 *   3. Brand tokens — inline EL_TOKENS bag (graphos design spec §14).
 *
 * Library B boundary: NEVER modifies any primitive's CSS, JSDoc, or tag name.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';

import type {
  AuthorityResolution,
  AttestorRef,
  OAuthProviderRef,
} from 'elohim-imagodei';

// ---------------------------------------------------------------------------
// Brand token declaration — graphos design spec §14
// ---------------------------------------------------------------------------

const EL_TOKENS = `
  --el-green-deep:  #2D5F3B;
  --el-green-light: #7FB069;
  --el-amber:       #D4A03E;
  --el-clay:        #B8664F;
  --el-cream:       #F5F0E8;
  --el-linen:       #FAF6EE;
  --el-stone:       #6B6157;
  --el-sky:         #7BAFCB;
  --el-plum:        #6E4B6B;
  --el-starlight:   #E8E4D9;
  --el-night:       #0F1A12;
  --el-font-display: 'Fraunces', Georgia, serif;
  --el-font-body:    'Source Serif 4', Georgia, serif;
  --el-font-ui:      'DM Sans', system-ui, sans-serif;
  --el-font-mono:    'JetBrains Mono', monospace;
  --el-space-xs:  8px;
  --el-space-sm:  16px;
  --el-space-md:  24px;
  --el-space-lg:  32px;
  --el-space-xl:  48px;
  --el-radius-sm: 4px;
  --el-radius-md: 8px;
  --el-radius-lg: 16px;
  --el-shadow-soft:   0 2px 8px rgba(107, 97, 87, 0.08);
  --el-shadow-medium: 0 4px 16px rgba(107, 97, 87, 0.12);
`;

// Portal-shell — light mode
const PORTAL_TOKENS_LIGHT = `
  --elohim-portal-bg:           var(--el-cream);
  --elohim-portal-fg:           var(--el-night);
  --elohim-portal-panel-bg:     var(--el-linen);
  --elohim-portal-panel-radius: var(--el-radius-lg);
  --elohim-portal-grid-gap:     var(--el-space-md);
  --elohim-portal-padding:      var(--el-space-xl);
  --elohim-portal-footer-size:  0.8125rem;
`;

// Trust-indicator — peer-conductor; sky-blue accent for peer mode
const TRUST_TOKENS_LIGHT = `
  --elohim-trust-bg:             rgba(123, 175, 203, 0.08);
  --elohim-trust-fg:             var(--el-stone);
  --elohim-trust-host-accent:    var(--el-amber);
  --elohim-trust-peer-accent:    var(--el-sky);
  --elohim-trust-radius:         var(--el-radius-sm);
  --elohim-trust-padding-block:  0.25rem;
  --elohim-trust-padding-inline: 0.625rem;
`;

// Login-card — warm earth forms; terracotta submit button
const LOGIN_TOKENS_LIGHT = `
  --elohim-login-bg:             transparent;
  --elohim-login-fg:             var(--el-night);
  --elohim-login-input-bg:       var(--el-cream);
  --elohim-login-input-border:   1px solid rgba(107, 97, 87, 0.35);
  --elohim-login-button-bg:      var(--el-green-deep);
  --elohim-login-button-fg:      var(--el-linen);
  --elohim-login-oauth-border:   1px solid rgba(107, 97, 87, 0.2);
  --elohim-login-gap:            var(--el-space-sm);
  --elohim-login-radius:         var(--el-radius-md);
  --elohim-login-error-fg:       var(--el-clay);
`;

// Attestor-row — warm ring, overlapping chips
const ATTESTOR_TOKENS_LIGHT = `
  --elohim-attestor-avatar-size:     32px;
  --elohim-attestor-bg:              var(--el-green-light);
  --elohim-attestor-ring:            var(--el-cream);
  --elohim-attestor-gap:             -6px;
  --elohim-attestor-overflow-opacity: 0.75;
`;

// Dark mode
const PORTAL_TOKENS_DARK = `
  --elohim-portal-bg:           var(--el-night);
  --elohim-portal-fg:           var(--el-starlight);
  --elohim-portal-panel-bg:     #162019;
  --elohim-portal-panel-radius: var(--el-radius-lg);
  --elohim-portal-grid-gap:     var(--el-space-md);
  --elohim-portal-padding:      var(--el-space-xl);
  --elohim-portal-footer-size:  0.8125rem;
`;

const TRUST_TOKENS_DARK = `
  --elohim-trust-bg:             rgba(123, 175, 203, 0.1);
  --elohim-trust-fg:             var(--el-starlight);
  --elohim-trust-host-accent:    var(--el-amber);
  --elohim-trust-peer-accent:    var(--el-sky);
  --elohim-trust-radius:         var(--el-radius-sm);
  --elohim-trust-padding-block:  0.25rem;
  --elohim-trust-padding-inline: 0.625rem;
`;

const LOGIN_TOKENS_DARK = `
  --elohim-login-bg:             transparent;
  --elohim-login-fg:             var(--el-starlight);
  --elohim-login-input-bg:       rgba(232, 228, 217, 0.05);
  --elohim-login-input-border:   1px solid rgba(232, 228, 217, 0.18);
  --elohim-login-button-bg:      var(--el-green-deep);
  --elohim-login-button-fg:      var(--el-starlight);
  --elohim-login-oauth-border:   1px solid rgba(232, 228, 217, 0.12);
  --elohim-login-gap:            var(--el-space-sm);
  --elohim-login-radius:         var(--el-radius-md);
  --elohim-login-error-fg:       var(--el-clay);
`;

const ATTESTOR_TOKENS_DARK = `
  --elohim-attestor-avatar-size:     32px;
  --elohim-attestor-bg:              var(--el-green-deep);
  --elohim-attestor-ring:            var(--el-night);
  --elohim-attestor-gap:             -6px;
  --elohim-attestor-overflow-opacity: 0.75;
`;

// ---------------------------------------------------------------------------
// Fixtures — realistic protocol vocabulary
// ---------------------------------------------------------------------------

/**
 * Matthew's aleph-household witnesses. These three neighbors carry his community
 * attestation — they are the social substrate of his security model. Not abstract
 * "contacts" or "friends": each has a named role in the protocol covenant.
 */
const susanElder: AttestorRef = {
  eprRef: 'epr:attestor:susan-qahal-elder-aleph',
  displayName: 'Susan Miller',
  role: 'qahal-elder',
};

const jamesCircle: AttestorRef = {
  eprRef: 'epr:attestor:james-dowell-intimate',
  displayName: 'James Dowell',
  role: 'intimate-circle',
};

const martaWitness: AttestorRef = {
  eprRef: 'epr:attestor:marta-reyes-witness',
  displayName: 'Marta Reyes',
  role: 'recovery-witness',
};

/**
 * Peer-conductor authority resolution. Alpha.elohim.host acts as network ingress
 * but Matthew's keys are on his own device. Flywheel hint is false — he has
 * already graduated.
 */
const peerConductorResolution: AuthorityResolution = {
  trustMode: 'peer-conductor',
  authority: { label: 'your conductor — alpha.elohim.host is helping with ingress' },
  flywheelHint: false,
  attestors: [susanElder, jamesCircle, martaWitness],
};

const atprotoProvider: OAuthProviderRef = {
  id: 'atproto',
  displayName: 'AT Protocol (Bluesky)',
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function injectAuthority(shellId: string, resolution: AuthorityResolution) {
  setTimeout(() => {
    const shell = document.getElementById(shellId);
    if (shell && '_setAuthority' in shell) {
      (shell as unknown as { _setAuthority: (r: AuthorityResolution) => void })._setAuthority(resolution);
    }
  }, 0);
}

function lightDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${PORTAL_TOKENS_LIGHT}
        ${TRUST_TOKENS_LIGHT}
        ${LOGIN_TOKENS_LIGHT}
        ${ATTESTOR_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        min-block-size: 100vh;
        color: var(--el-night);
      "
    >
      ${story()}
    </div>
  `;
}

function darkDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${PORTAL_TOKENS_DARK}
        ${TRUST_TOKENS_DARK}
        ${LOGIN_TOKENS_DARK}
        ${ATTESTOR_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        min-block-size: 100vh;
        color: var(--el-starlight);
        color-scheme: dark;
      "
    >
      ${story()}
    </div>
  `;
}

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Imagodei/ModeB_PeerConductorLogin',
  parameters: {
    docs: {
      description: {
        component: `
**Unlock your conductor.**

Matthew has graduated from doorway-host to peer-conductor. His keys live on his
own device; alpha.elohim.host routes network ingress but does not hold his credentials.

The trust-indicator carries the peer-conductor glyph (◇) in sky-blue — a cooler accent
than the Harvest Gold of doorway-host mode, signaling a different relationship to the
commons infrastructure. The addendum "alpha.elohim.host is helping with ingress" names
the doorway's limited but honest role.

The attestor-row is populated: Susan (qahal-elder), James (intimate-circle), Marta
(recovery-witness). Three people from Matthew's aleph-household qahal who can vouch
for him. Social trust is visible at the point of entry — not buried in a "security"
settings tab.

The login-card uses peer-conductor copy: "Unlock your conductor."

Library B — primitive untouched; brand tokens bound via story decorator.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Default — graduated household, login step
// ---------------------------------------------------------------------------

export const Default: Story = {
  name: 'Default (graduated peer)',
  decorators: [lightDecorator],
  render: () => {
    const el = html`
      <elohim-imagodei-portal-shell
        id="mode-b-peer-shell"
        authority-endpoint="/nonexistent-url-story-safe"
        step="login"
      >
        <elohim-imagodei-trust-indicator
          slot="header"
          trust-mode="peer-conductor"
          authority-label="your conductor — alpha.elohim.host is helping with ingress"
        ></elohim-imagodei-trust-indicator>

        <elohim-imagodei-login-card
          slot="primary"
          remembered-identifier="matthew@alpha.elohim.host"
          trust-mode="peer-conductor"
          allow-password
          .oauthProviders=${[atprotoProvider]}
        ></elohim-imagodei-login-card>

        <span slot="footer">
          Unlocking your conductor at alpha.elohim.host &middot;
          <a href="/identity/recovery" style="color: inherit; opacity: 0.7;">need your recovery witnesses?</a>
        </span>
      </elohim-imagodei-portal-shell>
    `;
    injectAuthority('mode-b-peer-shell', peerConductorResolution);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Graduated household — Mode B, peer-conductor via alpha.elohim.host ingress. ' +
          'The ◇ glyph in sky-blue (#7BAFCB) is the peer-conductor signal. Susan, James, ' +
          'and Marta appear in the attestor-row — their presence communicates "your neighbors ' +
          'are part of how you recover if something goes wrong," not as a technical footnote ' +
          'but as visible social infrastructure. Login-card copy: "Unlock your conductor."',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Dark theme
// ---------------------------------------------------------------------------

export const Dark: Story = {
  name: 'Dark (constellation)',
  decorators: [darkDecorator],
  render: () => {
    const el = html`
      <elohim-imagodei-portal-shell
        id="mode-b-peer-shell-dark"
        authority-endpoint="/nonexistent-url-story-safe"
        step="login"
      >
        <elohim-imagodei-trust-indicator
          slot="header"
          trust-mode="peer-conductor"
          authority-label="your conductor — alpha.elohim.host is helping with ingress"
        ></elohim-imagodei-trust-indicator>

        <elohim-imagodei-login-card
          slot="primary"
          remembered-identifier="matthew@alpha.elohim.host"
          trust-mode="peer-conductor"
          allow-password
          .oauthProviders=${[atprotoProvider]}
        ></elohim-imagodei-login-card>

        <span slot="footer">
          Unlocking your conductor at alpha.elohim.host &middot;
          <a href="/identity/recovery" style="color: inherit; opacity: 0.7;">need your recovery witnesses?</a>
        </span>
      </elohim-imagodei-portal-shell>
    `;
    injectAuthority('mode-b-peer-shell-dark', peerConductorResolution);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Dark / constellation register. The sky-blue peer-conductor accent (#7BAFCB) in dark ' +
          'mode reads as starlight seen through high atmosphere — the same warmth as the light ' +
          'theme but pitched against the Deep Sky background. Vineyard green for the submit ' +
          'button anchors the earth-tone register even in dark mode.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// With-error state (credential failure)
// ---------------------------------------------------------------------------

export const WithCredentialError: Story = {
  name: 'WithCredentialError',
  decorators: [lightDecorator],
  render: () => {
    const el = html`
      <elohim-imagodei-portal-shell
        id="mode-b-peer-shell-err"
        authority-endpoint="/nonexistent-url-story-safe"
        step="login"
      >
        <elohim-imagodei-trust-indicator
          slot="header"
          trust-mode="peer-conductor"
          authority-label="your conductor — alpha.elohim.host is helping with ingress"
        ></elohim-imagodei-trust-indicator>

        <elohim-imagodei-login-card
          id="mode-b-login-card-err"
          slot="primary"
          remembered-identifier="matthew@alpha.elohim.host"
          trust-mode="peer-conductor"
          allow-password
        ></elohim-imagodei-login-card>

        <span slot="footer">
          Unlocking your conductor at alpha.elohim.host &middot;
          <a href="/identity/recovery" style="color: inherit; opacity: 0.7;">need your recovery witnesses?</a>
        </span>
      </elohim-imagodei-portal-shell>
    `;
    injectAuthority('mode-b-peer-shell-err', peerConductorResolution);
    setTimeout(() => {
      const card = document.getElementById('mode-b-login-card-err');
      if (card && 'setError' in card) {
        (card as unknown as { setError: (msg: string) => void }).setError(
          'conductor key did not match — check your passphrase and try again, or ask Susan, James, or Marta for recovery help'
        );
      }
    }, 50);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Credential error state: the conductor rejected the passphrase. The error message ' +
          'names the recovery witnesses by first name — "Susan, James, or Marta" — because ' +
          'the protocol knows who they are. This is the "grandma standard" for recovery: ' +
          '"log in with help from your people." Clay (#B8664F) signals caution without aggression.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// RTL canary — Hebrew
// ---------------------------------------------------------------------------

export const Hebrew: Story = {
  name: 'Hebrew (RTL canary)',
  decorators: [
    (story: () => unknown) => html`
      <div
        dir="rtl"
        lang="he"
        style="
          ${EL_TOKENS}
          ${PORTAL_TOKENS_LIGHT}
          ${TRUST_TOKENS_LIGHT}
          ${LOGIN_TOKENS_LIGHT}
          ${ATTESTOR_TOKENS_LIGHT}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          min-block-size: 100vh;
          color: var(--el-night);
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => {
    const el = html`
      <elohim-imagodei-portal-shell
        id="mode-b-peer-shell-he"
        authority-endpoint="/nonexistent-url-story-safe"
        step="login"
      >
        <elohim-imagodei-trust-indicator
          slot="header"
          trust-mode="peer-conductor"
          authority-label="your conductor — alpha.elohim.host is helping with ingress"
        ></elohim-imagodei-trust-indicator>

        <elohim-imagodei-login-card
          slot="primary"
          remembered-identifier="matthew@alpha.elohim.host"
          trust-mode="peer-conductor"
          allow-password
        ></elohim-imagodei-login-card>

        <span slot="footer">alpha.elohim.host &middot; commons</span>
      </elohim-imagodei-portal-shell>
    `;
    injectAuthority('mode-b-peer-shell-he', peerConductorResolution);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary — he-IL locale. The attestor-row mirrors: the first chip (Susan, qahal-elder) ' +
          'appears at inline-start for RTL. The trust-indicator ◇ glyph is at inline-start of the chip. ' +
          'The login-card input label reads right-to-left. Logical properties carry the layout mirror.',
      },
    },
  },
};
