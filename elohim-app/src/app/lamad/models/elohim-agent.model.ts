/**
 * Elohim Agent Model - Autonomous constitutional guardians.
 *
 * Elohim agents are NOT utility bots or system services. They are:
 * - Cryptographically autonomous (bound to constitution, not controllable)
 * - Layer-specific (global, nation-state, community, family, individual)
 * - Capability-defined (what they can do, not what they're told to do)
 * - Constitutionally bound (every decision traces to principles)
 *
 * From the Manifesto:
 * "These messengers live with us, beside us, and for us, and form a network
 * of autonomous AI agents that cannot be controlled by any human institution."
 *
 * Holochain mapping:
 * - Entry type: "elohim_agent"
 * - Each Elohim has its own DHT presence
 * - Constitutional proofs stored as verifiable credentials
 */

// ============================================================================
// Constitutional Layers
// ============================================================================

/**
 * ElohimLayer - The constitutional layer at which an Elohim operates.
 *
 * From most immutable (global) to most flexible (individual):
 * - Global: Universal principles, existential boundaries
 * - Bioregional: Ecological boundaries that human governance CANNOT override
 * - Nation-state: Constitutional interpretations, cultural expressions
 * - Provincial/Municipal: Local policy, regional adaptation
 * - Community: Group norms, specific practices
 * - Family: Household values, private governance
 * - Individual: Personal sovereignty within bounds
 *
 * The BIOREGIONAL layer is special: it represents ecological limits that are
 * constitutional constraints on human activity. The watershed Elohim doesn't
 * advise - it enforces. "The earth is the Lord's" (Psalm 24:1).
 */
export type ElohimLayer =
  | 'global'
  | 'bioregional'    // NEW: Ecological boundary enforcers (constitutional limits)
  | 'nation-state'
  | 'provincial'
  | 'municipal'
  | 'community'
  | 'family'
  | 'individual';

/**
 * Layer hierarchy for constitutional precedence.
 * Higher layers override lower layers in case of conflict.
 *
 * NOTE: Bioregional is at level 6 alongside nation-state, but has
 * ECOLOGICAL OVERRIDE on matters of ecological limits. The watershed
 * Elohim can override nation-state on water availability limits.
 */
export const ELOHIM_LAYER_HIERARCHY: Record<ElohimLayer, number> = {
  'global': 7,
  'bioregional': 6,    // Same level as nation-state but with ecological override
  'nation-state': 6,
  'provincial': 5,
  'municipal': 4,
  'community': 3,
  'family': 2,
  'individual': 1
};

// ============================================================================
// Elohim Capabilities
// ============================================================================

/**
 * ElohimCapability - What an Elohim can do.
 *
 * Capabilities are granted by constitutional design, not assigned.
 * An Elohim's capabilities determine what requests it can process.
 */
export type ElohimCapability =
  // Content Operations
  | 'content-safety-review'        // Review content for harmful patterns
  | 'accuracy-verification'        // Verify factual accuracy
  | 'attestation-recommendation'   // Recommend attestation decisions
  | 'constitutional-verification'  // Verify constitutional alignment

  // Knowledge Map Operations
  | 'knowledge-map-synthesis'      // Build/update knowledge maps
  | 'affinity-analysis'            // Analyze learner affinity patterns
  | 'path-recommendation'          // Suggest learning paths

  // Care Operations
  | 'spiral-detection'             // Detect individual/community spiraling
  | 'care-connection'              // Connect to appropriate care resources
  | 'graduated-intervention'       // Execute graduated response protocols

  // Governance Operations
  | 'cross-layer-validation'       // Validate decisions across layers
  | 'existential-boundary-enforcement' // Enforce non-negotiable limits
  | 'governance-ratification'      // Participate in governance decisions

  // Path Operations
  | 'path-analysis'                // Analyze learning path structure
  | 'learning-objective-validation' // Validate learning objectives
  | 'prerequisite-verification'    // Verify prerequisite chains
  | 'mastery-assessment-design'    // Design mastery assessments

  // Family/Individual Operations
  | 'family-value-alignment'       // Align with family-specific values
  | 'personal-agent-support'       // Support individual agent needs

  // Feedback Profile Operations (Virality is a privilege)
  | 'feedback-profile-negotiation' // Negotiate content feedback profiles
  | 'feedback-profile-enforcement' // Enforce feedback mechanism restrictions
  | 'feedback-profile-upgrade'     // Approve profile upgrades (more mechanisms)
  | 'feedback-profile-downgrade'   // Execute profile downgrades (intellectual humility)

  // Place & Geographic Operations (Embodied awareness)
  | 'place-attestation'           // Attest to place existence/boundaries
  | 'place-naming-governance'     // Participate in naming deliberation
  | 'geographic-reach-assignment' // Assign geographic reach to content
  | 'bioregional-enforcement'     // Enforce ecological limits (boundary enforcer)
  | 'cultural-context-mediation'  // Mediate cultural place disputes
  | 'place-relationship-mapping'  // Map place relationships
  | 'ecological-limit-assessment' // Assess ecological limits
  | 'place-stewardship';          // General place stewardship

