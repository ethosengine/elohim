import { expect } from '@open-wc/testing';

import { clearMediaQueries, setMediaQuery } from '../testing/ua-prefs.js';
import {
  THEME_CHANGE_EVENT,
  THEME_STORAGE_KEY,
  ThemeStore,
  getThemeStore,
  resetThemeStoreInstance,
  type ElohimTheme,
} from './theme-store.js';

// NOTE: Tests use `new ThemeStore()` directly and do NOT call `getThemeStore()`
// in the main suite. The singleton is module-scoped and persists across test
// cases — calling `getThemeStore()` without a matching `resetThemeStoreInstance()`
// in afterEach would pollute subsequent tests. The singleton test below uses
// `resetThemeStoreInstance()` in afterEach to prevent that leakage.

describe('ThemeStore', () => {
  let store: ThemeStore | undefined;
  const extraStores: ThemeStore[] = [];

  beforeEach(() => {
    localStorage.removeItem(THEME_STORAGE_KEY);
    document.body.removeAttribute('data-theme');
    document.body.classList.remove('theme-light', 'theme-dark', 'theme-device');
  });

  afterEach(() => {
    store?.destroy();
    store = undefined;
    for (const s of extraStores) {
      s.destroy();
    }
    extraStores.length = 0;
  });

  it('defaults to device when nothing is persisted', () => {
    store = new ThemeStore();
    expect(store.theme).to.equal('device');
    expect(document.body.getAttribute('data-theme')).to.equal('device');
    expect(document.body.classList.contains('theme-device')).to.be.true;
  });

  it('loads a persisted valid theme and ignores garbage', () => {
    localStorage.setItem(THEME_STORAGE_KEY, 'dark');
    const s1 = new ThemeStore();
    extraStores.push(s1);
    expect(s1.theme).to.equal('dark');
    localStorage.setItem(THEME_STORAGE_KEY, 'neon');
    const s2 = new ThemeStore();
    extraStores.push(s2);
    expect(s2.theme).to.equal('device');
  });

  it('cycles device → light → dark → device', () => {
    store = new ThemeStore();
    store.cycle();
    expect(store.theme).to.equal('light');
    store.cycle();
    expect(store.theme).to.equal('dark');
    store.cycle();
    expect(store.theme).to.equal('device');
  });

  it('set() applies body class + data attribute and persists (the exact ThemeService contract)', () => {
    store = new ThemeStore();
    store.set('light');
    expect(document.body.getAttribute('data-theme')).to.equal('light');
    expect(document.body.classList.contains('theme-light')).to.be.true;
    expect(document.body.classList.contains('theme-device')).to.be.false;
    expect(localStorage.getItem(THEME_STORAGE_KEY)).to.equal('light');
  });

  it('notifies subscribers and dispatches the change event exactly once per set()', () => {
    store = new ThemeStore();
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
    store = new ThemeStore();
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
    store = new ThemeStore();
    store.set('dark');
    expect(store.effectiveTheme).to.equal('dark');
    store.set('light');
    expect(store.effectiveTheme).to.equal('light');
  });

  it('destroy() removes window listeners so a destroyed instance stops reacting', () => {
    store = new ThemeStore();
    store.destroy();
    // After destroy, dispatching an external change event must NOT update the store
    window.dispatchEvent(
      new CustomEvent(THEME_CHANGE_EVENT, { detail: { theme: 'dark' } }),
    );
    expect(store.theme).to.equal('device'); // unchanged
    // store is already destroyed; afterEach calling destroy() again is safe (idempotent)
  });

  it('destroy() guards set() — calling set() after destroy() leaves theme and document unchanged', () => {
    store = new ThemeStore();
    store.set('light'); // confirmed live before destroy
    expect(store.theme).to.equal('light');
    store.destroy();
    store.set('dark'); // must be a no-op
    expect(store.theme).to.equal('light'); // store.theme unchanged
    expect(document.body.getAttribute('data-theme')).to.equal('light'); // document unchanged
    // store is already destroyed; afterEach calling destroy() again is safe (idempotent)
  });

  describe('effectiveTheme — device branch', () => {
    afterEach(() => {
      clearMediaQueries();
    });

    it("resolves 'dark' when prefers-color-scheme is dark", () => {
      setMediaQuery('prefers-color-scheme', 'dark');
      store = new ThemeStore(); // default theme = 'device'
      expect(store.theme).to.equal('device');
      expect(store.effectiveTheme).to.equal('dark');
    });

    it("resolves 'light' when prefers-color-scheme is light", () => {
      setMediaQuery('prefers-color-scheme', 'light');
      store = new ThemeStore();
      expect(store.theme).to.equal('device');
      expect(store.effectiveTheme).to.equal('light');
    });

    it('effectiveTheme returns the explicit value (not matchMedia) when theme is not device', () => {
      setMediaQuery('prefers-color-scheme', 'dark');
      store = new ThemeStore();
      store.set('light');
      expect(store.effectiveTheme).to.equal('light'); // explicit wins; matchMedia says dark
    });
  });
});

describe('getThemeStore singleton', () => {
  afterEach(() => {
    // Always reset so the singleton does not pollute tests that follow
    resetThemeStoreInstance();
    localStorage.removeItem(THEME_STORAGE_KEY);
    document.body.removeAttribute('data-theme');
    document.body.classList.remove('theme-light', 'theme-dark', 'theme-device');
  });

  it('returns the same instance on repeated calls (identity contract)', () => {
    const a = getThemeStore();
    const b = getThemeStore();
    expect(a).to.equal(b);
  });

  it('returns a fresh instance after resetThemeStoreInstance()', () => {
    const a = getThemeStore();
    resetThemeStoreInstance();
    const b = getThemeStore();
    expect(a).to.not.equal(b);
  });
});
