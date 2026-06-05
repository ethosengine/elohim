# Omnibar Consolidation + EPR-Native Links — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Heal the cross-bundle `routerLink` 404 regression, lift the debug-bar into an opt-in trust-framed ServingContext segment on protocol-omni, restore lamad's theme toggle via a shared elohim-core theme store, and add an opt-in language picker — per spec `genesis/docs/superpowers/specs/2026-06-05-omnibar-consolidation-epr-native-links-design.md`.

**Architecture:** New elohim-core primitives (ThemeStore, LocaleStore, `<elohim-theme-toggle>`, `<elohim-lang-picker>`, epr-link-interceptor) consumed by two surfaces: the elohim-app Angular shell (protocol-omni, EprNavService, link sweep) and the lamad bundle (page-chrome auto-install, navigator restore). Zero backend changes.

**Tech Stack:** Lit 3 + @open-wc/testing (wtr) + @lit/localize in elohim-core; Angular 19 signals + Vitest in elohim-app; Cucumber a2o in genesis/a2o.

**Branch + hygiene:** Work lands on `shift/a2o-greenup` (shared worktree — SELECTIVE `git add` of exactly the listed files; never `git add -A`). Commit per task; **never push** (integrator owns push/merge). No cargo involved — pure TS/Angular, no CARGO_TARGET_DIR concerns.

**Key commands:**
- elohim-core suite: `cd /projects/elohim/app/elohim-elements/elohim-core && pnpm test` (web-test-runner, whole suite)
- elohim-core gates: `pnpm lint && pnpm lint:css && pnpm typecheck && pnpm build` (build regenerates `custom-elements.json` via cem)
- elohim-app single spec: `cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts <path>`
- a2o undefined-step gate: `cd /projects/elohim/genesis/a2o && npx cucumber-js --dry-run`

---

## File structure (locked decisions)

```
app/elohim-elements/elohim-core/src/
  theme/theme-store.ts            (NEW)  device-scoped theme preference store — exact ThemeService contract
  theme/theme-store.spec.ts       (NEW)
  localize/locale-store.ts        (NEW)  locale preference store — wraps lit-localize runtime, lang/dir, persistence
  localize/locale-store.spec.ts   (NEW)
  elohim-theme-toggle.ts          (NEW)  cycle button element
  elohim-theme-toggle.spec.ts     (NEW)
  elohim-lang-picker.ts           (NEW)  select-based locale picker element
  elohim-lang-picker.spec.ts      (NEW)
  navigation/epr-link-interceptor.ts      (NEW)  capture-phase cross-bundle anchor interceptor
  navigation/epr-link-interceptor.spec.ts (NEW)
  elohim-page-chrome.ts           (MOD)  auto-install interceptor
  elohim-default-omnibar.ts       (MOD)  show-theme-toggle / show-lang-picker attrs
  elohim-navigator.ts             (MOD)  tray theme/lang rows + visitor inline toggle
  index.ts / register.ts          (MOD)  exports + definitions

app/elohim-library/projects/graphos/src/default/core/__docs__/
  elohim-theme-toggle.default.stories.ts  (NEW)
  elohim-lang-picker.default.stories.ts   (NEW)

app/elohim-app/src/app/
  elohim/models/serving-context.model.ts  (NEW)
  elohim/services/epr-nav.service.ts      (NEW) + .spec.ts
  services/config.service.ts              (MOD)  AppConfig.gitHash
  services/theme.service.ts               (MOD)  cross-island sync listeners
  elohim/components/protocol-omni/*       (MOD)  serving-context segment + theme opt-in
  components/debug-bar/*                  (DEL)
  components/home/*                       (MOD)  remove debug-bar usage
  components/footer/*                     (MOD)  sweep
  components/not-found/*                  (MOD)  sweep
  imagodei/components/profile/*           (MOD)  sweep
  imagodei/services/tauri-auth.service.ts (MOD)  sweep
  elohim/services/elohim-presence.service.ts (MOD) sweep
  app.component.{ts,html}                 (MOD)  interceptor install + showEnvContext
  app.routes.spec.ts                      (MOD)  contract comment refresh

app/lamad/src/app/components/lamad-layout/*  (MOD)  footer link + navigate wiring

genesis/a2o/features/
  protocol/protocol-omni.feature          (MOD)  serving-context scenario
  browser/navigation-browser.feature      (MOD)  footer cross-bundle scenario
  elohim-core/chrome-preferences.feature  (NEW)  theme/RTL spine
```

---

### Task 1: ThemeStore (elohim-core)

**Files:**
- Create: `app/elohim-elements/elohim-core/src/theme/theme-store.ts`
- Create: `app/elohim-elements/elohim-core/src/theme/theme-store.spec.ts`
- Modify: `app/elohim-elements/elohim-core/src/index.ts`

- [ ] **Step 1.1: Write the failing test**

`app/elohim-elements/elohim-core/src/theme/theme-store.spec.ts`:

```ts
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
```

- [ ] **Step 1.2: Run, verify it fails**

Run: `cd /projects/elohim/app/elohim-elements/elohim-core && pnpm test`
Expected: FAIL — cannot resolve `./theme-store.js`.

- [ ] **Step 1.3: Implement**

`app/elohim-elements/elohim-core/src/theme/theme-store.ts`:

```ts
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

  constructor() {
    this._theme = this.load();
    this.applyToDocument(this._theme);
    if (typeof window !== 'undefined') {
      window.addEventListener('storage', (e: StorageEvent) => {
        if (e.key === THEME_STORAGE_KEY && isTheme(e.newValue)) this.adopt(e.newValue);
      });
      window.addEventListener(THEME_CHANGE_EVENT, (e: Event) => {
        const t = (e as CustomEvent<{ theme?: unknown }>).detail?.theme;
        if (isTheme(t)) this.adopt(t);
      });
    }
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
```

Append to `app/elohim-elements/elohim-core/src/index.ts` (alongside the existing Session export block):

```ts
export { ThemeStore, getThemeStore, THEME_STORAGE_KEY, THEME_CHANGE_EVENT } from './theme/theme-store.js';
export type { ElohimTheme } from './theme/theme-store.js';
```

- [ ] **Step 1.4: Run, verify pass**

Run: `cd /projects/elohim/app/elohim-elements/elohim-core && pnpm test`
Expected: PASS (whole suite green, 7 new tests).

- [ ] **Step 1.5: Commit**

```bash
cd /projects/elohim
git add app/elohim-elements/elohim-core/src/theme/theme-store.ts app/elohim-elements/elohim-core/src/theme/theme-store.spec.ts app/elohim-elements/elohim-core/src/index.ts
git commit -m "feat(elohim-core): ThemeStore — device-scoped theme preference on the exact ThemeService contract"
```

---

### Task 2: LocaleStore (elohim-core)

**Files:**
- Create: `app/elohim-elements/elohim-core/src/localize/locale-store.ts`
- Create: `app/elohim-elements/elohim-core/src/localize/locale-store.spec.ts`
- Modify: `app/elohim-elements/elohim-core/src/index.ts`

- [ ] **Step 2.1: Write the failing test**

`app/elohim-elements/elohim-core/src/localize/locale-store.spec.ts`:

```ts
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
```

- [ ] **Step 2.2: Run, verify it fails** (`pnpm test` — cannot resolve `./locale-store.js`)

- [ ] **Step 2.3: Implement**

`app/elohim-elements/elohim-core/src/localize/locale-store.ts`:

```ts
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
  private subscribers = new Set<Subscriber>();

  constructor() {
    this._locale = this.load() ?? detectLocale();
    this.applyToDocument(this._locale);
    if (typeof window !== 'undefined') {
      window.addEventListener('storage', (e: StorageEvent) => {
        if (e.key === LOCALE_STORAGE_KEY && isSupported(e.newValue)) this.adopt(e.newValue);
      });
      window.addEventListener(LOCALE_CHANGE_EVENT, (e: Event) => {
        const l = (e as CustomEvent<{ locale?: unknown }>).detail?.locale;
        if (isSupported(l)) this.adopt(l);
      });
    }
  }

  get locale(): ElohimLocale {
    return this._locale;
  }

  set(locale: ElohimLocale): void {
    if (!isSupported(locale) || locale === this._locale) return;
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

  private adopt(locale: ElohimLocale): void {
    if (locale === this._locale) return;
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
```

Append to `index.ts`:

```ts
export {
  LocaleStore,
  getLocaleStore,
  detectLocale,
  SUPPORTED_LOCALES,
  LOCALE_LABELS,
  LOCALE_STORAGE_KEY,
  LOCALE_CHANGE_EVENT,
} from './localize/locale-store.js';
export type { ElohimLocale } from './localize/locale-store.js';
```

- [ ] **Step 2.4: Run, verify pass** (`pnpm test`)

- [ ] **Step 2.5: Commit**

```bash
git add app/elohim-elements/elohim-core/src/localize/locale-store.ts app/elohim-elements/elohim-core/src/localize/locale-store.spec.ts app/elohim-elements/elohim-core/src/index.ts
git commit -m "feat(elohim-core): LocaleStore — device-scoped locale preference wrapping the lit-localize runtime"
```

---

### Task 3: `<elohim-theme-toggle>` element + Library A story

**Files:**
- Create: `app/elohim-elements/elohim-core/src/elohim-theme-toggle.ts`
- Create: `app/elohim-elements/elohim-core/src/elohim-theme-toggle.spec.ts`
- Modify: `app/elohim-elements/elohim-core/src/index.ts`, `src/register.ts`
- Create: `app/elohim-library/projects/graphos/src/default/core/__docs__/elohim-theme-toggle.default.stories.ts`

- [ ] **Step 3.1: Write the failing test**

`app/elohim-elements/elohim-core/src/elohim-theme-toggle.spec.ts`:

