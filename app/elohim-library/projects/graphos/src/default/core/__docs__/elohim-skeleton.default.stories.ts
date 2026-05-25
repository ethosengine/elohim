/**
 * Library A default story — elohim-skeleton
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * <elohim-skeleton> is a sized shimmer placeholder used during progressive
 * loading. The shimmer animation fires only when the user has not requested
 * reduced motion AND the display can update at full speed.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-skeleton',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-skeleton>\` — sized shimmer placeholder for progressive loading.

Renders an empty box at the declared width/height. Page does not reflow when
real content arrives. The shimmer animation is gated behind
\`@media (prefers-reduced-motion: no-preference) and (update: fast)\` — it
**does not fire** when the user has requested reduced motion.

**Override surface:** \`--elohim-skeleton-bg\`, \`--elohim-skeleton-shimmer\`,
\`--elohim-skeleton-radius\`.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Size stories
// ---------------------------------------------------------------------------

export const DefaultSize: Story = {
  name: 'DefaultSize',
  render: () => html`<elohim-skeleton></elohim-skeleton>`,
  parameters: {
    docs: {
      description: {
        story: 'Default dimensions: width=100%, height=1rem. Fills its inline container.',
      },
    },
  },
};

export const DifferentSizes: Story = {
  name: 'DifferentSizes',
  render: () => html`
    <div style="display: flex; flex-direction: column; gap: 0.75rem; padding: 1rem; max-inline-size: 400px;">
      <elohim-skeleton width="200px" height="1rem"></elohim-skeleton>
      <elohim-skeleton width="100%" height="2rem"></elohim-skeleton>
      <elohim-skeleton width="60px" height="60px" radius="50%"></elohim-skeleton>
      <elohim-skeleton width="100%" height="4rem" radius="8px"></elohim-skeleton>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Multiple sizes and border-radius values. The circular variant (60×60, radius=50%) works as an avatar placeholder.',
      },
    },
  },
};

export const InlineContext: Story = {
  name: 'InlineContext',
  render: () => html`
    <div style="display: flex; align-items: center; gap: 0.5rem; padding: 1rem;">
      <elohim-skeleton width="2rem" height="2rem" radius="50%"></elohim-skeleton>
      <div style="display: flex; flex-direction: column; gap: 0.25rem; flex: 1;">
        <elohim-skeleton width="60%" height="0.875rem"></elohim-skeleton>
        <elohim-skeleton width="40%" height="0.75rem"></elohim-skeleton>
      </div>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Skeletons composing an avatar + two-line text placeholder — the common card-row loading pattern.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Motion/accessibility notes
// ---------------------------------------------------------------------------

export const ReducedMotion: Story = {
  name: 'ReducedMotion (static fallback)',
  render: () => html`
    <div style="padding: 1rem;">
      <p style="font-size: 0.875rem; margin-block-end: 0.75rem; opacity: 0.8;">
        When <code>prefers-reduced-motion: reduce</code> is active the shimmer animation
        is suppressed. The skeleton renders as a static muted box — no motion whatsoever.
        The CSS gate is structural: <code>@media (prefers-reduced-motion: no-preference) and (update: fast)</code>.
      </p>
      <elohim-skeleton width="300px" height="1.25rem"></elohim-skeleton>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Documents the reduced-motion gate. Enable "prefers-reduced-motion: reduce" in browser DevTools to verify the shimmer stops.',
      },
    },
  },
};

export const ForcedColors: Story = {
  name: 'ForcedColors (high-contrast)',
  render: () => html`
    <div style="padding: 1rem;">
      <p style="font-size: 0.875rem; margin-block-end: 0.75rem; opacity: 0.8;">
        Under <code>forced-colors: active</code> the shimmer gradient is hidden and the
        element renders with <code>ButtonFace</code> background and a <code>ButtonText</code>
        border — fully visible in Windows High Contrast.
      </p>
      <elohim-skeleton width="300px" height="1.25rem"></elohim-skeleton>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Documents the forced-colors override. Enable "Force colors" in browser DevTools emulation to verify the border appears.',
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
      <div style="background: #1a1a1a; padding: 1.5rem; color: #e8e8e8; color-scheme: dark;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <div style="display: flex; flex-direction: column; gap: 0.75rem;">
      <elohim-skeleton width="300px" height="1rem"></elohim-skeleton>
      <elohim-skeleton width="200px" height="1rem"></elohim-skeleton>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Dark background canary. The skeleton uses `color-mix(in oklch, currentColor …)` so it adapts to the inherited text color without any token override.',
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
    <div style="display: flex; flex-direction: column; gap: 0.5rem;">
      <elohim-skeleton width="80%" height="1rem"></elohim-skeleton>
      <elohim-skeleton width="60%" height="1rem"></elohim-skeleton>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'RTL canary: skeleton layout in a right-to-left container. Width fills start-to-end correctly.',
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
  render: () => html`<elohim-skeleton width="200px" height="1.25rem"></elohim-skeleton>`,
  parameters: {
    docs: {
      description: {
        story:
          'Wrapped in `style="all: initial;"`. Proves the element renders with zero external CSS tokens. The shimmer uses `color-mix(in oklch, currentColor …)` which degrades gracefully when `currentColor` is browser-default black.',
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
          --elohim-skeleton-bg: #2a0a4a;
          --elohim-skeleton-shimmer: rgba(180, 100, 255, 0.4);
          --elohim-skeleton-radius: 0;
          color: #e8c8ff;
          background: #0e0020;
          padding: 1.5rem;
          font-family: Courier, monospace;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <div style="display: flex; flex-direction: column; gap: 0.75rem; max-inline-size: 400px;">
      <elohim-skeleton width="300px" height="1rem"></elohim-skeleton>
      <elohim-skeleton width="200px" height="1rem"></elohim-skeleton>
      <elohim-skeleton width="80%" height="3rem"></elohim-skeleton>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Override-surface proof: a deliberately non-Elohim purple-on-dark theme applied via `--elohim-skeleton-bg`, `--elohim-skeleton-shimmer`, and `--elohim-skeleton-radius`. Proves consumers can theme the primitive without forking it.',
      },
    },
  },
};
