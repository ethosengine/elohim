/**
 * Barrel export for Lamad models
 *
 * Lamad is the Content/Learning pillar of the Elohim Protocol.
 * This barrel re-exports types from other pillars for backward compatibility.
 *
 * Pillar Structure:
 * - elohim/: Protocol-core (shared primitives, agents, attestations)
 * - imagodei/: Identity (profile, session, human nodes)
 * - lamad/: Content (nodes, paths, exploration, mastery)
 * - qahal/: Community (consent, governance, affinity, places)
 * - shefa/: Economy (REA, economic events, contributor presence)
 *
 * Six-layer architecture:
 * - Territory: ContentNode (the knowledge graph)
 * - Journey: LearningPath, PathStep (curated sequences)
 * - Traveler: Agent, AgentProgress (human state)
 * - Maps: KnowledgeMap (polymorphic: domain, self, person, collective)
 * - Economic: REA value flows and contributor recognition
 * - Governance: Constitutional moderation, deliberation, feedback profiles
 */

// ============================================================================
// LAMAD-SPECIFIC MODELS (Content/Learning)
// ============================================================================

// Territory models (Content Graph)
export type {
  ContentNode,
  ContentMetadata,
  ContentType,
  ContentFormat,
  ContentRelationship,
  ContentGraph,
  ContentGraphMetadata,
  ContentGraphMetadata as GraphMetadata,
  ContentReach,
  ContentFlag
} from './content-node.model';
export { ContentRelationshipType } from './content-node.model';

// Open Graph models (Platform-agnostic social sharing metadata)
export * from './open-graph.model';

// JSON-LD models (Linked Data structure alignment for semantic web)
export * from './json-ld.model';

// Verifiable Credentials models (W3C VC structure alignment for attestations)
export * from './verifiable-credential.model';

// Content Attestation models (Trust credentials granted TO content)
export * from './content-attestation.model';

// Journey models (Learning Path)
export * from './learning-path.model';

// Knowledge Map models (Polymorphic: Domain, Person, Collective)
export * from './knowledge-map.model';

// Path Extension models (Learner customization)
export * from './path-extension.model';

// Exploration models (Graph traversal and discovery)
export * from './exploration.model';

// Search models (enhanced search with scoring and facets)
export * from './search.model';

// Content Access models (visitor/member/attested tiers)
export * from './content-access.model';

// Content Mastery models (Bloom's Taxonomy progression)
export * from './content-mastery.model';

// Content Lifecycle models (refresh policies, deprecation, archival)
export * from './content-lifecycle.model';

// Feedback Profile models (virality as privilege, emotional reaction constraints)
export * from './feedback-profile.model';

// Path Negotiation models (collaborative path creation)
export * from './path-negotiation.model';

// Source Chain models (Holochain-style agent-centric storage)
export * from './source-chain.model';

// Human Node models (humans in the graph - stays in lamad for graph integration)
export * from './human-node.model';

// ============================================================================
// RE-EXPORTS FROM ELOHIM (Protocol-Core)
// ============================================================================

// Protocol-core primitives
export * from '@app/elohim/models/protocol-core.model';

// Agent models (Traveler state)
export * from '@app/elohim/models/agent.model';

// Agent Attestation models (Credentials earned BY humans/agents)
export * from '@app/elohim/models/attestations.model';

// Elohim Agent models (Autonomous constitutional guardians)
export * from '@app/elohim/models/elohim-agent.model';

// Trust Badge models (UI-ready trust display)
export * from '@app/elohim/models/trust-badge.model';

// ============================================================================
// RE-EXPORTS FROM IMAGODEI (Identity)
// ============================================================================

// Session Human models (MVP temporary identity)
export * from '@app/imagodei/models/session-human.model';

// Profile models (human-centered identity, Imago Dei aligned)
export * from '@app/imagodei/models/profile.model';

// ============================================================================
// RE-EXPORTS FROM SHEFA (Economy)
// ============================================================================

// REA Bridge models (ValueFlows/hREA integration)
export * from '@app/shefa/models/rea-bridge.model';

// Contributor Presence models (stewardship lifecycle for absent contributors)
export * from '@app/shefa/models/contributor-presence.model';

// Economic Event models (immutable value flow records)
export * from '@app/shefa/models/economic-event.model';

// ============================================================================
// RE-EXPORTS FROM QAHAL (Community)
// ============================================================================

// Human Affinity models (engagement depth tracking)
export * from '@app/qahal/models/human-affinity.model';

// Human Consent models (graduated intimacy)
export * from '@app/qahal/models/human-consent.model';

// Governance & Feedback models (the protocol's immune system)
export * from '@app/qahal/models/governance-feedback.model';

// Governance Deliberation models (context-aware feedback, deliberation, sensemaking)
export * from '@app/qahal/models/governance-deliberation.model';

// Place models (bioregional awareness)
export * from '@app/qahal/models/place.model';