```ts
import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import { ElohimThemeToggle as ToggleClass } from './elohim-theme-toggle.js';
import { THEME_STORAGE_KEY, getThemeStore } from './theme/theme-store.js';
import { requiresLogicalProperties } from './testing/i18n.js';

describe('<elohim-theme-toggle>', () => {
  beforeEach(() => {
    localStorage.removeItem(THEME_STORAGE_KEY);
    getThemeStore().set('device');
  });

  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-theme-toggle')).to.equal(ToggleClass);
  });

  it('renders a labelled button with the auto indicator in device mode', async () => {
    const el = await fixture<ElohimThemeToggle>(html`<elohim-theme-toggle></elohim-theme-toggle>`);
    const btn = el.shadowRoot!.querySelector('button[part="button"]');
    expect(btn).to.exist;
    expect(btn!.getAttribute('aria-label')).to.be.a('string').and.not.empty;
    expect(el.shadowRoot!.querySelector('[part="auto-indicator"]')).to.exist;
  });

  it('click cycles the shared store device → light and hides the auto indicator', async () => {
    const el = await fixture<ElohimThemeToggle>(html`<elohim-theme-toggle></elohim-theme-toggle>`);
    el.shadowRoot!.querySelector<HTMLButtonElement>('button')!.click();
    await elementUpdated(el);
    expect(getThemeStore().theme).to.equal('light');
    expect(document.body.getAttribute('data-theme')).to.equal('light');
    expect(el.shadowRoot!.querySelector('[part="auto-indicator"]')).to.not.exist;
  });

  it('follows external store changes (two toggles stay in sync)', async () => {
    const el = await fixture<ElohimThemeToggle>(html`<elohim-theme-toggle></elohim-theme-toggle>`);
    getThemeStore().set('dark');
    await elementUpdated(el);
    expect(el.shadowRoot!.querySelector('[part="icon"]')!.textContent).to.contain('🌙');
  });

  it('dispatches theme-changed with the new theme', async () => {
    const el = await fixture<ElohimThemeToggle>(html`<elohim-theme-toggle></elohim-theme-toggle>`);
    let detail: { theme?: string } | null = null;
    el.addEventListener('theme-changed', (e) => {
      detail = (e as CustomEvent<{ theme: string }>).detail;
    });
    el.shadowRoot!.querySelector<HTMLButtonElement>('button')!.click();
    expect(detail).to.deep.equal({ theme: 'light' });
  });

  it('passes the a11y gate (axe)', async () => {
    const el = await fixture<ElohimThemeToggle>(html`<elohim-theme-toggle></elohim-theme-toggle>`);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.be.empty;
  });

  it('passes the i18n gate (logical properties only)', () => {
    expect(requiresLogicalProperties(ToggleClass.styles)).to.be.true;
  });

  it('passes the ua-prefs gate (no transitions declared)', () => {
    const cssText = String(ToggleClass.styles);
    expect(cssText).to.not.contain('transition');
  });
});

import type { ElohimThemeToggle } from './elohim-theme-toggle.js';
```

> NOTE: if `requiresLogicalProperties` in `testing/i18n.ts` takes a different argument shape (check its signature at `src/testing/i18n.ts:130` before writing), mirror how `elohim-page-chrome.spec.ts` calls it.

- [ ] **Step 3.2: Run, verify it fails** (`pnpm test` — element not defined)

- [ ] **Step 3.3: Implement the element**

`app/elohim-elements/elohim-core/src/elohim-theme-toggle.ts`:

```ts
import { msg, updateWhenLocaleChanges } from '@lit/localize';
import { css, html, LitElement, nothing } from 'lit';
import { state } from 'lit/decorators.js';

import { getThemeStore, type ElohimTheme } from './theme/theme-store.js';

/**
 * <elohim-theme-toggle> — cycles the device-scoped theme preference
 * (device → light → dark) through the shared ThemeStore. Pure
 * sense-and-respond: renders current state, forwards the user's intent to
 * the store; the theming itself happens via body[data-theme] + CSS cascade.
 *
 * @element elohim-theme-toggle
 *
 * @fires theme-changed - detail: { theme } after the user cycles
 *
 * @cssprop --elohim-theme-toggle-bg       - button background (default: transparent)
 * @cssprop --elohim-theme-toggle-fg       - button foreground (default: inherit)
 * @cssprop --elohim-theme-toggle-border   - button border color (default: transparent)
 * @cssprop --elohim-theme-toggle-badge-bg - auto badge background (default: ButtonFace)
 * @cssprop --elohim-theme-toggle-badge-fg - auto badge foreground (default: ButtonText)
 *
 * @csspart button         - the toggle button
 * @csspart icon           - the sun/moon glyph
 * @csspart auto-indicator - the "A" badge shown in device (auto) mode
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a, unauthorized:n/a
 */
export class ElohimThemeToggle extends LitElement {
  private _store = getThemeStore();

  @state() private _theme: ElohimTheme = this._store.theme;

  private _unsub?: () => void;

  constructor() {
    super();
    updateWhenLocaleChanges(this);
  }

  static override readonly styles = css`
    :host {
      display: inline-flex;
    }

    :host([hidden]) {
      display: none;
    }

    button {
      position: relative;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 0.15rem;
      min-inline-size: 44px;
      min-block-size: 44px;
      padding: 0.25rem;
      background: var(--elohim-theme-toggle-bg, transparent);
      color: var(--elohim-theme-toggle-fg, inherit);
      border: 1px solid var(--elohim-theme-toggle-border, transparent);
      border-radius: 999px;
      font: inherit;
      cursor: pointer;
    }

    button:focus-visible {
      outline: 2px solid Highlight;
      outline-offset: 2px;
    }

    [part='auto-indicator'] {
      font-size: 0.6em;
      font-weight: 700;
      padding-block: 0;
      padding-inline: 0.3em;
      border-radius: 999px;
      background: var(--elohim-theme-toggle-badge-bg, ButtonFace);
      color: var(--elohim-theme-toggle-badge-fg, ButtonText);
    }

    @media (forced-colors: active) {
      button {
        border-color: ButtonText;
        color: ButtonText;
      }
    }
  `;

  override connectedCallback(): void {
    super.connectedCallback();
    this._unsub = this._store.subscribe((t) => {
      this._theme = t;
    });
    this._theme = this._store.theme;
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this._unsub?.();
  }

  private _label(): string {
    const names: Record<ElohimTheme, string> = {
      device: msg('Theme: follow device — click for light'),
      light: msg('Theme: light — click for dark'),
      dark: msg('Theme: dark — click to follow device'),
    };
    return names[this._theme];
  }

  private _cycle(): void {
    this._store.cycle();
    this.dispatchEvent(
      new CustomEvent('theme-changed', {
        detail: { theme: this._store.theme },
        bubbles: true,
        composed: true,
      }),
    );
  }

  override render() {
    const label = this._label();
    return html`
      <button part="button" type="button" aria-label=${label} title=${label} @click=${this._cycle}>
        <span part="icon" aria-hidden="true">${this._store.effectiveTheme === 'dark' ? '🌙' : '☀️'}</span>
        ${this._theme === 'device'
          ? html`<span part="auto-indicator" aria-hidden="true">A</span>`
          : nothing}
      </button>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-theme-toggle': ElohimThemeToggle;
  }
}
```

`register.ts` — add (same guard idiom as the others):

```ts
import { ElohimThemeToggle } from './elohim-theme-toggle.js';

if (!customElements.get('elohim-theme-toggle')) {
  customElements.define('elohim-theme-toggle', ElohimThemeToggle);
}
```

`index.ts` — add:

```ts
export { ElohimThemeToggle } from './elohim-theme-toggle.js';
```

- [ ] **Step 3.4: Run, verify pass** (`pnpm test`)

- [ ] **Step 3.5: Library A default story**

`app/elohim-library/projects/graphos/src/default/core/__docs__/elohim-theme-toggle.default.stories.ts`:

```ts
/**
 * Library A default story — elohim-theme-toggle
 *
 * Blank-slate proof for the theme cycle button. The element drives the shared
 * ThemeStore (localStorage 'elohim-theme' + body[data-theme]) — the same
 * contract the Angular ThemeService speaks, so any toggle anywhere stays in sync.
 */

import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';

import 'elohim-core/register';

const meta: Meta = {
  title: 'Default/Core/elohim-theme-toggle',
  parameters: {
    docs: {
      description: {
        component: `
\`<elohim-theme-toggle>\` — cycles device → light → dark through the shared ThemeStore.

**Override surface:** \`--elohim-theme-toggle-bg\`, \`--elohim-theme-toggle-fg\`,
\`--elohim-theme-toggle-border\`, \`--elohim-theme-toggle-badge-bg\`, \`--elohim-theme-toggle-badge-fg\`.
**Parts:** \`button\`, \`icon\`, \`auto-indicator\`. **Events:** \`theme-changed\`.
        `.trim(),
      },
    },
  },
};

export default meta;
type Story = StoryObj;

export const Default: Story = {
  render: () => html`<elohim-theme-toggle></elohim-theme-toggle>`,
};

export const Dark: Story = {
  render: () => html`
    <div style="background:#101218;color:#e8eaf0;padding:2rem;">
      <elohim-theme-toggle></elohim-theme-toggle>
    </div>
  `,
};

export const Unstyled: Story = {
  name: 'Unstyled (blank-slate proof)',
  render: () => html`
    <div style="all: initial;">
      <elohim-theme-toggle></elohim-theme-toggle>
    </div>
  `,
};

export const CustomTheme: Story = {
  name: 'CustomTheme (override-surface proof)',
  render: () => html`
    <div
      style="
        --elohim-theme-toggle-bg: #062a06;
        --elohim-theme-toggle-fg: #6cff6c;
        --elohim-theme-toggle-border: #0c4d0c;
        --elohim-theme-toggle-badge-bg: #0c4d0c;
        --elohim-theme-toggle-badge-fg: #6cff6c;
        padding: 2rem;
        background: #021202;
      "
    >
      <elohim-theme-toggle></elohim-theme-toggle>
    </div>
  `,
};
```

- [ ] **Step 3.6: Commit**

```bash
git add app/elohim-elements/elohim-core/src/elohim-theme-toggle.ts app/elohim-elements/elohim-core/src/elohim-theme-toggle.spec.ts app/elohim-elements/elohim-core/src/index.ts app/elohim-elements/elohim-core/src/register.ts "app/elohim-library/projects/graphos/src/default/core/__docs__/elohim-theme-toggle.default.stories.ts"
git commit -m "feat(elohim-core): <elohim-theme-toggle> blank-slate cycle button + Library A story"
```

---

### Task 4: `<elohim-lang-picker>` element + Library A story

**Files:**
- Create: `app/elohim-elements/elohim-core/src/elohim-lang-picker.ts`, `.spec.ts`
- Modify: `index.ts`, `register.ts`
- Create: `app/elohim-library/projects/graphos/src/default/core/__docs__/elohim-lang-picker.default.stories.ts`

- [ ] **Step 4.1: Write the failing test**

`app/elohim-elements/elohim-core/src/elohim-lang-picker.spec.ts`:

```ts
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
```

- [ ] **Step 4.2: Run, verify it fails**

- [ ] **Step 4.3: Implement**

`app/elohim-elements/elohim-core/src/elohim-lang-picker.ts`:

```ts
import { msg, updateWhenLocaleChanges } from '@lit/localize';
import { css, html, LitElement } from 'lit';
import { state } from 'lit/decorators.js';

import {
  LOCALE_LABELS,
  SUPPORTED_LOCALES,
  getLocaleStore,
  type ElohimLocale,
} from './localize/locale-store.js';

/**
 * <elohim-lang-picker> — per-site language override the web platform doesn't
 * provide natively. Drives the shared LocaleStore (lit-localize setLocale +
 * document lang/dir + localStorage persistence). Native <select> for
 * built-in keyboard + screen-reader semantics.
 *
 * @element elohim-lang-picker
 *
 * @fires locale-changed - detail: { locale } after the user selects
 *
 * @cssprop --elohim-lang-picker-bg     - select background (default: transparent)
 * @cssprop --elohim-lang-picker-fg     - select foreground (default: inherit)
 * @cssprop --elohim-lang-picker-border - select border color
 *
 * @csspart label  - the (visually hidden) label wrapper
 * @csspart select - the locale select
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a, unauthorized:n/a
 */
