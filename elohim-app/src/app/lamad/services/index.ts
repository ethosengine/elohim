/**
 * Barrel export for Lamad services
 *
 * Lamad is the Content/Learning pillar of the Elohim Protocol.
 * This barrel re-exports services from other pillars for backward compatibility.
 *
 * Pillar Structure:
 * - elohim/: Protocol-core (data loading, agents, trust)
 * - imagodei/: Identity (session, profile)
 * - lamad/: Content (paths, content, exploration, maps)
 * - qahal/: Community (affinity, consent, governance)
 * - shefa/: Economy (future)
 */

// ============================================================================
// LAMAD-SPECIFIC SERVICES (Content/Learning)
// ============================================================================

// Path navigation
export { PathService } from './path.service';

// Content access
export { ContentService } from './content.service';

// Graph exploration
export { ExplorationService } from './exploration.service';

// Knowledge maps
export { KnowledgeMapService } from './knowledge-map.service';

// Path extensions
export { PathExtensionService } from './path-extension.service';

// Path-graph integration (paths as ContentNodes)
export { PathGraphService } from './path-graph.service';

// Search
export { SearchService } from './search.service';

// Assessments (psychometric instruments, self-knowledge)
export { AssessmentService } from './assessment.service';
export type { AssessmentResult, AssessmentSession, QuestionResponse } from './assessment.service';

// Path negotiation
export { PathNegotiationService } from './path-negotiation.service';

// Content mastery
export { ContentMasteryService } from './content-mastery.service';
export type { MigrationResult } from './content-mastery.service';

// Practice pool and challenges (Khan Academy-style)
export { PracticeService } from './practice.service';

// Learning points (Shefa integration)
export { PointsService } from './points.service';

/** @deprecated Import from '@app/elohim/services/local-source-chain.service' */
export { LocalSourceChainService } from '@app/elohim/services/local-source-chain.service';

// Progress migration
export { ProgressMigrationService } from './progress-migration.service';

// ============================================================================
// LAMAD STEWARD ECONOMY SERVICES
// ============================================================================

// Contributor dashboard & impact tracking
export { ContributorService } from './contributor.service';

// Steward economy (credentials, gates, access, revenue)
export { StewardService } from './steward.service';

// ============================================================================
// RE-EXPORTS FROM ELOHIM (Protocol-Core)
// TODO: Remove these re-exports. Import directly from @app/elohim/services/*
// ============================================================================

/** @deprecated Import from '@app/elohim/services/data-loader.service' */
export { DataLoaderService } from '@app/elohim/services/data-loader.service';

/** @deprecated Import from '@app/elohim/services/data-loader.service' */
export type {
  AssessmentIndex,
  AssessmentIndexEntry,
  GovernanceIndex,
  ChallengeRecord,
  ProposalRecord,
  PrecedentRecord,
  DiscussionRecord,
  GovernanceStateRecord
} from '@app/elohim/services/data-loader.service';

/** @deprecated Import from '@app/elohim/services/agent.service' */
export { AgentService } from '@app/elohim/services/agent.service';

/** @deprecated Import from '@app/elohim/services/elohim-agent.service' */
export { ElohimAgentService } from '@app/elohim/services/elohim-agent.service';

/** @deprecated Import from '@app/elohim/services/trust-badge.service' */
export { TrustBadgeService } from '@app/elohim/services/trust-badge.service';

// ============================================================================
// RE-EXPORTS FROM IMAGODEI (Identity)
// TODO: Remove these re-exports. Import directly from @app/imagodei/services/*
// ============================================================================

/** @deprecated Import from '@app/imagodei/services/session-human.service' */
export { SessionHumanService } from '@app/imagodei/services/session-human.service';

/** @deprecated Import from '@app/elohim/services/profile.service' - Note: should move to imagodei */
export { ProfileService } from '@app/elohim/services/profile.service';

// ============================================================================
// RE-EXPORTS FROM QAHAL (Community)
// TODO: Remove these re-exports. Import directly from @app/qahal/services/* or @app/elohim/services/*
// ============================================================================

/** @deprecated Import from '@app/elohim/services/affinity-tracking.service' */
export { AffinityTrackingService } from '@app/elohim/services/affinity-tracking.service';

/** @deprecated Import from '@app/elohim/services/human-consent.service' */
export { HumanConsentService } from '@app/elohim/services/human-consent.service';

/** @deprecated Import from '@app/elohim/services/governance.service' */
export { GovernanceService } from '@app/elohim/services/governance.service';
/** @deprecated Import from '@app/elohim/services/governance.service' */
export type { ChallengeSubmission, ProposalSubmission, Vote, DiscussionMessage } from '@app/elohim/services/governance.service';
