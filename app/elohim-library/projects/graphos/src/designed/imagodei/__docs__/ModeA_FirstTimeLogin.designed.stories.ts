/**
 * Library B designed story — ModeA_FirstTimeLogin
 *
 * Scene: A first-time visitor arrives at alpha.elohim.host. They have no
 * remembered identifier, no attestors yet. The doorway-host trust mode is
 * active; the flywheel hint surfaces the graduation path to a peer conductor.
 *
 * Composition: portal-shell (doorway-host, flywheel-hint) + trust-indicator
 * (header slot) + federated-resolver (primary slot). The resolver step is the
 * entry to the protocol — the household begins here.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — AttestorRef, AuthorityResolution, FederatedResolveOutcome from
 *      elohim-imagodei (typed output of spec §5.1).
 *   2. Manifest vocabulary — imagodei domain; returnTo is the fair-exchange
 *      lamad concept path.
 *   3. Brand tokens — inline EL_TOKENS bag (graphos design spec §14) until the
 *      global stylesheet ships; bound at story-decorator level only.
 *
 * Library B boundary: NEVER modifies any primitive's CSS, JSDoc, or tag name.
 * All brand binding happens via CSS custom properties in story decorators.
 *
 * Brand voice: "household" not "user"; "welcome" not "onboarding"; "your
 * neighbors" not "the network"; "commons" not "platform".
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';

import type {
  AuthorityResolution,
  FederatedResolveOutcome,
  ResolveIdentifierFn,
} from 'elohim-imagodei';

// ---------------------------------------------------------------------------
// Brand token declaration — graphos design spec §14
// Inline because no global stylesheet seeds them into storybook today.
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

// Portal-shell binding — light mode (Linen surface, Hearthstone text)
const PORTAL_TOKENS_LIGHT = `
  --elohim-portal-bg:           var(--el-cream);
  --elohim-portal-fg:           var(--el-night);
  --elohim-portal-panel-bg:     var(--el-linen);
  --elohim-portal-panel-radius: var(--el-radius-lg);
  --elohim-portal-grid-gap:     var(--el-space-md);
  --elohim-portal-padding:      var(--el-space-xl);
  --elohim-portal-footer-size:  0.8125rem;
`;

// Trust-indicator binding — host accent is Harvest Gold (terracotta warmth)
const TRUST_TOKENS_LIGHT = `
  --elohim-trust-bg:           transparent;
  --elohim-trust-fg:           var(--el-stone);
  --elohim-trust-host-accent:  var(--el-amber);
  --elohim-trust-peer-accent:  var(--el-sky);
  --elohim-trust-radius:       var(--el-radius-sm);
  --elohim-trust-padding-block:  0.25rem;
  --elohim-trust-padding-inline: 0.625rem;
`;

// Federated-resolver binding
const RESOLVER_TOKENS_LIGHT = `
  --elohim-resolver-bg:         transparent;
  --elohim-resolver-fg:         var(--el-night);
  --elohim-resolver-gap:        var(--el-space-sm);
  --elohim-resolver-label-size: 0.9375rem;
  --elohim-input-border:        1px solid rgba(107, 97, 87, 0.35);
  --elohim-input-focus-ring:    var(--el-amber);
  --elohim-input-error-fg:      var(--el-clay);
`;

// Attestor-row — empty state for first-time visitor
const ATTESTOR_TOKENS_LIGHT = `
  --elohim-attestor-avatar-size:    32px;
  --elohim-attestor-bg:             var(--el-linen);
  --elohim-attestor-ring:           var(--el-cream);
  --elohim-attestor-gap:            -6px;
  --elohim-attestor-overflow-opacity: 0.75;
`;

// Dark-mode token bag — constellation register
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
  --elohim-trust-bg:           rgba(232, 228, 217, 0.07);
  --elohim-trust-fg:           var(--el-starlight);
  --elohim-trust-host-accent:  var(--el-amber);
  --elohim-trust-peer-accent:  var(--el-sky);
  --elohim-trust-radius:       var(--el-radius-sm);
  --elohim-trust-padding-block:  0.25rem;
  --elohim-trust-padding-inline: 0.625rem;
`;

const RESOLVER_TOKENS_DARK = `
  --elohim-resolver-bg:         transparent;
  --elohim-resolver-fg:         var(--el-starlight);
  --elohim-resolver-gap:        var(--el-space-sm);
  --elohim-resolver-label-size: 0.9375rem;
  --elohim-input-border:        1px solid rgba(232, 228, 217, 0.2);
  --elohim-input-focus-ring:    var(--el-amber);
  --elohim-input-error-fg:      var(--el-clay);
`;

// ---------------------------------------------------------------------------
// Fixtures — realistic protocol vocabulary
// ---------------------------------------------------------------------------

/**
 * Doorway-host authority resolution for alpha.elohim.host.
 * Flywheel hint true — first-time visitor, so attestors are empty.
 */
