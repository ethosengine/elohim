import { expect, fixture, html, oneEvent } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimImagodeiPortalShell as ElohimImagodeiPortalShellClass } from './elohim-imagodei-portal-shell.js';
import type { ElohimImagodeiPortalShell } from './elohim-imagodei-portal-shell.js';

// ---------------------------------------------------------------------------
// trustMode discovery via authorityEndpoint
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-portal-shell> — trustMode discovery', () => {
  let originalFetch: typeof window.fetch;

  beforeEach(() => {
    originalFetch = window.fetch;
  });

  afterEach(() => {
    (window as any).fetch = originalFetch;
  });

  it('discovers trustMode from the authorityEndpoint mock', async () => {
    (window as any).fetch = async () =>
      new Response(
        JSON.stringify({
          authenticated: false,
          trustMode: 'doorway-host',
          authority: { label: 'alpha.elohim.host' },
        }),
        { status: 200, headers: { 'content-type': 'application/json' } }
      );

    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell
        authority-endpoint="/auth/me"
      ></elohim-imagodei-portal-shell>
    `);
    await oneEvent(el, 'authority-resolved');
    await el.updateComplete;

    expect(
      el.shadowRoot!.querySelector('elohim-imagodei-trust-indicator')?.getAttribute('trust-mode')
    ).to.equal('doorway-host');
  });

  it('discovers peer-conductor trustMode correctly', async () => {
    (window as any).fetch = async () =>
      new Response(
        JSON.stringify({
          authenticated: true,
          trustMode: 'peer-conductor',
          authority: { label: 'your-local-conductor' },
        }),
        { status: 200, headers: { 'content-type': 'application/json' } }
      );

    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell
        authority-endpoint="/auth/me"
      ></elohim-imagodei-portal-shell>
    `);
    await oneEvent(el, 'authority-resolved');
    await el.updateComplete;

    expect(
      el.shadowRoot!.querySelector('elohim-imagodei-trust-indicator')?.getAttribute('trust-mode')
    ).to.equal('peer-conductor');
  });

  it('survives a failed fetch without throwing (resilient default)', async () => {
    (window as any).fetch = async () => {
      throw new Error('network error');
    };

    // Should not throw; renders with fallback defaults
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell
        authority-endpoint="/auth/me"
      ></elohim-imagodei-portal-shell>
    `);
    await el.updateComplete;
    expect(el).to.exist;
    // Default trust mode is doorway-host
    const indicator = el.shadowRoot!.querySelector('elohim-imagodei-trust-indicator');
    expect(indicator?.getAttribute('trust-mode')).to.equal('doorway-host');
  });

  it('survives a non-OK HTTP response without throwing', async () => {
    (window as any).fetch = async () =>
      new Response(null, { status: 500 });

    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell
        authority-endpoint="/auth/me"
      ></elohim-imagodei-portal-shell>
    `);
    await el.updateComplete;
    expect(el).to.exist;
  });

  it('emits authority-resolved with the parsed resolution detail', async () => {
    (window as any).fetch = async () =>
      new Response(
        JSON.stringify({
          trustMode: 'doorway-host',
          authority: { label: 'alpha.elohim.host', id: 'host-1' },
          flywheelHint: true,
        }),
        { status: 200, headers: { 'content-type': 'application/json' } }
      );

    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell
        authority-endpoint="/auth/me"
      ></elohim-imagodei-portal-shell>
    `);
    const ev = (await oneEvent(el, 'authority-resolved')) as CustomEvent;
    expect(ev.detail.trustMode).to.equal('doorway-host');
    expect(ev.detail.authority.label).to.equal('alpha.elohim.host');
    expect(ev.detail.flywheelHint).to.be.true;
  });
});

// ---------------------------------------------------------------------------
// Primary slot + step management
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-portal-shell> — slot rendering', () => {
  it('renders the primary slot for the active step', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="login">
        <div slot="primary" id="step-content">login-card placeholder</div>
      </elohim-imagodei-portal-shell>
    `);
    const slot = el.shadowRoot!.querySelector('slot[name="primary"]') as HTMLSlotElement;
    expect(slot).to.exist;
    const assigned = slot.assignedElements();
    expect(assigned.some(n => (n as HTMLElement).id === 'step-content')).to.equal(true);
  });

  it('renders error-region slot when content provided', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="login">
        <div slot="primary" id="child"></div>
        <div slot="error-region" id="errors">errors here</div>
      </elohim-imagodei-portal-shell>
    `);
    const errSlot = el.shadowRoot!.querySelector('slot[name="error-region"]') as HTMLSlotElement;
    expect(errSlot).to.exist;
    expect(errSlot.assignedElements().some(n => (n as HTMLElement).id === 'errors')).to.equal(true);
  });

  it('renders the footer slot when content is provided', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="login">
        <span slot="footer" id="footer-text">Terms apply</span>
      </elohim-imagodei-portal-shell>
    `);
    const footerSlot = el.shadowRoot!.querySelector('slot[name="footer"]') as HTMLSlotElement;
    expect(footerSlot).to.exist;
    expect(footerSlot.assignedElements().some(n => (n as HTMLElement).id === 'footer-text')).to.equal(
      true
    );
  });

  it('has a default header slot that renders trust-indicator + attestor-row', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell></elohim-imagodei-portal-shell>
    `);
    await el.updateComplete;
    // Default header slot shows trust-indicator and attestor-row in shadow DOM
    expect(el.shadowRoot!.querySelector('elohim-imagodei-trust-indicator')).to.exist;
    expect(el.shadowRoot!.querySelector('elohim-imagodei-attestor-row')).to.exist;
  });

  it('consumer can override header slot', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell>
        <div slot="header" id="custom-header">My brand</div>
      </elohim-imagodei-portal-shell>
    `);
    const headerSlot = el.shadowRoot!.querySelector('slot[name="header"]') as HTMLSlotElement;
    expect(headerSlot.assignedElements().some(n => (n as HTMLElement).id === 'custom-header')).to.equal(
      true
    );
  });
});

