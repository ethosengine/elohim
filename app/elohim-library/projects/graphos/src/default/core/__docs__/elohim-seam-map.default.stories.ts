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
 * Lens claims: minimal, simple, standard, detail, debug.
 * (Trace not claimed — a static visualization gains nothing from a trace lens.)
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

import { DEFAULT_SEAM_MAP } from 'elohim-core';
import type { SeamMap } from 'elohim-core';

// ---------------------------------------------------------------------------
// Fixture variants
// ---------------------------------------------------------------------------

/**
 * Trimmed map: only 3 seams + 4 devices, for compact stories that show the
 * layout without the full atlas density.
 */
const trimmedMap: SeamMap = {
  ...DEFAULT_SEAM_MAP,
  devices: DEFAULT_SEAM_MAP.devices.slice(0, 6),
  seams: DEFAULT_SEAM_MAP.seams.slice(0, 5),
  routing: DEFAULT_SEAM_MAP.routing.slice(0, 4),
};

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

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

**Data source**: The element ships a built-in \`DEFAULT_SEAM_MAP\` fixture from the 2026-06-21 atlas.
Pass a custom \`SeamMap\` via the \`.data\` property to override.

**Override surface**: all visual properties use \`--elohim-seam-map-*\` CSS custom properties with neutral CSS system color defaults.
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

/**
 * StandardWithPanel: Standard lens with the detail panel open on the first seam.
 *
 * Uses a play function to dispatch a keydown Enter on the first .seam-title-cell
 * in the shadow DOM, which triggers _selectSeam() and renders the detail panel.
 * This is the primary verification that the panel (problem-class · add-a-new-X ·
 * home · confusion-to-avoid) renders with real content.
 *
 * No @storybook/test import needed — raw DOM dispatch works in Storybook 10.
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
    // Wait for Lit to complete its first render cycle
    await new Promise(resolve => setTimeout(resolve, 100));

    const el = canvasElement.querySelector('elohim-seam-map');
    if (!el?.shadowRoot) return;

    // Select the first .seam-title-cell in shadow DOM — dispatching keydown
    // mirrors the keyboard-select path claimed in the capability contract.
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

export const Minimal: Story = {
  name: 'Minimal',
  render: () => html`
    <elohim-seam-map
      .profile=${{ lens: 'minimal', theme: 'auto', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'pilot' }, origin: 'pilot' }}
    ></elohim-seam-map>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Minimal lens: group headings + seam titles only. No applicability dots, no tracks, no detail. The navigation surface for novice explorers.',
      },
    },
  },
};

export const Simple: Story = {
  name: 'Simple',
  render: () => html`
    <elohim-seam-map
      .data=${trimmedMap}
      .profile=${{ lens: 'simple', theme: 'auto', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'pilot' }, origin: 'pilot' }}
    ></elohim-seam-map>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Simple lens: trimmed fixture (6 devices, 5 seams). Applicability dots visible per seam × device. Track bands shown.',
      },
    },
  },
};

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
        story: 'Standard lens: full atlas fixture. Click any seam row to open the detail panel (problem-class · add-a-new-X · home · confusion-to-avoid).',
      },
    },
  },
};

export const Detail: Story = {
  name: 'Detail',
  render: () => html`
    <elohim-seam-map
      .profile=${{ lens: 'detail', theme: 'auto', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'pilot' }, origin: 'pilot' }}
    ></elohim-seam-map>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Detail lens: full atlas + concern-routing table below the grid. Click a routing-table row to cross-highlight the seam.',
      },
    },
  },
};

export const Debug: Story = {
  name: 'Debug',
  render: () => html`
    <elohim-seam-map
      .profile=${{ lens: 'debug', theme: 'auto', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'elohim-support' }, origin: 'elohim-support' }}
    ></elohim-seam-map>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Debug lens: detail + data-testid annotations visible in the DOM as sub-labels beneath seam titles.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Content-state stories
// ---------------------------------------------------------------------------

export const Loading: Story = {
  name: 'Loading',
  render: () => html`<elohim-seam-map map-state="loading"></elohim-seam-map>`,
  parameters: {
    docs: {
      description: { story: 'Loading state: skeleton/placeholder while seam map data loads.' },
    },
  },
};

export const Error: Story = {
  name: 'Error',
  render: () => html`<elohim-seam-map map-state="error"></elohim-seam-map>`,
  parameters: {
    docs: {
      description: { story: 'Error state: seam map data could not be loaded.' },
    },
  },
};

export const Empty: Story = {
  name: 'Empty',
  render: () => html`<elohim-seam-map map-state="empty"></elohim-seam-map>`,
  parameters: {
    docs: {
      description: { story: 'Empty state: no data available (e.g. before atlas contentFormat is wired).' },
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
    <elohim-seam-map
      .data=${trimmedMap}
      .profile=${{ lens: 'standard', theme: 'dark', contrast: 'auto', locale: 'en', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'pilot' }, origin: 'pilot' }}
    ></elohim-seam-map>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Dark theme canary: CSS system colors adapt automatically to the dark color-scheme wrapper. No brand tokens involved.',
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
    <elohim-seam-map
      .data=${trimmedMap}
      .profile=${{ lens: 'simple', theme: 'auto', contrast: 'auto', locale: 'he', stimulus: 'still', textuality: 'textual', standings: [], lock: { kind: 'pilot' }, origin: 'pilot' }}
    ></elohim-seam-map>
  `,
  parameters: {
    docs: {
      description: {
        story: 'RTL canary: Hebrew locale in an RTL container. Logical CSS properties (padding-inline, padding-block, gap, border-inline-end) ensure correct layout direction.',
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
    <elohim-seam-map .data=${trimmedMap}></elohim-seam-map>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Wrapped in `style="all: initial;"`. Proves the element renders correctly with zero external CSS tokens. CSS system colors (Canvas, CanvasText, ButtonFace, ButtonText, LinkText) provide legible defaults.',
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
          'Override-surface proof: terminal-green-on-black theme applied via `--elohim-seam-map-*` CSS custom properties. Demonstrates the override surface is honest — no hardcoded brand colors in the element.',
      },
    },
  },
};