export class ElohimLangPicker extends LitElement {
  private _store = getLocaleStore();

  @state() private _locale: ElohimLocale = this._store.locale;

  private _unsub?: () => void;

  constructor() {
    super();
    updateWhenLocaleChanges(this);
  }

  static override readonly styles = css`
    :host {
      display: inline-flex;
    }

    :host([hidden]) {
      display: none;
    }

    select {
      font: inherit;
      color: var(--elohim-lang-picker-fg, inherit);
      background: var(--elohim-lang-picker-bg, transparent);
      border: 1px solid var(--elohim-lang-picker-border, color-mix(in oklch, currentColor 25%, transparent));
      border-radius: 0.35rem;
      padding-block: 0.15rem;
      padding-inline: 0.35rem;
      min-block-size: 2rem;
      cursor: pointer;
    }

    select:focus-visible {
      outline: 2px solid Highlight;
      outline-offset: 2px;
    }

    .visually-hidden {
      position: absolute;
      inline-size: 1px;
      block-size: 1px;
      overflow: hidden;
      clip-path: inset(50%);
      white-space: nowrap;
    }

    @media (forced-colors: active) {
      select {
        border-color: ButtonText;
        color: ButtonText;
        background: ButtonFace;
      }
    }
  `;

  override connectedCallback(): void {
    super.connectedCallback();
    this._unsub = this._store.subscribe((l) => {
      this._locale = l;
    });
    this._locale = this._store.locale;
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this._unsub?.();
  }

  private _onChange(e: Event): void {
    const value = (e.target as HTMLSelectElement).value as ElohimLocale;
    this._store.set(value);
    this.dispatchEvent(
      new CustomEvent('locale-changed', {
        detail: { locale: value },
        bubbles: true,
        composed: true,
      }),
    );
  }

