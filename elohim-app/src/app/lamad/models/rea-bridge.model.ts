/**
 * REA Bridge Models - ValueFlows Ontology for Economic Coordination
 *
 * Part of the economic layer of the Elohim Protocol.
 * Provides the economic substrate - how value flows.
 *
 * This module bridges the content graph and human graph to the
 * hREA (Holochain REA) economic coordination framework. These types establish
 * contracts that will be fulfilled by Holochain integration while supporting
 * the current prototype.
 *
 * ValueFlows Core Concepts:
 * - Agent: Who participates (humans, orgs, Elohim, creator presences)
 * - Resource: What flows (content, attention, recognition, credentials, tokens)
 * - Event: What happened (view, appreciate, cite, transfer, claim)
 * - Process: Transformation (learning paths as value-creating processes)
 *
 * Economic Model:
 * - Value-aware accounting (every token carries its origin story)
 * - Constitutional governance of value flows
 * - Token types: care, time, learning, steward, creator, infrastructure
 * - Demurrage on hoarding (value decays back to commons)
 * - Attribution persistence (contributions remembered even when offline)
 *
 * Key Insight:
 * "REA doesn't ask 'how much money?' It asks 'what actually happened?'"
 *
 * Holochain mapping:
 * - These types map directly to hREA zome entry types
 * - IDs become ActionHashes in production
 * - Events form the immutable audit trail on agent source chains
 *
 * Protocol Core Integration:
 * - Uses TokenType from protocol-core for token categories
 * - Uses GovernanceLayer from protocol-core for flow governance
 * - Uses Attestation patterns from protocol-core
 * - Uses ProtocolAgent as base for REAAgent
 *
 * References:
 * - ValueFlows: https://www.valueflo.ws
 * - hREA: https://github.com/h-REA/hREA
 * - hREA Docs: https://docs.hrea.io
 */

import {
  type TokenType,
  type TokenDecayRate,
  type GovernanceLayer,
  type Attestation,
  type AttestationStatus,
  type ProtocolAgent,
  type AgentType,
} from './protocol-core.model';

// ============================================================================
// ValueFlows Action Vocabulary
// ============================================================================

/**
 * REAAction - The verb of an economic event.
 *
 * From ValueFlows specification, adapted for Lamad context.
 * Each action describes what happened to resources.
 */
export type REAAction =
  // Input actions (consume/use resources)
  | 'use'           // Use without consuming (view content, attend session)
  | 'consume'       // Use up completely (one-time access tokens)
  | 'cite'          // Reference another's work (creates recognition flow)

  // Output actions (create/produce resources)
  | 'produce'       // Create new resource (author content, synthesize map)
  | 'raise'         // Increase quantity (accumulate recognition)

  // Transfer actions (move between agents)
  | 'transfer'      // Move resource to another agent
  | 'transfer-custody' // Move custody without ownership change

  // Modification actions
  | 'modify'        // Change resource properties
  | 'combine'       // Merge resources (path extensions into base path)
  | 'separate'      // Split resource (fork a learning path)

  // Work actions
  | 'work'          // Contribute labor (stewardship, review, curation)
  | 'deliver-service' // Provide service (Elohim synthesis, tutoring)

  // Acceptance actions
  | 'accept'        // Accept a transfer or commitment (claim presence)
  | 'dropoff'       // Complete a transfer (finalize handoff);

/**
 * Action effects on resources - used for validation.
 */
