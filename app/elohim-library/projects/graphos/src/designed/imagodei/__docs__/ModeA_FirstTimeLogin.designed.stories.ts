/**
 * Library B designed story — ModeA_FirstTimeLogin
 *
 * Scene: A first-time visitor arrives at alpha.elohim.host. They have no
 * remembered address, no witnesses yet. The doorway-host trust mode is
 * active; the flywheel hint surfaces the graduation path to a peer conductor.
 *
 * Composition: portal-shell (doorway-host, flywheel-hint) + trust-indicator
 * (header slot) + a welcome heading and the federated-resolver (primary slot).
 * The resolver step is the entry to the protocol — the household begins here.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — AuthorityResolution, FederatedResolveOutcome from
 *      elohim-imagodei (typed output of spec §5.1).
 *   2. Manifest vocabulary — imagodei domain; returnTo is the fair-exchange
 *      lamad concept path.
 *   3. Brand tokens — inline EL_TOKENS bag (graphos design spec §14) until the
 *      global stylesheet ships; bound at story-decorator level only.
 *
 * Library B boundary: NEVER modifies any primitive's CSS, JSDoc, or tag name.
 * Brand binding happens via declared CSS custom properties in story
 * decorators; the welcome heading's face, the primary action's fill and the
 * panel's paper shadow go through declared `@csspart`s, which custom-property
 * inheritance cannot reach.
 *
 * Brand voice: "household" not "user"; "welcome" not "onboarding"; plain
 * words a newcomer already knows ("address", "your doorway").
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';

import type {
  AuthorityResolution,
  FederatedResolveOutcome,
  ResolveIdentifierFn,
} from 'elohim-imagodei';

// ---------------------------------------------------------------------------
// Brand token declaration — graphos design spec §14
// Inline because no global stylesheet seeds them into storybook today.
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
  --el-plum:        #6E4B6B;
  --el-starlight:   #E8E4D9;
  --el-night:       #0F1A12;
  --el-font-display: 'Fraunces', Georgia, serif;
  --el-font-body:    'Source Serif 4', Georgia, serif;
  --el-font-ui:      'DM Sans', system-ui, sans-serif;
  --el-font-mono:    'JetBrains Mono', monospace;
  --el-space-xs:  8px;
  --el-space-sm:  16px;
  --el-space-md:  24px;
  --el-space-lg:  32px;
  --el-space-xl:  48px;
  --el-radius-sm: 4px;
  --el-radius-md: 8px;
  --el-radius-lg: 16px;
  --el-shadow-soft:   0 2px 8px rgba(107, 97, 87, 0.08);
  --el-shadow-medium: 0 4px 16px rgba(107, 97, 87, 0.12);
`;

// Portal frame — shared by both themes. The block padding sets the column at
// the optical centre (about a third down a desktop window) instead of pinning
// it to the top edge; inline padding is 48px on desktop, the spec's 24px floor
// on a phone. The panel gets room to breathe (40px desktop, 16px phone).
const PORTAL_FRAME = `
  --elohim-portal-max-inline-size: 448px;
  --elohim-portal-padding:         clamp(24px, 16vh, 160px) clamp(24px, 5vw, 48px);
  --elohim-portal-panel-padding:   clamp(16px, 4.5vw, 40px);
  --elohim-portal-panel-radius:    var(--el-radius-lg);
  --elohim-portal-grid-gap:        var(--el-space-md);
  --elohim-portal-footer-size:     0.875rem;
`;

// Portal-shell binding — light mode (Linen surface, Deep Sky text)
const PORTAL_TOKENS_LIGHT = `
  ${PORTAL_FRAME}
  --elohim-portal-bg:       var(--el-cream);
  --elohim-portal-fg:       var(--el-night);
  --elohim-portal-panel-bg: var(--el-linen);
`;

// Trust-indicator binding — host accent is Harvest Gold
const TRUST_TOKENS_LIGHT = `
  --elohim-trust-bg:           transparent;
  --elohim-trust-fg:           var(--el-stone);
  --elohim-trust-host-accent:  var(--el-amber);
  --elohim-trust-peer-accent:  var(--el-sky);
  --elohim-trust-radius:       var(--el-radius-sm);
  --elohim-trust-padding-block:  0.25rem;
  --elohim-trust-padding-inline: 0.625rem;
`;

// Federated-resolver binding. `--elohim-input-border` is consumed as the
// COLOR inside the primitive's own `1px solid var(...)`, so it takes a color
// only — a full shorthand here invalidates the declaration and the field
// renders with no border at all.
const RESOLVER_TOKENS_LIGHT = `
  --elohim-resolver-bg:         transparent;
  --elohim-resolver-fg:         var(--el-night);
  --elohim-resolver-gap:        var(--el-space-sm);
  --elohim-resolver-label-size: 0.9375rem;
  --elohim-input-border:        rgba(107, 97, 87, 0.55);
  --elohim-input-focus-ring:    var(--el-green-deep);
  --elohim-input-error-fg:      #8E4A37;
`;

// Dark-mode token bag — constellation register
const PORTAL_TOKENS_DARK = `
  ${PORTAL_FRAME}
  --elohim-portal-bg:       var(--el-night);
  --elohim-portal-fg:       var(--el-starlight);
  --elohim-portal-panel-bg: #162019;
`;

const TRUST_TOKENS_DARK = `
  --elohim-trust-bg:           rgba(232, 228, 217, 0.07);
  --elohim-trust-fg:           var(--el-starlight);
  --elohim-trust-host-accent:  var(--el-amber);
  --elohim-trust-peer-accent:  var(--el-sky);
  --elohim-trust-radius:       var(--el-radius-sm);
  --elohim-trust-padding-block:  0.25rem;
  --elohim-trust-padding-inline: 0.625rem;
`;

const RESOLVER_TOKENS_DARK = `
  --elohim-resolver-bg:         transparent;
  --elohim-resolver-fg:         var(--el-starlight);
  --elohim-resolver-gap:        var(--el-space-sm);
  --elohim-resolver-label-size: 0.9375rem;
  --elohim-input-border:        rgba(232, 228, 217, 0.4);
  --elohim-input-focus-ring:    var(--el-amber);
  --elohim-input-error-fg:      #D99A85;
`;

/**
 * What custom properties cannot reach, through declared parts only:
 *   - the resolver's submit is THE primary action — Vineyard fill (New Growth
 *     in the dark). While the address field is empty the primitive disables
 *     it; it stays recognisably the same button, quieter, until there is
 *     something to continue with.
 *   - the input gets a comfortable target and the spec's 4px input radius.
 *   - the panel floats on the table with the warm paper shadow.
 * The welcome heading and lead are the story's own light-DOM elements.
 */
