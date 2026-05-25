/**
 * Library A default story — elohim-epr-link
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * <elohim-epr-link> is the protocol's HyperCard navigation primitive.
 * Progressive loading L1–L4:
 *   L1 — skeleton placeholder (waiting for resolution)
 *   L2 — EPR id shown as chip (resolver returned, no rich metadata)
 *   L3 — title and/or metadata resolved (full chip)
 *   L4 — target unreachable, falls back to <elohim-mention-base>
 *
 * Test seam: `setResolution(level, resolution)` drives load level without
 * a real network layer. Stories use this seam via Lit's `ref` directive.
 *
 * Right-click opens <elohim-context-menu> with MVP items: Open / About / Copy.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';
import { ref } from 'lit/directives/ref.js';

import 'elohim-core/register';
import type { EprLinkResolution } from 'elohim-core';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type EprLinkEl = Element & {
  setResolution: (l: 1 | 2 | 3 | 4, r: EprLinkResolution) => void;
};

/**
 * Returns a Lit ref callback that calls setResolution() on the element after
 * it mounts. Queued with queueMicrotask so the element's connectedCallback
 * has completed before the test seam fires.
 */
function withResolution(
  level: 1 | 2 | 3 | 4,
  resolution: EprLinkResolution,
): (el: Element | undefined) => void {
  return (el) => {
    if (!el) return;
    const link = el as EprLinkEl;
    if (typeof link.setResolution === 'function') {
      queueMicrotask(() => link.setResolution(level, resolution));
    }
  };
}

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-epr-link',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-epr-link>\` — the protocol's HyperCard navigation primitive.

Renders a sized skeleton immediately (L1), fills in title/metadata as the
Loader resolves (L2/L3), falls back to \`<elohim-mention-base>\` if the
target is unreachable (L4).

**Default click:** emits \`navigate\` with the EPR ref. Parent decides navigation.
**Right-click:** opens \`<elohim-context-menu>\` with Open / About / Copy EPR.
**Keyboard:** Shift+F10 opens the context menu per accessibility convention.

**Test seam:** \`setResolution(level, resolution)\` drives load levels without
a real network layer. Stories use this seam via the Lit \`ref\` directive.

**Override surface:** \`--elohim-link-border\`, \`--elohim-link-hover-bg\`.
**Parts:** \`skeleton\`, \`anchor\`, \`fallback\`, \`menu\`.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// L1–L4 progressive load level stories
// ---------------------------------------------------------------------------

export const L1Skeleton: Story = {
  name: 'L1Skeleton (loading)',
  render: () => html`
    <div style="padding: 1rem;">
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
        L1: waiting for resolution — an <code>&lt;elohim-skeleton&gt;</code>
        (6rem × 1rem) renders instantly. The page does not reflow when L2 arrives.
      </p>
      <elohim-epr-link epr="epr:lamad-spa"></elohim-epr-link>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'L1 is the initial render state — the element starts here whenever `epr` changes. A 6rem skeleton placeholder holds space. No network call has resolved.',
      },
    },
  },
};

export const L2Resolved: Story = {
  name: 'L2Resolved (EPR id as chip)',
  render: () => html`
    <div style="padding: 1rem;">
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
        L2: resolver returned with no rich metadata — the EPR id is shown as a chip.
      </p>
      <elohim-epr-link
        epr="epr:lamad-spa"
        ${ref(withResolution(2, {}))}
      ></elohim-epr-link>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'L2 is the default no-resolver state (or when the resolver has no title to offer). The EPR id itself is the chip label. Clicking emits `navigate`; right-clicking opens the context menu.',
      },
    },
  },
};

export const L3Full: Story = {
  name: 'L3Full (title resolved)',
  render: () => html`
    <div style="padding: 1rem;">
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
        L3: resolver returned with a title and metadata — full chip.
      </p>
      <elohim-epr-link
        epr="epr:lamad-spa"
        ${ref(
          withResolution(3, {
            title: 'Lamad Learning Platform',
            description: 'The primary learner surface of the Elohim Protocol.',
            pillar: 'lamad',
            reach: 'standard',
          }),
        )}
      ></elohim-epr-link>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'L3: the resolver returned `{ title, description, pillar, reach }`. The chip displays the human-readable title instead of the raw EPR id.',
      },
    },
  },
};

