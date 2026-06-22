/**
 * Library B designed story — elohim-contributor-card
 *
 * Binds the Elohim brand to `<elohim-contributor-card>` via story-decorator
 * overrides. The primitive is NEVER modified — binding lives at the decorator
 * level only, mapped through the element's declared `--elohim-contributor-card-*`
 * CSS custom properties.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types  — `ContributorPresenceView` from `@elohim/storage-client` (ts-rs generated).
 *   2. Manifest — presence field values follow the protocol vocabulary
 *                (`presenceState` values and `stewardId` formats from imagodei).
 *   3. Tokens — `--el-*` palette declared inline, matching design-spec §14.
 *
 * Fixtures are drawn from the protocol's seeded presence cohort
 * (genesis/data/presences/). Names, roles, and ids are realistic, not placeholders.
 *
 * Two stories — within sprint budget:
 *   1. UnclaimedInspirer (light)  — Ingrid Robeyns, a manifesto inspirer; warm Linen day palette.
 *      The "Open to steward" badge reads as invitation, not absence.
 *   2. CollectiveStewarded (dark) — Valueflows org; constellation Indigo Night palette.
 *
 * Bound props (all 14 from JSDoc `@cssprop`):
 *   --elohim-contributor-card-bg
 *   --elohim-contributor-card-fg
 *   --elohim-contributor-card-border
 *   --elohim-contributor-card-radius
 *   --elohim-contributor-card-padding
 *   --elohim-contributor-card-gap
 *   --elohim-contributor-card-avatar-size
 *   --elohim-contributor-card-avatar-bg
 *   --elohim-contributor-card-badge-bg
 *   --elohim-contributor-card-badge-fg
 *   --elohim-contributor-card-badge-border
 *   --elohim-contributor-card-badge-radius
 *   --elohim-contributor-card-stats-fg
 *   --elohim-contributor-card-hover-bg
 */

import type { ContributorPresenceView } from '@elohim/storage-client';
import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';

// ---------------------------------------------------------------------------
// Brand token constants (design spec §14)
// ---------------------------------------------------------------------------

const EL_TOKENS = `
  --el-green-deep:  #2D5F3B;
  --el-green-light: #7FB069;
  --el-amber:       #D4A03E;
  --el-clay:        #B8664F;
  --el-cream:       #F5F0E8;
  --el-stone:       #6B6157;
  --el-sky:         #7BAFCB;
  --el-plum:        #6E4B6B;
  --el-starlight:   #E8E4D9;
  --el-night:       #0F1A12;
  --el-night-alt:   #1A1A2E;
  --el-font-display: 'Fraunces', Georgia, serif;
  --el-font-body:    'Source Serif 4', Georgia, serif;
  --el-font-ui:      'DM Sans', system-ui, sans-serif;
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

/** Light — warm Linen surface, Hearthstone text, amber-tinted badge */
const CARD_TOKENS_LIGHT = `
  --elohim-contributor-card-bg:           var(--el-cream);
  --elohim-contributor-card-fg:           var(--el-stone);
  --elohim-contributor-card-border:       1px solid color-mix(in oklch, var(--el-stone) 18%, transparent);
  --elohim-contributor-card-radius:       var(--el-radius-md);
  --elohim-contributor-card-padding:      var(--el-space-md);
  --elohim-contributor-card-gap:          var(--el-space-sm);
  --elohim-contributor-card-avatar-size:  3rem;
  --elohim-contributor-card-avatar-bg:    color-mix(in oklch, var(--el-amber) 22%, var(--el-cream));
  --elohim-contributor-card-badge-bg:     color-mix(in oklch, var(--el-amber) 12%, var(--el-cream));
  --elohim-contributor-card-badge-fg:     var(--el-green-deep);
  --elohim-contributor-card-badge-border: 1px solid color-mix(in oklch, var(--el-amber) 40%, transparent);
  --elohim-contributor-card-badge-radius: 999px;
  --elohim-contributor-card-stats-fg:     color-mix(in oklch, var(--el-stone) 70%, transparent);
  --elohim-contributor-card-hover-bg:     color-mix(in oklch, var(--el-amber) 8%, var(--el-cream));