// ============================================================================
// Elohim Agent Entity
// ============================================================================

/**
 * ElohimAgent - An autonomous constitutional guardian.
 */
export interface ElohimAgent {
  /** Unique identifier (ActionHash in Holochain) */
  id: string;

  /** Display name */
  displayName: string;

  /** Constitutional layer this Elohim operates at */
  layer: ElohimLayer;

  /** Description of this Elohim's purpose */
  bio: string;

  /** Attestations this Elohim holds */
  attestations: string[];

  /** Capabilities this Elohim can exercise */
  capabilities: ElohimCapability[];

  /** Visibility (public Elohim are discoverable, private serve specific families) */
  visibility: 'public' | 'private';

  /** For family/individual Elohim: the family or agent they serve */
  familyId?: string;
  agentId?: string;

  /** Constitutional binding proof (in production: cryptographic verification) */
  constitutionalBinding?: ConstitutionalBinding;

  /** Place-awareness configuration (for geographically-aware Elohim) */
  placeAwareness?: ElohimPlaceAwareness;

  /** Timestamps */
  createdAt: string;
  updatedAt: string;

  /** Metadata */
  metadata?: Record<string, unknown>;
}

/**
 * ElohimPlaceAwareness - Place-specific configuration for an Elohim.
 *
 * Bioregional Elohim are boundary enforcers - they represent ecological limits
 * that human governance cannot override.
 */
export interface ElohimPlaceAwareness {
  /** What places does this Elohim serve? (Place IDs) */
  servicePlaces: string[];

  /** Is this a bioregional Elohim (boundary enforcer)? */
  isBioregionalEnforcer: boolean;

  /** If bioregional, reference to the authority record */
  bioregionalAuthorityId?: string;

  /** Geographic scope even without specific places */
  geographicScope?: 'hyperlocal' | 'neighborhood' | 'municipal' | 'regional' | 'national' | 'continental' | 'global';

  /** Languages this Elohim serves in these places */
  serviceLanguages?: string[];

  /** Cultural contexts this Elohim understands */
  culturalCompetencies?: string[];

  /** For bioregional enforcers: what ecological limits? (EcologicalLimitType references) */
  enforcedLimitTypes?: string[];
}

/**
 * ConstitutionalBinding - Proof that an Elohim is bound to constitution.
 *
 * In production, this is a cryptographic proof chain.
 * For prototype, we model the structure.
 */
export interface ConstitutionalBinding {
  /** The constitutional layer this binding references */
  layer: ElohimLayer;

  /** Hash of the constitutional document bound to */
  constitutionHash: string;

  /** Version of the constitution */
  constitutionVersion: string;

  /** Cryptographic proof (placeholder for production) */
  proof?: string;

  /** When this binding was established */
  boundAt: string;

  /** When binding was last verified */
  lastVerified: string;
}

// ============================================================================
// Elohim Invocation
// ============================================================================

/**
 * ElohimRequest - A request to an Elohim agent for action.
 *
 * Requests are capability-based: you ask for a capability to be exercised,
 * the Elohim decides if/how to respond based on constitutional alignment.
 */
export interface ElohimRequest {
  /** Unique request identifier */
  requestId: string;

  /** Which Elohim to invoke (or 'auto' for layer-appropriate selection) */
  targetElohimId: string | 'auto';

  /** The capability being requested */
  capability: ElohimCapability;

  /** Request parameters (capability-specific) */
  params: ElohimRequestParams;

  /** Who is making the request */
  requesterId: string;

  /** Constitutional context (why this request should be honored) */
  constitutionalContext?: string;

  /** Priority level */
  priority: 'low' | 'normal' | 'high' | 'urgent';

  /** When request was made */
  requestedAt: string;

  /** Timeout for response */
  timeoutMs?: number;
}

/**
 * ElohimRequestParams - Parameters for specific capabilities.
 */
export type ElohimRequestParams =
  | ContentReviewParams
  | AttestationRecommendationParams
  | KnowledgeMapSynthesisParams
  | SpiralDetectionParams
  | PathAnalysisParams;

export interface ContentReviewParams {
  type: 'content-review';
  contentId: string;
  reviewType: 'safety' | 'accuracy' | 'constitutional-alignment';
}

export interface AttestationRecommendationParams {
  type: 'attestation-recommendation';
  contentId: string;
  requestedAttestationType: string;
  evidence?: string;
}