export const L4Unreachable: Story = {
  name: 'L4Unreachable (preview fallback)',
  render: () => html`
    <div style="padding: 1rem;">
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
        L4: target unreachable — falls back to
        <code>&lt;elohim-mention-base&gt;</code> with the last known title
        (or raw EPR id if none).
      </p>
      <elohim-epr-link
        epr="epr:lamad-spa"
        ${ref(
          withResolution(4, {
            unreachable: true,
            preview: { title: 'Lamad Learning Platform' },
          }),
        )}
      ></elohim-epr-link>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'L4: the resolver returned `null` or threw — `unreachable: true` is set. The element renders `<elohim-mention-base>` with whatever preview title is available. The context menu is still accessible.',
      },
    },
  },
};

export const L4NoPreview: Story = {
  name: 'L4Unreachable (no preview)',
  render: () => html`
    <div style="padding: 1rem;">
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
        L4 with no preview title — the raw EPR id is the chip label.
      </p>
      <elohim-epr-link
        epr="epr:lamad-unknown-xyz"
        ${ref(withResolution(4, { unreachable: true }))}
      ></elohim-epr-link>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'L4 with no cached preview: the `<elohim-mention-base>` receives an empty label so the EPR id is the chip text.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Context menu (right-click) interactive story
// ---------------------------------------------------------------------------

export const RightClickOpensMenu: Story = {
  name: 'RightClickOpensMenu (interactive)',
  render: () => {
    const onNavigate = (e: Event) =>
      // eslint-disable-next-line no-console
      console.log('navigate:', (e as CustomEvent<{ epr: string }>).detail);
    const onAbout = (e: Event) =>
      // eslint-disable-next-line no-console
      console.log('about:', (e as CustomEvent<{ epr: string }>).detail);

    return html`
      <div style="padding: 2rem; position: relative;">
        <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.75rem;">
          Right-click the chip (or Shift+F10) to open the context menu.
          Keyboard: arrow keys navigate, Enter selects, Escape closes.
          Events are logged to the browser console.
        </p>
        <elohim-epr-link
          epr="epr:lamad-spa"
          ${ref(withResolution(3, { title: 'Lamad Learning Platform' }))}
          @navigate=${onNavigate}
          @about=${onAbout}
        ></elohim-epr-link>
      </div>
    `;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Interactive demo: a resolved L3 chip with right-click / Shift+F10 context menu. `navigate` and `about` events are logged to the console. The click-outside handler closes the menu when clicking elsewhere.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Display variants
// ---------------------------------------------------------------------------

export const DisplayInline: Story = {
  name: 'DisplayInline',
  render: () => html`
    <p style="padding: 1rem; line-height: 2; max-inline-size: 600px;">
      The
      <elohim-epr-link
        epr="epr:lamad-spa"
        display="inline"
        ${ref(withResolution(3, { title: 'Lamad Learning Platform' }))}
      ></elohim-epr-link>
      link appears inline in running prose, wrapping naturally with the surrounding text.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          '`display="inline"` — the element is `display: inline-block` by default, suitable for embedding in prose.',
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
    <elohim-epr-link
      epr="epr:lamad-spa"
      ${ref(withResolution(3, { title: 'Lamad Learning Platform' }))}
    ></elohim-epr-link>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Dark background canary. The chip border uses `color-mix(in oklch, currentColor 25%, transparent)` — adapts to the inherited light text color.',
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
    <p style="line-height: 2;">
      ראה את
      <elohim-epr-link
        epr="epr:lamad-spa"
        ${ref(withResolution(3, { title: 'פלטפורמת למד' }))}
      ></elohim-epr-link>
      לפרטים נוספים.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary: chip with a Hebrew title in a right-to-left prose context. Logical CSS properties on `padding-inline` ensure correct layout.',
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
    <elohim-epr-link
      epr="epr:lamad-spa"
      ${ref(withResolution(3, { title: 'Lamad Learning Platform' }))}
    ></elohim-epr-link>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Wrapped in `style="all: initial;"`. Proves the chip renders with zero external CSS tokens. Border is `1px solid color-mix(…)` with a transparent-currentColor fallback that stays legible.',
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
          --elohim-link-border: #9040c0;
          --elohim-link-hover-bg: rgba(144, 64, 192, 0.15);
          color: #f0c8ff;
          background: #0e0020;
          font-family: Courier, monospace;
          padding: 1.5rem;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <p style="line-height: 2;">
      Navigate to
      <elohim-epr-link
        epr="epr:lamad-spa"
        ${ref(withResolution(3, { title: 'Lamad Learning Platform' }))}
      ></elohim-epr-link>
      in the protocol.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Override-surface proof: a deliberately non-Elohim purple theme applied via `--elohim-link-border` and `--elohim-link-hover-bg`. Proves the override surface is honest — no hardcoded brand colors in the element.',
      },
    },
  },
};