const firstVisitResolution: AuthorityResolution = {
  trustMode: 'doorway-host',
  authority: { label: 'alpha.elohim.host', id: 'alpha' },
  flywheelHint: true,
  attestors: [],
};

/**
 * Success resolver — discovers alpha.elohim.host for matthew's identifier.
 * The returnTo path is the fair-exchange lamad concept.
 */
const alphaResolver: ResolveIdentifierFn = async (identifier: string): Promise<FederatedResolveOutcome> => {
  await new Promise(r => setTimeout(r, 500));
  return {
    ok: true,
    doorwayUrl: `https://alpha.elohim.host/doorway?for=${encodeURIComponent(identifier)}&returnTo=%2Flamad%2Fconcept%2Ffair-exchange`,
  };
};

// ---------------------------------------------------------------------------
// Decorator factories
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
        ${RESOLVER_TOKENS_LIGHT}
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
        ${RESOLVER_TOKENS_DARK}
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
  title: 'Designed/Imagodei/ModeA_FirstTimeLogin',
  parameters: {
    docs: {
      description: {
        component: `
**Welcome to the commons.**

A first-time visitor arrives at alpha.elohim.host — Mode A (doorway-host). No remembered
identifier, no attestors yet. The flywheel hint surfaces the graduation path to a peer
conductor for households that want to graduate to full protocol participation.

The federated-resolver is the entry step: the household identifies themselves with a
protocol identifier (e.g. \`matthew@alpha.elohim.host\`). Alpha.elohim.host discovers
the doorway endpoint, then advances to the login step.

The returnTo destination is \`/lamad/concept/fair-exchange\` — the household came from
a learning path and will be returned there after sign-in.

**Trust-indicator:** "Hosted via alpha.elohim.host (flywheel)" — the ⌂ glyph in Harvest
Gold signals doorway-host mode. The "(flywheel)" affordance names the graduation path
without requiring the household to read documentation.

**Brand binding:** Linen surface (#FAF6EE), Hearthstone body (#6B6157), Harvest Gold
trust accent (#D4A03E). Fraunces for display text; DM Sans for UI. Generous 48px margins,
8px base rhythm. Warm shadow instead of hard borders.

Library B — primitive untouched; brand tokens bound via story decorator.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Default — first-time visit, resolve step
// ---------------------------------------------------------------------------

export const Default: Story = {
  name: 'Default (first-time welcome)',
  decorators: [lightDecorator],
  render: () => {
    const el = html`
      <elohim-imagodei-portal-shell
        id="mode-a-shell"
        authority-endpoint="/nonexistent-url-story-safe"
        step="resolve"
        flywheel-hint
      >
        <elohim-imagodei-trust-indicator
          slot="header"
          trust-mode="doorway-host"
          authority-label="alpha.elohim.host"
          flywheel-hint
        ></elohim-imagodei-trust-indicator>

        <elohim-imagodei-federated-resolver
          slot="primary"
          remember-key="elohim_auth_identifier"
          placeholder="matthew@alpha.elohim.host"
          .resolveIdentifier=${alphaResolver}
        >
          <span slot="help-text">
            Enter your identifier — for example, matthew&#64;alpha.elohim.host.
            Your community steward can help if you are unsure.
          </span>
        </elohim-imagodei-federated-resolver>

        <span slot="footer">
          alpha.elohim.host &middot; commons stewardship &middot;
          <a href="/identity/help" style="color: inherit; opacity: 0.7;">need help?</a>
        </span>
      </elohim-imagodei-portal-shell>
    `;
    injectAuthority('mode-a-shell', firstVisitResolution);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story:
          'First-time welcome — Mode A entry point. The portal-shell carries the flywheel-hint ' +
          'attribute; the trust-indicator shows ⌂ alpha.elohim.host. The attestor-row is empty ' +
          '(no witnesses yet — enrollment completes after first sign-in). The resolver is the ' +
          'first and only active step. Brand bound: Linen surface, Harvest Gold trust accent, ' +
          'generous padding. Fraunces would render the display headline if added by the consumer.',
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
        id="mode-a-shell-dark"
        authority-endpoint="/nonexistent-url-story-safe"
        step="resolve"
        flywheel-hint
      >
        <elohim-imagodei-trust-indicator
          slot="header"
          trust-mode="doorway-host"
          authority-label="alpha.elohim.host"
          flywheel-hint
        ></elohim-imagodei-trust-indicator>

        <elohim-imagodei-federated-resolver
          slot="primary"
          remember-key="elohim_auth_identifier"
          placeholder="matthew@alpha.elohim.host"
          .resolveIdentifier=${alphaResolver}
        >
          <span slot="help-text">
            Enter your identifier — for example, matthew&#64;alpha.elohim.host.
            Your community steward can help if you are unsure.
          </span>
        </elohim-imagodei-federated-resolver>

        <span slot="footer">
          alpha.elohim.host &middot; commons stewardship &middot;
          <a href="/identity/help" style="color: inherit; opacity: 0.7;">need help?</a>
        </span>
      </elohim-imagodei-portal-shell>
    `;
    injectAuthority('mode-a-shell-dark', firstVisitResolution);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Dark / constellation register. Deep Sky background (#0F1A12 — carries the green ' +
          'undertone of growing things, not the cold black of outer space). Starlight text ' +
          '(#E8E4D9 — warm, not pure white). Harvest Gold trust-indicator accent reads the ' +
          'same in both registers: a warm horizon line that says "this host is accountable." ' +
          'No pure-black background; no pure-white text.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// RTL canary — Hebrew locale
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
          ${RESOLVER_TOKENS_LIGHT}
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
        id="mode-a-shell-he"
        authority-endpoint="/nonexistent-url-story-safe"
        step="resolve"
        flywheel-hint
      >
        <elohim-imagodei-trust-indicator
          slot="header"
          trust-mode="doorway-host"
          authority-label="alpha.elohim.host"
          flywheel-hint
        ></elohim-imagodei-trust-indicator>

        <elohim-imagodei-federated-resolver
          slot="primary"
          remember-key="elohim_auth_identifier"
          .resolveIdentifier=${alphaResolver}
        ></elohim-imagodei-federated-resolver>

        <span slot="footer">alpha.elohim.host &middot; commons</span>
      </elohim-imagodei-portal-shell>
    `;
    injectAuthority('mode-a-shell-he', firstVisitResolution);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary — he-IL locale in a dir="rtl" container. Logical CSS properties ' +
          '(padding-inline, padding-block, margin-inline) in the primitives mirror correctly. ' +
          'The trust-indicator chip should sit at inline-start in RTL. The resolver label ' +
          'and input read right-to-left. The flywheel hint should trail on the correct side.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Resolve-error state — unknown host
// ---------------------------------------------------------------------------

export const UnknownHost: Story = {
  name: 'UnknownHost (resolve error)',
  decorators: [lightDecorator],
  render: () => {
    const unknownHostResolver: ResolveIdentifierFn = async (): Promise<FederatedResolveOutcome> => {
      await new Promise(r => setTimeout(r, 400));
      return {
        ok: false,
        reason: 'no doorway found for this identifier — check the address and try again, or ask your community steward',
      };
    };

    const el = html`
      <elohim-imagodei-portal-shell
        id="mode-a-shell-err"
        authority-endpoint="/nonexistent-url-story-safe"
        step="resolve"
        flywheel-hint
      >
        <elohim-imagodei-trust-indicator
          slot="header"
          trust-mode="doorway-host"
          authority-label="alpha.elohim.host"
          flywheel-hint
        ></elohim-imagodei-trust-indicator>

        <elohim-imagodei-federated-resolver
          slot="primary"
          remember-key="elohim_auth_identifier"
          placeholder="matthew@unknown.host"
          .resolveIdentifier=${unknownHostResolver}
        >
          <span slot="help-text">
            Enter your identifier — for example, matthew&#64;alpha.elohim.host.
          </span>
        </elohim-imagodei-federated-resolver>

        <span slot="footer">alpha.elohim.host &middot; commons</span>
      </elohim-imagodei-portal-shell>
    `;
    injectAuthority('mode-a-shell-err', firstVisitResolution);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Resolve-error state — the doorway cannot find the identifier. ' +
          'Enter an identifier and submit; the resolver returns an error. ' +
          'The error part uses the warm-clay token (#B8664F) — earthy caution, ' +
          'never aggressive red. The protocol has no "danger red." The household ' +
          'is directed to their community steward for help.',
      },
    },
  },
};
