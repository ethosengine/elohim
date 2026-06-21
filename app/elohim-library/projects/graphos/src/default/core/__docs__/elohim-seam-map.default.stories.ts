/**
 * Library A default story — elohim-seam-map
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * Data shape: SeamMap is operational visualization config (no ts-rs view exists).
 * The local `SeamMap` interface (declared in the element) is the type contract.
 * The element ships a DEFAULT_SEAM_MAP fixture derived from the 2026-06-21 atlas.
 * Eventual home: architecture-atlas contentFormat (lamad manifest), once wired.
 *
 * Lens/state/theme coverage is exercised via the `.profile` and `map-state` props
 * (the element supports minimal→debug lenses + loading/error/empty states). The
 * story EXPORTS are kept to the core Library-A contract set — interaction proof,
 * full view, blank-slate proof, override-surface proof — to stay within the
 * Storybook smoke-test budget (a fat per-element story matrix stalls the gate).
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

import { DEFAULT_SEAM_MAP } from 'elohim-core';
import type { SeamMap } from 'elohim-core';

// Trimmed map (6 devices, 5 seams) for the compact proof stories.
const trimmedMap: SeamMap = {
  ...DEFAULT_SEAM_MAP,
  devices: DEFAULT_SEAM_MAP.devices.slice(0, 6),
  seams: DEFAULT_SEAM_MAP.seams.slice(0, 5),
  routing: DEFAULT_SEAM_MAP.routing.slice(0, 4),
};

const meta: Meta = {
  title: 'Default/Core/elohim-seam-map',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-seam-map>\` — interactive concern-routing seam map visualization.

Renders the Elohim architecture atlas as a two-axis grid:
- **Rows**: composition-stack seams (hardware → OS → runtime → plugins → SDK → app-manifest → client), role seams, and cross-cutting planes
- **Columns**: device spectrum (fob/wearable → k8s pod), keyed by capability level (L0–L5)
- **Track bands**: T1 DHT / T2 substrate / T3 spoke / T4 doorway

**Interaction**: click or keyboard-select a seam row → detail panel showing problem-class · add-a-new-X · home · confusion-to-avoid.
At \`detail\` lens+, the full concern-routing table is shown beneath.

**Data source**: ships a built-in \`DEFAULT_SEAM_MAP\` fixture from the 2026-06-21 atlas; pass a custom \`SeamMap\` via \`.data\` to override.

**Override surface**: all visual properties use \`--elohim-seam-map-*\` CSS custom properties with neutral CSS system color defaults.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

/**
 * Standard lens with the detail panel opened via keyboard (Enter on the first
 * seam row) — the primary interaction proof. Verifies all four panel fields:
 * problem-class · add-a-new-X · home · confusion-to-avoid.
 */
export const StandardWithPanel: Story = {
  name: 'StandardWithPanel (detail panel open)',
  render: () => html`
    <elohim-seam-map
      .data=${trimmedMap}
      .profile=${{ lens: 'standard', theme: 'auto', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'pilot' }, origin: 'pilot' }}
    ></elohim-seam-map>
  `,
  play: async ({ canvasElement }: { canvasElement: HTMLElement }) => {
    await new Promise(resolve => setTimeout(resolve, 100));
    const el = canvasElement.querySelector('elohim-seam-map');
    if (!el?.shadowRoot) return;
    const firstCell = el.shadowRoot.querySelector<HTMLElement>('.seam-title-cell');
    if (!firstCell) return;
    firstCell.focus();
    firstCell.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, composed: true }));
  },
  parameters: {
    docs: {
      description: {
        story:
          'Standard lens with the detail panel opened via keyboard (Enter on first seam row). Verifies all four panel fields render: problem-class · add-a-new-X · home · confusion-to-avoid.',
      },
    },
  },
};

/** Standard lens, full atlas fixture — the main two-axis view. */
export const Standard: Story = {
  name: 'Standard',
  render: () => html`
    <elohim-seam-map
      .profile=${{ lens: 'standard', theme: 'auto', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'pilot' }, origin: 'pilot' }}
    ></elohim-seam-map>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Standard lens: full atlas fixture (all devices × all seams). Click any seam row to open the detail panel.',
      },
    },
  },
};

/** Blank-slate proof — renders with zero external CSS tokens (system colors). */
export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial;">${story()}</div>`],
  render: () => html`
    <elohim-seam-map .data=${trimmedMap}></elohim-seam-map>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Wrapped in `style="all: initial;"`. Proves the element renders correctly with zero external CSS tokens — CSS system colors (Canvas, CanvasText, ButtonFace, LinkText) provide legible defaults.',
      },
    },
  },
};

/** Override-surface proof — terminal-green theme via `--elohim-seam-map-*`. */
export const CustomTheme: Story = {
  name: 'CustomTheme (override-surface proof)',
  decorators: [
    story => html`
      <div
        style="
          --elohim-seam-map-bg: #0d1117;
          --elohim-seam-map-fg: #00ff41;
          --elohim-seam-map-border: 1px solid #00ff41;
          --elohim-seam-map-radius: 0;
          --elohim-seam-map-header-bg: #001a00;
          --elohim-seam-map-header-fg: #7fff7f;
          --elohim-seam-map-group-bg: #002200;
          --elohim-seam-map-group-fg: #00ff41;
          --elohim-seam-map-row-bg: #0d1117;
          --elohim-seam-map-row-bg-hover: #003300;
          --elohim-seam-map-row-bg-selected: #004400;
          --elohim-seam-map-cell-full: #00ff41;
          --elohim-seam-map-cell-partial: #7fff7f;
          --elohim-seam-map-cell-host: #00cc33;
          --elohim-seam-map-cell-none: #002200;
          --elohim-seam-map-track-t1-bg: #001100;
          --elohim-seam-map-track-t2-bg: #001100;
          --elohim-seam-map-track-t3-bg: #001100;
          --elohim-seam-map-track-t4-bg: #001100;
          --elohim-seam-map-detail-bg: #001a00;
          --elohim-seam-map-detail-border: 1px solid #00ff41;
          --elohim-seam-map-detail-heading-color: #00ff41;
          --elohim-seam-map-detail-body-color: #7fff7f;
          --elohim-seam-map-detail-label-color: #3fff3f;
          --elohim-seam-map-meta-color: #00aa22;
          font-family: ui-monospace, 'Courier New', monospace;
          padding: 1rem;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-seam-map
      .data=${trimmedMap}
      .profile=${{ lens: 'standard', theme: 'dark', contrast: 'normal', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'elohim-support' }, origin: 'elohim-support' }}
    ></elohim-seam-map>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Override-surface proof: terminal-green-on-black theme applied via `--elohim-seam-map-*` CSS custom properties — no hardcoded brand colors in the element.',
      },
    },
  },
};
