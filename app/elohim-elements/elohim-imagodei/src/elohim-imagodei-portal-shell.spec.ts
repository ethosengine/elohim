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
import type {
  ElohimImagodeiPortalShell,
  AuthorityResolution,
} from './elohim-imagodei-portal-shell.js';

// ---------------------------------------------------------------------------
// Fixture authority resolutions (no invented identity — labels from doorway)
// ---------------------------------------------------------------------------

const doorwayAuthority: AuthorityResolution = {
  trustMode: 'doorway-host',
  authority: { label: 'alpha.elohim.host', id: 'alpha' },
  flywheelHint: true,
  attestors: [
    { eprRef: 'epr:attestor:susan-elder', displayName: 'Susan Miller', role: 'qahal-elder' },
  ],
};

const peerAuthority: AuthorityResolution = {
  trustMode: 'peer-conductor',
  authority: { label: 'your conductor on this device' },
  flywheelHint: false,
};

// ---------------------------------------------------------------------------
// authority property — host-pre-fetch contract
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-portal-shell> — authority property (host-pre-fetch contract)', () => {
  it('reflects doorway-host trustMode when host provides authority before first render', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell .authority=${doorwayAuthority}></elohim-imagodei-portal-shell>
    `);
    await el.updateComplete;

    expect(
      el.shadowRoot!.querySelector('elohim-imagodei-trust-indicator')?.getAttribute('trust-mode')
    ).to.equal('doorway-host');
  });

  it('reflects peer-conductor trustMode when host provides peer authority', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell .authority=${peerAuthority}></elohim-imagodei-portal-shell>
    `);
    await el.updateComplete;

    expect(
      el.shadowRoot!.querySelector('elohim-imagodei-trust-indicator')?.getAttribute('trust-mode')
    ).to.equal('peer-conductor');
  });

  it('renders with placeholder chrome when authority is null (loading state)', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell></elohim-imagodei-portal-shell>
    `);
    await el.updateComplete;
    // Renders without error; default trust-mode placeholder shown
    expect(el.shadowRoot!.querySelector('elohim-imagodei-trust-indicator')).to.exist;
  });

  it('emits authority-needed when authority is null on first render', async () => {
    // Re-trigger by creating fresh element in a wrapper div we can capture events from
    const wrapper = document.createElement('div');
    document.body.appendChild(wrapper);
    let neededFired = false;
    wrapper.addEventListener(
      'authority-needed',
      () => {
        neededFired = true;
      },
      true
    );
    const fresh = await fixture<ElohimImagodeiPortalShell>(
      html`
        <elohim-imagodei-portal-shell></elohim-imagodei-portal-shell>
      `,
      { parentNode: wrapper }
    );
    await fresh.updateComplete;
    wrapper.remove();
    // authority-needed fires in firstUpdated which is synchronous after fixture resolves
    expect(neededFired).to.be.true;
  });

  it('does NOT emit authority-needed when authority is provided', async () => {
    let fired = false;
    const wrapper = document.createElement('div');
    document.body.appendChild(wrapper);
    wrapper.addEventListener(
      'authority-needed',
      () => {
        fired = true;
      },
      true
    );
    const el = await fixture<ElohimImagodeiPortalShell>(
      html`
        <elohim-imagodei-portal-shell .authority=${doorwayAuthority}></elohim-imagodei-portal-shell>
      `,
      { parentNode: wrapper }
    );
    await el.updateComplete;
    wrapper.remove();
    expect(fired).to.be.false;
  });

  it('emits authority-resolved when authority property is set', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell></elohim-imagodei-portal-shell>
    `);

    const resolved = oneEvent(el, 'authority-resolved');
    el.authority = doorwayAuthority;
    const ev = (await resolved) as CustomEvent<AuthorityResolution>;
    expect(ev.detail.trustMode).to.equal('doorway-host');
    expect(ev.detail.authority.label).to.equal('alpha.elohim.host');
    expect(ev.detail.flywheelHint).to.be.true;
  });

  it('updates rendered trust-indicator when authority property is changed after mount', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell .authority=${doorwayAuthority}></elohim-imagodei-portal-shell>
    `);
    await el.updateComplete;

    el.authority = peerAuthority;
    await el.updateComplete;

    expect(
      el.shadowRoot!.querySelector('elohim-imagodei-trust-indicator')?.getAttribute('trust-mode')
    ).to.equal('peer-conductor');
  });

  it('element contains no fetch, XMLHttpRequest, or other network calls', () => {
    // Structural contract: the element source does not contain network calls.
    // This is the primary acceptance gate for M-ELEM-1.
    // If this test compiles and the element still works, the class does not own fetching.
    const proto = ElohimImagodeiPortalShellClass.prototype as Record<string, unknown>;
    const methodNames = Object.getOwnPropertyNames(proto).filter(k => k !== 'constructor');
    for (const name of methodNames) {
      const fn = proto[name];
      if (typeof fn === 'function') {
        const src = fn.toString();
        expect(src, `method "${name}" must not call fetch()`).to.not.contain('fetch(');
        expect(src, `method "${name}" must not use XMLHttpRequest`).to.not.contain(
          'XMLHttpRequest'
        );
      }
    }
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
    expect(
      footerSlot.assignedElements().some(n => (n as HTMLElement).id === 'footer-text')
    ).to.equal(true);
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
    expect(
      headerSlot.assignedElements().some(n => (n as HTMLElement).id === 'custom-header')
    ).to.equal(true);
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
  it('propagates trustMode + authority to slotted children when authority property is set', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="login">
        <div slot="primary" id="child"></div>
      </elohim-imagodei-portal-shell>
    `);

    el.authority = peerAuthority;
    await el.updateComplete;

    const child = el.querySelector('#child') as any;
    expect(child.trustMode).to.equal('peer-conductor');
    expect(child.authority).to.deep.equal({ label: 'your conductor on this device' });
  });

  it('propagates updated authority when authority property is changed again', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="login">
        <div slot="primary" id="child"></div>
      </elohim-imagodei-portal-shell>
    `);

    el.authority = doorwayAuthority;
    await el.updateComplete;

    el.authority = peerAuthority;
    await el.updateComplete;

    const child = el.querySelector('#child') as any;
    expect(child.trustMode).to.equal('peer-conductor');
    expect(child.authority).to.deep.equal({ label: 'your conductor on this device' });
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

  it('passes axe accessibility audit when authority is provided', async () => {
    const el = await fixture<ElohimImagodeiPortalShell>(html`
      <elohim-imagodei-portal-shell step="resolve" .authority=${doorwayAuthority}>
        <div slot="primary" role="region" aria-label="Resolve step">
          <button type="button">Continue</button>
        </div>
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
    const cssText = (ElohimImagodeiPortalShellClass as unknown as { styles: { cssText: string } })
      .styles.cssText;
    expect(cssText).to.not.contain('transition:');
    expect(cssText).to.not.contain('animation:');
  });

  it('CSS has a forced-colors override block', () => {
    const cssText = (ElohimImagodeiPortalShellClass as unknown as { styles: { cssText: string } })
      .styles.cssText;
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
    const cssText = (ElohimImagodeiPortalShellClass as unknown as { styles: { cssText: string } })
      .styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
