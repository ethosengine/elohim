/**
 * Library A default story — elohim-resiliency-badge
 *
 * Blank-slate proof: no brand tokens bound. The headline is supplied via args (it is the
 * resilience fold's own sentence in the product); no live network, and the element
 * itself does no lookup and owns no wording beyond the unknown-state sentence.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

const meta: Meta = {
  title: 'Default/Core/elohim-resiliency-badge',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-resiliency-badge>\` — a link-badge that shows the holders headline it is given.

**Attributes:** \`headline\`, \`href\`, \`state\` (held | thin | unknown | known).
**Part:** \`badge\`. **Reflects:** \`data-state\`.
**Custom properties:** \`--elohim-resiliency-badge-{fg,bg,border,radius,padding,held-fg,thin-fg,hover-bg}\`.
        `.trim(),
      },
    },
  },
  args: { href: '/epr/example', headline: 'Held by 3 of 3 households' },
  render: args =>
    html`<elohim-resiliency-badge
      href=${args['href']}
      headline=${args['headline'] ?? ''}
      state=${args['state'] ?? ''}
    ></elohim-resiliency-badge>`,
};

export default meta;
type Story = StoryObj;

export const Known: Story = { name: 'Known (held)', args: { state: 'held' } };

export const Thin: Story = {
  name: 'Thin',
  args: { headline: 'Held by 1 of 3 households', state: 'thin' },
};

export const NotYetHeld: Story = {
  name: 'NotYetHeld',
  args: { headline: 'Not yet held by any household', state: 'thin' },
};

export const Unknown: Story = {
  name: 'Unknown',
  args: { headline: '' },
};

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  render: () => html`
    <div style="all: initial;">
      <elohim-resiliency-badge
        href="/epr/example"
        headline="Held by 1 of 3 households"
      ></elohim-resiliency-badge>
    </div>
  `,
};

export const CustomTheme: Story = {
  name: 'CustomTheme (override-surface proof)',
  render: () => html`
    <div style="background:#111; padding:1rem; font-family: Georgia, serif;">
      <elohim-resiliency-badge
        href="/epr/example"
        headline="Held by 1 of 3 households"
        style="
          --elohim-resiliency-badge-fg: #ffd400;
          --elohim-resiliency-badge-border: 2px dashed #ff00aa;
          --elohim-resiliency-badge-radius: 0;
          --elohim-resiliency-badge-bg: #222;
        "
      ></elohim-resiliency-badge>
    </div>
  `,
};

export const Rtl: Story = {
  name: 'RTL canary',
  render: () => html`
    <div dir="rtl">
      <elohim-resiliency-badge
        href="/epr/example"
        headline="Held by 2 of 3 households"
      ></elohim-resiliency-badge>
    </div>
  `,
};