  override render() {
    return html`
      <label part="label">
        <span class="visually-hidden">${msg('Language')}</span>
        <select part="select" @change=${this._onChange} .value=${this._locale}>
          ${SUPPORTED_LOCALES.map(
            (l) => html`
              <option value=${l} ?selected=${l === this._locale} lang=${l}>
                ${LOCALE_LABELS[l]}
              </option>
            `,
          )}
        </select>
      </label>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-lang-picker': ElohimLangPicker;
  }
}
```

`register.ts` / `index.ts` — same idiom as Task 3 (`elohim-lang-picker` / `ElohimLangPicker`).

- [ ] **Step 4.4: Run, verify pass** (`pnpm test`)

- [ ] **Step 4.5: Library A story** — `elohim-lang-picker.default.stories.ts` mirroring Task 3.5: meta title `'Default/Core/elohim-lang-picker'`, stories `Default`, `Dark`, `RTLCanary` (decorator sets `document.documentElement.dir = 'rtl'` before render and restores after, matching the existing RTLCanary idiom in `elohim-default-omnibar.default.stories.ts:173-191`), `Unstyled`, `CustomTheme` (override the three `--elohim-lang-picker-*` props).

- [ ] **Step 4.6: Commit**

```bash
git add app/elohim-elements/elohim-core/src/elohim-lang-picker.ts app/elohim-elements/elohim-core/src/elohim-lang-picker.spec.ts app/elohim-elements/elohim-core/src/index.ts app/elohim-elements/elohim-core/src/register.ts "app/elohim-library/projects/graphos/src/default/core/__docs__/elohim-lang-picker.default.stories.ts"
git commit -m "feat(elohim-core): <elohim-lang-picker> native-select locale override + Library A story"
```

---

### Task 5: epr-link-interceptor (elohim-core)

**Files:**
- Create: `app/elohim-elements/elohim-core/src/navigation/epr-link-interceptor.ts`, `.spec.ts`
- Modify: `index.ts`
- Modify: `genesis/docs/superpowers/specs/2026-06-05-omnibar-consolidation-epr-native-links-design.md` (§4.2 refinement — see Step 5.6)

- [ ] **Step 5.1: Write the failing test**

`app/elohim-elements/elohim-core/src/navigation/epr-link-interceptor.spec.ts`:

```ts
import { expect, fixture, html } from '@open-wc/testing';

import {
  baseHrefOwnsPath,
  installEprLinkInterceptor,
  recordCrossBundleHandoff,
} from './epr-link-interceptor.js';

const NAV_STACK_KEY = 'elohim.session-nav-stack.v1';

describe('epr-link-interceptor', () => {
  let uninstall: (() => void) | null = null;
  let assigned: string[] = [];

  function install(opts: Parameters<typeof installEprLinkInterceptor>[0] = {}): void {
    uninstall = installEprLinkInterceptor({
      ...opts,
      // test seam: capture instead of really navigating
      assign: (href: string) => assigned.push(href),
    });
  }

  afterEach(() => {
    uninstall?.();
    uninstall = null;
    assigned = [];
    sessionStorage.removeItem(NAV_STACK_KEY);
  });

  async function clickAnchor(href: string, mod: Partial<MouseEventInit> = {}): Promise<MouseEvent> {
    const a = await fixture<HTMLAnchorElement>(html`<a href=${href}>link</a>`);
    const ev = new MouseEvent('click', { bubbles: true, cancelable: true, composed: true, ...mod });
    a.dispatchEvent(ev);
    return ev;
  }

  it('intercepts a cross-bundle anchor: preventDefault + assign + handoff record', async () => {
    install({ ownsPath: () => false });
    const ev = await clickAnchor('/lamad');
    expect(ev.defaultPrevented).to.be.true;
    expect(assigned).to.deep.equal(['/lamad']);
    const stack = JSON.parse(sessionStorage.getItem(NAV_STACK_KEY) ?? '[]') as unknown[];
    expect(stack).to.have.length(1);
  });

  it('passes through same-bundle anchors untouched', async () => {
    install({ ownsPath: () => true });
    const ev = await clickAnchor('/community');
    expect(ev.defaultPrevented).to.be.false;
    expect(assigned).to.be.empty;
  });

  it('passes through modified clicks, _blank targets, downloads, hash links, bypass-marked', async () => {
    install({ ownsPath: () => false });
    expect((await clickAnchor('/lamad', { ctrlKey: true })).defaultPrevented).to.be.false;
    expect((await clickAnchor('/lamad', { metaKey: true })).defaultPrevented).to.be.false;
    const blank = await fixture<HTMLAnchorElement>(html`<a href="/lamad" target="_blank">x</a>`);
    const evBlank = new MouseEvent('click', { bubbles: true, cancelable: true });
    blank.dispatchEvent(evBlank);
    expect(evBlank.defaultPrevented).to.be.false;
    const dl = await fixture<HTMLAnchorElement>(html`<a href="/lamad" download>x</a>`);
    const evDl = new MouseEvent('click', { bubbles: true, cancelable: true });
    dl.dispatchEvent(evDl);
    expect(evDl.defaultPrevented).to.be.false;
    expect((await clickAnchor('#frag')).defaultPrevented).to.be.false;
    const bypass = await fixture<HTMLAnchorElement>(html`<a href="/lamad" data-epr-bypass>x</a>`);
    const evBy = new MouseEvent('click', { bubbles: true, cancelable: true });
    bypass.dispatchEvent(evBy);
    expect(evBy.defaultPrevented).to.be.false;
    expect(assigned).to.be.empty;
  });

  it('calls beforeCrossBundle instead of the default handoff when provided', async () => {
    let called: string | null = null;
    install({ ownsPath: () => false, beforeCrossBundle: (href) => (called = href) });
    await clickAnchor('/lamad?x=1');
    expect(called).to.equal('/lamad?x=1');
    expect(sessionStorage.getItem(NAV_STACK_KEY)).to.be.null;
  });

  it('explicit install replaces a default install; default never replaces', () => {
    const u1 = installEprLinkInterceptor({ assign: () => undefined });
    const u2 = installEprLinkInterceptor({ assign: () => undefined }); // default vs existing → no-op handle
    const u3 = installEprLinkInterceptor({ explicit: true, ownsPath: () => true, assign: () => undefined });
    u2(); // must be safe and must NOT remove the active explicit install
    u3();
    u1();
    expect(window.__elohimEprLinkInterceptor).to.be.undefined;
  });

  it('recordCrossBundleHandoff caps the stack at 32 entries', () => {
    for (let i = 0; i < 40; i++) recordCrossBundleHandoff(`cid-${i}`);
    const stack = JSON.parse(sessionStorage.getItem(NAV_STACK_KEY) ?? '[]') as unknown[];
    expect(stack).to.have.length(32);
  });

  it('baseHrefOwnsPath owns everything under a "/" base', () => {
    // wtr serves with base "/", so the heuristic owns all paths here
    expect(baseHrefOwnsPath('/anything')).to.be.true;
  });
});
```

- [ ] **Step 5.2: Run, verify it fails**

- [ ] **Step 5.3: Implement**

`app/elohim-elements/elohim-core/src/navigation/epr-link-interceptor.ts`:

```ts
/**
 * EPR link interceptor — capture-phase document click listener that makes
 * cross-bundle anchors EPR-native.
 *
 * EPR-apps are separate SPA bundles dispatched by doorway's EprRouter. A
 * stale Angular routerLink targeting another bundle 404s on the local
 * router; a plain anchor full-reloads without recording navigation state.
 * This interceptor:
 *   - SAME-bundle anchors: always pass through untouched (Angular routerLink
 *     or default browser behavior own them).
 *   - CROSS-bundle anchors: preventDefault + stopImmediatePropagation
 *     (capture phase beats any routerLink target-phase handler that would
 *     404 on the local router), record a nav-handoff entry, then
 *     window.location.assign — the full load through doorway IS the
 *     projected EPR address.
 *
 * Fails open: any internal error falls back to default browser behavior.
 * Spec: 2026-06-05-omnibar-consolidation-epr-native-links-design.md §4.2.
 */

export interface EprLinkInterceptorOptions {
  /** Does THIS bundle's router own the given root-relative path? Default: base-href prefix heuristic. */
  ownsPath?: (path: string) => boolean;
  /** Called just before a cross-bundle navigation commits (write richer handoff state). Default: recordCrossBundleHandoff(). */
  beforeCrossBundle?: (target: string) => void;
  /**
   * Explicit installs (host-provided router-aware ownsPath, e.g. the Angular
   * shell) replace a default (page-chrome heuristic) install. Default
   * installs never replace an existing one.
   */
  explicit?: boolean;
  /** Test seam — defaults to window.location.assign. */
  assign?: (href: string) => void;
}

const NAV_STACK_KEY = 'elohim.session-nav-stack.v1';
const NAV_STACK_MAX = 32;

interface InstallRecord {
  uninstall: () => void;
  explicit: boolean;
}

declare global {
  interface Window {
    __elohimEprLinkInterceptor?: InstallRecord;
  }
}

/**
 * Append a handoff entry to the shared session-nav-stack — the same
 * sessionStorage shape elohim-app's SessionNavStackService reads, so the
 * protocol-omni back affordance survives the bundle boundary.
 */
export function recordCrossBundleHandoff(cid = ''): void {
  try {
    const raw = sessionStorage.getItem(NAV_STACK_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    const stack = Array.isArray(parsed) ? parsed : [];
    const entry = {
      url: location.pathname + location.search,
      cid,
      label: document.title,
      ts: Date.now(),
    };
    sessionStorage.setItem(NAV_STACK_KEY, JSON.stringify([...stack, entry].slice(-NAV_STACK_MAX)));
  } catch {
    // handoff is cosmetic — never block navigation
  }
}

/**
 * Default ownsPath: prefix match against this bundle's <base href>. A "/"
 * base owns everything — the shell (base "/") installs explicitly with a
 * router-aware predicate instead of relying on this heuristic.
 */
export function baseHrefOwnsPath(path: string): boolean {
  const base = new URL(document.baseURI).pathname;
  if (base === '/') return true;
  const trimmed = base.replace(/\/$/, '');
  return path === base || path === trimmed || path.startsWith(base);
}

function findAnchor(e: MouseEvent): HTMLAnchorElement | null {
  for (const t of e.composedPath()) {
    if (t instanceof HTMLAnchorElement) return t;
  }
  return null;
}

export function installEprLinkInterceptor(options: EprLinkInterceptorOptions = {}): () => void {
  if (typeof document === 'undefined') return () => undefined;

  const existing = window.__elohimEprLinkInterceptor;
  if (existing) {
    if (!options.explicit) return () => undefined; // default never disturbs the active install
    existing.uninstall();
  }

  const ownsPath = options.ownsPath ?? baseHrefOwnsPath;
  const assign = options.assign ?? ((href: string) => window.location.assign(href));

  const onClick = (e: MouseEvent): void => {
    try {
      if (e.defaultPrevented) return;
      if (e.button !== 0 || e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) return;
      const a = findAnchor(e);
      if (!a) return;
      if (a.hasAttribute('download') || a.hasAttribute('data-epr-bypass')) return;
      const target = a.getAttribute('target');
      if (target && target !== '_self') return;
      const rawHref = a.getAttribute('href');
      if (!rawHref || rawHref.startsWith('#')) return;
      const url = new URL(a.href, document.baseURI);
      if (url.origin !== location.origin) return;
      if (ownsPath(url.pathname)) return; // same-bundle: routerLink/browser own it

      // Cross-bundle: beat any stale routerLink handler, record handoff, go.
      e.preventDefault();
      e.stopImmediatePropagation();
      const targetHref = url.pathname + url.search + url.hash;
      try {
        if (options.beforeCrossBundle) options.beforeCrossBundle(url.pathname + url.search);
        else recordCrossBundleHandoff();
      } catch {
        // handoff failure never blocks navigation
      }
      assign(targetHref);
    } catch {
      // Fail open: default browser behavior proceeds.
    }
  };

  document.addEventListener('click', onClick, true);
  const uninstall = (): void => {
    document.removeEventListener('click', onClick, true);
    if (window.__elohimEprLinkInterceptor?.uninstall === uninstall) {
      delete window.__elohimEprLinkInterceptor;
    }
  };
  window.__elohimEprLinkInterceptor = { uninstall, explicit: options.explicit ?? false };
  return uninstall;
}
```

`index.ts` — add:

```ts
export {
  installEprLinkInterceptor,
  recordCrossBundleHandoff,
  baseHrefOwnsPath,
} from './navigation/epr-link-interceptor.js';
export type { EprLinkInterceptorOptions } from './navigation/epr-link-interceptor.js';
```

- [ ] **Step 5.4: Run, verify pass** (`pnpm test`)

- [ ] **Step 5.5: Spec §4.2 refinement edits** — two edits in the spec file:
  1. Replace the signature line
     `` `installEprLinkInterceptor({ ownsPath, onSameBundle, onCrossBundle? })`: ``
     with
     `` `installEprLinkInterceptor({ ownsPath?, beforeCrossBundle?, explicit?, assign? })`: ``
  2. Replace the bullet
     `- Same-bundle plain anchor → \`preventDefault\` + \`onSameBundle(path)\` (host routes`
     `  via Angular router — content-authored links get SPA navigation, no full reload).`
     with:
     `- Same-bundle anchors always pass through untouched — Angular \`routerLink\` (or`
     `  default browser behavior) owns them. Implementation refinement 2026-06-05: the`
     `  same-bundle upgrade hook was dropped because routerLink-managed anchors can't be`
     `  reliably distinguished from plain anchors in prod builds; interception is`
     `  cross-bundle-only, which is the regression being healed.`

- [ ] **Step 5.6: Commit**

```bash
git add app/elohim-elements/elohim-core/src/navigation/epr-link-interceptor.ts app/elohim-elements/elohim-core/src/navigation/epr-link-interceptor.spec.ts app/elohim-elements/elohim-core/src/index.ts genesis/docs/superpowers/specs/2026-06-05-omnibar-consolidation-epr-native-links-design.md
git commit -m "feat(elohim-core): capture-phase EPR link interceptor — cross-bundle anchors beat stale routerLink 404s"
```

---

### Task 6: page-chrome auto-install + default-omnibar opt-in attributes

**Files:**
- Modify: `app/elohim-elements/elohim-core/src/elohim-page-chrome.ts`, `elohim-default-omnibar.ts`
- Modify: `app/elohim-elements/elohim-core/src/elohim-page-chrome.spec.ts` (add tests; existing file)

- [ ] **Step 6.1: Write the failing tests** (append to `elohim-page-chrome.spec.ts`)

```ts
describe('<elohim-page-chrome> interceptor auto-install', () => {
  it('installs the epr-link interceptor on connect and removes it on disconnect', async () => {
    expect(window.__elohimEprLinkInterceptor).to.be.undefined;
    const el = await fixture(html`<elohim-page-chrome><p>x</p></elohim-page-chrome>`);
    expect(window.__elohimEprLinkInterceptor).to.exist;
    expect(window.__elohimEprLinkInterceptor!.explicit).to.be.false;
    el.remove();
    expect(window.__elohimEprLinkInterceptor).to.be.undefined;
  });
});

describe('<elohim-default-omnibar> opt-in controls', () => {
  it('renders no controls by default', async () => {
    const el = await fixture(html`<elohim-default-omnibar></elohim-default-omnibar>`);
    expect(el.shadowRoot!.querySelector('elohim-theme-toggle')).to.not.exist;
    expect(el.shadowRoot!.querySelector('elohim-lang-picker')).to.not.exist;
  });

  it('renders the theme toggle with show-theme-toggle', async () => {
    const el = await fixture(html`<elohim-default-omnibar show-theme-toggle></elohim-default-omnibar>`);
    expect(el.shadowRoot!.querySelector('elohim-theme-toggle')).to.exist;
  });

  it('renders the lang picker with show-lang-picker', async () => {
    const el = await fixture(html`<elohim-default-omnibar show-lang-picker></elohim-default-omnibar>`);
    expect(el.shadowRoot!.querySelector('elohim-lang-picker')).to.exist;
  });

  it('keeps the user part rendered alongside controls', async () => {
    const el = await fixture(html`<elohim-default-omnibar show-theme-toggle show-lang-picker></elohim-default-omnibar>`);
    expect(el.shadowRoot!.querySelector('[part="user"]')).to.exist;
  });
});
```

(`window.__elohimEprLinkInterceptor` typing comes from the interceptor module's global declaration — add `import './navigation/epr-link-interceptor.js';` to the spec's imports if tsc complains.)

- [ ] **Step 6.2: Run, verify the new tests fail**

- [ ] **Step 6.3: Implement page-chrome auto-install**

In `elohim-page-chrome.ts` add the import and lifecycle (and JSDoc note):

```ts
import { installEprLinkInterceptor } from './navigation/epr-link-interceptor.js';
```

```ts
export class ElohimPageChrome extends LitElement {
  @state() private hasSlottedOmnibar = false;

  private _uninstallInterceptor?: () => void;

  override connectedCallback(): void {
    super.connectedCallback();
    // Default (base-href heuristic) install — a shell that installs
    // explicitly (Angular elohim-app) wins via the explicit flag.
    this._uninstallInterceptor = installEprLinkInterceptor();
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this._uninstallInterceptor?.();
    this._uninstallInterceptor = undefined;
  }
  // ... rest unchanged
```

Add to the class JSDoc: `* Auto-installs the EPR link interceptor (cross-bundle anchor handling) on connect.`

- [ ] **Step 6.4: Implement default-omnibar attributes**

In `elohim-default-omnibar.ts`: change the lit import to `import { css, html, LitElement, nothing } from 'lit';`, change the decorators import to `import { property, state } from 'lit/decorators.js';`, add to the JSDoc:

```
 * @attr show-theme-toggle - Opt-in: render <elohim-theme-toggle> in the end cluster
 * @attr show-lang-picker  - Opt-in: render <elohim-lang-picker> in the end cluster
 * @csspart end - The inline-end cluster (controls + user area)
```

Add properties:

```ts
  @property({ type: Boolean, attribute: 'show-theme-toggle' }) showThemeToggle = false;
  @property({ type: Boolean, attribute: 'show-lang-picker' }) showLangPicker = false;
```

Add to styles (after `.user`):

```css
    .end {
      display: inline-flex;
      align-items: center;
      gap: 0.75rem;
    }
```

Replace `render()`:

```ts
  override render() {
    return html`
      <span class="brand" part="brand">elohim.host</span>
      <span class="end" part="end">
        ${this.showLangPicker ? html`<elohim-lang-picker></elohim-lang-picker>` : nothing}
        ${this.showThemeToggle ? html`<elohim-theme-toggle></elohim-theme-toggle>` : nothing}
        <span class="user" part="user">
          ${this._user
            ? html`<span part="user-name">${this._user.humanId}</span>`
            : html`<a href="/auth/signin">sign in</a>`}
        </span>
      </span>
    `;
  }
```

- [ ] **Step 6.5: Run, verify pass** (`pnpm test` — whole suite; existing default-omnibar tests must stay green)

- [ ] **Step 6.6: Commit**

```bash
git add app/elohim-elements/elohim-core/src/elohim-page-chrome.ts app/elohim-elements/elohim-core/src/elohim-default-omnibar.ts app/elohim-elements/elohim-core/src/elohim-page-chrome.spec.ts
git commit -m "feat(elohim-core): page-chrome auto-installs epr-link interceptor; default-omnibar gains opt-in theme/lang controls"
```

---

### Task 7: `<elohim-navigator>` — restore theme toggle + tray language

**Files:**
- Modify: `app/elohim-elements/elohim-core/src/elohim-navigator.ts`
- Modify: `app/elohim-elements/elohim-core/src/elohim-navigator.spec.ts` (add tests)

- [ ] **Step 7.1: Write the failing tests** (append to `elohim-navigator.spec.ts`)

```ts
describe('<elohim-navigator> preferences (theme/lang restore)', () => {
  it('shows the inline theme toggle for visitors', async () => {
    const el = await fixture<ElohimNavigator>(html`<elohim-navigator></elohim-navigator>`);
    expect(el.shadowRoot!.querySelector('[data-testid="nav-theme-inline"]')).to.exist;
  });

  it('hides the inline toggle when authenticated (tray owns it)', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator .isAuthenticated=${true}></elohim-navigator>`,
    );
    expect(el.shadowRoot!.querySelector('[data-testid="nav-theme-inline"]')).to.not.exist;
  });

  it('renders theme + language rows in the profile tray', async () => {
    const el = await fixture<ElohimNavigator>(html`<elohim-navigator></elohim-navigator>`);
    el.shadowRoot!.querySelector<HTMLButtonElement>('[data-testid="profile-bubble"]')!.click();
    await elementUpdated(el);
    expect(el.shadowRoot!.querySelector('[data-testid="nav-theme-toggle"]')).to.exist;
    expect(el.shadowRoot!.querySelector('[data-testid="nav-language"]')).to.exist;
  });

  it('show-preferences=false suppresses the tray rows', async () => {
    const el = await fixture<ElohimNavigator>(
      html`<elohim-navigator .showPreferences=${false}></elohim-navigator>`,
    );
    el.shadowRoot!.querySelector<HTMLButtonElement>('[data-testid="profile-bubble"]')!.click();
    await elementUpdated(el);
    expect(el.shadowRoot!.querySelector('[data-testid="nav-theme-toggle"]')).to.not.exist;
  });

  it('tray theme row cycles the shared store and keeps the tray open', async () => {
    const { getThemeStore } = await import('./theme/theme-store.js');
    getThemeStore().set('device');
    const el = await fixture<ElohimNavigator>(html`<elohim-navigator></elohim-navigator>`);
    el.shadowRoot!.querySelector<HTMLButtonElement>('[data-testid="profile-bubble"]')!.click();
    await elementUpdated(el);
    el.shadowRoot!.querySelector<HTMLButtonElement>('[data-testid="nav-theme-toggle"]')!.click();
    await elementUpdated(el);
    expect(getThemeStore().theme).to.equal('light');
    expect(el.shadowRoot!.querySelector('[part="profile-tray"]')).to.exist;
  });
});
```

- [ ] **Step 7.2: Run, verify the new tests fail**

- [ ] **Step 7.3: Implement**

In `elohim-navigator.ts`:

1. Imports (top of file, alongside existing imports):

```ts
import { getThemeStore, type ElohimTheme } from './theme/theme-store.js';
import {
  LOCALE_LABELS,
  SUPPORTED_LOCALES,
  getLocaleStore,
  type ElohimLocale,
} from './localize/locale-store.js';
```

2. Property + state + store wiring (next to the existing `@property` block at :352-368):

```ts
  /** Render the theme/language rows in the profile tray (restore of the pre-Lit-migration toggle; on by default). */
  @property({ type: Boolean, attribute: 'show-preferences' }) showPreferences = true;

  @state() private _theme: ElohimTheme = getThemeStore().theme;
  @state() private _locale: ElohimLocale = getLocaleStore().locale;

  private _unsubTheme?: () => void;
  private _unsubLocale?: () => void;

  override connectedCallback(): void {
    super.connectedCallback();
    this._unsubTheme = getThemeStore().subscribe((t) => {
      this._theme = t;
    });
    this._unsubLocale = getLocaleStore().subscribe((l) => {
      this._locale = l;
    });
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this._unsubTheme?.();
    this._unsubLocale?.();
  }

  private cyclePreferenceTheme(): void {
    getThemeStore().cycle(); // tray stays open so the change is visible
  }

  private cyclePreferenceLocale(): void {
    const i = SUPPORTED_LOCALES.indexOf(this._locale);
    getLocaleStore().set(SUPPORTED_LOCALES[(i + 1) % SUPPORTED_LOCALES.length] as ElohimLocale);
  }
