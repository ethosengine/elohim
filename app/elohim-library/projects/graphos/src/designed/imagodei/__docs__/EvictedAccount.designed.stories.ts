/**
 * Library B designed story — EvictedAccount
 *
 * Scene: §4.1 doorway-eviction error surface. Matthew's account has been evicted
 * from alpha.elohim.host — the doorway stopped hosting him. This might happen
 * because the doorway's covenant with him has ended (usage policy breach, account
 * migration, doorway shutdown).
 *
 * The portal-shell renders the error-region slot instead of the primary step.
 * The trust-indicator is preserved at the header — it is important the household
 * knows WHICH doorway evicted them. Without this, recovery is impossible.
 *
 * The error region provides:
 *   1. Plain explanation: "alpha.elohim.host has stopped hosting your account."
 *   2. "Recover + migrate" affordance — the household's data can move.
 *   3. "Talk to operator" affordance — the doorway steward may have answers.
 *
 * Brand voice: the protocol's stance is that eviction is a doorway decision, not
 * a household failure. The copy is calm and practical, not accusatory.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — AuthorityResolution from elohim-imagodei (authority label for the
 *      evicting doorway; attestors preserved from last known state).
 *   2. Manifest vocabulary — imagodei domain; eviction is an operational event.
 *   3. Brand tokens — inline EL_TOKENS bag (graphos design spec §14).
 *
 * Library B boundary: NEVER modifies any primitive's CSS, JSDoc, or tag name.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';

import type {
  AuthorityResolution,
  AttestorRef,
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

// Portal-shell — light mode
const PORTAL_TOKENS_LIGHT = `
  --elohim-portal-bg:           var(--el-cream);
  --elohim-portal-fg:           var(--el-night);
  --elohim-portal-panel-bg:     var(--el-linen);
  --elohim-portal-panel-radius: var(--el-radius-lg);
  --elohim-portal-grid-gap:     var(--el-space-md);
  --elohim-portal-padding:      var(--el-space-xl);
  --elohim-portal-footer-size:  0.8125rem;
`;

// Trust-indicator — kept visible even in eviction; this is the evicting doorway
const TRUST_TOKENS_LIGHT = `
  --elohim-trust-bg:             transparent;
  --elohim-trust-fg:             var(--el-stone);
  --elohim-trust-host-accent:    var(--el-clay);
  --elohim-trust-peer-accent:    var(--el-sky);
  --elohim-trust-radius:         var(--el-radius-sm);
  --elohim-trust-padding-block:  0.25rem;
  --elohim-trust-padding-inline: 0.625rem;
`;

// Error region shared styles
const ERROR_REGION_STYLE = `
  background: rgba(184, 102, 79, 0.07);
  border: 1px solid rgba(184, 102, 79, 0.3);
  border-radius: 8px;
  padding: 24px;
  max-inline-size: 480px;
  font-family: 'Source Serif 4', Georgia, serif;
`;

// Attestor-row
const ATTESTOR_TOKENS_LIGHT = `
  --elohim-attestor-avatar-size:     32px;
  --elohim-attestor-bg:              var(--el-green-light);
  --elohim-attestor-ring:            var(--el-cream);
  --elohim-attestor-gap:             -6px;
  --elohim-attestor-overflow-opacity: 0.75;
`;

// Dark mode
const PORTAL_TOKENS_DARK = `
  --elohim-portal-bg:           var(--el-night);
  --elohim-portal-fg:           var(--el-starlight);
  --elohim-portal-panel-bg:     #162019;
  --elohim-portal-panel-radius: var(--el-radius-lg);
  --elohim-portal-grid-gap:     var(--el-space-md);
  --elohim-portal-padding:      var(--el-space-xl);
  --elohim-portal-footer-size:  0.8125rem;
`;

const TRUST_TOKENS_DARK = `
  --elohim-trust-bg:             rgba(184, 102, 79, 0.1);
  --elohim-trust-fg:             var(--el-starlight);
  --elohim-trust-host-accent:    var(--el-clay);
  --elohim-trust-peer-accent:    var(--el-sky);
  --elohim-trust-radius:         var(--el-radius-sm);
  --elohim-trust-padding-block:  0.25rem;
  --elohim-trust-padding-inline: 0.625rem;
`;

const ATTESTOR_TOKENS_DARK = `
  --elohim-attestor-avatar-size:     32px;
  --elohim-attestor-bg:              var(--el-green-deep);
  --elohim-attestor-ring:            var(--el-night);
  --elohim-attestor-gap:             -6px;
  --elohim-attestor-overflow-opacity: 0.75;
`;

// ---------------------------------------------------------------------------
// Fixtures — realistic protocol vocabulary
// ---------------------------------------------------------------------------

const susanElder: AttestorRef = {
  eprRef: 'epr:attestor:susan-qahal-elder-aleph',
  displayName: 'Susan Miller',
  role: 'qahal-elder',
};

const jamesCircle: AttestorRef = {
  eprRef: 'epr:attestor:james-dowell-intimate',
  displayName: 'James Dowell',
  role: 'intimate-circle',
};

const martaWitness: AttestorRef = {
  eprRef: 'epr:attestor:marta-reyes-witness',
  displayName: 'Marta Reyes',
  role: 'recovery-witness',
};

/**
 * Eviction authority resolution. The trust-indicator carries the evicting doorway
 * label — the household must know who evicted them to begin the migration process.
 * The trust-mode stays 'doorway-host'; the host-accent is shifted to clay (terracotta)
 * to signal the relationship has changed without being alarming.
 */