export const REA_ACTION_EFFECTS: Record<REAAction, {
  resourceEffect: 'increment' | 'decrement' | 'no-effect';
  inputOutput: 'input' | 'output' | 'both' | 'na';
}> = {
  'use': { resourceEffect: 'no-effect', inputOutput: 'input' },
  'consume': { resourceEffect: 'decrement', inputOutput: 'input' },
  'cite': { resourceEffect: 'no-effect', inputOutput: 'input' },
  'produce': { resourceEffect: 'increment', inputOutput: 'output' },
  'raise': { resourceEffect: 'increment', inputOutput: 'output' },
  'transfer': { resourceEffect: 'no-effect', inputOutput: 'both' },
  'transfer-custody': { resourceEffect: 'no-effect', inputOutput: 'both' },
  'modify': { resourceEffect: 'no-effect', inputOutput: 'both' },
  'combine': { resourceEffect: 'decrement', inputOutput: 'input' },
  'separate': { resourceEffect: 'increment', inputOutput: 'output' },
  'work': { resourceEffect: 'no-effect', inputOutput: 'input' },
  'deliver-service': { resourceEffect: 'no-effect', inputOutput: 'output' },
  'accept': { resourceEffect: 'no-effect', inputOutput: 'na' },
  'dropoff': { resourceEffect: 'no-effect', inputOutput: 'na' },
};

// ============================================================================
// Resource Specification
// ============================================================================

/**
 * ResourceSpecification - Defines a type/class of resource.
 *
 * In Lamad, resource specifications define categories of value
 * that can flow through the network.
 */
export interface ResourceSpecification {
  /** Unique identifier */
  id: string;

  /** Human-readable name */
  name: string;

  /** Description of this resource type */
  note?: string;

  /** Default unit of measurement */
  defaultUnitOfResource?: Unit;

  /** Default unit for effort (if this is effort-based) */
  defaultUnitOfEffort?: Unit;

  /** Resource behavior classification */
  resourceClassifiedAs?: ResourceClassification[];

  /** Substitutable with other resources of same spec? */
  substitutable: boolean;

  /** Image for UI */
  image?: string;
}

/**
 * ResourceClassification - Categories for resource types.
 *
 * These map to Lamad's value domains and Shefa token types.
 * See protocol-core.model.ts TokenType for the canonical token categories.
 */
export type ResourceClassification =
  // Lamad content classifications
  | 'content'           // Learning content (epics, features, scenarios)
  | 'attention'         // Human attention/engagement
  | 'recognition'       // Attestation of value/contribution
  | 'credential'        // Earned capabilities/attestations
  | 'curation'          // Curated collections/paths
  | 'synthesis'         // AI-generated maps/analysis
  | 'stewardship'       // Care/maintenance of presences
  | 'membership'        // Network participation rights
  | 'compute'           // Computational resources (for Elohim)
  | 'currency'          // Mutual credit (Unyt/HoloFuel integration)

  // Shefa token classifications (from whitepaper)
  | 'care-token'        // Witnessed caregiving acts
  | 'time-token'        // Hours contributed to community
  | 'learning-token'    // Skills developed and taught
  | 'steward-token'     // Environmental/resource protection
  | 'creator-token'     // Content that helps others
  | 'infrastructure-token'; // Network maintenance contribution

/**
 * Maps ResourceClassification to TokenType from protocol-core.
 * Used for bridging between REA vocabulary and Shefa token system.
 */
export const CLASSIFICATION_TO_TOKEN_TYPE: Partial<Record<ResourceClassification, TokenType>> = {
  'care-token': 'care',
  'time-token': 'time',
  'learning-token': 'learning',
  'steward-token': 'steward',
  'creator-token': 'creator',
  'infrastructure-token': 'infrastructure',
  'recognition': 'recognition',
};

/**
 * Unit - Measurement units for quantities.
 */
export interface Unit {
  id: string;
  label: string;
  symbol: string;
}

/**
 * Standard unit names for type safety.
 */
export type LamadUnitName =
  | 'view' | 'minute' | 'session'       // Attention units
  | 'affinity' | 'endorsement' | 'attestation'  // Recognition units
  | 'node' | 'step' | 'path'            // Content units
  | 'token' | 'cycle'                   // Compute units
  | 'each' | 'one';                     // Generic

/**
 * Standard units for Lamad resources.
 */
