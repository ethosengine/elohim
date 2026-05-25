import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import { ElohimPageChrome as ElohimPageChromeClass } from './elohim-page-chrome.js';
import { ElohimDefaultOmnibar as ElohimDefaultOmnibarClass } from './elohim-default-omnibar.js';
import { clearMediaQueries, measureLuminanceChanges } from './testing/ua-prefs.js';
import { renderInLocale, requiresLogicalProperties } from './testing/i18n.js';

// ---------------------------------------------------------------------------
// <elohim-page-chrome>
// ---------------------------------------------------------------------------

describe('<elohim-page-chrome>', () => {
  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-page-chrome')).to.equal(ElohimPageChromeClass);
  });

  it('renders <elohim-default-omnibar> when no slotted omnibar is present', async () => {
    const el = await fixture(html`
      <elohim-page-chrome>
        <p>content</p>
      </elohim-page-chrome>
    `);
    const defaultBar = el.shadowRoot!.querySelector('elohim-default-omnibar');
    expect(defaultBar).to.exist;
    expect(defaultBar!.hasAttribute('hidden')).to.be.false;
  });

  it('hides <elohim-default-omnibar> when the omnibar slot is filled', async () => {
    const el = await fixture(html`
      <elohim-page-chrome>
        <div slot="omnibar" id="custom-bar">my custom bar</div>
        <p>content</p>
      </elohim-page-chrome>
    `);
    // Allow the slotchange microtask to settle.
    await elementUpdated(el);
    const slot = el.shadowRoot!.querySelector('slot[name="omnibar"]') as HTMLSlotElement;
    const assigned = slot
      .assignedNodes({ flatten: true })
      .filter((n) => n.nodeType === Node.ELEMENT_NODE);
    expect(assigned.some((n) => (n as HTMLElement).id === 'custom-bar')).to.be.true;
    const defaultBar = el.shadowRoot!.querySelector('elohim-default-omnibar');
    expect(defaultBar!.hasAttribute('hidden')).to.be.true;
  });

  it('shows <elohim-default-omnibar> again when the slotted element is removed', async () => {
    const container = await fixture<HTMLDivElement>(html`
      <div>
        <elohim-page-chrome>
          <div slot="omnibar" id="custom-bar">custom bar</div>
        </elohim-page-chrome>
      </div>
    `);
    const chrome = container.querySelector('elohim-page-chrome')!;
    await elementUpdated(chrome);

    // Confirm hidden first.
    let defaultBar = chrome.shadowRoot!.querySelector('elohim-default-omnibar');
    expect(defaultBar!.hasAttribute('hidden')).to.be.true;

    // Remove the slotted child.
    container.querySelector('#custom-bar')!.remove();
    await elementUpdated(chrome);

    defaultBar = chrome.shadowRoot!.querySelector('elohim-default-omnibar');
    expect(defaultBar!.hasAttribute('hidden')).to.be.false;
  });

  it('exposes omnibar-host and main parts', async () => {
    const el = await fixture(html`
      <elohim-page-chrome></elohim-page-chrome>
    `);
    expect(el.shadowRoot!.querySelector('[part="omnibar-host"]')).to.exist;
    expect(el.shadowRoot!.querySelector('[part="main"]')).to.exist;
  });

  it('renders slotted page content in the default slot', async () => {
    const el = await fixture(html`
      <elohim-page-chrome>
        <p id="pg">hello</p>
      </elohim-page-chrome>
    `);
    const mainSlot = el.shadowRoot!.querySelector('slot:not([name])') as HTMLSlotElement;
    const assigned = mainSlot.assignedNodes({ flatten: true });
    expect(assigned.some((n) => (n as HTMLElement).id === 'pg')).to.be.true;
  });

  it('passes axe-core a11y scan in default state', async () => {
    const el = await fixture(html`
      <elohim-page-chrome>
        <p>accessible page</p>
      </elohim-page-chrome>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-page-chrome> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('forced-colors overrides use CSS system colors (Canvas, CanvasText, ButtonText)', () => {
    const cssText = (
      ElohimPageChromeClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.contain('forced-colors: active');
    const forcedIdx = cssText.indexOf('forced-colors: active');
    const afterForced = cssText.slice(forcedIdx);
    const hasSystemColor =
      afterForced.includes('Canvas') ||
      afterForced.includes('CanvasText') ||
      afterForced.includes('ButtonText') ||
      afterForced.includes('ButtonText');
    expect(hasSystemColor).to.be.true;
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture(html`
      <elohim-page-chrome><p>content</p></elohim-page-chrome>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-page-chrome> — i18n precondition gate', () => {
  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimPageChromeClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });

  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale(
      'he-IL',
      html`<elohim-page-chrome><p>תוכן</p></elohim-page-chrome>`
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const rect = el.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// <elohim-default-omnibar>
// ---------------------------------------------------------------------------

describe('<elohim-default-omnibar>', () => {
  function clearSession() {
    document.cookie = 'elohim_session=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=/;';
  }

  beforeEach(() => clearSession());
  afterEach(() => clearSession());

  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-default-omnibar')).to.equal(ElohimDefaultOmnibarClass);
  });

  it('renders a sign-in link when anonymous (no session cookie)', async () => {
    const el = await fixture(html`<elohim-default-omnibar></elohim-default-omnibar>`);
    const link = el.shadowRoot!.querySelector('a[href="/auth/signin"]');
    expect(link).to.exist;
    expect(link!.textContent!.trim()).to.equal('sign in');
  });

  it('renders the user humanId when session cookie is present', async () => {
    document.cookie = `elohim_session=${encodeURIComponent(
      JSON.stringify({ humanId: 'matthew', capabilities: [], reach: 'authenticated' })
    )}; path=/`;
    const el = await fixture(html`<elohim-default-omnibar></elohim-default-omnibar>`);
    const userNameEl = el.shadowRoot!.querySelector('[part="user-name"]');
    expect(userNameEl).to.exist;
    expect(userNameEl!.textContent!.trim()).to.equal('matthew');
  });

  it('does not render a sign-in link when authenticated', async () => {
    document.cookie = `elohim_session=${encodeURIComponent(
      JSON.stringify({ humanId: 'jessica', capabilities: ['pilot'], reach: 'authenticated' })
    )}; path=/`;
    const el = await fixture(html`<elohim-default-omnibar></elohim-default-omnibar>`);
    const link = el.shadowRoot!.querySelector('a[href="/auth/signin"]');
    expect(link).to.be.null;
  });

  it('exposes brand and user as named parts', async () => {
    const el = await fixture(html`<elohim-default-omnibar></elohim-default-omnibar>`);
    expect(el.shadowRoot!.querySelector('[part="brand"]')).to.exist;
    expect(el.shadowRoot!.querySelector('[part="user"]')).to.exist;
  });

  it('passes axe-core a11y scan when anonymous', async () => {
    const el = await fixture(html`<elohim-default-omnibar></elohim-default-omnibar>`);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe-core a11y scan when authenticated', async () => {
    document.cookie = `elohim_session=${encodeURIComponent(
      JSON.stringify({ humanId: 'james', capabilities: [], reach: 'authenticated' })
    )}; path=/`;
    const el = await fixture(html`<elohim-default-omnibar></elohim-default-omnibar>`);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-default-omnibar> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('forced-colors overrides use CSS system colors (Canvas, CanvasText, LinkText)', () => {
    const cssText = (
      ElohimDefaultOmnibarClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.contain('forced-colors: active');
    const forcedIdx = cssText.indexOf('forced-colors: active');
    const afterForced = cssText.slice(forcedIdx);
    const hasSystemColor =
      afterForced.includes('Canvas') ||
      afterForced.includes('CanvasText') ||
      afterForced.includes('LinkText') ||
      afterForced.includes('ButtonText');
    expect(hasSystemColor).to.be.true;
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture(html`<elohim-default-omnibar></elohim-default-omnibar>`);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-default-omnibar> — i18n precondition gate', () => {
  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimDefaultOmnibarClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });

  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale(
      'he-IL',
      html`<elohim-default-omnibar></elohim-default-omnibar>`
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const brand = el.shadowRoot!.querySelector('[part="brand"]')!;
    const rect = brand.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
    expect(rect.height).to.be.greaterThan(0);
  });
});
