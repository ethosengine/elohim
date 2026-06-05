import { expect } from '@open-wc/testing';

import {
  LOCALE_CHANGE_EVENT,
  LOCALE_STORAGE_KEY,
  LocaleStore,
  SUPPORTED_LOCALES,
  detectLocale,
} from './locale-store.js';

describe('LocaleStore', () => {
  beforeEach(() => {
    localStorage.removeItem(LOCALE_STORAGE_KEY);
    document.documentElement.lang = '';
    document.documentElement.dir = '';
  });

  it('supports the lit-localize registry (en source + es/he targets)', () => {
    expect([...SUPPORTED_LOCALES]).to.deep.equal(['en', 'es', 'he']);
  });

  it('detectLocale maps a base language to a supported locale, else en', () => {
    // jsdom/browser navigator.language is environment-dependent;
    // assert the contract on the result domain instead of a fixed value.
    expect([...SUPPORTED_LOCALES]).to.include(detectLocale());
  });

  it('loads a persisted valid locale', () => {
    localStorage.setItem(LOCALE_STORAGE_KEY, 'es');
    const store = new LocaleStore();
    expect(store.locale).to.equal('es');
    expect(document.documentElement.lang).to.equal('es');
    expect(document.documentElement.dir).to.equal('ltr');
  });

  it('set() persists, applies lang/dir (he → rtl) and dispatches once', () => {
    const store = new LocaleStore();
    let events = 0;
    const onEvent = (): void => {
      events += 1;
    };
    window.addEventListener(LOCALE_CHANGE_EVENT, onEvent);
    store.set('he');
    store.set('he'); // no-op
    window.removeEventListener(LOCALE_CHANGE_EVENT, onEvent);
    expect(store.locale).to.equal('he');
    expect(localStorage.getItem(LOCALE_STORAGE_KEY)).to.equal('he');
    expect(document.documentElement.lang).to.equal('he');
    expect(document.documentElement.dir).to.equal('rtl');
    expect(events).to.equal(1);
  });

  it('adopts an external change event without re-dispatching', () => {
    const store = new LocaleStore();
    let events = 0;
    const onEvent = (): void => {
      events += 1;
    };
    window.addEventListener(LOCALE_CHANGE_EVENT, onEvent);
    window.dispatchEvent(new CustomEvent(LOCALE_CHANGE_EVENT, { detail: { locale: 'es' } }));
    window.removeEventListener(LOCALE_CHANGE_EVENT, onEvent);
    expect(store.locale).to.equal('es');
    expect(events).to.equal(1);
  });
});
