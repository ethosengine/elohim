/**
 * Library B designed story — ConsentCardThreeClaims
 *
 * Scene: Graphos Designer (graphos-designer.elohim.host) is requesting 3 claims
 * from Matthew as part of an OAuth authorization flow. One required, two optional.
 * The consent-card renders in the portal-shell, which already established Matthew's
 * trust mode (doorway-host via alpha.elohim.host in this case).
 *
 * Claims:
 *   - imagodei.displayName (required) — locked toggle, "(required)" badge
 *   - qahal.standing (optional) — can be toggled off
 *   - lamad.mastery (optional) — can be toggled off
 *
 * The RP brand strip carries a brand mark for graphos-designer. The consent-card
 * uses a brand-tinted background (warm cream-gold) for the RP strip to give it
 * visual weight without decoration for its own sake.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — ClaimRef, RequestingClient from elohim-imagodei.
 *   2. Manifest vocabulary — imagodei and qahal domains; lamad.mastery is a
 *      lamad-domain claim.
 *   3. Brand tokens — inline EL_TOKENS bag (graphos design spec §14).
 *
 * Library B boundary: NEVER modifies any primitive's CSS, JSDoc, or tag name.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';

import type {
  ClaimRef,
  RequestingClient,
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

// Trust-indicator — doorway-host context for this consent scene
const TRUST_TOKENS_LIGHT = `
  --elohim-trust-bg:             transparent;
  --elohim-trust-fg:             var(--el-stone);
  --elohim-trust-host-accent:    var(--el-amber);
  --elohim-trust-peer-accent:    var(--el-sky);
  --elohim-trust-radius:         var(--el-radius-sm);
  --elohim-trust-padding-block:  0.25rem;
  --elohim-trust-padding-inline: 0.625rem;
`;

// Consent-card — warm-tinted RP strip background; Vineyard approve button
const CONSENT_TOKENS_LIGHT = `
  --elohim-consent-rp-bg:              #FCF0D8;
  --elohim-consent-rp-fg:              var(--el-night);
  --elohim-consent-toggle-accent:     var(--el-green-deep);
  --elohim-consent-claim-row-bg:       var(--el-cream);
  --elohim-consent-claim-row-border:   1px solid rgba(107, 97, 87, 0.18);
  --elohim-consent-approve-bg:         var(--el-green-deep);
  --elohim-consent-approve-fg:         var(--el-linen);
  --elohim-consent-decline-bg:         transparent;
  --elohim-consent-decline-fg:         var(--el-stone);
  --elohim-consent-gap:                var(--el-space-sm);
  --elohim-consent-radius:             var(--el-radius-md);
  --elohim-consent-muted-opacity:      0.65;
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
  --elohim-trust-bg:             rgba(232, 228, 217, 0.07);
  --elohim-trust-fg:             var(--el-starlight);
  --elohim-trust-host-accent:    var(--el-amber);
  --elohim-trust-peer-accent:    var(--el-sky);
  --elohim-trust-radius:         var(--el-radius-sm);
  --elohim-trust-padding-block:  0.25rem;
  --elohim-trust-padding-inline: 0.625rem;
`;

const CONSENT_TOKENS_DARK = `
  --elohim-consent-rp-bg:              rgba(212, 160, 62, 0.1);
  --elohim-consent-rp-fg:              var(--el-starlight);
  --elohim-consent-toggle-accent:     var(--el-green-deep);
  --elohim-consent-claim-row-bg:       rgba(232, 228, 217, 0.04);
  --elohim-consent-claim-row-border:   1px solid rgba(232, 228, 217, 0.12);
  --elohim-consent-approve-bg:         var(--el-green-deep);
  --elohim-consent-approve-fg:         var(--el-starlight);
  --elohim-consent-decline-bg:         transparent;
  --elohim-consent-decline-fg:         var(--el-starlight);
  --elohim-consent-gap:                var(--el-space-sm);
  --elohim-consent-radius:             var(--el-radius-md);
  --elohim-consent-muted-opacity:      0.5;
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
 * Graphos Designer requesting client. This is the Elohim Protocol's own pattern
 * library asking for display context so it can render Matthew's lamad surface.
 * brandMark is a minimal inline SVG — the protocol's constellation glyph.
 */
