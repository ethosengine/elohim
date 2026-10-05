/**
 * Library B designed story — DeviceConsent
 *
 * Scene: Matthew is at a machine he calls "workspace". He ran a command in its
 * terminal; the terminal printed a link to his doorway's sign-in portal and is
 * waiting. He opened the link, signed in, and now his doorway is showing him
 * which device is asking and what it wants. Every story below is one moment of
 * that household errand — approving, agreeing to less, pasting a code, saying
 * no — not a tour of the component's props.
 *
 * Human story: genesis/a2o/features/auth/device-consent-grant.feature
 * (branch feat/device-authorization-grant).
 *
 * Composition: <elohim-imagodei-portal-shell> (consent step) with a lone
 * trust-indicator in the header slot, and <elohim-imagodei-device-consent-card>
 * in the primary slot.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — no ts-rs view exists yet for the device-authorization request;
 *      the fixture type is the element's own `DeviceConsentRequest` (the shape
 *      the doorway's request projection hands the host), as Library A uses.
 *   2. Manifest vocabulary — the two acts `device.enroll` / `device.bind-root`
 *      and the refusal codes are the doorway's own vocabulary.
 *   3. Brand tokens — inline EL_TOKENS bag (graphos design spec §14), bound at
 *      the story-decorator level only.
 *
 * Every `--elohim-device-consent-*` name bound below is declared as an
 * `@cssprop` on the element; every `--elohim-portal-*` / `--elohim-trust-*`
 * name is declared on the shell / trust-indicator. Typography that custom
 * properties cannot reach (the heading face, the reading face of the two
 * sentences a person must actually read) goes through declared `@csspart`s.
 *
 * Library B boundary: NEVER modifies any primitive's CSS, JSDoc, or tag name.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';

import type {
  AuthorityResolution,
  DeviceConsentRequest,
  DeviceConsentStrings,
} from 'elohim-imagodei';

// ---------------------------------------------------------------------------
// Brand token declaration — graphos design spec §14
// ---------------------------------------------------------------------------

const EL_TOKENS = `
  --el-green-deep:  #2D5F3B;
  --el-green-light: #7FB069;
  --el-amber:       #D4A03E;
  --el-clay:        #B8664F;
  --el-cream:       #F5F0E8;
  --el-linen:       #FAF6EE;
  --el-stone:       #6B6157;
  --el-sky:         #7BAFCB;
  --el-starlight:   #E8E4D9;
  --el-night:       #0F1A12;
  --el-font-display: 'Fraunces', Georgia, serif;
  --el-font-body:    'Source Serif 4', Georgia, serif;
  --el-font-ui:      'DM Sans', system-ui, sans-serif;
  --el-font-mono:    'JetBrains Mono', ui-monospace, monospace;
  --el-space-xs:  8px;
  --el-space-sm:  16px;
  --el-space-md:  24px;
  --el-space-lg:  32px;
  --el-space-xl:  48px;
  --el-radius-sm: 4px;
  --el-radius-md: 8px;
  --el-radius-lg: 16px;
  --el-shadow-soft: 0 2px 8px rgba(107, 97, 87, 0.08);
`;

// Portal shell — the same frame in both themes; only colors change. Spacing is
// fluid so one binding serves the desktop window and a 360px phone: 48px side
// margins on desktop down to the spec's 24px mobile floor.
const PORTAL_FRAME = `
  --elohim-portal-max-inline-size: 544px;
  --elohim-portal-padding:         clamp(24px, 7vh, 64px) clamp(24px, 5vw, 48px);
  --elohim-portal-panel-padding:   clamp(16px, 4.5vw, 40px);
  --elohim-portal-panel-radius:    var(--el-radius-lg);
  --elohim-portal-grid-gap:        var(--el-space-md);
  --elohim-portal-footer-size:     0.875rem;
`;

const PORTAL_LIGHT = `
  ${PORTAL_FRAME}
  --elohim-portal-bg:       var(--el-cream);
  --elohim-portal-fg:       var(--el-night);
  --elohim-portal-panel-bg: var(--el-linen);
`;

const PORTAL_DARK = `
  ${PORTAL_FRAME}
  --elohim-portal-bg:       var(--el-night);
  --elohim-portal-fg:       var(--el-starlight);
  --elohim-portal-panel-bg: #162019;
`;

const TRUST_LIGHT = `
  --elohim-trust-bg:             transparent;
  --elohim-trust-fg:             var(--el-stone);
  --elohim-trust-host-accent:    var(--el-amber);
  --elohim-trust-peer-accent:    var(--el-sky);
  --elohim-trust-radius:         var(--el-radius-sm);
  --elohim-trust-padding-block:  0.25rem;
  --elohim-trust-padding-inline: 0.625rem;
`;

const TRUST_DARK = `
  --elohim-trust-bg:             rgba(232, 228, 217, 0.06);
  --elohim-trust-fg:             var(--el-starlight);
  --elohim-trust-host-accent:    var(--el-amber);
  --elohim-trust-peer-accent:    var(--el-sky);
  --elohim-trust-radius:         var(--el-radius-sm);
  --elohim-trust-padding-block:  0.25rem;
  --elohim-trust-padding-inline: 0.625rem;
`;

// Device-consent card. Vineyard is the one filled control (Approve); Decline
// is a quiet outline. The key block and the code sit on a Harvest Gold wash —
// the golden-hour element of the screen — and are the only monospaced things.
const CARD_FRAME = `
  --elohim-device-consent-gap:           var(--el-space-md);
  --elohim-device-consent-radius:        var(--el-radius-md);
  --elohim-device-consent-code-font:     var(--el-font-mono);
  --elohim-device-consent-code-size:     2.25rem;
`;

const CARD_LIGHT = `
  ${CARD_FRAME}
  --elohim-device-consent-muted-opacity: 0.74;
  --elohim-device-consent-key-bg:        #F3E9D4;
  --elohim-device-consent-code-bg:       #F3E9D4;
  --elohim-device-consent-act-row-bg:    var(--el-cream);
  --elohim-device-consent-act-row-border: 1px solid rgba(107, 97, 87, 0.2);
  --elohim-device-consent-toggle-accent: var(--el-green-deep);
  --elohim-device-consent-note-border:   3px solid var(--el-amber);
  --elohim-device-consent-approve-bg:    var(--el-green-deep);
  --elohim-device-consent-approve-fg:    var(--el-linen);
  --elohim-device-consent-decline-bg:    transparent;
  --elohim-device-consent-decline-fg:    var(--el-stone);
  --elohim-device-consent-focus-ring:    2px solid var(--el-green-deep);
`;

const CARD_DARK = `
  ${CARD_FRAME}
  --elohim-device-consent-muted-opacity: 0.74;
  --elohim-device-consent-key-bg:        rgba(212, 160, 62, 0.11);
  --elohim-device-consent-code-bg:       rgba(212, 160, 62, 0.11);
  --elohim-device-consent-act-row-bg:    rgba(232, 228, 217, 0.04);
  --elohim-device-consent-act-row-border: 1px solid rgba(232, 228, 217, 0.16);
  --elohim-device-consent-toggle-accent: var(--el-green-light);
  --elohim-device-consent-note-border:   3px solid var(--el-amber);
  --elohim-device-consent-approve-bg:    var(--el-green-light);
  --elohim-device-consent-approve-fg:    var(--el-night);
  --elohim-device-consent-decline-bg:    transparent;
  --elohim-device-consent-decline-fg:    var(--el-starlight);
  --elohim-device-consent-focus-ring:    2px solid var(--el-amber);
`;

/**
 * Typography and surface the custom-property surface cannot reach, through
 * declared parts only: the heading in Fraunces, the two sentences a person
 * must read (the scope note, the closing message) in Source Serif 4, and the
 * panel lifted off the table with the spec's warm paper shadow.
 */
