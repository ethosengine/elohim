/**
 * Library B designed story — ModeB_TauriDirect
 *
 * Scene: Same household as ModeB_PeerConductorLogin — Matthew's graduated
 * conductor. But here there is no doorway in the picture. The conductor runs
 * directly on this device (Tauri desktop shell). The trust-indicator reads
 * "Your conductor on this device" — no ingress addendum because there is no
 * doorway involved.
 *
 * This composition demonstrates the "same portal, no doorway" property: the
 * visual and interaction contract is identical to the peer-conductor-via-doorway
 * scene, but the authority label communicates a different operational reality.
 * The household does not need to understand the difference technically — the
 * label surfaces it plainly.
 *
 * The attestor-row is populated identically: Susan, James, Marta. Social trust
 * is not a doorway function; it belongs to the qahal regardless of infrastructure.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — AuthorityResolution, AttestorRef from elohim-imagodei.
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

// Trust-indicator — peer-conductor direct; sky-blue, no doorway annotation
const TRUST_TOKENS_LIGHT = `
  --elohim-trust-bg:             rgba(123, 175, 203, 0.08);
  --elohim-trust-fg:             var(--el-stone);
  --elohim-trust-host-accent:    var(--el-amber);
  --elohim-trust-peer-accent:    var(--el-sky);
  --elohim-trust-radius:         var(--el-radius-sm);
  --elohim-trust-padding-block:  0.25rem;
  --elohim-trust-padding-inline: 0.625rem;
`;

// Login-card — earth tones, Vineyard green submit
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

// Attestor-row
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
 * Tauri-direct peer-conductor resolution. No doorway in the picture — the
 * conductor lives on THIS device. Flywheel hint is false (already graduated).
 * Authority label is plain: "your conductor on this device."
 */
