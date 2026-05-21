/**
 * Library B designed story — elohim-compute-tile
 *
 * Binds the Elohim brand tokens (§14, elohim-protocol-design-spec.md) to the
 * `<elohim-compute-tile>` primitive via story-decorator overrides. The element
 * itself is NEVER modified — binding happens above via `--elohim-compute-tile-*`
 * CSS custom properties mapped to `--el-*` brand tokens.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — `ComputeTileHubValue` and `ComputeTileDeviceValue` from
 *      `elohim-core`, which type-alias the protocol's generated
 *      `HubComputeAggregateView` and `DeviceSummary` views (ts-rs from
 *      `@elohim/storage-client`).
 *   2. Manifest vocabulary — none referenced directly in this tile (no contentType
 *      or contentFormat fields in the compute shape).
 *   3. Brand tokens — inline `EL_TOKENS` constant declares the full `--el-*`
 *      palette from design spec §14. Injected via story-level decorators because
 *      no global stylesheet injects them into the storybook today.
 *
 * Binding canvas:
 *   --elohim-compute-tile-bg          → --el-cream (light) / --el-night (dark)
 *   --elohim-compute-tile-fg          → --el-stone (light) / --el-starlight (dark)
 *   --elohim-compute-tile-border      → --el-stone at 20% opacity (light) / none (dark)
 *   --elohim-compute-tile-radius      → --el-radius-md (8px)
 *   --elohim-compute-tile-padding     → --el-space-md --el-space-lg
 *   --elohim-compute-tile-label-color → --el-stone (light) / --el-starlight (dark)
 *   --elohim-compute-tile-value-color → --el-green-deep (light) / --el-amber (dark)
 *   --elohim-compute-tile-secondary-color → --el-stone (light) / --el-starlight at 80%
 *   --elohim-compute-tile-state-color → --el-green-light (idle/online) / --el-clay (warn)
 *   --elohim-compute-tile-gauge-track → --el-starlight (light) / rgba night overlay
 *   --elohim-compute-tile-gauge-fill  → --el-amber
 *   --elohim-compute-tile-meta-color  → --el-stone (light) / --el-starlight
 *
 * Follow-up for component-architect:
 *   - No new @cssprop hooks needed — the override surface is complete.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

import type {
  ComputeTileHubValue,
  ComputeTileDeviceValue,
} from 'elohim-core';

// ---------------------------------------------------------------------------
// Brand token declaration
// ---------------------------------------------------------------------------

/**
 * The full --el-* token set from design spec §14.
 * Injected via story decorators because no global stylesheet seeds them today.
 * When the graphos token stylesheet ships, this constant can be removed and
 * the values will resolve naturally from the cascade.
 */
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

/**
 * Light-mode token binding for --elohim-compute-tile-*.
 * Maps every declared @cssprop hook to a graphos brand value.
 */
const TILE_TOKENS_LIGHT = `
  --elohim-compute-tile-bg:               var(--el-cream);
  --elohim-compute-tile-fg:               var(--el-stone);
  --elohim-compute-tile-border:           1px solid rgba(107, 97, 87, 0.2);
  --elohim-compute-tile-radius:           var(--el-radius-md);
  --elohim-compute-tile-padding:          var(--el-space-md) var(--el-space-lg);
  --elohim-compute-tile-label-color:      var(--el-stone);
  --elohim-compute-tile-value-color:      var(--el-green-deep);
  --elohim-compute-tile-secondary-color:  var(--el-stone);
  --elohim-compute-tile-state-color:      var(--el-green-light);
  --elohim-compute-tile-gauge-track:      var(--el-starlight);
  --elohim-compute-tile-gauge-fill:       var(--el-amber);
  --elohim-compute-tile-meta-color:       var(--el-stone);
`;

/**
 * Dark-mode (constellation) token binding.
 * Starlight text on Deep Sky — enters the register of the logo.
 */
