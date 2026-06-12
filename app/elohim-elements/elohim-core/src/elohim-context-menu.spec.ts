import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import { ElohimContextMenu } from './elohim-context-menu.js';
import type { ContextMenuItem } from './elohim-context-menu.js';
import { clearMediaQueries, measureLuminanceChanges } from './testing/ua-prefs.js';
import { renderInLocale, requiresLogicalProperties } from './testing/i18n.js';
import {
  assertThemeContrast,
  axeScanStrict,
  themeFixture,
  type ThemeCell,
} from './testing/theme-contrast.js';
import {
  assertInlineWithinViewport,
  resetViewport,
  setViewportArchetype,
} from './testing/viewport.js';

// ---------------------------------------------------------------------------
// Shared fixture data
// ---------------------------------------------------------------------------

const ITEMS: ContextMenuItem[] = [
  { id: 'open', label: 'Open' },
  { id: 'about', label: 'About this EPR' },
  { id: 'copy', label: 'Copy EPR link' },
];

// ---------------------------------------------------------------------------
// <elohim-context-menu> — core behavior
// ---------------------------------------------------------------------------

describe('<elohim-context-menu>', () => {
  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-context-menu')).to.equal(ElohimContextMenu);
  });

  it('is hidden when open is false (default)', async () => {
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${ITEMS}></elohim-context-menu>
    `);
    // :host { display: none } when open attribute is absent
    const style = getComputedStyle(el);
    expect(style.display).to.equal('none');
  });

  it('renders three MVP items when open', async () => {
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
    `);
    const items = el.shadowRoot!.querySelectorAll('[role="menuitem"]');
    expect(items.length).to.equal(3);
    expect(items[0]!.textContent!.trim()).to.equal('Open');
    expect(items[1]!.textContent!.trim()).to.equal('About this EPR');
    expect(items[2]!.textContent!.trim()).to.equal('Copy EPR link');
  });

  it('exposes menu and item parts', async () => {
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
    `);
    expect(el.shadowRoot!.querySelector('[part="menu"]')).to.exist;
    const itemParts = el.shadowRoot!.querySelectorAll('[part="item"]');
    expect(itemParts.length).to.equal(3);
  });

  it('emits item-select event with correct id when an item is clicked', async () => {
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
    `);
    let received: { id: string } | null = null;
    el.addEventListener('item-select', (e: Event) => {
      received = (e as CustomEvent<{ id: string }>).detail;
    });
    const items = el.shadowRoot!.querySelectorAll<HTMLElement>('[role="menuitem"]');
    items[0]!.click();
    expect(received).to.exist;
    expect(received!.id).to.equal('open');
  });

  it('emits close event when an item is activated', async () => {
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
    `);
    let closeFired = false;
    el.addEventListener('close', () => {
      closeFired = true;
    });
    const items = el.shadowRoot!.querySelectorAll<HTMLElement>('[role="menuitem"]');
    items[1]!.click();
    await elementUpdated(el);
    expect(closeFired).to.be.true;
    expect(el.open).to.be.false;
  });

  it('closes (open = false) on Escape key', async () => {
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
    `);
    const menu = el.shadowRoot!.querySelector('[role="menu"]') as HTMLElement;
    menu.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await elementUpdated(el);
    expect(el.open).to.be.false;
  });

  it('emits close event on Escape', async () => {
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
    `);
    let closeFired = false;
    el.addEventListener('close', () => {
      closeFired = true;
    });
    const menu = el.shadowRoot!.querySelector('[role="menu"]') as HTMLElement;
    menu.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await elementUpdated(el);
    expect(closeFired).to.be.true;
  });

  it('ArrowDown moves focus to the next enabled item', async () => {
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
    `);
    await elementUpdated(el);
    const menu = el.shadowRoot!.querySelector('[role="menu"]') as HTMLElement;

    // Initially first enabled item has tabindex=0 (index 0)
    const itemsBefore = el.shadowRoot!.querySelectorAll<HTMLElement>('[role="menuitem"]');
    expect(itemsBefore[0]!.getAttribute('tabindex')).to.equal('0');

    menu.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    await elementUpdated(el);

    const itemsAfter = el.shadowRoot!.querySelectorAll<HTMLElement>('[role="menuitem"]');
    expect(itemsAfter[1]!.getAttribute('tabindex')).to.equal('0');
    expect(itemsAfter[0]!.getAttribute('tabindex')).to.equal('-1');
  });

  it('ArrowUp wraps to the last item from the first', async () => {
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
    `);
    await elementUpdated(el);
    const menu = el.shadowRoot!.querySelector('[role="menu"]') as HTMLElement;

    menu.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true }));
    await elementUpdated(el);

    const items = el.shadowRoot!.querySelectorAll<HTMLElement>('[role="menuitem"]');
    // Wrapped from index 0 to index 2 (last)
    expect(items[2]!.getAttribute('tabindex')).to.equal('0');
  });

  it('Enter on focused item activates it and emits item-select', async () => {
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
    `);
    let received: { id: string } | null = null;
    el.addEventListener('item-select', (e: Event) => {
      received = (e as CustomEvent<{ id: string }>).detail;
    });
    const menu = el.shadowRoot!.querySelector('[role="menu"]') as HTMLElement;
    // focusedIndex starts at 0 ('open')
    menu.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await elementUpdated(el);
    expect(received).to.exist;
    expect(received!.id).to.equal('open');
    expect(el.open).to.be.false;
  });

  it('Space bar on focused item activates it', async () => {
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
    `);
    let received: { id: string } | null = null;
    el.addEventListener('item-select', (e: Event) => {
      received = (e as CustomEvent<{ id: string }>).detail;
    });
    const menu = el.shadowRoot!.querySelector('[role="menu"]') as HTMLElement;
    menu.dispatchEvent(new KeyboardEvent('keydown', { key: ' ', bubbles: true }));
    await elementUpdated(el);
    expect(received).to.exist;
  });

  it('disabled items carry aria-disabled=true', async () => {
    const items: ContextMenuItem[] = [
      { id: 'a', label: 'Action A', disabled: true },
      { id: 'b', label: 'Action B' },
    ];
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${items} open></elohim-context-menu>
    `);
    const liItems = el.shadowRoot!.querySelectorAll<HTMLElement>('[role="menuitem"]');
    expect(liItems[0]!.getAttribute('aria-disabled')).to.equal('true');
    expect(liItems[1]!.getAttribute('aria-disabled')).to.equal('false');
  });

  it('disabled items do not emit item-select on click', async () => {
    const items: ContextMenuItem[] = [
      { id: 'a', label: 'Locked', disabled: true },
      { id: 'b', label: 'Available' },
    ];
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${items} open></elohim-context-menu>
    `);
    let received: { id: string } | null = null;
    el.addEventListener('item-select', (e: Event) => {
      received = (e as CustomEvent<{ id: string }>).detail;
    });
    const li = el.shadowRoot!.querySelectorAll<HTMLElement>('[role="menuitem"]')[0]!;
    li.click();
    expect(received).to.be.null;
    // Menu should still be open (disabled click = no-op)
    expect(el.open).to.be.true;
  });

  it('ArrowDown skips over disabled items', async () => {
    const items: ContextMenuItem[] = [
      { id: 'a', label: 'A' },
      { id: 'b', label: 'B', disabled: true },
      { id: 'c', label: 'C' },
    ];
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${items} open></elohim-context-menu>
    `);
    await elementUpdated(el);
    const menu = el.shadowRoot!.querySelector('[role="menu"]') as HTMLElement;

    // Focused at index 0; ArrowDown should skip disabled index 1 and land on index 2
    menu.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    await elementUpdated(el);

    const liItems = el.shadowRoot!.querySelectorAll<HTMLElement>('[role="menuitem"]');
    expect(liItems[2]!.getAttribute('tabindex')).to.equal('0');
    expect(liItems[1]!.getAttribute('tabindex')).to.equal('-1');
  });

  it('has role=menu on the list element', async () => {
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
    `);
    const menu = el.shadowRoot!.querySelector('[role="menu"]');
    expect(menu).to.exist;
    expect(menu!.tagName.toLowerCase()).to.equal('ul');
  });

  it('host element carries role=presentation', async () => {
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
    `);
    expect(el.getAttribute('role')).to.equal('presentation');
  });
});

