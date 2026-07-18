import { ApplicationConfig, mergeApplicationConfig, NgZone, ɵNoopNgZone as NoopNgZone } from '@angular/core';
import { provideClientHydration } from '@angular/platform-browser';
import {
  FetchBackend,
  HttpBackend,
  ɵREQUESTS_CONTRIBUTE_TO_STABILITY as REQUESTS_CONTRIBUTE_TO_STABILITY,
} from '@angular/common/http';
import { provideServerRendering } from '@angular/platform-server';

import { appConfig } from './app.config';

// Server-side rendering config for the elohim-render V8 runtime.
//
// This mirrors app/elohim-app/src/app/app.config.server.ts — the canonical
// EPR-app SSR pattern (see that file for the full rationale on each provider).
// The short form:
//
// - NoopNgZone: Zone.js tracks polling timers that never complete in the
//   deno_core runtime, so ApplicationRef.whenStable() never resolves without it.
// - REQUESTS_CONTRIBUTE_TO_STABILITY=false: PendingTasks then only tracks
//   router navigation, which completes once the initial route activates.
// - FetchBackend: HttpXhrBackend lazy-imports 'xhr2', which strands a pending
//   dynamic module evaluation in the V8 event loop; globalThis.fetch is shimmed
//   by the runtime (elohim-render DataFetcher) and fails fast instead.
//
// The browser build is unaffected — these overrides exist only in the merged
// server config.
const serverConfig: ApplicationConfig = {
  providers: [
    provideServerRendering(),
    provideClientHydration(),
    { provide: NgZone, useClass: NoopNgZone },
    { provide: REQUESTS_CONTRIBUTE_TO_STABILITY, useValue: false },
    { provide: HttpBackend, useClass: FetchBackend },
  ],
};

export const config = mergeApplicationConfig(appConfig, serverConfig);
