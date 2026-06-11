/**
 * Library B designed story — elohim-default-omnibar
 *
 * Binds the Elohim brand tokens to `<elohim-default-omnibar>` via story-decorator
 * overrides. The primitive itself is NEVER modified — binding happens above via
 * `--elohim-omnibar-*` CSS custom properties mapped to `--el-*` brand tokens.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — no ts-rs view dependency. The element reads `elohim_session`
 *      cookie directly. Cookie helpers from Library A are reused.
 *   2. Manifest vocabulary — humanId vocabulary uses the protocol's real agent
 *      identifiers (matthew.dowell, jessica.dowell, james.dowell).
 *   3. Brand tokens — `--el-*` palette from design spec §14 declared inline.
 *
 * Binding canvas:
 *   --elohim-omnibar-bg     → --el-cream (light) / --el-night-alt (dark)
 *   --elohim-omnibar-border → --el-stone at 20% (light) / --el-starlight at 15% (dark)
 *
 * Brand voice notes:
 *   - Anonymous state: brand mark + "join the commons" (not "sign up" or "join the network")
 *   - Authenticated state: humanId + "(steward)" or "(manager)" — role from the session
 *   - The brand mark is the word "elohim" in Fraunces — no icon, no logo file needed
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

/** Light mode — Linen omnibar, Hearthstone ink, Vineyard-tinted border */
const OMNIBAR_TOKENS_LIGHT = `
  --elohim-omnibar-bg:     var(--el-cream);
  --elohim-omnibar-fg:     var(--el-stone);
  --elohim-omnibar-border: color-mix(in oklch, var(--el-stone) 20%, transparent);
`;

/** Dark mode — Indigo Night omnibar, Starlight ink, faint Starlight border */
const OMNIBAR_TOKENS_DARK = `
  --elohim-omnibar-bg:     var(--el-night-alt);
  --elohim-omnibar-fg:     var(--el-starlight);
  --elohim-omnibar-border: color-mix(in oklch, var(--el-starlight) 15%, transparent);
`;

