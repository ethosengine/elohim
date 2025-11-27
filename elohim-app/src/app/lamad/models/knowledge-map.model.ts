/**
 * Knowledge Map Models - Polymorphic containers for learnable territory.
 *
 * Inspired by:
 * - Khan Academy's "World of Math" (domain knowledge graphs)
 * - Gottman's Love Maps (relational knowledge about people)
 * - Organizational knowledge management (collective intelligence)
 *
 * The key insight: learning is fundamentally about building relationship -
 * with ideas, with practices, with people, with communities.
 * The same navigation/affinity mechanics apply to all three.
 *
 * Holochain mapping:
 * - Entry type: "knowledge_map"
 * - Links to subject (content graph, agent, or organization)
 * - Private maps on source chain, shared maps on DHT
 */

/**
 * KnowledgeMapType - The three flavors of knowledge territory.
 */
export type KnowledgeMapType = 'domain' | 'person' | 'collective';

/**
 * MapSubject - What is being mapped (the territory).
 */
export interface MapSubject {
  /** Type of subject being mapped */
  type: 'content-graph' | 'agent' | 'organization';

  /** Identifier of the subject */
  subjectId: string;

  /** Human-readable name of the subject */
  subjectName: string;
}

/**
 * KnowledgeMap - Base interface for all map types.
 *
 * A knowledge map is a personalized view of a learnable territory.
 * Unlike paths (which are curator-defined journeys), maps represent
 * the learner's own understanding and relationship with a subject.
 */
export interface KnowledgeMap {
  /** Unique identifier */
  id: string;

  /** Type discriminator for polymorphism */
  mapType: KnowledgeMapType;

  /** The subject being mapped */
  subject: MapSubject;

  /** Who created/owns this map */
  ownerId: string;

  /** Display title for this map */
  title: string;

  /** Description of what this map represents */
  description?: string;

  /**
   * Visibility controls who can see this map:
   * - 'private': Only the owner
   * - 'mutual': Owner and subject (for person maps)
   * - 'shared': Specific agents granted access
   * - 'public': Anyone can view
   */
  visibility: 'private' | 'mutual' | 'shared' | 'public';

  /** Agents granted access (when visibility is 'shared') */
  sharedWith?: string[];

  /** Knowledge nodes in this map */
  nodes: KnowledgeNode[];

  /** Paths through this map's territory */
  pathIds: string[];

  /** Overall affinity/familiarity score (0.0 - 1.0) */
  overallAffinity: number;

  /** Timestamps */
  createdAt: string;
  updatedAt: string;

  /** Optional metadata */
  metadata?: Record<string, unknown>;
}

/**
 * KnowledgeNode - A single piece of knowledge in a map.
 *
 * Unlike ContentNode (which is shared territory), KnowledgeNode
 * represents personal/relational knowledge that may be private.
 */
export interface KnowledgeNode {
  /** Unique identifier within the map */
  id: string;

  /** Category this knowledge belongs to */
  category: string;

  /** The knowledge content */
  title: string;
  content: string;

  /** Source of this knowledge */
  source?: KnowledgeSource;

  /** Affinity/confidence in this knowledge (0.0 - 1.0) */
  affinity: number;

  /** When was this last verified/updated? */
  lastVerified?: string;

  /** Related nodes within the same map */
  relatedNodeIds: string[];

  /** Tags for organization */
  tags: string[];

  /** Custom metadata */
  metadata?: Record<string, unknown>;
}

/**
 * KnowledgeSource - Where knowledge came from.
 */
export interface KnowledgeSource {
  type: 'direct-observation' | 'conversation' | 'shared-content' | 'inference' | 'external';
  sourceId?: string;
  timestamp: string;
  confidence: number;
}

// ============================================================================
// Domain Knowledge Map (Khan Academy / Elohim Protocol style)
// ============================================================================

/**
 * DomainKnowledgeMap - Knowledge map over a content graph.
 *
 * This is what we've been building: a learner's relationship with
 * a structured body of knowledge like "The Elohim Protocol" or
 * "World of Math".
 */
export interface DomainKnowledgeMap extends KnowledgeMap {
  mapType: 'domain';

  subject: {
    type: 'content-graph';
    subjectId: string;  // ID of the root content node or graph
    subjectName: string;
  };

  /** The content graph being mapped */
  contentGraphId: string;

  /** Mastery levels per content node */
  masteryLevels: Map<string, MasteryLevel>;

  /** Learning goals within this domain */
  goals?: DomainGoal[];
}

// MasteryLevel is imported from agent.model.ts to avoid duplication
// Re-export for convenience within this file
import type { MasteryLevel } from './agent.model';
export type { MasteryLevel };

export interface DomainGoal {
  id: string;
  title: string;
  targetNodes: string[];
  targetMastery: MasteryLevel;
  deadline?: string;
  completed: boolean;
}

// ============================================================================
// Person Knowledge Map (Gottman Love Maps)
// ============================================================================

/**
 * PersonKnowledgeMap - Knowledge map about another person.
 *
 * Inspired by Gottman's Love Maps research: the mental space where
 * you store detailed knowledge about someone you care about.
 *
 * Key differences from domain maps:
 * - Subject is a person, not a content graph
 * - Knowledge is relational (requires consent for deep access)
 * - Categories are relationship-oriented
 * - Privacy is paramount
 */
export interface PersonKnowledgeMap extends KnowledgeMap {
  mapType: 'person';

  subject: {
    type: 'agent';
    subjectId: string;  // The person being mapped
    subjectName: string;
  };

  /** Relationship type between mapper and subject */
  relationshipType: RelationshipType;

  /** Consent from the subject to be mapped */
  subjectConsent?: SubjectConsent;

  /** Categories of knowledge (Gottman-inspired) */
  categories: PersonKnowledgeCategory[];

