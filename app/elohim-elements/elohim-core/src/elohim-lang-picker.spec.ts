import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import { ElohimLangPicker as PickerClass } from './elohim-lang-picker.js';
import { LOCALE_STORAGE_KEY, getLocaleStore } from './localize/locale-store.js';

describe('<elohim-lang-picker>', () => {
  beforeEach(() => {
    localStorage.removeItem(LOCALE_STORAGE_KEY);
    getLocaleStore().set('en');
  });

  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-lang-picker')).to.equal(PickerClass);
  });

  it('renders a select with the three locales in native script', async () => {
    const el = await fixture<ElohimLangPicker>(html`<elohim-lang-picker></elohim-lang-picker>`);
    const options = [...el.shadowRoot!.querySelectorAll('option')];
    expect(options.map((o) => o.value)).to.deep.equal(['en', 'es', 'he']);
    expect(options.map((o) => o.textContent?.trim())).to.deep.equal(['English', 'Español', 'עברית']);
  });

  it('selecting a locale drives the store, document lang/dir, and persists', async () => {
    const el = await fixture<ElohimLangPicker>(html`<elohim-lang-picker></elohim-lang-picker>`);
    const select = el.shadowRoot!.querySelector<HTMLSelectElement>('select')!;
    select.value = 'he';
    select.dispatchEvent(new Event('change'));
    await elementUpdated(el);
    expect(getLocaleStore().locale).to.equal('he');
    expect(document.documentElement.dir).to.equal('rtl');
    expect(localStorage.getItem(LOCALE_STORAGE_KEY)).to.equal('he');
  });

  it('dispatches locale-changed with the new locale', async () => {
    const el = await fixture<ElohimLangPicker>(html`<elohim-lang-picker></elohim-lang-picker>`);
    let detail: { locale?: string } | null = null;
    el.addEventListener('locale-changed', (e) => {
      detail = (e as CustomEvent<{ locale: string }>).detail;
    });
    const select = el.shadowRoot!.querySelector<HTMLSelectElement>('select')!;
    select.value = 'es';
    select.dispatchEvent(new Event('change'));
    expect(detail).to.deep.equal({ locale: 'es' });
  });

  it('passes the a11y gate (axe)', async () => {
    const el = await fixture<ElohimLangPicker>(html`<elohim-lang-picker></elohim-lang-picker>`);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.be.empty;
  });
});

import type { ElohimLangPicker } from './elohim-lang-picker.js';