export const LAMAD_UNITS: Record<LamadUnitName, Unit> = {
  // Attention units
  view: { id: 'unit-view', label: 'View', symbol: 'view' },
  minute: { id: 'unit-minute', label: 'Minute', symbol: 'min' },
  session: { id: 'unit-session', label: 'Session', symbol: 'sess' },

  // Recognition units
  affinity: { id: 'unit-affinity', label: 'Affinity Point', symbol: 'aff' },
  endorsement: { id: 'unit-endorsement', label: 'Endorsement', symbol: 'end' },
  attestation: { id: 'unit-attestation', label: 'Attestation', symbol: 'att' },

  // Content units
  node: { id: 'unit-node', label: 'Content Node', symbol: 'node' },
  step: { id: 'unit-step', label: 'Path Step', symbol: 'step' },
  path: { id: 'unit-path', label: 'Learning Path', symbol: 'path' },

  // Compute units (for Elohim/Unyt)
  token: { id: 'unit-token', label: 'Token', symbol: 'tok' },
  cycle: { id: 'unit-cycle', label: 'Compute Cycle', symbol: 'cyc' },

  // Generic
  each: { id: 'unit-each', label: 'Each', symbol: 'ea' },
  one: { id: 'unit-one', label: 'One', symbol: '1' },
};

// ============================================================================
// Economic Resource
// ============================================================================

/**
 * EconomicResource - A resource that can flow through the network.
 *
 * In ValueFlows, resources are created and modified only through events.
 * This is the current state derived from event history.
 */
export interface EconomicResource {
  /** Unique identifier (ActionHash in Holochain) */
  id: string;

  /** What kind of resource this is */
  conformsTo: string; // ResourceSpecification.id

  /** Human-readable name */
  name: string;

  /** Description */
  note?: string;

  /** Current quantity */
  accountingQuantity?: Measure;

  /** Available quantity (may differ due to commitments) */
  onhandQuantity?: Measure;

  /** Who currently holds this resource */
  primaryAccountable: string; // Agent ID

  /** Who has custody (may differ from owner) */
  custodian?: string; // Agent ID

  /** Current state */
  state?: ResourceState;

  /** When this resource came into existence */
  createdAt: string;

  /** Tracking identifier (for physical goods, external refs) */
  trackingIdentifier?: string;

  /** Image for UI */
  image?: string;

  /** For content resources: the content node ID */
  contentNodeId?: string;

  /** For credential resources: the attestation ID */
  attestationId?: string;

  /** Classification tags */
  classifiedAs?: ResourceClassification[];

  /** Arbitrary metadata */
  metadata?: Record<string, unknown>;
}

/**
 * Measure - A quantity with unit.
 */
export interface Measure {
  hasNumericalValue: number;
  hasUnit: string; // Unit.id
}

/**
 * ResourceState - Lifecycle state of a resource.
 */
export type ResourceState =
  | 'available'     // Ready to use/transfer
  | 'committed'     // Promised but not yet transferred
  | 'in-use'        // Currently being used
  | 'consumed'      // Fully consumed
  | 'archived'      // No longer active
  | 'disputed';     // Under review/appeal

// ============================================================================
// Agent (REA Extension)
// ============================================================================

/**
 * REAAgent - Extended agent type for economic participation.
 *
 * Extends Lamad's Agent model with REA-specific fields.
 */
export interface REAAgent {
  /** Unique identifier (AgentPubKey in Holochain) */
  id: string;

  /** Display name */
  name: string;

  /** Agent classification */
  type: REAAgentType;

  /** For creator presences: the presence lifecycle state */
  presenceState?: 'unclaimed' | 'stewarded' | 'claimed';

  /** For presences: external identifiers for the creator */
  externalIdentifiers?: ExternalIdentifier[];

  /** Profile image */
  image?: string;

  /** Description/bio */
  note?: string;

  /** Primary location (optional) */
  primaryLocation?: string;

  /** Agent relationships */
  relationships?: AgentRelationship[];

  /** When this agent joined the network */
  createdAt: string;
}

/**
 * REAAgentType - Classification of economic agents.
 */
export type REAAgentType =
  | 'human'                  // Individual person
  | 'organization'           // Group/org with shared identity
  | 'contributor-presence'   // Unclaimed/stewarded external contributor
  | 'elohim'                 // Autonomous constitutional agent
  | 'family'                 // Family unit (economic household)
  | 'community';             // Community collective

