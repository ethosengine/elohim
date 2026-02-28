/**
 * Elohim Services - Protocol Core Services
 *
 * Infrastructure and cross-pillar services.
 * The Elohim pillar owns all services used across multiple pillars.
 */

// Infrastructure
export { DataLoaderService } from './data-loader.service';
export { LocalSourceChainService } from './local-source-chain.service';
export { HolochainClientService } from './holochain-client.service';
export { HolochainContentService } from './holochain-content.service';
export { LearnerBackendService } from './learner-backend.service';

// Agent & Trust
export { AgentService } from './agent.service';
export { ElohimAgentService } from './elohim-agent.service';
export { TrustBadgeService } from './trust-badge.service';

// Banner notification system
export { BannerService } from './banner.service';

// Elohim backend abstraction
export { ElohimBackendCatalog, MockBackend } from './elohim-backend';
export type { ElohimBackend, ElohimBackendType } from './elohim-backend';

// Elohim presence orchestrator
export { ElohimPresenceService } from './elohim-presence.service';

// Context assembly (reach negotiation orchestrator)
export { ContextAssemblyService } from './context-assembly.service';
export type { ContextAssemblyResult, AssemblyOptions } from './context-assembly.service';

// EPR resolution
export {
  EprResolverService,
  isContentAddress,
  normalizeContentAddress,
} from './epr-resolver.service';
export type {
  ResolvedEpr,
  ResolvedContent,
  ContextResolvedRoute,
  StepRef,
  CrossPathMatch,
} from './epr-resolver.service';

// Helia blob fetch (IBlobFetcher implementation)
export { HeliaFetchService } from './helia-fetch.service';

// Cross-pillar services (formerly in shared/)
export { ProfileService } from './profile.service';
export { HumanConsentService } from './human-consent.service';
export { GovernanceService } from './governance.service';
export { AffinityTrackingService } from './affinity-tracking.service';