// ---------------------------------------------------------------------------
// <elohim-context-menu> — a11y precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-context-menu> — a11y precondition gate', () => {
  it('passes axe-core scan when open with three items', async () => {
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe-core scan with a disabled item', async () => {
    const items: ContextMenuItem[] = [
      { id: 'x', label: 'Allowed' },
      { id: 'y', label: 'Blocked', disabled: true },
    ];
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${items} open></elohim-context-menu>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

// ---------------------------------------------------------------------------
// <elohim-context-menu> — ua-prefs precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-context-menu> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('fold-down animation is wrapped in (prefers-reduced-motion: no-preference)', () => {
    const cssText = (ElohimContextMenu as unknown as { styles: { cssText: string } }).styles
      .cssText;
    expect(cssText).to.contain('prefers-reduced-motion');
    // The animation keyframe should only appear inside that media query block
    const motionIdx = cssText.indexOf('prefers-reduced-motion: no-preference');
    expect(motionIdx).to.be.greaterThan(-1);

    const afterMotion = cssText.slice(motionIdx);
    expect(afterMotion).to.contain('elohim-menu-fold-down');
  });

  it('forced-colors overrides use CSS system colors (Canvas, CanvasText, Highlight, HighlightText)', () => {
    const cssText = (ElohimContextMenu as unknown as { styles: { cssText: string } }).styles
      .cssText;
    expect(cssText).to.contain('forced-colors: active');
    const forcedIdx = cssText.indexOf('forced-colors: active');
    // eslint-disable-next-line unicorn/prefer-set-has -- string scan, not membership lookup
    const afterForced = cssText.slice(forcedIdx);
    const hasSystemColor =
      afterForced.includes('Canvas') ||
      afterForced.includes('CanvasText') ||
      afterForced.includes('Highlight') ||
      afterForced.includes('HighlightText');
    expect(hasSystemColor).to.be.true;
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimContextMenu>(html`
      <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

// ---------------------------------------------------------------------------
// <elohim-context-menu> — i18n precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-context-menu> — i18n precondition gate', () => {
  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (ElohimContextMenu as unknown as { styles: { cssText: string } }).styles
      .cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });

  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimContextMenu>(
      'he-IL',
      html`
        <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const rect = el.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// <elohim-context-menu> — theme-contrast gate
// ---------------------------------------------------------------------------

describe('<elohim-context-menu> — theme-contrast gate', () => {
  // tokens cells: no shipped binding for this element yet — system cells only
  // (theme-authority spec §4.1). The blank-slate contract per scheme.
  const CELLS: ThemeCell[] = ['system-light', 'system-dark'];

  // Menu items must be visible for the contrast walk, so render OPEN with a
  // disabled item present (its dimmed state is the lowest-contrast surface).
  const THEME_ITEMS: ContextMenuItem[] = [
    { id: 'open', label: 'Open' },
    { id: 'about', label: 'About this EPR' },
    { id: 'copy', label: 'Copy EPR link', disabled: true },
  ];

  for (const cell of CELLS) {
    it(`passes contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimContextMenu>(
        html`
          <elohim-context-menu .items=${THEME_ITEMS} open></elohim-context-menu>
        `,
        cell
      );
      await el.updateComplete;
      assertThemeContrast(el);
      await axeScanStrict(el);
    });
  }
});

// ---------------------------------------------------------------------------
// <elohim-context-menu> — viewport precondition gate
//
// Same regression class as <elohim-hypercard-panel> (2026-06-12): an anchored
// fold-down surface near the inline-end viewport edge extends off-screen on
// phone archetypes. The menu's geometry was strictly worse — no insets at all
// (static position inside the consumer's .menu-anchor) and NO max-inline-size
// cap. The fix is additive: `align` is OPT-IN (no attribute → static position
// preserved for existing consumers like <elohim-epr-link>); the viewport
// width cap applies always.
// ---------------------------------------------------------------------------

describe('<elohim-context-menu> — viewport precondition gate', () => {
  afterEach(() => resetViewport());

  it('default (no align) keeps static positioning — below the anchor content, start-aligned', async () => {
    // Behavior pin for existing consumers (<elohim-epr-link>): with no align
    // attribute the menu lands at its static position inside the wrap.
    const wrap = await fixture<HTMLSpanElement>(html`
      <span style="position: relative; display: inline-block;">
        <button type="button">chip</button>
        <elohim-context-menu .items=${ITEMS} open></elohim-context-menu>
      </span>
    `);
    const menu = wrap.querySelector('elohim-context-menu') as ElohimContextMenu;
    await menu.updateComplete;
    const button = wrap.querySelector('button') as HTMLButtonElement;
    // Static position lands the block box at the line box after the inline
    // chip — within a few px of the button's bottom edge (sub-line-box slack).
    expect(menu.getBoundingClientRect().top).to.be.greaterThan(
      button.getBoundingClientRect().bottom - 5
    );
    expect(
      Math.abs(menu.getBoundingClientRect().left - wrap.getBoundingClientRect().left)
    ).to.be.lessThan(1);
  });

  it('align="end" keeps an end-edge-anchored menu inside the phone viewport', async () => {
    await setViewportArchetype('phone');
    const row = await fixture<HTMLDivElement>(html`
      <div style="display: flex; justify-content: flex-end;">
        <span style="position: relative; display: inline-block;">
          <button type="button">chip</button>
          <elohim-context-menu .items=${ITEMS} align="end" open></elohim-context-menu>
        </span>
      </div>
    `);
    const menu = row.querySelector('elohim-context-menu') as ElohimContextMenu;
    await menu.updateComplete;
    assertInlineWithinViewport(menu);
    const wrap = menu.parentElement as HTMLElement;
    expect(
      Math.abs(menu.getBoundingClientRect().right - wrap.getBoundingClientRect().right)
    ).to.be.lessThan(1);
  });

  it('align="end" folds the menu down below the anchor wrap', async () => {
    await setViewportArchetype('phone');
    const row = await fixture<HTMLDivElement>(html`
      <div style="display: flex; justify-content: flex-end;">
        <span style="position: relative; display: inline-block;">
          <button type="button">chip</button>
          <elohim-context-menu .items=${ITEMS} align="end" open></elohim-context-menu>
        </span>
      </div>
    `);
    const menu = row.querySelector('elohim-context-menu') as ElohimContextMenu;
    await menu.updateComplete;
    const wrap = menu.parentElement as HTMLElement;
    expect(menu.getBoundingClientRect().top).to.be.greaterThan(
      wrap.getBoundingClientRect().bottom - 1
    );
  });

  it('caps its inline size below the viewport width on phone-small (unbreakable label)', async () => {
    await setViewportArchetype('phone-small');
    // A content address is the realistic unbreakable token (no hyphenation
    // break opportunities mid-hex) — EPR-flavored labels can carry these.
    const longItems: ContextMenuItem[] = [
      {
        id: 'long',
        label: 'sha256e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
      },
    ];
    const wrap = await fixture<HTMLSpanElement>(html`
      <span style="position: relative; display: inline-block;">
        <button type="button">chip</button>
        <elohim-context-menu .items=${longItems} open></elohim-context-menu>
      </span>
    `);
    const menu = wrap.querySelector('elohim-context-menu') as ElohimContextMenu;
    await menu.updateComplete;
    const viewportWidth = document.documentElement.clientWidth;
    expect(menu.getBoundingClientRect().width).to.be.at.most(viewportWidth - 8);
  });
});
