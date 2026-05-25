/**
 * Library A default story — elohim-context-menu
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * <elohim-context-menu> is a Google Drive-style fold-down menu. It handles
 * arrow-key navigation, Enter/Space selection, Escape to close, focus trap
 * while open, and focus restoration on close.
 *
 * Stories use an interactive wrapper (button + context-menu) to show the full
 * open/close lifecycle. The element is controlled entirely via `open` and `items`.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';
import type { ContextMenuItem } from 'elohim-core';

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

const mvpItems: ContextMenuItem[] = [
  { id: 'open', label: 'Open' },
  { id: 'about', label: 'About this EPR' },
  { id: 'copy', label: 'Copy EPR link' },
];

const itemsWithDisabled: ContextMenuItem[] = [
  { id: 'open', label: 'Open' },
  { id: 'move', label: 'Move to...', disabled: true },
  { id: 'about', label: 'About this EPR' },
  { id: 'copy', label: 'Copy EPR link' },
];

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-context-menu',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-context-menu>\` — fold-down context menu for EPR link right-click.

MVP: flat list of items. Submenu support (View as..., Where this leads...)
is deferred to a fast-follow shift per design spec §7.4.

**Accessibility:** arrow-key navigation, Enter/Space to select, Escape to
close, focus trap while open (\`role="menu"\`), focus restoration on close.

**Animation:** the fold-down animation is gated behind
\`@media (prefers-reduced-motion: no-preference) and (update: fast)\`.

**Override surface:** \`--elohim-menu-bg\`, \`--elohim-menu-border\`,
\`--elohim-menu-radius\`, \`--elohim-menu-shadow\`.
**Parts:** \`menu\` (the \`<ul>\`), \`item\` (each \`<li>\`).
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Open/closed state stories
// ---------------------------------------------------------------------------

export const Closed: Story = {
  name: 'Closed (open=false renders nothing)',
  render: () => html`
    <div style="position: relative; padding: 1rem;">
      <p style="font-size: 0.875rem; opacity: 0.8;">
        The menu below has <code>open=false</code> — it is
        <code>display: none</code> and occupies no space.
      </p>
      <elohim-context-menu .items=${mvpItems}></elohim-context-menu>
      <em style="font-size: 0.75rem; opacity: 0.6;">(nothing visible below — the element is present but hidden)</em>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'When `open` is false (default) the element is `display: none`. No space is consumed and focus is not affected.',
      },
    },
  },
};

export const ThreeMvpItems: Story = {
  name: 'ThreeMvpItems (Open / About / Copy)',
  render: () => html`
    <div style="position: relative; padding: 4rem 1rem 1rem; min-block-size: 180px;">
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
        The three MVP items shown open — Open, About this EPR, Copy EPR link.
      </p>
      <elohim-context-menu
        .open=${true}
        .items=${mvpItems}
        style="position: static;"
      ></elohim-context-menu>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'The three MVP context menu items in the open state. Each item is a `<li role="menuitem">` with a minimum 44px block-size for touch-target compliance.',
      },
    },
  },
};

export const WithDisabledItem: Story = {
  name: 'WithDisabledItem',
  render: () => html`
    <div style="position: relative; padding: 4rem 1rem 1rem; min-block-size: 220px;">
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
        "Move to..." is disabled — <code>aria-disabled="true"</code>, cursor is default, arrow-key
        navigation skips it.
      </p>
      <elohim-context-menu
        .open=${true}
        .items=${itemsWithDisabled}
        style="position: static;"
      ></elohim-context-menu>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'A disabled item is rendered with `aria-disabled="true"` and `opacity: 0.5`. Arrow-key navigation skips disabled items. Click is a no-op.',
      },
    },
  },
};

export const Interactive: Story = {
  name: 'Interactive (trigger button)',
  render: () => {
    const items: ContextMenuItem[] = [
      { id: 'open', label: 'Open' },
      { id: 'about', label: 'About this EPR' },
      { id: 'copy', label: 'Copy EPR link' },
    ];
    return html`
      <div style="position: relative; display: inline-block; padding: 1rem;">
        <button
          type="button"
          style="font: inherit; padding: 0.5rem 1rem; cursor: pointer;"
          @click=${(e: Event) => {
            const btn = e.currentTarget as HTMLElement;
            const menu = btn.nextElementSibling as HTMLElement & { open: boolean };
            if (menu) menu.open = !menu.open;
          }}
        >
          Right-click me (or click to toggle)
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
          'Interactive story: a trigger button toggles `open`. Check the browser console for `item-select` and `close` events. Arrow keys, Enter, Escape, and Shift+F10 are wired per ARIA menu pattern.',
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
      <div style="background: #1a1a1a; color: #e8e8e8; color-scheme: dark; padding: 4rem 1rem 1rem; position: relative; min-block-size: 200px;">
        ${story()}
      </div>
    `,
  ],
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
          'Dark canary. Menu background uses the CSS system color `Canvas` which resolves to the dark surface under `color-scheme: dark`. Border and hover use `color-mix` from `currentColor`.',
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
      <div dir="rtl" lang="he" style="padding: 4rem 1rem 1rem; position: relative; min-block-size: 200px;">
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
          'RTL canary: menu with Hebrew labels in a right-to-left container. `padding-inline` on items renders correctly under RTL layout.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Blank-slate proof
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial; display: block; position: relative; padding: 1rem;">${story()}</div>`],
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
          'Wrapped in `style="all: initial;"`. Proves the menu renders with zero external CSS tokens. CSS system color `Canvas` provides the background; items are legible with `color: inherit`.',
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
          --elohim-menu-bg: #1a0030;
          --elohim-menu-border: #9040c0;
          --elohim-menu-radius: 0;
          --elohim-menu-shadow: 4px 4px 0 #9040c0;
          color: #f0c8ff;
          background: #0e0020;
          font-family: Courier, monospace;
          position: relative;
          padding: 4rem 1rem 1rem;
          min-block-size: 220px;
        "
      >
        ${story()}
      </div>
    `,
  ],
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
          'Override-surface proof: a deliberately non-Elohim purple retro theme applied via the four `--elohim-menu-*` CSS custom properties. Border-radius is zeroed and a hard drop shadow is applied — proving the override surface is honest.',
      },
    },
  },
};
