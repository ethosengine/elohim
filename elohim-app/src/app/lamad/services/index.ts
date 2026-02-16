/**
 * Barrel export for Lamad services
 *
 * Lamad is the Content/Learning pillar of the Elohim Protocol.
 *
 * For services from other pillars, import directly:
 * - @app/elohim/services/* - Protocol-core (data loading, agents, trust)
 * - @app/imagodei/services/* - Identity (session, profile)
 * - @app/qahal/services/* - Community (affinity, consent, governance)
 * - @app/shefa/services/* - Economy
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

// Mastery service (elohim-storage backend)
export { MasteryService, MasteryLevels, MASTERY_LEVEL_ORDER } from './mastery.service';
export type { MasteryLevelType } from './mastery.service';

// Content relationships (elohim-storage backend)
export { RelationshipService } from './relationship.service';

// Practice pool and challenges (Khan Academy-style)
export { PracticeService } from './practice.service';

// Learning points (Shefa integration)
export { PointsService } from './points.service';

// Progress migration
export { ProgressMigrationService } from './progress-migration.service';

// Mastery stats aggregation (gamified dashboard)
export { MasteryStatsService } from './mastery-stats.service';

// ============================================================================
// LAMAD STEWARD ECONOMY SERVICES
// ============================================================================

// Stewardship allocations (one-to-many content stewardship)
export { StewardshipAllocationService } from './stewardship-allocation.service';
export type { StewardPortfolio, RecognitionDistribution } from './stewardship-allocation.service';

// Contributor dashboard & impact tracking
export { ContributorService } from './contributor.service';

// Steward economy (credentials, gates, access, revenue)
export { StewardService } from './steward.service';
