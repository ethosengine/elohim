/**
 * Library A default story — elohim-default-omnibar
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * <elohim-default-omnibar> is the fallback omnibar used when a bundle doesn't
 * BYO its own toolbar. It reads the `elohim_session` cookie opportunistically
 * and shows either the humanId or a sign-in link.
 *
 * Authenticated story: sets the elohim_session cookie via a story decorator
 * before render, so the Session primitive picks it up on connectedCallback.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

// ---------------------------------------------------------------------------
// Cookie helpers
// ---------------------------------------------------------------------------

/**
 * Build the elohim_session cookie value matching the CurrentUserView shape.
 * In production this cookie is set by doorway after imagodei auth.
 */
function buildSessionCookie(humanId: string, capabilities: string[], reach: string): string {
  return encodeURIComponent(JSON.stringify({ humanId, capabilities, reach }));
}

function setSessionCookie(humanId: string): void {
  const value = buildSessionCookie(humanId, ['pilot'], 'standard');
  document.cookie = `elohim_session=${value}; path=/; SameSite=Lax`;
}

function clearSessionCookie(): void {
  document.cookie = 'elohim_session=; path=/; expires=Thu, 01 Jan 1970 00:00:00 GMT';
}

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-default-omnibar',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-default-omnibar>\` — fallback omnibar for bundles that don't BYO a toolbar.

MVP shape: brand mark on the inline-start, auth indicator on the inline-end.
When the \`elohim_session\` cookie is present the humanId is shown; otherwise
a "sign in" link renders.

The session cookie is NOT polled. On \`visibilitychange\` (tab re-focus) the
element calls \`session.refreshFromCookies()\` so cross-tab auth changes are
picked up opportunistically.

**Override surface:** \`--elohim-omnibar-bg\`, \`--elohim-omnibar-border\`.
**Parts:** \`brand\`, \`user\`, \`user-name\`.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Auth-state stories
// ---------------------------------------------------------------------------

export const Anonymous: Story = {
  name: 'Anonymous (no session cookie)',
  decorators: [
    story => {
      // Ensure no session cookie bleeds in from a prior story
      clearSessionCookie();
      return story();
    },
  ],
  render: () => html`<elohim-default-omnibar></elohim-default-omnibar>`,
  parameters: {
    docs: {
      description: {
        story:
          'No `elohim_session` cookie present — the element renders the brand mark and a "sign in" link. This is the expected state for unauthenticated visitors.',
      },
    },
  },
};

export const Authenticated: Story = {
  name: 'Authenticated (session cookie pre-set)',
  decorators: [
    story => {
      // Set the cookie BEFORE render so Session.refreshFromCookies() on
      // connectedCallback finds it. The element does not poll; it reads once
      // on mount and again on visibilitychange.
      setSessionCookie('matthew.dowell');
      return story();
    },
  ],
  render: () => html`
    <div>
      <elohim-default-omnibar></elohim-default-omnibar>
      <p style="margin-block-start: 0.5rem; font-size: 0.75rem; opacity: 0.7;">
        Cookie set to <code>matthew.dowell</code> — the humanId shown in the user area.
        Clear the cookie in DevTools Application tab to return to Anonymous state.
      </p>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'A `elohim_session` cookie is pre-set in the story decorator before render, simulating a signed-in state. The omnibar shows `matthew.dowell` in the user area. In production this cookie is set by doorway after imagodei auth.',
      },
    },
  },
};

export const Unauthorized: Story = {
  name: 'Unauthorized (sign-in link)',
  decorators: [
    story => {
      clearSessionCookie();
      return story();
    },
  ],
  render: () => html`<elohim-default-omnibar></elohim-default-omnibar>`,
  parameters: {
    docs: {
      description: {
        story:
          'The `unauthorized` state (as declared in `@capabilityStates`): session cookie absent or expired. The element renders a "sign in" link pointing to `/auth/signin`. This is the designed unauthorized surface.',
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
      <div style="background: #1a1a1a; color: #e8e8e8; color-scheme: dark;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-default-omnibar></elohim-default-omnibar>`,
  parameters: {
    docs: {
      description: {
        story:
          'Dark background canary. The background uses `color-mix(in oklch, Canvas 92%, currentColor)` which adapts via `color-scheme: dark` to the system\'s dark Canvas color.',
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
      <div dir="rtl" lang="he" style="padding: 0;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-default-omnibar></elohim-default-omnibar>`,
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary: omnibar in a right-to-left container. `padding-inline` and `justify-content: space-between` ensure brand mark and user area flip correctly under RTL.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Blank-slate proof
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial; display: block;">${story()}</div>`],
  render: () => html`<elohim-default-omnibar></elohim-default-omnibar>`,
  parameters: {
    docs: {
      description: {
        story:
          'Wrapped in `style="all: initial;"`. Proves the omnibar renders (flex row, brand + user) with zero external CSS tokens. CSS system colors provide legible defaults.',
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
          --elohim-omnibar-bg: #0d1117;
          --elohim-omnibar-border: #00ff41;
          color: #00ff41;
          font-family: ui-monospace, monospace;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-default-omnibar></elohim-default-omnibar>`,
  parameters: {
    docs: {
      description: {
        story:
          'Override-surface proof: terminal-green-on-black theme applied via `--elohim-omnibar-bg` and `--elohim-omnibar-border`. Proves the override surface is honest — no hardcoded brand colors in the element.',
      },
    },
  },
};
