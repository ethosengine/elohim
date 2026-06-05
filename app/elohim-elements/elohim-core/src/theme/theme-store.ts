/**
 * ThemeStore — reactive device-scoped theme preference for the current browser.
 *
 * Speaks the EXACT contract of elohim-app's Angular ThemeService so the two
 * implementations can never disagree:
 *   - localStorage key 'elohim-theme'
 *   - body[data-theme="device|light|dark"] + body.theme-{device|light|dark}
 *   - cycle order device → light → dark → device
 *
 * Cross-context sync:
 *   - 'storage' event (other tabs)
 *   - 'elohim-theme-changed' CustomEvent on window (other islands on the same
 *     page — e.g. the Angular ThemeService after its sync patch)
 *
 * Pure TS; no DOM rendering. Sibling of Session (session/session.ts).
 * Classification: Operational (C) — device-local display preference;
 * reconstruction = default 'device' + UA signal. See spec
 * 2026-06-05-omnibar-consolidation-epr-native-links-design.md §2.
 */

export type ElohimTheme = 'device' | 'light' | 'dark';

export const THEME_STORAGE_KEY = 'elohim-theme';
export const THEME_CHANGE_EVENT = 'elohim-theme-changed';

const THEME_CYCLE: readonly ElohimTheme[] = ['device', 'light', 'dark'];

function isTheme(v: unknown): v is ElohimTheme {
  return v === 'device' || v === 'light' || v === 'dark';
}

type Subscriber = (theme: ElohimTheme) => void;

export class ThemeStore {
  private _theme: ElohimTheme = 'device';
  private subscribers = new Set<Subscriber>();
  private readonly _onStorage: (e: StorageEvent) => void;
  private readonly _onThemeChange: (e: Event) => void;

  constructor() {
    this._theme = this.load();
    this.applyToDocument(this._theme);
    this._onStorage = (e: StorageEvent): void => {
      if (e.key === THEME_STORAGE_KEY && isTheme(e.newValue)) this.adopt(e.newValue);
    };
    this._onThemeChange = (e: Event): void => {
      const t = (e as CustomEvent<{ theme?: unknown }>).detail?.theme;
      if (isTheme(t)) this.adopt(t);
    };
    if (typeof window !== 'undefined') {
      window.addEventListener('storage', this._onStorage);
      window.addEventListener(THEME_CHANGE_EVENT, this._onThemeChange);
    }
  }

  /**
   * Remove all window listeners and clear subscribers.
   * Call from a web component's disconnectedCallback (or in tests) to avoid
   * listener accumulation on short-lived instances.
   * The module singleton (getThemeStore()) is intentionally never destroyed.
   */
  destroy(): void {
    if (typeof window !== 'undefined') {
      window.removeEventListener('storage', this._onStorage);
      window.removeEventListener(THEME_CHANGE_EVENT, this._onThemeChange);
    }
    this.subscribers.clear();
  }

  get theme(): ElohimTheme {
    return this._theme;
  }

  /** The theme actually in effect ('device' resolved against the UA signal). */
  get effectiveTheme(): 'light' | 'dark' {
    if (this._theme !== 'device') return this._theme;
    if (typeof window === 'undefined' || !window.matchMedia) return 'light';
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }

  set(theme: ElohimTheme): void {
    if (!isTheme(theme) || theme === this._theme) return;
    this._theme = theme;
    this.applyToDocument(theme);
    this.persist(theme);
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent(THEME_CHANGE_EVENT, { detail: { theme } }));
    }
    this.subscribers.forEach((s) => s(theme));
  }

  cycle(): void {
    const i = THEME_CYCLE.indexOf(this._theme);
    this.set(THEME_CYCLE[(i + 1) % THEME_CYCLE.length] as ElohimTheme);
  }

  /** Subscribe to theme changes. Returns an unsubscribe function. */
  subscribe(fn: Subscriber): () => void {
    this.subscribers.add(fn);
    return () => {
      this.subscribers.delete(fn);
    };
  }

  /** External change (other tab/island): apply + notify, never re-persist/dispatch. */
  private adopt(theme: ElohimTheme): void {
    if (theme === this._theme) return;
    this._theme = theme;
    this.applyToDocument(theme);
    this.subscribers.forEach((s) => s(theme));
  }

  private applyToDocument(theme: ElohimTheme): void {
    if (typeof document === 'undefined') return;
    const body = document.body;
    body.classList.remove('theme-light', 'theme-dark', 'theme-device');
    body.classList.add(`theme-${theme}`);
    body.setAttribute('data-theme', theme);
  }

  private persist(theme: ElohimTheme): void {
    try {
      localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch {
      // localStorage write failure is non-critical (matches ThemeService posture)
    }
  }

  private load(): ElohimTheme {
    try {
      const saved = localStorage.getItem(THEME_STORAGE_KEY);
      return isTheme(saved) ? saved : 'device';
    } catch {
      return 'device';
    }
  }
}

let instance: ThemeStore | null = null;

/** Module-level singleton — all islands in a document share one store. */
export function getThemeStore(): ThemeStore {
  instance ??= new ThemeStore();
  return instance;
}

/**
 * Reset the module-level singleton — FOR TESTS ONLY.
 *
 * Tests MUST use `new ThemeStore()` directly and never call `getThemeStore()`,
 * because the singleton is module-scoped and persists across test cases.
 * If a test does call `getThemeStore()`, call `resetThemeStoreInstance()` in
 * afterEach to prevent the polluted singleton from leaking into later tests.
 *
 * @internal
 */
export function resetThemeStoreInstance(): void {
  instance?.destroy();
  instance = null;
}
