/**
 * Barrel export for Lamad services
 *
 * Service Categories:
 * - Data Loading: DataLoaderService (JSON data access, Holochain adapter point)
 * - Path: PathService (learning path navigation)
 * - Content: ContentService (content access with reach checking)
 * - Agent: AgentService, ElohimAgentService (agent state and AI guardians)
 * - Exploration: ExplorationService (attestation-gated graph traversal)
 * - Maps: KnowledgeMapService (polymorphic knowledge maps)
 * - Extensions: PathExtensionService (learner path mutations)
 * - Trust: TrustBadgeService (UI-ready trust indicators)
 * - Search: SearchService (enhanced search with scoring and facets)
 * - Affinity: AffinityTrackingService (learner engagement tracking)
 */

// Data loading
export { DataLoaderService } from './data-loader.service';

// Path navigation
export { PathService } from './path.service';

// Content access
export { ContentService } from './content.service';

// Agent management
export { AgentService } from './agent.service';
export { ElohimAgentService } from './elohim-agent.service';

// Graph exploration
export { ExplorationService } from './exploration.service';

// Knowledge maps
export { KnowledgeMapService } from './knowledge-map.service';

// Path extensions
export { PathExtensionService } from './path-extension.service';

// Trust indicators
export { TrustBadgeService } from './trust-badge.service';

// Search
export { SearchService } from './search.service';

// Learning state
export { AffinityTrackingService } from './affinity-tracking.service';

// Session user (MVP temporary identity)
export { SessionUserService } from './session-user.service';

// Profile (human-centered identity view, Imago Dei aligned)
export { ProfileService } from './profile.service';
