import { provideHttpClient } from '@angular/common/http';
import { ApplicationConfig, provideZoneChangeDetection } from '@angular/core';
import { provideRouter } from '@angular/router';
import { provideAnimations } from '@angular/platform-browser/animations';

import { routes } from './app.routes';
import {
  ELOHIM_ENV,
  ELOHIM_CLIENT,
  BLOB_FETCHER,
  GOVERNANCE,
  CONTENT_ATTESTATION,
  GovernanceApiService,
  provideElohimClient,
  detectClientMode,
} from '@elohim/service';
import { ECONOMIC_EVENT_FACTORY } from '@elohim/rea-runtime';
import { environment } from '../environments/environment';
import { LEARNER_BACKEND } from './interfaces/learner-backend.interface';
import { LearnerBackendApiService } from './services/learner-backend-api.service';
import { LAMAD_STORAGE_API, LAMAD_STORAGE_CLIENT } from './interfaces/storage.interface';
import { StorageApiService } from '@app/elohim/services/storage-api.service';
import { StorageClientService } from '@app/elohim/services/storage-client.service';
import { LAMAD_AGENT } from './interfaces/agent.interface';
import { AgentService } from '@app/elohim/services/agent.service';
import {
  LAMAD_HOLOCHAIN_CLIENT,
  LAMAD_AFFINITY_TRACKING,
  LAMAD_EPR_RESOLVER,
  LAMAD_GOVERNANCE_SIGNAL,
  LAMAD_ELOHIM_PRESENCE,
  LAMAD_HUMAN_CONSENT,
  LAMAD_GOVERNANCE,
  LAMAD_PROFILE,
  LAMAD_ELOHIM_AGENT,
  LAMAD_CONTEXT_ASSEMBLY,
  LAMAD_LENS_REGISTRY,
  LAMAD_IDENTITY,
} from './interfaces/cross-pillar.interface';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { AffinityTrackingService } from '@app/elohim/services/affinity-tracking.service';
import { EprResolverService } from '@app/elohim/services/epr-resolver.service';
import { GovernanceSignalService } from '@app/elohim/services/governance-signal.service';
import { ElohimPresenceService } from '@app/elohim/services/elohim-presence.service';
import { HumanConsentService } from '@app/elohim/services/human-consent.service';
import { GovernanceService } from '@app/elohim/services/governance.service';
import { ProfileService } from '@app/elohim/services/profile.service';
import { ElohimAgentService } from '@app/elohim/services/elohim-agent.service';
import { ContextAssemblyService } from '@app/elohim/services/context-assembly.service';
import { LensRegistryService } from '@app/elohim/services/lens-registry.service';
import { IdentityService } from '@app/imagodei/services/identity.service';
import { HeliaFetchService } from '@app/elohim/services/helia-fetch.service';
import { ContentAttestationApiService } from '@app/elohim/services/content-attestation-api.service';
import { EconomicEventsApiService } from '@app/shefa/services/economic-events-api.service';

/**
 * Resolve the doorway URL at runtime.
 *
 * The lamad SPA is served as a projected /apps/{slug}/ bundle from doorway —
 * meaning window.location.origin IS the doorway. Using it keeps all API calls
 * co-origin with the serving instance (alpha or prod) without baking a hostname.
 *
 * In local dev (hostname === 'localhost') the Angular dev-server proxy forwards
 * /api, /db, /blob to doorway at the configured port — honour the environment URL.
 */
function resolveDoorwayUrl(configured: string | undefined): string {
  if (typeof window !== 'undefined' && window.location.hostname !== 'localhost') {
    return window.location.origin;
  }
  return configured ?? 'http://localhost:8888';
}

/**
 * Lamad bundle application config.
 *
 * B18d: Provides HTTP client, router (wired to LAMAD_ROUTES via app.routes.ts),
 * and animations. The lamad bundle is served at /lamad/ with <base href="/lamad/">.
 *
 * B18d omnibar note:
 * When spec Task B6 lands <elohim-page-chrome>, the lamad toolbar becomes a
 * standalone component with `host: { 'slot': 'omnibar' }` that slots into the
 * page chrome custom element. For now, the <elohim-navigator> Lit element
 * (from elohim-core workspace, Slice 2.2b) provides the shell from within
 * lamad-layout.
 *
 * TODO(B18d-followup): Extract lamad toolbar as a standalone slot='omnibar'
 * component once elohim-page-chrome (Task B6) is available.
 */