/**
 * ExternalIdentifier - Links to external identity systems.
 *
 * Used for contributor presences to enable claiming.
 *
 * W3C DECENTRALIZED IDENTIFIERS (DID) ALIGNMENT:
 * This structure is compatible with DID verification methods.
 * External identifiers can be transformed to DID Documents with service endpoints.
 *
 * Examples of DID mappings:
 * - ORCID → did:web:orcid.org:{orcid-id}
 * - GitHub → did:web:github.com:{username}
 * - Email → did:web:domain.com:users:{email-hash}
 *
 * The verification system here already implements the core DID principle:
 * cryptographically verifiable claims about identity on external systems.
 *
 * Future: Can serialize to DID Documents for interoperability.
 */
export interface ExternalIdentifier {
  /** Type of identifier */
  type: 'orcid' | 'github' | 'twitter' | 'email' | 'website' | 'isbn' | 'doi' | 'other';

  /** The identifier value */
  value: string;

  /** Is this verified? */
  verified: boolean;

  /** Verification method if verified */
  verificationMethod?: string;

  /** When verified */
  verifiedAt?: string;
}

/**
 * AgentRelationship - Formal relationship between agents.
 */
export interface AgentRelationship {
  /** Unique identifier */
  id: string;

  /** The related agent */
  relatedAgentId: string;

  /** Type of relationship */
  relationship: AgentRelationshipType;

  /** When established */
  establishedAt: string;

  /** When ended (if applicable) */
  endedAt?: string;
}

/**
 * AgentRelationshipType - Types of agent relationships.
 */
export type AgentRelationshipType =
  | 'member-of'          // Agent is member of organization
  | 'steward-of'         // Agent stewards a presence
  | 'created-by'         // Resource created by agent
  | 'endorsed-by'        // Agent endorsed another
  | 'student-of'         // Learning relationship
  | 'mentor-of'          // Teaching relationship
  | 'family-of'          // Family relationship
  | 'delegate-of';       // Agent acts on behalf of another

// ============================================================================
// Process (Learning as Value Creation)
// ============================================================================

/**
 * Process - A transformation that creates value.
 *
 * In Lamad, learning paths are processes that transform
 * attention and effort into capabilities and recognition.
 */
export interface Process {
  /** Unique identifier */
  id: string;

  /** What kind of process this is */
  basedOn?: string; // ProcessSpecification.id

  /** Human-readable name */
  name: string;

  /** Description */
  note?: string;

  /** When the process began */
  hasBeginning?: string;

  /** When the process ended */
  hasEnd?: string;

  /** Is this process finished? */
  finished: boolean;

  /** Inputs to this process (events that fed in) */
  inputs?: string[]; // EconomicEvent IDs

  /** Outputs from this process (events that came out) */
  outputs?: string[]; // EconomicEvent IDs

  /** For learning paths: the path ID */
  pathId?: string;

  /** For synthesis: the Elohim that performed it */
  performedBy?: string; // Agent ID

  /** Classification */
  classifiedAs?: ProcessClassification[];
}

/**
 * ProcessClassification - Types of processes in Lamad.
 */
export type ProcessClassification =
  | 'learning-journey'    // Human traversing a learning path
  | 'content-creation'    // Creating new content
  | 'curation'            // Curating content into paths
  | 'synthesis'           // Elohim synthesizing knowledge maps
  | 'review'              // Peer review process
  | 'attestation'         // Granting attestations
  | 'stewardship'         // Maintaining creator presences
  | 'governance';         // Governance decision process

/**
 * ProcessSpecification - Template for a type of process.
 */
export interface ProcessSpecification {
  /** Unique identifier */
  id: string;

  /** Name of this process type */
  name: string;

  /** Description */
  note?: string;

  /** Expected duration */
  estimatedDuration?: string;

  /** Classification */
  classifiedAs?: ProcessClassification[];
}

// ============================================================================
// Intent & Commitment (Future Flows)
// ============================================================================