const graphosDesignerClient: RequestingClient = {
  id: 'graphos-designer',
  displayName: 'Graphos Designer',
  brandMark: 'data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PGNpcmNsZSBjeD0iMTYiIGN5PSIxNiIgcj0iMTQiIGZpbGw9IiMyRDVGM0IiLz48Y2lyY2xlIGN4PSIxNiIgY3k9IjEwIiByPSIyIiBmaWxsPSIjRDRBMDNFIi8+PGNpcmNsZSBjeD0iMTAiIGN5PSIyMCIgcj0iMiIgZmlsbD0iI0Q0QTAzRSIvPjxjaXJjbGUgY3g9IjIyIiBjeT0iMjAiIHI9IjIiIGZpbGw9IiNENEEwM0UiLz48bGluZSB4MT0iMTYiIHkxPSIxMCIgeDI9IjEwIiB5Mj0iMjAiIHN0cm9rZT0iI0Q0QTAzRSIgc3Ryb2tlLXdpZHRoPSIxLjUiIG9wYWNpdHk9IjAuNiIvPjxsaW5lIHgxPSIxNiIgeTE9IjEwIiB4Mj0iMjIiIHkyPSIyMCIgc3Ryb2tlPSIjRDRBMDNFIiBzdHJva2Utd2lkdGg9IjEuNSIgb3BhY2l0eT0iMC42Ii8+PGxpbmUgeDE9IjEwIiB5MT0iMjAiIHgyPSIyMiIgeTI9IjIwIiBzdHJva2U9IiNENEEwM0UiIHN0cm9rZS13aWR0aD0iMS41IiBvcGFjaXR5PSIwLjYiLz48L3N2Zz4=',
};

/** Required claim — display name. Cannot be declined. */
const displayNameClaim: ClaimRef = {
  id: 'imagodei.displayName',
  label: 'Display name',
  description: 'Your public-facing name as shown to neighbors in your qahal',
};

/** Optional claim — community standing. Can be declined. */
const qahalStandingClaim: ClaimRef = {
  id: 'qahal.standing',
  label: 'Community standing',
  description: 'Your attested standing in your primary qahal — helps Graphos surface the right content lens',
};

/** Optional claim — lamad mastery. Can be declined. */
const lamadMasteryClaim: ClaimRef = {
  id: 'lamad.mastery',
  label: 'Learning mastery',
  description: 'Your current lamad mastery level — used to tune the learning surface to where you are',
};

const allThreeClaims: ClaimRef[] = [displayNameClaim, qahalStandingClaim, lamadMasteryClaim];

// Attestors for the portal header
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

const doorwayHostResolution: AuthorityResolution = {
  trustMode: 'doorway-host',
  authority: { label: 'alpha.elohim.host', id: 'alpha' },
  flywheelHint: false,
  attestors: [susanElder, jamesCircle, martaWitness],
};

// ---------------------------------------------------------------------------
// Decorators
// ---------------------------------------------------------------------------

function lightDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${PORTAL_TOKENS_LIGHT}
        ${TRUST_TOKENS_LIGHT}
        ${CONSENT_TOKENS_LIGHT}
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
        ${CONSENT_TOKENS_DARK}
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
  title: 'Designed/Imagodei/ConsentCardThreeClaims',
  parameters: {
    docs: {
      description: {
        component: `
**Graphos Designer is asking for your participation.**

Graphos Designer (graphos-designer.elohim.host) is requesting three claims as part
of an OAuth authorization flow. Matthew is already authenticated via alpha.elohim.host;
the portal has advanced to the consent step.

Claims:
- \`imagodei.displayName\` — required; locked toggle; Matthew must share his display name
- \`qahal.standing\` — optional; he may decline without losing access
- \`lamad.mastery\` — optional; he may decline

The RP brand strip carries Graphos Designer's constellation glyph in a warm cream-gold
tinted background (#FCF0D8 in light mode) — the Harvest Gold warmth without the
aggressive orange of a CTA button. The approve button is Vineyard green; decline is quiet.

The trust-indicator is inherited from the portal-shell context: alpha.elohim.host (⌂)
is still present in the header, so Matthew knows which doorway is signing this consent.

Library B — primitive untouched; brand tokens bound via story decorator.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Default — three claims, consent step
// ---------------------------------------------------------------------------

export const Default: Story = {
  name: 'Default (three claims)',
  decorators: [lightDecorator],
  render: () => {
    return html`
      <elohim-imagodei-portal-shell
        .authority=${doorwayHostResolution}
        step="consent"
      >

        <elohim-imagodei-consent-card
          slot="primary"
          .requestingClient=${graphosDesignerClient}
          .requestedClaims=${allThreeClaims}
          .requiredClaims=${['imagodei.displayName']}
        >
          <div slot="policy-link" style="
            font-family: var(--el-font-body);
            font-size: 0.8125rem;
            color: var(--el-stone);
            opacity: 0.8;
          ">
            By approving, Graphos Designer will receive only the claims you share above.
            You can revoke access from your imagodei settings at any time.
          </div>
        </elohim-imagodei-consent-card>

        <span slot="footer">
          Signing as matthew&#64;alpha.elohim.host &middot;
          <a href="/identity/settings" style="color: inherit; opacity: 0.7;">manage access</a>
        </span>
      </elohim-imagodei-portal-shell>
    `;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Three-claim consent — the full scene. The RP brand strip carries the Graphos ' +
          'constellation glyph in a warm cream-gold (#FCF0D8) background. The display-name ' +
          'toggle is checked and disabled ("required"). Community standing and lamad mastery ' +
          'can be toggled off. The Vineyard green approve button (#2D5F3B) is quiet and ' +
          'grounded — not a pushy green CTA. The trust-indicator at top confirms which ' +
          'doorway is hosting this consent.',
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
    return html`
      <elohim-imagodei-portal-shell
        .authority=${doorwayHostResolution}
        step="consent"
      >

        <elohim-imagodei-consent-card
          slot="primary"
          .requestingClient=${graphosDesignerClient}
          .requestedClaims=${allThreeClaims}
          .requiredClaims=${['imagodei.displayName']}
        >
          <div slot="policy-link" style="
            font-family: var(--el-font-body);
            font-size: 0.8125rem;
            color: var(--el-starlight);
            opacity: 0.6;
          ">
            By approving, Graphos Designer receives only the claims you share above.
            Revoke access from your imagodei settings at any time.
          </div>
        </elohim-imagodei-consent-card>

        <span slot="footer">
          Signing as matthew&#64;alpha.elohim.host &middot;
          <a href="/identity/settings" style="color: inherit; opacity: 0.5;">manage access</a>
        </span>
      </elohim-imagodei-portal-shell>
    `;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Dark mode consent. The RP brand strip uses a Harvest Gold wash at low opacity ' +
          '(rgba(212, 160, 62, 0.1)) — warm signal without brightness. The claim rows are ' +
          'barely-there translucent panels (rgba(232, 228, 217, 0.04)). The Vineyard green ' +
          'approve button holds its ground in dark mode: same earthy anchor.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// All-optional variant — no required claims
// ---------------------------------------------------------------------------

export const AllOptional: Story = {
  name: 'AllOptional',
  decorators: [lightDecorator],
  render: () => {
    return html`
      <elohim-imagodei-portal-shell
        .authority=${doorwayHostResolution}
        step="consent"
      >

        <elohim-imagodei-consent-card
          slot="primary"
          .requestingClient=${graphosDesignerClient}
          .requestedClaims=${[qahalStandingClaim, lamadMasteryClaim]}
          .requiredClaims=${[]}
        >
          <div slot="policy-link" style="
            font-family: var(--el-font-body);
            font-size: 0.8125rem;
            color: var(--el-stone);
            opacity: 0.8;
          ">
            All claims are optional. You may approve with none selected.
          </div>
        </elohim-imagodei-consent-card>

        <span slot="footer">
          Signing as matthew&#64;alpha.elohim.host
        </span>
      </elohim-imagodei-portal-shell>
    `;
  },
  parameters: {
    docs: {
      description: {
        story:
          'All-optional variant: qahal.standing + lamad.mastery, no required claims. ' +
          'Both toggles start checked but can be turned off. Approving with none checked ' +
          'is valid — the household participates on their terms. The approve button does not ' +
          'change copy or become disabled.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Peer-conductor context
// ---------------------------------------------------------------------------

export const PeerConductorContext: Story = {
  name: 'PeerConductorContext',
  decorators: [lightDecorator],
  render: () => {
    const peerResolution: AuthorityResolution = {
      trustMode: 'peer-conductor',
      authority: { label: 'your conductor — alpha.elohim.host is helping with ingress' },
      flywheelHint: false,
      attestors: [susanElder, jamesCircle, martaWitness],
    };

    return html`
      <elohim-imagodei-portal-shell
        .authority=${peerResolution}
        step="consent"
      >

        <elohim-imagodei-consent-card
          slot="primary"
          .requestingClient=${graphosDesignerClient}
          .requestedClaims=${allThreeClaims}
          .requiredClaims=${['imagodei.displayName']}
          trust-mode="peer-conductor"
        ></elohim-imagodei-consent-card>

        <span slot="footer">
          Your conductor is signing this consent
        </span>
      </elohim-imagodei-portal-shell>
    `;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Peer-conductor context: same three claims, but the trust-indicator shows ◇ ' +
          '(peer mode, sky-blue accent). Matthew\'s own conductor is signing the consent ' +
          '— a stronger claim than doorway-signed. The consent-card copy adapts to the ' +
          'peer-conductor trust mode.',
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
          ${CONSENT_TOKENS_LIGHT}
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
    return html`
      <elohim-imagodei-portal-shell
        .authority=${doorwayHostResolution}
        step="consent"
      >

        <elohim-imagodei-consent-card
          slot="primary"
          .requestingClient=${graphosDesignerClient}
          .requestedClaims=${allThreeClaims}
          .requiredClaims=${['imagodei.displayName']}
        ></elohim-imagodei-consent-card>

        <span slot="footer">alpha.elohim.host &middot; commons</span>
      </elohim-imagodei-portal-shell>
    `;
  },
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary — he-IL locale. The claim rows should mirror: toggle on the left ' +
          '(inline-start in LTR becomes inline-end in RTL, and vice versa). The RP brand ' +
          'strip mirrors. The approve and decline buttons should respect inline flow direction. ' +
          'The "(required)" badge should sit on the correct side of the label.',
      },
    },
  },
};
