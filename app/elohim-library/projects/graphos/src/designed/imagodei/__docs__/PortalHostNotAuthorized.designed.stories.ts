/**
 * Library B designed story — PortalHostNotAuthorized
 *
 * Scene: §4.5 substrate-gate surface. Matthew entered `matthew@beta.elohim.host`
 * on alpha.elohim.host's portal, but beta.elohim.host has not authorized alpha as
 * a portal-host for its accounts. The portal cannot proceed.
 *
 * Key design decisions:
 *   1. Trust-indicator is HIDDEN (no authority resolved). Alpha.elohim.host is the
 *      portal host but it has no standing to claim authority over beta.elohim.host
 *      accounts. Showing the trust-indicator here would be dishonest.
 *   2. The error region explains the situation plainly — this is not Matthew's
 *      fault; it is an inter-doorway authorization gap.
 *   3. A static link to /identity/recovery gives the household an exit path.
 *
 * The portal-shell renders the error-region slot. No primary step content.
 *
 * Sources of truth (per elohim-library/CLAUDE.md):
 *   1. Types — no AuthorityResolution available; the shell has no resolved authority.
 *   2. Manifest vocabulary — imagodei domain; portal-host authorization.
 *   3. Brand tokens — inline EL_TOKENS bag (graphos design spec §14).
 *
 * Library B boundary: NEVER modifies any primitive's CSS, JSDoc, or tag name.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-imagodei/register';

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

// Portal-shell — no trust header shown; shell still provides layout
const PORTAL_TOKENS_LIGHT = `
  --elohim-portal-bg:           var(--el-cream);
  --elohim-portal-fg:           var(--el-night);
  --elohim-portal-panel-bg:     var(--el-linen);
  --elohim-portal-panel-radius: var(--el-radius-lg);
  --elohim-portal-grid-gap:     var(--el-space-md);
  --elohim-portal-padding:      var(--el-space-xl);
  --elohim-portal-footer-size:  0.8125rem;
`;

const PORTAL_TOKENS_DARK = `
  --elohim-portal-bg:           var(--el-night);
  --elohim-portal-fg:           var(--el-starlight);
  --elohim-portal-panel-bg:     #162019;
  --elohim-portal-panel-radius: var(--el-radius-lg);
  --elohim-portal-grid-gap:     var(--el-space-md);
  --elohim-portal-padding:      var(--el-space-xl);
  --elohim-portal-footer-size:  0.8125rem;
`;

// Error region base style (light)
const ERROR_REGION_LIGHT = `
  background: rgba(107, 97, 87, 0.06);
  border: 1px solid rgba(107, 97, 87, 0.2);
  border-radius: 8px;
  padding: 24px;
  max-inline-size: 480px;
`;

// Error region (dark)
const ERROR_REGION_DARK = `
  background: rgba(232, 228, 217, 0.04);
  border: 1px solid rgba(232, 228, 217, 0.12);
  border-radius: 8px;
  padding: 24px;
  max-inline-size: 480px;
`;

// ---------------------------------------------------------------------------
// Decorators
// ---------------------------------------------------------------------------

function lightDecorator(story: () => unknown) {
  return html`
    <div
      style="
        ${EL_TOKENS}
        ${PORTAL_TOKENS_LIGHT}
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
  title: 'Designed/Imagodei/PortalHostNotAuthorized',
  parameters: {
    docs: {
      description: {
        component: `
**This portal cannot reach your doorway.**

Spec §4.5 substrate-gate surface. Matthew entered \`matthew@beta.elohim.host\`
on alpha.elohim.host's portal. Beta.elohim.host has not authorized alpha to act
as a portal-host for its accounts.

The trust-indicator is intentionally HIDDEN — no authority has been resolved.
Alpha.elohim.host has no standing to claim authority over beta.elohim.host accounts.
Showing the trust-indicator here would misrepresent the protocol's authority model.

The error copy is informational: this is an inter-doorway authorization gap, not
a household error. The household is given a static recovery link so they can proceed
even without an interactive flow available.

Brand voice: calm, practical, not accusatory. The household is not at fault.
The protocol's infrastructure has an authorization boundary that has not been
crossed — this is expected behavior, not a bug.

Library B — primitive untouched; brand tokens bound via story decorator.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Default — portal-host not authorized
// ---------------------------------------------------------------------------

export const Default: Story = {
  name: 'Default (not authorized)',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-imagodei-portal-shell
      authority-endpoint="/nonexistent-url-story-safe"
    >
      <!-- Trust-indicator intentionally omitted: no authority resolved -->

      <p slot="primary" style="display: none;"></p>

      <div slot="error-region" role="alert" style="${ERROR_REGION_LIGHT}">
        <p style="
          font-family: var(--el-font-display);
          font-size: 1.1875rem;
          font-weight: 500;
          color: var(--el-night);
          margin-block: 0 var(--el-space-sm);
          line-height: 1.35;
        ">
          This portal cannot reach your doorway.
        </p>

        <p style="
          font-family: var(--el-font-body);
          font-size: 0.9375rem;
          color: var(--el-stone);
          margin-block: 0 var(--el-space-md);
          line-height: 1.65;
        ">
          You entered <strong>matthew&#64;beta.elohim.host</strong>, but
          beta.elohim.host has not authorized alpha.elohim.host to act as a
          portal host for its accounts.
        </p>

        <p style="
          font-family: var(--el-font-body);
          font-size: 0.9375rem;
          color: var(--el-stone);
          margin-block: 0 var(--el-space-md);
          line-height: 1.65;
        ">
          To sign in, visit beta.elohim.host directly, or ask your community
          steward how to reach your account.
        </p>

        <nav style="
          display: flex;
          flex-direction: column;
          gap: var(--el-space-xs);
        ">
          <a
            href="https://beta.elohim.host/auth"
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
          >Go to beta.elohim.host</a>

          <a
            href="/identity/recovery"
            style="
              font-family: var(--el-font-ui);
              font-size: 0.9375rem;
              color: var(--el-stone);
              text-decoration: underline;
              text-underline-offset: 2px;
            "
          >Visit identity recovery</a>
        </nav>
      </div>

      <span slot="footer">
        alpha.elohim.host &middot; commons stewardship &middot;
        <a href="/identity/help" style="color: inherit; opacity: 0.7;">help</a>
      </span>
    </elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Portal-host-not-authorized error surface — §4.5. No trust-indicator in the header ' +
          '(intentional). Alpha.elohim.host has no authority claim over beta.elohim.host ' +
          'accounts. The error region uses a quiet stone-tinted background — this is not a ' +
          'crisis, it is a boundary. The primary path ("Go to beta.elohim.host") redirects ' +
          'the household to the right portal. The secondary link ("Visit identity recovery") ' +
          'is a static fallback that works even without an interactive flow.',
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
  render: () => html`
    <elohim-imagodei-portal-shell
      authority-endpoint="/nonexistent-url-story-safe"
    >
      <p slot="primary" style="display: none;"></p>

      <div slot="error-region" role="alert" style="${ERROR_REGION_DARK}">
        <p style="
          font-family: var(--el-font-display);
          font-size: 1.1875rem;
          font-weight: 500;
          color: var(--el-starlight);
          margin-block: 0 var(--el-space-sm);
          line-height: 1.35;
        ">
          This portal cannot reach your doorway.
        </p>

        <p style="
          font-family: var(--el-font-body);
          font-size: 0.9375rem;
          color: var(--el-starlight);
          opacity: 0.75;
          margin-block: 0 var(--el-space-md);
          line-height: 1.65;
        ">
          You entered <strong>matthew&#64;beta.elohim.host</strong>, but
          beta.elohim.host has not authorized alpha.elohim.host to act as a
          portal host for its accounts.
        </p>

        <p style="
          font-family: var(--el-font-body);
          font-size: 0.9375rem;
          color: var(--el-starlight);
          opacity: 0.75;
          margin-block: 0 var(--el-space-md);
          line-height: 1.65;
        ">
          Visit beta.elohim.host directly, or ask your community steward.
        </p>

        <nav style="
          display: flex;
          flex-direction: column;
          gap: var(--el-space-xs);
        ">
          <a
            href="https://beta.elohim.host/auth"
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
          >Go to beta.elohim.host</a>

          <a
            href="/identity/recovery"
            style="
              font-family: var(--el-font-ui);
              font-size: 0.9375rem;
              color: var(--el-starlight);
              opacity: 0.6;
              text-decoration: underline;
              text-underline-offset: 2px;
            "
          >Visit identity recovery</a>
        </nav>
      </div>

      <span slot="footer">
        alpha.elohim.host &middot;
        <a href="/identity/help" style="color: inherit; opacity: 0.5;">help</a>
      </span>
    </elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'Dark constellation register. The error region is a barely-there translucent panel ' +
          'on Deep Sky — quiet, not dramatic. The absence of the trust-indicator is more ' +
          'noticeable in dark mode because the header region is visually empty. This is ' +
          'deliberate: "no authority resolved" should feel like a gap, not a fill.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// With identifier detail — shows what was entered
// ---------------------------------------------------------------------------

export const WithIdentifierDetail: Story = {
  name: 'WithIdentifierDetail',
  decorators: [lightDecorator],
  render: () => html`
    <elohim-imagodei-portal-shell
      authority-endpoint="/nonexistent-url-story-safe"
    >
      <p slot="primary" style="display: none;"></p>

      <div slot="error-region" role="alert" style="${ERROR_REGION_LIGHT}">
        <p style="
          font-family: var(--el-font-display);
          font-size: 1.1875rem;
          font-weight: 500;
          color: var(--el-night);
          margin-block: 0 var(--el-space-xs);
        ">
          This portal cannot reach your doorway.
        </p>

        <p style="
          font-family: var(--el-font-mono);
          font-size: 0.875rem;
          color: var(--el-clay);
          background: rgba(184, 102, 79, 0.07);
          border-radius: var(--el-radius-sm);
          padding: 0.25rem 0.5rem;
          display: inline-block;
          margin-block: var(--el-space-xs) var(--el-space-sm);
          letter-spacing: 0.04em;
        ">
          matthew&#64;beta.elohim.host
        </p>

        <p style="
          font-family: var(--el-font-body);
          font-size: 0.9375rem;
          color: var(--el-stone);
          margin-block: 0 var(--el-space-md);
          line-height: 1.65;
        ">
          beta.elohim.host has not authorized this portal to act on its behalf.
          Visit beta.elohim.host directly, or use a different identifier.
        </p>

        <nav style="display: flex; gap: var(--el-space-sm); flex-wrap: wrap;">
          <a
            href="https://beta.elohim.host/auth"
            style="
              font-family: var(--el-font-ui);
              font-size: 0.875rem;
              font-weight: 600;
              color: var(--el-linen);
              background: var(--el-green-deep);
              border-radius: var(--el-radius-md);
              padding: 0.5rem 1rem;
              text-decoration: none;
              display: inline-block;
            "
          >Go to beta.elohim.host</a>

          <a
            href="/auth"
            style="
              font-family: var(--el-font-ui);
              font-size: 0.875rem;
              color: var(--el-stone);
              border: 1px solid rgba(107, 97, 87, 0.35);
              border-radius: var(--el-radius-md);
              padding: 0.5rem 1rem;
              text-decoration: none;
              display: inline-block;
            "
          >Try a different identifier</a>

          <a
            href="/identity/recovery"
            style="
              font-family: var(--el-font-ui);
              font-size: 0.875rem;
              color: var(--el-stone);
              text-decoration: underline;
              text-underline-offset: 2px;
              align-self: center;
            "
          >identity recovery</a>
        </nav>
      </div>

      <span slot="footer">
        alpha.elohim.host &middot;
        <a href="/identity/help" style="color: inherit; opacity: 0.7;">help</a>
      </span>
    </elohim-imagodei-portal-shell>
  `,
  parameters: {
    docs: {
      description: {
        story:
          'With identifier detail: the entered identifier is echoed in a monospaced clay-tinted ' +
          'badge — JetBrains Mono in Terracotta (#B8664F) on a warm clay wash. Three affordances: ' +
          '"Go to beta.elohim.host" (primary), "Try a different identifier" (reset), and ' +
          '"identity recovery" (fallback). This variant is appropriate when the error surface ' +
          'can safely echo back what was entered.',
      },
    },
  },
};
