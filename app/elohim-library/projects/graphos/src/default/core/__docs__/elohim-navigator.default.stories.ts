/**
 * Library A default story — elohim-navigator
 *
 * Proves the element works as a blank-slate primitive: no brand tokens bound,
 * CSS system colors as defaults, override surface honest.
 *
 * <elohim-navigator> is the protocol's application navigation bar.
 * Includes: context switcher, search (optional), profile tray, banners.
 * All navigation is callback/event-driven — no direct routing.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';
import type { ElohimContextAppConfig, ElohimNavigatorBannerItem } from 'elohim-core';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/** Host-supplied context apps (§12.3: routing lives in the host, not the primitive). */
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

const INFO_BANNER: ElohimNavigatorBannerItem = {
  id: 'system-notice-1',
  message: 'Protocol maintenance window tonight at 23:00 UTC.',
  severity: 'info',
};

const WARNING_BANNER: ElohimNavigatorBannerItem = {
  id: 'upgrade-notice-1',
  message: 'A new version of the protocol is available. Please restart your node.',
  severity: 'warning',
};

// ---------------------------------------------------------------------------
// Meta
// ---------------------------------------------------------------------------

const meta: Meta = {
  title: 'Default/Core/elohim-navigator',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-navigator>\` — the protocol's application navigation bar.

Includes:
- Context switcher (dropdown) — entries are **host-supplied** via \`.contextApps\` (§12.3: no routing baked into the primitive). Empty → switcher hidden.
- Identity tray routes — **host-supplied** via \`.identityRoutes\`. Absent entries render no tray item.
- Optional search input (\`show-search\`)
- Profile tray (authenticated or sign-in)
- Banner notices at top

All navigation dispatches \`navigate\`, search dispatches \`search\`,
logout dispatches \`logout\`, banner dismiss dispatches \`banner-dismiss\`.
All support equivalent callback props.

**Override surface:** \`--elohim-nav-*\`.
**Parts:** \`nav-bar\`, \`context-switcher-btn\`, \`context-tray\`, \`search-input\`, \`profile-bubble\`, \`profile-tray\`, \`banner\`.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

// ---------------------------------------------------------------------------
// Authenticated / unauthenticated
// ---------------------------------------------------------------------------

export const Authenticated: Story = {
  name: 'Authenticated',
  render: () => html`
    <elohim-navigator
      context="lamad"
      .contextApps=${CONTEXT_APPS}
      .identityRoutes=${IDENTITY_ROUTES}
      .isAuthenticated=${true}
      display-name="Alice"
      identifier="alice.pilot"
      identity-mode="hosted"
    ></elohim-navigator>
  `,
};

export const Unauthenticated: Story = {
  name: 'Unauthenticated',
  render: () => html`
    <elohim-navigator
      context="lamad"
      .contextApps=${CONTEXT_APPS}
      .identityRoutes=${IDENTITY_ROUTES}
      .isAuthenticated=${false}
    ></elohim-navigator>
  `,
  parameters: {
    docs: {
      description: { story: 'No session — profile bubble shows sign-in options when opened.' },
    },
  },
};

// ---------------------------------------------------------------------------
// Context variants
// ---------------------------------------------------------------------------

export const ShefaContext: Story = {
  name: 'ShefaContext (active)',
  render: () => html`
    <elohim-navigator
      context="shefa"
      .contextApps=${CONTEXT_APPS}
      .identityRoutes=${IDENTITY_ROUTES}
      .isAuthenticated=${true}
      display-name="Bob"
    ></elohim-navigator>
  `,
};

export const WithSearch: Story = {
  name: 'WithSearch',
  render: () => html`
    <elohim-navigator
      context="lamad"
      show-search
      .contextApps=${CONTEXT_APPS}
      .identityRoutes=${IDENTITY_ROUTES}
      .isAuthenticated=${true}
      display-name="Carol"
    ></elohim-navigator>
  `,
};

// ---------------------------------------------------------------------------
// Banner notices
// ---------------------------------------------------------------------------

export const WithInfoBanner: Story = {
  name: 'WithInfoBanner',
  render: () => html`
    <elohim-navigator
      context="lamad"
      .contextApps=${CONTEXT_APPS}
      .banners=${[INFO_BANNER]}
    ></elohim-navigator>
  `,
};

export const WithWarningBanner: Story = {
  name: 'WithWarningBanner',
  render: () => html`
    <elohim-navigator
      context="lamad"
      .contextApps=${CONTEXT_APPS}
      .banners=${[WARNING_BANNER]}
    ></elohim-navigator>
  `,
};

export const WithMultipleBanners: Story = {
  name: 'WithMultipleBanners',
  render: () => html`
    <elohim-navigator
      context="lamad"
      .contextApps=${CONTEXT_APPS}
      .banners=${[INFO_BANNER, WARNING_BANNER]}
    ></elohim-navigator>
  `,
};

// ---------------------------------------------------------------------------
// Interactive
// ---------------------------------------------------------------------------

export const Interactive: Story = {
  name: 'Interactive (callbacks)',
  render: () => {
    const onNavigate = (route: string) =>
      // eslint-disable-next-line no-console
      console.log('navigate:', route);
    const onLogout = () =>
      // eslint-disable-next-line no-console
      console.log('logout');
    const onBannerDismiss = (id: string) =>
      // eslint-disable-next-line no-console
      console.log('banner-dismiss:', id);
    return html`
      <elohim-navigator
        context="lamad"
        show-search
        .contextApps=${CONTEXT_APPS}
        .identityRoutes=${IDENTITY_ROUTES}
        .isAuthenticated=${true}
        display-name="Dave"
        .banners=${[INFO_BANNER]}
        .onNavigate=${onNavigate}
        .onLogout=${onLogout}
        .onBannerDismiss=${onBannerDismiss}
      ></elohim-navigator>
    `;
  },
  parameters: {
    docs: {
      description: {
        story:
          'All callbacks wired — navigate, logout, banner-dismiss events are logged to the console.',
      },
    },
  },
};

// ---------------------------------------------------------------------------
// Theme canaries
// ---------------------------------------------------------------------------

export const Dark: Story = {
  name: 'Dark',
  decorators: [
    story => html`
      <div style="background: #0d1117; color-scheme: dark;">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-navigator
      context="lamad"
      .contextApps=${CONTEXT_APPS}
      .identityRoutes=${IDENTITY_ROUTES}
      .isAuthenticated=${true}
      display-name="Alice"
    ></elohim-navigator>
  `,
};

export const RTLCanary: Story = {
  name: 'RTLCanary',
  decorators: [
    story => html`
      <div dir="rtl" lang="he">
        ${story()}
      </div>
    `,
  ],
  render: () => html`
    <elohim-navigator
      context="lamad"
      .contextApps=${CONTEXT_APPS}
      .isAuthenticated=${true}
      display-name="עלי"
    ></elohim-navigator>
  `,
};

// ---------------------------------------------------------------------------
// Blank-slate and override-surface proofs
// ---------------------------------------------------------------------------

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial; display: block;">${story()}</div>`],
  render: () => html`<elohim-navigator context="lamad" .contextApps=${CONTEXT_APPS}></elohim-navigator>`,
  parameters: {
    docs: {
      description: {
        story: 'Wrapped in `style="all: initial;"`. Proves the nav renders with zero external tokens.',
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
          --elohim-nav-bg: #0f172a;
          --elohim-nav-fg: #e2e8f0;
          --elohim-nav-border: 1px solid #334155;
          --elohim-nav-bubble-bg: #1e293b;
          --elohim-nav-bubble-fg: #f1f5f9;
          --elohim-nav-banner-info-bg: #1e3a8a;
          --elohim-nav-banner-info-fg: #dbeafe;
          --elohim-nav-tray-shadow: 0 4px 12px rgba(2, 6, 23, 0.6);
          font-family: ui-sans-serif, sans-serif;
        "
      >
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
      display-name="Eve"
      .banners=${[INFO_BANNER]}
    ></elohim-navigator>
  `,
  parameters: {
    docs: {
      description: {
        story: 'Deliberately non-Elohim slate-blue theme. Proves the override surface is honest.',
      },
    },
  },
};
