import { FetchBackend, HttpBackend } from '@angular/common/http';
import {
  ApplicationConfig,
  mergeApplicationConfig,
  provideZonelessChangeDetection,
} from '@angular/core';
import { provideClientHydration, withNoIncrementalHydration } from '@angular/platform-browser';
import { provideServerRendering } from '@angular/platform-server';

import { appConfig } from './app.config';

const serverConfig: ApplicationConfig = {
  providers: [
    provideServerRendering(),
    provideClientHydration(withNoIncrementalHydration()),
    // Zoneless change detection for the server render — public API (stable since
    // Angular 20.2), replacing two ɵ-prefixed private symbols this file used to
    // reach for (`ɵNoopNgZone` via `{provide: NgZone}` and
    // `ɵREQUESTS_CONTRIBUTE_TO_STABILITY = false`).
    //
    // The problem those hacks worked around: in the deno_core SSR runtime,
    // Zone.js tracks setInterval/setTimeout macrotasks that never complete
    // (health-check polling, performance uptime timers). `ZoneStablePendingTask`
    // bridges `NgZone.onUnstable` into `PendingTasks`, so with a live zone those
    // never-completing macrotasks keep the bridge task registered and
    // `ApplicationRef.whenStable()` never resolves — the render hangs.
    //
    // Under zoneless, stability is PendingTasks-only: there is no Zone macrotask
    // bridge at all, so a polling timer cannot register as pending work. Only
    // explicit contributors count — router navigation (resolves once the initial
    // route activates) and, unlike the old config, HTTP requests (zoneless keeps
    // REQUESTS_CONTRIBUTE_TO_STABILITY at its default `true`). That is why the
    // FetchBackend override below is still load-bearing rather than vestigial:
    // it makes every SSR request settle immediately instead of pending forever.
    //
    // Note `appConfig` still provides `provideZoneChangeDetection()` for the
    // browser — merging zoneless over it is intentional and wins (ZONELESS_ENABLED
    // and the NgZone binding both resolve to the later provider; the zone
    // ENVIRONMENT_INITIALIZERs remain but are inert against a NoopNgZone). The
    // NG0408 "both provided" notice is ngDevMode-only, so the production server
    // bundle is silent. The browser build is untouched and stays zone-based.
    //
    // TODO(ssr-runtime): wire the DataFetcher from elohim-render to Angular's
    // HttpClient so real content fetches work during SSR (Task 14+).
    provideZonelessChangeDetection(),
    // Use FetchBackend instead of HttpXhrBackend for SSR. HttpXhrBackend.handle()
    // calls xhrFactory.ɵloadImpl() which does `await import('xhr2')`. In deno_core's
    // FsModuleLoader, bare specifier imports like 'xhr2' fail and leave pending dynamic
    // module evaluations in the V8 event loop — keeping poll_event_loop() in
    // Poll::Pending indefinitely. FetchBackend uses globalThis.fetch (shimmed to
    // reject immediately), so HTTP requests fail fast without hanging the event loop.
    // Under zoneless this is doubly load-bearing: HTTP requests DO contribute to
    // stability, so a request that never settles would now stall whenStable().
    //
    // The browser build is unaffected — this override only applies to the server config
    // merged via mergeApplicationConfig(appConfig, serverConfig).
    { provide: HttpBackend, useClass: FetchBackend },
  ],
};

export const config = mergeApplicationConfig(appConfig, serverConfig);