const SIGN_IN_CSS = html`
  <style>
    .el-signin elohim-imagodei-federated-resolver::part(input) {
      padding-block: 0.75rem;
      border-radius: var(--el-radius-sm);
    }

    .el-signin elohim-imagodei-federated-resolver::part(submit) {
      margin-block-start: var(--el-space-xs);
      padding-block: 0.75rem;
      border-radius: var(--el-radius-md);
      font-weight: 600;
      cursor: pointer;
    }

    .el-signin--light elohim-imagodei-federated-resolver::part(submit) {
      background: var(--el-green-deep);
      border-color: var(--el-green-deep);
      color: var(--el-linen);
    }

    .el-signin--dark elohim-imagodei-federated-resolver::part(submit) {
      background: var(--el-green-light);
      border-color: var(--el-green-light);
      color: var(--el-night);
    }

    .el-signin elohim-imagodei-federated-resolver::part(submit):disabled {
      cursor: default;
    }

    .el-signin elohim-imagodei-federated-resolver::part(help) {
      font-size: 0.875rem;
      line-height: 1.5;
      opacity: 0.8;
    }

    .el-signin--light elohim-imagodei-portal-shell::part(primary-region) {
      box-shadow: var(--el-shadow-soft);
    }

    .el-signin--dark elohim-imagodei-portal-shell::part(primary-region) {
      box-shadow: 0 0 0 1px rgba(232, 228, 217, 0.06);
    }

    .el-signin .el-signin__welcome {
      display: grid;
      gap: var(--el-space-xs);
      margin-block-end: var(--el-space-md);
    }

    .el-signin .el-signin__welcome h1 {
      margin: 0;
      font-family: var(--el-font-display);
      font-weight: 500;
      font-size: 2rem;
      line-height: 1.15;
      font-variation-settings:
        'SOFT' 50,
        'WONK' 1;
    }

    .el-signin .el-signin__welcome p {
      margin: 0;
      font-family: var(--el-font-body);
      font-size: 1.0625rem;
      line-height: 1.6;
    }

    .el-signin .el-signin__footer a {
      color: inherit;
      text-underline-offset: 0.2em;
    }
  </style>
`;