/**
 * Intent - Expression of desired economic activity.
 *
 * Intents are like requests/offers before they become commitments.
 * In Lamad: "I want to learn X", "I offer to review Y"
 */
export interface Intent {
  /** Unique identifier */
  id: string;

  /** What action is intended */
  action: REAAction;

  /** Who is expressing this intent */
  provider?: string; // Agent ID (who will give)

  /** Who is the intended recipient */
  receiver?: string; // Agent ID (who will receive)

  /** What resource type */
  resourceConformsTo?: string; // ResourceSpecification.id

  /** How much */
  resourceQuantity?: Measure;

  /** For effort-based: how much effort */
  effortQuantity?: Measure;

  /** When intended */
  hasPointInTime?: string;

  /** Available from */
  availableFrom?: string;

  /** Available until */
  availableUntil?: string;

  /** Description */
  note?: string;

  /** Is this satisfied by commitments? */
  satisfied: boolean;

  /** Classification */
  classifiedAs?: string[];
}

/**
 * Commitment - A promise of future economic activity.
 *
 * Commitments are binding promises, stronger than intents.
 * In Lamad: "I commit to stewarding this presence"
 */
export interface Commitment {
  /** Unique identifier */
  id: string;

  /** What action is committed */
  action: REAAction;

  /** Who is committing to provide */
  provider: string; // Agent ID

  /** Who will receive */
  receiver: string; // Agent ID

  /** What resource type */
  resourceConformsTo?: string; // ResourceSpecification.id

  /** Specific resource (if known) */
  resourceInventoriedAs?: string; // EconomicResource.id

  /** How much committed */
  resourceQuantity?: Measure;

  /** For effort-based: how much effort */
  effortQuantity?: Measure;

  /** When due */
  due?: string;

  /** Part of what agreement */
  clauseOf?: string; // Agreement.id

  /** Part of what process */
  inputOf?: string; // Process.id
  outputOf?: string; // Process.id

  /** Description */
  note?: string;

  /** Is this fully satisfied by events? */
  finished: boolean;

  /** Current state */
  state: CommitmentState;

  /** When created */
  createdAt: string;
}

/**
 * CommitmentState - Lifecycle of a commitment.
 */
export type CommitmentState =
  | 'proposed'    // Offered but not accepted
  | 'accepted'    // Accepted by receiver
  | 'in-progress' // Being fulfilled
  | 'fulfilled'   // Fully satisfied
  | 'cancelled'   // Cancelled before fulfillment
  | 'breached';   // Failed to fulfill

// ============================================================================
// Agreement (Governance)
// ============================================================================

/**
 * Agreement - A contract governing economic activity.
 *
 * In Lamad: constitutions, community agreements, stewardship contracts.
 */
export interface Agreement {
  /** Unique identifier */
  id: string;

  /** Human-readable name */
  name: string;

  /** Description/terms */
  note?: string;

  /** When the agreement begins */
  hasBeginning?: string;

  /** When the agreement ends */
  hasEnd?: string;

  /** Commitments made under this agreement */
  commitments?: string[]; // Commitment IDs

  /** Parties to this agreement */
  parties: string[]; // Agent IDs

  /** Classification */
  classifiedAs?: AgreementClassification[];
}

/**
 * AgreementClassification - Types of agreements.
 */
export type AgreementClassification =
  | 'constitution'        // Foundational governance document
  | 'community-covenant'  // Community membership terms
  | 'stewardship-contract' // Creator presence stewardship
  | 'learning-commitment' // Commitment to learning path
  | 'review-assignment'   // Peer review agreement
  | 'attestation-grant';  // Attestation issuance terms

// ============================================================================
// Appreciation (Recognition Flows)
// ============================================================================

/**
 * Appreciation - Recognition of value created by another.
 *
 * This is the core of Lamad's recognition economics.
 * Appreciations flow to creators (or their presences) when their
 * work is used, cited, or endorsed.
 */
export interface Appreciation {
  /** Unique identifier */
  id: string;