const SCENE_CSS = html`
  <style>
    .el-device-scene elohim-imagodei-device-consent-card::part(heading) {
      font-family: var(--el-font-display);
      font-weight: 500;
      font-size: 1.625rem;
      line-height: 1.25;
      font-variation-settings:
        'SOFT' 50,
        'WONK' 1;
    }

    .el-device-scene elohim-imagodei-device-consent-card::part(scope-note),
    .el-device-scene elohim-imagodei-device-consent-card::part(message) {
      font-family: var(--el-font-body);
      font-size: 1.0625rem;
      line-height: 1.6;
    }

    .el-device-scene--light elohim-imagodei-portal-shell::part(primary-region) {
      box-shadow: var(--el-shadow-soft);
    }

    .el-device-scene--dark elohim-imagodei-portal-shell::part(primary-region) {
      box-shadow: 0 0 0 1px rgba(232, 228, 217, 0.06);
    }
  </style>
`;

function lightDecorator(story: () => unknown) {
  return html`
    ${SCENE_CSS}
    <div
      class="el-device-scene el-device-scene--light"
      style="
        ${EL_TOKENS}
        ${PORTAL_LIGHT}
        ${TRUST_LIGHT}
        ${CARD_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        color: var(--el-night);
        min-block-size: 100vh;
      "
    >
      ${story()}
    </div>
  `;
}

