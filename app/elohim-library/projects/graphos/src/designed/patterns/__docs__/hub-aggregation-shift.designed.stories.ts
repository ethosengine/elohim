/**
 * Library B pattern story — Hub Aggregation Shift
 *
 * Title: Designed/Patterns/Hub-Aggregation-Shift
 *
 * The protocol's signature moment: a household that begins as one laptop
 * participating alone, then slides a blade into the rack, and becomes a
 * three-device hub. Same household identity, same hub-id, dramatically more
 * capacity.
 *
 * Three named stories:
 *   Before      — Matthew's laptop as a hub-of-one. The degenerate case.
 *                 One star in the constellation. Complete, not diminished.
 *   After       — The same household, now three devices aggregated. The hub
 *                 identity is stable; only the aggregate changed.
 *   SideBySide  — Both rendered together with captions explaining the shift.
 *                 The hub-optional-floor principle made visible.
 *
 * Brand voice throughout:
 *   - "household" not "user"
 *   - "what your household can hold" not "storage"
 *   - "capacity" not "quota"
 *   - "neighbors" not "peers" in user-facing copy
 *   - Protocol vocabulary (hub, blade, provision, steward) used as proper nouns
 *
 * Motion discipline:
 *   maxStimulus = still. No motion introduced. The shift between Before and
 *   After is told through the static contrast of the two states, not through
 *   transition animation. Stillness is the floor; the comparison is the story.
 *
 * Sources of truth:
 *   Types: ComputeTileHubValue / ComputeTileDeviceValue from elohim-core,
 *          which type-alias the canonical `HubComputeAggregateView` and
 *          `DeviceSummary` views generated from the Rust ts-rs boundary.
 *   Brand tokens: --el-* from design spec §14, declared inline as EL_TOKENS
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

import type { ComputeTileHubValue, ComputeTileDeviceValue } from 'elohim-core';

// ---------------------------------------------------------------------------
// Brand tokens (inline — see designed composition story module JSDoc)
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

// ---------------------------------------------------------------------------
// Shared capability profile
// ---------------------------------------------------------------------------

function profile(lens: string = 'standard', locale: string = 'en') {
  return {
    lens,
    theme: 'auto',
    contrast: 'auto',
    locale,
    stimulus: 'still',
    textuality: 'symbolic',
    standings: [],
    lock: { kind: 'pilot' },
    origin: 'pilot',
  };
}

// ---------------------------------------------------------------------------
// Fixtures — the same household at two moments in time
// ---------------------------------------------------------------------------

/**
 * BEFORE: Matthew's household as a degenerate hub-of-one.
 * One laptop. The floor of participation — present, but alone.
 * ~10 GB total capacity.
 */
const matthewHouseholdBefore: ComputeTileHubValue = {
  kind: 'hub',
  hubId: 'hub:household:matthew',
  hubKind: 'computed',          // single-device participant — no notarized binding yet
  displayLabel: "Matthew's household",
  compute: {
    free: 2_147_483_648n,       //  2 GB free
    used: 5_368_709_120n,       //  5 GB used
    stewarded: 0n,              //  no REA commitments yet — fresh participant
  },
  memberDeviceCount: 1,
};

/**
 * The laptop that IS Matthew's hub-of-one in the Before state.
 * Shown in the SideBySide as supplementary context.
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
    stewarded: 0n,
  },
};

/**
 * AFTER: The same household hub — three devices now contributing.
 * Matthew's laptop + a blade slid into the rack + Jessica's home server.
 * Same hubId. The identity is stable; the aggregate has grown.
 * Hub-kind upgrades from `computed` to `dwelling` once the household binding
 * is notarized.
 * ~100 GB total capacity.
 */
const matthewHouseholdAfter: ComputeTileHubValue = {
  kind: 'hub',
  hubId: 'hub:household:matthew',           // same hub identity
  hubKind: 'dwelling',                      // notarized household binding now in place
  displayLabel: "Matthew's household",      // same display name
  compute: {
    free: 53_687_091_200n,                  //  50 GB free (blade contributes most)
    used: 53_687_091_200n,                  //  50 GB used across all three
    stewarded: 21_474_836_480n,             // 20 GB stewarded for neighbors
  },
  memberDeviceCount: 3,
};

// ---------------------------------------------------------------------------
// Shared prose styles
// ---------------------------------------------------------------------------

const PROSE_HEADING = `
  font-family: var(--el-font-display);
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--el-green-deep);
  margin-block: 0 var(--el-space-xs);
  line-height: 1.3;
`;

const PROSE_BODY = `
  font-family: var(--el-font-body);
  font-size: 0.9375rem;
  color: var(--el-stone);
  line-height: 1.65;
  margin-block: 0 var(--el-space-sm);
`;