```

(If the class already defines `connectedCallback`/`disconnectedCallback`, merge these bodies into them instead of redeclaring.)

3. Visitor inline toggle — in the `actions-section` div (line ~470), BEFORE the `<div style="position: relative;">` profile-bubble wrapper:

```ts
            ${!this.isAuthenticated && this.showPreferences
              ? html`<elohim-theme-toggle data-testid="nav-theme-inline"></elohim-theme-toggle>`
              : nothing}
```

4. Tray preference rows — inside the profile-tray div, AFTER the authed/unauth `${this.isAuthenticated ? ... : ...}` block and before the tray's closing `</div>`:

```ts
                      ${this.showPreferences
                        ? html`
                            <div class="tray-divider"></div>
                            <button
                              class="tray-item"
                              type="button"
                              role="menuitem"
                              data-testid="nav-theme-toggle"
                              @click=${this.cyclePreferenceTheme}
                            >
                              <span aria-hidden="true">
                                ${getThemeStore().effectiveTheme === 'dark' ? '🌙' : '☀️'}
                              </span>
                              Theme: ${this._theme}
                            </button>
                            <button
                              class="tray-item"
                              type="button"
                              role="menuitem"
                              data-testid="nav-language"
                              @click=${this.cyclePreferenceLocale}
                            >
                              <span aria-hidden="true">🌐</span>
                              ${LOCALE_LABELS[this._locale]}
                            </button>
                          `
                        : nothing}
```

5. JSDoc: add `@attr show-preferences` line and `elohim-theme-toggle` to the element description; update capability tag `@capabilityLocales en, es, he` (already present).

- [ ] **Step 7.4: Run, verify pass** (`pnpm test` — whole suite)

- [ ] **Step 7.5: elohim-core full gates + custom-elements.json regen**

```bash
cd /projects/elohim/app/elohim-elements/elohim-core
pnpm lint && pnpm lint:css && pnpm typecheck && pnpm build && pnpm test
```
Expected: all green; `pnpm build` regenerates `dist/custom-elements.json` (cem analyze) — dist is gitignored, nothing extra to stage.

- [ ] **Step 7.6: Commit**

```bash
git add app/elohim-elements/elohim-core/src/elohim-navigator.ts app/elohim-elements/elohim-core/src/elohim-navigator.spec.ts
git commit -m "feat(elohim-core): navigator regains theme toggle (tray + visitor inline) + tray language row — heals the 8ce50c4e2 migration drop"
```

---

### Task 8: ServingContext model + AppConfig.gitHash (elohim-app)

**Files:**
- Create: `app/elohim-app/src/app/elohim/models/serving-context.model.ts`
- Modify: `app/elohim-app/src/app/services/config.service.ts`
- Modify: `app/elohim-app/src/app/services/config.service.spec.ts`

- [ ] **Step 8.1: Write the failing test** — add to `config.service.spec.ts` (mirror the file's existing TestBed setup):

```ts
  it('dev config exposes gitHash from the environment (ServingContext.buildId source)', done => {
    service.getConfig().subscribe(config => {
      expect(config.gitHash).toBe(environment.gitHash);
      done();
    });
  });
```

(Import `environment` from `../../environments/environment` at the top of the spec if not present.)

- [ ] **Step 8.2: Run, verify it fails**

Run: `cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts src/app/services/config.service.spec.ts`
Expected: FAIL — `gitHash` not in AppConfig.

- [ ] **Step 8.3: Implement**

`app/elohim-app/src/app/elohim/models/serving-context.model.ts` (new):

```ts
/**
 * ServingContext — a dimension orthogonal to reach: the system state an EPR
 * is being projected through. Rendered by the protocol-omni serving-context
 * segment (trust surface). Read-only view-model over existing build/config
 * values — nothing persisted, no entity (spec §2, §5.1).
 *
 * `variant` is RESERVED: EPR-natively a variant is WHICH project-epr
 * commitment / bundle CID served you (blue/green, A/B) — it fills from
 * substrate provenance when spec §9.7 lands, never from k8s vocabulary.
 */
export interface ServingContext {
  readonly tier?: 'development' | 'alpha' | 'staging' | 'production';
  readonly logLevel?: string;
  /** Short gitHash today → doorway-attested bundle CID when spec §9.7 lands. */
  readonly buildId?: string;
  readonly variant?: string;
}
```

`config.service.ts` — updated interface + stream:

```ts
export interface AppConfig {
  readonly logLevel: 'debug' | 'info' | 'warn' | 'error';
  readonly environment: string;
  /** CI-substituted git commit hash (ServingContext.buildId source). */
  readonly gitHash: string;
}

const DEFAULT_PROD_CONFIG: AppConfig = {
  logLevel: 'error',
  environment: 'production',
  gitHash: environment.gitHash,
} as const;
```

```ts
  private createConfigStream(): Observable<AppConfig> {
    if (!environment.production) {
      return of(this.getDevConfig());
    }

    return this.http.get<Partial<AppConfig>>('/assets/config.json').pipe(
      map(config => ({ ...DEFAULT_PROD_CONFIG, ...(config ?? {}) })),
      catchError(() => of(DEFAULT_PROD_CONFIG)),
      shareReplay(1)
    );
  }

  private getDevConfig(): AppConfig {
    return {
      logLevel: environment.logLevel || 'debug',
      environment: environment.environment || 'development',
      gitHash: environment.gitHash,
    };
  }
```

- [ ] **Step 8.4: Run, verify pass** (same vitest command — all config.service tests green)

- [ ] **Step 8.5: Commit**

```bash
git add app/elohim-app/src/app/elohim/models/serving-context.model.ts app/elohim-app/src/app/services/config.service.ts app/elohim-app/src/app/services/config.service.spec.ts
git commit -m "feat(app): ServingContext model + AppConfig.gitHash passthrough (CI-substituted)"
```

---

### Task 9: ThemeService cross-island sync patch

**Files:**
- Modify: `app/elohim-app/src/app/services/theme.service.ts`
- Modify: `app/elohim-app/src/app/services/theme.service.spec.ts`

- [ ] **Step 9.1: Write the failing tests** (append to the existing describe; reuse its TestBed):

```ts
  it('adopts an elohim-theme-changed event from a Lit island without re-dispatching', () => {
    let events = 0;
    const onEvent = (): void => {
      events += 1;
    };
    window.addEventListener('elohim-theme-changed', onEvent);
    window.dispatchEvent(new CustomEvent('elohim-theme-changed', { detail: { theme: 'dark' } }));
    window.removeEventListener('elohim-theme-changed', onEvent);
    expect(service.getCurrentTheme()).toBe('dark');
    expect(document.body.getAttribute('data-theme')).toBe('dark');
    expect(events).toBe(1); // only the one we dispatched
  });

  it('dispatches elohim-theme-changed when Angular sets the theme (Lit follows)', () => {
    let detail: { theme?: string } | null = null;
    const onEvent = (e: Event): void => {
      detail = (e as CustomEvent<{ theme: string }>).detail;
    };
    window.addEventListener('elohim-theme-changed', onEvent);
    service.setTheme('light');
    window.removeEventListener('elohim-theme-changed', onEvent);
    expect(detail).toEqual({ theme: 'light' });
  });
