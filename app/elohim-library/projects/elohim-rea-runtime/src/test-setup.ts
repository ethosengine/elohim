import '@analogjs/vitest-angular/setup-zone';

// platformBrowserTesting() (unlike the deprecated platformBrowserDynamicTesting()) does
// NOT pull in the JIT compiler implicitly — specs that JIT-compile injectables/components
// fail with "needs to be compiled using the JIT compiler" without this import.
import '@angular/compiler';
import { BrowserTestingModule, platformBrowserTesting } from '@angular/platform-browser/testing';
import { getTestBed } from '@angular/core/testing';

getTestBed().initTestEnvironment(BrowserTestingModule, platformBrowserTesting(), {
  teardown: { destroyAfterEach: true },
});
