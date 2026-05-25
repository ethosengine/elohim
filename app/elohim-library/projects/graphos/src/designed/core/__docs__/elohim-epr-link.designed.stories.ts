/**
 * Library B designed story — elohim-epr-link
 *
 * Binds the Elohim brand tokens to `<elohim-epr-link>` via story-decorator
 * overrides. The primitive itself is NEVER modified — binding happens above via
 * `--elohim-link-*` CSS custom properties mapped to `--el-*` brand tokens.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — `EprLinkResolution` from `elohim-core` drives load level states.
 *      EPR references use the protocol's pillar-namespaced identifiers.
 *   2. Manifest vocabulary — pillar names (lamad, shefa, qahal, imagodei) are
 *      the resolution `pillar` field values. Reach follows the manifest's
 *      defined reach vocabulary ('minimal', 'simple', 'standard', 'detail').
 *   3. Brand tokens — `--el-*` palette from design spec §14 declared inline.
 *
 * Binding canvas:
 *   --elohim-link-border    → --el-stone at 25% (light) / --el-starlight at 20% (dark)
 *   --elohim-link-hover-bg  → --el-amber at 8% (light) / --el-amber at 12% (dark)
 *
 * The `<elohim-skeleton>` inside L1 and the `<elohim-mention-base>` inside L4
 * inherit the skeleton/mention brand tokens from the decorator container.
 * No extra token overrides are needed for those sub-elements — the decorator
 * cascade covers them.
 *
 * Four progressive states (L1–L4) plus two contextual placements:
 *   1. "Concept card in a lamad lesson" — L3 link to a related concept inline
 *   2. "Preview card when offline" — L4 offline fallback in constellation dark
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';
import { ref } from 'lit/directives/ref.js';

import 'elohim-core/register';
import type { EprLinkResolution } from 'elohim-core';

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

/** Light mode — warm Linen chip, amber hover */
const LINK_TOKENS_LIGHT = `
  --elohim-link-border:   color-mix(in oklch, var(--el-stone) 25%, transparent);
  --elohim-link-hover-bg: color-mix(in oklch, var(--el-amber) 8%, transparent);
  --elohim-skeleton-bg:      color-mix(in oklch, var(--el-stone) 12%, var(--el-cream));
  --elohim-skeleton-shimmer: color-mix(in oklch, var(--el-amber) 30%, transparent);
  --elohim-skeleton-radius:  4px;
  --elohim-mention-bg:           color-mix(in oklch, var(--el-stone) 8%, var(--el-cream));
  --elohim-mention-fg:           var(--el-stone);
  --elohim-mention-border:       color-mix(in oklch, var(--el-stone) 25%, transparent);
  --elohim-mention-fallback-dot: var(--el-amber);
  --elohim-mention-radius:       999px;
`;

/** Dark mode — constellation Indigo Night chip, amber hover */
const LINK_TOKENS_DARK = `
  --elohim-link-border:   color-mix(in oklch, var(--el-starlight) 20%, transparent);
  --elohim-link-hover-bg: color-mix(in oklch, var(--el-amber) 12%, transparent);
  --elohim-skeleton-bg:      color-mix(in oklch, var(--el-starlight) 10%, var(--el-night));
  --elohim-skeleton-shimmer: color-mix(in oklch, var(--el-amber) 20%, transparent);
  --elohim-skeleton-radius:  4px;
  --elohim-mention-bg:           color-mix(in oklch, var(--el-starlight) 8%, var(--el-night-alt));
  --elohim-mention-fg:           var(--el-starlight);
  --elohim-mention-border:       color-mix(in oklch, var(--el-starlight) 20%, transparent);
  --elohim-mention-fallback-dot: var(--el-amber);
  --elohim-mention-radius:       999px;
`;

// ---------------------------------------------------------------------------
// Test seam helpers (same pattern as Library A)
// ---------------------------------------------------------------------------

type EprLinkEl = Element & {
  setResolution: (l: 1 | 2 | 3 | 4, r: EprLinkResolution) => void;
};

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
// Decorator factories
// ---------------------------------------------------------------------------

function lightDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${LINK_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        padding: var(--el-space-lg);
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
        ${LINK_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        padding: var(--el-space-lg);
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
  title: 'Designed/Core/elohim-epr-link',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-epr-link>\` — Elohim brand-bound HyperCard navigation primitive.

The EPR link is how the protocol's named things travel. When a lamad lesson
references the "fair-exchange" concept, the link is not a URL — it is an EPR
reference that resolves through the commons. The chip changes as the commons
responds: from a warm shimmer (L1) to the EPR id (L2) to the full title (L3)
to a named mention if the target is unreachable (L4).

**Progressive states:**
| Level | What happened | What renders |
|---|---|---|
| L1 | Waiting for resolution | Amber-shimmer skeleton (6rem × 1rem) |
| L2 | Resolver returned; no rich metadata | EPR id as chip |
| L3 | Title and metadata resolved | Title chip (the ideal state) |
| L4 | Target unreachable | \`<elohim-mention-base>\` fallback |

**Token binding:**
| Property | Light | Dark |
|---|---|---|
| \`--elohim-link-border\` | stone 25% | starlight 20% |
| \`--elohim-link-hover-bg\` | amber 8% | amber 12% |
| Skeleton + mention sub-tokens | cascade from decorator | cascade from decorator |

Library B — graphos-designer. Primitive untouched.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// L1–L4 progressive state stories
// ---------------------------------------------------------------------------

export const L1Skeleton: Story = {
  name: 'L1 Skeleton (loading)',
  decorators: [lightDecorator],
  render: () => html`
    <p
      style="
        font-family: var(--el-font-body);
        font-size: 1rem;
        line-height: 2;
        color: var(--el-stone);
        max-inline-size: 560px;
      "
    >
      The household's lamad pathway begins with
      <elohim-epr-link epr="epr:lamad-concept-fair-exchange"></elohim-epr-link>
      — the concept is resolving from the commons.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'L1 — the amber shimmer skeleton inline in Source Serif 4 prose. ' +
          'The skeleton holds 6rem × 1rem so the text line does not collapse ' +
          'when the content arrives. The shimmer is Harvest Gold at 30% — ' +
          'warm, not grey, not neutral. The page is not broken; it is becoming.',
      },
    },
  },
};

export const L2Resolved: Story = {
  name: 'L2 Resolved (EPR id)',
  decorators: [lightDecorator],
  render: () => html`
    <p
      style="
        font-family: var(--el-font-body);
        font-size: 1rem;
        line-height: 2;
        color: var(--el-stone);
        max-inline-size: 560px;
      "
    >
      The household's pathway begins with
      <elohim-epr-link
        epr="epr:lamad-concept-fair-exchange"
        ${ref(withResolution(2, {}))}
      ></elohim-epr-link>
      — the EPR has resolved but no title is available yet.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'L2 — the EPR identifier is the chip text. The commons has responded ' +
          'but carries no rich metadata. The chip is present and navigable; ' +
          'the fallback dot is Harvest Gold — "the resolution is not complete, ' +
          'but the reference is real."',
      },
    },
  },
};

export const L3Full: Story = {
  name: 'L3 Full (title resolved)',
  decorators: [lightDecorator],
  render: () => html`
    <p
      style="
        font-family: var(--el-font-body);
        font-size: 1rem;
        line-height: 2;
        color: var(--el-stone);
        max-inline-size: 560px;
      "
    >
      The household's pathway begins with
      <elohim-epr-link
        epr="epr:lamad-concept-fair-exchange"
        ${ref(
          withResolution(3, {
            title: 'fair-exchange',
            description: 'A provision is fair-exchange when both parties confirm the balance.',
            pillar: 'lamad',
            reach: 'standard',
          }),
        )}
      ></elohim-epr-link>
      and continues through stewardship-of-commons before arriving at covenant-of-care.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'L3 — the ideal state. The chip reads "fair-exchange" in DM Sans, inline in ' +
          'Source Serif 4 body prose. The concept name is lower-case by protocol convention: ' +
          'lamad concepts are named like verbs, not like proper nouns. ' +
          'Hover shows a warm Harvest Gold tint (--elohim-link-hover-bg).',
      },
    },
  },
};

