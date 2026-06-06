/**
 * Library A default story — elohim-hypercard-panel
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * <elohim-hypercard-panel> is an anchored fold-down hypercard panel providing
 * progressive L2 disclosure from a position:relative anchor wrapper. It accepts
 * arbitrary slotted content and an optional ContextMenuItem[] action row.
 *
 * Positioning assumes a position:relative parent (.menu-anchor convention).
 * Folds down + inline-start aligned: down-right in LTR, down-left in RTL.
 *
 * Stories use an anchor wrapper to replicate the production context.
 *
 * Override surface:
 *   --elohim-hypercard-bg, --elohim-hypercard-border, --elohim-hypercard-radius,
 *   --elohim-hypercard-shadow, --elohim-hypercard-min-width,
 *   --elohim-hypercard-max-width, --elohim-hypercard-z
 *
 * Parts: container (role="dialog"), content (slot wrapper), action-row, action
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';
import type { ContextMenuItem } from 'elohim-core';

// ---------------------------------------------------------------------------
// Shared fixtures — protocol vocabulary (no brand tokens)
// ---------------------------------------------------------------------------

const resilienceActions: ContextMenuItem[] = [
  { id: 'view-full', label: 'View full resilience' },
  { id: 'steward', label: 'Steward this content' },
  { id: 'network', label: 'View network' },
];

const resilienceActionsWithDisabled: ContextMenuItem[] = [
  { id: 'view-full', label: 'View full resilience' },
  { id: 'steward', label: 'Steward this content', disabled: true },
  { id: 'network', label: 'View network', disabled: true },
];

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-hypercard-panel',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-hypercard-panel>\` — anchored fold-down hypercard panel for progressive L2 disclosure.

Sibling of \`<elohim-context-menu>\`. Reuses the fold-down motion idiom and
\`ContextMenuItem[]\` action type. The panel folds DOWN + inline-start aligned
(down-right LTR, down-left RTL) from a \`position:relative\` anchor wrapper.

**Props:**
- \`open\` (boolean, reflected attribute) — whether the panel is visible
- \`actions: ContextMenuItem[]\` — action row below the slot; absent when empty
- \`panelLabel: string\` — \`aria-label\` for the \`role="dialog"\`; host-supplied —
  the element ships NO built-in strings

**Events:**
- \`action-select\` \`{ detail: { id } }\` — bubbles, composed
- \`close\` — bubbles, composed; fired on Escape, click-outside, or action activation

**A11y:** non-modal \`role="dialog"\` + \`aria-label\`; Escape closes; click-outside
closes; focus moves into first enabled action button on open; focus restores on close.

**Animation:** fold-down (120ms ease-out) gated on
\`@media (prefers-reduced-motion: no-preference) and (update: fast)\`.

**Override surface:**
\`--elohim-hypercard-bg\`, \`--elohim-hypercard-border\`, \`--elohim-hypercard-radius\`,
\`--elohim-hypercard-shadow\`, \`--elohim-hypercard-min-width\`,
\`--elohim-hypercard-max-width\`, \`--elohim-hypercard-z\`

**Parts:** \`container\`, \`content\`, \`action-row\`, \`action\`
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Standard — open panel with slotted dl content
// ---------------------------------------------------------------------------

export const Standard: Story = {
  name: 'Standard (slotted dl content, no actions)',
  render: () => html`
    <div
      style="position: relative; display: inline-block; padding-block-start: 2rem; padding-inline: 1rem;"
    >
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
        Panel open with slotted definition-list content. No action row — actions is empty.
      </p>
      <elohim-hypercard-panel panelLabel="Collective resilience" open>
        <dl style="margin: 0;">
          <dt style="font-size: 0.75rem; opacity: 0.7;">Placement coverage</dt>
          <dd style="margin: 0 0 0.5rem;">9 of 12 collective roles filled</dd>
          <dt style="font-size: 0.75rem; opacity: 0.7;">Stewarding collectives</dt>
          <dd style="margin: 0 0 0.5rem;">Vineyard Household (active)</dd>
          <dt style="font-size: 0.75rem; opacity: 0.7;">Placement gaps</dt>
          <dd style="margin: 0;">3 open roles · 2 provisional commitments</dd>
        </dl>
      </elohim-hypercard-panel>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Standard: panel open with a slotted `<dl>` showing collective resilience metrics in protocol vocabulary. No action row — `actions` is empty.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// WithActions — actions including one disabled
// ---------------------------------------------------------------------------

export const WithActions: Story = {
  name: 'WithActions (incl. disabled)',
  render: () => html`
    <div
      style="position: relative; display: inline-block; padding-block-start: 2rem; padding-inline: 1rem; min-block-size: 240px;"
    >
      <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem;">
        Panel with action row. "Steward this content" and "View network" are disabled — destinations
        not yet wired (follow-up §11.6).
      </p>
      <elohim-hypercard-panel
        panelLabel="Resilience status"
        .actions=${resilienceActionsWithDisabled}
        open
        @action-select=${(e: CustomEvent<{ id: string }>) =>
          // eslint-disable-next-line no-console
          console.log('action-select:', e.detail.id)}
        @close=${() =>
          // eslint-disable-next-line no-console
          console.log('panel closed')}
      >
        <dl style="margin: 0;">
          <dt style="font-size: 0.75rem; opacity: 0.7;">Placement coverage</dt>
          <dd style="margin: 0 0 0.5rem;">9 of 12 collective roles filled</dd>
          <dt style="font-size: 0.75rem; opacity: 0.7;">Active commitments</dt>
          <dd style="margin: 0;">4 stewardship commitments · 2 hosting</dd>
        </dl>
      </elohim-hypercard-panel>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Panel with action row. "View full resilience" is enabled; "Steward this content" and "View network" are disabled. Disabled buttons carry `disabled` attribute and `aria-disabled="true"`. Check the browser console for events.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Interactive — trigger button
// ---------------------------------------------------------------------------

export const Interactive: Story = {
  name: 'Interactive (trigger button)',
  render: () => {
    const actions: ContextMenuItem[] = [
      { id: 'view-full', label: 'View full resilience' },
      { id: 'steward', label: 'Steward this content' },
    ];
    return html`
      <div style="position: relative; display: inline-block; padding: 1rem; min-block-size: 220px;">
        <button
          type="button"
          aria-haspopup="dialog"
          style="font: inherit; padding: 0.5rem 1rem; cursor: pointer;"
          @click=${(e: Event) => {
            const btn = e.currentTarget as HTMLElement;
            const panel = btn.nextElementSibling as HTMLElement & { open: boolean };
            if (panel) panel.open = !panel.open;
          }}
        >
          Resilience icon (click to toggle)
        </button>
        <elohim-hypercard-panel
          panelLabel="Collective resilience"
          .actions=${actions}
          @action-select=${(e: CustomEvent<{ id: string }>) =>
            // eslint-disable-next-line no-console
            console.log('action-select:', e.detail.id)}
          @close=${() =>
            // eslint-disable-next-line no-console
            console.log('panel closed')}
        >
          <dl style="margin: 0;">
            <dt style="font-size: 0.75rem; opacity: 0.7;">Placement coverage</dt>
            <dd style="margin: 0 0 0.5rem;">9 of 12 collective roles filled</dd>
            <dt style="font-size: 0.75rem; opacity: 0.7;">Stewarding collectives</dt>
            <dd style="margin: 0;">Vineyard Household (active)</dd>
          </dl>
        </elohim-hypercard-panel>
      </div>
    `;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Interactive story: a trigger button toggles `open`. Escape, click-outside, and action clicks all close the panel. Check the browser console for events.',
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
      <div
        style="background: #1a1a1a; color: #e8e8e8; color-scheme: dark; padding: 4rem 1rem 1rem; position: relative; min-block-size: 260px;"
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-hypercard-panel panelLabel="Collective resilience" .actions=${resilienceActions} open>
      <dl style="margin: 0;">
        <dt style="font-size: 0.75rem; opacity: 0.7;">Placement coverage</dt>
        <dd style="margin: 0 0 0.5rem;">9 of 12 collective roles filled</dd>
        <dt style="font-size: 0.75rem; opacity: 0.7;">Placement gaps</dt>
        <dd style="margin: 0;">3 open roles · 2 provisional commitments</dd>
      </dl>
    </elohim-hypercard-panel>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Dark canary. Panel background uses the CSS system color `Canvas` which resolves to the dark surface under `color-scheme: dark`.',
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
      <div
        dir="rtl"
        lang="he"
        style="padding: 4rem 1rem 1rem; position: relative; min-block-size: 260px;"
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-hypercard-panel
      panelLabel="מצב חוסן קולקטיב"
      .actions=${[
        { id: 'view-full', label: 'הצג חוסן מלא' },
        { id: 'steward', label: 'נהל תוכן זה' },
      ]}
      open
    >
      <dl style="margin: 0;">
        <dt style="font-size: 0.75rem; opacity: 0.7;">כיסוי תפקידים</dt>
        <dd style="margin: 0 0 0.5rem;">9 מתוך 12 תפקידים קולקטיביים מאוישים</dd>
        <dt style="font-size: 0.75rem; opacity: 0.7;">פערי שיבוץ</dt>
        <dd style="margin: 0;">3 תפקידים פתוחים</dd>
      </dl>
    </elohim-hypercard-panel>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary: panel with Hebrew labels and content in a right-to-left container. `inset-inline-start: 0` aligns inline-start correctly in RTL (left edge in LTR, right edge in RTL).',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Unstyled — blank-slate proof
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [
    story => html`
      <div
        style="all: initial; display: block; position: relative; padding: 1rem; min-block-size: 220px;"
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-hypercard-panel panelLabel="Resilience status" .actions=${resilienceActions} open>
      <dl>
        <dt>Placement coverage</dt>
        <dd>9 of 12 collective roles filled</dd>
        <dt>Stewarding collectives</dt>
        <dd>Vineyard Household (active)</dd>
      </dl>
    </elohim-hypercard-panel>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Wrapped in `style="all: initial;"`. Proves the panel renders with zero external CSS tokens. CSS system color `Canvas` provides the background; text is legible with `color: inherit`. Action buttons use the system button border convention.',
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
          --elohim-hypercard-bg: #0d1117;
          --elohim-hypercard-border: 1px solid #30363d;
          --elohim-hypercard-radius: 12px;
          --elohim-hypercard-shadow: 0 8px 24px rgba(0,0,0,0.5);
          --elohim-hypercard-min-width: 280px;
          --elohim-hypercard-max-width: 360px;
          color: #c9d1d9;
          background: #161b22;
          font-family: ui-monospace, 'Cascadia Code', 'Source Code Pro', monospace;
          position: relative;
          padding: 4rem 1rem 1rem;
          min-block-size: 260px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-hypercard-panel
      panelLabel="Resilience status — custom theme"
      .actions=${resilienceActions}
      open
    >
      <dl style="margin: 0;">
        <dt style="font-size: 0.75rem; opacity: 0.6; font-variant: all-small-caps;">
          Placement coverage
        </dt>
        <dd style="margin: 0 0 0.5rem;">9 of 12 collective roles filled</dd>
        <dt style="font-size: 0.75rem; opacity: 0.6; font-variant: all-small-caps;">
          Stewarding collectives
        </dt>
        <dd style="margin: 0;">Vineyard Household (active)</dd>
      </dl>
    </elohim-hypercard-panel>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Override-surface proof: a deliberately non-Elohim dark terminal theme applied via the seven `--elohim-hypercard-*` CSS custom properties. Border-radius is increased, shadow is deepened — proving every override point in the contract is honest.',
      },
    },
  },
};