function darkDecorator(story: () => unknown) {
  return html`
    ${SCENE_CSS}
    <div
      class="el-device-scene el-device-scene--dark"
      style="
        ${EL_TOKENS}
        ${PORTAL_DARK}
        ${TRUST_DARK}
        ${CARD_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        color: var(--el-starlight);
        color-scheme: dark;
        min-block-size: 100vh;
      "
    >
      ${story()}
    </div>
  `;
}

function hebrewDecorator(story: () => unknown) {
  return html`
    ${SCENE_CSS}
    <div
      dir="rtl"
      lang="he"
      class="el-device-scene el-device-scene--light"
      style="
        ${EL_TOKENS}
        ${PORTAL_LIGHT}
        ${TRUST_LIGHT}
        ${CARD_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        color: var(--el-night);
        min-block-size: 100vh;
      "
    >
      ${story()}
    </div>
  `;
}

// ---------------------------------------------------------------------------
// Fixtures — the machine Matthew is approving, exactly as its terminal printed
// ---------------------------------------------------------------------------

const workspaceEnroll: DeviceConsentRequest = {
  clientId: 'elohim-cli',
  label: 'workspace',
  deviceFingerprint: 'uhCAk…8f3a',
  askedActs: ['device.enroll'],
};

const workspaceEnrollAndRoot: DeviceConsentRequest = {
  ...workspaceEnroll,
  deviceRootFingerprint: 'uhCAk…c41e',
  askedActs: ['device.enroll', 'device.bind-root'],
};

/** Jessica runs her own node; she is approving her studio laptop from it. */
const studioLaptop: DeviceConsentRequest = {
  clientId: 'elohim-cli',
  label: 'studio-laptop',
  deviceFingerprint: 'uhCAk…2b9d',
  deviceRootFingerprint: 'uhCAk…7e05',
  askedActs: ['device.enroll', 'device.bind-root'],
};

const hostedByAlpha: AuthorityResolution = {
  trustMode: 'doorway-host',
  authority: { label: 'alpha.elohim.host', id: 'alpha' },
  flywheelHint: false,
  attestors: [],
};

const jessicasOwnNode: AuthorityResolution = {
  trustMode: 'peer-conductor',
  authority: { label: 'jessica-desk' },
  flywheelHint: false,
  attestors: [],
};

const CODE = 'KQ7M-4XTD';
/** A code shown a little while ago: about four minutes of its five remain. */
const CODE_REMAINING_MS = 4 * 60_000 + 12_000;

// ---------------------------------------------------------------------------
// Hebrew strings — RTL canary. Plain, careful translation; names (Matthew,
// workspace, alpha.elohim.host) stay as they are and the element isolates them.
// ---------------------------------------------------------------------------

