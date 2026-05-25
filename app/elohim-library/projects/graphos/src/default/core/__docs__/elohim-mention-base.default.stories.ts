/**
 * Library A default story — elohim-mention-base
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * <elohim-mention-base> is the generic fallback chip for any EPR reference
 * whose pillar-specific mention element is not loaded. Renders a dotted chip
 * with label + EPR id from whatever metadata is resolvable.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-mention-base',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-mention-base>\` — generic fallback chip for EPR references.

Pillar mention elements (e.g. \`<elohim-lamad-mention>\`) supersede this element
when the relevant pillar bundle is loaded. This element is the safe
substrate-level default.

Renders a small inline chip with a fallback-indicator dot, an optional display
label, and the EPR identifier. When \`label\` is provided, both the label and
the id are shown; when \`label\` is absent, the \`epr\` value is the chip text.

**Override surface:** \`--elohim-mention-bg\`, \`--elohim-mention-fg\`,
\`--elohim-mention-border\`, \`--elohim-mention-fallback-dot\`,
\`--elohim-mention-radius\`.

**Parts:** \`chip\`, \`fallback-mark\`, \`label\`, \`id\`.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// State stories
// ---------------------------------------------------------------------------

export const EprOnly: Story = {
  name: 'EprOnly',
  render: () => html`
    <elohim-mention-base epr="epr:lamad-spa"></elohim-mention-base>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'No label provided — the EPR identifier is the chip text. This is the state before any resolution metadata is available.',
      },
    },
  },
};

export const WithLabel: Story = {
  name: 'WithLabel',
  render: () => html`
    <elohim-mention-base
      epr="epr:lamad-spa"
      label="Lamad Learning Platform"
    ></elohim-mention-base>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Label is known — the human-readable title leads, with the EPR id shown at reduced opacity alongside it.',
      },
    },
  },
};

export const WithPillarHint: Story = {
  name: 'WithPillarHint',
  render: () => html`
    <elohim-mention-base
      epr="epr:shefa-commitment-7a3f"
      label="Compute commitment"
      pillar="shefa"
    ></elohim-mention-base>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'The `pillar` attribute is set (reserved for iconography in a future shift). Currently has no visible effect beyond being present in the DOM for pillar-specific upgrade logic.',
      },
    },
  },
};

export const Empty: Story = {
  name: 'Empty (no epr)',
  render: () => html`<elohim-mention-base></elohim-mention-base>`,
  parameters: {
    docs: {
      description: {
        story:
          'Empty state: no epr or label provided. Renders the chip with the fallback dot and empty text — the designed empty state.',
      },
    },
  },
};

export const MultipleInFlow: Story = {
  name: 'MultipleInFlow',
  render: () => html`
    <p style="line-height: 2; padding: 1rem; max-inline-size: 600px;">
      The agent reviewed
      <elohim-mention-base epr="epr:lamad-spa" label="Lamad Learning Platform"></elohim-mention-base>
      and confirmed that
      <elohim-mention-base epr="epr:shefa-commitment-7a3f" label="Compute commitment"></elohim-mention-base>
      was fulfilled per the stewardship schedule from
      <elohim-mention-base epr="epr:qahal-gathering-3b2c"></elohim-mention-base>.
    </p>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Multiple mention chips inline in prose. Demonstrates the inline-flex rendering at body line-height — chips wrap naturally and align with surrounding text.',
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
      <div style="background: #1a1a1a; padding: 1.5rem; color: #e8e8e8; color-scheme: dark;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-mention-base
      epr="epr:lamad-spa"
      label="Lamad Learning Platform"
    ></elohim-mention-base>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Dark background canary. The chip border uses `color-mix(in oklch, currentColor …)` and the id uses `opacity: 0.6` — both adapt to the inherited light text color.',
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
      <div dir="rtl" lang="he" style="padding: 1rem;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-mention-base
      epr="epr:lamad-spa"
      label="פלטפורמת למד"
    ></elohim-mention-base>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary: chip in a right-to-left container with a Hebrew label. Logical CSS properties ensure correct dot + label + id ordering.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Blank-slate proof
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial;">${story()}</div>`],
  render: () => html`
    <elohim-mention-base
      epr="epr:lamad-spa"
      label="Lamad Learning Platform"
    ></elohim-mention-base>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Wrapped in `style="all: initial;"`. Proves the chip renders with zero external CSS tokens. Border is `1px solid color-mix(…)` with a sensible transparent-currentColor fallback.',
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
          --elohim-mention-bg: #1a0030;
          --elohim-mention-fg: #f0c8ff;
          --elohim-mention-border: #9040c0;
          --elohim-mention-fallback-dot: #c060ff;
          --elohim-mention-radius: 4px;
          background: #0e0020;
          padding: 1.5rem;
          color: #f0c8ff;
          font-family: Courier, monospace;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-mention-base
      epr="epr:lamad-spa"
      label="Lamad Learning Platform"
    ></elohim-mention-base>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Override-surface proof: a deliberately non-Elohim purple theme applied via the five `--elohim-mention-*` CSS custom properties. The chip border-radius is also square-ified from 999px to 4px, proving full visual reconfigurability.',
      },
    },
  },
};
