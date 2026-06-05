import { expect } from '@open-wc/testing';

import {
  THEME_CHANGE_EVENT,
  THEME_STORAGE_KEY,
  ThemeStore,
  type ElohimTheme,
} from './theme-store.js';

describe('ThemeStore', () => {
  beforeEach(() => {
    localStorage.removeItem(THEME_STORAGE_KEY);
    document.body.removeAttribute('data-theme');
    document.body.classList.remove('theme-light', 'theme-dark', 'theme-device');
  });

  it('defaults to device when nothing is persisted', () => {
    const store = new ThemeStore();
    expect(store.theme).to.equal('device');
    expect(document.body.getAttribute('data-theme')).to.equal('device');
    expect(document.body.classList.contains('theme-device')).to.be.true;
  });

  it('loads a persisted valid theme and ignores garbage', () => {
    localStorage.setItem(THEME_STORAGE_KEY, 'dark');
    expect(new ThemeStore().theme).to.equal('dark');
    localStorage.setItem(THEME_STORAGE_KEY, 'neon');
    expect(new ThemeStore().theme).to.equal('device');
  });

  it('cycles device → light → dark → device', () => {
    const store = new ThemeStore();
    store.cycle();
    expect(store.theme).to.equal('light');
    store.cycle();
    expect(store.theme).to.equal('dark');
    store.cycle();
    expect(store.theme).to.equal('device');
  });

  it('set() applies body class + data attribute and persists (the exact ThemeService contract)', () => {
    const store = new ThemeStore();
    store.set('light');
    expect(document.body.getAttribute('data-theme')).to.equal('light');
    expect(document.body.classList.contains('theme-light')).to.be.true;
    expect(document.body.classList.contains('theme-device')).to.be.false;
    expect(localStorage.getItem(THEME_STORAGE_KEY)).to.equal('light');
  });

  it('notifies subscribers and dispatches the change event exactly once per set()', () => {
    const store = new ThemeStore();
    const seen: ElohimTheme[] = [];
    let events = 0;
    const onEvent = (): void => {
      events += 1;
    };
    window.addEventListener(THEME_CHANGE_EVENT, onEvent);
    const unsub = store.subscribe((t) => seen.push(t));
    store.set('dark');
    store.set('dark'); // no-op: same value
    unsub();
    window.removeEventListener(THEME_CHANGE_EVENT, onEvent);
    expect(seen).to.deep.equal(['dark']);
    expect(events).to.equal(1);
  });

  it('adopts an external change event without re-dispatching (no loops)', () => {
    const store = new ThemeStore();
    let events = 0;
    const onEvent = (): void => {
      events += 1;
    };
    window.addEventListener(THEME_CHANGE_EVENT, onEvent);
    window.dispatchEvent(
      new CustomEvent(THEME_CHANGE_EVENT, { detail: { theme: 'dark' } }),
    );
    window.removeEventListener(THEME_CHANGE_EVENT, onEvent);
    expect(store.theme).to.equal('dark');
    expect(events).to.equal(1); // only the one we dispatched ourselves
  });

  it('resolves effectiveTheme for explicit themes', () => {
    const store = new ThemeStore();
    store.set('dark');
    expect(store.effectiveTheme).to.equal('dark');
    store.set('light');
    expect(store.effectiveTheme).to.equal('light');
  });
});
