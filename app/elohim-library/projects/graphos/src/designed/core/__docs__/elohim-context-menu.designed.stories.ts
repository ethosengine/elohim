/**
 * Library B designed story — elohim-context-menu
 *
 * Binds the Elohim brand tokens to `<elohim-context-menu>` via story-decorator
 * overrides. The primitive itself is NEVER modified — binding happens above via
 * `--elohim-menu-*` CSS custom properties mapped to `--el-*` brand tokens.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — `ContextMenuItem[]` from `elohim-core` (the item shape).
 *   2. Manifest vocabulary — menu item labels use protocol vocabulary:
 *      "Open" → "Open", "About this EPR" → kept, "Copy EPR link" → kept.
 *      The menu is MVP-flat; the label vocabulary is the brand voice.
 *   3. Brand tokens — `--el-*` palette from design spec §14 declared inline.
 *
 * Binding canvas:
 *   --elohim-menu-bg     → --el-cream (light) / --el-night-alt (dark)
 *   --elohim-menu-border → --el-stone at 20% (light) / --el-starlight at 15% (dark)
 *   --elohim-menu-radius → --el-radius-md (8px)
 *   --elohim-menu-shadow → --el-shadow-medium (warm, not hard)
 *
 * Two contextual placement stories:
 *   1. Opens above a concept card mention in a lamad lesson
 *   2. Opens beside a steward affordance in a shefa view
 *
 * The menu is stillness-default — it appears at once, no ambient animation.
 * The fold-down animation is a named story: "Gentle (fold-down)".
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';
import type { ContextMenuItem } from 'elohim-core';

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

/** Light mode — Linen menu, warm shadow */
const MENU_TOKENS_LIGHT = `
  --elohim-menu-bg:     var(--el-cream);
  --elohim-menu-border: color-mix(in oklch, var(--el-stone) 20%, transparent);
  --elohim-menu-radius: var(--el-radius-md);
  --elohim-menu-shadow: var(--el-shadow-medium);
`;

/** Dark mode — Indigo Night menu, faint Starlight border */
const MENU_TOKENS_DARK = `
  --elohim-menu-bg:     var(--el-night-alt);
  --elohim-menu-border: color-mix(in oklch, var(--el-starlight) 15%, transparent);
  --elohim-menu-radius: var(--el-radius-md);
  --elohim-menu-shadow: 0 4px 16px rgba(15, 26, 18, 0.4);
`;

// ---------------------------------------------------------------------------
// Protocol-realistic menu item fixtures
// ---------------------------------------------------------------------------

/**
 * The three MVP context menu items, in the protocol's brand voice.
 * Labels are stable across all light/dark/RTL stories.
 */
const mvpItems: ContextMenuItem[] = [
  { id: 'open', label: 'Open' },
  { id: 'about', label: 'About this EPR' },
  { id: 'copy', label: 'Copy EPR link' },
];

/**
 * Items for the steward-affordance context (shefa view).
 * A steward has one additional affordance: "Transfer stewardship."
 */
const stewardItems: ContextMenuItem[] = [
  { id: 'open', label: 'Open provision' },
  { id: 'about', label: 'About this commitment' },
  { id: 'transfer', label: 'Transfer stewardship' },
  { id: 'copy', label: 'Copy EPR link' },
];

/**
 * Items for the lamad concept card (learner view).
 * "Move to" is not yet available for concept links.
 */
const lamadItems: ContextMenuItem[] = [
  { id: 'open', label: 'Open concept' },
  { id: 'about', label: 'About this concept' },
  { id: 'practice', label: 'Practice this concept', disabled: true },
  { id: 'copy', label: 'Copy EPR link' },
];

// ---------------------------------------------------------------------------
// Decorator factories
// ---------------------------------------------------------------------------

/**
 * The invocation anchor the menu was opened FROM — selected, therefore amber
 * (spec §6: "Active/selected nodes use --el-amber"). Carries the golden-hour
 * warmth in every composition that uses the bare theme decorators.
 */
