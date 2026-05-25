/**
 * Library B designed story — NetworkOffline
 *
 * Scene: §4.2 doorway-unreachable surface. The network connection to the doorway
 * is unavailable. The household cannot complete the portal flow.
 *
 * Design decisions:
 *   1. Trust-indicator is dimmed (opacity: 0.5) — it is still present (the household
 *      knows WHICH doorway they were trying to reach) but clearly inactive.
 *   2. Error region: "your doorway is having trouble..." — warm, not alarming.
 *   3. Retry button rendered in a disabled-during-cooldown state for the screenshot
 *      (debounce prevents hammer-clicking; the button is shown but visually muted).
 *   4. Status link for network diagnostics.
 *
 * Stillness is the Sabbath default. The offline surface does NOT have a spinner
 * or continuous animation — it holds its form and waits.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — AuthorityResolution from elohim-imagodei; authority partially resolved
 *      before connection dropped.
 *   2. Manifest vocabulary — imagodei domain; offline operational state.
 *   3. Brand tokens — inline EL_TOKENS bag (graphos design spec §14).
 *
 * Library B boundary: NEVER modifies any primitive's CSS, JSDoc, or tag name.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';

import type { AuthorityResolution } from 'elohim-imagodei';

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

// Portal-shell tokens — light
const PORTAL_TOKENS_LIGHT = `
  --elohim-portal-bg:           var(--el-cream);
  --elohim-portal-fg:           var(--el-night);
  --elohim-portal-panel-bg:     var(--el-linen);
  --elohim-portal-panel-radius: var(--el-radius-lg);
  --elohim-portal-grid-gap:     var(--el-space-md);
  --elohim-portal-padding:      var(--el-space-xl);
  --elohim-portal-footer-size:  0.8125rem;
`;

// Trust-indicator — dimmed for offline state; host-accent at reduced opacity
const TRUST_TOKENS_OFFLINE_LIGHT = `
  --elohim-trust-bg:             rgba(107, 97, 87, 0.05);
  --elohim-trust-fg:             var(--el-stone);
  --elohim-trust-host-accent:    rgba(212, 160, 62, 0.45);
  --elohim-trust-peer-accent:    rgba(123, 175, 203, 0.45);
  --elohim-trust-radius:         var(--el-radius-sm);
  --elohim-trust-padding-block:  0.25rem;
  --elohim-trust-padding-inline: 0.625rem;
`;

// Attestor-row — muted for offline state
const ATTESTOR_TOKENS_OFFLINE_LIGHT = `
  --elohim-attestor-avatar-size:     32px;
  --elohim-attestor-bg:              var(--el-stone);
  --elohim-attestor-ring:            var(--el-cream);
  --elohim-attestor-gap:             -6px;
  --elohim-attestor-overflow-opacity: 0.45;
`;

// Error region (light)
const ERROR_REGION_LIGHT = `
  background: rgba(107, 97, 87, 0.05);
  border: 1px solid rgba(107, 97, 87, 0.18);
  border-radius: 8px;
  padding: 24px;
  max-inline-size: 480px;
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

const TRUST_TOKENS_OFFLINE_DARK = `
  --elohim-trust-bg:             rgba(232, 228, 217, 0.04);
  --elohim-trust-fg:             rgba(232, 228, 217, 0.5);
  --elohim-trust-host-accent:    rgba(212, 160, 62, 0.3);
  --elohim-trust-peer-accent:    rgba(123, 175, 203, 0.3);
  --elohim-trust-radius:         var(--el-radius-sm);
  --elohim-trust-padding-block:  0.25rem;
  --elohim-trust-padding-inline: 0.625rem;
`;

const ATTESTOR_TOKENS_OFFLINE_DARK = `
  --elohim-attestor-avatar-size:     32px;
  --elohim-attestor-bg:              rgba(107, 97, 87, 0.4);
  --elohim-attestor-ring:            var(--el-night);
  --elohim-attestor-gap:             -6px;
  --elohim-attestor-overflow-opacity: 0.4;
`;

const ERROR_REGION_DARK = `
  background: rgba(232, 228, 217, 0.03);
  border: 1px solid rgba(232, 228, 217, 0.1);
  border-radius: 8px;
  padding: 24px;
  max-inline-size: 480px;
`;

// ---------------------------------------------------------------------------
// Fixtures — realistic protocol vocabulary
// ---------------------------------------------------------------------------

/**
 * Partially resolved authority — the doorway was identified but connection dropped
 * before credentials could be exchanged. The resolution is available for display
 * but the doorway is unreachable.
 */