```

- [ ] **Step 9.2: Run, verify fail**

Run: `cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts src/app/services/theme.service.spec.ts`

- [ ] **Step 9.3: Implement** — in `theme.service.ts`:

Constructor gains the listeners (after `this.loadTheme();`):

```ts
  constructor() {
    this.renderer = this.rendererFactory.createRenderer(null, null);
    this.loadTheme();

    // Cross-island/tab sync: Lit's ThemeStore (elohim-core) speaks the same
    // contract; adopt its changes without re-dispatching (no loops).
    window.addEventListener('storage', (e: StorageEvent) => {
      if (e.key === 'elohim-theme' && this.isValidTheme(e.newValue)) {
        this.adoptExternal(e.newValue);
      }
    });
    window.addEventListener('elohim-theme-changed', (e: Event) => {
      const theme = (e as CustomEvent<{ theme?: unknown }>).detail?.theme;
      if (this.isValidTheme(theme)) this.adoptExternal(theme);
    });
  }
```

`setTheme` gains the dispatch (so Lit toggles follow Angular's floating toggle):

```ts
  setTheme(theme: Theme): void {
    this.currentTheme$.next(theme);
    this.applyTheme(theme);
    this.saveTheme(theme);
    window.dispatchEvent(new CustomEvent('elohim-theme-changed', { detail: { theme } }));
  }
```

New private helpers:

```ts
  private isValidTheme(v: unknown): v is Theme {
    return v === 'light' || v === 'dark' || v === 'device';
  }

  /** External change (Lit island / other tab): apply + emit, never re-persist/dispatch. */
  private adoptExternal(theme: Theme): void {
    if (theme === this.currentTheme$.value) return;
    this.currentTheme$.next(theme);
    this.applyTheme(theme);
  }
```

(The `adoptExternal` guard `theme === currentTheme$.value` is what breaks the Angular↔Lit echo loop: each side adopts silently and only the originator dispatches.)

- [ ] **Step 9.4: Run, verify pass** (whole theme.service spec green)

- [ ] **Step 9.5: Commit**

```bash
git add app/elohim-app/src/app/services/theme.service.ts app/elohim-app/src/app/services/theme.service.spec.ts
git commit -m "feat(app): ThemeService adopts/announces elohim-theme-changed — Angular and Lit theme controls stay in sync"
```

---

### Task 10: protocol-omni — serving-context segment + theme opt-in

**Files:**
- Modify: `app/elohim-app/src/app/elohim/components/protocol-omni/protocol-omni.component.{ts,html,css,spec.ts}`
- Modify: `app/elohim-app/src/app/app.component.html` (enable `showEnvContext`)

- [ ] **Step 10.1: Write the failing tests** — append a new describe to `protocol-omni.component.spec.ts`. Mirror the file's existing TestBed bootstrapping (it already stubs `ProtocolNavigationService`); add a ConfigService stub you can swap per test:

```ts
import { of } from 'rxjs';

import { ConfigService } from '../../../services/config.service';

describe('ProtocolOmniComponent serving context', () => {
  function setup(opts: {
    environment?: string;
    logLevel?: string;
    gitHash?: string;
    showEnvContext?: boolean;
  }) {
    // Mirror the TestBed setup of the existing describe block above, PLUS:
    //   providers: [..., { provide: ConfigService, useValue: {
    //     getConfig: () => of({
    //       environment: opts.environment ?? 'alpha',
    //       logLevel: opts.logLevel ?? 'debug',
    //       gitHash: opts.gitHash ?? 'abc1234def',
    //     }),
    //   }}]
    // then:
    //   fixture.componentRef.setInput('contentId', 'elohim-host-landing');
    //   fixture.componentRef.setInput('showEnvContext', opts.showEnvContext ?? true);
    //   fixture.detectChanges();
    // returns { fixture, expand: () => click [data-testid="protocol-omni-chip"] + detectChanges }
  }

  it('renders nothing when showEnvContext is false (default)', () => {
    const { fixture, expand } = setup({ showEnvContext: false });
    expand();
    expect(fixture.nativeElement.querySelector('[data-testid="protocol-omni-env"]')).toBeNull();
  });

  it('renders tier + short buildId adjacent to the EPR on alpha', () => {
    const { fixture, expand } = setup({ environment: 'alpha', gitHash: 'abc1234def' });
    expand();
    const env = fixture.nativeElement.querySelector('[data-testid="protocol-omni-env"]');
    expect(env).toBeTruthy();
    expect(env.textContent).toContain('alpha');
    expect(env.textContent).toContain('abc1234');
    expect(env.getAttribute('aria-label')).toContain('alpha environment');
  });

  it('renders on staging and development too', () => {
    for (const tier of ['staging', 'development']) {
      const { fixture, expand } = setup({ environment: tier });
      expand();
      expect(
        fixture.nativeElement.querySelector('[data-testid="protocol-omni-env"]'),
      ).toBeTruthy();
    }
  });

  it('stays silent on production even when enabled', () => {
    const { fixture, expand } = setup({ environment: 'production' });
    expand();
    expect(fixture.nativeElement.querySelector('[data-testid="protocol-omni-env"]')).toBeNull();
    const chip = fixture.nativeElement.querySelector('[data-testid="protocol-omni-chip"]');
    expect(chip?.classList.contains('omni-chip-env')).toBe(false);
  });

  it('omits a placeholder/local gitHash from the segment', () => {
    const { fixture, expand } = setup({ gitHash: 'GIT_HASH_PLACEHOLDER' });
    expand();
    const env = fixture.nativeElement.querySelector('[data-testid="protocol-omni-env"]');
    expect(env.querySelector('.omni-env-build')).toBeNull();
  });

  it('tints the collapsed chip when env context is active', () => {
    const { fixture } = setup({ environment: 'alpha' });
    const chip = fixture.nativeElement.querySelector('[data-testid="protocol-omni-chip"]');
    expect(chip.classList.contains('omni-chip-env')).toBe(true);
  });

  it('renders the theme toggle only when showThemeToggle is set', () => {
    const a = setup({ showEnvContext: false });
    a.expand();
    expect(a.fixture.nativeElement.querySelector('elohim-theme-toggle')).toBeNull();
    const b = setup({ showEnvContext: false });
    b.fixture.componentRef.setInput('showThemeToggle', true);
    b.fixture.detectChanges();
    b.expand();
    expect(b.fixture.nativeElement.querySelector('elohim-theme-toggle')).toBeTruthy();
  });
});
```

Write the `setup` helper for real (the comment block above describes it — implement it concretely against the existing spec's TestBed pattern; it is the same pattern the 13 existing tests use, plus the ConfigService provider and the two setInput calls).

- [ ] **Step 10.2: Run, verify fail**

Run: `cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts src/app/elohim/components/protocol-omni/protocol-omni.component.spec.ts`

- [ ] **Step 10.3: Implement the component**

`protocol-omni.component.ts` — add imports and members. (Not a storage schema: `CUSTOM_ELEMENTS_SCHEMA` is Angular's template-compiler constant; this design is operational/Category C throughout — source of truth unchanged, see spec §2.)

```ts
import { CUSTOM_ELEMENTS_SCHEMA } from '@angular/core'; // merge into the existing @angular/core import list

import 'elohim-core/register';

import { ConfigService } from '../../../services/config.service';
import type { ServingContext } from '../../models/serving-context.model';
```

Component decorator: add `schemas: [CUSTOM_ELEMENTS_SCHEMA],` (Angular template-compiler setting — operational, no source-of-truth impact).

Class additions:

```ts
  /** Opt-in: render the serving-context segment (spec §5). Default off — the trust surface never cries wolf. */
  readonly showEnvContext = input<boolean>(false);
  /** Opt-in: render <elohim-theme-toggle> in the expanded toolbar. Default off. */
  readonly showThemeToggle = input<boolean>(false);

  private readonly configService = inject(ConfigService);

  readonly servingContext = signal<ServingContext | null>(null);

  readonly envVisible = computed(() => {
    const ctx = this.servingContext();
    return this.showEnvContext() && !!ctx?.tier && ctx.tier !== 'production';
  });

  readonly shortBuildId = computed(() => {
    const id = this.servingContext()?.buildId;
    return id ? id.slice(0, 7) : '';
  });

  readonly envLabel = computed(() => {
    const ctx = this.servingContext();
    if (!ctx) return '';
    const facets = [`You're viewing this EPR through the ${ctx.tier} environment`];
    if (this.shortBuildId()) facets.push(`build ${this.shortBuildId()}`);
    if (ctx.logLevel) facets.push(`log: ${ctx.logLevel}`);
    return `${facets.join(' · ')} — backend details`;
  });
```

`ngOnInit` — prepend (before the suppression check):

```ts
    this.configService.getConfig().subscribe(cfg => {
      const buildId =
        cfg.gitHash && cfg.gitHash !== 'GIT_HASH_PLACEHOLDER' && cfg.gitHash !== 'local-dev'
          ? cfg.gitHash
          : undefined;
      this.servingContext.set({
        tier: cfg.environment as ServingContext['tier'],
        logLevel: cfg.logLevel,
        buildId,
      });
    });
```

`protocol-omni.component.html` — wrap the EPR button + env link in a group. Replace the existing `.omni-epr` button block (lines 34-44) with:

```html
    <span class="omni-epr-group">
      <button
        type="button"
        class="omni-epr"
        data-testid="protocol-omni-epr"
        (click)="copyCid()"
        title="Click to copy content identifier"
        aria-label="Copy content identifier"
      >
        <span class="omni-epr-label">EPR</span>
        <code class="omni-epr-value">{{ shortCid() }}</code>
      </button>

      <a
        *ngIf="envVisible()"
        class="omni-env"
        data-testid="protocol-omni-env"
        routerLink="/doorway/elohim"
        [attr.title]="envLabel()"
        [attr.aria-label]="envLabel()"
      >
        <span class="omni-env-tier">{{ servingContext()?.tier }}</span>
        <code *ngIf="shortBuildId()" class="omni-env-build">{{ shortBuildId() }}</code>
      </a>
    </span>
```

Theme toggle — insert before the `.omni-collapse` button:

```html
    <elohim-theme-toggle
      *ngIf="showThemeToggle()"
      data-testid="protocol-omni-theme"
    ></elohim-theme-toggle>