const evictedResolution: AuthorityResolution = {
  trustMode: 'doorway-host',
  authority: { label: 'alpha.elohim.host', id: 'alpha' },
  flywheelHint: false,
  attestors: [susanElder, jamesCircle, martaWitness],
};

// ---------------------------------------------------------------------------
// Decorators
// ---------------------------------------------------------------------------

function lightDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${PORTAL_TOKENS_LIGHT}
        ${TRUST_TOKENS_LIGHT}
        ${ATTESTOR_TOKENS_LIGHT}
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
    <div
      style="
        ${EL_TOKENS}
        ${PORTAL_TOKENS_DARK}
        ${TRUST_TOKENS_DARK}
        ${ATTESTOR_TOKENS_DARK}
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
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Imagodei/EvictedAccount',
  parameters: {
    docs: {
      description: {
        component: `
**alpha.elohim.host has stopped hosting your account.**

Spec §4.1 eviction surface. The portal-shell renders the error-region slot. The
trust-indicator is preserved at the top so the household knows WHICH doorway made
this decision — without that, recovery is impossible.

The error copy is calm and practical: this is a doorway decision, not a household
failure. The household has two paths: recover + migrate their data to a new doorway,
or talk to the operator to understand what happened.

The trust-indicator host accent shifts to Terracotta (clay, #B8664F) to signal
that the relationship has changed. The attestors (Susan, James, Marta) remain
visible in the row — they are still the household's recovery witnesses regardless
of what alpha.elohim.host decided.

Library B — primitive untouched; brand tokens bound via story decorator.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Default — eviction error, light mode
// ---------------------------------------------------------------------------

export const Default: Story = {
  name: 'Default (eviction)',
  decorators: [lightDecorator],
  render: () => {
    return html`
      <elohim-imagodei-portal-shell
        .authority=${evictedResolution}
      >
        <elohim-imagodei-trust-indicator
          slot="header"
          trust-mode="doorway-host"
          authority-label="alpha.elohim.host"
        ></elohim-imagodei-trust-indicator>

        <p slot="primary" style="display: none;"></p>

        <div slot="error-region" role="alert" style="${ERROR_REGION_STYLE}">
          <p style="
            font-family: var(--el-font-display);
            font-size: 1.25rem;
            font-weight: 500;
            color: var(--el-clay);
            margin-block: 0 var(--el-space-sm);
            line-height: 1.3;
          ">
            alpha.elohim.host has stopped hosting your account.
          </p>

          <p style="
            font-family: var(--el-font-body);
            font-size: 0.9375rem;
            color: var(--el-stone);
            margin-block: 0 var(--el-space-md);
            line-height: 1.6;
          ">
            Your data and your learning record still belong to you — this is a
            doorway stewardship decision, not a loss of your participation in the
            commons. You can migrate to another doorway or move to a peer conductor.
          </p>

          <nav style="
            display: flex;
            flex-direction: column;
            gap: var(--el-space-xs);
            margin-block-end: var(--el-space-md);
          ">
            <a
              href="/identity/migrate"
              style="
                font-family: var(--el-font-ui);
                font-size: 0.9375rem;
                font-weight: 600;
                color: var(--el-linen);
                background: var(--el-green-deep);
                border-radius: var(--el-radius-md);
                padding: 0.625rem 1.25rem;
                text-decoration: none;
                display: inline-block;
                inline-size: fit-content;
              "
            >Recover + migrate your account</a>

            <a
              href="mailto:steward@alpha.elohim.host"
              style="
                font-family: var(--el-font-ui);
                font-size: 0.9375rem;
                color: var(--el-stone);
                text-decoration: underline;
                text-underline-offset: 2px;
              "
            >Talk to the alpha.elohim.host steward</a>
          </nav>

          <p style="
            font-family: var(--el-font-body);
            font-size: 0.8125rem;
            color: var(--el-stone);
            opacity: 0.75;
            margin-block: 0;
          ">
            Susan, James, and Marta can help you through the migration process.
          </p>
        </div>

        <span slot="footer">
          <a href="/identity/help" style="color: inherit; opacity: 0.7;">need help?</a>
          &middot; commons stewardship
        </span>
      </elohim-imagodei-portal-shell>
    `;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Eviction error surface — §4.1. The trust-indicator remains at the header: ' +
          '"alpha.elohim.host" with the host-accent shifted to Terracotta (#B8664F). ' +
          'The attestor-row (Susan, James, Marta) stays visible — they are the household\'s ' +
          'recovery path regardless of doorway status. The error region uses a warm clay ' +
          'tint (rgba(184, 102, 79, 0.07)) — caution without alarm. Two affordances: ' +
          '"Recover + migrate" (primary path) and "Talk to the steward" (secondary). ' +
          'The bottom line names the witnesses by first name.',
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
  render: () => {
    const darkErrorRegionStyle = `
      background: rgba(184, 102, 79, 0.1);
      border: 1px solid rgba(184, 102, 79, 0.25);
      border-radius: 8px;
      padding: 24px;
      max-inline-size: 480px;
      font-family: 'Source Serif 4', Georgia, serif;
    `;

    return html`
      <elohim-imagodei-portal-shell
        .authority=${evictedResolution}
      >
        <elohim-imagodei-trust-indicator
          slot="header"
          trust-mode="doorway-host"
          authority-label="alpha.elohim.host"
        ></elohim-imagodei-trust-indicator>

        <p slot="primary" style="display: none;"></p>

        <div slot="error-region" role="alert" style="${darkErrorRegionStyle}">
          <p style="
            font-family: var(--el-font-display);
            font-size: 1.25rem;
            font-weight: 500;
            color: var(--el-clay);
            margin-block: 0 var(--el-space-sm);
            line-height: 1.3;
          ">
            alpha.elohim.host has stopped hosting your account.
          </p>

          <p style="
            font-family: var(--el-font-body);
            font-size: 0.9375rem;
            color: var(--el-starlight);
            opacity: 0.8;
            margin-block: 0 var(--el-space-md);
            line-height: 1.6;
          ">
            Your data and your learning record still belong to you. Migrate to
            another doorway or move to a peer conductor.
          </p>

          <nav style="
            display: flex;
            flex-direction: column;
            gap: var(--el-space-xs);
            margin-block-end: var(--el-space-md);
          ">
            <a
              href="/identity/migrate"
              style="
                font-family: var(--el-font-ui);
                font-size: 0.9375rem;
                font-weight: 600;
                color: var(--el-starlight);
                background: var(--el-green-deep);
                border-radius: var(--el-radius-md);
                padding: 0.625rem 1.25rem;
                text-decoration: none;
                display: inline-block;
                inline-size: fit-content;
              "
            >Recover + migrate your account</a>

            <a
              href="mailto:steward@alpha.elohim.host"
              style="
                font-family: var(--el-font-ui);
                font-size: 0.9375rem;
                color: var(--el-starlight);
                opacity: 0.7;
                text-decoration: underline;
                text-underline-offset: 2px;
              "
            >Talk to the alpha.elohim.host steward</a>
          </nav>
        </div>

        <span slot="footer">
          <a href="/identity/help" style="color: inherit; opacity: 0.5;">need help?</a>
          &middot; commons stewardship
        </span>
      </elohim-imagodei-portal-shell>
    `;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Dark constellation register. The clay tint for the error region is slightly ' +
          'richer in dark mode — the warmth is visible against Deep Sky without becoming ' +
          'hot. The trust-indicator host-accent (clay) now reads as a gentle ember rather ' +
          'than a warning flash. The Vineyard green migration button holds its anchor.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// With active recovery path — Susan has initiated recovery
// ---------------------------------------------------------------------------

export const RecoveryInProgress: Story = {
  name: 'RecoveryInProgress',
  decorators: [lightDecorator],
  render: () => {
    return html`
      <elohim-imagodei-portal-shell
        .authority=${evictedResolution}
      >
        <elohim-imagodei-trust-indicator
          slot="header"
          trust-mode="doorway-host"
          authority-label="alpha.elohim.host"
        ></elohim-imagodei-trust-indicator>

        <p slot="primary" style="display: none;"></p>

        <div slot="error-region" role="status" style="${ERROR_REGION_STYLE}">
          <p style="
            font-family: var(--el-font-display);
            font-size: 1.125rem;
            font-weight: 500;
            color: var(--el-green-deep);
            margin-block: 0 var(--el-space-sm);
          ">
            Recovery is underway.
          </p>

          <p style="
            font-family: var(--el-font-body);
            font-size: 0.9375rem;
            color: var(--el-stone);
            margin-block: 0 var(--el-space-md);
            line-height: 1.6;
          ">
            Susan Miller (qahal-elder) has initiated a recovery session on your behalf.
            When she completes the attestation, you will be able to migrate to
            beta.elohim.host or run your own conductor.
          </p>

          <p style="
            font-family: var(--el-font-body);
            font-size: 0.8125rem;
            color: var(--el-stone);
            opacity: 0.75;
            margin-block: 0;
          ">
            Recovery code: <code style="font-family: var(--el-font-mono); letter-spacing: 0.08em;">ALB-2026-7742</code>
          </p>
        </div>

        <span slot="footer">commons stewardship</span>
      </elohim-imagodei-portal-shell>
    `;
  },
  parameters: {
    docs: {
      description: {
        story:
          'Recovery in progress: Susan Miller (qahal-elder) has initiated recovery. ' +
          'The error-region shifts from an alert (role="alert") to a status (role="status"). ' +
          'The Vineyard green headline signals the active path forward. The recovery code ' +
          'in JetBrains Mono (--el-font-mono) carries the reference number. This is the ' +
          '"log in with help from your people" moment made concrete.',
      },
    },
  },
};
