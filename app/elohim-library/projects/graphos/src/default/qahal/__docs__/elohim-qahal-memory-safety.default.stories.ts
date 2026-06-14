/**
 * Library A default story — elohim-qahal-memory-safety
 *
 * Proves the element works as a blank-slate primitive: no Elohim brand tokens
 * bound, CSS system colors as defaults, override surface honest.
 *
 * <elohim-qahal-memory-safety> is the household-facing "Family Vault" /
 * Memory Safety card — the INVERSE of the operator debug Lens.
 * It renders NAMES, not nines, so that when grandma's edge node goes
 * offline, her family sees that her photos are still held by people
 * who love them.
 *
 * NOTE: FeltStatusView will be imported from '@elohim/storage-client'
 * once cargo test export_bindings regenerates the generated/ tree with
 * the additive feltStatus block on ResilienceSnapshotView.  Until then
 * the fixture objects below are typed against the identical local interface
 * from the element module.
 *
 * Data shape: FeltStatusView (additive Cat-C projection on ResilienceSnapshotView)
 * Home: elohim-elements/elohim-qahal/src/elohim-qahal-memory-safety.ts
 * Tag: <elohim-qahal-memory-safety>
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-qahal/register';
import type { FeltStatusView } from 'elohim-qahal';

// ---------------------------------------------------------------------------
// Fixture mock data — shapes must match FeltStatusView exactly.
// Realistic protocol vocabulary; no invented hashes surfaced to user.
// ---------------------------------------------------------------------------

/** Protected: floor wants 3, has 3.  All holders named. */
const mockProtected: FeltStatusView = {
  headline: 'Held by 3 households: the Dowells, Aunt Ruth, First Church',
  reassurance: 'protected',
  heldBy: [
    {
      id: 'sha256-a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2',
      kind: 'household',
      label: 'the Dowells',
    },
    {
      id: 'sha256-b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3',
      kind: 'household',
      label: 'Aunt Ruth',
    },
    {
      id: 'sha256-c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4',
      kind: 'church',
      label: 'First Church',
    },
  ],
  floor: {
    tier: 'standard',
    tierDeclared: false,
    wantsHouseholds: 3,
    hasHouseholds: 3,
  },
};

/** Watching: 2 of 3 — a holder lapsed but coverage survives. */
const mockWatching: FeltStatusView = {
  headline: 'These photos are safe — 2 households still hold a complete copy',
  reassurance: 'watching',
  heldBy: [
    {
      id: 'sha256-a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2',
      kind: 'household',
      label: 'the Dowells',
    },
    {
      id: 'sha256-c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4',
      kind: 'church',
      label: 'First Church',
    },
  ],
  floor: {
    tier: 'standard',
    tierDeclared: false,
    wantsHouseholds: 3,
    hasHouseholds: 2,
  },
};

/** Needs help: 1 of 3 — pro-social invite CTA present. */
const mockNeedsHelp: FeltStatusView = {
  headline: 'Only 1 household is holding these photos',
  reassurance: 'needs-help',
  heldBy: [
    {
      id: 'sha256-a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2',
      kind: 'household',
      label: 'the Dowells',
    },
  ],
  floor: {
    tier: 'standard',
    tierDeclared: false,
    wantsHouseholds: 3,
    hasHouseholds: 1,
  },
  suggestedAction: 'Invite a household to help hold these',
};

/**
 * Not yet seen: distributionState=unmeasured.
 * CRITICAL: must NEVER look like "protected".  Dashed badge + honest headline.
 */
const mockNotYetSeen: FeltStatusView = {
  headline: "We can't confirm these are backed up yet",
  reassurance: 'not-yet-seen',
  heldBy: [],
  floor: {
    tier: 'standard',
    tierDeclared: false,
    wantsHouseholds: 3,
    hasHouseholds: 0,
  },
  suggestedAction: 'Invite a household to help hold these',
};

/**
 * Vault floor: 3 named holders but tier=vault wants 5.
 * Reassurance="watching" — proves floor-relative honesty:
 * having 3 holders is NOT "protected" for a vault-tier memory.
 */