`;

/** Dark — constellation Indigo Night surface, Starlight text, subdued amber badge */
const CARD_TOKENS_DARK = `
  --elohim-contributor-card-bg:           var(--el-night-alt);
  --elohim-contributor-card-fg:           var(--el-starlight);
  --elohim-contributor-card-border:       1px solid color-mix(in oklch, var(--el-starlight) 12%, transparent);
  --elohim-contributor-card-radius:       var(--el-radius-md);
  --elohim-contributor-card-padding:      var(--el-space-md);
  --elohim-contributor-card-gap:          var(--el-space-sm);
  --elohim-contributor-card-avatar-size:  3rem;
  --elohim-contributor-card-avatar-bg:    color-mix(in oklch, var(--el-plum) 35%, var(--el-night-alt));
  --elohim-contributor-card-badge-bg:     color-mix(in oklch, var(--el-amber) 10%, var(--el-night-alt));
  --elohim-contributor-card-badge-fg:     var(--el-starlight);
  --elohim-contributor-card-badge-border: 1px solid color-mix(in oklch, var(--el-amber) 30%, transparent);
  --elohim-contributor-card-badge-radius: 999px;
  --elohim-contributor-card-stats-fg:     color-mix(in oklch, var(--el-starlight) 60%, transparent);
  --elohim-contributor-card-hover-bg:     color-mix(in oklch, var(--el-amber) 6%, var(--el-night-alt));
`;

// ---------------------------------------------------------------------------
// Decorator factories
// ---------------------------------------------------------------------------

function lightDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${CARD_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        padding: var(--el-space-xl);
        max-inline-size: 360px;
        box-shadow: var(--el-shadow-soft);
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
        ${CARD_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        padding: var(--el-space-xl);
        max-inline-size: 360px;
      "
    >
      ${story()}
    </div>
  `;
}

// ---------------------------------------------------------------------------
// Fixtures — drawn from the protocol's seeded presence cohort
//   genesis/data/presences/ingrid-robeyns.md (presence-ingrid-robeyns)
//   genesis/data/presences/valueflows.md     (presence-valueflows)
//
// CIDs are sha256- + 64 hex chars (mock-data discipline).
// ---------------------------------------------------------------------------

// Ingrid Robeyns — philosopher, limitarian, named in the elohim.host manifesto
// "Inspired by" acknowledgements. presenceState = 'unclaimed' because she has
// been observed but no household member has yet taken up the stewardship.
const INGRID_ROBEYNS: ContributorPresenceView = {
  id: 'sha256-a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2',
  hAppId: 'sha256-fffe000000000000000000000000000000000000000000000000000000000001',
  displayName: 'Ingrid Robeyns',
  presenceState: 'unclaimed',
  externalIdentifiers: null,
  establishingContentIds: [],
  affinityTotal: 8.5,
  uniqueEngagers: 2,
  citationCount: 4,
  recognitionScore: 27.0,
  recognitionByContent: null,
  lastRecognitionAt: '2026-03-14T00:00:00Z',
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
  note: 'Philosopher; proponent of limitarianism. Cited in the elohim.host manifesto "Inspired by" section.',
  metadata: null,
  createdAt: '2026-01-15T00:00:00Z',
  updatedAt: '2026-03-14T00:00:00Z',
  dhtAnchorHash: null,
};

