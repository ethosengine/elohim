import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import type { ElohimNavigator } from './elohim-navigator.js';
import { ElohimNavigator as NavigatorClass } from './elohim-navigator.js';
import { requiresLogicalProperties } from './testing/i18n.js';

describe('<elohim-navigator>', () => {
  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-navigator')).to.equal(NavigatorClass);
  });

  it('renders the nav element with role navigation', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator></elohim-navigator>`,
    );
    const nav = el.shadowRoot?.querySelector('nav[aria-label]');
    expect(nav).to.exist;
  });

  it('renders context switcher button', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator></elohim-navigator>`,
    );
    const btn = el.shadowRoot?.querySelector('[part="context-switcher-btn"]');
    expect(btn).to.exist;
  });

  it('renders profile bubble button', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator></elohim-navigator>`,
    );
    const bubble = el.shadowRoot?.querySelector('[data-testid="profile-bubble"]');
    expect(bubble).to.exist;
  });

  it('shows the active context app name in the switcher', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator context="shefa"></elohim-navigator>`,
    );
    const switcher = el.shadowRoot?.querySelector('[part="context-switcher-btn"]');
    expect(switcher?.textContent).to.contain('Shefa');
  });

  it('opens context switcher dropdown on click', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator></elohim-navigator>`,
    );
    const switcherBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[part="context-switcher-btn"]',
    );
    switcherBtn?.click();
    await elementUpdated(el);

    const tray = el.shadowRoot?.querySelector('[part="context-tray"]');
    expect(tray).to.exist;
  });

  it('dispatches navigate event when a context app is selected', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator></elohim-navigator>`,
    );

    let navigatedRoute: string | null = null;
    el.addEventListener('navigate', (e: Event) => {
      navigatedRoute = (e as CustomEvent<{ route: string }>).detail.route;
    });

    const switcherBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[part="context-switcher-btn"]',
    );
    switcherBtn?.click();
    await elementUpdated(el);

    const shefaApp = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="context-app-shefa"]',
    );
    shefaApp?.click();

    expect(navigatedRoute).to.equal('/shefa');
  });

  it('calls onNavigate callback instead of dispatching event when set', async () => {
    let callbackRoute: string | null = null;
    const onNavigate = (route: string) => { callbackRoute = route; };

    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator .onNavigate=${onNavigate}></elohim-navigator>`,
    );

    const switcherBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[part="context-switcher-btn"]',
    );
    switcherBtn?.click();
    await elementUpdated(el);

    const shefaApp = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="context-app-shefa"]',
    );
    shefaApp?.click();

    expect(callbackRoute).to.equal('/shefa');
  });

  it('renders search input when showSearch=true', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator show-search></elohim-navigator>`,
    );
    const searchInput = el.shadowRoot?.querySelector('[part="search-input"]');
    expect(searchInput).to.exist;
  });

  it('hides search input when showSearch=false', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator .showSearch=${false}></elohim-navigator>`,
    );
    const searchInput = el.shadowRoot?.querySelector('[part="search-input"]');
    expect(searchInput).to.be.null;
  });

  it('shows profile tray when profile bubble is clicked', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator .isAuthenticated=${true} display-name="Alice"></elohim-navigator>`,
    );

    const bubble = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="profile-bubble"]',
    );
    bubble?.click();
    await elementUpdated(el);

    const tray = el.shadowRoot?.querySelector('[part="profile-tray"]');
    expect(tray).to.exist;
  });

  it('shows sign-in menu items when not authenticated', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator .isAuthenticated=${false}></elohim-navigator>`,
    );

    const bubble = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="profile-bubble"]',
    );
    bubble?.click();
    await elementUpdated(el);

    const loginBtn = el.shadowRoot?.querySelector('[data-testid="nav-login"]');
    expect(loginBtn).to.exist;
  });

  it('dispatches logout event when logout is triggered', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator .isAuthenticated=${true}></elohim-navigator>`,
    );

    let loggedOut = false;
    el.addEventListener('logout', () => { loggedOut = true; });

    const bubble = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="profile-bubble"]',
    );
    bubble?.click();
    await elementUpdated(el);

    const logoutBtn = el.shadowRoot?.querySelector<HTMLButtonElement>('[data-testid="nav-logout"]');
    logoutBtn?.click();

    expect(loggedOut).to.be.true;
  });

  it('renders banner notices', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator
        .banners=${[{ id: 'b1', message: 'System notice', severity: 'info' }]}
      ></elohim-navigator>`,
    );
    const banner = el.shadowRoot?.querySelector('[data-testid="banner-b1"]');
    expect(banner?.textContent).to.contain('System notice');
  });

  it('dispatches banner-dismiss event on dismiss click', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator
        .banners=${[{ id: 'b1', message: 'Notice', severity: 'info' }]}
      ></elohim-navigator>`,
    );

    let dismissedId: string | null = null;
    el.addEventListener('banner-dismiss', (e: Event) => {
      dismissedId = (e as CustomEvent<{ id: string }>).detail.id;
    });

    const dismissBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="banner-b1"] button',
    );
    dismissBtn?.click();

    expect(dismissedId).to.equal('b1');
  });

  it('passes axe-core a11y scan', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator></elohim-navigator>`,
    );
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-navigator> — i18n gate', () => {
  it('uses only logical CSS properties', () => {
    const cssText = (NavigatorClass as { styles: { cssText: string } }).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-navigator> — ua-prefs gate', () => {
  it('forced-colors overrides present', () => {
    const cssText = (NavigatorClass as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.contain('forced-colors: active');
  });

  it('touch targets meet 44px minimum for interactive elements', () => {
    const cssText = (NavigatorClass as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.contain('44px');
  });
});