export const appConfig: ApplicationConfig = {
  providers: [
    provideZoneChangeDetection({ eventCoalescing: true }),
    provideRouter(routes),
    provideHttpClient(),
    provideAnimations(),
    // ELOHIM_ENV — maps lamad-local environment to the @elohim/service token contract.
    // doorwayUrl uses the same origin-aware resolution as the ElohimClient below.
    {
      provide: ELOHIM_ENV,
      useValue: {
        production: environment.production,
        doorwayUrl: resolveDoorwayUrl(environment.client?.doorwayUrl),
      },
    },
    // ELOHIM_CLIENT — @elohim/service token consumed by ContentBackendService.
    // doorwayUrl resolves to window.location.origin when served as a projected bundle,
    // so API calls reach the correct doorway instance without a hardcoded hostname.
    ...provideElohimClient({
      mode: detectClientMode({
        doorwayUrl: resolveDoorwayUrl(environment.client?.doorwayUrl),
        apiKey: environment.client?.apiKey,
      }),
    }),
    // BLOB_FETCHER — HeliaFetchService provides CID-verified blob fetch with
    // HTTP doorway fallback. It only injects HttpClient + StorageClientService
    // (both present in this injector), and uses dynamic import() for @helia —
    // safe in a browser context.
    { provide: BLOB_FETCHER, useClass: HeliaFetchService },
    // GOVERNANCE — GovernanceApiService (from @elohim/service) is a thin HTTP
    // client that only injects HttpClient. Satisfies data-loader.service.ts.
    { provide: GOVERNANCE, useExisting: GovernanceApiService },
    // CONTENT_ATTESTATION — ContentAttestationApiService (from @app/elohim)
    // is a thin HTTP client that only injects HttpClient.
    { provide: CONTENT_ATTESTATION, useExisting: ContentAttestationApiService },
    // ECONOMIC_EVENT_FACTORY — EconomicEventsApiService (from @app/shefa)
    // is a thin HTTP client that only injects HttpClient. Satisfies
    // signal-harness.service.ts.
    { provide: ECONOMIC_EVENT_FACTORY, useExisting: EconomicEventsApiService },
    // LEARNER_BACKEND — concrete provider for the lamad-local learner backend token
    // (P-disposition: token + interface live in lamad/interfaces/; elohim-app's token retired)
    { provide: LEARNER_BACKEND, useClass: LearnerBackendApiService },
    // LAMAD_STORAGE_API / LAMAD_STORAGE_CLIENT — narrow tokens that decouple lamad services
    // from direct elohim-app class imports. The concrete classes remain in elohim-app (they
    // have significant elohim-app consumers) but lamad services inject via these tokens.
    // (Slice 2.1c P+inversion pattern)
    { provide: LAMAD_STORAGE_API, useExisting: StorageApiService },
    { provide: LAMAD_STORAGE_CLIENT, useExisting: StorageClientService },
    { provide: LAMAD_AGENT, useExisting: AgentService },
    // Cross-pillar P+inversion tokens — concrete classes stay in elohim-app due to
    // non-lamad deps (imagodei, qahal, CONNECTION_STRATEGY, PerformanceMetricsService, etc.)
    // app.config.ts is the composition root, so cross-pillar imports here are intentional.
    // (Slice 2.1c P+inversion pattern)
    { provide: LAMAD_HOLOCHAIN_CLIENT, useExisting: HolochainClientService },
    { provide: LAMAD_AFFINITY_TRACKING, useExisting: AffinityTrackingService },
    { provide: LAMAD_EPR_RESOLVER, useExisting: EprResolverService },
    { provide: LAMAD_GOVERNANCE_SIGNAL, useExisting: GovernanceSignalService },
    { provide: LAMAD_ELOHIM_PRESENCE, useExisting: ElohimPresenceService },
    { provide: LAMAD_HUMAN_CONSENT, useExisting: HumanConsentService },
    { provide: LAMAD_GOVERNANCE, useExisting: GovernanceService },
    { provide: LAMAD_PROFILE, useExisting: ProfileService },
    { provide: LAMAD_ELOHIM_AGENT, useExisting: ElohimAgentService },
    { provide: LAMAD_CONTEXT_ASSEMBLY, useExisting: ContextAssemblyService },
    { provide: LAMAD_LENS_REGISTRY, useExisting: LensRegistryService },
    { provide: LAMAD_IDENTITY, useExisting: IdentityService },
  ],
};
