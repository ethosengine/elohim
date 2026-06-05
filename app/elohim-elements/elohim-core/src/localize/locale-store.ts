/**
 * LocaleStore — reactive device-scoped locale preference.
 *
 * Wraps the lit-localize runtime (runtime.ts): on change it loads the locale
 * bundle via setLocale(), sets document lang/dir (he → rtl), persists to
 * localStorage['elohim-locale'], and syncs across tabs/islands like ThemeStore.
 *
 * First-run default: navigator.language matched against the target locales,
 * else the en source locale. Bundle load failure falls back silently to the
 * en source strings (lit-localize keeps source templates on load error).
 *
 * Classification: Operational (C) — see spec §2. The person-level
 * (imagodei) preference sync is a captured follow-up (spec §9.1).
 */

import { setLocale, sourceLocale, targetLocales } from './runtime.js';

export const LOCALE_STORAGE_KEY = 'elohim-locale';
export const LOCALE_CHANGE_EVENT = 'elohim-locale-changed';

export const SUPPORTED_LOCALES = [sourceLocale, ...targetLocales] as const;
export type ElohimLocale = (typeof SUPPORTED_LOCALES)[number];

/** Native-script labels — intentionally NOT localized (each language names itself). */
export const LOCALE_LABELS: Readonly<Record<ElohimLocale, string>> = {
  en: 'English',
  es: 'Español',
  he: 'עברית',
};

const RTL_LOCALES: ReadonlySet<string> = new Set(['he']);

function isSupported(v: unknown): v is ElohimLocale {
  return typeof v === 'string' && (SUPPORTED_LOCALES as readonly string[]).includes(v);
}

/** navigator.language 'es-MX' → 'es' when supported, else the en source locale. */
export function detectLocale(): ElohimLocale {
  if (typeof navigator !== 'undefined' && navigator.language) {
    const base = navigator.language.toLowerCase().split('-')[0];
    if (isSupported(base)) return base;
  }
  return sourceLocale as ElohimLocale;
}

type Subscriber = (locale: ElohimLocale) => void;

export class LocaleStore {
  private _locale: ElohimLocale;
  private _destroyed = false;
  private subscribers = new Set<Subscriber>();
  private readonly _onStorage: (e: StorageEvent) => void;
  private readonly _onLocaleChange: (e: Event) => void;

  constructor() {
    this._locale = this.load() ?? detectLocale();
    this.applyToDocument(this._locale);
    this._onStorage = (e: StorageEvent): void => {
      if (e.key === LOCALE_STORAGE_KEY && isSupported(e.newValue)) this.adopt(e.newValue);
    };
    this._onLocaleChange = (e: Event): void => {
      const l = (e as CustomEvent<{ locale?: unknown }>).detail?.locale;
      if (isSupported(l)) this.adopt(l);
    };
    if (typeof window !== 'undefined') {
      window.addEventListener('storage', this._onStorage);
      window.addEventListener(LOCALE_CHANGE_EVENT, this._onLocaleChange);
    }
  }

  /**
   * Remove all window listeners and clear subscribers.
   * Call from a web component's disconnectedCallback (or in tests) to avoid
   * listener accumulation on short-lived instances.
   * The module singleton (getLocaleStore()) is intentionally never destroyed.
   */
  destroy(): void {
    this._destroyed = true;
    if (typeof window !== 'undefined') {
      window.removeEventListener('storage', this._onStorage);
      window.removeEventListener(LOCALE_CHANGE_EVENT, this._onLocaleChange);
    }
    this.subscribers.clear();
  }

  get locale(): ElohimLocale {
    return this._locale;
  }

  set(locale: ElohimLocale): void {
    if (this._destroyed || !isSupported(locale) || locale === this._locale) return;
    this._locale = locale;
    this.applyToDocument(locale);
    try {
      localStorage.setItem(LOCALE_STORAGE_KEY, locale);
    } catch {
      // non-critical
    }
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent(LOCALE_CHANGE_EVENT, { detail: { locale } }));
    }
    this.subscribers.forEach((s) => s(locale));
  }

  /** Subscribe to locale changes. Returns an unsubscribe function. */
  subscribe(fn: Subscriber): () => void {
    this.subscribers.add(fn);
    return () => {
      this.subscribers.delete(fn);
    };
  }

  /** External change (other tab/island): apply + notify, never re-persist/dispatch. */
  private adopt(locale: ElohimLocale): void {
    if (this._destroyed || locale === this._locale) return;
    this._locale = locale;
    this.applyToDocument(locale);
    this.subscribers.forEach((s) => s(locale));
  }

  private applyToDocument(locale: ElohimLocale): void {
    if (typeof document !== 'undefined') {
      document.documentElement.lang = locale;
      document.documentElement.dir = RTL_LOCALES.has(locale) ? 'rtl' : 'ltr';
    }
    // Fire-and-forget: lit-localize falls back to source (en) templates on
    // bundle load failure; lang/dir above are already correct either way.
    void setLocale(locale).catch(() => undefined);
  }

  private load(): ElohimLocale | null {
    try {
      const saved = localStorage.getItem(LOCALE_STORAGE_KEY);
      return isSupported(saved) ? saved : null;
    } catch {
      return null;
    }
  }
}

let instance: LocaleStore | null = null;

/** Module-level singleton — all islands in a document share one store. */
export function getLocaleStore(): LocaleStore {
  instance ??= new LocaleStore();
  return instance;
}

/**
 * Reset the module-level singleton — FOR TESTS ONLY.
 *
 * Tests MUST use `new LocaleStore()` directly and never call `getLocaleStore()`,
 * because the singleton is module-scoped and persists across test cases.
 * If a test does call `getLocaleStore()`, call `resetLocaleStoreInstance()` in
 * afterEach to prevent the polluted singleton from leaking into later tests.
 *
 * @internal
 */
export function resetLocaleStoreInstance(): void {
  instance?.destroy();
  instance = null;
}