const TILE_TOKENS_DARK = `
  --elohim-compute-tile-bg:               var(--el-night);
  --elohim-compute-tile-fg:               var(--el-starlight);
  --elohim-compute-tile-border:           none;
  --elohim-compute-tile-radius:           var(--el-radius-md);
  --elohim-compute-tile-padding:          var(--el-space-md) var(--el-space-lg);
  --elohim-compute-tile-label-color:      var(--el-starlight);
  --elohim-compute-tile-value-color:      var(--el-amber);
  --elohim-compute-tile-secondary-color:  var(--el-starlight);
  --elohim-compute-tile-state-color:      var(--el-green-light);
  --elohim-compute-tile-gauge-track:      rgba(232, 228, 217, 0.12);
  --elohim-compute-tile-gauge-fill:       var(--el-amber);
  --elohim-compute-tile-meta-color:       var(--el-starlight);
`;

/**
 * High-contrast override — no opacity reductions, explicit borders,
 * warm-but-legible against Linen background.
 */
const TILE_TOKENS_HIGH_CONTRAST = `
  --elohim-compute-tile-bg:               var(--el-cream);
  --elohim-compute-tile-fg:               var(--el-night);
  --elohim-compute-tile-border:           2px solid var(--el-night);
  --elohim-compute-tile-radius:           var(--el-radius-md);
  --elohim-compute-tile-padding:          var(--el-space-md) var(--el-space-lg);
  --elohim-compute-tile-label-color:      var(--el-night);
  --elohim-compute-tile-value-color:      var(--el-green-deep);
  --elohim-compute-tile-secondary-color:  var(--el-night);
  --elohim-compute-tile-state-color:      var(--el-green-deep);
  --elohim-compute-tile-gauge-track:      var(--el-stone);
  --elohim-compute-tile-gauge-fill:       var(--el-green-deep);
  --elohim-compute-tile-meta-color:       var(--el-night);
`;

// ---------------------------------------------------------------------------
// Fixtures — protocol vocabulary, realistic shapes
// ---------------------------------------------------------------------------

/**
 * Household hub aggregate. Primary UX surface per hub-compute-aggregate-primary
 * design note. Matthew, Jessica, and James sharing a household pool. Shape
 * comes from the canonical `HubComputeAggregateView` (ts-rs).
 */
const householdHub: ComputeTileHubValue = {
  kind: 'hub',
  hubId: 'hub:household:matthew-jessica-james',
  hubKind: 'dwelling',
  displayLabel: "Matthew's household",
  compute: {
    free: 5_368_709_120n,     //  5 GB free
    used: 10_737_418_240n,    // 10 GB used
    stewarded: 12_884_901_888n, // 12 GB stewarded for neighbors
  },
  memberDeviceCount: 3,
};

/**
 * Per-device drill-down — Matthew's laptop. Shape comes from the canonical
 * `DeviceSummary` (ts-rs); the element renders the compute subset.
 */
const matthewLaptop: ComputeTileDeviceValue = {
  kind: 'device',
  peerId: 'peer:12D3KooWMatthewLaptopHashFakeButWellFormed',
  displayName: "Matthew's laptop",
  archetype: 'desktop',
  online: true,
  compute: {
    free: 2_147_483_648n,  // 2 GB
    used: 5_368_709_120n,  // 5 GB
    stewarded: 3_221_225_472n, // 3 GB
  },
};

/** Hub-of-one — degenerate single-device hub; hub-optional-floor pattern. */
const hubOfOne: ComputeTileHubValue = {
  kind: 'hub',
  hubId: 'hub:household:solo-device',
  hubKind: 'computed',
  displayLabel: 'Solo device hub',
  compute: {
    free: 1_073_741_824n,
    used: 2_147_483_648n,
    stewarded: 0n,
  },
  memberDeviceCount: 1,
};

/** Fresh node — no REA commitments, no system_metrics sample yet. */
const freshNode: ComputeTileHubValue = {
  kind: 'hub',
  hubId: 'hub:household:fresh-node',
  hubKind: 'computed',
  displayLabel: 'Fresh node',
  compute: null,
  memberDeviceCount: 1,
};

/** Offline device — compute null because no metrics sample available. */
const offlineNode: ComputeTileDeviceValue = {
  kind: 'device',
  peerId: 'peer:12D3KooWOfflineAlphaNodeHashFake00000000',
  displayName: 'Alpha node',
  archetype: 'node',
  online: false,
  compute: null,
};

// ---------------------------------------------------------------------------
// Shared capability profile shorthand
// ---------------------------------------------------------------------------

