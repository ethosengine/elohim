/**
 * Library B designed story — elohim-navigator
 *
 * Binds the Elohim brand tokens to `<elohim-navigator>` via story-decorator
 * overrides. The primitive itself is NEVER modified — binding happens via
 * `--elohim-nav-*` CSS custom properties mapped to `--el-*`.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';
import type { ElohimContextAppConfig } from 'elohim-core';

const CONTEXT_APPS: ElohimContextAppConfig[] = [
  { id: 'lamad', name: 'Lamad', icon: '📚', route: '/lamad', tagline: 'Learning & Content', available: true },
  { id: 'community', name: 'Qahal', icon: '👥', route: '/community', tagline: 'Community & Governance', available: true },
  { id: 'shefa', name: 'Shefa', icon: '✨', route: '/shefa', tagline: 'Economics of Flourishing', available: true },
  { id: 'avodah', name: 'Avodah', icon: '🔨', route: '/avodah', tagline: 'Work & Stewardship', available: true },
  { id: 'map', name: 'Map', icon: '🌍', route: '/map', tagline: 'Living Places', available: true },
];

const IDENTITY_ROUTES = {
  profile: '/identity/profile',
  login: '/identity/login',
  register: '/identity/register',
};

// ---------------------------------------------------------------------------
// Brand tokens
// ---------------------------------------------------------------------------

const EL_TOKENS = `
  --el-green-deep:  #2D5F3B;
  --el-green-light: #7FB069;
  --el-amber:       #D4A03E;
  --el-cream:       #F5F0E8;
  --el-stone:       #6B6157;
  --el-starlight:   #E8E4D9;
  --el-night:       #0F1A12;
  --el-night-alt:   #1A1A2E;
  --el-font-ui:     'DM Sans', system-ui, sans-serif;
  --el-font-display: 'Fraunces', Georgia, serif;
  --el-radius-sm:   4px;
  --el-shadow-soft: 0 2px 8px rgba(107, 97, 87, 0.08);
`;

const NAV_TOKENS_LIGHT = `
  --elohim-nav-bg:              var(--el-cream);
  --elohim-nav-fg:              var(--el-stone);
  --elohim-nav-border:          1px solid color-mix(in oklch, var(--el-stone) 15%, transparent);
  --elohim-nav-shadow:          var(--el-shadow-soft);
  --elohim-nav-switcher-bg:     color-mix(in oklch, var(--el-stone) 6%, var(--el-cream));
  --elohim-nav-switcher-fg:     var(--el-green-deep);
  --elohim-nav-profile-bg:      var(--el-green-deep);
  --elohim-nav-profile-fg:      var(--el-cream);
  --elohim-nav-banner-bg:       color-mix(in oklch, var(--el-amber) 15%, var(--el-cream));
  --elohim-nav-banner-fg:       var(--el-stone);
  --elohim-nav-tray-bg:         var(--el-cream);
  --elohim-nav-tray-border:     1px solid color-mix(in oklch, var(--el-stone) 15%, transparent);
`;

const NAV_TOKENS_DARK = `
  --elohim-nav-bg:              var(--el-night);
  --elohim-nav-fg:              var(--el-starlight);
  --elohim-nav-border:          1px solid color-mix(in oklch, var(--el-starlight) 12%, transparent);
  --elohim-nav-shadow:          0 2px 8px rgba(0, 0, 0, 0.3);
  --elohim-nav-switcher-bg:     color-mix(in oklch, var(--el-starlight) 8%, var(--el-night));
  --elohim-nav-switcher-fg:     var(--el-green-light);
  --elohim-nav-profile-bg:      var(--el-green-deep);
  --elohim-nav-profile-fg:      var(--el-cream);
  --elohim-nav-banner-bg:       color-mix(in oklch, var(--el-amber) 20%, var(--el-night));
  --elohim-nav-banner-fg:       var(--el-starlight);
  --elohim-nav-tray-bg:         var(--el-night-alt);
  --elohim-nav-tray-border:     1px solid color-mix(in oklch, var(--el-starlight) 12%, transparent);
`;

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Designed/Core/elohim-navigator',
  parameters: {
    docs: {
      description: {
        component:
          'Library B — brand-bound view of `<elohim-navigator>`. ' +
          'Tokens bound via decorator; primitive not modified.',
      },
    },
  },
};

export default meta;
type Story = StoryObj;

export const Light: Story = {
  name: 'Light (Linen) — Authenticated',
  decorators: [
    story => html`
      <div style="${EL_TOKENS}${NAV_TOKENS_LIGHT}font-family: var(--el-font-ui);">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-navigator
      context="lamad"
      show-search
      .contextApps=${CONTEXT_APPS}
      .identityRoutes=${IDENTITY_ROUTES}
      .isAuthenticated=${true}
      display-name="Alice"
      identifier="alice.pilot"
    ></elohim-navigator>
  `,
};

export const Dark: Story = {
  name: 'Dark (Night) — Authenticated',
  decorators: [
    story => html`
      <div style="${EL_TOKENS}${NAV_TOKENS_DARK}font-family: var(--el-font-ui); color-scheme: dark;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-navigator
      context="shefa"
      show-search
      .contextApps=${CONTEXT_APPS}
      .identityRoutes=${IDENTITY_ROUTES}
      .isAuthenticated=${true}
      display-name="Bob"
      identifier="bob.steward"
    ></elohim-navigator>
  `,
};

export const LightUnauthenticated: Story = {
  name: 'Light (Linen) — Unauthenticated',
  decorators: [
    story => html`
      <div style="${EL_TOKENS}${NAV_TOKENS_LIGHT}font-family: var(--el-font-ui);">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-navigator
      context="lamad"
      .contextApps=${CONTEXT_APPS}
      .identityRoutes=${IDENTITY_ROUTES}
      .isAuthenticated=${false}
    ></elohim-navigator>
  `,
};