// ---------------------------------------------------------------------------
// Fixtures — realistic protocol vocabulary
// ---------------------------------------------------------------------------

/**
 * Doorway-host authority resolution for alpha.elohim.host.
 * Flywheel hint true — first-time visitor, so attestors are empty.
 */
const firstVisitResolution: AuthorityResolution = {
  trustMode: 'doorway-host',
  authority: { label: 'alpha.elohim.host', id: 'alpha' },
  flywheelHint: true,
  attestors: [],
};

/**
 * Success resolver — discovers alpha.elohim.host for matthew's address.
 * The returnTo path is the fair-exchange lamad concept.
 */
const alphaResolver: ResolveIdentifierFn = async (
  identifier: string
): Promise<FederatedResolveOutcome> => {
  await new Promise(r => setTimeout(r, 500));
  return {
    ok: true,
    doorwayUrl: `https://alpha.elohim.host/doorway?for=${encodeURIComponent(identifier)}&returnTo=%2Flamad%2Fconcept%2Ffair-exchange`,
  };
};

// ---------------------------------------------------------------------------
// Decorator factories
// ---------------------------------------------------------------------------

function lightDecorator(story: () => unknown) {
  return html`
    ${SIGN_IN_CSS}
    <div
      class="el-signin el-signin--light"
      style="
        ${EL_TOKENS}
        ${PORTAL_TOKENS_LIGHT}
        ${TRUST_TOKENS_LIGHT}
        ${RESOLVER_TOKENS_LIGHT}
        font-family: var(--el-font-ui);
        background: var(--el-cream);
        min-block-size: 100vh;
        color: var(--el-night);
      "
    >
      ${story()}
    </div>
  `;
}

function darkDecorator(story: () => unknown) {
  return html`
    ${SIGN_IN_CSS}
    <div
      class="el-signin el-signin--dark"
      style="
        ${EL_TOKENS}
        ${PORTAL_TOKENS_DARK}
        ${TRUST_TOKENS_DARK}
        ${RESOLVER_TOKENS_DARK}
        font-family: var(--el-font-ui);
        background: var(--el-night);
        min-block-size: 100vh;
        color: var(--el-starlight);
        color-scheme: dark;
      "
    >
      ${story()}
    </div>
  `;
}

// ---------------------------------------------------------------------------
// Scene — one composition, copy supplied per locale
// ---------------------------------------------------------------------------

interface SignInCopy {
  heading: string;
  lead: string;
  help?: unknown;
  footer: unknown;
}

const EN_COPY: SignInCopy = {
  heading: 'Welcome',
  lead: 'Start with your address, and we’ll find the doorway that keeps your account.',
  help: 'It looks like name@alpha.elohim.host. Not sure what yours is? Ask the neighbor who invited you.',
  footer: html`
    You’re at alpha.elohim.host ·
    <a href="/identity/help">Need help signing in?</a>
  `,
};

