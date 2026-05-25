/**
 * Library A default story — elohim-imagodei-login-card
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * The login-card is the credential step of the peer OAuth portal. It renders a
 * password form and/or OAuth provider buttons. Copy adapts to trustMode:
 * "Sign in with password" (doorway-host) vs "Unlock your conductor" (peer-conductor).
 * The consumer calls `setError(message)` to surface async errors. Per spec §3.4.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';
import type { OAuthProviderRef } from 'elohim-imagodei';

// ---------------------------------------------------------------------------
// Provider fixtures
// ---------------------------------------------------------------------------

const atprotoProvider: OAuthProviderRef = {
  id: 'atproto',
  displayName: 'AT Protocol (Bluesky)',
};

const activityPubProvider: OAuthProviderRef = {
  id: 'activitypub',
  displayName: 'ActivityPub (Mastodon)',
};

const oauthProviders: OAuthProviderRef[] = [atprotoProvider, activityPubProvider];

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Imagodei/elohim-imagodei-login-card',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-imagodei-login-card>\` — credentials form for the peer OAuth portal.

Renders a password form (\`allowPassword=true\`) and/or OAuth provider buttons
(\`oauthProviders=[...]\`). Copy adapts to \`trustMode\`:
- \`doorway-host\` → "Sign in with password"
- \`peer-conductor\` → "Unlock your conductor"

The \`unlock-prompt\` slot supports the Tauri local-key-unlock flow.
The consumer calls \`setError(message)\` to surface async credential failures.

**Override surface:** bound via \`--elohim-login-*\` CSS custom properties with neutral defaults.
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
    <elohim-imagodei-login-card
      remembered-identifier="matthew@alpha.elohim.host"
      allow-password
    ></elohim-imagodei-login-card>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Wrapped in `style="all: initial;"`. Form renders with system defaults — transparent backgrounds, inherited typography, system border colors.',
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
        font-family: 'Trebuchet MS', sans-serif;
        color: #2d3748;
        background: #f7fafc;
        --elohim-login-bg: #f7fafc;
        --elohim-login-fg: #2d3748;
        --elohim-login-input-bg: #fff;
        --elohim-login-input-border: 2px solid #4299e1;
        --elohim-login-button-bg: #3182ce;
        --elohim-login-button-fg: #fff;
        --elohim-login-oauth-border: 2px solid #e2e8f0;
        --elohim-login-gap: 1rem;
        --elohim-login-radius: 8px;
        --elohim-login-error-fg: #e53e3e;
        padding: 1.5rem;
        max-inline-size: 400px;
      ">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-login-card
      remembered-identifier="matthew@alpha.elohim.host"
      allow-password
      .oauthProviders=${oauthProviders}
    ></elohim-imagodei-login-card>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Override-surface proof: blue-grey SaaS palette — deliberately non-Elohim. Demonstrates `--elohim-login-*` properties are the honest override surface.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// State stories per spec §5.2
// ---------------------------------------------------------------------------

export const PasswordOnly: Story = {
  name: 'PasswordOnly',
  render: () => html`
    <div style="max-inline-size: 400px; padding: 1rem;">
      <elohim-imagodei-login-card
        remembered-identifier="matthew@alpha.elohim.host"
        allow-password
        .oauthProviders=${[]}
      ></elohim-imagodei-login-card>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Password-only mode: `allow-password=true`, `oauthProviders=[]`. Only the password form is rendered. Copy: "Sign in with password" (doorway-host default).',
      },
    },
  },
};

export const OAuthOnly: Story = {
  name: 'OAuthOnly',
  render: () => html`
    <div style="max-inline-size: 400px; padding: 1rem;">
      <elohim-imagodei-login-card
        remembered-identifier="matthew@alpha.elohim.host"
        .oauthProviders=${oauthProviders}
      ></elohim-imagodei-login-card>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'OAuth-only mode: `allow-password` unset (defaults false), two providers. No divider or password form rendered.',
      },
    },
  },
};

export const Both: Story = {
  name: 'Both',
  render: () => html`
    <div style="max-inline-size: 400px; padding: 1rem;">
      <elohim-imagodei-login-card
        remembered-identifier="matthew@alpha.elohim.host"
        allow-password
        .oauthProviders=${oauthProviders}
      ></elohim-imagodei-login-card>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Combined mode: OAuth buttons above, "or" divider, then password form below. Full credential surface.',
      },
    },
  },
};

export const PeerConductorCopy: Story = {
  name: 'PeerConductorCopy',
  render: () => html`
    <div style="max-inline-size: 400px; padding: 1rem;">
      <elohim-imagodei-login-card
        remembered-identifier="matthew@alpha.elohim.host"
        allow-password
        trust-mode="peer-conductor"
      ></elohim-imagodei-login-card>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Peer-conductor copy: `trustMode="peer-conductor"` changes the label to "Unlock your conductor" and the submit to "Unlock". OPERATIONAL framing — not philosophical.',
      },
    },
  },
};

export const WithUnlockPromptSlot: Story = {
  name: 'WithUnlockPromptSlot',
  render: () => html`
    <div style="max-inline-size: 400px; padding: 1rem;">
      <elohim-imagodei-login-card
        remembered-identifier="matthew@alpha.elohim.host"
        allow-password
        trust-mode="peer-conductor"
      >
        <div slot="unlock-prompt" style="padding: 0.5rem; border: 1px dashed currentColor; border-radius: 4px; font-size: 0.875rem;">
          <strong>Biometric unlock available.</strong>
          Use your fingerprint or face to unlock your conductor without a password.
          <br>
          <button type="button">Use biometrics</button>
        </div>
      </elohim-imagodei-login-card>
    </div>
  `,
  parameters: {
    docs: {
      description: {
        story: 'With `unlock-prompt` slot: Tauri local-key-unlock UI rendered above the password form. The slot content is consumer-provided; the card slots it at the top of its grid.',
      },
    },
  },
};

export const WithError: Story = {
  name: 'WithError',
  render: () => {
    const el = html`
      <div style="max-inline-size: 400px; padding: 1rem;">
        <elohim-imagodei-login-card
          id="login-card-error"
          remembered-identifier="matthew@alpha.elohim.host"
          allow-password
        ></elohim-imagodei-login-card>
      </div>
    `;
    // Inject error via the public setError() API after a tick.
    setTimeout(() => {
      const card = document.getElementById('login-card-error');
      if (card && 'setError' in card) {
        (card as unknown as { setError: (msg: string) => void }).setError('credentials did not match — check your password and try again');
      }
    }, 0);
    return el;
  },
  parameters: {
    docs: {
      description: {
        story: 'Error state: `setError("credentials did not match…")` called via the public API (simulating an async credential failure). The `error` part renders with `role="alert"`.',
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
      <div style="background: #111; padding: 1.5rem; color-scheme: dark; color: #f0f0f0; max-inline-size: 400px;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-login-card
      remembered-identifier="matthew@alpha.elohim.host"
      allow-password
      .oauthProviders=${oauthProviders}
    ></elohim-imagodei-login-card>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Dark theme canary: transparent/inherit defaults adapt to the dark container.',
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
      <div dir="rtl" lang="he" style="padding: 1rem; max-inline-size: 400px;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-login-card
      remembered-identifier="matthew@alpha.elohim.host"
      allow-password
    ></elohim-imagodei-login-card>
  `,
  parameters: {
    docs: {
      description: {
        story: 'RTL canary: he locale in RTL container. Logical CSS properties ensure correct layout mirroring.',
      },
    },
  },
};
