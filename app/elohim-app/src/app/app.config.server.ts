import { ApplicationConfig, mergeApplicationConfig, NgZone, ɵNoopNgZone as NoopNgZone } from '@angular/core';
import { provideClientHydration } from '@angular/platform-browser';
import {
  FetchBackend,
  HttpBackend,
  ɵREQUESTS_CONTRIBUTE_TO_STABILITY as REQUESTS_CONTRIBUTE_TO_STABILITY,
} from '@angular/common/http';
import { provideServerRendering } from '@angular/platform-server';

import { appConfig } from './app.config';

const serverConfig: ApplicationConfig = {
  providers: [
    provideServerRendering(),
    provideClientHydration(),
    // In the deno_core SSR runtime, Zone.js tracks setInterval/setTimeout tasks
    // that never complete (e.g. health-check polling intervals, performance uptime
    // timers). These keep ZoneStablePendingTask from removing its entry from
    // PendingTasks, so ApplicationRef.whenStable() never resolves.
    //
    // Overriding NgZone with NoopNgZone makes ZoneStablePendingTask see
    // hasPendingMacrotasks=false immediately — its onStable/onUnstable subscriptions
    // become no-ops, so the PendingTasks bridge task is never added. Combined with
    // REQUESTS_CONTRIBUTE_TO_STABILITY=false, PendingTasks only tracks router
    // navigation, which completes normally once the initial route activates.
    //
    // The browser build retains Zone.js for change detection. The server build uses
    // a noop zone purely to unblock SSR stability detection via PendingTasks.
    //
    // TODO(ssr-runtime): wire the DataFetcher from elohim-render to Angular's
    // HttpClient so real content fetches work during SSR (Task 14+).
    { provide: NgZone, useClass: NoopNgZone },
    { provide: REQUESTS_CONTRIBUTE_TO_STABILITY, useValue: false },
    // Use FetchBackend instead of HttpXhrBackend for SSR. HttpXhrBackend.handle()
    // calls xhrFactory.ɵloadImpl() which does `await import('xhr2')`. In deno_core's
    // FsModuleLoader, bare specifier imports like 'xhr2' fail and leave pending dynamic
    // module evaluations in the V8 event loop — keeping poll_event_loop() in
    // Poll::Pending indefinitely. FetchBackend uses globalThis.fetch (shimmed to
    // reject immediately), so HTTP requests fail fast without hanging the event loop.
    //
    // The browser build is unaffected — this override only applies to the server config
    // merged via mergeApplicationConfig(appConfig, serverConfig).
    { provide: HttpBackend, useClass: FetchBackend },
  ],
};

export const config = mergeApplicationConfig(appConfig, serverConfig);
