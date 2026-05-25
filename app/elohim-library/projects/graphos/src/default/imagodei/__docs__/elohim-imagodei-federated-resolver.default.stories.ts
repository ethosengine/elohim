/**
 * Library A default story — elohim-imagodei-federated-resolver
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * The federated-resolver is the identifier input step of the peer OAuth portal.
 * The viewer enters their federated identifier (e.g. `matthew@alpha.elohim.host`);
 * the injected `resolveIdentifier` callback discovers the doorway endpoint.
 * Persistence to localStorage pre-fills on next mount.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';
import type { FederatedResolveOutcome, ResolveIdentifierFn } from 'elohim-imagodei';

// ---------------------------------------------------------------------------
// Resolver fixtures
// ---------------------------------------------------------------------------

/** Always resolves successfully. */
const successResolver: ResolveIdentifierFn = async (identifier: string): Promise<FederatedResolveOutcome> => {
  await new Promise(r => setTimeout(r, 600));
  return { ok: true, doorwayUrl: `https://alpha.elohim.host/doorway?for=${encodeURIComponent(identifier)}` };
};

/** Always fails with 'unknown-host'. */
const errorResolver: ResolveIdentifierFn = async (): Promise<FederatedResolveOutcome> => {
  await new Promise(r => setTimeout(r, 400));
  return { ok: false, reason: 'unknown-host — no doorway found for this identifier' };
};

/** Never resolves (simulates hung request). */
const hangingResolver: ResolveIdentifierFn = (): Promise<FederatedResolveOutcome> =>
  new Promise(() => {/* intentionally never resolves */});

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Imagodei/elohim-imagodei-federated-resolver',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-imagodei-federated-resolver>\` — input element for federated identifiers
(e.g. \`matthew@alpha.elohim.host\`).

Calls the injected \`resolveIdentifier\` callback to discover the doorway endpoint.
Persists the last-used identifier under \`rememberKey\` (default \`elohim_auth_identifier\`)
and pre-fills on mount.

Emits \`resolved\` on success or \`resolve-error\` on failure.

**Override surface:** bound via \`--elohim-resolver-*\` and \`--elohim-input-*\` CSS custom
properties with neutral defaults.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Blank-slate proof
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial; display: block; padding: 1rem;">${story()}</div>`],
  render: () => html`
    <elohim-imagodei-federated-resolver
      .resolveIdentifier=${successResolver}
    ></elohim-imagodei-federated-resolver>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Wrapped in `style="all: initial;"`. Form renders with system defaults; the input and submit button are usable with zero tokens.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Override-surface proof
// ---------------------------------------------------------------------------

export const CustomTheme: Story = {
  name: 'CustomTheme (override-surface proof)',
  decorators: [
    story => html`
      <div style="
        font-family: 'Palatino Linotype', Palatino, serif;
        color: #1a1a2e;
        background: #e8e0d5;
        --elohim-resolver-bg: #e8e0d5;
        --elohim-resolver-fg: #1a1a2e;
        --elohim-input-border: 2px solid #8b7355;
        --elohim-input-focus-ring: #c0392b;
        --elohim-input-error-fg: #c0392b;
        --elohim-resolver-gap: 0.75rem;
        --elohim-resolver-label-size: 1rem;
        padding: 1.5rem;
        max-inline-size: 480px;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-federated-resolver
      .resolveIdentifier=${successResolver}
      placeholder="you@your-community.host"
    ></elohim-imagodei-federated-resolver>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Override-surface proof: warm parchment palette with serif typography — deliberately non-Elohim. Demonstrates `--elohim-resolver-*` and `--elohim-input-*` are the honest override surface.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// State stories per spec §5.2
// ---------------------------------------------------------------------------

export const Default: Story = {
  name: 'Default',
  render: () => html`
    <div style="max-inline-size: 480px; padding: 1rem;">
      <elohim-imagodei-federated-resolver
        .resolveIdentifier=${successResolver}
      ></elohim-imagodei-federated-resolver>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Default idle state: input empty, no pre-filled identifier. Submit is disabled until the user types. The successResolver resolves after 600ms on submit.',
      },
    },
  },
};

export const WithError: Story = {
  name: 'WithError',
  render: () => html`
    <div style="max-inline-size: 480px; padding: 1rem;">
      <elohim-imagodei-federated-resolver
        .resolveIdentifier=${errorResolver}
      ></elohim-imagodei-federated-resolver>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Error state: enter any identifier and submit. The `errorResolver` returns `{ ok: false, reason: "unknown-host..." }` which renders in the `error` part with `role="alert"`.',
      },
    },
  },
};

export const RememberedIdentifier: Story = {
  name: 'RememberedIdentifier',
  decorators: [
    story => {
      // Set the remembered identifier before the element mounts.
      try {
        localStorage.setItem('elohim_auth_identifier', 'matthew@alpha.elohim.host');
      } catch {
        // localStorage unavailable in some test contexts; degrade silently.
      }
      return html`<div style="max-inline-size: 480px; padding: 1rem;">${story()}</div>`;
    },
  ],
  render: () => html`
    <elohim-imagodei-federated-resolver
      .resolveIdentifier=${successResolver}
    ></elohim-imagodei-federated-resolver>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Pre-filled via localStorage: the decorator sets `elohim_auth_identifier` to `matthew@alpha.elohim.host` before mount, simulating a returning user whose last identifier is remembered.',
      },
    },
  },
};

export const WithHelpSlot: Story = {
  name: 'WithHelpSlot',
  render: () => html`
    <div style="max-inline-size: 480px; padding: 1rem;">
      <elohim-imagodei-federated-resolver
        .resolveIdentifier=${successResolver}
        placeholder="matthew@alpha.elohim.host"
      >
        <span slot="help-text">
          Your identifier is in the format <code>you@your-doorway.host</code>.
          Not sure what this is? <a href="#">Ask your community steward</a>.
        </span>
      </elohim-imagodei-federated-resolver>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'With help slot: content in the `help-text` slot renders below the input inside the `help` csspart, guiding the user on identifier format.',
      },
    },
  },
};

export const Resolving: Story = {
  name: 'Resolving (busy state)',
  render: () => html`
    <div style="max-inline-size: 480px; padding: 1rem;">
      <elohim-imagodei-federated-resolver
        .resolveIdentifier=${hangingResolver}
      ></elohim-imagodei-federated-resolver>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Busy/resolving state: type any identifier and submit. The `hangingResolver` never resolves, so the element stays in the busy state with the submit button disabled and showing "Resolving…".',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Dark canary
// ---------------------------------------------------------------------------

export const Dark: Story = {
  name: 'Dark',
  decorators: [
    story => html`
      <div style="background: #111; padding: 1.5rem; color-scheme: dark; color: #f0f0f0; max-inline-size: 480px;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-federated-resolver
      .resolveIdentifier=${successResolver}
    ></elohim-imagodei-federated-resolver>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Dark theme canary: transparent background and color-inherit defaults adapt to the dark container.',
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
      <div dir="rtl" lang="he" style="padding: 1rem; max-inline-size: 480px;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-federated-resolver
      .resolveIdentifier=${successResolver}
    ></elohim-imagodei-federated-resolver>
  `,
  parameters: {
    docs: {
      description: {
        story: 'RTL canary: he locale in RTL container. Logical CSS properties (padding-inline, padding-block) ensure correct layout mirroring.',
      },
    },
  },
};