  /** The event that triggered this appreciation */
  appreciationOf: string; // EconomicEvent.id

  /** The agent being appreciated */
  appreciatedBy: string; // Agent ID (who is giving appreciation)

  /** The agent receiving appreciation */
  appreciationTo: string; // Agent ID (recipient, may be a presence)

  /** Quantity of appreciation */
  quantity: Measure;

  /** Note/reason */
  note?: string;

  /** When given */
  createdAt: string;
}

// ============================================================================
// Claim (Entitlements)
// ============================================================================

/**
 * Claim - An entitlement arising from economic activity.
 *
 * In Lamad: claims to recognition that has accumulated at a presence.
 */
export interface Claim {
  /** Unique identifier */
  id: string;

  /** What event triggered this claim */
  triggeredBy: string; // EconomicEvent.id

  /** Who has the claim */
  claimant: string; // Agent ID

  /** Against whom/what */
  claimAgainst: string; // Agent ID or Resource ID

  /** What is claimed */
  resourceConformsTo?: string; // ResourceSpecification.id

  /** How much is claimed */
  resourceQuantity?: Measure;

  /** When the claim was created */
  createdAt: string;

  /** When the claim was settled */
  settledAt?: string;

  /** Is this claim settled? */
  settled: boolean;

  /** How it was settled */
  settledBy?: string; // EconomicEvent.id

  /** Note */
  note?: string;
}

// ============================================================================
// Standard Resource Specifications for Lamad
// ============================================================================

/**
 * Pre-defined resource specifications for the Lamad economy.
 */
export const LAMAD_RESOURCE_SPECS: Record<string, ResourceSpecification> = {
  // Content resources
  'content-node': {
    id: 'spec-content-node',
    name: 'Content Node',
    note: 'A unit of learning content (epic, feature, scenario, etc.)',
    defaultUnitOfResource: LAMAD_UNITS.node,
    resourceClassifiedAs: ['content'],
    substitutable: false,
  },
  'learning-path': {
    id: 'spec-learning-path',
    name: 'Learning Path',
    note: 'A curated sequence of content nodes forming a journey',
    defaultUnitOfResource: LAMAD_UNITS.path,
    resourceClassifiedAs: ['curation'],
    substitutable: false,
  },

  // Attention resources
  'attention': {
    id: 'spec-attention',
    name: 'Attention',
    note: 'Human engagement with content',
    defaultUnitOfResource: LAMAD_UNITS.view,
    defaultUnitOfEffort: LAMAD_UNITS.minute,
    resourceClassifiedAs: ['attention'],
    substitutable: true,
  },

  // Recognition resources
  'recognition': {
    id: 'spec-recognition',
    name: 'Recognition',
    note: 'Acknowledgment of value created',
    defaultUnitOfResource: LAMAD_UNITS.affinity,
    resourceClassifiedAs: ['recognition'],
    substitutable: true,
  },
  'endorsement': {
    id: 'spec-endorsement',
    name: 'Endorsement',
    note: 'Formal endorsement from a qualified agent',
    defaultUnitOfResource: LAMAD_UNITS.endorsement,
    resourceClassifiedAs: ['recognition'],
    substitutable: false,
  },

  // Credential resources
  'attestation': {
    id: 'spec-attestation',
    name: 'Attestation',
    note: 'A credential earned through demonstrated capability',
    defaultUnitOfResource: LAMAD_UNITS.attestation,
    resourceClassifiedAs: ['credential'],
    substitutable: false,
  },

  // Synthesis resources
  'knowledge-map': {
    id: 'spec-knowledge-map',
    name: 'Knowledge Map',
    note: 'Synthesized knowledge structure',
    defaultUnitOfResource: LAMAD_UNITS.each,
    resourceClassifiedAs: ['synthesis'],
    substitutable: false,
  },

  // Stewardship resources
  'stewardship-commitment': {
    id: 'spec-stewardship',
    name: 'Stewardship Commitment',
    note: 'Commitment to steward a creator presence',
    defaultUnitOfResource: LAMAD_UNITS.each,
    resourceClassifiedAs: ['stewardship'],
    substitutable: false,
  },

  // Compute resources (for Unyt integration)
  'compute-credit': {
    id: 'spec-compute',
    name: 'Compute Credit',
    note: 'Computational resource allocation',
    defaultUnitOfResource: LAMAD_UNITS.cycle,
    resourceClassifiedAs: ['compute'],
    substitutable: true,
  },
};

