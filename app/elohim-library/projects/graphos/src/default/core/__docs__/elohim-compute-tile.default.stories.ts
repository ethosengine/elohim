/**
 * Library A default story — elohim-compute-tile
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * NOTE: The hub-kind input shape uses a locally-typed fixture because no ts-rs
 * `HubComputeAggregateView` exists yet. The device-kind `compute` field uses
 * `ComputeTriptychShape` from `elohim-core` (structurally compatible with
 * `ComputeTriptych` from `@elohim/storage-client`). Follow-up: rust-architect
 * should define `HubComputeAggregateView` so the hub fixture can use the
 * generated type.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

import type {
  ComputeTileValue,
  ComputeTileHubValue,
  ComputeTileDeviceValue,
} from 'elohim-core';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/**
 * Household hub aggregate — the primary UX surface per the hub-compute-aggregate-primary
 * design note. Three member devices contributing to a single pool.
 *
 * Local interface (no ts-rs HubComputeAggregateView yet — see module JSDoc).
 */
const householdHub: ComputeTileHubValue = {
  kind: 'hub',
  hubId: 'hub:household:matthew-jessica-james',
  displayLabel: "Matthew's household",
  free: 5_368_709_120n, //  5 GB
  used: 10_737_418_240n, // 10 GB
  stewarded: 12_884_901_888n, // 12 GB
  memberDeviceCount: 3,
};

/**
 * Per-device drill-down — Matthew's laptop.
 * The `compute` field is structurally compatible with `ComputeTriptych` from
 * `@elohim/storage-client`; assign directly without cast.
 */
const matthewLaptop: ComputeTileDeviceValue = {
  kind: 'device',
  peerId: 'peer:12D3KooWMatthewLaptopHashFakeButWellFormed',
  displayName: "Matthew's laptop",
  archetype: 'desktop',
  online: true,
  compute: {
    free: 2_147_483_648n, // 2 GB
    used: 5_368_709_120n, // 5 GB
    stewarded: 3_221_225_472n, // 3 GB
  },
};

/** Hub-of-one — degenerate single-device hub; the hub-optional-floor design pattern. */
const hubOfOne: ComputeTileHubValue = {
  kind: 'hub',
  hubId: 'hub:household:solo-device',
  displayLabel: 'Solo device hub',
  free: 1_073_741_824n,
  used: 2_147_483_648n,
  stewarded: 0n,
  memberDeviceCount: 1,
};

/** Stewarded-zero — fresh node with no REA commitments yet. */
const freshNode: ComputeTileHubValue = {
  kind: 'hub',
  hubId: 'hub:household:fresh-node',
  displayLabel: 'Fresh node',
  free: null,
  used: null,
  stewarded: null,
  memberDeviceCount: 1,
};

/** Offline device — compute is null because no metrics sample available. */
const offlineNode: ComputeTileDeviceValue = {
  kind: 'device',
  peerId: 'peer:12D3KooWOfflineAlphaNodeHashFake00000000',
  displayName: 'Alpha node',
  archetype: 'node',
  online: false,
  compute: null,
};

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-compute-tile',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-compute-tile>\` — polymorphic compute-capacity tile.

Renders the compute triptych (free / used / stewarded bytes) for either a hub aggregate
or a single device, with **progressive disclosure** matching the viewer's capability lens.

Accepts two input shapes via \`.value\`:
- \`kind: 'hub'\` — aggregated household/collective capacity (primary UX surface)
- \`kind: 'device'\` — per-device drill-down (secondary UX surface)

**Override surface:** all visual properties are bound via \`--elohim-compute-tile-*\` CSS custom
properties with neutral CSS system color defaults (\`Canvas\`, \`CanvasText\`, \`ButtonFace\`).
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Lens stories — capability lens coverage
// ---------------------------------------------------------------------------

export const Minimal: Story = {
  name: 'Minimal',
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${{ lens: 'minimal', theme: 'auto', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'pilot' }, origin: 'pilot' }}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Minimal lens: qualitative summary only. "You have enough." when free > 1 GB. No numbers exposed — the grandma surface.',
      },
    },
  },
};

export const Simple: Story = {
  name: 'Simple',
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${{ lens: 'simple', theme: 'auto', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'pilot' }, origin: 'pilot' }}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: { story: 'Simple lens: "5 GB free" — just free bytes, no total shown.' },
    },
  },
};

export const Standard: Story = {
  name: 'Standard',
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${{ lens: 'standard', theme: 'auto', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'pilot' }, origin: 'pilot' }}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Standard lens: "5 GB of 15 GB free" — the clean two-tuple default for adult pilots.',
      },
    },
  },
};

export const Detail: Story = {
  name: 'Detail',
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${{ lens: 'detail', theme: 'auto', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'pilot' }, origin: 'pilot' }}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Detail lens: full triptych revealed — free / total + stewarded + used breakdown.',
      },
    },
  },
};

export const Debug: Story = {
  name: 'Debug',
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      last-sample-at="2026-05-20T10:00:00Z"
      .profile=${{ lens: 'debug', theme: 'auto', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'elohim-support' }, origin: 'elohim-support' }}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Debug lens: triptych + hub-id + member count + freshness timestamp.',
      },
    },
  },
};

export const Trace: Story = {
  name: 'Trace',
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      last-sample-at="2026-05-20T10:00:00Z"
      source-route="aggregate_stewarded_bytes_by_peer + system_metrics → ClusterView"
      .profile=${{ lens: 'trace', theme: 'auto', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'elohim-support' }, origin: 'elohim-support' }}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Trace lens: debug info + source-route annotation showing which DHT query and REA aggregation produced these numbers.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Input-shape stories
// ---------------------------------------------------------------------------

export const HubAggregate: Story = {
  name: 'HubAggregate',
  render: () => html`
    <elohim-compute-tile .value=${householdHub}></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Hub-kind input: household aggregate (3 member devices). Standard lens default.',
      },
    },
  },
};