export const L4Unreachable: Story = {
  name: 'L4 Unreachable (fallback)',
  decorators: [lightDecorator],
  render: () => html`
    <p
      style="
        font-family: var(--el-font-body);
        font-size: 1rem;
        line-height: 2;
        color: var(--el-stone);
        max-inline-size: 560px;
      "
    >
      The provision references
      <elohim-epr-link
        epr="epr:shefa-commitment-sha256-4a7b9c2d1e8f3a5b6c0d9e2f1a4b7c8d9e0f1a2b"
        ${ref(
          withResolution(4, {
            unreachable: true,
            preview: { title: 'harvest-bounty commitment' },
          }),
        )}
      ></elohim-epr-link>
      — the commons is currently unreachable. The last known title is shown.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'L4 — target unreachable. The element falls back to `<elohim-mention-base>` ' +
          'with the last cached title ("harvest-bounty commitment"). ' +
          'The fallback chip is identical to a bare mention chip: Harvest Gold dot, ' +
          'Linen background, Hearthstone text. The content is still named; ' +
          'it is just currently out of reach.',
      },
    },
  },
};

export const L4NoPreview: Story = {
  name: 'L4 Unreachable (no preview)',
  decorators: [lightDecorator],
  render: () => html`
    <p
      style="
        font-family: var(--el-font-body);
        font-size: 1rem;
        line-height: 2;
        color: var(--el-stone);
        max-inline-size: 560px;
      "
    >
      This reference
      <elohim-epr-link
        epr="epr:lamad-concept-sha256-9e8d7c6b5a4f3e2d1c0b9a8f7e6d5c4b3a2f1e0d"
        ${ref(withResolution(4, { unreachable: true }))}
      ></elohim-epr-link>
      has never resolved on this device. The EPR identifier is the fallback label.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'L4 with no cached preview — the EPR identifier is the chip text. ' +
          'The identifier uses a sha256 hash prefix: it is long but well-formed. ' +
          'It is not a placeholder. The household member sees the reference is real ' +
          'even when the commons is silent.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Contextual placement 1 — concept card in a lamad lesson
// ---------------------------------------------------------------------------

export const LamadLessonConcept: Story = {
  name: 'LamadLesson (concept link)',
  decorators: [lightDecorator],
  render: () => html`
    <article
      style="
        max-inline-size: 640px;
        padding: var(--el-space-lg);
        background: var(--el-cream);
        border-radius: var(--el-radius-md);
        box-shadow: var(--el-shadow-soft);
      "
    >
      <h2
        style="
          font-family: var(--el-font-display);
          font-size: 1.5rem;
          font-weight: 400;
          color: var(--el-green-deep);
          margin-block: 0 var(--el-space-sm);
        "
      >stewardship-of-commons</h2>
      <p
        style="
          font-family: var(--el-font-body);
          font-size: 1rem;
          line-height: 1.75;
          color: var(--el-stone);
          margin-block: 0 var(--el-space-sm);
        "
      >
        Stewardship of commons follows naturally from mastering
        <elohim-epr-link
          epr="epr:lamad-concept-fair-exchange"
          ${ref(
            withResolution(3, {
              title: 'fair-exchange',
              pillar: 'lamad',
              reach: 'standard',
            }),
          )}
        ></elohim-epr-link>.
        When you can account for what was given and received, you are ready
        to hold something on behalf of your neighbors.
      </p>
      <p
        style="
          font-family: var(--el-font-body);
          font-size: 1rem;
          line-height: 1.75;
          color: var(--el-stone);
          margin-block: 0;
        "
      >
        The prerequisite pathway includes
        <elohim-epr-link
          epr="epr:lamad-concept-covenant-of-care"
          ${ref(
            withResolution(3, {
              title: 'covenant-of-care',
              pillar: 'lamad',
              reach: 'standard',
            }),
          )}
        ></elohim-epr-link>
        and
        <elohim-epr-link
          epr="epr:lamad-concept-provision-vocabulary"
          ${ref(
            withResolution(3, {
              title: 'provision vocabulary',
              pillar: 'lamad',
              reach: 'simple',
            }),
          )}
        ></elohim-epr-link>.
      </p>
    </article>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Contextual placement: three concept links in a lamad lesson body. ' +
          'The L3-resolved chips ("fair-exchange", "covenant-of-care", "provision vocabulary") ' +
          'sit inline in Source Serif 4 at 1rem / 1.75 line-height. ' +
          'The lesson card uses the full brand surface: Fraunces heading in Vineyard, ' +
          'body in Hearthstone, card in Linen with a warm shadow.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Contextual placement 2 — offline fallback in constellation dark
// ---------------------------------------------------------------------------

export const OfflineDarkFallback: Story = {
  name: 'OfflineFallback (dark)',
  decorators: [darkDecorator],
  render: () => html`
    <article
      style="
        max-inline-size: 640px;
        padding: var(--el-space-lg);
        background: var(--el-night-alt);
        border-radius: var(--el-radius-md);
        border: 1px solid color-mix(in oklch, var(--el-starlight) 10%, transparent);
      "
    >
      <h2
        style="
          font-family: var(--el-font-display);
          font-size: 1.5rem;
          font-weight: 400;
          color: var(--el-amber);
          margin-block: 0 var(--el-space-sm);
        "
      >stewardship-of-commons</h2>
      <p
        style="
          font-family: var(--el-font-body);
          font-size: 1rem;
          line-height: 1.75;
          color: var(--el-starlight);
          margin-block: 0;
        "
      >
        This lesson references
        <elohim-epr-link
          epr="epr:lamad-concept-fair-exchange"
          ${ref(
            withResolution(4, {
              unreachable: true,
              preview: { title: 'fair-exchange' },
            }),
          )}
        ></elohim-epr-link>
        — the commons is not currently reachable on this device.
        The last known title is shown. Your progress is saved locally.
      </p>
    </article>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Offline L4 fallback in the constellation dark theme. The mention chip ' +
          '("fair-exchange") is in Starlight on an Indigo Night background. ' +
          'The lesson card tells the household member their progress is safe — ' +
          '"Your progress is saved locally" — not "You are offline."',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// All four levels side-by-side (light)
// ---------------------------------------------------------------------------

export const ProgressiveStates: Story = {
  name: 'ProgressiveStates (L1–L4)',
  decorators: [
    (story: () => unknown) => html`
      <div
        style="
          ${EL_TOKENS}
          ${LINK_TOKENS_LIGHT}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: var(--el-space-lg);
          display: flex;
          flex-direction: column;
          gap: var(--el-space-md);
          max-inline-size: 560px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <div>
      <p style="font-size: 0.75rem; color: var(--el-stone); margin-block: 0 4px; font-family: var(--el-font-ui); text-transform: uppercase; letter-spacing: 0.04em;">L1 — skeleton</p>
      <elohim-epr-link epr="epr:lamad-concept-fair-exchange"></elohim-epr-link>
    </div>
    <div>
      <p style="font-size: 0.75rem; color: var(--el-stone); margin-block: 0 4px; font-family: var(--el-font-ui); text-transform: uppercase; letter-spacing: 0.04em;">L2 — EPR id</p>
      <elohim-epr-link epr="epr:lamad-concept-fair-exchange" ${ref(withResolution(2, {}))}></elohim-epr-link>
    </div>
    <div>
      <p style="font-size: 0.75rem; color: var(--el-stone); margin-block: 0 4px; font-family: var(--el-font-ui); text-transform: uppercase; letter-spacing: 0.04em;">L3 — title resolved</p>
      <elohim-epr-link epr="epr:lamad-concept-fair-exchange" ${ref(withResolution(3, { title: 'fair-exchange', pillar: 'lamad', reach: 'standard' }))}></elohim-epr-link>
    </div>
    <div>
      <p style="font-size: 0.75rem; color: var(--el-stone); margin-block: 0 4px; font-family: var(--el-font-ui); text-transform: uppercase; letter-spacing: 0.04em;">L4 — unreachable</p>
      <elohim-epr-link epr="epr:lamad-concept-fair-exchange" ${ref(withResolution(4, { unreachable: true, preview: { title: 'fair-exchange' } }))}></elohim-epr-link>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'All four progressive states stacked — the loading sequence for the same ' +
          '"fair-exchange" concept reference. L1 is the amber shimmer; L2 is the ' +
          'raw EPR id; L3 is the resolved title; L4 is the unreachable fallback. ' +
          'Each state is named and distinct; no state looks like an error unless ' +
          'it actually is one.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Light / Dark named theme stories
// ---------------------------------------------------------------------------

export const Light: Story = {
  name: 'Light',
  decorators: [lightDecorator],
  render: () => html`
    <p style="font-family: var(--el-font-body); font-size: 1rem; line-height: 2; color: var(--el-stone); max-inline-size: 560px;">
      The lamad pathway begins with
      <elohim-epr-link
        epr="epr:lamad-concept-fair-exchange"
        ${ref(withResolution(3, { title: 'fair-exchange', pillar: 'lamad', reach: 'standard' }))}
      ></elohim-epr-link>
      and continues through stewardship-of-commons.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Light theme — Linen background, Hearthstone text, stone-bordered chip with amber hover.',
      },
    },
  },
};

