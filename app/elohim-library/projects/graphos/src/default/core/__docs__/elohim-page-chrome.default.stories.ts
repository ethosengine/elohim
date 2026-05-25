/**
 * Library A default story — elohim-page-chrome
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * <elohim-page-chrome> is the bundle's root wrapper. It manages the sticky
 * omnibar slot: when slot="omnibar" is populated it hides the built-in
 * <elohim-default-omnibar>; when it is empty the default omnibar renders
 * automatically.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-page-chrome',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-page-chrome>\` — the bundle's root wrapper element.

Provides a slotted omnibar contract:
- Bundles that **have** their own toolbar place it in \`slot="omnibar"\`.
  The default omnibar is suppressed automatically.
- Bundles that **don't** get \`<elohim-default-omnibar>\` rendered in the
  sticky header area automatically.

The sticky omnibar z-index is overridable via \`--elohim-chrome-omnibar-z\`.

**Parts:** \`omnibar-host\` (sticky container), \`main\` (scrolling content area).
**Slots:** \`omnibar\` (optional toolbar), default (page content).
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Omnibar slot stories
// ---------------------------------------------------------------------------

export const DefaultOmnibarFallback: Story = {
  name: 'DefaultOmnibarFallback',
  render: () => html`
    <elohim-page-chrome style="min-block-size: 200px;">
      <p style="padding: 1rem;">
        No content in <code>slot="omnibar"</code> — the built-in
        <code>&lt;elohim-default-omnibar&gt;</code> renders in the sticky header.
      </p>
    </elohim-page-chrome>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'The default state: no omnibar slotted. The element renders `<elohim-default-omnibar>` automatically in the sticky header area.',
      },
    },
  },
};

export const SlottedOmnibar: Story = {
  name: 'SlottedOmnibar',
  render: () => html`
    <elohim-page-chrome style="min-block-size: 200px;">
      <nav
        slot="omnibar"
        style="
          display: flex;
          align-items: center;
          gap: 1rem;
          padding: 0.5rem 1rem;
          background: ButtonFace;
          border-block-end: 1px solid ButtonText;
          font: inherit;
        "
      >
        <strong>My App Toolbar</strong>
        <span style="opacity: 0.6;">Custom bundle omnibar suppresses the default</span>
      </nav>
      <p style="padding: 1rem;">
        Page content. The <code>slot="omnibar"</code> is occupied — the built-in
        omnibar is hidden via the <code>?hidden</code> attribute.
      </p>
    </elohim-page-chrome>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'A custom element placed in `slot="omnibar"` suppresses the built-in default omnibar. The element detects slot assignment via `slotchange` and toggles the `hidden` attribute on `<elohim-default-omnibar>` without re-rendering.',
      },
    },
  },
};

export const WithContent: Story = {
  name: 'WithContent',
  render: () => html`
    <elohim-page-chrome style="min-block-size: 300px;">
      <section style="padding: 1.5rem; max-inline-size: 600px;">
        <h1 style="font-size: 1.5rem; margin-block-end: 0.5rem;">Page heading</h1>
        <p>
          Main content flows into the default slot beneath the sticky omnibar.
          The chrome manages the flex layout: omnibar-host is sticky at the
          block-start; main fills the remaining viewport height.
        </p>
      </section>
    </elohim-page-chrome>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Default slot populated with a page section. The `main` part fills the remaining vertical space below the sticky omnibar.',
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
      <div style="background: #1a1a1a; color: #e8e8e8; color-scheme: dark; min-block-size: 200px;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-page-chrome>
      <p style="padding: 1rem;">Page content in dark context.</p>
    </elohim-page-chrome>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Dark background canary. The omnibar host border uses CSS system colors that adapt.',
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
      <div dir="rtl" lang="he" style="min-block-size: 200px;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-page-chrome>
      <p style="padding: 1rem;">תוכן הדף מימין לשמאל.</p>
    </elohim-page-chrome>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary: chrome in a right-to-left container. The `inset-block-start` sticky positioning uses logical properties and does not change under RTL.',
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
  render: () => html`
    <elohim-page-chrome style="min-block-size: 200px;">
      <p style="padding: 1rem;">Zero-token render.</p>
    </elohim-page-chrome>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Wrapped in `style="all: initial;"`. Proves the chrome renders (flex column, sticky header) with zero external CSS tokens.',
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
          --elohim-chrome-omnibar-z: 100;
          --elohim-omnibar-bg: #0d1117;
          --elohim-omnibar-border: #00ff41;
          color: #00ff41;
          background: #0d1117;
          font-family: ui-monospace, monospace;
          min-block-size: 200px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-page-chrome>
      <p style="padding: 1rem; opacity: 0.8;">Terminal green theme via CSS custom properties.</p>
    </elohim-page-chrome>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Override-surface proof: the omnibar z-index is raised to 100 and the default omnibar receives a terminal-green theme via `--elohim-omnibar-bg` and `--elohim-omnibar-border`. Demonstrates that the chrome is a transparent relay for child-element CSS props.',
      },
    },
  },
};