// ---------------------------------------------------------------------------
// No-auto-advance contract
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-portal-shell> — step management (no auto-advance)', () => {
  it('does NOT auto-advance step; only consumer setting shell.step changes it', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="resolve"></elohim-imagodei-portal-shell>
    `);
    expect(el.step).to.equal('resolve');

    // Emit a child success event — shell must ignore it
    el.dispatchEvent(new CustomEvent('resolved', { detail: {}, bubbles: true }));
    await el.updateComplete;
    expect(el.step).to.equal('resolve');

    // Consumer sets the step explicitly
    el.step = 'login';
    await el.updateComplete;
    expect(el.step).to.equal('login');
  });

  it('emits step-change event when consumer sets a new step', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="resolve"></elohim-imagodei-portal-shell>
    `);

    let stepChangeDetail: unknown = null;
    el.addEventListener('step-change', e => {
      stepChangeDetail = (e as CustomEvent).detail;
    });

    el.step = 'consent';
    await el.updateComplete;
    // step-change fires on `updated` after the property changes
    await el.updateComplete;
    expect((stepChangeDetail as any)?.step).to.equal('consent');
  });

  it('defaults to step="resolve"', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell></elohim-imagodei-portal-shell>
    `);
    expect(el.step).to.equal('resolve');
  });

  it('accepts all valid PortalStep values', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="callback"></elohim-imagodei-portal-shell>
    `);
    expect(el.step).to.equal('callback');
    el.step = 'consent';
    await el.updateComplete;
    expect(el.step).to.equal('consent');
  });
});

// ---------------------------------------------------------------------------
// Context propagation to slotted children
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-portal-shell> — context propagation', () => {
  it('propagates trustMode + authority to slotted children via property assignment', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="login">
        <div slot="primary" id="child"></div>
      </elohim-imagodei-portal-shell>
    `);

    (el as any)._setAuthority({
      trustMode: 'peer-conductor',
      authority: { label: 'your conductor' },
    });
    await el.updateComplete;

    const child = el.querySelector('#child') as any;
    expect(child.trustMode).to.equal('peer-conductor');
    expect(child.authority).to.deep.equal({ label: 'your conductor' });
  });

  it('propagates updated authority when _setAuthority called again', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="login">
        <div slot="primary" id="child"></div>
      </elohim-imagodei-portal-shell>
    `);

    (el as any)._setAuthority({
      trustMode: 'doorway-host',
      authority: { label: 'first authority' },
    });
    await el.updateComplete;

    (el as any)._setAuthority({
      trustMode: 'peer-conductor',
      authority: { label: 'updated authority' },
    });
    await el.updateComplete;

    const child = el.querySelector('#child') as any;
    expect(child.trustMode).to.equal('peer-conductor');
    expect(child.authority).to.deep.equal({ label: 'updated authority' });
  });
});

// ---------------------------------------------------------------------------
// a11y precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-portal-shell> — a11y precondition gate', () => {
  it('passes axe accessibility audit in default (resolve) state', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="resolve"></elohim-imagodei-portal-shell>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe accessibility audit with primary slot content', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="login">
        <div slot="primary" role="region" aria-label="Login step">
          <button type="button">Sign in</button>
        </div>
      </elohim-imagodei-portal-shell>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe accessibility audit with error-region content', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="login">
        <div slot="primary"><button type="button">Go</button></div>
        <div slot="error-region" role="alert">Something went wrong</div>
      </elohim-imagodei-portal-shell>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

// ---------------------------------------------------------------------------
// ua-prefs precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-portal-shell> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions and animations (still by default)', () => {
    const cssText = (
      ElohimImagodeiPortalShellClass as unknown as { styles: { cssText: string } }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
    expect(cssText).to.not.contain('animation:');
  });

  it('CSS has a forced-colors override block', () => {
    const cssText = (
      ElohimImagodeiPortalShellClass as unknown as { styles: { cssText: string } }
    ).styles.cssText;
    expect(cssText).to.contain('forced-colors');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="resolve"></elohim-imagodei-portal-shell>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

// ---------------------------------------------------------------------------
// i18n precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-portal-shell> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimImagodeiPortalShell>(
      'he-IL',
      html`
        <elohim-imagodei-portal-shell step="resolve">
          <div slot="primary">step content</div>
        </elohim-imagodei-portal-shell>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const frame = el.shadowRoot!.querySelector('.frame')!;
    const rect = frame.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimImagodeiPortalShellClass as unknown as { styles: { cssText: string } }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
