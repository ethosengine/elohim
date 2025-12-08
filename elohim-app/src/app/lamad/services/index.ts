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

// Kuzu embedded database (WASM) - re-exported from elohim
export { KuzuDataService } from '@app/elohim/services/kuzu-data.service';

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

// Local source chain (Holochain-style agent-centric storage) - re-exported from elohim
export { LocalSourceChainService } from '@app/elohim/services/local-source-chain.service';

// Progress migration
export { ProgressMigrationService } from './progress-migration.service';

// ============================================================================
// RE-EXPORTS FROM ELOHIM (Protocol-Core)
// ============================================================================

// Data loading
export { DataLoaderService } from '@app/elohim/services/data-loader.service';

// Also export data types for consumers
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

// Agent management
export { AgentService } from '@app/elohim/services/agent.service';

// Elohim agent (AI guardians)
export { ElohimAgentService } from '@app/elohim/services/elohim-agent.service';

// Trust indicators
export { TrustBadgeService } from '@app/elohim/services/trust-badge.service';

// ============================================================================
// RE-EXPORTS FROM IMAGODEI (Identity)
// ============================================================================

// Session human (MVP temporary identity)
export { SessionHumanService } from '@app/imagodei/services/session-human.service';

// Profile (human-centered identity view, Imago Dei aligned)
export { ProfileService } from '@app/imagodei/services/profile.service';

// ============================================================================
// RE-EXPORTS FROM QAHAL (Community)
// ============================================================================

// Learning state / affinity tracking
export { AffinityTrackingService } from '@app/elohim/services/affinity-tracking.service';

// Human consent (graduated intimacy)
export { HumanConsentService } from '@app/qahal/services/human-consent.service';

// Governance (challenges, proposals, precedents, deliberation)
export { GovernanceService } from '@app/qahal/services/governance.service';
export type { ChallengeSubmission, ProposalSubmission, Vote, DiscussionMessage } from '@app/qahal/services/governance.service';