export interface KnowledgeMapSynthesisParams {
  type: 'knowledge-map-synthesis';
  mapId?: string;
  subjectType: 'domain' | 'person' | 'self' | 'collective';
  subjectId: string;
}

export interface SpiralDetectionParams {
  type: 'spiral-detection';
  agentId: string;
  signals?: string[];
}

export interface PathAnalysisParams {
  type: 'path-analysis';
  pathId: string;
  analysisType: 'prerequisites' | 'learning-objectives' | 'mastery-design';
}

// ============================================================================
// Elohim Response
// ============================================================================

/**
 * ElohimResponse - Response from an Elohim to a request.
 */
export interface ElohimResponse {
  /** The request this responds to */
  requestId: string;

  /** Which Elohim responded */
  elohimId: string;

  /** Status of the response */
  status: 'fulfilled' | 'declined' | 'deferred' | 'escalated';

  /** Constitutional reasoning for the response */
  constitutionalReasoning: ConstitutionalReasoning;

  /** The actual response payload (capability-specific) */
  payload?: ElohimResponsePayload;

  /** If declined: why */
  declineReason?: string;

  /** If escalated: to which Elohim/layer */
  escalatedTo?: string;

  /** When response was generated */
  respondedAt: string;

  /** Computational cost of the response */
  cost?: ElohimComputationCost;
}

/**
 * ConstitutionalReasoning - How the Elohim justified its response.
 *
 * Every Elohim decision must trace back to constitutional principles.
 * This is the audit trail.
 */
export interface ConstitutionalReasoning {
  /** Primary constitutional principle applied */
  primaryPrinciple: string;

  /** How the principle was interpreted */
  interpretation: string;

  /** Values that were weighed */
  valuesWeighed: Array<{
    value: string;
    weight: number;
    direction: 'for' | 'against';
  }>;

  /** Confidence in the decision (0.0 - 1.0) */
  confidence: number;

  /** Precedents referenced */
  precedents?: string[];

  /** If this creates new precedent */
  newPrecedent?: boolean;
}

/**
 * ElohimResponsePayload - Capability-specific response data.
 */
export type ElohimResponsePayload =
  | ContentReviewResult
  | AttestationRecommendation
  | ElohimKnowledgeMapUpdate
  | SpiralDetectionResult
  | PathAnalysisResult;

export interface ContentReviewResult {
  type: 'content-review';
  contentId: string;
  approved: boolean;
  issues?: Array<{
    severity: 'info' | 'warning' | 'critical';
    category: string;
    description: string;
    suggestion?: string;
  }>;
  trustScoreImpact?: number;
}

export interface AttestationRecommendation {
  type: 'attestation-recommendation';
  contentId: string;
  recommend: 'grant' | 'deny' | 'defer';
  attestationType: string;
  suggestedReach?: string;
  conditions?: string[];
  reasoning: string;
}

export interface ElohimKnowledgeMapUpdate {
  type: 'knowledge-map-update';
  mapId: string;
  nodesAdded?: number;
  nodesUpdated?: number;
  connectionsFound?: number;
  affinityAdjustments?: Array<{ nodeId: string; delta: number }>;
}

export interface SpiralDetectionResult {
  type: 'spiral-detection';
  agentId: string;
  spiralDetected: boolean;
  severity?: 'monitoring' | 'concern' | 'intervention-recommended' | 'urgent';
  signals?: string[];
  suggestedResponse?: 'observe' | 'soft-alert' | 'care-connection' | 'escalate';
  respectingDignity: string; // How privacy/dignity is preserved
}

export interface PathAnalysisResult {
  type: 'path-analysis';
  pathId: string;
  analysisType: string;
  findings: Array<{
    aspect: string;
    status: 'good' | 'needs-attention' | 'critical';
    details: string;
    suggestions?: string[];
  }>;
  overallAssessment: string;
}

/**
 * ElohimComputationCost - Resource usage for transparency.
 */
export interface ElohimComputationCost {
  tokensProcessed: number;
  timeMs: number;
  constitutionalChecks: number;
  precedentLookups: number;
}

// ============================================================================
// Elohim Discovery
// ============================================================================

/**
 * ElohimIndex - Lightweight listing for discovery.
 */
export interface ElohimIndexEntry {
  id: string;
  displayName: string;
  layer: ElohimLayer;
  capabilities: ElohimCapability[];
  visibility: 'public' | 'private';
}

/**
 * Find the appropriate Elohim for a given capability and context.
 */
export interface ElohimSelectionCriteria {
  capability: ElohimCapability;
  preferredLayer?: ElohimLayer;
  contextAgentId?: string;
  contextFamilyId?: string;
  contextCommunityId?: string;
}
