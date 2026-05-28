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
import { PerformanceMetricsService } from './elohim/services/performance-metrics.service';
import { ContentIOModuleWithPlugins } from '@app/lamad/content-io/content-io.module';
import { ECONOMIC_EVENT_FACTORY, EVENT_API } from '@elohim/rea-runtime';
import { EconomicEventsApiService } from './shefa/services/economic-events-api.service';
import { StorageApiService } from './elohim/services/storage-api.service';

export const appConfig: ApplicationConfig = {
  providers: [
    provideZoneChangeDetection({ eventCoalescing: true }),
    provideRouter(routes),
    provideHttpClient(withInterceptors([apiBaseUrlInterceptor])),
    // ElohimClient - mode-aware content client (browser via doorway, tauri via local storage)
    ...provideElohimClient({
      mode: detectClientMode({
        doorwayUrl: environment.client?.doorwayUrl,
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
    {
      provide: ELOHIM_ENV,
      useValue: {
        production: environment.production,
        doorwayUrl: environment.client?.doorwayUrl,
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
    // Shefa metrics and custodian selection services
    CustodianCommitmentService,
    PerformanceMetricsService,
    CustodianSelectionService,
    CustodianMetricsReporterService,
  ],
};