function signInScene(
  copy: SignInCopy,
  resolver: ResolveIdentifierFn = alphaResolver,
  placeholder?: string
) {
  return html`
    <elohim-imagodei-portal-shell .authority=${firstVisitResolution} step="resolve" flywheel-hint>
      <elohim-imagodei-trust-indicator
        slot="header"
        trust-mode="doorway-host"
        authority-label="alpha.elohim.host"
        flywheel-hint
      ></elohim-imagodei-trust-indicator>

      <div slot="primary" class="el-signin__welcome">
        <h1>${copy.heading}</h1>
        <p>${copy.lead}</p>
      </div>

      <elohim-imagodei-federated-resolver
        slot="primary"
        remember-key="elohim_auth_identifier"
        placeholder=${placeholder ?? 'you@your-doorway.host'}
        .resolveIdentifier=${resolver}
      >
        ${copy.help
          ? html`
              <span slot="help-text">${copy.help}</span>
            `
          : ''}
      </elohim-imagodei-federated-resolver>

      <span slot="footer" class="el-signin__footer">${copy.footer}</span>
    </elohim-imagodei-portal-shell>
  `;
}

/** Type an address into the resolver, as a person would, so Continue wakes. */
async function typeAddress(canvasElement: HTMLElement, address: string): Promise<void> {
  const resolver = canvasElement.querySelector('elohim-imagodei-federated-resolver');
  await (resolver as { updateComplete?: Promise<unknown> } | null)?.updateComplete;
  const input = resolver?.shadowRoot?.querySelector<HTMLInputElement>('[part="input"]');
  if (!input) return;
  input.value = address;
  input.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
}

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Imagodei/ModeA_FirstTimeLogin',
  parameters: {
    layout: 'fullscreen',
    docs: {
      description: {
        component: `
**Welcome to the commons.**

A first-time visitor arrives at alpha.elohim.host — Mode A (doorway-host). No remembered
address, no witnesses yet. The flywheel hint on the trust-indicator surfaces the graduation
path to running one's own node.

The federated-resolver is the entry step: the household gives its address (e.g.
\`matthew@alpha.elohim.host\`), alpha.elohim.host discovers the doorway endpoint, then
advances to the login step. The returnTo destination is \`/lamad/concept/fair-exchange\` —
the household came from a learning path and will be returned there after sign-in.

**Composition.** A Fraunces welcome and a one-line Source Serif lead sit above the field;
the column rests at the optical centre of the window, not pinned to its top edge. Continue
is the one filled control (Vineyard; New Growth in the dark) — the primitive keeps it
disabled until an address is typed, so it rests quieter, then wakes (see *Address typed*).
The placeholder is a pattern, not a plausible address, so an empty field never looks filled.

**Brand binding:** Linen panel on a Linen table with the warm paper shadow, Deep Sky text,
Harvest Gold trust accent. DM Sans for the doing; serif for the sentence a person reads.

Library B — primitive untouched; tokens bound via story decorator, parts styled from above.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Default — first-time visit, resolve step
// ---------------------------------------------------------------------------

export const Default: Story = {
  name: 'Default (first-time welcome)',
  decorators: [lightDecorator],
  render: () => signInScene(EN_COPY),
  parameters: {
    docs: {
      description: {
        story:
          'First-time welcome — Mode A entry point. A welcome heading, one sentence on what ' +
          'happens next, the address field with a plain example, and Continue as the single ' +
          'primary action (resting, disabled, until there is an address). The footer says where ' +
          'the person is and offers help in words, not an icon.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Address typed — the primary action awake
// ---------------------------------------------------------------------------

export const AddressTyped: Story = {
  name: 'Address typed (Continue ready)',
  decorators: [lightDecorator],
  render: () => signInScene(EN_COPY),
  play: async ({ canvasElement }) => {
    await typeAddress(canvasElement, 'matthew@alpha.elohim.host');
  },
  parameters: {
    docs: {
      description: {
        story: 'The same welcome once an address is typed: Continue is enabled at full Vineyard.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Dark theme
// ---------------------------------------------------------------------------

export const Dark: Story = {
  name: 'Dark (constellation)',
  decorators: [darkDecorator],
  render: () => signInScene(EN_COPY),
  parameters: {
    docs: {
      description: {
        story:
          'Dark / constellation register. Deep Sky background (#0F1A12 — carries the green ' +
          'undertone of growing things, not the cold black of outer space). Starlight text ' +
          '(#E8E4D9 — warm, not pure white). Continue fills with New Growth so the one action ' +
          'still reads first; Harvest Gold holds the trust-indicator and the focus ring.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// RTL canary — Hebrew locale
// ---------------------------------------------------------------------------

const HE_COPY: SignInCopy = {
  heading: 'ברוכים הבאים',
  lead: 'התחילו בכתובת שלכם, ואנחנו נמצא את השער שמחזיק את החשבון שלכם.',
  // The Latin address is wrapped in first-strong isolates (U+2068 … U+2069) so
  // the sentence's full stop stays at the Hebrew end of the line.
  help: 'היא נראית כמו ⁨name@alpha.elohim.host⁩. לא בטוחים מה הכתובת שלכם? שאלו את השכן שהזמין אתכם.',
  footer: html`
    זהו alpha.elohim.host ·
    <a href="/identity/help">צריכים עזרה בכניסה?</a>
  `,
};

export const Hebrew: Story = {
  name: 'Hebrew (RTL canary)',
  decorators: [
    (story: () => unknown) => html`
      ${SIGN_IN_CSS}
      <div
        dir="rtl"
        lang="he"
        class="el-signin el-signin--light"
        style="
          ${EL_TOKENS}
          ${PORTAL_TOKENS_LIGHT}
          ${TRUST_TOKENS_LIGHT}
          ${RESOLVER_TOKENS_LIGHT}
          font-family: var(--el-font-ui);
          background: var(--el-cream);
          min-block-size: 100vh;
          color: var(--el-night);
        "
      >
        ${story()}
      </div>
    `,
  ],
  render: () => signInScene(HE_COPY),
  parameters: {
    docs: {
      description: {
        story:
          'RTL canary — he-IL locale in a dir="rtl" container. Logical CSS properties ' +
          '(padding-inline, padding-block, margin-inline) in the primitives mirror correctly. ' +
          'The trust-indicator chip should sit at inline-start in RTL. The resolver label ' +
          'and input read right-to-left. The story-owned welcome and footer are Hebrew; the ' +
          'resolver label and button are not yet localisable by the host.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Resolve-error state — unknown host
// ---------------------------------------------------------------------------

export const UnknownHost: Story = {
  name: 'UnknownHost (resolve error)',
  decorators: [lightDecorator],
  render: () => {
    const unknownHostResolver: ResolveIdentifierFn = async (): Promise<FederatedResolveOutcome> => {
      await new Promise(r => setTimeout(r, 400));
      return {
        ok: false,
        reason:
          'No doorway answered for that address. Check the spelling, or ask the neighbor who invited you.',
      };
    };

    return signInScene(EN_COPY, unknownHostResolver);
  },
  play: async ({ canvasElement }) => {
    await typeAddress(canvasElement, 'matthew@unknown.host');
    const resolver = canvasElement.querySelector('elohim-imagodei-federated-resolver');
    await (resolver as { updateComplete?: Promise<unknown> } | null)?.updateComplete;
    resolver?.shadowRoot?.querySelector<HTMLButtonElement>('[part="submit"]')?.click();
  },
  parameters: {
    docs: {
      description: {
        story:
          'Resolve-error state — the doorway cannot find the address. ' +
          'Enter an address and submit; the resolver returns an error. ' +
          'The error part uses a deepened Terracotta (#8E4A37 light, #D99A85 dark) — earthy ' +
          'caution at a readable contrast, never aggressive red. The protocol has no ' +
          '"danger red." The household is pointed to the neighbor who invited them.',
      },
    },
  },
};
