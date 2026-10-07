/**
 * Library A default story — elohim-imagodei-device-consent-card
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * The card is the portal screen where a person approves a device that asked
 * (from a terminal) to act for them. Human story:
 * genesis/a2o/features/auth/device-consent-grant.feature.
 *
 * No ts-rs view exists yet for the device-authorization request; the fixture
 * type is the element's own `DeviceConsentRequest` (the shape the doorway's
 * request projection hands the host). Fingerprints are short forms, exactly as
 * the terminal prints them.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';
import type { DeviceConsentRequest } from 'elohim-imagodei';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const enrollOnly: DeviceConsentRequest = {
  clientId: 'elohim-cli',
  label: 'workspace',
  deviceFingerprint: 'uhCAk…8f3a',
  askedActs: ['device.enroll'],
};

const enrollAndRoot: DeviceConsentRequest = {
  ...enrollOnly,
  deviceRootFingerprint: 'uhCAk…c41e',
  askedActs: ['device.enroll', 'device.bind-root'],
};

const FIVE_MINUTES = 5 * 60_000;
const CODE = 'KQ7M-4XTD';

const frame = (story: () => unknown) => html`
  <div style="max-inline-size: 440px; padding: 1rem;">${story()}</div>
`;

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Imagodei/elohim-imagodei-device-consent-card',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-imagodei-device-consent-card>\` — a person approves a device that is asking to act for them.

Review names the device (\`request.label\`), shows its short key fingerprint to compare with
the terminal, and lists each asked act (\`enroll this device\`, \`bind this device's root key\`)
as its own agree/leave-out row. A root key can only be bound alongside enrollment: leaving
enrollment out leaves the root key out too. Agreeing to nothing disables Approve.

Phases (host-driven): \`review\` → \`signing\` → \`code\` (paste) | \`handed-back\` (same machine);
or \`declined\` / \`refused\`.

Events: \`approve\` \`{ agreedActs, declinedActs }\`, \`decline\` \`{ reason }\`,
\`code-copied\` \`{ method }\`, \`expired\` \`{ expiresAt }\`.

**Override surface:** \`--elohim-device-consent-*\` CSS custom properties with neutral defaults.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Blank-slate + override-surface proofs
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [
    story => html`
      <div style="all: initial; display: block; padding: 1rem;">${story()}</div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollAndRoot}
      person-label="Matthew"
    ></elohim-imagodei-device-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Wrapped in `style="all: initial;"`. Heading, key block, act rows, notes and actions render with system defaults and zero tokens.',
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
          font-family: 'Gill Sans', 'Gill Sans MT', Calibri, sans-serif;
          color: #10243a;
          background: #e6eef5;
          --elohim-device-consent-gap: 1.25rem;
          --elohim-device-consent-radius: 0;
          --elohim-device-consent-muted-opacity: 0.85;
          --elohim-device-consent-key-bg: #cfdfee;
          --elohim-device-consent-act-row-bg: #ffffff;
          --elohim-device-consent-act-row-border: 2px solid #2a6f97;
          --elohim-device-consent-toggle-accent: #2a6f97;
          --elohim-device-consent-note-border: 4px double #2a6f97;
          --elohim-device-consent-approve-bg: #2a6f97;
          --elohim-device-consent-approve-fg: #ffffff;
          --elohim-device-consent-decline-bg: transparent;
          --elohim-device-consent-decline-fg: #10243a;
          --elohim-device-consent-focus-ring: 3px dashed #c2410c;
          padding: 1.5rem;
          max-inline-size: 440px;
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollAndRoot}
      person-label="Matthew"
      host-label="alpha"
    ></elohim-imagodei-device-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Override-surface proof: a square-cornered steel-blue palette with a dashed orange focus ring — deliberately non-Elohim. Every visual value comes through `--elohim-device-consent-*`.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Review states
// ---------------------------------------------------------------------------

export const ReviewEnrollOnly: Story = {
  name: 'ReviewEnrollOnly',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollOnly}
      person-label="Matthew"
      host-label="alpha"
    ></elohim-imagodei-device-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story: 'The terminal asked only to enroll the device. One act row, agreed by default.',
      },
    },
  },
};

export const ReviewEnrollAndRootKey: Story = {
  name: 'ReviewEnrollAndRootKey',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollAndRoot}
      person-label="Matthew"
      host-label="alpha"
    ></elohim-imagodei-device-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Enrollment and root key asked together: two separate rows, each agreed by default; the root key row shows its own short fingerprint.',
      },
    },
  },
};

export const ReviewPartial: Story = {
  name: 'ReviewPartial (root key left out)',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollAndRoot}
      person-label="Matthew"
      host-label="alpha"
    ></elohim-imagodei-device-consent-card>
  `,
  play: async ({ canvasElement }) => {
    const card = canvasElement.querySelector('elohim-imagodei-device-consent-card');
    await card?.updateComplete;
    const root = card?.shadowRoot?.querySelector<HTMLInputElement>(
      '[data-act="device.bind-root"] input'
    );
    if (root?.checked) root.click();
  },
  parameters: {
    docs: {
      description: {
        story:
          'The person agrees to enrollment and leaves the root key out. Approve stays enabled; the approve event will list `device.bind-root` under `declinedActs`.',
      },
    },
  },
};

export const ReviewNothingAgreed: Story = {
  name: 'ReviewNothingAgreed (coherence rule)',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollAndRoot}
      person-label="Matthew"
    ></elohim-imagodei-device-consent-card>
  `,
  play: async ({ canvasElement }) => {
    const card = canvasElement.querySelector('elohim-imagodei-device-consent-card');
    await card?.updateComplete;
    const enroll = card?.shadowRoot?.querySelector<HTMLInputElement>(
      '[data-act="device.enroll"] input'
    );
    if (enroll?.checked) enroll.click();
  },
  parameters: {
    docs: {
      description: {
        story:
          'Leaving enrollment out also leaves the root key out (its control disabled, with the reason). Nothing is agreed, so Approve is disabled.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Flow states
// ---------------------------------------------------------------------------

export const Signing: Story = {
  name: 'Signing',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollAndRoot}
      phase="signing"
      person-label="Matthew"
      host-label="alpha"
    ></elohim-imagodei-device-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Busy: controls disabled, `aria-busy` set, status line announced.',
      },
    },
  },
};

export const CodeToPaste: Story = {
  name: 'CodeToPaste',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollOnly}
      phase="code"
      code=${CODE}
      .expiresAt=${Date.now() + FIVE_MINUTES}
    ></elohim-imagodei-device-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Remote device: the one-time code, large and monospaced, a copy button, and a live countdown (announced to assistive tech once a minute).',
      },
    },
  },
};

export const CodeExpired: Story = {
  name: 'CodeExpired',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollOnly}
      phase="code"
      code=${CODE}
      .expiresAt=${Date.now() - 1000}
    ></elohim-imagodei-device-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'The countdown has passed: the code is withdrawn and the card says how to start again.',
      },
    },
  },
};

export const HandedBack: Story = {
  name: 'HandedBack',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollOnly}
      phase="handed-back"
    ></elohim-imagodei-device-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Same machine: the browser handed the code to the terminal. No code is shown.',
      },
    },
  },
};

export const HeldForDevice: Story = {
  name: 'HeldForDevice',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollOnly}
      phase="handed-back"
      handed-back-to="held"
    ></elohim-imagodei-device-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'The node holds the code and the asking device collects it itself: done, it finishes ' +
          'joining on its own, nothing to copy. Its key is shown once more so the person knows which.',
      },
    },
  },
};

export const Declined: Story = {
  name: 'Declined',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollOnly}
      phase="declined"
    ></elohim-imagodei-device-consent-card>
  `,
};

export const RefusedIncoherent: Story = {
  name: 'RefusedIncoherent',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${{ ...enrollAndRoot, askedActs: ['device.bind-root'] }}
      phase="refused"
      refusal-code="request_acts_incoherent"
    ></elohim-imagodei-device-consent-card>
  `,
};

export const RefusedUnavailableWayThrough: Story = {
  name: 'RefusedUnavailableWayThrough',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollOnly}
      phase="refused"
      refusal-code="consent_unavailable"
      command="epr device approve 'https://doorway.example/threshold/consent/device?request=eyJjbGllbnRJZCI6ImVwci1jbGkifQ'"
    ></elohim-imagodei-device-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'This host takes no device approvals, said before any sign-in, with the way through: ' +
          'the same approval as a command to run on a device that is already the person’s, ' +
          'with a copy button (`command`).',
      },
    },
  },
};

export const RefusedUnknownAct: Story = {
  name: 'RefusedUnknownAct',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollOnly}
      phase="refused"
      refusal-code="act_unknown"
    ></elohim-imagodei-device-consent-card>
  `,
};

// ---------------------------------------------------------------------------
// Signer variant
// ---------------------------------------------------------------------------

export const PeerConductorSigner: Story = {
  name: 'PeerConductorSigner',
  decorators: [frame],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollAndRoot}
      signer="peer-conductor"
      person-label="Jessica"
      host-label="jessica-desk"
    ></elohim-imagodei-device-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story: 'A person on their own node: this node holds their key and signs as them.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Theme + direction canaries
// ---------------------------------------------------------------------------

export const Dark: Story = {
  name: 'Dark',
  decorators: [
    story => html`
      <div
        style="background: Canvas; color: CanvasText; color-scheme: dark; padding: 1.5rem; max-inline-size: 440px;"
      >
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollAndRoot}
      person-label="Matthew"
      host-label="alpha"
    ></elohim-imagodei-device-consent-card>
  `,
};

export const RTLCanary: Story = {
  name: 'RTLCanary',
  decorators: [
    story => html`
      <div dir="rtl" lang="he" style="padding: 1rem; max-inline-size: 440px;">${story()}</div>
    `,
  ],
  render: () => html`
    <elohim-imagodei-device-consent-card
      .request=${enrollAndRoot}
      person-label="Matthew"
      host-label="alpha"
    ></elohim-imagodei-device-consent-card>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'RTL container: layout mirrors through logical properties; fingerprints and the code stay left-to-right.',
      },
    },
  },
};
