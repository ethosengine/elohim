/**
 * Lit-localize runtime configuration for elohim-core.
 *
 * Source locale: en. Targets: es, he (he is the RTL canary).
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §8.2
 */

import { configureLocalization } from '@lit/localize';

export const sourceLocale = 'en';
export const targetLocales = ['es', 'he'] as const;

/**
 * WHY the globalThis guard: `configureLocalization()` is a configure-once API —
 * `@lit/localize` throws "lit-localize can only be configured once" on a
 * second call, because the "already installed" flag it checks lives inside
 * `@lit/localize`'s OWN module scope (untouched by anything we do here).
 *
 * Under `ng serve` + Vite's SSR module runner (`app/elohim-app`
 * `pnpm start:alpha`), a hot reload re-evaluates THIS module's top-level scope
 * whenever any elohim-core module in the import chain changes — but it does
 * NOT re-execute `@lit/localize` itself (only the invalidated chain is
 * re-run). So `@lit/localize`'s one-time "installed" flag survives across our
 * re-eval, and re-running `configureLocalization()` at module scope throws on
 * the second HMR pass, crashing the whole Vite SSR dev server (observed twice
 * 2026-07-02).
 *
 * A module-local guard (`let configured = false`) would NOT work: it resets
 * to `false` on every re-evaluation along with the rest of this module's
 * scope. `globalThis` is the one thing that survives module re-evaluation, so
 * we stash the `{ getLocale, setLocale }` result there on first configure and
 * reuse it on every subsequent re-eval instead of calling
 * `configureLocalization()` again.
 *
 * This also answers what the exports must be on a re-evaluated instance:
 * because we reuse the ORIGINAL result object, `getLocale`/`setLocale`
 * exported from a re-evaluated module are the exact same live closures wired
 * to `@lit/localize`'s internal state from the first (real) configuration —
 * so callers keep working correctly after a hot reload, not merely "the
 * process didn't crash."
 */
interface GlobalWithLocalizeRuntime {
  __ELOHIM_CORE_LOCALIZE_RUNTIME__?: ReturnType<typeof configureLocalization>;
}

const globalHost = globalThis as typeof globalThis & GlobalWithLocalizeRuntime;

export const { getLocale, setLocale } = (globalHost.__ELOHIM_CORE_LOCALIZE_RUNTIME__ ??=
  configureLocalization({
    sourceLocale,
    targetLocales: [...targetLocales],
    loadLocale: async (locale: string) => {
      // eslint-disable-next-line @typescript-eslint/no-unsafe-return -- dynamic locale import; shape verified at runtime by lit-localize
      return import(`./generated/${locale}.js`);
    },
  }));
