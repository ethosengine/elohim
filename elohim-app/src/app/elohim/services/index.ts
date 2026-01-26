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

// Cross-pillar services (formerly in shared/)
export { ProfileService } from './profile.service';
export { HumanConsentService } from './human-consent.service';
export { GovernanceService } from './governance.service';
export { AffinityTrackingService } from './affinity-tracking.service';