const ANCHOR_CHIP_STYLE = `
  position: absolute;
  inset-block-start: 1.25rem;
  inset-inline-start: var(--el-space-lg);
  font-size: 0.8125rem;
  padding: 2px 8px;
  border-radius: 4px;
  background: color-mix(in oklch, var(--el-amber) 28%, transparent);
  border: 1px solid var(--el-amber);
`;

function lightDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${MENU_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        color: var(--el-stone);
        padding: 4rem var(--el-space-lg) var(--el-space-lg);
        position: relative;
        min-block-size: 240px;
      "
    >
      <span style="${ANCHOR_CHIP_STYLE}">household-budgeting</span>
      ${story()}
    </div>
  `;
}

function darkDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${MENU_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        color: var(--el-starlight);
        color-scheme: dark;
        padding: 4rem var(--el-space-lg) var(--el-space-lg);
        position: relative;
        min-block-size: 240px;
      "
    >
      <span style="${ANCHOR_CHIP_STYLE}">household-budgeting</span>
      ${story()}
    </div>
  `;
}

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Core/elohim-context-menu',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-context-menu>\` — Elohim brand-bound fold-down context menu.

The context menu is a deliberate surface, not an ambient gesture. It appears
when a household member makes an intentional choice — right-clicking a concept
link in a lamad lesson, or long-pressing a steward affordance in a shefa
provision view. It should feel like pulling open a small, well-organized drawer,
not summoning a modal.

MVP items in the brand voice:
- "Open" — primary navigation
- "About this EPR" — epistemic transparency: who made this, when, under what covenant
- "Copy EPR link" — sharing the provision, not sharing a URL

**Token binding:**
| Property | Light | Dark |
|---|---|---|
| \`--elohim-menu-bg\` | cream | night-alt |
| \`--elohim-menu-border\` | stone 20% | starlight 15% |
| \`--elohim-menu-radius\` | 8px | 8px |
| \`--elohim-menu-shadow\` | soft warm | deep night |

**Motion:** the fold-down animation is named and explicit. The default is stillness.

Library B — graphos-designer. Primitive untouched.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Contextual placement — lamad concept card
// ---------------------------------------------------------------------------

export const LamadConceptCard: Story = {
  name: 'LamadConceptCard',
  decorators: [lightDecorator],
  render: () => html`
    <!-- Simulated concept card context -->
    <div
      style="
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 3px 10px 3px 8px;
        background: color-mix(in oklch, var(--el-stone) 8%, var(--el-cream));
        border: 1px solid color-mix(in oklch, var(--el-stone) 25%, transparent);
        border-radius: 999px;
        font-family: var(--el-font-ui);
        font-size: 0.875rem;
        color: var(--el-stone);
        position: relative;
      "
    >
      <span
        style="
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: var(--el-amber);
          flex-shrink: 0;
        "
      ></span>
      <span>fair-exchange</span>
    </div>

    <!-- Menu positioned below the concept chip — opened via right-click -->
    <elohim-context-menu
      .open=${true}
      .items=${lamadItems}
      style="position: absolute; inset-block-start: 80px; inset-inline-start: var(--el-space-lg);"
    ></elohim-context-menu>

    <p
      style="
        position: absolute;
        inset-block-start: 230px;
        inset-inline-start: var(--el-space-lg);
        font-family: var(--el-font-body);
        font-size: 0.75rem;
        color: var(--el-stone);
        opacity: 0.7;
      "
    >
      Right-clicking the "fair-exchange" concept link opens this menu.
      "Practice this concept" is disabled until the lamad concept is fully resolved.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Menu opened above a lamad concept chip — the "fair-exchange" reference in a lesson. ' +
          'The disabled "Practice this concept" item shows the brand vocabulary: ' +
          'practice, not "quiz"; concept, not "topic." The Harvest Gold shimmer dot on the chip ' +
          'is the fallback indicator — the concept is known but not yet fully resolved.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Contextual placement — steward affordance in a shefa view
