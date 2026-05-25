import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import { ElohimEprLink } from './elohim-epr-link.js';
import { clearMediaQueries, measureLuminanceChanges } from './testing/ua-prefs.js';
import { renderInLocale, requiresLogicalProperties } from './testing/i18n.js';

// ---------------------------------------------------------------------------
// <elohim-epr-link> — core behavior
// ---------------------------------------------------------------------------

describe('<elohim-epr-link>', () => {
  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-epr-link')).to.equal(ElohimEprLink);
  });

  it('renders skeleton at L1 (instant, EPR id only, no resolver)', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:lamad-spa" display="chip"></elohim-epr-link>
    `);
    el.setResolution(1, {});
    await el.updateComplete;
    expect(el.shadowRoot!.querySelector('elohim-skeleton')).to.exist;
  });

  it('renders chip after L2 resolves (title + epr fallback)', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:lamad-spa" display="chip"></elohim-epr-link>
    `);
    el.setResolution(2, { title: 'Lamad Learning Platform' });
    await el.updateComplete;
    const button = el.shadowRoot!.querySelector('button.anchor');
    expect(button?.textContent?.trim()).to.equal('Lamad Learning Platform');
  });

  it('renders EPR id as fallback label when no title resolved', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:fair-exchange" display="inline"></elohim-epr-link>
    `);
    el.setResolution(2, {});
    await el.updateComplete;
    const button = el.shadowRoot!.querySelector('button.anchor');
    expect(button?.textContent?.trim()).to.equal('epr:fair-exchange');
  });

  it('emits navigate event on single click', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:lamad-spa" display="chip"></elohim-epr-link>
    `);
    el.setResolution(2, { title: 'Lamad' });
    await el.updateComplete;

    let navigated: { epr: string } | null = null;
    el.addEventListener('navigate', (e) => {
      navigated = (e as CustomEvent<{ epr: string }>).detail;
    });
    const button = el.shadowRoot!.querySelector('button.anchor') as HTMLElement;
    button.click();
    expect(navigated).to.exist;
    expect(navigated!.epr).to.equal('epr:lamad-spa');
  });

  it('opens context menu on right-click', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:lamad-spa" display="chip"></elohim-epr-link>
    `);
    el.setResolution(2, { title: 'Lamad' });
    await el.updateComplete;

    const button = el.shadowRoot!.querySelector('button.anchor') as HTMLElement;
    button.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
    await el.updateComplete;
    const menu = el.shadowRoot!.querySelector('elohim-context-menu');
    expect(menu).to.exist;
    expect(menu!.hasAttribute('open')).to.be.true;
  });

  it('falls back to <elohim-mention-base> when unreachable (L4)', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:gated-content" display="chip"></elohim-epr-link>
    `);
    el.setResolution(4, { unreachable: true, preview: { title: 'Preview' } });
    await el.updateComplete;
    const fallback = el.shadowRoot!.querySelector('elohim-mention-base');
    expect(fallback).to.exist;
    expect(fallback!.getAttribute('label')).to.equal('Preview');
  });

  it('uses resolution.title as fallback label at L4 when no preview.title', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:gated-content" display="chip"></elohim-epr-link>
    `);
    el.setResolution(4, { unreachable: true, title: 'Partial Title' });
    await el.updateComplete;
    const fallback = el.shadowRoot!.querySelector('elohim-mention-base');
    expect(fallback).to.exist;
    expect(fallback!.getAttribute('label')).to.equal('Partial Title');
  });

  it('emits about event when About this EPR is selected from menu', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:foo" display="chip"></elohim-epr-link>
    `);
    el.setResolution(2, { title: 'Foo' });
    await el.updateComplete;

    let received: { epr: string } | null = null;
    el.addEventListener('about', (e) => {
      received = (e as CustomEvent<{ epr: string }>).detail;
    });

    const button = el.shadowRoot!.querySelector('button.anchor') as HTMLElement;
    button.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
    await el.updateComplete;

    const menu = el.shadowRoot!.querySelector('elohim-context-menu')!;
    // Simulate menu item-select (mirrors how elohim-context-menu dispatches)
    menu.dispatchEvent(
      new CustomEvent('item-select', {
        detail: { id: 'about' },
        bubbles: true,
        composed: true,
      }),
    );
    await el.updateComplete;
    expect(received).to.exist;
    expect(received!.epr).to.equal('epr:foo');
  });

  it('emits navigate event when Open is selected from menu', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:foo" display="chip"></elohim-epr-link>
    `);
    el.setResolution(2, { title: 'Foo' });
    await el.updateComplete;

    let navigated: { epr: string } | null = null;
    el.addEventListener('navigate', (e) => {
      navigated = (e as CustomEvent<{ epr: string }>).detail;
    });

    const button = el.shadowRoot!.querySelector('button.anchor') as HTMLElement;
    button.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
    await el.updateComplete;

    const menu = el.shadowRoot!.querySelector('elohim-context-menu')!;
    menu.dispatchEvent(
      new CustomEvent('item-select', {
        detail: { id: 'open' },
        bubbles: true,
        composed: true,
      }),
    );
    await el.updateComplete;
    expect(navigated).to.exist;
    expect(navigated!.epr).to.equal('epr:foo');
  });

  it('uses resolver to advance to L3 on success', async () => {
    // Set resolver BEFORE setting epr so the resolve() call triggered by
    // updated() picks up the resolver.
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link display="inline"></elohim-epr-link>
    `);
    el.resolver = async (_epr) => ({ title: 'Resolved Title' });
    el.epr = 'epr:resolved-thing';
    await el.updateComplete;
    // One async tick for the promise in resolve() to settle
    await new Promise((r) => setTimeout(r, 0));
    await el.updateComplete;
    const button = el.shadowRoot!.querySelector('button.anchor');
    expect(button?.textContent?.trim()).to.equal('Resolved Title');
  });

  it('advances to L4 unreachable when resolver returns null', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link display="inline"></elohim-epr-link>
    `);
    el.resolver = async () => null;
    el.epr = 'epr:unreachable';
    await el.updateComplete;
    await new Promise((r) => setTimeout(r, 0));
    await el.updateComplete;
    const fallback = el.shadowRoot!.querySelector('elohim-mention-base');
    expect(fallback).to.exist;
  });

  it('Shift+F10 keyboard shortcut opens context menu', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:kbdtest" display="chip"></elohim-epr-link>
    `);
    el.setResolution(2, { title: 'Keyboard Test' });
    await el.updateComplete;

    const button = el.shadowRoot!.querySelector('button.anchor') as HTMLElement;
    button.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'F10', shiftKey: true, bubbles: true }),
    );
    await el.updateComplete;
    const menu = el.shadowRoot!.querySelector('elohim-context-menu');
    expect(menu).to.exist;
    expect(menu!.hasAttribute('open')).to.be.true;
  });
});

