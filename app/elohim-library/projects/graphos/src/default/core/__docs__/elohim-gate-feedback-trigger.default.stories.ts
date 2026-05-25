/**
 * Library A default story — elohim-gate-feedback-trigger
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * <elohim-gate-feedback-trigger> is a governance feedback trigger with an
 * inline modal (flag, challenge, feedback, report). The menu opens on click
 * and displays a modal for the selected feedback type.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-gate-feedback-trigger',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-gate-feedback-trigger>\` — governance feedback trigger with inline modal.

Click the trigger button to open a dropdown menu (Flag, Challenge, Feedback, Report).
Click a menu item to open the modal for that feedback type.
Submit requires non-empty text in the modal textarea.
Emits \`feedback-submit\` with the type and content ID.

**Override surface:** \`--elohim-gate-trigger-*\`, \`--elohim-gate-modal-*\`.
**Parts:** \`trigger-btn\`, \`menu\`, \`menu-item\`, \`modal-overlay\`, \`modal-panel\`, \`modal-title\`, \`modal-textarea\`, \`submit-btn\`, \`close-btn\`.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Core stories
// ---------------------------------------------------------------------------

export const Default: Story = {
  name: 'Default (closed)',
  render: () => html`
    <div style="padding: 1rem; display: flex; justify-content: flex-end;">
      <elohim-gate-feedback-trigger content-id="test-content-1"></elohim-gate-feedback-trigger>
    </div>
  `,
};

export const Interactive: Story = {
  name: 'Interactive (click to open menu)',
  render: () => {
    const onSubmit = (type: string, contentId: string, text: string) =>
      // eslint-disable-next-line no-console
      console.log('feedback-submit:', { type, contentId, text });
    return html`
      <div style="padding: 1rem; display: flex; justify-content: flex-end; min-block-size: 200px;">
        <p style="font-size: 0.875rem; opacity: 0.8; margin-block-end: 0.5rem; flex: 1;">
          Click the trigger to open the menu, then select a feedback type.
          Events are logged to the console.
        </p>
        <elohim-gate-feedback-trigger
          content-id="demo-content"
          .onFeedbackSubmit=${onSubmit}
        ></elohim-gate-feedback-trigger>
      </div>
    `;
  },
};

export const WithCustomMenuItems: Story = {
  name: 'WithCustomMenuItems',
  render: () => html`
    <div style="padding: 1rem; display: flex; justify-content: flex-end; min-block-size: 200px;">
      <elohim-gate-feedback-trigger
        content-id="test-content-2"
        .menuItems=${[
          { type: 'flag', label: 'Flag content', icon: '🚩' },
          { type: 'challenge', label: 'Challenge accuracy', icon: '⚖️' },
        ]}
      ></elohim-gate-feedback-trigger>
    </div>
  `,
  parameters: {
    docs: {
      description: { story: 'Custom menu items — only Flag and Challenge shown.' },
    },
  },
};

// ---------------------------------------------------------------------------
// Theme canaries
// ---------------------------------------------------------------------------

export const Dark: Story = {
  name: 'Dark',
  decorators: [
    story => html`
      <div style="background: #1a1a1a; color: #e8e8e8; color-scheme: dark; padding: 1.5rem; display: flex; justify-content: flex-end;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-gate-feedback-trigger content-id="dark-test"></elohim-gate-feedback-trigger>`,
};

export const RTLCanary: Story = {
  name: 'RTLCanary',
  decorators: [
    story => html`
      <div dir="rtl" lang="he" style="padding: 1rem; display: flex; justify-content: flex-start;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-gate-feedback-trigger content-id="rtl-test"></elohim-gate-feedback-trigger>`,
};

// ---------------------------------------------------------------------------
// Blank-slate and override-surface proofs
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial; display: block; padding: 1rem;">${story()}</div>`],
  render: () => html`<elohim-gate-feedback-trigger></elohim-gate-feedback-trigger>`,
  parameters: {
    docs: {
      description: {
        story: 'Wrapped in `style="all: initial;"`. Proves the trigger renders with zero external tokens.',
      },
    },
  },
};

export const CustomTheme: Story = {
  name: 'CustomTheme (override-surface proof)',
  decorators: [
    story => html`
      <div
        style="
          --elohim-gate-trigger-bg: #1a0a0a;
          --elohim-gate-trigger-fg: #fca5a5;
          --elohim-gate-trigger-border: 1px solid #ef4444;
          --elohim-gate-menu-bg: #1a0a0a;
          --elohim-gate-menu-border: 1px solid #ef4444;
          --elohim-gate-menu-item-hover-bg: #2a1010;
          --elohim-gate-modal-bg: #1a0a0a;
          --elohim-gate-modal-fg: #fca5a5;
          background: #0a0000;
          color: #fca5a5;
          font-family: ui-monospace, monospace;
          padding: 1.5rem;
          display: flex;
          justify-content: flex-end;
          min-block-size: 200px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`<elohim-gate-feedback-trigger content-id="custom-test"></elohim-gate-feedback-trigger>`,
  parameters: {
    docs: {
      description: {
        story: 'Deliberately non-Elohim red-alert theme. Proves the override surface is honest.',
      },
    },
  },
};
