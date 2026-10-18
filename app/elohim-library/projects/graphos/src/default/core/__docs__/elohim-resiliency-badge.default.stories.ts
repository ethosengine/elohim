/**
 * Library A default story — elohim-resiliency-badge
 *
 * Blank-slate proof: no brand tokens bound. Figures are supplied via args;
 * no live network. The fetch path is exercised by the unit spec with a stubbed fetch.
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
\`<elohim-resiliency-badge>\` — a link-badge showing how many households hold a piece of content.

**Attributes:** \`content-id\`, \`href\`, \`holders\`, \`target\`, \`state\` (held | thin | unknown), \`base-url\`.
**Part:** \`badge\`. **Reflects:** \`data-state\`.
**Custom properties:** \`--elohim-resiliency-badge-{fg,bg,border,radius,padding,held-fg,thin-fg,hover-bg}\`.
        `.trim(),
      },
    },
  },
  args: { href: '/epr/example', holders: 3, target: 3 },
  render: args =>
    html`<elohim-resiliency-badge
      href=${args['href']}
      .holders=${args['holders']}
      .target=${args['target']}
    ></elohim-resiliency-badge>`,
};

export default meta;
type Story = StoryObj;

export const Known: Story = { name: 'Known (held)' };

export const Thin: Story = { name: 'Thin', args: { holders: 1, target: 3 } };

export const NotYetHeld: Story = { name: 'NotYetHeld', args: { holders: 0, target: 3 } };

export const Unknown: Story = {
  name: 'Unknown',
  args: { holders: undefined, target: undefined },
};

export const WithFetchMocked: Story = {
  name: 'WithFetchMocked (figures as the fetch would derive them)',
  args: { holders: 2, target: 3 },
  parameters: {
    docs: {
      description: {
        story:
          'With `content-id` set and no figures, the element fetches `/api/v1/resilience/{id}/household` and reads `feltStatus.floor`. Shown here with the derived figures supplied; no network is used.',
      },
    },
  },
};

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  render: () => html`
    <div style="all: initial;">
      <elohim-resiliency-badge href="/epr/example" .holders=${1} .target=${3}></elohim-resiliency-badge>
    </div>
  `,
};

export const CustomTheme: Story = {
  name: 'CustomTheme (override-surface proof)',
  render: () => html`
    <div style="background:#111; padding:1rem; font-family: Georgia, serif;">
      <elohim-resiliency-badge
        href="/epr/example"
        .holders=${1}
        .target=${3}
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
      <elohim-resiliency-badge href="/epr/example" .holders=${2} .target=${3}></elohim-resiliency-badge>
    </div>
  `,
};
