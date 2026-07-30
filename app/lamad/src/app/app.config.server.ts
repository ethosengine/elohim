import { FetchBackend, HttpBackend } from '@angular/common/http';
import {
  ApplicationConfig,
  mergeApplicationConfig,
  provideZonelessChangeDetection,
} from '@angular/core';
import { provideClientHydration, withNoIncrementalHydration } from '@angular/platform-browser';
import { provideServerRendering } from '@angular/platform-server';

import { appConfig } from './app.config';

// Server-side rendering config for the elohim-render V8 runtime.
//
// This mirrors app/elohim-app/src/app/app.config.server.ts — the canonical
// EPR-app SSR pattern (see that file for the full rationale on each provider).
// The short form:
//
// - provideZonelessChangeDetection(): SSR stability runs on PendingTasks alone.
//   With a live Zone, `ZoneStablePendingTask` bridges never-completing polling
//   macrotasks into PendingTasks and `ApplicationRef.whenStable()` never
//   resolves in the deno_core runtime. Zoneless has no Zone macrotask bridge, so
//   only explicit contributors count — router navigation and HTTP requests.
//   (Public API since 20.2; replaces the previous `ɵNoopNgZone` +
//   `ɵREQUESTS_CONTRIBUTE_TO_STABILITY=false` private-symbol pair.)
// - FetchBackend: HttpXhrBackend lazy-imports 'xhr2', which strands a pending
//   dynamic module evaluation in the V8 event loop; globalThis.fetch is shimmed
//   by the runtime (elohim-render DataFetcher) and fails fast instead. Under
//   zoneless this also keeps HTTP out of the stability path by settling fast,
//   since requests DO contribute to stability here.
//
// The browser build is unaffected — these overrides exist only in the merged
// server config, and the browser keeps `provideZoneChangeDetection()` from
// appConfig (the NG0408 "both provided" notice is ngDevMode-only).
const serverConfig: ApplicationConfig = {
  providers: [
    provideServerRendering(),
    provideClientHydration(withNoIncrementalHydration()),
    provideZonelessChangeDetection(),
    { provide: HttpBackend, useClass: FetchBackend },
  ],
};

export const config = mergeApplicationConfig(appConfig, serverConfig);