function profile(
  lens: string,
  overrides: Partial<{
    theme: string;
    contrast: string;
    locale: string;
    textuality: string;
    origin: string;
  }> = {}
) {
  return {
    lens,
    theme: 'auto',
    contrast: 'auto',
    locale: 'en',
    stimulus: 'still',
    textuality: 'textual',
    standings: [],
    lock: { kind: overrides.origin ?? 'pilot' },
    origin: overrides.origin ?? 'pilot',
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Core/elohim-compute-tile',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-compute-tile>\` — Elohim brand-bound compute-capacity tile.

Renders the compute triptych (free / used / stewarded) for either a household hub
aggregate or a single device, with progressive disclosure matching the viewer's
capability lens. Brand tokens from the graphos design spec are bound at the
story-decorator level; the primitive is untouched.

**Lens → visible content:**
| Lens     | What your household sees                                         |
|----------|------------------------------------------------------------------|
| minimal  | "You have enough." — the grandma surface, no numbers             |
| simple   | "5 GB free"                                                      |
| standard | "5 GB of 15 GB free"                                             |
| detail   | "5 GB of 15 GB · 12 GB stewarded · 10 GB used"                  |
| debug    | Above + hub-id / peer-id / sample freshness                     |
| trace    | Above + source-route (which DHT + REA aggregation produced this) |

**Token binding:** all \`--elohim-compute-tile-*\` hooks are mapped to \`--el-*\` brand
tokens from the graphos design spec §14. See the
\`Designed/Foundations/Compute Capacity Tokens\` entry for the full binding catalog.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Decorator factories
// ---------------------------------------------------------------------------

function lightDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${TILE_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        padding: var(--el-space-lg);
        box-shadow: var(--el-shadow-soft);
        max-inline-size: 360px;
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
        ${TILE_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        padding: var(--el-space-lg);
        max-inline-size: 360px;
      "
    >
      ${story()}
    </div>
  `;
}

// ---------------------------------------------------------------------------
// Lens stories — capability lens coverage (light theme)
// ---------------------------------------------------------------------------

export const Minimal: Story = {
  name: 'Minimal',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${profile('minimal')}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Minimal lens — the hospitality surface. "You have enough." when free space is healthy. ' +
          'No numbers, no triptych. A grandmother reading this knows whether to worry or not. ' +
          'Brand-bound with Linen background, Hearthstone body text, Vineyard value color.',
      },
    },
  },
};

export const Simple: Story = {
  name: 'Simple',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${profile('simple')}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Simple lens — "5 GB free." What your household can hold, in plain words. ' +
          'Harvest Gold fills the capacity gauge; Vineyard headlines the free count.',
      },
    },
  },
};

export const Standard: Story = {
  name: 'Standard',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${profile('standard')}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Standard lens — the clean two-tuple: "5 GB of 15 GB free." The adult-pilot default. ' +
          'Fraunces would carry the number well at display size if the element later adds ' +
          'a size variant; for now DM Sans from the UI font inherits cleanly.',
      },
    },
  },
};

export const Detail: Story = {
  name: 'Detail',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${profile('detail')}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Detail lens — the full triptych: free / total / stewarded / used. ' +
          'Source Serif 4 at body size for the detail row gives the ledger-page feel ' +
          'this lens calls for — unhurried, precise, trustworthy.',
      },
    },
  },
};

export const Debug: Story = {
  name: 'Debug',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      last-sample-at="2026-05-20T10:14:00Z"
      .profile=${profile('debug', { origin: 'elohim-support' })}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Debug lens — triptych + hub-id + member count + sample freshness timestamp. ' +
          'The meta section uses JetBrains Mono via `--elohim-compute-tile-meta-color` ' +
          'in Hearthstone — warm monospace, not cold terminal. elohim-support standing required.',
      },
    },
  },
};

export const Trace: Story = {
  name: 'Trace',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      last-sample-at="2026-05-20T10:14:00Z"
      source-route="aggregate_stewarded_bytes_by_peer + system_metrics → ClusterView"
      .profile=${profile('trace', { origin: 'elohim-support' })}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Trace lens — debug info plus the source-route annotation: which DHT query and ' +
          'REA aggregation produced these numbers. The meta section shows the full lineage. ' +
          'Operational visibility for elohim-support only.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Theme stories
// ---------------------------------------------------------------------------

export const Light: Story = {
  name: 'Light',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${profile('standard', { theme: 'light' })}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Light theme — the garden register. Linen surface, Hearthstone text, Vineyard ' +
          'value color, Harvest Gold gauge fill. The golden-hour rule: Harvest Gold appears ' +
          'in the gauge, anchoring warmth in the composition.',
      },
    },
  },
};

