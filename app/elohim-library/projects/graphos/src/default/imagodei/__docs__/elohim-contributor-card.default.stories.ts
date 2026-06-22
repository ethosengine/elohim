/**
 * Library A default story — elohim-contributor-card
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * The contributor card renders ONE `ContributorPresenceView` (the ts-rs wire
 * type from @elohim/storage-client) as a card in the "Contributors / Inspired
 * by" panel. It is used in EPR content-viewer Task 3 and, later, the presence
 * profile surface.
 *
 * NOTE: `ContributorPresenceView` does not carry a `presenceType` field on the
 * wire. Consumers supply it via the `presence-type` property when they have
 * domain context. Stories covering person/organisation differentiation bind
 * this property directly.
 */

import type { ContributorPresenceView } from '@elohim/storage-client';
import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';

// ---------------------------------------------------------------------------
// Fixtures — trimmed, realistic, from the wire type shape
// ---------------------------------------------------------------------------

const PERSON_UNCLAIMED: ContributorPresenceView = {
  id: 'sha256-0000000000000000000000000000000000000000000000000000000000000001',
  hAppId: 'sha256-app0000000000000000000000000000000000000000000000000000000000001',
  displayName: 'Ada Lovelace',
  presenceState: 'unclaimed',
  externalIdentifiers: null,
  establishingContentIds: [],
  affinityTotal: 14.5,
  uniqueEngagers: 3,
  citationCount: 7,
  recognitionScore: 42.0,
  recognitionByContent: null,
  lastRecognitionAt: null,
  stewardId: null,
  stewardshipStartedAt: null,
  stewardshipCommitmentId: null,
  stewardshipQualityScore: null,
  claimInitiatedAt: null,
  claimVerifiedAt: null,
  claimVerificationMethod: null,
  claimEvidence: null,
  claimedAgentId: null,
  claimRecognitionTransferredValue: null,
  claimFacilitatedBy: null,
  image: null,
  note: null,
  metadata: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
  dhtAnchorHash: null,
};

const PERSON_CLAIMED: ContributorPresenceView = {
  ...PERSON_UNCLAIMED,
  id: 'sha256-0000000000000000000000000000000000000000000000000000000000000002',
  displayName: 'Grace Hopper',
  presenceState: 'claimed',
  image:
    'https://upload.wikimedia.org/wikipedia/commons/thumb/a/ad/Commodore_Grace_M._Hopper%2C_USN_%28covered%29.jpg/220px-Commodore_Grace_M._Hopper%2C_USN_%28covered%29.jpg',
  recognitionScore: 128.0,
  citationCount: 22,
  affinityTotal: 63.0,
  uniqueEngagers: 11,
  claimedAgentId: 'uhCAkSomeAgentKey',
};

const ORG_STEWARDED: ContributorPresenceView = {
  ...PERSON_UNCLAIMED,
  id: 'sha256-0000000000000000000000000000000000000000000000000000000000000003',
  displayName: 'Open Source Collective',
  presenceState: 'stewarded',
  recognitionScore: 305.0,
  citationCount: 47,
  affinityTotal: 120.0,
  uniqueEngagers: 28,
  stewardId: 'sha256-steward00000000000000000000000000000000000000000000000000000001',
  stewardshipStartedAt: '2026-02-01T00:00:00Z',
};

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Imagodei/elohim-contributor-card',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-contributor-card>\` — renders one \`ContributorPresenceView\` as a card.

Used in the "Contributors / Inspired by" panel on the EPR content viewer and
the presence profile surface. The element:

- Accepts a single \`.presence\` property typed against the ts-rs
  \`ContributorPresenceView\` wire shape.
- Derives a state badge from \`presenceState\` ("Open to steward" for
  unclaimed — recognition-before-registration, framed as opportunity not error).
- Emits \`contributor-select { id }\` on activation (click or Enter/Space).
- Accepts an optional \`href\` for when a profile route exists.
- Shows/hides affinity stats via the \`compact\` boolean.

**Override surface:** all visual properties are \`--elohim-contributor-card-*\`
CSS custom properties with neutral CSS system color defaults.
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
  decorators: [
    story => html`
      <div style="all: initial;">${story()}</div>
    `,
  ],
  render: () => html`
    <elohim-contributor-card .presence=${PERSON_UNCLAIMED}></elohim-contributor-card>
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
      <div
        style="
        font-family: Georgia, serif;
        color: #222;
        max-inline-size: 22rem;
        --elohim-contributor-card-bg: #f5f0e8;
        --elohim-contributor-card-fg: #2b2319;
        --elohim-contributor-card-border: 2px dashed #8b7355;
        --elohim-contributor-card-radius: 0;
        --elohim-contributor-card-badge-bg: #fff8ec;
        --elohim-contributor-card-badge-border: 1px solid #8b7355;
        --elohim-contributor-card-stats-fg: #6b5a3e;
        padding: 1rem;
      "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-contributor-card .presence=${PERSON_CLAIMED}></elohim-contributor-card>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Override-surface proof: parchment background, dashed border, serif font — deliberately non-Elohim. Demonstrates the `--elohim-contributor-card-*` properties are the honest override surface.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// State stories
// ---------------------------------------------------------------------------

export const UnclaimedOpportunity: Story = {
  name: 'Unclaimed — recognition-before-registration',
  decorators: [
    story => html`
      <div style="max-inline-size: 22rem;">${story()}</div>
    `,
  ],
  render: () => html`
    <elohim-contributor-card
      .presence=${PERSON_UNCLAIMED}
      presence-type="person"
    ></elohim-contributor-card>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'The unclaimed state: "Open to steward" badge with opportunity framing. `presenceState = "unclaimed"` means a contributor has been recognised by content but no steward has picked them up yet — this is the protocol\'s recognition-before-registration moment.',
      },
    },
  },
};

export const ClaimedWithImage: Story = {
  name: 'Claimed — with avatar image',
  decorators: [
    story => html`
      <div style="max-inline-size: 22rem;">${story()}</div>
    `,
  ],
  render: () => html`
    <elohim-contributor-card
      .presence=${PERSON_CLAIMED}
      presence-type="person"
    ></elohim-contributor-card>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Claimed presence with a real avatar image. The `<img>` renders inside the avatar part; the initials placeholder is hidden.',
      },
    },
  },
};

export const OrganisationStewarded: Story = {
  name: 'Organisation — stewarded',
  decorators: [
    story => html`
      <div style="max-inline-size: 22rem;">${story()}</div>
    `,
  ],
  render: () => html`
    <elohim-contributor-card
      .presence=${ORG_STEWARDED}
      presence-type="organization"
    ></elohim-contributor-card>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'An organisation presence in "stewarded" state. `presence-type="organization"` makes the ◻ glyph visible in the name row (consumers can replace it via the `presence-type-glyph` slot).',
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
      <div
        style="background: #111; padding: 1.5rem; color-scheme: dark; color: #f0f0f0; max-inline-size: 22rem;"
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-contributor-card .presence=${PERSON_UNCLAIMED}></elohim-contributor-card>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Dark theme canary: card on dark background. CSS system colors adapt via `color-scheme: dark`.',
      },
    },
  },
};
