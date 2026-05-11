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
export { GovernanceApiService } from './governance-api.service';
export { ContentAttestationApiService } from './content-attestation-api.service';
export { AttestationApiService } from './attestation-api.service';
export type { IssueAttestationInput, RevokeAttestationInput } from './attestation-api.service';
export { GovernanceActionApiService } from './governance-action-api.service';
export type {
  ProposeGovernanceActionInput,
  VoteInput,
  GovernanceActionWithChildrenView,
} from './governance-action-api.service';
export { LearnerBackendApiService } from './learner-backend-api.service';

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

// Gate evaluation (mutation interceptor responses)
export { GateService } from './gate.service';
export { GateInteractionService } from './gate-interaction.service';
export type { GateArtifactState, ReachTier, MutationContext } from './gate-interaction.service';

// Lens registry (cross-pillar resource exploration)
export { LensRegistryService } from './lens-registry.service';

// Diagnostic collection (issue reporting)
export { DiagnosticCollectorService } from './diagnostic-collector.service';
export type { DiagnosticBundle } from './diagnostic-collector.service';

// Issue report service (content-node API)
export { IssueReportService } from './issue-report.service';
export type { IssueReportInput, ResolutionStatus } from './issue-report.service';

// SSE event streaming
export { EventStreamService } from './event-stream.service';

// Kairos temporal scheduling
export { ScheduleService } from './schedule.service';

// Service Worker bridge (HTML5 app cache invalidation)
export { SwBridgeService } from './sw-bridge.service';