  /** Reciprocal map (if subject also maps the owner) */
  reciprocalMapId?: string;

  /** Relationship health metrics */
  relationshipMetrics?: RelationshipMetrics;
}

export type RelationshipType =
  | 'spouse'
  | 'partner'
  | 'parent'
  | 'child'
  | 'sibling'
  | 'friend'
  | 'mentor'
  | 'mentee'
  | 'colleague'
  | 'acquaintance'
  | 'other';

/**
 * SubjectConsent - Permission from the person being mapped.
 *
 * Critical for ethical knowledge mapping. Without consent,
 * maps are limited to publicly available information.
 */
export interface SubjectConsent {
  /** Has the subject granted permission? */
  granted: boolean;

  /** What scope of access is permitted? */
  scope: ConsentScope;

  /** When was consent granted? */
  grantedAt?: string;

  /** When does consent expire? (optional) */
  expiresAt?: string;

  /** Can the subject see what's in the map? */
  transparencyLevel: 'none' | 'categories-only' | 'full-read' | 'collaborative';
}

export type ConsentScope =
  | 'public-info'    // Only publicly shared information
  | 'shared-only'    // Only what subject explicitly shares with mapper
  | 'full-access';   // Deep knowledge mapping permitted

/**
 * PersonKnowledgeCategory - Gottman-inspired knowledge categories.
 */
export interface PersonKnowledgeCategory {
  id: string;
  type: PersonKnowledgeCategoryType;
  title: string;
  description?: string;
  nodes: KnowledgeNode[];
  affinity: number;
}

export type PersonKnowledgeCategoryType =
  | 'life-history'         // Past experiences, childhood, formative events
  | 'current-stressors'    // Present challenges, worries, pressures
  | 'dreams-aspirations'   // Future hopes, goals, ambitions
  | 'values-beliefs'       // Core principles, worldview, ethics
  | 'preferences-dislikes' // Daily preferences, pet peeves, favorites
  | 'friends-family'       // Social network, important relationships
  | 'work-career'          // Professional life, skills, ambitions
  | 'health-wellbeing'     // Physical/mental health, self-care
  | 'communication-style'  // How they express, receive love/feedback
  | 'conflict-patterns'    // How they handle disagreement
  | 'love-language'        // Primary ways of giving/receiving love
  | 'custom';              // User-defined categories

/**
 * RelationshipMetrics - Health indicators for the relationship.
 */
export interface RelationshipMetrics {
  /** Overall relationship health (0.0 - 1.0) */
  overallHealth: number;

  /** How complete is the knowledge map? */
  mapCompleteness: number;

  /** How recent is the knowledge? */
  knowledgeFreshness: number;

  /** Are both parties actively mapping each other? */
  reciprocity: number;

  /** Last meaningful interaction/update */
  lastInteraction: string;
}

// ============================================================================
// Collective Knowledge Map (Organizations, Teams)
// ============================================================================

/**
 * CollectiveKnowledgeMap - Shared knowledge within a group.
 *
 * Represents what "we" know as a team, organization, or community.
 * Combines individual contributions into collective intelligence.
 */
export interface CollectiveKnowledgeMap extends KnowledgeMap {
  mapType: 'collective';

  subject: {
    type: 'organization';
    subjectId: string;  // The collective being mapped
    subjectName: string;
  };

  /** Members who contribute to this map */
  members: CollectiveMember[];

  /** Governance model for the map */
  governance: CollectiveGovernance;

  /** Domains of collective knowledge */
  domains: CollectiveDomain[];

  /** Attestations granted by collective consensus */
  collectiveAttestations: string[];
}

export interface CollectiveMember {
  agentId: string;
  role: 'steward' | 'contributor' | 'viewer';
  joinedAt: string;
  contributionCount: number;
}

export interface CollectiveGovernance {
  /** How are changes approved? */
  approvalModel: 'steward-only' | 'majority-vote' | 'consensus' | 'open';

  /** Minimum contributors for changes */
  quorum?: number;

  /** Who can add new members? */
  membershipControl: 'steward-only' | 'member-invite' | 'open';
}

export interface CollectiveDomain {
  id: string;
  title: string;
  description: string;
  stewards: string[];  // Agent IDs responsible for this domain
  nodes: KnowledgeNode[];
  affinity: number;  // Collective mastery level
}

// ============================================================================
// Knowledge Map Index & Discovery
// ============================================================================

/**
 * KnowledgeMapIndex - Lightweight entry for map discovery.
 */
export interface KnowledgeMapIndexEntry {
  id: string;
  mapType: KnowledgeMapType;
  title: string;
  subjectName: string;
  ownerId: string;
  ownerName: string;
  visibility: string;
  overallAffinity: number;
  nodeCount: number;
  updatedAt: string;
}

/**
 * KnowledgeMapIndex - Response from map catalog endpoint.
 */
export interface KnowledgeMapIndex {
  lastUpdated: string;
  totalCount: number;
  maps: KnowledgeMapIndexEntry[];
}

// ============================================================================
// Map Operations
// ============================================================================

/**
 * KnowledgeMapUpdate - Mutation operation on a map.
 */
export interface KnowledgeMapUpdate {
  mapId: string;
  operation: 'add-node' | 'update-node' | 'remove-node' | 'update-affinity';
  nodeId?: string;
  data: Partial<KnowledgeNode>;
  source?: KnowledgeSource;
  timestamp: string;
}

/**
 * MapMergeRequest - Request to merge knowledge from another map.
 */
export interface MapMergeRequest {
  sourceMapId: string;
  targetMapId: string;
  nodeIds: string[];  // Specific nodes to merge
  conflictResolution: 'source-wins' | 'target-wins' | 'manual';
}
