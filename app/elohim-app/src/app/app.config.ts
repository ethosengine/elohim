import { provideHttpClient, withInterceptors } from '@angular/common/http';
import { ApplicationConfig, provideZoneChangeDetection, importProvidersFrom } from '@angular/core';
import { provideRouter } from '@angular/router';

// @coverage: 100.0% (2026-02-24)

import { environment } from '../environments/environment';

import { routes } from './app.routes';
import { apiBaseUrlInterceptor } from './elohim/interceptors/api-base-url.interceptor';
import { GOVERNANCE } from '@elohim/service';
import { ELOHIM_ENV } from '@elohim/service';
import {
  provideElohimClient,
  detectClientMode,
  ELOHIM_CLIENT as LOCAL_ELOHIM_CLIENT,
} from './elohim/providers/elohim-client.provider';
import {
  provideEprResolution,
  EPR_RESOLUTION_PROVIDER,
} from './elohim/providers/epr-resolution.provider';
import { ELOHIM_CLIENT } from '@elohim/service';
import { CustodianCommitmentService } from './elohim/services/custodian-commitment.service';
import { CustodianMetricsReporterService } from './elohim/services/custodian-metrics-reporter.service';
import { CustodianSelectionService } from './elohim/services/custodian-selection.service';
import { GovernanceApiService } from '@elohim/service';
import { BLOB_FETCHER } from '@elohim/service';
import { HeliaFetchService } from './elohim/services/helia-fetch.service';
import { HolochainClientService } from './elohim/services/holochain-client.service';
import { PerformanceMetricsService } from './elohim/services/performance-metrics.service';
import { ContentIOModuleWithPlugins } from '@app/lamad/content-io/content-io.module';
import {
  LAMAD_HOLOCHAIN_CLIENT,
  LAMAD_AFFINITY_TRACKING,
  LAMAD_EPR_RESOLVER,
  LAMAD_EPR_NAV,
  LAMAD_GOVERNANCE,
  LAMAD_GOVERNANCE_SIGNAL,
  LAMAD_CONTEXT_ASSEMBLY,
  LAMAD_EPR_RESOLUTION_PROVIDER,
} from '@app/lamad/interfaces/cross-pillar.interface';
import { LAMAD_AGENT } from '@app/lamad/interfaces/agent.interface';
import { LAMAD_STORAGE_API, LAMAD_STORAGE_CLIENT } from '@app/lamad/interfaces/storage.interface';
import { LEARNER_BACKEND } from '@app/lamad/interfaces/learner-backend.interface';
import { LearnerBackendApiService } from '@app/lamad/services/learner-backend-api.service';
import { ECONOMIC_EVENT_FACTORY, EVENT_API, AGENT_CONTEXT } from '@elohim/rea-runtime';
import {
  BUNDLE_ROUTE_CONTEXT,
  claimsFromDeclaration,
  type BundleRouteContext,
} from '@elohim/service';
import { CONTENT_SYNC_STORAGE_BASE_URL } from '@elohim/service';
import { AffinityTrackingService } from './elohim/services/affinity-tracking.service';
import { AgentService } from './elohim/services/agent.service';
import { ContextAssemblyService } from './elohim/services/context-assembly.service';
import { EprNavService } from './elohim/services/epr-nav.service';
import { EprResolverService } from './elohim/services/epr-resolver.service';
import { GovernanceSignalService } from './elohim/services/governance-signal.service';
import { GovernanceService } from './elohim/services/governance.service';
import { StorageApiService } from './elohim/services/storage-api.service';
import { StorageClientService } from './elohim/services/storage-client.service';
import { ELOHIM_OWNS_UNIVERSAL_ROUTE, ELOHIM_ROUTE_CLAIMS } from './generated/route-claims';
import { EconomicEventsApiService } from './shefa/services/economic-events-api.service';

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
  // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside typeof window guard
  if (typeof window !== 'undefined' && window.location.hostname !== 'localhost') {
    // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside typeof window guard
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
        doorwayIdentity: environment.client?.doorwayIdentity ?? environment.client?.doorwayUrl,
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
    // CONTENT_ATTESTATION provider retired (attestation-consolidation Phase-2a):
    // the elohim-app TrustBadgeService twin had no rendering consumer and is removed;
    // the live trust-badge read (lamad) now uses AttestationApiService directly.
    // Wire rea-runtime's ECONOMIC_EVENT_FACTORY token to the shefa concrete service.
    // Cross-pillar consumers (lamad signal-harness) inject via @elohim/rea-runtime's
    // ECONOMIC_EVENT_FACTORY token, not the shefa-local one, to avoid @app/shefa imports.
    { provide: ECONOMIC_EVENT_FACTORY, useExisting: EconomicEventsApiService },
    // M-REA-1: Wire rea-runtime's EVENT_API token to StorageApiService.
    // The EventService from @elohim/rea-runtime delegates emitEvent() to
    // eventApi.emitLamadIntent() which calls POST /api/v1/lamad/events —
    // the conductor-first intent path where the substrate composes REA shape.
    { provide: EVENT_API, useExisting: StorageApiService },
    // §12.3: the shell claims nothing by type — it OWNS the universal /epr
    // route, so every unclaimed EPR resolves in-shell to ['/epr', id].
    // Declared in elohim/sdk/domains/elohim/manifest.json (I3) — generated,
    // never hand-edited here.
    {
      provide: BUNDLE_ROUTE_CONTEXT,
      useValue: {
        claims: claimsFromDeclaration(ELOHIM_ROUTE_CLAIMS),
        ownsUniversalRoute: ELOHIM_OWNS_UNIVERSAL_ROUTE,
      } satisfies BundleRouteContext,
    },
    // I2: the single ambient EprResolutionProvider, baking THIS bundle's
    // BUNDLE_ROUTE_CONTEXT above so `resolveRoute` is claims-correct. Injected by
    // the four resolution consumers (Angular) and installed at the shell root as
    // the Lit @lit/context provider (app.component) for every <elohim-*> element.
    provideEprResolution(),
    // LAMAD_HOLOCHAIN_CLIENT — cross-pillar token consumed by BlobBootstrapService
    // (app.component.ts) and ContentMasteryService (session-migration.service.ts).
    // HolochainClientService is providedIn:'root' in @app/elohim; this useExisting
    // binding wires the lamad narrow interface to the concrete elohim-app service.
    { provide: LAMAD_HOLOCHAIN_CLIENT, useExisting: HolochainClientService },
    // §12.3 cross-pillar viewer chain — the shell OWNS the universal /epr and
    // /resource routes (app.routes.ts), which lazy-load @app/lamad's
    // ContentViewerComponent. That component + its transitive service chain inject
    // the lamad narrow tokens below, so the shell's composition root must provide
    // them exactly as the lamad bundle does (mirrors app/lamad/src/app/app.config.ts).
    // Each concrete class is providedIn:'root' in @app/elohim; useExisting bridges
    // the narrow lamad interface to the shell-resolved instance (same pattern as
    // LAMAD_HOLOCHAIN_CLIENT above). Only the tokens the viewer chain reaches are
    // provided here — not the full lamad cross-pillar catalog.
    //
    // LAMAD_AFFINITY_TRACKING — content-viewer.component (direct inject) +
    // hierarchical-graph / path-negotiation services.
    { provide: LAMAD_AFFINITY_TRACKING, useExisting: AffinityTrackingService },
    // LAMAD_AGENT — content-viewer (direct) + content/trust-badge/signal-harness.
    { provide: LAMAD_AGENT, useExisting: AgentService },
    // LAMAD_GOVERNANCE / LAMAD_GOVERNANCE_SIGNAL — content-viewer (direct inject).
    { provide: LAMAD_GOVERNANCE, useExisting: GovernanceService },
    { provide: LAMAD_GOVERNANCE_SIGNAL, useExisting: GovernanceSignalService },
    // LAMAD_EPR_RESOLVER / LAMAD_EPR_NAV — content-viewer (direct) + markdown-renderer.
    { provide: LAMAD_EPR_RESOLVER, useExisting: EprResolverService },
    { provide: LAMAD_EPR_NAV, useExisting: EprNavService },
    // LAMAD_EPR_RESOLUTION_PROVIDER — the markdown renderer (viewer chain) consumes
    // the single provider through this narrow token; bridge it to the shell's
    // concrete EPR_RESOLUTION_PROVIDER (mirrors app/lamad/src/app/app.config.ts).
    { provide: LAMAD_EPR_RESOLUTION_PROVIDER, useExisting: EPR_RESOLUTION_PROVIDER },
    // LAMAD_STORAGE_CLIENT — content-viewer (direct) + resilience / household-
    // resilience / projection-api / content-backend services in the viewer chain.
    { provide: LAMAD_STORAGE_CLIENT, useExisting: StorageClientService },
    // CONTENT_SYNC_STORAGE_BASE_URL — the connection seam for @elohim/service's
    // ContentDocSyncService (Automerge content-sync plane). The library defaults
    // to origin-relative; the shell wires it to the live connection strategy so
    // the doc-sync `/sync/v1/...` calls honour doorway-vs-Tauri-vs-direct (Tauri's
    // local sidecar is non-origin) exactly as the rest of the storage surface does.
    {
      provide: CONTENT_SYNC_STORAGE_BASE_URL,
      useFactory: (storage: StorageClientService) => () => storage.getStorageBaseUrl(),
      deps: [StorageClientService],
    },
    // LAMAD_STORAGE_API — StewardshipAllocationService (injected by content-viewer).
    { provide: LAMAD_STORAGE_API, useExisting: StorageApiService },
    // LAMAD_CONTEXT_ASSEMBLY — ContentEditorService (injected by content-viewer for
    // edit/reach-negotiation surfaces) injects this token. ContextAssemblyService is
    // providedIn:'root' in @app/elohim and pulls only concrete root services.
    { provide: LAMAD_CONTEXT_ASSEMBLY, useExisting: ContextAssemblyService },
    // LEARNER_BACKEND — the markdown-renderer (and sophia-renderer) injects PathService,
    // which reaches ContentMasteryService/PointsService/PracticeService; all three inject the
    // lamad-local LEARNER_BACKEND token. That token carries NO providedIn factory (lamad's
    // app.config supplies the concrete class), so the shell injector must provide it too or the
    // viewer chain throws NullInjectorError and content never renders. LearnerBackendApiService
    // is providedIn:'root' and injects only HttpClient; useExisting bridges to that root instance
    // (mirrors app/lamad/src/app/app.config.ts).
    { provide: LEARNER_BACKEND, useExisting: LearnerBackendApiService },
    // AGENT_CONTEXT (rea-runtime) — AttentionTrackerService injects this token and is
    // injected directly by content-viewer.component. AgentService satisfies
    // IAgentContext via getCurrentAgentId() (mirrors lamad app.config.ts).
    { provide: AGENT_CONTEXT, useExisting: AgentService },
    // §12.3 cross-pillar viewer chain — lamad's ContentBackendService (reached via
    // DataLoaderService -> ProjectionAPIService) injects the LIBRARY @elohim/service
    // ELOHIM_CLIENT token, whereas this shell's provideElohimClient registers the
    // LOCAL @app/elohim provider token. The two tokens share the description
    // 'ElohimClient' but are distinct object references — this useExisting bridges
    // the library token to the shell-resolved local instance so both consumers share
    // one ElohimClient (the mirror-image of lamad app.config.ts's local->library alias).
    { provide: ELOHIM_CLIENT, useExisting: LOCAL_ELOHIM_CLIENT },
    // Shefa metrics and custodian selection services
    CustodianCommitmentService,
    PerformanceMetricsService,
    CustodianSelectionService,
    CustodianMetricsReporterService,
  ],
};