export const Dark: Story = {
  name: 'Dark',
  decorators: [darkDecorator],
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${profile('standard', { theme: 'dark' })}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Dark theme — the constellation register. Deep Sky background, Starlight text, ' +
          'Harvest Gold value color. The interface enters the visual language of the logo: ' +
          'reading stars by their own light. No pure-black background — the night carries ' +
          'the green undertone of Deep Sky (#0F1A12).',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// High contrast
// ---------------------------------------------------------------------------

export const HighContrast: Story = {
  name: 'HighContrast',
  decorators: [
    (story: () => unknown) => html`
      <div
        style="
          ${EL_TOKENS}
          ${TILE_TOKENS_HIGH_CONTRAST}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: var(--el-space-lg);
          max-inline-size: 360px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${profile('standard', { contrast: 'high' })}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'High-contrast binding — explicit 2px solid border replaces the soft box-shadow, ' +
          'Deep Sky text on Linen for maximum contrast ratio. Gauge fill shifts from Harvest ' +
          'Gold to Vineyard for a stronger stroke weight in the track.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// RTL canary — Hebrew (he-IL)
// ---------------------------------------------------------------------------

export const Hebrew: Story = {
  name: 'Hebrew',
  decorators: [
    (story: () => unknown) => html`
      <div
        dir="rtl"
        lang="he"
        style="
          ${EL_TOKENS}
          ${TILE_TOKENS_LIGHT}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: var(--el-space-lg);
          max-inline-size: 360px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${profile('detail', { locale: 'he' })}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary — he-IL locale inside a dir="rtl" container. Logical CSS properties ' +
          '(margin-block, padding-inline) in the primitive ensure the header, gauge, and ' +
          'detail rows mirror correctly. The label and value should read right-to-left with ' +
          'the status dot trailing on the left (inline-start in LTR becomes inline-end in RTL).',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Symbolic textuality
// ---------------------------------------------------------------------------

export const Symbolic: Story = {
  name: 'Symbolic',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${profile('standard', { textuality: 'symbolic' } as Parameters<typeof profile>[1])}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Symbolic textuality — the capacity gauge bar appears alongside the text value. ' +
          'Harvest Gold fill on a Starlight track: a warm horizon line. The gauge is ' +
          'aria-hidden and decorative — the text carries the semantic content. ' +
          'This mode suits glanceable interfaces where the proportional read matters more ' +
          'than the precise number.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Content-state stories
// ---------------------------------------------------------------------------

export const Loading: Story = {
  name: 'Loading',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-compute-tile
      tile-state="loading"
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Loading state — "Loading compute data…" in Hearthstone on Linen. ' +
          'The tile holds its card shape while waiting. No spinner — stillness is the ' +
          'Sabbath default. The state-banner uses the state-color token.',
      },
    },
  },
};

export const Error: Story = {
  name: 'Error',
  decorators: [
    (story: () => unknown) => html`
      <div
        style="
          ${EL_TOKENS}
          ${TILE_TOKENS_LIGHT}
          --elohim-compute-tile-state-color: var(--el-clay);
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: var(--el-space-lg);
          max-inline-size: 360px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-compute-tile
      tile-state="error"
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Error state — "Unable to load compute data." The state-color is Terracotta ' +
          '(--el-clay) — earthy caution, never aggressive red. The protocol has no ' +
          '"danger red." Urgency is communicated through warm language and Terracotta warmth.',
      },
    },
  },
};

export const Stale: Story = {
  name: 'Stale',
  decorators: [
    (story: () => unknown) => html`
      <div
        style="
          ${EL_TOKENS}
          ${TILE_TOKENS_LIGHT}
          --elohim-compute-tile-state-color: var(--el-plum);
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: var(--el-space-lg);
          max-inline-size: 360px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-compute-tile
      tile-state="stale"
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Stale state — ContentCertainty=stale. "Compute data may be stale." ' +
          'State-color bound to Sabbath Plum (--el-plum): a rest-register color that ' +
          'communicates "this is paused" rather than "this is broken." The data was ' +
          'real — it just hasn\'t been refreshed.',
      },
    },
  },
};

