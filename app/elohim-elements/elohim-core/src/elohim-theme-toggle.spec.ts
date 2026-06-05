import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import { ElohimThemeToggle as ToggleClass } from './elohim-theme-toggle.js';
import { THEME_STORAGE_KEY, getThemeStore, resetThemeStoreInstance } from './theme/theme-store.js';
import { requiresLogicalProperties } from './testing/i18n.js';
import {
  assertThemeContrast,
  assertThemeReactivity,
  axeScanStrict,
  themeFixture,
  type ThemeCell,
} from './testing/theme-contrast.js';

describe('<elohim-theme-toggle>', () => {
  beforeEach(() => {
    localStorage.removeItem(THEME_STORAGE_KEY);
    getThemeStore().set('device');
  });

  afterEach(() => {
    resetThemeStoreInstance();
  });

  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-theme-toggle')).to.equal(ToggleClass);
  });

  it('renders a labelled button with the auto indicator in device mode', async () => {
    const el = await fixture<ElohimThemeToggle>(html`
      <elohim-theme-toggle></elohim-theme-toggle>
    `);
    const btn = el.shadowRoot!.querySelector('button[part="button"]');
    expect(btn).to.exist;
    expect(btn!.getAttribute('aria-label')).to.be.a('string').and.not.empty;
    expect(el.shadowRoot!.querySelector('[part="auto-indicator"]')).to.exist;
  });

  it('click cycles the shared store device → light and hides the auto indicator', async () => {
    const el = await fixture<ElohimThemeToggle>(html`
      <elohim-theme-toggle></elohim-theme-toggle>
    `);
    el.shadowRoot!.querySelector<HTMLButtonElement>('button')!.click();
    await elementUpdated(el);
    expect(getThemeStore().theme).to.equal('light');
    expect(document.body.getAttribute('data-theme')).to.equal('light');
    expect(el.shadowRoot!.querySelector('[part="auto-indicator"]')).to.not.exist;
  });

  it('follows external store changes (two toggles stay in sync)', async () => {
    const el = await fixture<ElohimThemeToggle>(html`
      <elohim-theme-toggle></elohim-theme-toggle>
    `);
    getThemeStore().set('dark');
    await elementUpdated(el);
    expect(el.shadowRoot!.querySelector('[part="icon"]')!.textContent).to.contain('🌙');
  });

  it('dispatches theme-changed with the new theme', async () => {
    const el = await fixture<ElohimThemeToggle>(html`
      <elohim-theme-toggle></elohim-theme-toggle>
    `);
    let detail: { theme?: string } | null = null;
    el.addEventListener('theme-changed', e => {
      detail = (e as CustomEvent<{ theme: string }>).detail;
    });
    el.shadowRoot!.querySelector<HTMLButtonElement>('button')!.click();
    expect(detail).to.deep.equal({ theme: 'light' });
  });

  it('passes the a11y gate (axe)', async () => {
    const el = await fixture<ElohimThemeToggle>(html`
      <elohim-theme-toggle></elohim-theme-toggle>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.be.empty;
  });

  it('passes the i18n gate (logical properties only)', () => {
    const cssText = (
      ToggleClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });

  it('passes the ua-prefs gate (no transitions declared)', () => {
    const cssText = (
      ToggleClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition');
  });
});

import type { ElohimThemeToggle } from './elohim-theme-toggle.js';

describe('<elohim-theme-toggle> — theme-contrast gate', () => {
  beforeEach(() => {
    localStorage.removeItem(THEME_STORAGE_KEY);
    getThemeStore().set('device'); // device mode renders the "A" auto-indicator badge
  });

  afterEach(() => {
    resetThemeStoreInstance();
  });

  const CELLS: ThemeCell[] = ['system-light', 'system-dark', 'tokens-light', 'tokens-dark'];

  for (const cell of CELLS) {
    it(`toggle (with auto badge) passes contrast in ${cell}`, async () => {
      const { el } = await themeFixture<InstanceType<typeof ToggleClass>>(
        html`
          <elohim-theme-toggle></elohim-theme-toggle>
        `,
        cell
      );
      await el.updateComplete;
      // the badge must actually be rendered for this cell to mean anything
      expect(el.shadowRoot!.querySelector("[part='auto-indicator']")).to.exist;
      assertThemeContrast(el);
      await axeScanStrict(el);
    });
  }

  it('toggle reacts to the theme (frozen-chain canary)', async () => {
    await assertThemeReactivity<InstanceType<typeof ToggleClass>>(
      () =>
        html`
          <elohim-theme-toggle></elohim-theme-toggle>
        `,
      el => el.shadowRoot!.querySelector('button')
    );
  });
});
