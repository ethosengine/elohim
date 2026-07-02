import { ContextProvider } from '@lit/context';
import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import { ElohimEprLink } from './elohim-epr-link.js';
import {
  eprResolutionContext,
  type EprResolutionProvider,
} from './navigation/epr-resolution-provider.js';
import { clearMediaQueries, measureLuminanceChanges } from './testing/ua-prefs.js';
import { renderInLocale, requiresLogicalProperties } from './testing/i18n.js';
import {
  assertThemeContrast,
  axeScanStrict,
  themeFixture,
  type ThemeCell,
} from './testing/theme-contrast.js';

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
    el.addEventListener('navigate', e => {
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

  // ── Typed degradation (forbidden / missing / error) ───────────────────────

  it('renders an honest "unavailable at your reach" affordance for the forbidden state', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:gated-content" display="chip"></elohim-epr-link>
    `);
    el.setResolution(4, { state: 'forbidden', title: 'Manifesto' });
    await el.updateComplete;
    // The title still shows (never a raw epr id when metadata is present)…
    const fallback = el.shadowRoot!.querySelector('elohim-mention-base');
    expect(fallback).to.exist;
    expect(fallback!.getAttribute('label')).to.equal('Manifesto');
    // …paired with an honest reason the visitor can read.
    const note = el.shadowRoot!.querySelector('[part="fallback-note"]');
    expect(note).to.exist;
    expect(note!.textContent?.trim()).to.equal('Unavailable at your reach');
  });

  it('renders an honest missing state for the missing state', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:gone" display="chip"></elohim-epr-link>
    `);
    el.setResolution(4, { state: 'missing' });
    await el.updateComplete;
    expect(el.shadowRoot!.querySelector('elohim-mention-base')).to.exist;
    const note = el.shadowRoot!.querySelector('[part="fallback-note"]');
    expect(note!.textContent?.trim()).to.equal('No longer available');
  });

  it('renders an honest error state for the error state', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:boom" display="chip"></elohim-epr-link>
    `);
    el.setResolution(4, { state: 'error' });
    await el.updateComplete;
    const note = el.shadowRoot!.querySelector('[part="fallback-note"]');
    expect(note!.textContent?.trim()).to.equal('Could not load');
  });

  it('never shows a raw epr id as the visible chip when the head resolves (state: resolved)', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link display="inline"></elohim-epr-link>
    `);
    // A head-first resolver returns a typed resolved outcome.
    el.resolver = async _epr => ({
      state: 'resolved',
      title: 'Governance Epic',
      reach: 'community',
    });
    el.epr = 'epr:governance-epic';
    await el.updateComplete;
    await new Promise(r => setTimeout(r, 0));
    await el.updateComplete;
    const button = el.shadowRoot!.querySelector('button.anchor');
    expect(button?.textContent?.trim()).to.equal('Governance Epic');
  });

  it('advances to the forbidden degraded face when the resolver returns a forbidden outcome', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link display="inline"></elohim-epr-link>
    `);
    el.resolver = async _epr => ({ state: 'forbidden' });
    el.epr = 'epr:gated';
    await el.updateComplete;
    await new Promise(r => setTimeout(r, 0));
    await el.updateComplete;
    expect(el.shadowRoot!.querySelector('elohim-mention-base')).to.exist;
    const note = el.shadowRoot!.querySelector('[part="fallback-note"]');
    expect(note!.textContent?.trim()).to.equal('Unavailable at your reach');
  });

  it('emits about event when About this EPR is selected from menu', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:foo" display="chip"></elohim-epr-link>
    `);
    el.setResolution(2, { title: 'Foo' });
    await el.updateComplete;

    let received: { epr: string } | null = null;
    el.addEventListener('about', e => {
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
      })
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
    el.addEventListener('navigate', e => {
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
      })
    );
    await el.updateComplete;
    expect(navigated).to.exist;
    expect(navigated!.epr).to.equal('epr:foo');
  });

  it('renders host-injected contextMenuItems instead of the default three', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:foo" display="chip"></elohim-epr-link>
    `);
    el.setResolution(2, { title: 'Foo' });
    el.contextMenuItems = [
      { id: 'open', label: 'Open' },
      { id: 'network', label: 'View network & resilience' },
      { id: 'flag', label: 'Flag' },
    ];
    await el.updateComplete;

    const button = el.shadowRoot!.querySelector('button.anchor') as HTMLElement;
    button.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
    await el.updateComplete;

    const menu = el.shadowRoot!.querySelector('elohim-context-menu')!;
    await (menu as ElohimEprLink & { updateComplete: Promise<unknown> }).updateComplete;
    const labels = [...menu.shadowRoot!.querySelectorAll('[role="menuitem"]')].map(li =>
      li.textContent?.trim()
    );
    expect(labels).to.deep.equal(['Open', 'View network & resilience', 'Flag']);
  });

  it('re-emits epr-menu-select with id+epr for a host-injected selection', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:foo" display="chip"></elohim-epr-link>
    `);
    el.setResolution(2, { title: 'Foo' });
    el.contextMenuItems = [{ id: 'network', label: 'View network & resilience' }];
    await el.updateComplete;

    let received: { id: string; epr: string } | null = null;
    el.addEventListener('epr-menu-select', e => {
      received = (e as CustomEvent<{ id: string; epr: string }>).detail;
    });

    const button = el.shadowRoot!.querySelector('button.anchor') as HTMLElement;
    button.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
    await el.updateComplete;

    const menu = el.shadowRoot!.querySelector('elohim-context-menu')!;
    menu.dispatchEvent(
      new CustomEvent('item-select', {
        detail: { id: 'network' },
        bubbles: true,
        composed: true,
      })
    );
    await el.updateComplete;
    expect(received).to.exist;
    expect(received!.id).to.equal('network');
    expect(received!.epr).to.equal('epr:foo');
  });

  it('renders the default MVP three when no contextMenuItems override is set', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:foo" display="chip"></elohim-epr-link>
    `);
    el.setResolution(2, { title: 'Foo' });
    await el.updateComplete;

    const button = el.shadowRoot!.querySelector('button.anchor') as HTMLElement;
    button.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
    await el.updateComplete;

    const menu = el.shadowRoot!.querySelector('elohim-context-menu')!;
    await (menu as ElohimEprLink & { updateComplete: Promise<unknown> }).updateComplete;
    const labels = [...menu.shadowRoot!.querySelectorAll('[role="menuitem"]')].map(li =>
      li.textContent?.trim()
    );
    expect(labels).to.deep.equal(['Open', 'About this EPR', 'Copy EPR link']);
  });

  it('uses resolver to advance to L3 on success', async () => {
    // Set resolver BEFORE setting epr so the resolve() call triggered by
    // updated() picks up the resolver.
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link display="inline"></elohim-epr-link>
    `);
    el.resolver = async _epr => ({ title: 'Resolved Title' });
    el.epr = 'epr:resolved-thing';
    await el.updateComplete;
    // One async tick for the promise in resolve() to settle
    await new Promise(r => setTimeout(r, 0));
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
    await new Promise(r => setTimeout(r, 0));
    await el.updateComplete;
    const fallback = el.shadowRoot!.querySelector('elohim-mention-base');
    expect(fallback).to.exist;
  });

  // ── Ambient resolution provider (@lit/context) ────────────────────────────

  /** Install an ambient EprResolutionProvider on a fresh host and return it. */
  function withProvider(value: EprResolutionProvider): HTMLElement {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const provider = new ContextProvider(host, {
      context: eprResolutionContext,
      initialValue: value,
    });
    provider.hostConnected();
    return host;
  }

  it('resolves through the ambient context provider when no explicit resolver prop is set', async () => {
    const host = withProvider({
      resolveHead: async (ref: string) => ({
        state: 'resolved',
        head: { id: ref, title: 'Ambient Title', reach: 'commons' },
      }),
      resolveRoute: () => null,
      resolveBody: async () => ({ state: 'missing' }),
    });

    const el = document.createElement('elohim-epr-link') as ElohimEprLink;
    el.setAttribute('display', 'chip');
    el.epr = 'epr:ambient-thing';
    host.appendChild(el);
    await el.updateComplete;
    await new Promise(r => setTimeout(r, 0));
    await el.updateComplete;

    const button = el.shadowRoot!.querySelector('button.anchor');
    expect(button?.textContent?.trim()).to.equal('Ambient Title');
    host.remove();
  });

  it('lets an explicit .resolver prop win over the ambient context provider', async () => {
    const host = withProvider({
      resolveHead: async () => ({
        state: 'resolved',
        head: { id: 'x', title: 'AMBIENT (should not win)' },
      }),
      resolveRoute: () => null,
      resolveBody: async () => ({ state: 'missing' }),
    });

    const el = document.createElement('elohim-epr-link') as ElohimEprLink;
    el.resolver = async () => ({ state: 'resolved', title: 'EXPLICIT WINS' });
    el.epr = 'epr:override';
    host.appendChild(el);
    await el.updateComplete;
    await new Promise(r => setTimeout(r, 0));
    await el.updateComplete;

    const button = el.shadowRoot!.querySelector('button.anchor');
    expect(button?.textContent?.trim()).to.equal('EXPLICIT WINS');
    host.remove();
  });

  it('degrades to the forbidden face when the ambient provider returns forbidden', async () => {
    const host = withProvider({
      resolveHead: async () => ({ state: 'forbidden', head: { id: 'm', title: 'Manifesto' } }),
      resolveRoute: () => null,
      resolveBody: async () => ({ state: 'forbidden' }),
    });

    const el = document.createElement('elohim-epr-link') as ElohimEprLink;
    el.setAttribute('display', 'chip');
    el.epr = 'epr:gated';
    host.appendChild(el);
    await el.updateComplete;
    await new Promise(r => setTimeout(r, 0));
    await el.updateComplete;

    const fallback = el.shadowRoot!.querySelector('elohim-mention-base');
    expect(fallback!.getAttribute('label')).to.equal('Manifesto');
    const note = el.shadowRoot!.querySelector('[part="fallback-note"]');
    expect(note!.textContent?.trim()).to.equal('Unavailable at your reach');
    host.remove();
  });

  it('Shift+F10 keyboard shortcut opens context menu', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:kbdtest" display="chip"></elohim-epr-link>
    `);
    el.setResolution(2, { title: 'Keyboard Test' });
    await el.updateComplete;

    const button = el.shadowRoot!.querySelector('button.anchor') as HTMLElement;
    button.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'F10', shiftKey: true, bubbles: true })
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

  it('passes axe-core scan at L4 (forbidden degraded face with honest note)', async () => {
    const el = await fixture<ElohimEprLink>(html`
      <elohim-epr-link epr="epr:gated-content" display="chip"></elohim-epr-link>
    `);
    el.setResolution(4, { state: 'forbidden', title: 'Manifesto' });
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
    const cssText = (ElohimEprLink as unknown as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.contain('forced-colors: active');
    const forcedIdx = cssText.indexOf('forced-colors: active');
    // Substring containment on the CSS text after the forced-colors block — not
    // Set membership (a Set built from a string iterates single characters, so
    // .has('CanvasText') would always be false). Disable the prefer-set-has
    // autofix, which would wrongly rewrite these String.includes calls to Set.has.
    // eslint-disable-next-line unicorn/prefer-set-has
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
    const cssText = (ElohimEprLink as unknown as { styles: { cssText: string } }).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });

  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimEprLink>(
      'he-IL',
      html`
        <elohim-epr-link epr="epr:lamad-spa" display="chip"></elohim-epr-link>
      `
    );
    // Force to a visible level since no resolver is wired
    el.setResolution(2, { title: 'Lamad' });
    await el.updateComplete;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const rect = el.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// <elohim-epr-link> — theme-contrast gate
// ---------------------------------------------------------------------------

describe('<elohim-epr-link> — theme-contrast gate', () => {
  // tokens cells: no shipped binding for this element yet — system cells only
  // (theme-authority spec §4.1). The blank-slate contract per scheme.
  const CELLS: ThemeCell[] = ['system-light', 'system-dark'];

  for (const cell of CELLS) {
    it(`passes contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimEprLink>(
        html`
          <elohim-epr-link epr="epr:lamad-spa" display="chip"></elohim-epr-link>
        `,
        cell
      );
      await el.updateComplete;
      // No resolver is wired in the fixture; force to L2 so the anchor text is
      // visible (mirrors the a11y/ua-prefs gates above).
      el.setResolution(2, { title: 'Lamad Learning Platform' });
      await el.updateComplete;
      assertThemeContrast(el);
      await axeScanStrict(el);
    });
  }
});