export const Offline: Story = {
  name: 'Offline',
  decorators: [
    (story: () => unknown) => html`
      <div
        style="
          ${EL_TOKENS}
          ${TILE_TOKENS_LIGHT}
          --elohim-compute-tile-state-color: var(--el-stone);
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: var(--el-space-lg);
          max-inline-size: 360px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-compute-tile
      tile-state="offline"
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Offline state — "Device is offline." State-color is Hearthstone (--el-stone): ' +
          'the quietest, most neutral signal. Being offline is not an error — it\'s a ' +
          'natural state for a household node at rest.',
      },
    },
  },
};

export const Empty: Story = {
  name: 'Empty',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-compute-tile
      tile-state="empty"
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Empty state — "No compute data available." The tile holds its card shape, ' +
          'announcing readiness. Per the constellation design language: an empty state ' +
          'is a single node waiting for the first connection — not an error, not a void.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Input-shape stories — hub vs. device polymorphism
// ---------------------------------------------------------------------------

export const HubAggregate: Story = {
  name: 'HubAggregate',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${profile('standard')}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Hub-kind input: the household aggregate (3 member devices). ' +
          'This is the primary UX surface per the hub-compute-aggregate-primary ' +
          'design note — most households will see this, not a per-device drill-down.',
      },
    },
  },
};

export const PerDevice: Story = {
  name: 'PerDevice',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-compute-tile
      .value=${matthewLaptop}
      .profile=${profile('standard')}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          "Device-kind input: Matthew's laptop drill-down. " +
          'Secondary surface — shown when a household member asks "which device is this?" ' +
          'The archetype label ("Laptop") and status dot distinguish it from the hub card.',
      },
    },
  },
};

export const HubOfOne: Story = {
  name: 'HubOfOne',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-compute-tile
      .value=${hubOfOne}
      .profile=${profile('standard')}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Degenerate hub-of-one — a single device participating as its own hub. ' +
          'Per the hub-optional-floor design principle: a laptop alone is a full ' +
          'protocol participant. No hub hardware required. This is the honest ' +
          'starting state for most new households.',
      },
    },
  },
};

export const StewardedZero: Story = {
  name: 'StewardedZero',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-compute-tile
      .value=${freshNode}
      .profile=${profile('detail')}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Fresh node — null triptych (no system_metrics sample yet). ' +
          'Detail lens shows "—" for all values. The tile remains calm: ' +
          '"not yet known" is different from "something is wrong."',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// PolymorphicComparison — hub vs device side-by-side at Standard lens
// ---------------------------------------------------------------------------

export const PolymorphicComparison: Story = {
  name: 'PolymorphicComparison',
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
          gap: var(--el-space-md);
          max-inline-size: 800px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <div
      style="
        ${TILE_TOKENS_LIGHT}
        flex: 1;
        min-inline-size: 280px;
      "
    >
      <p
        style="
          font-family: var(--el-font-body);
          font-size: 0.875rem;
          color: var(--el-stone);
          margin-block: 0 var(--el-space-xs);
          letter-spacing: 0.04em;
          text-transform: uppercase;
          font-weight: 500;
        "
      >
        Household (hub aggregate)
      </p>
      <elohim-compute-tile
        .value=${householdHub}
        .profile=${profile('standard')}
      ></elohim-compute-tile>
    </div>

    <div
      style="
        ${TILE_TOKENS_LIGHT}
        flex: 1;
        min-inline-size: 280px;
      "
    >
      <p
        style="
          font-family: var(--el-font-body);
          font-size: 0.875rem;
          color: var(--el-stone);
          margin-block: 0 var(--el-space-xs);
          letter-spacing: 0.04em;
          text-transform: uppercase;
          font-weight: 500;
        "
      >
        Device (per-device drill-down)
      </p>
      <elohim-compute-tile
        .value=${matthewLaptop}
        .profile=${profile('standard')}
      ></elohim-compute-tile>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Polymorphic comparison — the same Standard lens rendering both input shapes ' +
          'side by side. The hub card shows the household aggregate; the device card shows ' +
          "Matthew's laptop as a single member. Same primitive, different data contracts, " +
          'coherent visual register. Proves the discriminated union renders consistently.',
      },
    },
  },
};
