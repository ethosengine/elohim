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

export const { getLocale, setLocale } = configureLocalization({
  sourceLocale,
  targetLocales: [...targetLocales],
  loadLocale: async (locale: string) => {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-return -- dynamic locale import; shape verified at runtime by lit-localize
    return import(`./generated/${locale}.js`);
  },
});