```

Chip tint — change the chip opening tag:

```html
  <button
    type="button"
    class="omni-chip"
    [class.omni-chip-env]="envVisible()"
    data-testid="protocol-omni-chip"
```

`protocol-omni.component.css` — add `--omni-env-ring: #d97706;` to BOTH `:host` blocks' custom-property lists (light at :12-17, dark at :22-27), and append:

```css
.omni-epr-group {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
}

.omni-env {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  border-color: var(--omni-env-ring);
}

.omni-env-tier {
  text-transform: uppercase;
  font-weight: 700;
  font-size: 10px;
  letter-spacing: 0.04em;
}

.omni-env-build {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

.omni-chip-env {
  box-shadow:
    0 0 0 2px var(--omni-env-ring),
    var(--omni-shadow);
}
```

`app.component.html` — enable on the shell (the surface where debug-bar lived):

```html
@if (protocolRouteCtx.isProtocol() && protocolRouteCtx.cid(); as cid) {
  <app-protocol-omni [contentId]="cid" [showEnvContext]="true"></app-protocol-omni>
}
```

- [ ] **Step 10.4: Run, verify pass** (component spec — all 13 existing + new tests green)

- [ ] **Step 10.5: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/protocol-omni app/elohim-app/src/app/app.component.html
git commit -m "feat(app): protocol-omni serving-context segment (opt-in, trust-framed, EPR-adjacent) + theme-toggle opt-in"
```

---

### Task 11: Delete debug-bar (concerns lifted)

**Files:**
- Delete: `app/elohim-app/src/app/components/debug-bar/` (all 4 files)
- Modify: `app/elohim-app/src/app/components/home/home.component.{html,ts,spec.ts}`

- [ ] **Step 11.1: Remove usage** — in `home.component.html` delete line 14 (`<app-debug-bar></app-debug-bar>`) and its comment line 13; in `home.component.ts` delete the import (line 10) and the `DebugBarComponent` entry in the `imports` array (line 23); in `home.component.spec.ts` delete the `should render debug bar` test (lines 128-132).

- [ ] **Step 11.2: Delete the component**

```bash
git rm -r app/elohim-app/src/app/components/debug-bar
```

- [ ] **Step 11.3: Verify**

```bash
cd /projects/elohim/app/elohim-app
grep -rn "debug-bar\|DebugBar" src && echo "LEFTOVERS — fix" || echo "clean"
pnpm exec vitest run --config vite.config.ts src/app/components/home/home.component.spec.ts
```
Expected: "clean"; home spec green.

- [ ] **Step 11.4: Commit**

```bash
git add app/elohim-app/src/app/components/home
git commit -m "refactor(app): delete debug-bar — env identity/log-level/backend-config lifted into protocol-omni serving context"
```

---

### Task 12: EprNavService + shell interceptor install

**Files:**
- Create: `app/elohim-app/src/app/elohim/services/epr-nav.service.ts`, `.spec.ts`
- Modify: `app/elohim-app/src/app/app.component.ts`
- Modify: `app/elohim-app/src/app/app.routes.spec.ts` (comment refresh)

- [ ] **Step 12.1: Write the failing test**

`app/elohim-app/src/app/elohim/services/epr-nav.service.spec.ts`:

```ts
import { TestBed } from '@angular/core/testing';
import { Router } from '@angular/router';

import { describe, it, expect, beforeEach, vi } from 'vitest';

import { EprNavService } from './epr-nav.service';
import { ProtocolRouteContextService } from './protocol-route-context.service';
import { SessionNavStackService } from './session-nav-stack.service';

