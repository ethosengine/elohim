import { provideHttpClient, withInterceptors } from '@angular/common/http';
import { ApplicationConfig, provideZoneChangeDetection, importProvidersFrom } from '@angular/core';
import { provideRouter } from '@angular/router';

// @coverage: 100.0% (2026-02-24)

import { environment } from '../environments/environment';

import { routes } from './app.routes';
import { apiBaseUrlInterceptor } from './elohim/interceptors/api-base-url.interceptor';
import { CONTENT_ATTESTATION } from '@elohim/service';
import { GOVERNANCE } from '@elohim/service';
import { ELOHIM_ENV } from '@elohim/service';
import { provideElohimClient, detectClientMode } from './elohim/providers/elohim-client.provider';
import { ContentAttestationApiService } from './elohim/services/content-attestation-api.service';
import { CustodianCommitmentService } from './elohim/services/custodian-commitment.service';
import { CustodianMetricsReporterService } from './elohim/services/custodian-metrics-reporter.service';
import { CustodianSelectionService } from './elohim/services/custodian-selection.service';
import { GovernanceApiService } from '@elohim/service';
import { BLOB_FETCHER } from '@elohim/service';
import { HeliaFetchService } from './elohim/services/helia-fetch.service';
import { HolochainClientService } from './elohim/services/holochain-client.service';
import { PerformanceMetricsService } from './elohim/services/performance-metrics.service';
import { ContentIOModuleWithPlugins } from '@app/lamad/content-io/content-io.module';
import { LAMAD_HOLOCHAIN_CLIENT } from '@app/lamad/interfaces/cross-pillar.interface';
import { ECONOMIC_EVENT_FACTORY, EVENT_API } from '@elohim/rea-runtime';
import { EconomicEventsApiService } from './shefa/services/economic-events-api.service';
import { StorageApiService } from './elohim/services/storage-api.service';

/**
 * Resolve the doorway URL at runtime.
 *
 * When served as a projected /apps/{slug}/ bundle from doorway (alpha or prod),
 * window.location.origin IS the doorway — use it so API calls route to the correct
 * instance without baking a specific hostname into the build.
 *
 * In local dev (hostname === 'localhost') the Angular dev-server proxy forwards
 * /api, /db, /blob to doorway at :8888 — honour the configured URL so the proxy
 * keeps working.
 *
 * Tauri mode: detectClientMode() returns type:'tauri' regardless of doorwayUrl;
 * the value is passed through as an optional fallback and does not block boot.
 */
function resolveDoorwayUrl(configured: string | undefined): string {
  if (typeof window !== 'undefined' && window.location.hostname !== 'localhost') {
    return window.location.origin;
  }
  return configured ?? 'http://localhost:8888';
}

export const appConfig: ApplicationConfig = {
  providers: [
    provideZoneChangeDetection({ eventCoalescing: true }),
    provideRouter(routes),
    provideHttpClient(withInterceptors([apiBaseUrlInterceptor])),
    // ElohimClient - mode-aware content client (browser via doorway, tauri via local storage)
    // doorwayUrl resolves to window.location.origin when served from a projected bundle,
    // keeping API calls co-origin with the serving doorway (alpha or prod).
    ...provideElohimClient({
      mode: detectClientMode({
        doorwayUrl: resolveDoorwayUrl(environment.client?.doorwayUrl),
        doorwayFallbacks: environment.client?.doorwayFallbacks,
        apiKey: environment.client?.apiKey,
        nodeUrls: environment.client?.nodeUrls,
        storageUrl: environment.client?.storageUrl,
      }),
      // Holochain connection for agent-centric data (attestations, identity, points)
      holochain: environment.client?.holochainHAppId
        ? {
            hAppId: environment.client.holochainHAppId,
            enabled: true,
            directConductorUrl: environment.client.holochainConductorUrl,
          }
        : undefined,
    }),
    // Import ContentIO module with built-in format plugins (Markdown, Gherkin)
    importProvidersFrom(ContentIOModuleWithPlugins),
    // ELOHIM_ENV — maps app environment to the @elohim/service token contract
    // doorwayUrl uses the same origin-aware resolution as the ElohimClient above.
    {
      provide: ELOHIM_ENV,
      useValue: {
        production: environment.production,
        doorwayUrl: resolveDoorwayUrl(environment.client?.doorwayUrl),
        holochain: environment.client?.holochainConductorUrl
          ? {
              adminUrl: environment.client.holochainConductorUrl,
              appUrl: environment.client.holochainConductorUrl,
            }
          : undefined,
      },
    },
    // API boundary services wired to InjectionToken contracts
    { provide: BLOB_FETCHER, useClass: HeliaFetchService },
    { provide: GOVERNANCE, useExisting: GovernanceApiService },
    { provide: CONTENT_ATTESTATION, useExisting: ContentAttestationApiService },
    // Wire rea-runtime's ECONOMIC_EVENT_FACTORY token to the shefa concrete service.
    // Cross-pillar consumers (lamad signal-harness) inject via @elohim/rea-runtime's
    // ECONOMIC_EVENT_FACTORY token, not the shefa-local one, to avoid @app/shefa imports.
    { provide: ECONOMIC_EVENT_FACTORY, useExisting: EconomicEventsApiService },
    // M-REA-1: Wire rea-runtime's EVENT_API token to StorageApiService.
    // The EventService from @elohim/rea-runtime delegates emitEvent() to
    // eventApi.emitLamadIntent() which calls POST /api/v1/lamad/events —
    // the conductor-first intent path where the substrate composes REA shape.
    { provide: EVENT_API, useExisting: StorageApiService },
    // LAMAD_HOLOCHAIN_CLIENT — cross-pillar token consumed by BlobBootstrapService
    // (app.component.ts) and ContentMasteryService (session-migration.service.ts).
    // HolochainClientService is providedIn:'root' in @app/elohim; this useExisting
    // binding wires the lamad narrow interface to the concrete elohim-app service.
    { provide: LAMAD_HOLOCHAIN_CLIENT, useExisting: HolochainClientService },
    // Shefa metrics and custodian selection services
    CustodianCommitmentService,
    PerformanceMetricsService,
    CustodianSelectionService,
    CustodianMetricsReporterService,
  ],
};