// ---------------------------------------------------------------------------

export const StewardAffordance: Story = {
  name: 'StewardAffordance',
  decorators: [lightDecorator],
  render: () => html`
    <!-- Simulated shefa provision row -->
    <div
      style="
        display: flex;
        align-items: center;
        gap: var(--el-space-sm);
        padding: var(--el-space-sm) var(--el-space-md);
        background: color-mix(in oklch, var(--el-stone) 5%, var(--el-cream));
        border-radius: var(--el-radius-md);
        box-shadow: var(--el-shadow-soft);
        font-family: var(--el-font-ui);
        font-size: 0.875rem;
        color: var(--el-stone);
      "
    >
      <span style="flex: 1; font-weight: 500; color: var(--el-green-deep);">
        harvest-bounty commitment
      </span>
      <span style="opacity: 0.7; font-size: 0.8125rem;">
        Aleph household → Bethlehem household
      </span>
    </div>

    <!-- Menu positioned beside the provision row -->
    <elohim-context-menu
      .open=${true}
      .items=${stewardItems}
      style="position: absolute; inset-block-start: 100px; inset-inline-end: var(--el-space-lg);"
    ></elohim-context-menu>

    <p
      style="
        position: absolute;
        inset-block-start: 260px;
        inset-inline-start: var(--el-space-lg);
        font-family: var(--el-font-body);
        font-size: 0.75rem;
        color: var(--el-stone);
        opacity: 0.7;
      "
    >
      The steward menu adds "Transfer stewardship" — available only to the
      household steward. "Open provision" replaces generic "Open."
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Menu opened beside a shefa provision row. The steward-specific item ' +
          '"Transfer stewardship" extends the MVP set — not "Reassign", not "Delete", ' +
          'not "Archive." Stewardship is transferred; it is not removed. ' +
          '"Open provision" replaces the generic "Open" for shefa context.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Three MVP items — light theme
// ---------------------------------------------------------------------------

export const ThreeMvpItemsLight: Story = {
  name: 'ThreeMvpItems (light)',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-context-menu
      .open=${true}
      .items=${mvpItems}
      style="position: static;"
    ></elohim-context-menu>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'The three MVP items in light theme: Open, About this EPR, Copy EPR link. ' +
          'Linen background, Hearthstone text, warm shadow. The menu radius is 8px — ' +
          'soft corners, not sharp, not round-pill. The shadow says "paper on table," ' +
          'not "modal over the world."',
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
    <elohim-context-menu
      .open=${true}
      .items=${mvpItems}
      style="position: static;"
    ></elohim-context-menu>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Light theme — Linen background, Hearthstone text, warm shadow.',
      },
    },
  },
};

export const Dark: Story = {
  name: 'Dark',
  decorators: [darkDecorator],
  render: () => html`
    <elohim-context-menu
      .open=${true}
      .items=${mvpItems}
      style="position: static;"
    ></elohim-context-menu>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Constellation dark theme — Indigo Night background, Starlight text, ' +
          'deep night shadow. The menu is a small lit panel in the dark; its warmth ' +
          'comes from the Starlight text color, not from a glow effect.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Gentle (fold-down) — the named motion story the file header promises
// ---------------------------------------------------------------------------

export const GentleFoldDown: Story = {
  name: 'Gentle (fold-down)',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-context-menu
      .open=${true}
      .items=${mvpItems}
      style="position: static;"
    ></elohim-context-menu>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'The fold-down motion, named: the menu unfolds from its anchor in 120ms ' +
          'ease-out — drawing, not loading (spec §9). The animation is gated behind ' +
          '`prefers-reduced-motion: no-preference` and `(update: fast)`: stillness is ' +
          'the default, and reduced-motion users get an instant, calm appearance. ' +
          'Replay the fold by remounting the story (canvas toolbar → remount); a ' +
          'static frame cannot carry the motion, only the resting state.',
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
          ${MENU_TOKENS_LIGHT}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: 4rem var(--el-space-lg) var(--el-space-lg);
          position: relative;
          min-block-size: 240px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-context-menu
      .open=${true}
      .items=${[
        { id: 'open', label: 'פתח' },
        { id: 'about', label: 'אודות EPR זה' },
        { id: 'copy', label: 'העתק קישור EPR' },
      ]}
      style="position: static;"
    ></elohim-context-menu>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary — he-IL locale inside a dir="rtl" container. ' +
          'The menu items use logical `padding-inline` so the text aligns correctly ' +
          'under RTL. Arrow-key navigation is direction-agnostic.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Disabled item — "Practice this concept" not yet available
// ---------------------------------------------------------------------------

export const WithDisabledItem: Story = {
  name: 'WithDisabledItem',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-context-menu
      .open=${true}
      .items=${lamadItems}
      style="position: static;"
    ></elohim-context-menu>
  `,
  parameters: {
    docs: {
      description: {
        story:
          '"Practice this concept" is `aria-disabled="true"` — it is visible but not ' +
          'interactive. The brand renders disabled items at reduced opacity in Hearthstone, ' +
          'not with a strikethrough or a "coming soon" badge. ' +
          'The item is present because the function is real; it is disabled because the ' +
          'content has not yet resolved fully.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Interactive — trigger story demonstrating the full open/close lifecycle
// ---------------------------------------------------------------------------

export const Interactive: Story = {
  name: 'Interactive (trigger)',
  decorators: [
    (story: () => unknown) => html`
      <div
        style="
          ${EL_TOKENS}
          ${MENU_TOKENS_LIGHT}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          padding: var(--el-space-lg);
          position: relative;
          min-block-size: 300px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => {
    const items: ContextMenuItem[] = [
      { id: 'open', label: 'Open concept' },
      { id: 'about', label: 'About this EPR' },
      { id: 'copy', label: 'Copy EPR link' },
    ];
    return html`
      <p
        style="
          font-family: var(--el-font-body);
          font-size: 0.875rem;
          color: var(--el-stone);
          margin-block: 0 var(--el-space-sm);
          line-height: 1.6;
        "
      >
        Click the concept chip (or press Shift+F10) to open the menu.
        Arrow keys navigate; Enter selects; Escape closes.
        Events log to the browser console.
      </p>
      <div style="display: inline-block; position: relative;">
        <button
          type="button"
          style="
            display: inline-flex;
            align-items: center;
            gap: 6px;
            padding: 3px 10px 3px 8px;
            background: color-mix(in oklch, var(--el-stone) 8%, var(--el-cream));
            border: 1px solid color-mix(in oklch, var(--el-stone) 25%, transparent);
            border-radius: 999px;
            font-family: var(--el-font-ui);
            font-size: 0.875rem;
            color: var(--el-stone);
            cursor: pointer;
          "
          @click=${(e: Event) => {
            const btn = e.currentTarget as HTMLElement;
            const menu = btn.nextElementSibling as HTMLElement & { open: boolean };
            if (menu) menu.open = !menu.open;
          }}
        >
          <span style="width:6px;height:6px;border-radius:50%;background:var(--el-amber);flex-shrink:0;"></span>
          fair-exchange
        </button>
        <elohim-context-menu
          .items=${items}
          @item-select=${(e: CustomEvent<{ id: string }>) =>
            // eslint-disable-next-line no-console
            console.log('item-select:', e.detail.id)}
          @close=${() =>
            // eslint-disable-next-line no-console
            console.log('menu closed')}
        ></elohim-context-menu>
      </div>
    `;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Full interactive story: a branded concept chip triggers the context menu. ' +
          'The Harvest Gold dot on the chip is the fallback indicator. ' +
          'The menu appears and closes via the full keyboard contract. ' +
          '`item-select` and `close` events are logged to the console.',
      },
    },
  },
};
