/**
 * Library A default story — elohim-imagodei-oauth-callback
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * The oauth-callback element handles the OAuth authorization code exchange after
 * the redirect lands. It runs the exchange exactly once when both `code` and `state`
 * are set. States: idle → exchanging → done | error. The `recovery-cta` slot lets
 * consumers provide retry or account-recovery links.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';
import type { ExchangeCodeFn, ExchangeOutcome } from 'elohim-imagodei';

// ---------------------------------------------------------------------------
// Exchange callback fixtures
// ---------------------------------------------------------------------------

/** Never resolves — simulates an in-flight exchange. */
const hangingExchange: ExchangeCodeFn = (): Promise<ExchangeOutcome> =>
  new Promise(() => {/* intentionally never resolves */});

/** Resolves successfully with a synthetic session envelope. */
const successExchange: ExchangeCodeFn = async (): Promise<ExchangeOutcome> => {
  await new Promise(r => setTimeout(r, 100));
  return {
    session: {
      agentPubKey: 'uhCAk-fake-agent-pub-key-for-story',
      displayName: 'Matthew Dowell',
      doorwayUrl: 'https://alpha.elohim.host',
      grantedAt: '2026-05-25T10:00:00Z',
    },
  };
};

/** Always rejects with a credential error. */
const failureExchange: ExchangeCodeFn = (_code: string, _state: string): Promise<ExchangeOutcome> => {
  return new Promise((_resolve, reject) => {
    setTimeout(() => {
      reject(new globalThis.Error('code expired or already used — please start the sign-in flow again'));
    }, 100);
  });
};

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Imagodei/elohim-imagodei-oauth-callback',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-imagodei-oauth-callback>\` — OAuth code-exchange callback handler.

State machine:
- **idle** — code or state absent; waiting for OAuth redirect params.
- **exchanging** — both params present; exchange in flight; skeleton shown.
- **done** — exchange resolved; \`success\` event emitted.
- **error** — exchange rejected; \`error\` event emitted; error + recovery slots visible.

Exchange runs **exactly once** — re-assigning the same values does not re-trigger.
Auto-retry is not performed; the \`recovery-cta\` slot lets consumers provide their own links.

**Override surface:** bound via \`--elohim-callback-*\` CSS custom properties.
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
    <elohim-imagodei-oauth-callback
      provider-label="AT Protocol"
    ></elohim-imagodei-oauth-callback>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Wrapped in `style="all: initial;"`. Element in idle state (no code/state) — renders the waiting message with zero tokens.',
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
        font-family: 'Impact', 'Haettenschweiler', sans-serif;
        color: #ff6b35;
        background: #1a0a00;
        --elohim-callback-padding: 2rem;
        --elohim-callback-skeleton-bg: rgba(255, 107, 53, 0.2);
        --elohim-callback-skeleton-radius: 0;
        --elohim-callback-error-color-mix: 100%;
        --elohim-callback-gap: 0.75rem;
        padding: 2rem;
        max-inline-size: 480px;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-oauth-callback
      code="story-code-xyz"
      state="story-state-abc"
      provider-label="Demo Provider"
      .exchangeCode=${hangingExchange}
    ></elohim-imagodei-oauth-callback>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Override-surface proof: deep-orange-on-black palette — deliberately non-Elohim. Element in `exchanging` state via the hanging exchange. Demonstrates `--elohim-callback-*` are the honest override surface.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// State stories per spec §5.2
// ---------------------------------------------------------------------------

export const Exchanging: Story = {
  name: 'Exchanging',
  render: () => html`
    <div style="max-inline-size: 480px; padding: 1rem;">
      <elohim-imagodei-oauth-callback
        code="oauth-code-alpha-1234"
        state="csrf-state-token-5678"
        provider-label="AT Protocol (Bluesky)"
        .exchangeCode=${hangingExchange}
      ></elohim-imagodei-oauth-callback>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Exchanging state: both `code` and `state` are present; the hanging exchange never resolves so the skeleton and "Finishing sign-in with AT Protocol (Bluesky)…" message remain visible. `aria-busy="true"` on the skeleton container.',
      },
    },
  },
};

export const Success: Story = {
  name: 'Success',
  render: () => html`
    <div style="max-inline-size: 480px; padding: 1rem;">
      <elohim-imagodei-oauth-callback
        code="oauth-code-alpha-1234"
        state="csrf-state-token-5678"
        provider-label="AT Protocol (Bluesky)"
        .exchangeCode=${successExchange}
      ></elohim-imagodei-oauth-callback>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Success state: exchange resolves → "Signed in. Redirecting…" shown in the `done` part. The `success` event fires with `{ session: { agentPubKey, displayName, … } }`.',
      },
    },
  },
};

export const Error: Story = {
  name: 'Error',
  render: () => html`
    <div style="max-inline-size: 480px; padding: 1rem;">
      <elohim-imagodei-oauth-callback
        code="oauth-code-alpha-1234"
        state="csrf-state-token-5678"
        provider-label="AT Protocol (Bluesky)"
        .exchangeCode=${failureExchange}
      ></elohim-imagodei-oauth-callback>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Error state: exchange rejects → `error` part (role="alert") shows the reason "code expired or already used…". The `error-detail` and `recovery-cta` slots are visible but empty here.',
      },
    },
  },
};

export const WithRecoveryCta: Story = {
  name: 'WithRecoveryCta',
  render: () => html`
    <div style="max-inline-size: 480px; padding: 1rem;">
      <elohim-imagodei-oauth-callback
        code="oauth-code-alpha-1234"
        state="csrf-state-token-5678"
        provider-label="AT Protocol (Bluesky)"
        .exchangeCode=${failureExchange}
      >
        <div slot="error-detail" style="font-size: 0.875rem; margin-block-start: 0.5rem; opacity: 0.8;">
          This can happen if you clicked Back or if the sign-in link expired.
        </div>
        <nav slot="recovery-cta" style="display: flex; gap: 0.75rem; margin-block-start: 1rem; flex-wrap: wrap;">
          <a href="#" onclick="return false;">Try signing in again</a>
          <a href="#" onclick="return false;">Use a different identity</a>
          <a href="#" onclick="return false;">Get help from your recovery witnesses</a>
        </nav>
      </elohim-imagodei-oauth-callback>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Error state with recovery CTA: `error-detail` slot provides contextual explanation; `recovery-cta` slot provides three recovery links (retry, different identity, recovery witness help). The grandma-standard recovery path: "get help from your people."',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Idle state (no code/state)
// ---------------------------------------------------------------------------

export const IdleState: Story = {
  name: 'IdleState',
  render: () => html`
    <div style="max-inline-size: 480px; padding: 1rem;">
      <elohim-imagodei-oauth-callback
        provider-label="ActivityPub (Mastodon)"
      ></elohim-imagodei-oauth-callback>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Idle state: no `code` or `state` props set. Renders "Waiting for OAuth response from ActivityPub (Mastodon)…" in the `idle` part. Exchange never starts.',
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
    <elohim-imagodei-oauth-callback
      code="oauth-code-alpha-1234"
      state="csrf-state-token-5678"
      provider-label="AT Protocol (Bluesky)"
      .exchangeCode=${hangingExchange}
    ></elohim-imagodei-oauth-callback>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Dark theme canary: skeleton bone uses color-mix() defaults that adapt to dark context.',
      },
    },
  },
};
