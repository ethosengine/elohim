import '@analogjs/vitest-angular/setup-zone';
import 'fake-indexeddb/auto';

// jsdom lacks matchMedia — stub it for component tests that check prefers-color-scheme
Object.defineProperty(globalThis, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
});

import { getTestBed } from '@angular/core/testing';
// platformBrowserTesting() (unlike the deprecated platformBrowserDynamicTesting()) does
// NOT pull in the JIT compiler implicitly — specs that JIT-compile injectables/components
// fail with "needs to be compiled using the JIT compiler" without this import.
import '@angular/compiler';
import { BrowserTestingModule, platformBrowserTesting } from '@angular/platform-browser/testing';

getTestBed().initTestEnvironment(BrowserTestingModule, platformBrowserTesting());
