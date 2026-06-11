/**
 * Library B designed story — elohim-page-chrome
 *
 * Binds the Elohim brand tokens to `<elohim-page-chrome>` via story-decorator
 * overrides. The primitive itself is NEVER modified — binding happens above via
 * `--elohim-chrome-omnibar-z` and the child `--elohim-omnibar-*` properties
 * mapped to `--el-*` brand tokens.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — elohim-page-chrome is a structural container. No ts-rs view.
 *   2. Manifest vocabulary — three domain surfaces demonstrated:
 *      lamad learning surface, shefa household balance, qahal proposal listing.
 *   3. Brand tokens — `--el-*` palette from design spec §14 declared inline.
 *
 * What Library B adds beyond Library A:
 *   - The omnibar is brand-bound (Linen surface, Vineyard border, DM Sans)
 *   - Slotted content shows realistic protocol scenes, not placeholder text
 *   - The domain-specific toolbar story shows a lamad toolbar with the brand voice
 *   - Dark mode (constellation) named explicitly
 *
 * Three contextual placements:
 *   1. Lamad learning surface — omnibar shows concept nav affordance
 *   2. Shefa household balance view — slotted domain toolbar with provision vocab
 *   3. Qahal proposal listing — slotted domain toolbar with governance vocab
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

/** Light mode — warm Linen omnibar, Hearthstone ink, Vineyard border */
const CHROME_TOKENS_LIGHT = `
  --elohim-chrome-omnibar-z: 10;
  --elohim-omnibar-bg:       var(--el-cream);
  --elohim-omnibar-fg:       var(--el-stone);
  --elohim-omnibar-border:   color-mix(in oklch, var(--el-stone) 20%, transparent);
`;

/** Dark mode — Indigo Night omnibar, Starlight ink, subtle starlight border */
const CHROME_TOKENS_DARK = `
  --elohim-chrome-omnibar-z: 10;
  --elohim-omnibar-bg:       var(--el-night-alt);
  --elohim-omnibar-fg:       var(--el-starlight);
  --elohim-omnibar-border:   color-mix(in oklch, var(--el-starlight) 15%, transparent);
`;

/**
 * The omnibar brand mark in the protocol's display voice — Fraunces, Harvest
 * Gold. Part-level styling has to travel as a <style> element (custom-property
 * inheritance can't reach ::part), shared by both theme decorators. This is
 * what makes the "Harvest Gold brand mark" the story docs describe actually
 * render, and it carries the golden-hour warmth in every chrome composition.
 */
const BRAND_MARK_CSS = html`
  <style>
    elohim-default-omnibar::part(brand) {
      font-family: var(--el-font-display);
      font-weight: 600;
      color: var(--el-amber);
    }
  </style>
`;

// ---------------------------------------------------------------------------
// Shared surface styles for slotted content
// ---------------------------------------------------------------------------

const SURFACE_LIGHT = `
  background: var(--el-cream);
  color: var(--el-stone);
  font-family: var(--el-font-body);
`;

const SURFACE_DARK = `
  background: var(--el-night);
  color: var(--el-starlight);
  font-family: var(--el-font-body);
`;

// ---------------------------------------------------------------------------
// Decorator factories
// ---------------------------------------------------------------------------

function lightDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${CHROME_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        min-block-size: 300px;
      "
    >
      ${BRAND_MARK_CSS}${story()}
    </div>
  `;
}

function darkDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${CHROME_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        color-scheme: dark;
        min-block-size: 300px;
      "
    >
      ${BRAND_MARK_CSS}${story()}
    </div>
  `;
}

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Core/elohim-page-chrome',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-page-chrome>\` — Elohim brand-bound root page wrapper.