/**
 * Brand mark in the display voice — Fraunces at Harvest Gold via ::part
 * (custom-property inheritance can't reach parts). Makes the "word elohim in
 * Fraunces / Harvest Gold" the story docs promise actually render, and is
 * the golden-hour element of both themes.
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
// Cookie helpers (same as Library A but only for story setup)
// ---------------------------------------------------------------------------

function buildSessionCookie(humanId: string, capabilities: string[], reach: string): string {
  return encodeURIComponent(JSON.stringify({ humanId, capabilities, reach }));
}

function setSessionCookie(humanId: string, capabilities: string[] = ['pilot']): void {
  const value = buildSessionCookie(humanId, capabilities, 'standard');
  document.cookie = `elohim_session=${value}; path=/; SameSite=Lax`;
}

function clearSessionCookie(): void {
  document.cookie = 'elohim_session=; path=/; expires=Thu, 01 Jan 1970 00:00:00 GMT';
}

// ---------------------------------------------------------------------------
// Decorator factories
// ---------------------------------------------------------------------------

function lightDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${OMNIBAR_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        color: var(--el-stone);
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
        ${OMNIBAR_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        color: var(--el-starlight);
        color-scheme: dark;
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
  title: 'Designed/Core/elohim-default-omnibar',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-default-omnibar>\` — Elohim brand-bound fallback omnibar.

The omnibar is the threshold — the narrow strip that carries the protocol's
identity mark and the household member's presence indicator. It should feel
like a doorpost, not a navigation bar.

**Brand mark:** the word "elohim" in Fraunces at 1rem — understated,
unhurried, warm. Not a logo file. Not an icon.

**Anonymous state:** brand mark + "join the commons" — hospitality, not
enrollment. The protocol welcomes; it does not acquire users.

**Authenticated state:** brand mark + humanId + role. Role comes from the
session's capabilities array. A pilot is a "(steward)"; an elder is
"(elder)"; a support agent is "(elohim-support)".

**Token binding:**
| Property | Light | Dark |
|---|---|---|
| \`--elohim-omnibar-bg\` | cream | night-alt |
| \`--elohim-omnibar-border\` | stone 20% | starlight 15% |

Library B — graphos-designer. Primitive untouched.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Auth-state stories — light theme
// ---------------------------------------------------------------------------

export const AnonymousLight: Story = {
  name: 'Anonymous (light)',
  decorators: [
    (story: () => unknown) => {
      clearSessionCookie();
      return lightDecorator(story);
    },
  ],
  render: () => html`
    <div>
      <elohim-default-omnibar></elohim-default-omnibar>
      <p
        style="
          padding-inline: var(--el-space-lg);
          padding-block: var(--el-space-sm);
          font-family: var(--el-font-body);
          font-size: 0.875rem;
          color: var(--el-stone);
          line-height: 1.6;
        "
      >
        No session present. The omnibar shows the brand mark and a
        "join the commons" invitation — hospitality, not an enrollment funnel.
      </p>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Anonymous state, light theme. No `elohim_session` cookie — ' +
          'the omnibar renders the brand mark in Vineyard Fraunces and a ' +
          '"join the commons" link in Hearthstone DM Sans. ' +
          'No hero copy, no sign-up urgency, no "Get started free."',
      },
    },
  },
};

export const AuthenticatedLight: Story = {
  name: 'Authenticated (light)',
  decorators: [
    (story: () => unknown) => {
      setSessionCookie('matthew.dowell', ['pilot']);
      return lightDecorator(story);
    },
  ],
  render: () => html`
    <div>
      <elohim-default-omnibar></elohim-default-omnibar>
      <p
        style="
          padding-inline: var(--el-space-lg);
          padding-block: var(--el-space-sm);
          font-family: var(--el-font-body);
          font-size: 0.875rem;
          color: var(--el-stone);
          line-height: 1.6;
        "
      >
        Session cookie set to <code>matthew.dowell</code> with
        <code>['pilot']</code> capabilities. The omnibar shows the
        household member's humanId and their role.
      </p>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Authenticated state, light theme. The session cookie carries ' +
          '`matthew.dowell` as the humanId. The omnibar shows the brand mark ' +
          'on the inline-start and "matthew.dowell (steward)" on the inline-end. ' +
          'DM Sans for the user area; Fraunces for the brand mark.',
      },
    },
  },
};

export const AuthenticatedElderLight: Story = {
  name: 'AuthenticatedElder (light)',
  decorators: [
    (story: () => unknown) => {
      setSessionCookie('jessica.dowell', ['pilot', 'elder']);
      return lightDecorator(story);
    },
  ],
  render: () => html`
    <div>
      <elohim-default-omnibar></elohim-default-omnibar>
      <p
        style="
          padding-inline: var(--el-space-lg);
          padding-block: var(--el-space-sm);
          font-family: var(--el-font-body);
          font-size: 0.875rem;
          color: var(--el-stone);
          line-height: 1.6;
        "
      >
        Session cookie set to <code>jessica.dowell</code> with
        <code>['pilot', 'elder']</code> capabilities.
      </p>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Authenticated elder state — Jessica, who carries "elder" capability. ' +
          'The user area shows "jessica.dowell (elder)" — the role follows the humanId ' +
          'in parentheses. An elder\'s presence in the threshold is quiet but distinct.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Dark theme (constellation register)
// ---------------------------------------------------------------------------

export const AnonymousDark: Story = {
  name: 'Anonymous (dark)',
  decorators: [
    (story: () => unknown) => {
      clearSessionCookie();
      return darkDecorator(story);
    },
  ],
  render: () => html`
    <div>
      <elohim-default-omnibar></elohim-default-omnibar>
      <p
        style="
          padding-inline: var(--el-space-lg);
          padding-block: var(--el-space-sm);
          font-family: var(--el-font-body);
          font-size: 0.875rem;
          color: var(--el-starlight);
          line-height: 1.6;
        "
      >
        Anonymous state in the constellation dark theme.
        Indigo Night background, Starlight text.
      </p>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Anonymous state, constellation dark theme. Indigo Night (#1A1A2E) background, ' +
          'Harvest Gold brand mark, Starlight "join the commons" link. ' +
          'The omnibar enters the visual language of the protocol at night: ' +
          'a single lit doorpost in a dark sky.',
      },
    },
  },
};

export const AuthenticatedDark: Story = {
  name: 'Authenticated (dark)',
  decorators: [
    (story: () => unknown) => {
      setSessionCookie('matthew.dowell', ['pilot']);
      return darkDecorator(story);
    },
  ],
  render: () => html`
    <elohim-default-omnibar></elohim-default-omnibar>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Authenticated state, constellation dark theme. The brand mark is Harvest Gold ' +
          'in Fraunces; the user area is Starlight in DM Sans. ' +
          'The Indigo Night border is barely visible — warmth, not architecture.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Light / Dark pair (explicit named stories required by Library B discipline)
// ---------------------------------------------------------------------------

export const Light: Story = {
  name: 'Light',
  decorators: [
    (story: () => unknown) => {
      setSessionCookie('matthew.dowell', ['pilot']);
      return lightDecorator(story);
    },
  ],
  render: () => html`<elohim-default-omnibar></elohim-default-omnibar>`,
  parameters: {
    docs: {
      description: {
        story: 'Light theme, authenticated. Linen background, Vineyard brand mark, Hearthstone user text.',
      },
    },
  },
};

export const Dark: Story = {
  name: 'Dark',
  decorators: [
    (story: () => unknown) => {
      setSessionCookie('matthew.dowell', ['pilot']);
      return darkDecorator(story);
    },
  ],
  render: () => html`<elohim-default-omnibar></elohim-default-omnibar>`,
  parameters: {
    docs: {
      description: {
        story: 'Dark (constellation) theme, authenticated. Indigo Night background, Harvest Gold brand mark, Starlight user text.',
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
    (story: () => unknown) => {
      setSessionCookie('matthew.dowell', ['pilot']);
      return html`
        <div
          dir="rtl"
          lang="he"
          style="
            ${EL_TOKENS}
            ${OMNIBAR_TOKENS_LIGHT}
            font-family: var(--el-font-ui);
            background: var(--el-cream);
          "
        >
          ${story()}
        </div>
      `;
    },
  ],
  render: () => html`<elohim-default-omnibar></elohim-default-omnibar>`,
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary — he-IL locale inside a dir="rtl" container. ' +
          'The brand mark moves to the inline-end (visual right in RTL) and the ' +
          'user area to the inline-start (visual left). ' +
          'Logical CSS `justify-content: space-between` handles the flip automatically.',
      },
    },
  },
};