const unreachableResolution: AuthorityResolution = {
  trustMode: 'doorway-host',
  authority: { label: 'alpha.elohim.host', id: 'alpha' },
  flywheelHint: false,
  attestors: [
    { eprRef: 'epr:attestor:susan-qahal-elder-aleph', displayName: 'Susan Miller', role: 'qahal-elder' },
    { eprRef: 'epr:attestor:james-dowell-intimate', displayName: 'James Dowell', role: 'intimate-circle' },
    { eprRef: 'epr:attestor:marta-reyes-witness', displayName: 'Marta Reyes', role: 'recovery-witness' },
  ],
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

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Imagodei/NetworkOffline',
  parameters: {
    docs: {
      description: {
        component: `
**Your doorway is having trouble.**

Spec §4.2 network-offline surface. The connection to alpha.elohim.host dropped
before the portal flow could complete. The household cannot proceed, but they
know what happened and what to do.

The trust-indicator is dimmed (opacity: 0.5 on the wrapping div) — the authority
was identified, but the relationship is currently inactive. The dimming communicates
"we know who you were trying to reach" without claiming an active authority.

The attestor-row is similarly muted — the witnesses are still named, but the
visual weight is reduced.

The retry button is rendered in a disabled-during-cooldown state for the storybook
snapshot — debounced to prevent hammer-clicking. The button shows but is visually
quieter. A status link (alpha.elohim.host/status) gives the household something
useful to check.

Stillness is the Sabbath default. No spinner. The surface holds its form and waits.

Library B — primitive untouched; brand tokens bound via story decorator.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Default — offline, light mode
// ---------------------------------------------------------------------------

export const Default: Story = {
  name: 'Default (offline)',
  decorators: [
    (story: () => unknown) => html`
      <div
        style="
          ${EL_TOKENS}
          ${PORTAL_TOKENS_LIGHT}
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
        id="offline-shell"
        authority-endpoint="/nonexistent-url-story-safe"
      >
        <!-- Trust-indicator dimmed via wrapper opacity -->
        <div slot="header" style="
          ${TRUST_TOKENS_OFFLINE_LIGHT}
          ${ATTESTOR_TOKENS_OFFLINE_LIGHT}
          opacity: 0.5;
          pointer-events: none;
          display: contents;
        ">
          <elohim-imagodei-trust-indicator
            trust-mode="doorway-host"
            authority-label="alpha.elohim.host"
          ></elohim-imagodei-trust-indicator>
        </div>

        <p slot="primary" style="display: none;"></p>

        <div slot="error-region" role="alert" style="${ERROR_REGION_LIGHT}">
          <p style="
            font-family: var(--el-font-display);
            font-size: 1.125rem;
            font-weight: 500;
            color: var(--el-stone);
            margin-block: 0 var(--el-space-sm);
            line-height: 1.35;
          ">
            Your doorway is having trouble.
          </p>

          <p style="
            font-family: var(--el-font-body);
            font-size: 0.9375rem;
            color: var(--el-stone);
            margin-block: 0 var(--el-space-md);
            line-height: 1.65;
            opacity: 0.85;
          ">
            alpha.elohim.host is not reachable right now. Your data is safe —
            this is a network connection issue, not a loss of your account.
          </p>

          <div style="
            display: flex;
            flex-direction: column;
            gap: var(--el-space-xs);
          ">
            <!-- Retry button in cooldown state — debounced, visually muted -->
            <button
              type="button"
              disabled
              aria-label="Retry connection to alpha.elohim.host (available in a few seconds)"
              style="
                font-family: var(--el-font-ui);
                font-size: 0.9375rem;
                font-weight: 600;
                color: var(--el-linen);
                background: var(--el-stone);
                border: none;
                border-radius: var(--el-radius-md);
                padding: 0.625rem 1.25rem;
                cursor: not-allowed;
                opacity: 0.55;
                inline-size: fit-content;
              "
            >Retry (cooling down…)</button>

            <a
              href="https://alpha.elohim.host/status"
              target="_blank"
              rel="noopener noreferrer"
              style="
                font-family: var(--el-font-ui);
                font-size: 0.9375rem;
                color: var(--el-stone);
                text-decoration: underline;
                text-underline-offset: 2px;
                opacity: 0.8;
              "
            >Check alpha.elohim.host status</a>
          </div>
        </div>

        <span slot="footer">
          alpha.elohim.host &middot; commons &middot;
          <a href="/identity/recovery" style="color: inherit; opacity: 0.7;">identity recovery</a>
        </span>
      </elohim-imagodei-portal-shell>
    `;
    injectAuthority('offline-shell', unreachableResolution);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Network-offline surface — §4.2. Trust-indicator wrapped in opacity: 0.5 — dimmed ' +
          'but present. The household knows "we were trying to reach alpha.elohim.host." ' +
          'The retry button is in cooldown state: disabled, visually muted (stone background ' +
          'at 0.55 opacity). No spinner. The error copy is calm: "your data is safe — this ' +
          'is a network connection issue." The status link is actionable and static.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Dark theme
// ---------------------------------------------------------------------------

export const Dark: Story = {
  name: 'Dark (constellation)',
  decorators: [
    (story: () => unknown) => html`
      <div
        style="
          ${EL_TOKENS}
          ${PORTAL_TOKENS_DARK}
          font-family: var(--el-font-ui);
          background: var(--el-night);
          min-block-size: 100vh;
          color: var(--el-starlight);
          color-scheme: dark;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => {
    const el = html`
      <elohim-imagodei-portal-shell
        id="offline-shell-dark"
        authority-endpoint="/nonexistent-url-story-safe"
      >
        <div slot="header" style="
          ${TRUST_TOKENS_OFFLINE_DARK}
          ${ATTESTOR_TOKENS_OFFLINE_DARK}
          opacity: 0.5;
          pointer-events: none;
          display: contents;
        ">
          <elohim-imagodei-trust-indicator
            trust-mode="doorway-host"
            authority-label="alpha.elohim.host"
          ></elohim-imagodei-trust-indicator>
        </div>

        <p slot="primary" style="display: none;"></p>

        <div slot="error-region" role="alert" style="${ERROR_REGION_DARK}">
          <p style="
            font-family: var(--el-font-display);
            font-size: 1.125rem;
            font-weight: 500;
            color: var(--el-starlight);
            opacity: 0.75;
            margin-block: 0 var(--el-space-sm);
            line-height: 1.35;
          ">
            Your doorway is having trouble.
          </p>

          <p style="
            font-family: var(--el-font-body);
            font-size: 0.9375rem;
            color: var(--el-starlight);
            opacity: 0.6;
            margin-block: 0 var(--el-space-md);
            line-height: 1.65;
          ">
            alpha.elohim.host is not reachable right now. Your data is safe.
          </p>

          <div style="
            display: flex;
            flex-direction: column;
            gap: var(--el-space-xs);
          ">
            <button
              type="button"
              disabled
              style="
                font-family: var(--el-font-ui);
                font-size: 0.9375rem;
                font-weight: 600;
                color: rgba(232, 228, 217, 0.4);
                background: rgba(107, 97, 87, 0.3);
                border: 1px solid rgba(107, 97, 87, 0.4);
                border-radius: var(--el-radius-md);
                padding: 0.625rem 1.25rem;
                cursor: not-allowed;
                inline-size: fit-content;
              "
            >Retry (cooling down…)</button>

            <a
              href="https://alpha.elohim.host/status"
              target="_blank"
              rel="noopener noreferrer"
              style="
                font-family: var(--el-font-ui);
                font-size: 0.9375rem;
                color: var(--el-starlight);
                opacity: 0.5;
                text-decoration: underline;
                text-underline-offset: 2px;
              "
            >Check alpha.elohim.host status</a>
          </div>
        </div>

        <span slot="footer">
          alpha.elohim.host &middot;
          <a href="/identity/recovery" style="color: inherit; opacity: 0.4;">identity recovery</a>
        </span>
      </elohim-imagodei-portal-shell>
    `;
    injectAuthority('offline-shell-dark', unreachableResolution);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Dark constellation register, offline. The opacity reduction on the trust-indicator ' +
          '(0.5) is more dramatic against Deep Sky — the header region feels genuinely quiet, ' +
          'as if the doorway has receded into the background. The error region is a barely-visible ' +
          'panel: presence without assertion. Stillness as the default state.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Retry available — after cooldown, button is active
// ---------------------------------------------------------------------------

export const RetryAvailable: Story = {
  name: 'RetryAvailable',
  decorators: [
    (story: () => unknown) => html`
      <div
        style="
          ${EL_TOKENS}
          ${PORTAL_TOKENS_LIGHT}
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
        id="offline-retry-shell"
        authority-endpoint="/nonexistent-url-story-safe"
      >
        <div slot="header" style="
          ${TRUST_TOKENS_OFFLINE_LIGHT}
          ${ATTESTOR_TOKENS_OFFLINE_LIGHT}
          opacity: 0.5;
          pointer-events: none;
          display: contents;
        ">
          <elohim-imagodei-trust-indicator
            trust-mode="doorway-host"
            authority-label="alpha.elohim.host"
          ></elohim-imagodei-trust-indicator>
        </div>

        <p slot="primary" style="display: none;"></p>

        <div slot="error-region" role="alert" style="${ERROR_REGION_LIGHT}">
          <p style="
            font-family: var(--el-font-display);
            font-size: 1.125rem;
            font-weight: 500;
            color: var(--el-stone);
            margin-block: 0 var(--el-space-sm);
            line-height: 1.35;
          ">
            Your doorway is having trouble.
          </p>

          <p style="
            font-family: var(--el-font-body);
            font-size: 0.9375rem;
            color: var(--el-stone);
            margin-block: 0 var(--el-space-md);
            line-height: 1.65;
            opacity: 0.85;
          ">
            alpha.elohim.host is not reachable right now. Your data is safe.
          </p>

          <div style="
            display: flex;
            flex-direction: column;
            gap: var(--el-space-xs);
          ">
            <!-- Retry button: cooldown cleared, ready to attempt again -->
            <button
              type="button"
              style="
                font-family: var(--el-font-ui);
                font-size: 0.9375rem;
                font-weight: 600;
                color: var(--el-linen);
                background: var(--el-green-deep);
                border: none;
                border-radius: var(--el-radius-md);
                padding: 0.625rem 1.25rem;
                cursor: pointer;
                inline-size: fit-content;
              "
            >Try again</button>

            <a
              href="https://alpha.elohim.host/status"
              target="_blank"
              rel="noopener noreferrer"
              style="
                font-family: var(--el-font-ui);
                font-size: 0.9375rem;
                color: var(--el-stone);
                text-decoration: underline;
                text-underline-offset: 2px;
                opacity: 0.8;
              "
            >Check alpha.elohim.host status</a>
          </div>
        </div>

        <span slot="footer">
          alpha.elohim.host &middot;
          <a href="/identity/recovery" style="color: inherit; opacity: 0.7;">identity recovery</a>
        </span>
      </elohim-imagodei-portal-shell>
    `;
    injectAuthority('offline-retry-shell', unreachableResolution);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Retry-available state: the debounce cooldown has cleared. The button label changes ' +
          'from "Retry (cooling down…)" to "Try again"; the button becomes active (Vineyard green, ' +
          'not the muted stone). The trust-indicator remains dimmed — the authority is still ' +
          'unresolved. One well-placed affordance, not a wall of buttons.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Peer-conductor offline — different trust mode, same offline posture
// ---------------------------------------------------------------------------

export const PeerConductorOffline: Story = {
  name: 'PeerConductorOffline',
  decorators: [
    (story: () => unknown) => html`
      <div
        style="
          ${EL_TOKENS}
          ${PORTAL_TOKENS_LIGHT}
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
    const peerOfflineResolution: AuthorityResolution = {
      trustMode: 'peer-conductor',
      authority: { label: 'your conductor on this device' },
      flywheelHint: false,
      attestors: [
        { eprRef: 'epr:attestor:susan-qahal-elder-aleph', displayName: 'Susan Miller', role: 'qahal-elder' },
        { eprRef: 'epr:attestor:james-dowell-intimate', displayName: 'James Dowell', role: 'intimate-circle' },
        { eprRef: 'epr:attestor:marta-reyes-witness', displayName: 'Marta Reyes', role: 'recovery-witness' },
      ],
    };

    const peerTrustOfflineTokens = `
      --elohim-trust-bg:             rgba(123, 175, 203, 0.04);
      --elohim-trust-fg:             var(--el-stone);
      --elohim-trust-host-accent:    rgba(212, 160, 62, 0.45);
      --elohim-trust-peer-accent:    rgba(123, 175, 203, 0.35);
      --elohim-trust-radius:         var(--el-radius-sm);
      --elohim-trust-padding-block:  0.25rem;
      --elohim-trust-padding-inline: 0.625rem;
    `;

    const el = html`
      <elohim-imagodei-portal-shell
        id="offline-peer-shell"
        authority-endpoint="/nonexistent-url-story-safe"
      >
        <div slot="header" style="
          ${peerTrustOfflineTokens}
          opacity: 0.5;
          pointer-events: none;
          display: contents;
        ">
          <elohim-imagodei-trust-indicator
            trust-mode="peer-conductor"
            authority-label="your conductor on this device"
          ></elohim-imagodei-trust-indicator>
        </div>

        <p slot="primary" style="display: none;"></p>

        <div slot="error-region" role="alert" style="${ERROR_REGION_LIGHT}">
          <p style="
            font-family: var(--el-font-display);
            font-size: 1.125rem;
            font-weight: 500;
            color: var(--el-stone);
            margin-block: 0 var(--el-space-sm);
            line-height: 1.35;
          ">
            Your conductor is not responding.
          </p>

          <p style="
            font-family: var(--el-font-body);
            font-size: 0.9375rem;
            color: var(--el-stone);
            margin-block: 0 var(--el-space-md);
            line-height: 1.65;
            opacity: 0.85;
          ">
            The conductor process on this device is unreachable. It may have
            stopped or the port may be blocked. Your data is safe.
          </p>

          <div style="
            display: flex;
            flex-direction: column;
            gap: var(--el-space-xs);
          ">
            <button
              type="button"
              disabled
              style="
                font-family: var(--el-font-ui);
                font-size: 0.9375rem;
                font-weight: 600;
                color: var(--el-linen);
                background: var(--el-stone);
                border: none;
                border-radius: var(--el-radius-md);
                padding: 0.625rem 1.25rem;
                cursor: not-allowed;
                opacity: 0.5;
                inline-size: fit-content;
              "
            >Retry (cooling down…)</button>

            <a
              href="/identity/recovery"
              style="
                font-family: var(--el-font-ui);
                font-size: 0.9375rem;
                color: var(--el-stone);
                text-decoration: underline;
                text-underline-offset: 2px;
                opacity: 0.8;
              "
            >Get help from Susan, James, or Marta</a>
          </div>
        </div>

        <span slot="footer">
          Your device &middot;
          <a href="/identity/recovery" style="color: inherit; opacity: 0.7;">identity recovery</a>
        </span>
      </elohim-imagodei-portal-shell>
    `;
    injectAuthority('offline-peer-shell', peerOfflineResolution);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Peer-conductor offline: the trust-indicator shows ◇ "your conductor on this device" ' +
          'at 0.5 opacity. The error copy changes: "the conductor process on this device is ' +
          'unreachable" — a more local explanation than the doorway-unreachable variant. ' +
          'The recovery link names the witnesses by first name: "Get help from Susan, James, ' +
          'or Marta." The same warm stillness, different infrastructure context.',
      },
    },
  },
};