// ============================================================================
// Token Resources
// ============================================================================

/**
 * Token resource specifications for multi-dimensional value tracking.
 *
 * - Care tokens: Witnessed caregiving acts
 * - Time tokens: Hours contributed to community
 * - Learning tokens: Skills developed and taught
 * - Steward tokens: Environmental/resource protection
 * - Creator tokens: Content that helps others
 * - Infrastructure tokens: Network maintenance contribution
 */
export const TOKEN_RESOURCE_SPECS: Record<string, ResourceSpecification> = {
  'care-token': {
    id: 'spec-care-token',
    name: 'Care Token',
    note: 'Generated by witnessed caregiving acts. Circulates for services, goods, recognition.',
    defaultUnitOfResource: LAMAD_UNITS.each,
    resourceClassifiedAs: ['care-token'],
    substitutable: true,
  },
  'time-token': {
    id: 'spec-time-token',
    name: 'Time Token',
    note: 'Generated by hours contributed to community. Circulates for coordination, services.',
    defaultUnitOfResource: LAMAD_UNITS.minute,
    resourceClassifiedAs: ['time-token'],
    substitutable: true,
  },
  'learning-token': {
    id: 'spec-learning-token',
    name: 'Learning Token',
    note: 'Generated by skills developed and taught. Circulates for education, mentorship. Does not decay.',
    defaultUnitOfResource: LAMAD_UNITS.each,
    resourceClassifiedAs: ['learning-token'],
    substitutable: false,
  },
  'steward-token': {
    id: 'spec-steward-token',
    name: 'Steward Token',
    note: 'Generated by environmental/resource protection. Circulates for sustainable goods, restoration.',
    defaultUnitOfResource: LAMAD_UNITS.each,
    resourceClassifiedAs: ['steward-token'],
    substitutable: true,
  },
  'creator-token': {
    id: 'spec-creator-token',
    name: 'Creator Token',
    note: 'Generated by content that helps others. Circulates for derivative rights, recognition. Does not decay.',
    defaultUnitOfResource: LAMAD_UNITS.each,
    resourceClassifiedAs: ['creator-token'],
    substitutable: false,
  },
  'infrastructure-token': {
    id: 'spec-infrastructure-token',
    name: 'Infrastructure Token',
    note: 'Generated by network maintenance contribution. Circulates for protocol services. High decay rate.',
    defaultUnitOfResource: LAMAD_UNITS.each,
    resourceClassifiedAs: ['infrastructure-token'],
    substitutable: true,
  },
};

// ============================================================================
// Constitutional Flow Control
// ============================================================================

/**
 * ConstitutionalLayer - The four-layer governance for value flows.
 *
 * These layers define constitutional constraints on value flow:
 * 1. Dignity Floor - Basic needs, care labor recognition, cannot be extracted
 * 2. Attribution - Value flows to creators, accumulates in trust for absent contributors
 * 3. Circulation - Tokens must circulate, demurrage on hoarding
 * 4. Sustainability - Portion flows to next community liberation
 */
export type ConstitutionalLayer =
  | 'dignity_floor'    // Layer 1: Existential minimums
  | 'attribution'      // Layer 2: Contribution recognition
  | 'circulation'      // Layer 3: Community velocity
  | 'sustainability';  // Layer 4: Network development

/**
 * ConstitutionalConstraint - Inviolable constraints on value flow.
 *
 * Core constraints:
 * - No accumulation without responsibility
 * - Graduated claiming
 * - Demurrage on hoarding
 * - Attribution persistence
 * - Anti-capture
 */
export interface ConstitutionalConstraint {
  /** Unique identifier */
  id: string;

