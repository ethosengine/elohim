/**
 * Barrel export for Lamad models
 *
 * Four-layer architecture:
 * - Territory: ContentNode (the knowledge graph)
 * - Journey: LearningPath, PathStep (curated sequences)
 * - Traveler: Agent, AgentProgress (human state)
 * - Maps: KnowledgeMap (polymorphic: domain, person, collective)
 *
 * Trust Model (Bidirectional Attestations):
 * - Agent attestations: Earned credentials for accessing gated content
 * - Content attestations: Earned trust for reaching broader audiences
 * - Symmetric accountability: Both agents AND content need trust to participate
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

// Content Attestation models (Bidirectional Trust)
export * from './content-attestation.model';

// Journey models (Learning Path)
export * from './learning-path.model';

// Traveler models (Agent & Progress)
export * from './agent.model';

// Agent Attestation models (Earned credentials)
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

// Session User models (MVP temporary identity)
export * from './session-user.model';

// Content Access models (access control for gated content)
export * from './content-access.model';

// Profile models (human-centered identity, Imago Dei aligned)
export * from './profile.model';

// REA Bridge models (ValueFlows/hREA integration)
export * from './rea-bridge.model';

// Contributor Presence models (stewardship lifecycle for absent contributors)
export * from './contributor-presence.model';

// Economic Event models (immutable value flow records)
export * from './economic-event.model';