// ---------------------------------------------------------------------------
// <elohim-epr-link> — a11y precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-epr-link> — a11y precondition gate', () => {
  it('passes axe-core scan at L2 (chip visible)', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:lamad-spa" display="chip"></elohim-epr-link>
    `);
    el.setResolution(2, { title: 'Lamad Learning Platform' });
    await el.updateComplete;
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe-core scan at L4 (unreachable fallback)', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:gated-content" display="chip"></elohim-epr-link>
    `);
    el.setResolution(4, { unreachable: true, preview: { title: 'Preview Title' } });
    await el.updateComplete;
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('anchor button is keyboard-focusable at L2', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:kbdtest" display="chip"></elohim-epr-link>
    `);
    el.setResolution(2, { title: 'Keyboard Test' });
    await el.updateComplete;
    const button = el.shadowRoot!.querySelector('button.anchor') as HTMLButtonElement;
    expect(button.tabIndex).to.be.greaterThanOrEqual(0);
  });
});

// ---------------------------------------------------------------------------
// <elohim-epr-link> — ua-prefs precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-epr-link> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('forced-colors overrides use CSS system colors (CanvasText, ButtonFace, ButtonText)', () => {
    const cssText = (
      ElohimEprLink as unknown as { styles: { cssText: string } }
    ).styles.cssText;
    expect(cssText).to.contain('forced-colors: active');
    const forcedIdx = cssText.indexOf('forced-colors: active');
    const afterForced = cssText.slice(forcedIdx);
    const hasSystemColor =
      afterForced.includes('CanvasText') ||
      afterForced.includes('ButtonFace') ||
      afterForced.includes('ButtonText') ||
      afterForced.includes('Highlight');
    expect(hasSystemColor).to.be.true;
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:lamad-spa" display="chip"></elohim-epr-link>
    `);
    el.setResolution(2, { title: 'Lamad' });
    await el.updateComplete;
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

// ---------------------------------------------------------------------------
// <elohim-epr-link> — i18n precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-epr-link> — i18n precondition gate', () => {
  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimEprLink as unknown as { styles: { cssText: string } }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });

  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimEprLink>(
      'he-IL',
      html`<elohim-epr-link epr="epr:lamad-spa" display="chip"></elohim-epr-link>`,
    );
    // Force to a visible level since no resolver is wired
    el.setResolution(2, { title: 'Lamad' });
    await el.updateComplete;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const rect = el.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });
});