const PROSE_LABEL = `
  font-family: var(--el-font-ui);
  font-size: 0.75rem;
  color: var(--el-stone);
  letter-spacing: 0.06em;
  text-transform: uppercase;
  font-weight: 600;
  margin-block: 0 var(--el-space-xs);
`;

const PROSE_CAPTION = `
  font-family: var(--el-font-body);
  font-size: 0.8125rem;
  color: var(--el-stone);
  line-height: 1.5;
  margin-block: var(--el-space-xs) 0;
  font-style: italic;
`;

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Patterns/Hub-Aggregation-Shift',
  parameters: {
    docs: {
      description: {
        component: `
## Sliding a blade into the rack

The hub-aggregation shift is the moment a household's capacity visibly grows — not because
anyone changed a setting, but because a new device joined the household and the hub absorbed it.

**The protocol makes a promise:** the hub identity is stable. "Matthew's household" before and
after the blade slides in is the same hub, the same covenant, the same neighbors. Only the
aggregate changed.

Three stories show this arc:

- **Before** — one laptop, hub-of-one. The floor of participation. Enough to begin.
- **After** — three devices, same household hub. The aggregate swells with Harvest Gold.
- **Side by Side** — both states rendered together. The question the pattern answers:
  *"What does a household look like as it grows?"*

### Brand voice in this pattern

At the \`simple\` and \`minimal\` lens, visible copy describes capacity in plain household
language: "what your household can hold," not "storage quota." Numbers appear at
\`standard\` and above — the grandma surface never shows bytes.

### Motion discipline

\`maxStimulus: still\`. The shift is told through static contrast, not through transition
animation. Stillness is the Sabbath default. The comparison IS the story.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Before — hub-of-one, one laptop
// ---------------------------------------------------------------------------

export const Before: Story = {
  name: 'Before',
  decorators: [
    (story: () => unknown) => html`
      <div
        style="
          ${EL_TOKENS}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: var(--el-space-xl);
          max-inline-size: 480px;
          box-shadow: var(--el-shadow-soft);
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <div>
      <p style="${PROSE_HEADING}">One laptop, one household.</p>
      <p style="${PROSE_BODY}">
        Matthew is new to the commons. He joined last week — just his laptop,
        nothing else. That's enough. The protocol doesn't require hardware.
        A single device makes you a full participant: you can share what you have,
        receive what you need, and your household's capacity is honestly yours.
      </p>

      <p style="${PROSE_LABEL}">Matthew's household · 1 device</p>

      <div style="${TILE_TOKENS_LIGHT}">
        <elohim-compute-tile
          .value=${matthewHouseholdBefore}
          .profile=${profile('standard')}
        ></elohim-compute-tile>
      </div>

      <p style="${PROSE_CAPTION}">
        2 GB free — the floor. Not much for stewarding a neighbor's quilt archive yet,
        but enough to participate and earn trust.
      </p>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Before state — Matthew\'s household as a degenerate hub-of-one. ' +
          'One laptop. ~7 GB total. Zero stewarded bytes (no REA commitments yet). ' +
          'Symbolic gauge shows the capacity proportionally in Harvest Gold. ' +
          'The hub-optional-floor principle: a laptop alone is a full protocol participant.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// After — three devices, same hub identity
// ---------------------------------------------------------------------------

export const After: Story = {
  name: 'After',
  decorators: [
    (story: () => unknown) => html`
      <div
        style="
          ${EL_TOKENS}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: var(--el-space-xl);
          max-inline-size: 480px;
          box-shadow: var(--el-shadow-soft);
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <div>
      <p style="${PROSE_HEADING}">Three devices, one household.</p>
      <p style="${PROSE_BODY}">
        Three months later, Matthew slid a blade into the rack. Jessica's home server
        joined the following week. The household is the same — same name, same neighbors,
        same covenant. The capacity is different. The commons grew.
      </p>

      <p style="${PROSE_LABEL}">Matthew's household · 3 devices</p>

      <div style="${TILE_TOKENS_LIGHT}">
        <elohim-compute-tile
          .value=${matthewHouseholdAfter}
          .profile=${profile('standard')}
        ></elohim-compute-tile>
      </div>

      <p style="${PROSE_CAPTION}">
        50 GB free, 20 GB stewarded for neighbors. The gauge shows the same shape —
        the same household, a fuller promise.
      </p>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'After state — same hub identity (hub:household:matthew), now three member devices. ' +
          '~100 GB total, 20 GB stewarded for neighbors. The hub-id and displayLabel are ' +
          'unchanged: the household identity is stable across the aggregation shift. ' +
          'The Harvest Gold gauge fill is visually wider — more capacity, same warmth.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// SideBySide — the full arc
// ---------------------------------------------------------------------------

export const SideBySide: Story = {
  name: 'SideBySide',
  decorators: [
    (story: () => unknown) => html`
      <div
        style="
          ${EL_TOKENS}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: var(--el-space-xl);
          max-inline-size: 900px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <div>
      <!-- Page heading -->
      <h2
        style="
          ${PROSE_HEADING}
          font-size: 1.5rem;
          margin-block-end: var(--el-space-xs);
        "
      >
        Sliding a blade into the rack
      </h2>
      <p style="${PROSE_BODY}">
        The same household hub — before and after a new device joined.
        The hub identity is stable. Only the capacity changed.
      </p>

      <!-- Two-column comparison -->
      <div
        style="
          display: flex;
          flex-wrap: wrap;
          gap: var(--el-space-lg);
          margin-block-start: var(--el-space-md);
        "
      >

        <!-- Before column -->
        <div style="flex: 1; min-inline-size: 260px;">
          <p style="${PROSE_LABEL}">Before · 1 device</p>

          <div style="${TILE_TOKENS_LIGHT}">
            <elohim-compute-tile
              .value=${matthewHouseholdBefore}
              .profile=${profile('standard')}
            ></elohim-compute-tile>
          </div>

          <p style="${PROSE_CAPTION}">
            Matthew's laptop only. 2 GB free. A full participant —
            just a quieter one.
          </p>

          <!-- Device drill-down for context -->
          <p
            style="
              ${PROSE_LABEL}
              margin-block-start: var(--el-space-md);
              color: var(--el-stone);
              opacity: 0.7;
            "
          >
            Device drill-down
          </p>
          <div
            style="
              ${TILE_TOKENS_LIGHT}
              --elohim-compute-tile-padding: var(--el-space-sm) var(--el-space-md);
            "
          >
            <elohim-compute-tile
              .value=${matthewLaptop}
              .profile=${profile('detail')}
            ></elohim-compute-tile>
          </div>
        </div>

        <!-- Separator -->
        <div
          aria-hidden="true"
          style="
            display: flex;
            align-items: center;
            padding-block: var(--el-space-md);
            color: var(--el-amber);
            font-size: 1.5rem;
            line-height: 1;
            flex-shrink: 0;
            align-self: flex-start;
            margin-block-start: 2rem;
          "
        >
          →
        </div>

        <!-- After column -->
        <div style="flex: 1; min-inline-size: 260px;">
          <p style="${PROSE_LABEL}">After · 3 devices</p>

          <div style="${TILE_TOKENS_LIGHT}">
            <elohim-compute-tile
              .value=${matthewHouseholdAfter}
              .profile=${profile('standard')}
            ></elohim-compute-tile>
          </div>

          <p style="${PROSE_CAPTION}">
            Laptop + blade + Jessica's server. 50 GB free.
            The commons deepened.
          </p>

          <!-- Hub detail for context -->
          <p
            style="
              ${PROSE_LABEL}
              margin-block-start: var(--el-space-md);
              color: var(--el-stone);
              opacity: 0.7;
            "
          >
            Detail lens
          </p>
          <div
            style="
              ${TILE_TOKENS_LIGHT}
              --elohim-compute-tile-padding: var(--el-space-sm) var(--el-space-md);
            "
          >
            <elohim-compute-tile
              .value=${matthewHouseholdAfter}
              .profile=${profile('detail')}
            ></elohim-compute-tile>
          </div>
        </div>

      </div><!-- end two-column -->

      <!-- Footer provenance note -->
      <p
        style="
          font-family: var(--el-font-ui);
          font-size: 0.8125rem;
          color: var(--el-stone);
          margin-block-start: var(--el-space-lg);
          padding-block-start: var(--el-space-sm);
          border-block-start: 1px solid rgba(107, 97, 87, 0.15);
          line-height: 1.5;
        "
      >
        Hub identity: <code style="font-family: var(--el-font-mono); font-size: 0.875em;">hub:household:matthew</code>
        — unchanged across the aggregation shift. The hub-id is the covenant; the member count is a current reading.
      </p>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Side-by-side comparison — the full arc of the hub-aggregation shift rendered in one view. ' +
          'Left: hub-of-one (Matthew\'s laptop only). Right: hub-of-three (same household, blade + server added). ' +
          'The hub-id is the same in both columns — the household identity is stable. ' +
          'The device drill-down (Before) and Detail lens (After) show how the per-device and ' +
          'aggregate surfaces compose: same primitive, different input shapes, coherent register. ' +
          'No motion — the comparison is the story.',
      },
    },
  },
};