describe('EprNavService', () => {
  let service: EprNavService;
  let router: Router;
  const navStack = { record: vi.fn() };
  const routeCtx = { cid: () => 'elohim-host-landing' };

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        EprNavService,
        { provide: SessionNavStackService, useValue: navStack },
        { provide: ProtocolRouteContextService, useValue: routeCtx },
        {
          provide: Router,
          useValue: {
            config: [
              { path: '' },
              { path: 'community' },
              { path: 'shefa' },
              { path: 'doorway' },
              { path: 'deliver/:slug' },
              { path: '**' },
            ],
            url: '/current',
            navigateByUrl: vi.fn().mockResolvedValue(true),
            createUrlTree: vi.fn((cmds: string[]) => ({ toString: () => cmds.join('/') })),
          },
        },
      ],
    });
    service = TestBed.inject(EprNavService);
    router = TestBed.inject(Router);
    navStack.record.mockClear();
  });

  it('owns top-level paths present in the router config (catch-all excluded)', () => {
    expect(service.ownsPath('/')).toBe(true);
    expect(service.ownsPath('/community')).toBe(true);
    expect(service.ownsPath('/deliver/some-slug')).toBe(true);
    expect(service.ownsPath('/lamad')).toBe(false);
    expect(service.ownsPath('/lamad/path/x')).toBe(false);
  });

  it('routes same-bundle paths through the Angular router', () => {
    service.navigate('/community');
    expect(router.navigateByUrl).toHaveBeenCalledWith('/community');
    expect(navStack.record).not.toHaveBeenCalled();
  });

  it('hands off cross-bundle paths: nav-stack record + full load', () => {
    const assign = vi.fn();
    (service as unknown as { assign: (h: string) => void }).assign = assign;
    service.navigate('/lamad/path/abc');
    expect(navStack.record).toHaveBeenCalledWith({
      url: '/current',
      cid: 'elohim-host-landing',
      label: document.title,
    });
    expect(assign).toHaveBeenCalledWith('/lamad/path/abc');
    expect(router.navigateByUrl).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 12.2: Run, verify fail**

Run: `cd /projects/elohim/app/elohim-app && pnpm exec vitest run --config vite.config.ts src/app/elohim/services/epr-nav.service.spec.ts`

- [ ] **Step 12.3: Implement**

`app/elohim-app/src/app/elohim/services/epr-nav.service.ts`:

```ts
import { Injectable, inject } from '@angular/core';
import { Router } from '@angular/router';

import { ProtocolRouteContextService } from './protocol-route-context.service';
import { SessionNavStackService } from './session-nav-stack.service';

/**
 * EPR-aware navigation: same-bundle paths go through the Angular router;
 * cross-bundle paths get a nav-handoff record then a full doorway load
 * (the URL IS the projected EPR address — spec §4).
 *
 * ownsPath derives from the LIVE router config, so when a pillar splits
 * into its own bundle (as lamad did) the same call sites flip to
 * cross-bundle automatically — no hand-maintained route list.
 */
@Injectable({ providedIn: 'root' })
export class EprNavService {
  private readonly router = inject(Router);
  private readonly navStack = inject(SessionNavStackService);
  private readonly routeCtx = inject(ProtocolRouteContextService);

  /** Test seam — defaults to a full browser navigation. */
  assign: (href: string) => void = href => globalThis.location.assign(href);

  ownsPath(path: string): boolean {
    const top = path.replace(/^\//, '').split(/[/?#]/)[0] ?? '';
    if (top === '') return true; // root landing is shell-owned
    return this.router.config.some(r => {
      if (!r.path || r.path === '**') return false;
      return r.path.split('/')[0] === top;
    });
  }

  navigate(pathOrCommands: string | readonly unknown[]): void {
    const url = Array.isArray(pathOrCommands)
      ? this.router.createUrlTree(pathOrCommands as never[]).toString()
      : (pathOrCommands as string);
    if (this.ownsPath(url)) {
      void this.router.navigateByUrl(url);
      return;
    }
    this.recordHandoff();
    this.assign(url);
  }

  /** Write the cross-bundle handoff entry (back affordance survives the boundary). */
  recordHandoff(): void {
    this.navStack.record({
      url: this.router.url,
      cid: this.routeCtx.cid() ?? '',
      label: document.title,
    });
  }
}
```

`app.component.ts` — add the explicit interceptor install:

```ts
import { installEprLinkInterceptor } from 'elohim-core';

import { EprNavService } from './elohim/services/epr-nav.service';
```

In the class: `private readonly eprNav = inject(EprNavService);`

In `ngOnInit()` (after `this.registerEprProtocolHandler();`):

```ts
    // Cross-bundle anchors (e.g. /lamad) are EPR-native: capture-phase
    // interception beats stale routerLink 404s; ownsPath derives from the
    // live router config. Explicit install wins over page-chrome's default.
    installEprLinkInterceptor({
      explicit: true,
      ownsPath: p => this.eprNav.ownsPath(p),
      beforeCrossBundle: () => this.eprNav.recordHandoff(),
    });
```

`app.routes.spec.ts` — refresh the TODO comment (lines 19-26): replace the sentence `This is a tracked Slice-2 deferral (BundleRouteContext claims + /epr/{id} resolver), NOT a regression:` with `Cross-bundle anchors are now handled by the epr-link interceptor + EprNavService (2026-06-05 omnibar-consolidation spec §4) pending the Slice-2 /epr resolver; NOT a regression:` — the two canary tests stay exactly as they are.

- [ ] **Step 12.4: Run, verify pass** (epr-nav spec + `pnpm exec vitest run --config vite.config.ts src/app/app.routes.spec.ts`)

- [ ] **Step 12.5: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/epr-nav.service.ts app/elohim-app/src/app/elohim/services/epr-nav.service.spec.ts app/elohim-app/src/app/app.component.ts app/elohim-app/src/app/app.routes.spec.ts
git commit -m "feat(app): EprNavService (router-config-derived ownsPath) + explicit shell install of the epr-link interceptor"
```

---### Task 13: Link sweep (all first-party cross-bundle sites)

**Files (elohim-app):**
- Modify: `src/app/components/footer/footer.component.{html,ts}`
- Modify: `src/app/components/not-found/not-found.component.{html,ts}`
- Modify: `src/app/imagodei/components/profile/profile.component.{html,ts}`
- Modify: `src/app/imagodei/services/tauri-auth.service.ts`
- Modify: `src/app/elohim/services/elohim-presence.service.ts`

**Files (lamad):**
- Modify: `app/lamad/src/app/components/lamad-layout/lamad-layout.component.{html,ts}`

**Rule of the sweep:** anchors to currently-cross-bundle targets (`/lamad*`, and `/` *from* lamad) become plain `href` (+ testid where a2o needs it). Anchors to currently-intra targets (`/shefa`, `/community`, `/imagodei/*`, `/doorway/*`) KEEP `routerLink` — the capture-phase interceptor future-proofs them if those pillars split later. Programmatic cross-bundle `router.navigate` calls go through `EprNavService`.

- [ ] **Step 13.1: footer** — in `footer.component.html` line 14:

```html
        <a href="/lamad" class="docs-link" data-testid="footer-lamad-link">📚 Lamad Learning Platform</a>
```

In `footer.component.ts`: remove `RouterLink` from the `imports` array and delete `import { RouterLink } from '@angular/router';` (that anchor was the template's only routerLink — verify with `grep -c routerLink src/app/components/footer/footer.component.html` → expect 0 after the edit).

- [ ] **Step 13.2: not-found** — in `not-found.component.html` lines 39-41 change the three lamad anchors to `href` (keep line 38's `routerLink="/"`):

```html
        <li><a href="/lamad">Lamad - Learning Platform</a></li>
        <li><a href="/lamad/explore">Knowledge Explorer</a></li>
        <li><a href="/lamad/search">Search</a></li>
```

In `not-found.component.ts`: inject the service (`private readonly eprNav = inject(EprNavService);` with `import { EprNavService } from '../../elohim/services/epr-nav.service';` — match the file's existing inject/constructor style) and change `goToLamad()`:

```ts
  goToLamad(): void {
    this.eprNav.navigate('/lamad');
  }
```

- [ ] **Step 13.3: profile** — in `profile.component.html` line 149 change `routerLink="/lamad/human"` to `href="/lamad/human"` (line 141 `/shefa` keeps routerLink — intra-bundle). In `profile.component.ts`:

```ts
  navigateToDiscovery(): void {
    this.eprNav.navigate('/lamad/discovery');
  }
```

(inject EprNavService as in 13.2; `navigateToResource` stays on the router — `/resource/...` is shell-owned.)

- [ ] **Step 13.4: tauri-auth** — in `tauri-auth.service.ts` line 395 replace `void this.router.navigate(['/lamad']);` with `this.eprNav.navigate('/lamad');` (inject EprNavService; keep the router injection if other call sites use it).

- [ ] **Step 13.5: presence** — in `elohim-presence.service.ts` lines 267-270:

```ts
    if (pathId) {
      this.eprNav.navigate(['/lamad/path', pathId]);
    } else {
      // Fallback: navigate to path catalog
      this.eprNav.navigate('/lamad');
    }
```

(inject EprNavService.)

- [ ] **Step 13.6: lamad-layout** — in `lamad-layout.component.html`:
  - Line 3: wire the navigator's navigate event:
    ```html
    <elohim-navigator [attr.context]="'lamad'" [attr.show-search]="true" (navigate)="onNavigatorNavigate($event)">
    ```
  - Line 20 footer link (cross-bundle: lamad → landing; routerLink="/" wrongly resolved to lamad's own home):
    ```html
    <a href="/" class="footer-link" data-testid="lamad-footer-home-link">Powered by Elohim Protocol</a>
    ```

  In `lamad-layout.component.ts`: remove `RouterLink` from imports array + import statement (it was the only routerLink in this template); add:

```ts
  /**
   * Navigator routes are protocol-absolute. Lamad-owned ones go through this
   * bundle's router (base-href '/lamad/' is stripped); everything else is a
   * cross-bundle handoff to the doorway-projected address.
   */
  onNavigatorNavigate(event: Event): void {
    const route = (event as CustomEvent<{ route?: string }>).detail?.route;
    if (!route) return;
    if (route === '/lamad' || route.startsWith('/lamad/')) {
      void this.router.navigateByUrl(route.slice('/lamad'.length) || '/');
    } else {
      globalThis.location.assign(route);
    }
  }
```

- [ ] **Step 13.7: doorway-app verify-only** — confirm the two `/identity/profile` links are plain hrefs (no change):

```bash
grep -n 'href="/identity/profile"' doorway/doorway-app/src/app/components/toolbar/doorway-toolbar.component.html doorway/doorway-app/src/app/components/account/doorway-account.component.ts
```
Expected: both lines print; nothing to edit.

- [ ] **Step 13.8: Verify the sweep is complete**

```bash
cd /projects/elohim
grep -rn 'routerLink="/lamad' app/elohim-app/src doorway/doorway-app/src app/lamad/src && echo "MISSED SITES" || echo "sweep clean"
grep -rn "router.navigate(\['/lamad" app/elohim-app/src && echo "MISSED PROGRAMMATIC" || echo "programmatic clean"
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts src/app/components src/app/imagodei src/app/elohim/services
```
Expected: both "clean"; affected suites green (footer/not-found/profile/presence specs may need their mocks extended with an EprNavService stub `{ navigate: vi.fn() }` — add where compilation demands).

- [ ] **Step 13.9: Commit**

```bash
git add app/elohim-app/src/app/components/footer app/elohim-app/src/app/components/not-found app/elohim-app/src/app/imagodei/components/profile app/elohim-app/src/app/imagodei/services/tauri-auth.service.ts app/elohim-app/src/app/elohim/services/elohim-presence.service.ts app/lamad/src/app/components/lamad-layout
git commit -m "feat(app): EPR-native link sweep — cross-bundle anchors to plain href, programmatic nav via EprNavService, lamad navigator wiring"
```

---

### Task 14: a2o scenarios (story-first closure)

**Files:**
- Modify: `genesis/a2o/features/protocol/protocol-omni.feature`
- Modify: `genesis/a2o/features/browser/navigation-browser.feature`
- Create: `genesis/a2o/features/elohim-core/chrome-preferences.feature`

- [ ] **Step 14.1: protocol-omni.feature** — append:

```gherkin
  @browser-only
  Scenario: The serving-context segment contextualizes the EPR on non-production environments
    When I open the landing page in a browser
    And I click the element [data-testid="protocol-omni-chip"]
    Then the element [data-testid="protocol-omni-env"] is visible
    And the element [data-testid="protocol-omni-env"] text contains "alpha"
```

- [ ] **Step 14.2: navigation-browser.feature** — append:

```gherkin
  Scenario: Footer Lamad link crosses the bundle boundary without a 404
    When Matthew navigates to "/" in the browser
    And Matthew clicks the element with testid "footer-lamad-link"
    Then the page should load successfully
    And the page should display the main content
    And there should be no console errors
```

Check step coverage: `cd /projects/elohim/genesis/a2o && npx cucumber-js --dry-run features/browser/navigation-browser.feature`. If `clicks the element with testid` is undefined, add to the browser steps file (find it: `grep -rln "navigates to .* in the browser" steps/`) following its existing Playwright World idiom:

```ts
When(
  '{word} clicks the element with testid {string}',
  async function (this: ElohimWorld, _persona: string, testid: string) {
    // mirror the page/World accessor used by the neighboring browser steps
    await this.page.click(`[data-testid="${testid}"]`);
    await this.page.waitForLoadState('networkidle');
  },
);
```

(Adapt `this.page` to whatever the neighboring steps in that file actually use — read two steps above/below and copy their accessor exactly.)

- [ ] **Step 14.3: chrome-preferences.feature** (new spine; `@wip` until the deliver phase wires browser steps for storage assertions):

```gherkin
@browser @elohim-core @chrome-preferences
Feature: Chrome preferences follow the person across EPR-app boundaries
  Theme and language controls live in the protocol chrome (omnibar, navigator)
  and persist device-wide through one shared contract, so crossing a bundle
  boundary never resets how the protocol looks or speaks.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @wip @browser-only
  Scenario: Theme choice persists across the app boundary
    When Matthew navigates to "/lamad" in the browser
    And Matthew clicks the element with testid "nav-theme-inline"
    And Matthew clicks the element with testid "footer-lamad-link"
    Then the body data-theme attribute equals the chosen theme
    And there should be no console errors

  @wip @browser-only
  Scenario: Switching to Hebrew flips the chrome to RTL and persists
    When Matthew navigates to "/lamad" in the browser
    And Matthew clicks the element with testid "profile-bubble"
    And Matthew clicks the element with testid "nav-language"
    And Matthew clicks the element with testid "nav-language"
    Then the document dir attribute is "rtl"
```

- [ ] **Step 14.4: Verify** — `cd /projects/elohim/genesis/a2o && npx cucumber-js --dry-run`
Expected: 0 undefined steps among non-@wip scenarios (the @wip spine steps may be undefined — that's the @wip contract; if dry-run counts them, add `--tags "not @wip"`).

- [ ] **Step 14.5: Commit**

```bash
git add genesis/a2o/features/protocol/protocol-omni.feature genesis/a2o/features/browser/navigation-browser.feature genesis/a2o/features/elohim-core/chrome-preferences.feature genesis/a2o/steps
git commit -m "test(a2o): serving-context + cross-bundle footer link scenarios; chrome-preferences spine (@wip)"
```

---

### Task 15: Full quality gates

- [ ] **Step 15.1: elohim-core** — `cd /projects/elohim/app/elohim-elements/elohim-core && pnpm lint && pnpm lint:css && pnpm typecheck && pnpm test && pnpm build` → all green.
- [ ] **Step 15.2: elohim-app** — `cd /projects/elohim/app/elohim-app && pnpm run lint && pnpm exec vitest run --config vite.config.ts` → lint clean, full suite green. Fix any spec that newly needs an `EprNavService`/`ConfigService` stub.
- [ ] **Step 15.3: lamad** — `cd /projects/elohim/app/lamad && pnpm exec vitest run --config vite.config.ts 2>/dev/null || pnpm test` → green (lamad-layout compile).
- [ ] **Step 15.4: storybook compile** — `cd /projects/elohim/app/elohim-library && pnpm run build-storybook` → builds (validates the two new story files).
- [ ] **Step 15.5: a2o dry-run** — `cd /projects/elohim/genesis/a2o && npx cucumber-js --dry-run --tags "not @wip"` → 0 undefined.
- [ ] **Step 15.6: Fix-up commit if needed**

```bash
git add <exactly-the-fixed-files>
git commit -m "test: gate fix-ups for omnibar-consolidation sweep"
```

**Done = all five gates green + 15 commits on `shift/a2o-greenup`. Do NOT push — integrator owns push/merge.**

---

## Self-review record (plan author)

- **Spec coverage:** §4 sweep+interceptor → Tasks 5, 6, 12, 13; §5 serving context → Tasks 8, 10, 11; §6 theme → Tasks 1, 3, 6, 7, 9, 10; §7 lang/a11y → Tasks 2, 4, 6, 7 (a11y = no new toggles ✓ nothing to build); §8 testing → per-task TDD + Task 14; §9 follow-ups → none implemented (correct); §10 inventory → matches file structure above.
- **Spec deviation (documented):** §4.2 same-bundle `onSameBundle` upgrade dropped (routerLink-managed anchors indistinguishable in prod builds) — spec amended in Task 5 Step 5.5.
- **Type consistency:** `ElohimTheme`/`ElohimLocale`/`ServingContext` names consistent across tasks; `getThemeStore()`/`getLocaleStore()` used uniformly; testids `protocol-omni-env`, `nav-theme-inline`, `nav-theme-toggle`, `nav-language`, `footer-lamad-link` consistent between unit tests, templates, and a2o.
- **Known soft spots (engineer judgment expected):** the `requiresLogicalProperties` signature (check `testing/i18n.ts` before Task 3), the existing protocol-omni spec's TestBed idiom (mirror it in Task 10), and the a2o browser-step World accessor (read neighbors in Task 14).
