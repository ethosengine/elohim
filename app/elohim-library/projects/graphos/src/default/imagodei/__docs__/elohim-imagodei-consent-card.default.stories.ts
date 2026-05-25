/**
 * Library A default story — elohim-imagodei-consent-card
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * The consent-card is the RFC-6749 authorization-step consent surface of the peer
 * OAuth portal. Required claims are locked-on (checked + disabled toggle). Optional
 * claims can be toggled off. Defense-in-depth: a partial-decline-blocked guard fires
 * if a required claim is somehow removed via DOM manipulation.
 *
 * The requesting client in these stories is "Graphos Designer" — the lamad pattern
 * library — requesting `imagodei.displayName` (required) and `qahal.standing` (optional).
 * Protocol vocabulary from the imagodei + qahal manifest domains.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';
import type { ClaimRef, RequestingClient } from 'elohim-imagodei';

// ---------------------------------------------------------------------------
// Fixtures — realistic protocol vocabulary
// ---------------------------------------------------------------------------

const graphosDesignerClient: RequestingClient = {
  id: 'graphos-designer',
  displayName: 'Graphos Designer',
};

const graphosWithBrandMark: RequestingClient = {
  id: 'graphos-designer',
  displayName: 'Graphos Designer',
  brandMark: 'data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHJlY3Qgd2lkdGg9IjMyIiBoZWlnaHQ9IjMyIiByeD0iNiIgZmlsbD0iIzMzMyIvPjx0ZXh0IHg9IjE2IiB5PSIyMiIgZm9udC1zaXplPSIxOCIgdGV4dC1hbmNob3I9Im1pZGRsZSIgZmlsbD0iI2ZmZiI+RzwvdGV4dD48L3N2Zz4=',
};

const displayNameClaim: ClaimRef = {
  id: 'imagodei.displayName',
  label: 'Display name',
  description: 'Your public-facing name as shown to others in your community',
};

const standingClaim: ClaimRef = {
  id: 'qahal.standing',
  label: 'Community standing',
  description: 'Your attested standing in your primary qahal',
};

const profilePhotoClaim: ClaimRef = {
  id: 'imagodei.profilePhoto',
  label: 'Profile photo',
  description: 'Your avatar or profile image URL',
};

const allClaims: ClaimRef[] = [displayNameClaim, standingClaim, profilePhotoClaim];

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Imagodei/elohim-imagodei-consent-card',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-imagodei-consent-card>\` — RFC-6749 authorization-step consent surface.

Renders the RP brand strip (\`requestingClient.displayName\` + optional \`brandMark\`) plus
per-claim toggle rows. Required claims are locked-on (checked + disabled). Optional claims
can be toggled off before approval.

Defense-in-depth: if a required claim is somehow toggled off (impossible via UI, guarded
against DOM manipulation), \`approve\` emits a \`decline\` with \`reason: 'partial-decline-blocked'\`.

Emits \`approve\` with \`{ grantedClaims: string[] }\` or \`decline\` with \`{ reason: string }\`.

**Override surface:** bound via \`--elohim-consent-*\` CSS custom properties with neutral defaults.
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
  decorators: [story => html`<div style="all: initial; display: block; padding: 1rem;">${story()}</div>`],
  render: () => html`
    <elohim-imagodei-consent-card
      .requestingClient=${graphosDesignerClient}
      .requestedClaims=${[displayNameClaim, standingClaim]}
      .requiredClaims=${['imagodei.displayName']}
    ></elohim-imagodei-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Wrapped in `style="all: initial;"`. Card renders with system defaults — RP strip, claim rows, and action buttons all visible with zero tokens.',
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
        font-family: 'Gill Sans', 'Gill Sans MT', Calibri, sans-serif;
        color: #1b1b2f;
        background: #e2e2e2;
        --elohim-consent-rp-bg: #d4d4e8;
        --elohim-consent-rp-fg: #1b1b2f;
        --elohim-consent-claim-row-bg: #f5f5f5;
        --elohim-consent-claim-row-border: 2px solid #7b6da8;
        --elohim-consent-approve-bg: #7b6da8;
        --elohim-consent-approve-fg: #fff;
        --elohim-consent-decline-bg: transparent;
        --elohim-consent-decline-fg: #1b1b2f;
        --elohim-consent-gap: 1.25rem;
        --elohim-consent-radius: 4px;
        --elohim-consent-muted-opacity: 0.6;
        padding: 1.5rem;
        max-inline-size: 440px;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-consent-card
      .requestingClient=${graphosDesignerClient}
      .requestedClaims=${[displayNameClaim, standingClaim]}
      .requiredClaims=${['imagodei.displayName']}
    ></elohim-imagodei-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Override-surface proof: lavender-grey palette with purple approve button — deliberately non-Elohim. Demonstrates `--elohim-consent-*` properties are the honest override surface.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// State stories per spec §5.2
// ---------------------------------------------------------------------------

export const OneRequiredClaim: Story = {
  name: 'OneRequiredClaim',
  render: () => html`
    <div style="max-inline-size: 440px; padding: 1rem;">
      <elohim-imagodei-consent-card
        .requestingClient=${graphosDesignerClient}
        .requestedClaims=${[displayNameClaim]}
        .requiredClaims=${['imagodei.displayName']}
      ></elohim-imagodei-consent-card>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Single required claim: `imagodei.displayName`. The toggle is checked and disabled — the user cannot opt out. The "(required)" badge is shown.',
      },
    },
  },
};

export const RequiredPlusOptional: Story = {
  name: 'RequiredPlusOptional',
  render: () => html`
    <div style="max-inline-size: 440px; padding: 1rem;">
      <elohim-imagodei-consent-card
        .requestingClient=${graphosDesignerClient}
        .requestedClaims=${allClaims}
        .requiredClaims=${['imagodei.displayName']}
      ></elohim-imagodei-consent-card>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Three claims: one required (`imagodei.displayName`) and two optional (`qahal.standing`, `imagodei.profilePhoto`). Optional claims can be toggled off before approving.',
      },
    },
  },
};

export const WithPolicyLink: Story = {
  name: 'WithPolicyLink',
  render: () => html`
    <div style="max-inline-size: 440px; padding: 1rem;">
      <elohim-imagodei-consent-card
        .requestingClient=${graphosDesignerClient}
        .requestedClaims=${[displayNameClaim, standingClaim]}
        .requiredClaims=${['imagodei.displayName']}
      >
        <div slot="policy-link" style="font-size: 0.875rem; opacity: 0.7;">
          By approving, you agree to Graphos Designer's
          <a href="#" target="_blank" rel="noopener noreferrer">Privacy Policy</a>
          and <a href="#" target="_blank" rel="noopener noreferrer">Terms of Service</a>.
        </div>
      </elohim-imagodei-consent-card>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'With `policy-link` slot: RP privacy policy and terms links rendered below the claim rows. The consumer provides the link copy; the card slots it before the action buttons.',
      },
    },
  },
};

export const WithBrandMark: Story = {
  name: 'WithBrandMark',
  render: () => html`
    <div style="max-inline-size: 440px; padding: 1rem;">
      <elohim-imagodei-consent-card
        .requestingClient=${graphosWithBrandMark}
        .requestedClaims=${[displayNameClaim, standingClaim]}
        .requiredClaims=${['imagodei.displayName']}
      ></elohim-imagodei-consent-card>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'With brand mark: `requestingClient.brandMark` provided as a data-URI SVG. The RP strip renders the image inside the `rp-mark` csspart (32×32 circle, `object-fit: cover`).',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Trust-mode variant
// ---------------------------------------------------------------------------

export const PeerConductorContext: Story = {
  name: 'PeerConductorContext',
  render: () => html`
    <div style="max-inline-size: 440px; padding: 1rem;">
      <elohim-imagodei-consent-card
        .requestingClient=${graphosDesignerClient}
        .requestedClaims=${[displayNameClaim, standingClaim]}
        .requiredClaims=${['imagodei.displayName']}
        trust-mode="peer-conductor"
      ></elohim-imagodei-consent-card>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Peer-conductor context: `trustMode` set to `peer-conductor`. Currently the card adapts contextual copy based on trust mode — rendered in both operational contexts.',
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
      <div style="background: #111; padding: 1.5rem; color-scheme: dark; color: #f0f0f0; max-inline-size: 440px;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-consent-card
      .requestingClient=${graphosDesignerClient}
      .requestedClaims=${[displayNameClaim, standingClaim]}
      .requiredClaims=${['imagodei.displayName']}
    ></elohim-imagodei-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Dark theme canary: RP strip uses color-mix() defaults; claim rows use transparent background. CSS system colors adapt to dark color-scheme.',
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
      <div dir="rtl" lang="he" style="padding: 1rem; max-inline-size: 440px;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-consent-card
      .requestingClient=${graphosDesignerClient}
      .requestedClaims=${[displayNameClaim, standingClaim]}
      .requiredClaims=${['imagodei.displayName']}
    ></elohim-imagodei-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story: 'RTL canary: he locale in RTL container. Logical CSS properties (padding-inline, padding-block) ensure correct layout mirroring throughout the card.',
      },
    },
  },
};