// Valueflows — protocol/collective presence, stewarded by matthew-manager
// The Valueflows vocabulary is the economic substrate of the elohim protocol.
const VALUEFLOWS_ORG: ContributorPresenceView = {
  id: 'sha256-b3c4d5e6f7a8b3c4d5e6f7a8b3c4d5e6f7a8b3c4d5e6f7a8b3c4d5e6f7a8b3c4',
  hAppId: 'sha256-fffe000000000000000000000000000000000000000000000000000000000001',
  displayName: 'Valueflows',
  presenceState: 'stewarded',
  externalIdentifiers: [{ type: 'homepage', value: 'https://valueflo.ws/' }],
  establishingContentIds: [],
  affinityTotal: 64.0,
  uniqueEngagers: 14,
  citationCount: 31,
  recognitionScore: 182.0,
  recognitionByContent: null,
  lastRecognitionAt: '2026-05-01T00:00:00Z',
  stewardId: 'sha256-steward00000000000000000000000000000000000000000000000000000001',
  stewardshipStartedAt: '2021-08-07T00:00:00Z',
  stewardshipCommitmentId: null,
  stewardshipQualityScore: 0.91,
  claimInitiatedAt: null,
  claimVerifiedAt: null,
  claimVerificationMethod: null,
  claimEvidence: null,
  claimedAgentId: null,
  claimRecognitionTransferredValue: null,
  claimFacilitatedBy: null,
  image: null,
  note: 'The economic vocabulary that underpins the protocol\'s provision, stewardship, and REA flows.',
  metadata: null,
  createdAt: '2021-08-07T00:00:00Z',
  updatedAt: '2026-05-01T00:00:00Z',
  dhtAnchorHash: 'sha256-dht00000000000000000000000000000000000000000000000000000000001',
};

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Imagodei/elohim-contributor-card',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-contributor-card>\` bound to the Elohim brand — the "who inspired this"
card in the EPR content viewer's Contributors panel.

The protocol holds presence records for inspirers who have not yet been matched
with a steward. These are not empty states — they are **opportunities to honor
someone**. The "Open to steward" badge carries warm Harvest Gold tones in light
mode and a subdued amber note in constellation dark, so it reads as invitation
rather than absence.

**Token binding (all 14 \`--elohim-contributor-card-*\` props):**

| Property | Light (Linen day) | Dark (constellation) |
|---|---|---|
| bg | \`--el-cream\` | \`--el-night-alt\` |
| fg | \`--el-stone\` | \`--el-starlight\` |
| border | stone 18% | starlight 12% |
| avatar-bg | amber 22% in cream | plum 35% in night-alt |
| badge-bg | amber 12% in cream | amber 10% in night-alt |
| badge-fg | \`--el-green-deep\` | \`--el-starlight\` |
| stats-fg | stone 70% | starlight 60% |
| hover-bg | amber 8% in cream | amber 6% in night-alt |

Library B — graphos-designer. Primitive CSS untouched.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Story 1 — Unclaimed inspirer, light/day palette
// ---------------------------------------------------------------------------

export const UnclaimedInspirer: Story = {
  name: 'Unclaimed Inspirer (light)',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-contributor-card
      .presence=${INGRID_ROBEYNS}
      presence-type="person"
    ></elohim-contributor-card>
  `,
  parameters: {
    docs: {
      description: {
        story: `
Ingrid Robeyns — philosopher and limitarian — named in the elohim.host
manifesto "Inspired by" acknowledgements. Her presence in the commons is
**unclaimed**: she has been observed and cited, but no household member has
yet taken up the stewardship of her representation in the protocol.

The badge reads "Open to steward" in warm Harvest Gold on Linen — not a
missing-data grey. This is the **recognition-before-registration** moment:
the protocol knows this person shaped its thinking, and the card is an
open invitation for someone who knows her work to carry that stewardship
forward.

Light palette: Linen background (\`--el-cream\`), Hearthstone foreground
(\`--el-stone\`), warm shadow. Avatar placeholder uses amber tones rather
than a neutral grey — there is warmth here, not emptiness.
        `.trim(),
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Story 2 — Collective/organisation, dark/constellation palette
// ---------------------------------------------------------------------------

export const CollectiveStewarded: Story = {
  name: 'Collective Stewarded (dark)',
  decorators: [darkDecorator],
  render: () => html`
    <elohim-contributor-card
      .presence=${VALUEFLOWS_ORG}
      presence-type="organization"
    ></elohim-contributor-card>
  `,
  parameters: {
    docs: {
      description: {
        story: `
Valueflows — the economic vocabulary that underpins the protocol's
provision, stewardship, and REA flows. A **collective** presence, stewarded
from 2021 onward; 31 citations across commons content.

The ◻ glyph in the name row (from \`presence-type="organization"\`) marks
this as a collective contributor rather than a person. The "Stewarded" badge
carries a subdued amber note against the Indigo Night surface.

Dark palette: Indigo Night background (\`--el-night-alt\`), Starlight text
(\`--el-starlight\`). The card sits quietly in the constellation dark —
present, tended, and ready to be followed.
        `.trim(),
      },
    },
  },
};
