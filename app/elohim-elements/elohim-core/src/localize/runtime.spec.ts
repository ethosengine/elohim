import { expect } from '@open-wc/testing';

// ---------------------------------------------------------------------------
// Regression: Vite SSR module-runner re-evaluation of runtime.ts must not
// throw (2026-07-02 dev-server crash — see runtime.ts's guard comment).
//
// Vite's SSR module runner re-executes a module's top-level scope on HMR
// without re-executing its unchanged dependencies (like `@lit/localize`
// itself). We reproduce that exact shape in a real browser: importing this
// module via two DIFFERENT URLs (cache-busting query params) forces the
// browser's ES module loader to create two DISTINCT instances of
// `runtime.ts`'s top-level scope, while the bare `@lit/localize` specifier
// still resolves to the SAME already-loaded module instance both times —
// which is exactly what made the second `configureLocalization()` call throw
// before the fix.
// ---------------------------------------------------------------------------

interface RuntimeModule {
  getLocale: () => string;
  setLocale: (locale: string) => Promise<void>;
}

describe('localize runtime — module re-evaluation idempotency', () => {
  it('re-evaluating the top-level module scope does not throw, and reuses the live configureLocalization instance', async () => {
    const first = (await import(
      /* @vite-ignore */ `./runtime.js?regression-cachebust=a`
    )) as RuntimeModule;
    // If configureLocalization() ran a second time here, @lit/localize would
    // throw "lit-localize can only be configured once" and this import would
    // reject — reproducing the observed dev-server crash.
    const second = (await import(
      /* @vite-ignore */ `./runtime.js?regression-cachebust=b`
    )) as RuntimeModule;

    // Both re-evaluated instances must expose WORKING getLocale/setLocale —
    // not just "didn't throw". Since the guard reuses the original
    // configureLocalization() result via globalThis, both should be the same
    // live closures.
    expect(second.getLocale).to.equal(first.getLocale);
    expect(second.setLocale).to.equal(first.setLocale);
    expect(first.getLocale()).to.equal('en');
  });
});