export const PerDevice: Story = {
  name: 'PerDevice',
  render: () => html`
    <elohim-compute-tile .value=${matthewLaptop}></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story: "Device-kind input: Matthew's laptop drill-down. Standard lens default.",
      },
    },
  },
};

export const HubOfOne: Story = {
  name: 'HubOfOne',
  render: () => html`
    <elohim-compute-tile .value=${hubOfOne}></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Degenerate hub-of-one: single device participating as its own hub. Per hub-optional-floor design — a laptop alone is a full protocol participant.',
      },
    },
  },
};

export const StewardedZero: Story = {
  name: 'StewardedZero',
  render: () => html`
    <elohim-compute-tile
      .value=${freshNode}
      .profile=${{ lens: 'detail', theme: 'auto', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'pilot' }, origin: 'pilot' }}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Fresh node with null triptych (no system_metrics sample yet). Detail lens shows "—" for all values.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Symbolic textuality
// ---------------------------------------------------------------------------

export const SymbolicGauge: Story = {
  name: 'SymbolicGauge',
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${{ lens: 'standard', theme: 'auto', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'symbolic', standings: [], lock: { kind: 'pilot' }, origin: 'pilot' }}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: { story: 'Symbolic textuality: capacity bar shown alongside the text value.' },
    },
  },
};

// ---------------------------------------------------------------------------
// Content-state stories
// ---------------------------------------------------------------------------

export const Loading: Story = {
  name: 'Loading',
  render: () => html`<elohim-compute-tile tile-state="loading"></elohim-compute-tile>`,
  parameters: {
    docs: { description: { story: 'Loading state: waiting for compute data from the backend.' } },
  },
};

export const Error: Story = {
  name: 'Error',
  render: () => html`<elohim-compute-tile tile-state="error"></elohim-compute-tile>`,
  parameters: {
    docs: { description: { story: 'Error state: compute data could not be loaded.' } },
  },
};

export const Stale: Story = {
  name: 'Stale',
  render: () => html`<elohim-compute-tile tile-state="stale"></elohim-compute-tile>`,
  parameters: {
    docs: {
      description: {
        story: 'Stale state: ContentCertainty=stale — data was loaded but may be out of date.',
      },
    },
  },
};

export const Offline: Story = {
  name: 'Offline',
  render: () => html`<elohim-compute-tile tile-state="offline"></elohim-compute-tile>`,
  parameters: {
    docs: { description: { story: 'Offline state: device is offline, compute unavailable.' } },
  },
};

export const Empty: Story = {
  name: 'Empty',
  render: () => html`<elohim-compute-tile tile-state="empty"></elohim-compute-tile>`,
  parameters: {
    docs: { description: { story: 'Empty state: no compute data available for this node yet.' } },
  },
};

// ---------------------------------------------------------------------------
// Offline device (per-device kind with online: false)
// ---------------------------------------------------------------------------

export const OfflineDevice: Story = {
  name: 'OfflineDevice',
  render: () => html`
    <elohim-compute-tile .value=${offlineNode}></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story: "Device is offline (status-dot shows offline). No compute sample available (device.compute is null).",
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
      <div style="background: #1a1a1a; padding: 1.5rem; color-scheme: dark;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${{ lens: 'standard', theme: 'dark', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'pilot' }, origin: 'pilot' }}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: { story: 'Dark theme canary — tile with dark background wrapper, CSS system colors adapt.' },
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
    <elohim-compute-tile
      .value=${householdHub}
      .profile=${{ lens: 'detail', theme: 'auto', contrast: 'auto', locale: 'he', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'pilot' }, origin: 'pilot' }}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story: 'RTL canary: he-IL locale in an RTL container. Logical CSS properties ensure correct layout.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Blank-slate proof
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial;">${story()}</div>`],
  render: () => html`
    <elohim-compute-tile .value=${householdHub}></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Wrapped in `style="all: initial;"`. Proves the element renders correctly with zero external CSS tokens. CSS system colors provide legible defaults.',
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
          --elohim-compute-tile-bg: #0d1117;
          --elohim-compute-tile-fg: #00ff41;
          --elohim-compute-tile-border: 1px solid #00ff41;
          --elohim-compute-tile-radius: 0;
          --elohim-compute-tile-label-color: #7fff7f;
          --elohim-compute-tile-value-color: #00ff41;
          --elohim-compute-tile-secondary-color: #7fff7f;
          --elohim-compute-tile-meta-color: #3fff3f;
          --elohim-compute-tile-gauge-fill: #00ff41;
          --elohim-compute-tile-gauge-track: #001a00;
          font-family: ui-monospace, 'Courier New', monospace;
          padding: 1rem;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-compute-tile
      .value=${householdHub}
      last-sample-at="2026-05-20T10:00:00Z"
      source-route="aggregate_stewarded_bytes_by_peer + system_metrics"
      .profile=${{ lens: 'trace', theme: 'dark', contrast: 'normal', locale: 'en', stimulus: 'still', textuality: 'symbolic', standings: [], lock: { kind: 'elohim-support' }, origin: 'elohim-support' }}
    ></elohim-compute-tile>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Override-surface proof: terminal-green-on-black theme applied via `--elohim-compute-tile-*` CSS custom properties. Demonstrates the override surface is honest — no hardcoded brand colors in the element.',
      },
    },
  },
};
