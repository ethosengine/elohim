/**
 * Library B designed story — elohim-skeleton
 *
 * Binds the Elohim brand tokens to `<elohim-skeleton>` via story-decorator
 * overrides. The primitive itself is NEVER modified — binding happens above via
 * `--elohim-skeleton-*` CSS custom properties mapped to `--el-*` brand tokens.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — elohim-skeleton is a pure layout primitive with no ts-rs view
 *      dependency. Width/height/radius are set as attributes.
 *   2. Manifest vocabulary — none (skeleton carries no content type).
 *   3. Brand tokens — `--el-*` palette from design spec §14 declared inline
 *      as EL_TOKENS until the global graphos token stylesheet ships.
 *
 * Binding canvas:
 *   --elohim-skeleton-bg       → color-mix of --el-stone at low opacity on --el-cream
 *   --elohim-skeleton-shimmer  → --el-amber at 30% opacity (warm earth shimmer)
 *   --elohim-skeleton-radius   → --el-radius-sm (4px) for text lines;
 *                                 attr radius="50%" for avatar circles
 *
 * Contextual placements:
 *   - Household dashboard tile loading (avatar + two text lines)
 *   - Lamad concept card loading (card block + title line + body lines)
 *   - Shefa balance summary loading (value block + label line)
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

// ---------------------------------------------------------------------------
// Brand token declaration
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

/** Light mode — warm earth shimmer on Linen */
const SKELETON_TOKENS_LIGHT = `
  --elohim-skeleton-bg:      color-mix(in oklch, var(--el-stone) 12%, var(--el-cream));
  --elohim-skeleton-shimmer: color-mix(in oklch, var(--el-amber) 30%, transparent);
  --elohim-skeleton-radius:  var(--el-radius-sm);
`;

/** Dark mode — constellation night shimmer */
const SKELETON_TOKENS_DARK = `
  --elohim-skeleton-bg:      color-mix(in oklch, var(--el-starlight) 10%, var(--el-night));
  --elohim-skeleton-shimmer: color-mix(in oklch, var(--el-amber) 20%, transparent);
  --elohim-skeleton-radius:  var(--el-radius-sm);
`;

// ---------------------------------------------------------------------------
// Decorator factories
// ---------------------------------------------------------------------------

function lightDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${SKELETON_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        padding: var(--el-space-lg);
        max-inline-size: 480px;
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
        ${SKELETON_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        padding: var(--el-space-lg);
        max-inline-size: 480px;
      "
    >
      ${story()}
    </div>
  `;
}

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Core/elohim-skeleton',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-skeleton>\` — Elohim brand-bound loading placeholder.

Warm earth shimmer replaces the generic grey: Harvest Gold at low opacity
slides across a Linen-toned base while the household's content loads. In the
constellation dark mode the shimmer cools to a faint amber trace on Deep Sky.

The shimmer animation is gated behind
\`@media (prefers-reduced-motion: no-preference) and (update: fast)\` — it
does not fire when the household member has requested stillness.

**Token binding:**
| Property | Light | Dark |
|---|---|---|
| \`--elohim-skeleton-bg\` | stone 12% on cream | starlight 10% on night |
| \`--elohim-skeleton-shimmer\` | amber 30% | amber 20% |
| \`--elohim-skeleton-radius\` | 4px | 4px |

Library B — graphos-designer. Primitive untouched.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Contextual placement — household dashboard tile loading
// ---------------------------------------------------------------------------

export const HouseholdTileLoading: Story = {
  name: 'HouseholdTileLoading',
  decorators: [lightDecorator],
  render: () => html`
    <div
      style="
        display: flex;
        align-items: flex-start;
        gap: var(--el-space-sm);
        padding: var(--el-space-md);
        background: var(--el-cream);
        border-radius: var(--el-radius-md);
        box-shadow: var(--el-shadow-soft);
      "
    >
      <!-- Avatar circle placeholder -->
      <elohim-skeleton width="48px" height="48px" radius="50%"></elohim-skeleton>
      <!-- Name + role lines -->
      <div style="display: flex; flex-direction: column; gap: 6px; flex: 1; padding-block-start: 4px;">
        <elohim-skeleton width="55%" height="1rem"></elohim-skeleton>
        <elohim-skeleton width="35%" height="0.75rem"></elohim-skeleton>
      </div>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Household dashboard tile loading — avatar circle + name + role line. ' +
          'The warm Harvest Gold shimmer on a Linen background communicates "your ' +
          'household is here, we are just fetching the details." ' +
          'No spinner — stillness is the Sabbath default.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Contextual placement — lamad concept card loading
// ---------------------------------------------------------------------------

export const LamadConceptCardLoading: Story = {
  name: 'LamadConceptCardLoading',
  decorators: [lightDecorator],
  render: () => html`
    <div
      style="
        display: flex;
        flex-direction: column;
        gap: var(--el-space-sm);
        padding: var(--el-space-md);
        background: var(--el-cream);
        border-radius: var(--el-radius-md);
        box-shadow: var(--el-shadow-soft);
        max-inline-size: 380px;
      "
    >
      <!-- Card image / concept cover block -->
      <elohim-skeleton width="100%" height="120px" radius="6px"></elohim-skeleton>
      <!-- Concept title -->
      <elohim-skeleton width="70%" height="1.125rem"></elohim-skeleton>
      <!-- Body text lines -->
      <elohim-skeleton width="100%" height="0.875rem"></elohim-skeleton>
      <elohim-skeleton width="90%" height="0.875rem"></elohim-skeleton>
      <elohim-skeleton width="60%" height="0.875rem"></elohim-skeleton>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Lamad concept card loading — a cover block, title line, and three body lines. ' +
          'This is what a household member sees while the lamad concept resolves from the ' +
          'commons. The shimmer traces the card shape so the content arrives without ' +
          'reflow — no visual "pop" when the real card slots in.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Contextual placement — shefa balance summary loading
// ---------------------------------------------------------------------------

export const ShefaBalanceSummaryLoading: Story = {
  name: 'ShefaBalanceSummaryLoading',
  decorators: [lightDecorator],
  render: () => html`
    <div
      style="
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: var(--el-space-xs);
        padding: var(--el-space-md);
        background: var(--el-cream);
        border-radius: var(--el-radius-md);
        box-shadow: var(--el-shadow-soft);
      "
    >
      <!-- Large value placeholder -->
      <elohim-skeleton width="120px" height="2rem" radius="4px"></elohim-skeleton>
      <!-- Label beneath the value -->
      <elohim-skeleton width="80px" height="0.75rem"></elohim-skeleton>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Shefa balance summary loading — a large value block + label line. ' +
          'This is the skeleton that holds the household\'s stewardship balance ' +
          'while the shefa view resolves from the commons. ' +
          'The wide block prevents layout shift when the currency value arrives.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Light theme (explicit named story)
// ---------------------------------------------------------------------------

export const Light: Story = {
  name: 'Light',
  decorators: [lightDecorator],
  render: () => html`
    <div style="display: flex; flex-direction: column; gap: var(--el-space-sm);">
      <elohim-skeleton width="48px" height="48px" radius="50%"></elohim-skeleton>
      <elohim-skeleton width="200px" height="1rem"></elohim-skeleton>
      <elohim-skeleton width="140px" height="0.875rem"></elohim-skeleton>
      <elohim-skeleton width="100%" height="3rem" radius="6px"></elohim-skeleton>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Light theme — Linen background, stone-tinted skeleton base, Harvest Gold shimmer. ' +
          'The four shapes demonstrate the three common sizes: avatar circle, text lines, card block.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Dark theme (constellation)
// ---------------------------------------------------------------------------

export const Dark: Story = {
  name: 'Dark',
  decorators: [darkDecorator],
  render: () => html`
    <div style="display: flex; flex-direction: column; gap: var(--el-space-sm);">
      <elohim-skeleton width="48px" height="48px" radius="50%"></elohim-skeleton>
      <elohim-skeleton width="200px" height="1rem"></elohim-skeleton>
      <elohim-skeleton width="140px" height="0.875rem"></elohim-skeleton>
      <elohim-skeleton width="100%" height="3rem" radius="6px"></elohim-skeleton>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Constellation dark theme — Deep Sky (#0F1A12) background, starlight-tinted skeleton ' +
          'base, faint amber shimmer. The skeleton reads like a constellation chart: ' +
          'dim outlines waiting to be filled with light.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// RTL canary
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
          ${SKELETON_TOKENS_LIGHT}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: var(--el-space-lg);
          max-inline-size: 480px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <div style="display: flex; align-items: flex-start; gap: var(--el-space-sm);">
      <elohim-skeleton width="48px" height="48px" radius="50%"></elohim-skeleton>
      <div style="display: flex; flex-direction: column; gap: 6px; flex: 1; padding-block-start: 4px;">
        <elohim-skeleton width="55%" height="1rem"></elohim-skeleton>
        <elohim-skeleton width="35%" height="0.75rem"></elohim-skeleton>
      </div>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary — he-IL locale inside a dir="rtl" container. The skeleton widths use ' +
          'percentages that resolve correctly from the inline-end under RTL. ' +
          'The household avatar and text placeholders mirror naturally.',
      },
    },
  },
};