  /** Which constitutional layer this constraint belongs to */
  layer: ConstitutionalLayer;

  /** Human-readable name */
  name: string;

  /** Description of the constraint */
  description: string;

  /** Is this constraint currently active? */
  isActive: boolean;

  /**
   * Constraint parameters.
   * Examples:
   * - dignityThreshold: minimum tokens before decay
   * - maxAccumulation: maximum tokens before redistribution
   * - demurrageRate: decay rate per period
   * - claimingThreshold: identity attestation required for claim size
   */
  parameters: Record<string, number | string | boolean>;

  /** Governance layer that can modify this constraint */
  governedBy: GovernanceLayer;
}

/**
 * CommonsPool - Collective value pool stewarded by Elohim.
 *
 * Network-generated value accumulates in trust for attributed contributors,
 * claimable when they present attested identity.
 */
export interface CommonsPool {
  /** Unique identifier */
  id: string;

  /** Human-readable name */
  name: string;

  /** Description */
  note?: string;

  /** Token type held in this pool */
  tokenType: TokenType;

  /** Current balance */
  balance: Measure;

  /** Total attributed but unclaimed */
  attributedUnclaimed: Measure;

  /** Constitutional layer this pool serves */
  constitutionalLayer: ConstitutionalLayer;

  /** Governance layer that manages this pool */
  governedBy: GovernanceLayer;

  /** When this pool was created */
  createdAt: string;

  /** Last activity timestamp */
  lastActivityAt: string;
}

/**
 * ValueAttribution - Attribution of value to a contributor.
 *
 * Tracks value attributed to an agent (even when offline) that can be
 * claimed when they present attested identity.
 */
export interface ValueAttribution {
  /** Unique identifier */
  id: string;

  /** Agent ID this attribution belongs to */
  agentId: string;

  /** Token type attributed */
  tokenType: TokenType;

  /** Amount attributed */
  amount: Measure;

  /** Source event(s) that generated this attribution */
  sourceEventIds: string[];

  /** Commons pool this attribution draws from */
  commonsPoolId: string;

  /** Has this been claimed? */
  claimed: boolean;

  /** When claimed (if applicable) */
  claimedAt?: string;

  /** Claiming event ID (if applicable) */
  claimEventId?: string;

  /** When this attribution was created */
  createdAt: string;

  /** When this attribution expires (for time-bound attributions) */
  expiresAt?: string;
}

/**
 * AttributionClaim - A claim against attributed value.
 *
 * Extends the base Claim type with identity and responsibility requirements:
 * - Identity attestation level required
 * - Responsibility threshold verification
 * - Constitutional constraint compliance
 */
export interface AttributionClaim {
  /** Unique identifier */
  id: string;

  /** Attribution being claimed */
  attributionId: string;

  /** Agent making the claim */
  claimantId: string;

  /** Amount being claimed */
  amount: Measure;

  /** Required identity attestation level (graduated claiming) */
  requiredAttestationLevel: 'basic' | 'relationship' | 'biometric' | 'full';

  /** Has identity attestation been verified? */
  identityVerified: boolean;

  /** Identity attestation ID used for verification */
  identityAttestationId?: string;

  /** Has responsibility threshold been met? */
  responsibilityVerified: boolean;

  /** Current claim state */
  state: ClaimState;

  /** When claim was submitted */
  submittedAt: string;

  /** When claim was processed (approved/rejected) */
  processedAt?: string;

  /** Processing notes */
  processingNote?: string;
}

/**
 * ClaimState - Lifecycle of an attribution claim.
 */
export type ClaimState =
  | 'pending_identity'      // Awaiting identity attestation
  | 'pending_responsibility' // Awaiting responsibility verification
  | 'pending_review'        // Under Elohim review
  | 'approved'              // Claim approved
  | 'rejected'              // Claim rejected
  | 'withdrawn';            // Claimant withdrew

/**
 * Re-export Shefa-related types from protocol-core for convenience.
 */
export type { TokenType, TokenDecayRate } from './protocol-core.model';