export const Dark: Story = {
  name: 'Dark',
  decorators: [darkDecorator],
  render: () => html`
    <p style="font-family: var(--el-font-body); font-size: 1rem; line-height: 2; color: var(--el-starlight); max-inline-size: 560px;">
      The lamad pathway begins with
      <elohim-epr-link
        epr="epr:lamad-concept-fair-exchange"
        ${ref(withResolution(3, { title: 'fair-exchange', pillar: 'lamad', reach: 'standard' }))}
      ></elohim-epr-link>
      and continues through stewardship-of-commons.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Dark (constellation) theme — Deep Sky background, Starlight text, Starlight-bordered chip with amber hover.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// RTL canary — Hebrew
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
          ${LINK_TOKENS_LIGHT}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: var(--el-space-lg);
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <p
      style="
        font-family: var(--el-font-body);
        font-size: 1rem;
        line-height: 2;
        color: var(--el-stone);
        max-inline-size: 560px;
      "
    >
      מסלול הלמד מתחיל עם
      <elohim-epr-link
        epr="epr:lamad-concept-fair-exchange"
        ${ref(withResolution(3, { title: 'חליפה-הוגנת', pillar: 'lamad', reach: 'standard' }))}
      ></elohim-epr-link>
      ומשם לניהול-השיתוף.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary — he-IL locale inside a dir="rtl" container. The L3 chip shows ' +
          '"חליפה-הוגנת" (fair-exchange in Hebrew) and sits inline in right-to-left prose. ' +
          'The chip\'s `padding-inline` uses logical properties and mirrors correctly.',
      },
    },
  },
};
