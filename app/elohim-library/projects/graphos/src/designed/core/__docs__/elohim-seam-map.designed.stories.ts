/**
 * Library B designed story — elohim-seam-map
 *
 * Binds the Elohim brand tokens to `<elohim-seam-map>` via story-decorator
 * overrides. The primitive itself is NEVER modified — binding happens via
 * `--elohim-seam-map-*` CSS custom properties mapped to `--el-*` tokens.
 *
 * Design language:
 * - Dots are constellation nodes: amber = active/full, green-light = resting/partial,
 *   green-deep = host glow, stone-at-15% = dim/empty star.
 * - Golden-hour rule: amber accent border-block-start on the map container.
 * - Dark = constellation mode: night bg, starlight fg, dots glow.
 * - No danger red: state-color uses clay (earthy caution).
 * - Track bands: subtle warm tints via color-mix — t1 green-deep (notary trust),
 *   t2 sky (flow), t3 amber (edge), t4 plum (projection).
 *
 * component-architect follow-ups:
 * - `--elohim-seam-map-header-font` (or equivalent) does not exist; setting
 *   `font-family` on the wrapper cascades into the shadow DOM for body text but
 *   cannot target the header row independently with a display font (Fraunces).
 *   A `--elohim-seam-map-header-font` cssprop would unlock that binding.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';
import { DEFAULT_SEAM_MAP } from 'elohim-core';
import type { SeamMap } from 'elohim-core';

// ---------------------------------------------------------------------------
// Fixture — trimmed map (HARD budget constraint: ≤6 devices / ≤5 seams)
// ---------------------------------------------------------------------------

const trimmedMap: SeamMap = {
  ...DEFAULT_SEAM_MAP,
  devices: DEFAULT_SEAM_MAP.devices.slice(0, 6),
  seams: DEFAULT_SEAM_MAP.seams.slice(0, 5),
  routing: DEFAULT_SEAM_MAP.routing.slice(0, 4),
};

// ---------------------------------------------------------------------------
// Brand tokens — full palette (all tokens referenced below must be defined here)
// ---------------------------------------------------------------------------

const EL_TOKENS = `
  --el-green-deep:   #2D5F3B;
  --el-green-light:  #7FB069;
  --el-amber:        #D4A03E;
  --el-clay:         #B8664F;
  --el-cream:        #F5F0E8;
  --el-stone:        #6B6157;
  --el-sky:          #7BAFCB;
  --el-plum:         #6E4B6B;
  --el-starlight:    #E8E4D9;
  --el-night:        #0F1A12;
  --el-night-alt:    #1A1A2E;
  --el-font-ui:      'DM Sans', system-ui, sans-serif;
  --el-font-display: 'Fraunces', Georgia, serif;
`;

// ---------------------------------------------------------------------------
// Seam-map token blocks — Light (Linen register)
// ---------------------------------------------------------------------------

const SEAM_MAP_TOKENS_LIGHT = `
  --elohim-seam-map-bg:                   var(--el-cream);
  --elohim-seam-map-fg:                   var(--el-stone);
  --elohim-seam-map-border:               1px solid var(--el-stone);
  --elohim-seam-map-radius:               12px;
  --elohim-seam-map-padding:              1.5rem;
  --elohim-seam-map-header-bg:            color-mix(in oklch, var(--el-green-deep) 8%, var(--el-cream));
  --elohim-seam-map-header-fg:            var(--el-green-deep);
  --elohim-seam-map-group-bg:             color-mix(in oklch, var(--el-stone) 6%, var(--el-cream));
  --elohim-seam-map-group-fg:             var(--el-stone);
  --elohim-seam-map-row-bg:               var(--el-cream);
  --elohim-seam-map-row-bg-hover:         color-mix(in oklch, var(--el-amber) 8%, var(--el-cream));
  --elohim-seam-map-row-bg-selected:      color-mix(in oklch, var(--el-amber) 14%, var(--el-cream));
  --elohim-seam-map-row-border:           1px solid color-mix(in oklch, var(--el-stone) 20%, transparent);
  --elohim-seam-map-cell-full:            var(--el-amber);
  --elohim-seam-map-cell-partial:         var(--el-green-light);
  --elohim-seam-map-cell-host:            var(--el-green-deep);
  --elohim-seam-map-cell-none:            color-mix(in oklch, var(--el-stone) 15%, transparent);
  --elohim-seam-map-track-t1-bg:          color-mix(in oklch, var(--el-green-deep) 7%, var(--el-cream));
  --elohim-seam-map-track-t2-bg:          color-mix(in oklch, var(--el-sky) 7%, var(--el-cream));
  --elohim-seam-map-track-t3-bg:          color-mix(in oklch, var(--el-amber) 6%, var(--el-cream));
  --elohim-seam-map-track-t4-bg:          color-mix(in oklch, var(--el-plum) 6%, var(--el-cream));
  --elohim-seam-map-detail-bg:            color-mix(in oklch, var(--el-green-deep) 5%, var(--el-cream));
  --elohim-seam-map-detail-border:        1px solid color-mix(in oklch, var(--el-green-deep) 25%, transparent);
  --elohim-seam-map-detail-heading-color: var(--el-green-deep);
  --elohim-seam-map-detail-body-color:    var(--el-stone);
  --elohim-seam-map-detail-label-color:   color-mix(in oklch, var(--el-stone) 75%, transparent);
  --elohim-seam-map-meta-color:           color-mix(in oklch, var(--el-stone) 55%, transparent);
  --elohim-seam-map-state-color:          var(--el-clay);
`;

// ---------------------------------------------------------------------------
// Seam-map token blocks — Dark (Constellation register)
// ---------------------------------------------------------------------------

const SEAM_MAP_TOKENS_DARK = `
  --elohim-seam-map-bg:                   var(--el-night);
  --elohim-seam-map-fg:                   var(--el-starlight);
  --elohim-seam-map-border:               1px solid color-mix(in oklch, var(--el-starlight) 18%, transparent);
  --elohim-seam-map-radius:               12px;
  --elohim-seam-map-padding:              1.5rem;
  --elohim-seam-map-header-bg:            color-mix(in oklch, var(--el-green-deep) 30%, var(--el-night));
  --elohim-seam-map-header-fg:            color-mix(in oklch, var(--el-green-light) 85%, var(--el-starlight));
  --elohim-seam-map-group-bg:             color-mix(in oklch, var(--el-starlight) 5%, var(--el-night));
  --elohim-seam-map-group-fg:             color-mix(in oklch, var(--el-starlight) 70%, transparent);
  --elohim-seam-map-row-bg:               var(--el-night);
  --elohim-seam-map-row-bg-hover:         color-mix(in oklch, var(--el-amber) 10%, var(--el-night));
  --elohim-seam-map-row-bg-selected:      color-mix(in oklch, var(--el-amber) 18%, var(--el-night));
  --elohim-seam-map-row-border:           1px solid color-mix(in oklch, var(--el-starlight) 10%, transparent);
  --elohim-seam-map-cell-full:            var(--el-amber);
  --elohim-seam-map-cell-partial:         color-mix(in oklch, var(--el-starlight) 60%, var(--el-green-light));
  --elohim-seam-map-cell-host:            color-mix(in oklch, var(--el-green-light) 80%, var(--el-amber));
  --elohim-seam-map-cell-none:            color-mix(in oklch, var(--el-starlight) 12%, transparent);
  --elohim-seam-map-track-t1-bg:          color-mix(in oklch, var(--el-green-deep) 18%, var(--el-night));
  --elohim-seam-map-track-t2-bg:          color-mix(in oklch, var(--el-sky) 12%, var(--el-night));
  --elohim-seam-map-track-t3-bg:          color-mix(in oklch, var(--el-amber) 10%, var(--el-night));
  --elohim-seam-map-track-t4-bg:          color-mix(in oklch, var(--el-plum) 14%, var(--el-night));
  --elohim-seam-map-detail-bg:            color-mix(in oklch, var(--el-green-deep) 20%, var(--el-night));
  --elohim-seam-map-detail-border:        1px solid color-mix(in oklch, var(--el-green-light) 30%, transparent);
  --elohim-seam-map-detail-heading-color: var(--el-green-light);
  --elohim-seam-map-detail-body-color:    var(--el-starlight);
  --elohim-seam-map-detail-label-color:   color-mix(in oklch, var(--el-starlight) 55%, transparent);
  --elohim-seam-map-meta-color:           color-mix(in oklch, var(--el-starlight) 40%, transparent);
  --elohim-seam-map-state-color:          var(--el-clay);
`;

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Core/elohim-seam-map',
  parameters: {
    docs: {
      description: {
        component:
          'Library B — brand-bound view of `<elohim-seam-map>`. ' +
          'Tokens bound via decorator; primitive not modified. ' +
          'Constellation dot language: amber = active seam, green = resting, clay = caution state.',
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Story 1: Light (Linen)
// ---------------------------------------------------------------------------

export const Light: Story = {
  name: 'Light (Linen)',
  decorators: [
    story => html`
      <div
        style="${EL_TOKENS}${SEAM_MAP_TOKENS_LIGHT}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: 1.5rem;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-seam-map
      style="display: block; border-block-start: 2px solid var(--el-amber);"
      .data=${trimmedMap}
      .profile=${{
        lens: 'standard',
        theme: 'light',
        contrast: 'normal',
        locale: 'en',
        stimulus: 'still',
        textuality: 'textual',
        standings: [],
        lock: { kind: 'pilot' },
        origin: 'pilot',
      }}
    ></elohim-seam-map>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Linen register — warm earth tones. Amber dots mark active seams (constellation nodes). ' +
          'Track bands carry subtle warm tints: green-deep (T1 notary trust), sky (T2 substrate flow), ' +
          'amber (T3 edge), plum (T4 doorway projection). Golden-hour amber border-block-start.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Story 2: Dark (Constellation)
// ---------------------------------------------------------------------------

export const Dark: Story = {
  name: 'Dark (Constellation)',
  decorators: [
    story => html`
      <div
        style="${EL_TOKENS}${SEAM_MAP_TOKENS_DARK}
          font-family: var(--el-font-ui);
          background: var(--el-night);
          color: var(--el-starlight);
          color-scheme: dark;
          padding: 1.5rem;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-seam-map
      style="display: block; border-block-start: 2px solid var(--el-amber);"
      .data=${trimmedMap}
      .profile=${{
        lens: 'standard',
        theme: 'dark',
        contrast: 'normal',
        locale: 'en',
        stimulus: 'still',
        textuality: 'textual',
        standings: [],
        lock: { kind: 'pilot' },
        origin: 'pilot',
      }}
    ></elohim-seam-map>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Constellation mode — night sky with amber-glow active nodes and starlight resting nodes. ' +
          'Track bands are warm tints over the night base. ' +
          'State-color uses clay (earthy caution) rather than danger red. ' +
          'color-scheme: dark set on the wrapper.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Story 3: Light (Linen) · seam selected  [MANDATORY — detail-panel detector]
// ---------------------------------------------------------------------------

export const LightWithPanel: Story = {
  name: 'Light (Linen) · seam selected',
  decorators: [
    story => html`
      <div
        style="${EL_TOKENS}${SEAM_MAP_TOKENS_LIGHT}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: 1.5rem;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-seam-map
      style="display: block; border-block-start: 2px solid var(--el-amber);"
      .data=${trimmedMap}
      .profile=${{
        lens: 'standard',
        theme: 'light',
        contrast: 'normal',
        locale: 'en',
        stimulus: 'still',
        textuality: 'textual',
        standings: [],
        lock: { kind: 'pilot' },
        origin: 'pilot',
      }}
    ></elohim-seam-map>
  `,
  play: async ({ canvasElement }: { canvasElement: HTMLElement }) => {
    await new Promise(resolve => setTimeout(resolve, 100));
    const el = canvasElement.querySelector('elohim-seam-map');
    if (!el?.shadowRoot) return;
    const firstCell = el.shadowRoot.querySelector<HTMLElement>('.seam-title-cell');
    if (!firstCell) return;
    firstCell.focus();
    firstCell.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, composed: true })
    );
  },
  parameters: {
    docs: {
      description: {
        story:
          'Light theme with the detail panel opened via keyboard (Enter on first seam row). ' +
          'Light theme is the authoritative detector: dark color-scheme fallbacks can mask unbound ' +
          'detail-* properties. All 6 detail-* bindings (bg, border, heading-color, body-color, ' +
          'label-color, state-color) are exercised here. ' +
          'Verifies: warm linen panel, green-deep headings, clay state-color (no danger red).',
      },
    },
  },
};