const heNamed = (host: string | undefined, fallback: string): string =>
  host ? `${fallback} (${host})` : fallback;

const HE_STRINGS: Partial<DeviceConsentStrings> = {
  reviewHeading: label => `מכשיר בשם "${label}" מבקש לפעול בשמך`,
  signedInAs: person => `נכנסת בתור ${person}`,
  deviceKeyLabel: 'מפתח המכשיר',
  compareHint: 'ודאו שזה תואם למה שהטרמינל הדפיס. אם לא, סרבו.',
  asksLegend: 'מה המכשיר הזה מבקש',
  enrollName: 'לרשום את המכשיר הזה',
  enrollWhat: 'הרשת תזהה את המחשב הזה כאחד משלך.',
  bindRootName: 'לקשור את מפתח השורש של המכשיר',
  bindRootWhat: 'את מה שהמחשב הזה מפיק אפשר יהיה לייחס אליך.',
  rootKeyLabel: 'מפתח שורש',
  bindRootNeedsEnroll: 'אפשר לקשור מפתח שורש רק למכשיר רשום, ולכן גם הוא לא ייכלל.',
  scopeNote: 'זה רק מזהה את המכשיר כשלך. כשלעצמו, זה לא מאפשר למכשיר לשנות שום תוכן שלך.',
  signerHosted: host =>
    `${heNamed(host, 'השער שלך')} שומר את המפתח שלך, ויחתום על זה בשמך כשתאשרו.`,
  signerOwnNode: host =>
    `${heNamed(host, 'הצומת הזה')} מחזיק את המפתח שלך, ויחתום על זה בשמך כשתאשרו.`,
  nothingChosen: 'בחרו לפחות דבר אחד לאישור, או סרבו.',
  approve: 'אישור',
  decline: 'סירוב',
  signingHeading: label => `מאשרים את "${label}"…`,
  signingHosted: host => `${heNamed(host, 'השער שלך')} חותם על זה בשמך. רק רגע.`,
  signingOwnNode: host => `${heNamed(host, 'הצומת הזה')} חותם על זה בשמך. רק רגע.`,
  codeHeading: 'הנה הקוד שלך',
  codePasteHint: 'הדביקו אותו בטרמינל שביקש. רק הטרמינל הזה יכול להשתמש בו.',
  codeLabel: 'קוד חד־פעמי',
  copy: 'העתקת הקוד',
  copied: 'הועתק',
  selectedToCopy: 'הקוד מסומן. העתיקו אותו במקלדת או מהתפריט.',
  countdown: clock => `בתוקף עוד ${clock}`,
  announceMinutes: minutes =>
    minutes === 1 ? 'נותרה בערך דקה אחת לשימוש בקוד.' : `נותרו בערך ${minutes} דקות לשימוש בקוד.`,
  announceSeconds: seconds => `נותרו ${seconds} שניות לשימוש בקוד.`,
  expiredHeading: 'תוקף הקוד פג',
  expiredBody: 'הוא היה בתוקף רק כמה דקות. התחילו מחדש מהטרמינל שבמכשיר כדי לקבל קוד חדש.',
  handedBackHeading: 'המכשיר קיבל אותו',
  handedBackBody: 'הטרמינל במחשב הזה קיבל את הקוד. אפשר לסגור את הלשונית.',
  declinedHeading: 'שום דבר לא אושר',
  declinedBody: label => `"${label}" לא מזוהה כמכשיר שלך. אפשר לסגור את הלשונית.`,
  refusedHeading: 'אי אפשר להמשיך עם הבקשה הזו',
  refusal: {
    request_acts_incoherent:
      'המכשיר ביקש לקשור את מפתח השורש שלו בלי להירשם, ואת זה אי אפשר לעשות לבד.',
    act_unknown: 'המכשיר ביקש משהו שהשער שלך לא מכיר, ולכן לא היה מה לאשר.',
    redemption_expired: 'תוקף הקוד פג לפני שנעשה בו שימוש. התחילו מחדש מהטרמינל שבמכשיר.',
    consent_unavailable: 'כרגע אי אפשר לקבל אישורים. נסו שוב בעוד זמן מה.',
  },
  refusedUnknown: 'משהו עצר את הבקשה הזו. התחילו מחדש מהטרמינל שבמכשיר.',
  refusalCodeLabel: 'מזהה',
};