The chrome provides the sticky omnibar contract: when a bundle brings its own
toolbar it occupies \`slot="omnibar"\` and the built-in
\`<elohim-default-omnibar>\` is suppressed. When the slot is empty the
brand-bound default omnibar renders automatically.

In brand terms: the omnibar is the threshold — the narrow strip that carries
the protocol's identity mark and the household member's presence indicator.
It should feel like a doorpost, not a navigation bar.

**Token binding:**
| Property | Light | Dark |
|---|---|---|
| \`--elohim-omnibar-bg\` | cream | night-alt |
| \`--elohim-omnibar-border\` | stone 20% | starlight 15% |
| \`--elohim-chrome-omnibar-z\` | 10 | 10 |

Library B — graphos-designer. Primitive untouched.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Default omnibar — brand bound
// ---------------------------------------------------------------------------

export const DefaultOmnibarBound: Story = {
  name: 'DefaultOmnibarBound',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-page-chrome>
      <section
        style="
          ${SURFACE_LIGHT}
          padding: var(--el-space-lg);
          max-inline-size: 640px;
        "
      >
        <h1
          style="
            font-family: var(--el-font-display);
            font-size: 2rem;
            font-weight: 400;
            color: var(--el-green-deep);
            margin-block: 0 var(--el-space-sm);
          "
        >
          Welcome to the commons
        </h1>
        <p style="font-size: 1rem; line-height: 1.7; color: var(--el-stone);">
          Your household is connected. The fair-exchange concept is available
          on your learning path. Aleph household has a harvest-bounty provision
          ready to share.
        </p>
      </section>
    </elohim-page-chrome>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Default state — no omnibar slotted, so `<elohim-default-omnibar>` renders ' +
          'automatically in the sticky header, bound to the Linen + Vineyard palette. ' +
          'The slotted content shows a household welcome surface in Source Serif 4.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Lamad learning surface — domain toolbar slotted
// ---------------------------------------------------------------------------

export const LamadLearningToolbar: Story = {
  name: 'LamadLearningToolbar',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-page-chrome>
      <!-- Lamad domain toolbar in the omnibar slot -->
      <nav
        slot="omnibar"
        role="toolbar"
        aria-label="Lamad learning toolbar"
        style="
          display: flex;
          align-items: center;
          gap: var(--el-space-md);
          padding-inline: var(--el-space-lg);
          padding-block: var(--el-space-xs);
          background: var(--el-cream);
          border-block-end: 1px solid color-mix(in oklch, var(--el-stone) 20%, transparent);
          font-family: var(--el-font-ui);
          color: var(--el-stone);
          font-size: 0.875rem;
        "
      >
        <!-- Protocol brand mark position (inline-start) -->
        <span
          style="
            font-family: var(--el-font-display);
            font-size: 1rem;
            color: var(--el-green-deep);
            font-weight: 600;
            letter-spacing: -0.01em;
          "
        >elohim</span>
        <!-- Lamad breadcrumb -->
        <span style="color: color-mix(in oklch, var(--el-stone) 40%, transparent);">/</span>
        <span style="color: var(--el-green-deep); font-weight: 500;">fair-exchange</span>
        <!-- Spacer -->
        <span style="flex: 1;"></span>
        <!-- Lamad context affordances (inline-end) -->
        <button
          type="button"
          style="
            font: inherit;
            background: none;
            border: none;
            color: var(--el-stone);
            cursor: pointer;
            padding: 4px 8px;
            border-radius: var(--el-radius-sm);
          "
        >Practice</button>
        <button
          type="button"
          style="
            font: inherit;
            background: none;
            border: none;
            color: var(--el-stone);
            cursor: pointer;
            padding: 4px 8px;
            border-radius: var(--el-radius-sm);
          "
        >Reflect</button>
      </nav>

      <!-- Lamad content surface -->
      <article
        style="
          ${SURFACE_LIGHT}
          padding: var(--el-space-xl) var(--el-space-lg);
          max-inline-size: 680px;
        "
      >
        <h1
          style="
            font-family: var(--el-font-display);
            font-size: 2.25rem;
            font-weight: 400;
            font-variant-ligatures: none;
            color: var(--el-green-deep);
            margin-block: 0 var(--el-space-sm);
          "
        >
          fair-exchange
        </h1>
        <p style="font-size: 1.0625rem; line-height: 1.75; color: var(--el-stone); margin-block: 0;">
          A provision is fair-exchange when both parties confirm that what was
          given was worth what was received — not by an algorithm, but by
          their own account. The protocol records both accounts and holds the
          difference as a learning signal.
        </p>
      </article>
    </elohim-page-chrome>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Lamad learning surface with a brand-bound domain toolbar. The toolbar shows ' +
          'the protocol\'s brand mark in Fraunces, a lamad breadcrumb in Vineyard, ' +
          'and "Practice / Reflect" affordances in DM Sans. ' +
          'The concept body is set in Source Serif 4 at generous measure.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Shefa household balance — domain toolbar slotted
// ---------------------------------------------------------------------------

export const ShefaBalanceToolbar: Story = {
  name: 'ShefaBalanceToolbar',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-page-chrome>
      <nav
        slot="omnibar"
        role="toolbar"
        aria-label="Shefa stewardship toolbar"
        style="
          display: flex;
          align-items: center;
          gap: var(--el-space-md);
          padding-inline: var(--el-space-lg);
          padding-block: var(--el-space-xs);
          background: var(--el-cream);
          border-block-end: 1px solid color-mix(in oklch, var(--el-stone) 20%, transparent);
          font-family: var(--el-font-ui);
          color: var(--el-stone);
          font-size: 0.875rem;
        "
      >
        <span
          style="
            font-family: var(--el-font-display);
            font-size: 1rem;
            color: var(--el-green-deep);
            font-weight: 600;
            letter-spacing: -0.01em;
          "
        >elohim</span>
        <span style="color: color-mix(in oklch, var(--el-stone) 40%, transparent);">/</span>
        <span style="color: var(--el-green-deep); font-weight: 500;">Aleph household · stewardship</span>
        <span style="flex: 1;"></span>
        <button
          type="button"
          style="
            font: inherit;
            background: none;
            border: none;
            color: var(--el-stone);
            cursor: pointer;
            padding: 4px 8px;
            border-radius: var(--el-radius-sm);
          "
        >New provision</button>
      </nav>

      <section
        style="
          ${SURFACE_LIGHT}
          padding: var(--el-space-xl) var(--el-space-lg);
          max-inline-size: 680px;
        "
      >
        <h1
          style="
            font-family: var(--el-font-display);
            font-size: 2rem;
            font-weight: 400;
            color: var(--el-green-deep);
            margin-block: 0 var(--el-space-xs);
          "
        >
          Aleph household
        </h1>
        <p style="font-size: 0.875rem; color: var(--el-stone); margin-block: 0 var(--el-space-lg);">
          Stewardship balance as of the first of Harvest month
        </p>
        <p
          style="
            font-family: var(--el-font-display);
            font-size: 3rem;
            font-weight: 300;
            color: var(--el-amber);
            margin-block: 0;
            line-height: 1;
          "
        >
          15 GB
        </p>
        <p style="font-size: 0.875rem; color: var(--el-stone); margin-block: var(--el-space-xs) 0;">
          stewarded for neighbors · 5 GB free for your household
        </p>
      </section>
    </elohim-page-chrome>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Shefa household balance view with a domain toolbar. ' +
          'The balance figure is set in Fraunces at 3rem — the display font carries ' +
          'numerical weight. "Stewarded for neighbors" uses the protocol vocabulary: ' +
          'provision, stewardship, neighbors — never "shared storage", "used space", "users."',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Qahal proposal listing — domain toolbar slotted
// ---------------------------------------------------------------------------

export const QahalProposalToolbar: Story = {
  name: 'QahalProposalToolbar',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-page-chrome>
      <nav
        slot="omnibar"
        role="toolbar"
        aria-label="Qahal governance toolbar"
        style="
          display: flex;
          align-items: center;
          gap: var(--el-space-md);
          padding-inline: var(--el-space-lg);
          padding-block: var(--el-space-xs);
          background: var(--el-cream);
          border-block-end: 1px solid color-mix(in oklch, var(--el-stone) 20%, transparent);
          font-family: var(--el-font-ui);
          color: var(--el-stone);
          font-size: 0.875rem;
        "
      >
        <span
          style="
            font-family: var(--el-font-display);
            font-size: 1rem;
            color: var(--el-green-deep);
            font-weight: 600;
            letter-spacing: -0.01em;
          "
        >elohim</span>
        <span style="color: color-mix(in oklch, var(--el-stone) 40%, transparent);">/</span>
        <span style="color: var(--el-green-deep); font-weight: 500;">Sunrise qahal · proposals</span>
        <span style="flex: 1;"></span>
        <button
          type="button"
          style="
            font: inherit;
            background: none;
            border: none;
            color: var(--el-stone);
            cursor: pointer;
            padding: 4px 8px;
            border-radius: var(--el-radius-sm);
          "
        >Propose</button>
      </nav>

      <section
        style="
          ${SURFACE_LIGHT}
          padding: var(--el-space-lg);
          max-inline-size: 680px;
        "
      >
        <h1
          style="
            font-family: var(--el-font-display);
            font-size: 1.75rem;
            font-weight: 400;
            color: var(--el-green-deep);
            margin-block: 0 var(--el-space-md);
          "
        >
          Open proposals
        </h1>
        <!-- Proposal list — realistic content -->
        <ol style="list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: var(--el-space-sm);">
          <li
            style="
              padding: var(--el-space-md);
              background: color-mix(in oklch, var(--el-stone) 5%, var(--el-cream));
              border-radius: var(--el-radius-md);
              box-shadow: var(--el-shadow-soft);
            "
          >
            <p style="font-weight: 500; color: var(--el-green-deep); margin-block: 0 4px; font-family: var(--el-font-ui);">
              covenant-of-care revision
            </p>
            <p style="font-size: 0.875rem; color: var(--el-stone); margin-block: 0; font-family: var(--el-font-body);">
              Proposed by Matthew · 3 of 7 affirmed · closes in 2 days
            </p>
          </li>
          <li
            style="
              padding: var(--el-space-md);
              background: color-mix(in oklch, var(--el-stone) 5%, var(--el-cream));
              border-radius: var(--el-radius-md);
              box-shadow: var(--el-shadow-soft);
            "
          >
            <p style="font-weight: 500; color: var(--el-green-deep); margin-block: 0 4px; font-family: var(--el-font-ui);">
              storage stewardship threshold increase
            </p>
            <p style="font-size: 0.875rem; color: var(--el-stone); margin-block: 0; font-family: var(--el-font-body);">
              Proposed by Jessica · 5 of 7 affirmed · closes in 4 days
            </p>
          </li>
        </ol>
      </section>
    </elohim-page-chrome>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Qahal proposal listing with a governance domain toolbar. ' +
          '"Propose" not "Create new", "affirmed" not "voted yes", "covenant-of-care" ' +
          'not "policy update." The proposal cards use warm Linen surface with ' +
          'the soft shadow — provisional infrastructure that feels composable.',
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
    <elohim-page-chrome>
      <section
        style="
          ${SURFACE_LIGHT}
          padding: var(--el-space-lg);
          max-inline-size: 640px;
        "
      >
        <h1
          style="
            font-family: var(--el-font-display);
            font-size: 1.75rem;
            font-weight: 400;
            color: var(--el-green-deep);
            margin-block: 0 var(--el-space-xs);
          "
        >Your commons</h1>
        <p style="font-size: 1rem; line-height: 1.7; color: var(--el-stone);">
          The Aleph household has stewarded 15 GB for its neighbors this month.
        </p>
      </section>
    </elohim-page-chrome>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Light theme — Linen omnibar background, Vineyard brand mark, ' +
          'warm earth page surface. The default omnibar renders automatically ' +
          'because no slot="omnibar" content is provided.',
      },
    },
  },
};

export const Dark: Story = {
  name: 'Dark',
  decorators: [darkDecorator],
  render: () => html`
    <elohim-page-chrome>
      <section
        style="
          ${SURFACE_DARK}
          padding: var(--el-space-lg);
          max-inline-size: 640px;
        "
      >
        <h1
          style="
            font-family: var(--el-font-display);
            font-size: 1.75rem;
            font-weight: 400;
            color: var(--el-amber);
            margin-block: 0 var(--el-space-xs);
          "
        >Your commons</h1>
        <p style="font-size: 1rem; line-height: 1.7; color: var(--el-starlight);">
          The Aleph household has stewarded 15 GB for its neighbors this month.
        </p>
      </section>
    </elohim-page-chrome>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Constellation dark theme — Indigo Night omnibar background, ' +
          'Harvest Gold brand mark, Starlight page text. ' +
          'The chrome enters the visual register of reading stars by their own light.',
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
          ${CHROME_TOKENS_LIGHT}
          font-family: var(--el-font-ui);
          min-block-size: 300px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-page-chrome>
      <section
        style="
          ${SURFACE_LIGHT}
          padding: var(--el-space-lg);
          max-inline-size: 640px;
        "
      >
        <h1
          style="
            font-family: var(--el-font-display);
            font-size: 1.75rem;
            font-weight: 400;
            color: var(--el-green-deep);
            margin-block: 0 var(--el-space-xs);
          "
        >בית השיתוף שלכם</h1>
        <p style="font-size: 1rem; line-height: 1.7; color: var(--el-stone);">
          משק הבית אלף העביר 15 GB לשכניו החודש.
        </p>
      </section>
    </elohim-page-chrome>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary — he-IL locale inside a dir="rtl" container. The sticky omnibar ' +
          'uses logical CSS so brand mark and user area flip naturally. ' +
          'The Hebrew page content reads right-to-left under the brand-bound header.',
      },
    },
  },
};
