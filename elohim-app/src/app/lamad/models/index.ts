/**
 * Barrel export for Lamad models
 *
 * Six-layer architecture:
 * - Territory: ContentNode (the knowledge graph)
 * - Journey: LearningPath, PathStep (curated sequences)
 * - Traveler: Agent, AgentProgress (human state)
 * - Maps: KnowledgeMap (polymorphic: domain, self, person, collective)
 * - Economic: REA value flows and contributor recognition
 * - Governance: Constitutional moderation, deliberation, feedback profiles
 *
 * Mastery System (Bloom's Taxonomy):
 * - ContentMastery: Per-node mastery tracking (not_started → create)
 * - Attestation Gate: "apply" level unlocks participation privileges
 * - ContentLifecycle: Refresh policies, deprecation, "right to be forgotten"
 * - ExpertiseDiscovery: Graph-emergent expertise without gamified scores
 *
 * Trust Model (Bidirectional Attestations):
 * - Agent attestations: Earned credentials for accessing gated content
 * - Content attestations: Earned trust for reaching broader audiences
 * - Symmetric accountability: Both agents AND content need trust to participate
 *
 * IMPORTANT - Three distinct Attestation models:
 * 1. Agent Attestations (attestations.model.ts):
 *    - Credentials earned by humans/agents
 *    - Granted after completing learning paths
 *    - Example: "4th grade math mastery", "Trauma Support Capacity"
 *
 * 2. Content Attestations (content-attestation.model.ts):
 *    - Trust credentials granted TO content
 *    - Determines reach (private → commons)
 *    - Example: "governance-ratified", "steward-approved"
 *
 * 3. Content Access (content-access.model.ts):
 *    - Access tier requirements (visitor/member/attested)
 *    - Determines WHO can view content
 *    - Example: "requires member status", "requires CSAM training attestation"
 *
 * Date Field Convention:
 * - All timestamp fields use ISO 8601 string format (not Date objects)
 * - Ensures JSON serialization, Holochain compatibility, timezone safety
 *
 * Elohim Agents:
 * - Autonomous constitutional guardians (not controllable utilities)
 * - Layer-specific (global, community, family, individual)
 * - Capability-based invocation with constitutional reasoning
 *
 * REA Economic Coordination (hREA/Unyt integration):
 * - REA Bridge: ValueFlows ontology types (Agent, Resource, Event, Process)
 * - Contributor Presence: Stewardship lifecycle for absent contributors
 * - Economic Event: Immutable audit trail of value flows
 * - Recognition Economics: Value accumulates at presences until claimed
 *
 * Governance & Feedback:
 * - Governance Feedback: Constitutional moderation, challenges, appeals, precedent
 * - Governance Deliberation: Loomio/Polis/Wikipedia-inspired deliberation
 * - Feedback Profile: Virality as privilege, emotional reaction constraints
 *
 * Extensions:
 * - PathExtension: Learner-owned mutations to paths
 * - CollaborativePath: Multi-author path creation
 */

// Territory models (Content)
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
// Determines ContentReach: private → invited → local → community → federated → commons
export * from './content-attestation.model';

// Journey models (Learning Path)
export * from './learning-path.model';

// Traveler models (Agent & Progress)
export * from './agent.model';

// Agent Attestation models (Credentials earned BY humans/agents)
// AttestationAccessRequirement: What attestations unlock which content
// NOTE: Distinct from ContentAccessRequirement in content-access.model.ts
export * from './attestations.model';

// Elohim Agent models (Autonomous constitutional guardians)
export * from './elohim-agent.model';

// Knowledge Map models (Polymorphic: Domain, Person, Collective)
export * from './knowledge-map.model';

// Path Extension models (Learner customization)
export * from './path-extension.model';

// Exploration models (Graph traversal and discovery)
export * from './exploration.model';

// Trust Badge models (UI-ready trust display)
export * from './trust-badge.model';

// Search models (enhanced search with scoring and facets)
export * from './search.model';

// Session Human models (MVP temporary identity)
export * from './session-human.model';

// Content Access models (visitor/member/attested tiers)
// ContentAccessRequirement: What access level is required for content
// NOTE: Distinct from AttestationAccessRequirement in attestations.model.ts
export * from './content-access.model';

// Profile models (human-centered identity, Imago Dei aligned)
export * from './profile.model';

// REA Bridge models (ValueFlows/hREA integration)
export * from './rea-bridge.model';

// Contributor Presence models (stewardship lifecycle for absent contributors)
export * from './contributor-presence.model';

// Economic Event models (immutable value flow records)
export * from './economic-event.model';

// Governance & Feedback models (the protocol's immune system)
export * from './governance-feedback.model';

// Governance Deliberation models (context-aware feedback, deliberation, sensemaking)
export * from './governance-deliberation.model';

// Feedback Profile models (virality as privilege, emotional reaction constraints)
export * from './feedback-profile.model';

// Content Mastery models (Bloom's Taxonomy progression)
// Tracks mastery level from passive (seen → apply) to active (analyze → create)
// The "apply" level is the attestation gate for participation privileges
export * from './content-mastery.model';

// Content Lifecycle models (refresh policies, deprecation, archival)
// Implements "the right to be forgotten" - content has natural lifecycles
export * from './content-lifecycle.model';

// Human Affinity models (engagement depth tracking)
export * from './human-affinity.model';

// Human Consent models (graduated intimacy and consent-based relationships)
export * from './human-consent.model';

// Path Negotiation models (agent-to-agent path customization)
export * from './path-negotiation.model';

// Source Chain models (Holochain-compatible agent-centric data)
export * from './source-chain.model';

// Place models (bioregional and jurisdictional grounding)
export * from './place.model';

// Human Node models (humans as graph nodes in the social layer)
export * from './human-node.model';

// BACKLOGGED: Expertise Discovery models (finding experts in domains)
// See BACKLOG.md - expertise-discovery.model.ts exists but not yet integrated
// export * from './expertise-discovery.model';