// ---------------------------------------------------------------------------
// Scene helper — the shell around the card, with a footer that says where the
// person is and what happens if they walk away
// ---------------------------------------------------------------------------

interface SceneOptions {
  authority?: AuthorityResolution;
  footer?: unknown;
}

function scene(card: unknown, { authority = hostedByAlpha, footer }: SceneOptions = {}) {
  return html`
    <elohim-imagodei-portal-shell .authority=${authority} step="consent">
      <elohim-imagodei-trust-indicator
        slot="header"
        trust-mode=${authority.trustMode}
        authority-label=${authority.authority.label}
      ></elohim-imagodei-trust-indicator>

      ${card}

      <span slot="footer">${footer ?? REVIEW_FOOTER}</span>
    </elohim-imagodei-portal-shell>
  `;
}

const REVIEW_FOOTER = html`
  Opened from the link your terminal printed. Nothing changes until you choose.
`;

// The card's own closing sentence already says the tab can be closed; the
// footer only says where the person still is.
const DONE_FOOTER = html`
  Still signed in to your doorway, alpha.elohim.host, as Matthew.
`;

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Imagodei/DeviceConsent',
  parameters: {
    layout: 'fullscreen',
    docs: {
      description: {
        component: `
**A machine is asking to act for you.**

Matthew ran a command on his workspace machine. Its terminal printed a link to his doorway's
sign-in portal and is waiting. He signed in; now the doorway shows him which device is asking,
its short key (to compare with what the terminal printed), and each thing it asks for as its
own row he can agree to or leave out. His doorway keeps his key, so it signs when he approves.

Then one of two endings: the code to paste into a remote terminal, large and calm with its
countdown; or, on the same machine, nothing to copy at all — the terminal already has it.

**Binding.** Linen panel on a Linen table with the warm paper shadow; Fraunces heading;
DM Sans for the doing; Source Serif 4 for the two sentences a person must read (what this
does not allow, and the closing message). Vineyard fills the one primary action. The key block
and the code sit on a Harvest Gold wash in JetBrains Mono — the only monospaced things. Dark
is constellation: Starlight on Deep Sky, New Growth for the primary action, Harvest Gold rule.

Library B — primitives untouched; tokens bound by decorator, typography through declared parts.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// 1. Default — enroll only, hosted by his doorway
// ---------------------------------------------------------------------------

export const Default: Story = {
  name: 'Default (approve the workspace machine)',
  decorators: [lightDecorator],
  render: () =>
    scene(html`
      <elohim-imagodei-device-consent-card
        slot="primary"
        .request=${workspaceEnroll}
        person-label="Matthew"
        host-label="alpha.elohim.host"
      ></elohim-imagodei-device-consent-card>
    `),
  parameters: {
    docs: {
      description: {
        story:
          'The terminal on "workspace" asked only to be enrolled. One row, agreed by default. ' +
          'The key block is the thing to check against the terminal; the scope note says ' +
          'plainly what approving does not allow; the signer line says alpha.elohim.host ' +
          'keeps his key and will sign. Approve is the only filled control.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// 2. Two things asked — enroll + root key, both fingerprints
// ---------------------------------------------------------------------------

const twoThingsCard = html`
  <elohim-imagodei-device-consent-card
    slot="primary"
    .request=${workspaceEnrollAndRoot}
    person-label="Matthew"
    host-label="alpha.elohim.host"
  ></elohim-imagodei-device-consent-card>
`;

export const TwoThingsAsked: Story = {
  name: 'Two things asked (enroll + root key)',
  decorators: [lightDecorator],
  render: () => scene(twoThingsCard),
  parameters: {
    docs: {
      description: {
        story:
          'Enrollment and the root key, asked together, as two separate rows. Each shows its ' +
          'own short key so both can be compared with the terminal.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// 3. Agreeing to less — root key left out
// ---------------------------------------------------------------------------

export const AgreeingToLess: Story = {
  name: 'Agreeing to less (root key left out)',
  decorators: [lightDecorator],
  render: () => scene(twoThingsCard),
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
          'Matthew agrees to enrollment and leaves the root key out. Approve stays the ' +
          'primary action; the consent record will list the root key as declined.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Signing — the moment between approving and the code
// ---------------------------------------------------------------------------

export const Signing: Story = {
  name: 'Signing (a moment)',
  decorators: [lightDecorator],
  render: () =>
    scene(html`
      <elohim-imagodei-device-consent-card
        slot="primary"
        .request=${workspaceEnrollAndRoot}
        phase="signing"
        person-label="Matthew"
        host-label="alpha.elohim.host"
      ></elohim-imagodei-device-consent-card>
    `),
};

// ---------------------------------------------------------------------------
// 4. Code to paste — remote device
// ---------------------------------------------------------------------------

const codeCard = () => html`
  <elohim-imagodei-device-consent-card
    slot="primary"
    .request=${workspaceEnroll}
    phase="code"
    code=${CODE}
    .expiresAt=${Date.now() + CODE_REMAINING_MS}
  ></elohim-imagodei-device-consent-card>
`;

const CODE_FOOTER = html`
  Once the terminal has the code, you can close this tab.
`;

export const CodeToPaste: Story = {
  name: 'Code to paste',
  decorators: [lightDecorator],
  render: () => scene(codeCard(), { footer: CODE_FOOTER }),
  parameters: {
    docs: {
      description: {
        story:
          'The device is remote, so the portal shows the one-time code to paste. The code is ' +
          'large, monospaced and unhurried on its Harvest Gold wash; the countdown sits beside ' +
          'the copy button. Still by default — the clock ticks, nothing else moves.',
      },
    },
  },
};

export const CodeRanOut: Story = {
  name: 'Code ran out',
  decorators: [lightDecorator],
  render: () =>
    scene(
      html`
        <elohim-imagodei-device-consent-card
          slot="primary"
          .request=${workspaceEnroll}
          phase="code"
          code=${CODE}
          .expiresAt=${Date.now() - 1000}
        ></elohim-imagodei-device-consent-card>
      `,
      { footer: DONE_FOOTER }
    ),
};

// ---------------------------------------------------------------------------
// 5. Handed back — same machine, nothing to copy
// ---------------------------------------------------------------------------

export const HandedBack: Story = {
  name: 'Handed back (same machine)',
  decorators: [lightDecorator],
  render: () =>
    scene(
      html`
        <elohim-imagodei-device-consent-card
          slot="primary"
          .request=${workspaceEnroll}
          phase="handed-back"
        ></elohim-imagodei-device-consent-card>
      `,
      { footer: DONE_FOOTER }
    ),
  parameters: {
    docs: {
      description: {
        story:
          'The browser and the terminal are on the same machine: the browser handed the code ' +
          'to the terminal. No code is shown, nothing to copy.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// 6. Declined
// ---------------------------------------------------------------------------

export const Declined: Story = {
  name: 'Declined',
  decorators: [lightDecorator],
  render: () =>
    scene(
      html`
        <elohim-imagodei-device-consent-card
          slot="primary"
          .request=${workspaceEnroll}
          phase="declined"
        ></elohim-imagodei-device-consent-card>
      `,
      { footer: DONE_FOOTER }
    ),
};

// ---------------------------------------------------------------------------
// 7. Refused — the doorway would not put these in front of Matthew
// ---------------------------------------------------------------------------

const REFUSED_FOOTER = html`
  Nothing changed on your account.
`;

export const RefusedIncoherent: Story = {
  name: 'Refused (root key without enrolling)',
  decorators: [lightDecorator],
  render: () =>
    scene(
      html`
        <elohim-imagodei-device-consent-card
          slot="primary"
          .request=${{ ...workspaceEnrollAndRoot, askedActs: ['device.bind-root'] }}
          phase="refused"
          refusal-code="request_acts_incoherent"
        ></elohim-imagodei-device-consent-card>
      `,
      { footer: REFUSED_FOOTER }
    ),
  parameters: {
    docs: {
      description: {
        story:
          'A request the doorway would not show: binding a root key without enrolling. The ' +
          'reference is small and quotable for whoever helps; the sentence is the message.',
      },
    },
  },
};

export const RefusedUnavailable: Story = {
  name: 'Refused (approvals unavailable)',
  decorators: [lightDecorator],
  render: () =>
    scene(
      html`
        <elohim-imagodei-device-consent-card
          slot="primary"
          .request=${workspaceEnroll}
          phase="refused"
          refusal-code="consent_unavailable"
        ></elohim-imagodei-device-consent-card>
      `,
      { footer: REFUSED_FOOTER }
    ),
};

// ---------------------------------------------------------------------------
// 8. Own node signs — a person whose own node holds their key
// ---------------------------------------------------------------------------

export const OwnNodeSigns: Story = {
  name: 'Own node signs (Jessica)',
  decorators: [lightDecorator],
  render: () =>
    scene(
      html`
        <elohim-imagodei-device-consent-card
          slot="primary"
          .request=${studioLaptop}
          signer="peer-conductor"
          person-label="Jessica"
          host-label="jessica-desk"
        ></elohim-imagodei-device-consent-card>
      `,
      {
        authority: jessicasOwnNode,
        footer: html`
          Your own node, jessica-desk, holds your key. Nothing changes until you choose.
        `,
      }
    ),
  parameters: {
    docs: {
      description: {
        story:
          'Jessica runs her own node. The signer line says this node holds her key and will ' +
          'sign; the header indicator switches to the own-node mode with its Morning accent.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// 9. Dark (constellation) — scenes 2 and 4
// ---------------------------------------------------------------------------

export const TwoThingsAskedDark: Story = {
  name: 'Two things asked — Dark (constellation)',
  decorators: [darkDecorator],
  render: () => scene(twoThingsCard),
};

export const CodeToPasteDark: Story = {
  name: 'Code to paste — Dark (constellation)',
  decorators: [darkDecorator],
  render: () => scene(codeCard(), { footer: CODE_FOOTER }),
};

// ---------------------------------------------------------------------------
// 10. Phone width — scene 2 at 360px
// ---------------------------------------------------------------------------

export const TwoThingsAskedPhone: Story = {
  name: 'Two things asked — Phone (360px)',
  decorators: [lightDecorator],
  render: () => scene(twoThingsCard),
  parameters: {
    docs: {
      description: {
        story:
          'Scene 2 at a 360px phone width. The same fluid binding drops the side margins to ' +
          '24px and the panel padding to 16px; act rows, fingerprints and both actions must fit ' +
          'without sideways scrolling.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// 11. Hebrew — RTL canary
// ---------------------------------------------------------------------------

export const Hebrew: Story = {
  name: 'Hebrew (RTL canary)',
  decorators: [hebrewDecorator],
  render: () =>
    scene(
      html`
        <elohim-imagodei-device-consent-card
          slot="primary"
          .request=${workspaceEnrollAndRoot}
          .strings=${HE_STRINGS}
          person-label="Matthew"
          host-label="alpha.elohim.host"
        ></elohim-imagodei-device-consent-card>
      `,
      {
        footer: html`
          נפתח מהקישור שהטרמינל הדפיס. שום דבר לא משתנה עד שתבחרו.
        `,
      }
    ),
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary in he-IL. Every visible sentence of the card comes through `strings`. ' +
          'Names (Matthew, workspace, alpha.elohim.host) stay Latin and isolated, so the ' +
          'quotation marks and parentheses around them land on the right side; the ' +
          'fingerprints stay left-to-right.',
      },
    },
  },
};