const mockVaultFloor: FeltStatusView = {
  headline: 'Held by 3 households: the Dowells, Aunt Ruth, First Church',
  reassurance: 'watching',
  heldBy: [
    {
      id: 'sha256-a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2',
      kind: 'household',
      label: 'the Dowells',
    },
    {
      id: 'sha256-b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3',
      kind: 'household',
      label: 'Aunt Ruth',
    },
    {
      id: 'sha256-c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4',
      kind: 'church',
      label: 'First Church',
    },
  ],
  floor: {
    tier: 'vault',
    tierDeclared: true,
    wantsHouseholds: 5,
    hasHouseholds: 3,
  },
};

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Qahal/elohim-qahal-memory-safety',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-qahal-memory-safety>\` — the household-facing "Family Vault" / Memory Safety card.

The INVERSE of the operator debug Lens: renders NAMES, not nines.
When grandma's edge node dies, her family opens this card and SEES —
in plain, named language — that her photos are still held by people
who love her.

**Data shape:** \`FeltStatusView\` — the additive \`feltStatus\` block
on \`ResilienceSnapshotView\` (Cat-C operational view, same projection
as \`GET /api/v1/resilience/{contentId}/household\`).

**Four reassurance states:**
- \`protected\` — settled; all floor coverage met
- \`watching\` — coverage survives but attentive; NOT alarming
- \`needs-help\` — below floor; warm pro-social CTA
- \`not-yet-seen\` — unmeasured; HONEST gap (never looks like protected)

**Override surface:** \`--elohim-memory-safety-*\` CSS custom properties.
**Parts:** \`container\`, \`heading\`, \`status-badge\`, \`headline\`, \`holder-list\`,
\`holder-chip\`, \`floor-line\`, \`cta\`, \`empty\`.

**Acceptance target:** the \`@browser-only\` Scenario "The Family Vault surface
shows the holders by name" in
\`genesis/a2o/features/resilience/grandma-photos-survive-node-loss.feature\`.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Four reassurance-state stories
// ---------------------------------------------------------------------------

/** Protected: all 3 named holders; floor wants 3 has 3. */
export const Protected: Story = {
  name: 'Protected',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 480px;">
      <elohim-qahal-memory-safety
        content-label="summer-1974"
        .feltStatus=${mockProtected}
      ></elohim-qahal-memory-safety>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          '`protected` state: 3 named households hold the content; floor (wants 3) is fully met. Settled, not alarming. Names visible, no shard hashes.',
      },
    },
  },
};

/** Watching: 2 of 3 holders remain — a lapse but coverage survives. */
export const Watching: Story = {
  name: 'Watching',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 480px;">
      <elohim-qahal-memory-safety
        content-label="summer-1974"
        .feltStatus=${mockWatching}
      ></elohim-qahal-memory-safety>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          '`watching` state: Aunt Ruth\'s copy went offline; 2 households still hold the content. Attentive, NOT alarming. Floor line reads "2 of the 3 households".',
      },
    },
  },
};

/** Needs help: 1 of 3 — pro-social invite CTA. */
export const NeedsHelp: Story = {
  name: 'NeedsHelp',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 480px;">
      <elohim-qahal-memory-safety
        content-label="wedding-album"
        .feltStatus=${mockNeedsHelp}
      ></elohim-qahal-memory-safety>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          '`needs-help` state: only 1 household holds the content. CTA renders as a warm invitation — "Invite a household to help hold these". No red alarm language.',
      },
    },
  },
};

/**
 * Not yet seen: unmeasured — the critical honesty state.
 * MUST NOT look like protected.  Badge uses a dashed border; headline
 * says "can't confirm" not "safe".
 */
export const NotYetSeen: Story = {
  name: 'NotYetSeen (honesty critical)',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 480px;">
      <elohim-qahal-memory-safety
        content-label="private-letters"
        .feltStatus=${mockNotYetSeen}
      ></elohim-qahal-memory-safety>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          '`not-yet-seen` state: this content has never entered the distribution plane (`distributionState=unmeasured`). The badge is dashed (visually distinct from protected), the headline is honest ("can\'t confirm"), and a CTA invites action. **This must NEVER look like the protected state.** Unmeasured ≠ safe.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Floor honesty proof
// ---------------------------------------------------------------------------

/**
 * Vault floor: 3 named holders but vault tier wants 5.
 * Reassurance is "watching" — proves 3 holders is NOT "protected"
 * when the floor demands 5.
 */
export const VaultFloor: Story = {
  name: 'VaultFloor (floor-relative honesty)',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 480px;">
      <elohim-qahal-memory-safety
        content-label="grandmother-archive"
        .feltStatus=${mockVaultFloor}
      ></elohim-qahal-memory-safety>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Vault tier: the floor wants 5 holders, has 3. Even though 3 holders are named, reassurance is `watching` — not `protected`. Proves floor-relative honesty. The floor line reads "3 of the 5 households this should live in (vault tier)".',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// States
// ---------------------------------------------------------------------------

/** Loading/empty: feltStatus is null. */
export const Empty: Story = {
  name: 'Empty (loading state)',
  render: () => html`
    <div style="padding: 1rem; max-inline-size: 480px;">
      <elohim-qahal-memory-safety content-label="summer-1974"></elohim-qahal-memory-safety>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          '`feltStatus=null`: the loading/empty state. Renders a neutral placeholder message. The `loading` slot can be overridden by the host for a custom skeleton.',
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
      <div style="background: #1a1a1a; color: #e8e8e8; color-scheme: dark; padding: 1.5rem;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <div style="max-inline-size: 480px;">
      <elohim-qahal-memory-safety
        content-label="summer-1974"
        .feltStatus=${mockProtected}
      ></elohim-qahal-memory-safety>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Dark background canary. CSS system colors (`Canvas`, `CanvasText`, `ButtonFace`, `ButtonText`) adapt automatically to the dark color scheme.',
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
      <div dir="rtl" lang="he" style="padding: 1rem;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <div style="max-inline-size: 480px;">
      <elohim-qahal-memory-safety
        content-label="summer-1974"
        .feltStatus=${mockWatching}
      ></elohim-qahal-memory-safety>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary: the element uses logical CSS properties throughout (`padding-block`, `padding-inline`, `margin-block`, `margin-inline`, `border-radius`, `flex-direction`). Layout mirrors correctly in a right-to-left context.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Blank-slate proof
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial; display: block; padding: 1rem;">${story()}</div>`],
  render: () => html`
    <elohim-qahal-memory-safety
      content-label="summer-1974"
      .feltStatus=${mockProtected}
    ></elohim-qahal-memory-safety>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Wrapped in `style="all: initial;"` — zero external CSS. The element still renders with CSS system colors (`Canvas`, `ButtonFace`, `ButtonText`) and logical properties. Proves no brand bake.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// CustomTheme — override-surface proof
// ---------------------------------------------------------------------------

export const CustomTheme: Story = {
  name: 'CustomTheme (override-surface proof)',
  decorators: [
    story => html`
      <div
        style="
          --elohim-memory-safety-bg: #0a0a2e;
          --elohim-memory-safety-fg: #d0eaff;
          --elohim-memory-safety-border: 2px solid #4488cc;
          --elohim-memory-safety-radius: 0.125rem;
          --elohim-memory-safety-holder-chip-bg: #1a2a4a;
          --elohim-memory-safety-holder-chip-fg: #90c8ff;
          --elohim-memory-safety-status-protected-color: #66ff88;
          --elohim-memory-safety-cta-bg: #1a2a4a;
          --elohim-memory-safety-cta-fg: #90c8ff;
          --elohim-memory-safety-cta-border: 1px solid #4488cc;
          font-family: ui-monospace, monospace;
          padding: 1.5rem;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <div style="max-inline-size: 480px;">
      <elohim-qahal-memory-safety
        content-label="summer-1974"
        .feltStatus=${mockProtected}
      ></elohim-qahal-memory-safety>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Override-surface proof: a deliberately non-Elohim "deep-space terminal" theme applied entirely via `--elohim-memory-safety-*` CSS custom properties and `font-family`. Proves the override surface is honest — no hardcoded brand values inside the element.',
      },
    },
  },
};