const tauriDirectResolution: AuthorityResolution = {
  trustMode: 'peer-conductor',
  authority: { label: 'your conductor on this device' },
  flywheelHint: false,
  attestors: [susanElder, jamesCircle, martaWitness],
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
  title: 'Designed/Imagodei/ModeB_TauriDirect',
  parameters: {
    docs: {
      description: {
        component: `
**Your conductor on this device.**

Same graduated household as ModeB_PeerConductorLogin, but running in the Tauri
desktop shell. The conductor runs locally — no doorway in the network path. The
trust-indicator reads "Your conductor on this device" with no ingress addendum,
because there is no ingress helper in this deployment context.

The portal, the form, the attestor-row, and the login-card are identical to the
doorway-ingress variant. This composition proves the "same portal, no doorway"
property: the UI contract is stable across deployment contexts; only the authority
label changes to reflect the actual infrastructure reality.

Susan, James, and Marta appear in the attestor-row regardless of infrastructure.
Social trust is a qahal relationship, not a doorway feature.

Library B — primitive untouched; brand tokens bound via story decorator.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Default — Tauri direct login, peer-conductor
// ---------------------------------------------------------------------------

export const Default: Story = {
  name: 'Default (Tauri direct)',
  decorators: [lightDecorator],
  render: () => {
    const el = html`
      <elohim-imagodei-portal-shell
        id="mode-b-tauri-shell"
        authority-endpoint="/nonexistent-url-story-safe"
        step="login"
      >
        <elohim-imagodei-trust-indicator
          slot="header"
          trust-mode="peer-conductor"
          authority-label="your conductor on this device"
        ></elohim-imagodei-trust-indicator>

        <elohim-imagodei-login-card
          slot="primary"
          remembered-identifier="matthew@alpha.elohim.host"
          trust-mode="peer-conductor"
          allow-password
        >
          <div slot="unlock-prompt" style="
            padding: var(--el-space-sm);
            border: 1px solid rgba(107, 97, 87, 0.25);
            border-radius: var(--el-radius-md);
            font-size: 0.875rem;
            color: var(--el-stone);
            background: var(--el-cream);
            margin-block-end: var(--el-space-xs);
          ">
            <strong style="color: var(--el-green-deep);">Biometric unlock available.</strong>
            Use your fingerprint to unlock without a passphrase.
            <br>
            <button
              type="button"
              style="
                margin-block-start: var(--el-space-xs);
                font-family: var(--el-font-ui);
                font-size: 0.875rem;
                color: var(--el-green-deep);
                background: transparent;
                border: 1px solid var(--el-green-deep);
                border-radius: var(--el-radius-sm);
                padding: 0.25rem 0.75rem;
                cursor: pointer;
              "
            >Use biometrics</button>
          </div>
        </elohim-imagodei-login-card>

        <span slot="footer">
          Running on this device &middot;
          <a href="/identity/recovery" style="color: inherit; opacity: 0.7;">need your recovery witnesses?</a>
        </span>
      </elohim-imagodei-portal-shell>
    `;
    injectAuthority('mode-b-tauri-shell', tauriDirectResolution);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Tauri direct — the conductor is local, no doorway present. The trust-indicator ' +
          'shows ◇ "Your conductor on this device" — a clean peer-conductor claim with no ' +
          'ingress addendum. The unlock-prompt slot carries a biometric affordance from the ' +
          'Tauri shell. Same attestor-row (Susan, James, Marta) as the doorway-ingress scene ' +
          '— social trust persists across deployment contexts.',
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
        id="mode-b-tauri-shell-dark"
        authority-endpoint="/nonexistent-url-story-safe"
        step="login"
      >
        <elohim-imagodei-trust-indicator
          slot="header"
          trust-mode="peer-conductor"
          authority-label="your conductor on this device"
        ></elohim-imagodei-trust-indicator>

        <elohim-imagodei-login-card
          slot="primary"
          remembered-identifier="matthew@alpha.elohim.host"
          trust-mode="peer-conductor"
          allow-password
        ></elohim-imagodei-login-card>

        <span slot="footer">
          Running on this device &middot;
          <a href="/identity/recovery" style="color: inherit; opacity: 0.5;">need your recovery witnesses?</a>
        </span>
      </elohim-imagodei-portal-shell>
    `;
    injectAuthority('mode-b-tauri-shell-dark', tauriDirectResolution);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Dark / constellation register. The trust-indicator ◇ in sky-blue on Deep Sky ' +
          'background reads with calm authority — no urgency, no drama. The attestors\' ' +
          'avatar chips use Vineyard green (#2D5F3B) backgrounds on Deep Sky rings: ' +
          'a quiet signal that your people are here.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Side-by-side comparison: Tauri-direct vs doorway-ingress
// ---------------------------------------------------------------------------

export const SideBySideComparison: Story = {
  name: 'SideBySide (Tauri vs doorway)',
  decorators: [
    (story: () => unknown) => html`
      <div
        style="
          ${EL_TOKENS}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: var(--el-space-lg);
          display: flex;
          flex-wrap: wrap;
          gap: var(--el-space-lg);
          align-items: flex-start;
          color: var(--el-night);
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <div style="flex: 1; min-inline-size: 320px;">
      <p style="
        font-family: var(--el-font-body);
        font-size: 0.8125rem;
        color: var(--el-stone);
        margin-block: 0 var(--el-space-xs);
        letter-spacing: 0.05em;
        text-transform: uppercase;
        font-weight: 500;
      ">Tauri direct (no doorway)</p>

      <div style="
        ${PORTAL_TOKENS_LIGHT}
        ${TRUST_TOKENS_LIGHT}
        ${LOGIN_TOKENS_LIGHT}
        ${ATTESTOR_TOKENS_LIGHT}
        background: var(--el-linen);
        border-radius: var(--el-radius-lg);
        padding: var(--el-space-lg);
        box-shadow: var(--el-shadow-soft);
      ">
        <elohim-imagodei-trust-indicator
          trust-mode="peer-conductor"
          authority-label="your conductor on this device"
        ></elohim-imagodei-trust-indicator>
        <div style="margin-block-start: var(--el-space-sm); font-size: 0.9rem; color: var(--el-stone);">
          No ingress addendum — conductor is purely local.
        </div>
      </div>
    </div>

    <div style="flex: 1; min-inline-size: 320px;">
      <p style="
        font-family: var(--el-font-body);
        font-size: 0.8125rem;
        color: var(--el-stone);
        margin-block: 0 var(--el-space-xs);
        letter-spacing: 0.05em;
        text-transform: uppercase;
        font-weight: 500;
      ">Via doorway ingress</p>

      <div style="
        ${PORTAL_TOKENS_LIGHT}
        ${TRUST_TOKENS_LIGHT}
        ${LOGIN_TOKENS_LIGHT}
        ${ATTESTOR_TOKENS_LIGHT}
        background: var(--el-linen);
        border-radius: var(--el-radius-lg);
        padding: var(--el-space-lg);
        box-shadow: var(--el-shadow-soft);
      ">
        <elohim-imagodei-trust-indicator
          trust-mode="peer-conductor"
          authority-label="your conductor — alpha.elohim.host is helping with ingress"
        ></elohim-imagodei-trust-indicator>
        <div style="margin-block-start: var(--el-space-sm); font-size: 0.9rem; color: var(--el-stone);">
          Ingress addendum names alpha.elohim.host's limited role.
        </div>
      </div>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Trust-indicator comparison: same peer-conductor mode, two different deployment ' +
          'contexts. The ◇ glyph is identical; the authority label is the only difference. ' +
          '"Same portal, no doorway" — the household experiences the same credential surface ' +
          'in both contexts. The label communicates operational reality without requiring the ' +
          'household to understand infrastructure.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// RTL canary
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
        id="mode-b-tauri-shell-he"
        authority-endpoint="/nonexistent-url-story-safe"
        step="login"
      >
        <elohim-imagodei-trust-indicator
          slot="header"
          trust-mode="peer-conductor"
          authority-label="your conductor on this device"
        ></elohim-imagodei-trust-indicator>

        <elohim-imagodei-login-card
          slot="primary"
          remembered-identifier="matthew@alpha.elohim.host"
          trust-mode="peer-conductor"
          allow-password
        ></elohim-imagodei-login-card>

        <span slot="footer">Running on this device &middot; commons</span>
      </elohim-imagodei-portal-shell>
    `;
    injectAuthority('mode-b-tauri-shell-he', tauriDirectResolution);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary — he-IL locale, Tauri-direct context. The authority label "your conductor ' +
          'on this device" should render right-to-left in the trust-indicator chip. The attestor-row ' +
          'mirrors; the login-card input label reads RTL. No ingress addendum to complicate the ' +
          'label reversal.',
      },
    },
  },
};
