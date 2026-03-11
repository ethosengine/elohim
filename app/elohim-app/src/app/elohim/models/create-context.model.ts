/**
 * CreateContext Model - Rich context envelope for every "create" action.
 *
 * Assembled progressively on the user's device before the action
 * leaves the local DHT shard. Gives the elohim-agent sufficient
 * meaning to negotiate reach with constitutional wisdom.
 *
 * Architecture parallels Anthropic's layered context pipeline:
 *   Constitutional stack   ~ system prompt         (stable, verified)
 *   Community/family norms ~ project instructions   (session-level)
 *   Author reputation      ~ memory files           (cached, periodic)
 *   Graph neighborhood     ~ context search         (on-demand)
 *   Thread context         ~ conversation history   (immediate)
 *   The payload            ~ the user message       (the content)
 *
 * Design ethos: Trust is the computational substrate.
 * High-trust authors get lighter pipelines (fewer layers, less inference).
 * Low-trust or new authors get deeper analysis (all layers, more reasoning).
 * Trust is earned through contribution, never purchased.
 * See: Evolution of Trust (Nicky Case) for the game theory.
 *
 * From the Manifesto:
 * "Free speech does not mean free reach.
 *  There is no right to algorithmic amplification."
 */

import type { ElohimLayer } from './elohim-agent.model';
import type { AffinityScope, GeographicContext } from './protocol-core.model';
import type { ContentAttestationType } from '@app/lamad/models/content-attestation.model';
import type {
  ContentType,
  ContentFormat,
  ContentReach,
} from '@app/lamad/models/content-node.model';

// ============================================================================
// Context Layers
// ============================================================================

/**
 * Names for each layer in the assembly pipeline.
 * Used for transparency reporting and trust-adaptive depth decisions.
 */
export type ContextLayerName =
  | 'constitutional'
  | 'community-norms'
  | 'author-profile'
  | 'graph-neighborhood'
  | 'thread-context'
  | 'payload';

/**
 * Layer 1: Constitutional Stack (stable, DHT-verified).
 *
 * The foundational layer. Loaded once at session start, verified
 * against the DHT hash. Equivalent to Anthropic's system prompt.
 */
export interface ConstitutionalContextLayer {
  /** Hash of the active constitutional stack (for DHT verification) */
  stackHash: string;

  /** When the stack was last verified against DHT */
  lastVerifiedAt: string;

  /** Applicable constitutional layer for this action */
  applicableLayer: ElohimLayer;

  /** Relevant principles for this content type (pre-filtered) */
  relevantPrinciples: { id: string; name: string; weight: number }[];

  /** Active boundaries that could apply to this action */
  activeBoundaries: { id: string; type: string; enforcement: string }[];
}

/**
 * Layer 2: Community/Family Norms (session-level, cached).
 *
 * Community-specific rules and defaults. Cached in IndexedDB with
 * 1-hour TTL. Equivalent to Anthropic's project instructions.
 */
export interface CommunityNormsLayer {
  /** Community IDs this action is contextualized within */
  communityIds: string[];

  /** Family ID if applicable */
  familyId?: string;

  /** Community-specific feedback profile overrides */
  feedbackProfileOverrides?: string[];

  /** Community reach defaults for this content type */
  communityReachDefaults?: Partial<Record<ContentType, ContentReach>>;

  /** Active governance proposals affecting content (if any) */
  activeGovernanceContext?: string[];
}

/**
 * Layer 3: Author Reputation (cached, periodic update).
 *
 * The author's contribution history and trust standing. Cached in
 * IndexedDB with 15-minute TTL. Equivalent to Anthropic's memory files.
 *
 * This layer drives the trust-adaptive pipeline: higher trust means
 * lighter assembly and less inference cost.
 */
export interface AuthorProfileLayer {
  /** Author's agent ID */
  authorId: string;

  /** Current trust indicators (reuses TrustBadgeService computation) */
  trustIndicators: {
    /** Aggregate trust score (0.0-1.0) */
    trustScore: number;
    /** Types of attestations currently active for this author */
    activeAttestationTypes: ContentAttestationType[];
    /** Total content contributions across all time */
    totalContributions: number;
    /** Engagement metrics */
    engagementHistory: {
      totalEndorsements: number;
      totalCitations: number;
      mediationPassThroughRate?: number;
    };
  };

  /** Author's current reach ceiling (highest reach they've earned) */
  reachCeiling: ContentReach;

  /** Recent contribution summary */
  recentContributions: {
    last30Days: number;
    averageTrustScore: number;
    topContentTypes: ContentType[];
  };

  /** When this profile was last computed */
  computedAt: string;
}

/**
 * Layer 4: Content Graph Neighborhood (retrieved on demand).
 *
 * 1-hop (or 2-hop for low-trust) traversal of the content graph
 * around the payload. Never cached - always fresh from the local
 * DHT shard. Equivalent to Anthropic's context search.
 */
export interface GraphNeighborhoodLayer {
  /** Content nodes related to the payload (1-hop neighborhood) */
  relatedNodes: {
    nodeId: string;
    title: string;
    contentType: ContentType;
    reach: ContentReach;
    trustScore: number;
    relationshipType: string;
    confidence: number;
  }[];

  /** If this is a reply/comment: the parent content's context */
  parentContext?: {
    nodeId: string;
    reach: ContentReach;
    trustScore: number;
    authorId: string;
    feedbackProfileId?: string;
    activeFlags: { type: string; reason: string }[];
  };

  /** Existing attestations in the neighborhood */
  neighborhoodAttestationSummary: {
    totalAttestations: number;
    averageTrustScore: number;
    dominantReach: ContentReach;
  };

  /** Community sentiment signal (aggregated from recent activity) */
  communitySentiment?: {
    recentActivityLevel: 'quiet' | 'normal' | 'active' | 'heated';
    recentFlagRate: number;
  };
}

/**
 * Layer 5: Thread/Session Context (immediate, in-memory).
 *
 * Where the user is in their session - learning path position,
 * discussion thread, recent actions. Equivalent to Anthropic's
 * conversation history.
 */
export interface ThreadContextLayer {
  /** If within a learning path: path context */
  pathContext?: {
    pathId: string;
    pathTitle: string;
    stepIndex: number;
    totalSteps: number;
  };

  /** If within an existing thread/discussion */
  threadId?: string;

  /** Previous actions in this session (for spiral detection) */
  sessionActions: {
    actionType: string;
    contentType?: string;
    timestamp: string;
  }[];
}

// ============================================================================
// Create Payload (Layer 6: The Content Itself)
// ============================================================================

/**
 * Action types that trigger context assembly and reach negotiation.
 * Every "create" action goes through the pipeline.
 */
export type CreateActionType =
  | 'content-create'
  | 'comment'
  | 'attestation-request'
  | 'endorsement'
  | 'governance-vote'
  | 'feedback-reaction'
  | 'share-with-context';

/**
 * Layer 6: The Payload - what the human is actually creating.
 *
 * Contains the content plus the author's preferences for reach/affinity.
 * The requestedReach is the author's PREFERENCE, not the final decision.
 */
export interface CreatePayload {
  /** Action type */
  actionType: CreateActionType;

  /** The content being created */
  content: string | object;

  /** Content type (semantic category) */
  contentType: ContentType;

  /** Content format for rendering */
  contentFormat: ContentFormat;

  /** Author-requested reach (preference, not final decision) */
  requestedReach: ContentReach;

  /** Author-requested affinity scope */
  requestedAffinity?: AffinityScope;

  /** Geographic context for place-aware content */
  geographicContext?: GeographicContext;

  /** Tags for categorization */
  tags: string[];

  /** If this is a reply: the parent content ID */
  parentContentId?: string;

  /** If this cites other content */
  citedContentIds?: string[];
}

// ============================================================================
// The CreateContext Envelope
// ============================================================================

/**
 * CreateContext - The full context envelope assembled before a "create" action.
 *
 * Assembled progressively on the user's device. All 6 layers populated
 * (or partially, with skipped reasons for transparency). Passed to
 * elohim-agent for reach negotiation.
 */
export interface CreateContext {
  /** Unique context assembly identifier (for audit trail) */
  contextId: string;

  /** When this context was assembled */
  assembledAt: string;

  /** Assembly duration in ms (transparency) */
  assemblyDurationMs: number;

  // Six layers - from stable to immediate
  constitutional: ConstitutionalContextLayer;
  communityNorms: CommunityNormsLayer;
  authorProfile: AuthorProfileLayer;
  graphNeighborhood: GraphNeighborhoodLayer;
  threadContext: ThreadContextLayer;
  payload: CreatePayload;

  // Transparency metadata
  /** Which layers were actually populated */
  layersPopulated: ContextLayerName[];

  /** Which layers were skipped and why */
  layersSkipped: { layer: ContextLayerName; reason: string }[];

  /** Cost of assembly (for transparency) */
  assemblyCost: ContextAssemblyCost;
}

/**
 * Cost breakdown for context assembly (transparency).
 */
export interface ContextAssemblyCost {
  /** Time spent on each layer (ms) */
  layerTimings: Partial<Record<ContextLayerName, number>>;

  /** Number of local DHT lookups */
  dhtLookups: number;

  /** Number of IndexedDB cache hits */
  cacheHits: number;

  /** Number of cache misses requiring computation */
  cacheMisses: number;

  /** Total bytes assembled */
  totalSizeBytes: number;
}

// ============================================================================
// Trust-Adaptive Pipeline Depth
// ============================================================================

/**
 * Safety scan intensity level.
 * High-trust authors can skip or get light scans.
 * Low-trust or bad-faith patterns trigger deep analysis.
 */
export type SafetyScanLevel = 'skip' | 'light' | 'standard' | 'deep';

/**
 * PipelineDepth - Trust-adaptive configuration for context assembly.
 *
 * Trust is the substrate that determines inference cost.
 * High-trust authors get lightweight pipelines (fewer layers, less LLM reasoning).
 * Low-trust authors get full pipelines (all layers, deep analysis).
 *
 * This creates the virtuous loop:
 *   Good faith → trust grows → less friction → easier posting → more good content
 * And the friction loop:
 *   Bad faith → trust drops → more friction → harder posting → exploitation collapses
 */
export interface PipelineDepth {
  /** Which layers to assemble (vs skip) */
  requiredLayers: ContextLayerName[];

  /** Safety review threshold */
  safetyScanLevel: SafetyScanLevel;

  /** How much graph neighborhood to retrieve (0=none, 1=direct, 2=two-hop) */
  neighborhoodHops: 0 | 1 | 2;

  /** Inference budget hint for the LLM (max tokens for reasoning) */
  maxReasoningTokens: number;
}

/**
 * Determine pipeline depth based on author trust profile.
 *
 * Trust must be earned, never purchased. No amount of BYOK compute
 * can bypass this function - pipeline depth is determined by
 * demonstrated behavior in the social graph.
 */
/** Layers always required regardless of trust level */
const CONSTITUTIONAL_LAYER: ContextLayerName = 'constitutional';
const AUTHOR_PROFILE_LAYER: ContextLayerName = 'author-profile';

export function determinePipelineDepth(authorProfile: AuthorProfileLayer): PipelineDepth {
  const { trustScore } = authorProfile.trustIndicators;
  const { last30Days } = authorProfile.recentContributions;

  // High trust: lightweight pipeline - trust IS the evidence
  if (trustScore > 0.8 && last30Days > 10) {
    return {
      requiredLayers: [CONSTITUTIONAL_LAYER, AUTHOR_PROFILE_LAYER, 'payload'],
      safetyScanLevel: 'light',
      neighborhoodHops: 0,
      maxReasoningTokens: 256,
    };
  }

  // Low trust or brand new: full pipeline with deep analysis
  if (trustScore < 0.3 || last30Days === 0) {
    return {
      requiredLayers: [
        CONSTITUTIONAL_LAYER,
        'community-norms',
        AUTHOR_PROFILE_LAYER,
        'graph-neighborhood',
        'thread-context',
        'payload',
      ],
      safetyScanLevel: 'deep',
      neighborhoodHops: 2,
      maxReasoningTokens: 2048,
    };
  }

  // Standard pipeline for moderate trust
  return {
    requiredLayers: [CONSTITUTIONAL_LAYER, AUTHOR_PROFILE_LAYER, 'graph-neighborhood', 'payload'],
    safetyScanLevel: 'standard',
    neighborhoodHops: 1,
    maxReasoningTokens: 1024,
  };
}

// ============================================================================
// Reach Negotiation (Request/Response)
// ============================================================================

/**
 * ReachNegotiationParams - Request params for reach-negotiation capability.
 */
export interface ReachNegotiationParams {
  type: 'reach-negotiation';
  createContext: CreateContext;
}

/**
 * ReachNegotiationResult - What the Elohim recommends for this content's reach.
 */
export interface ReachNegotiationResult {
  type: 'reach-negotiation';

  /** The negotiated reach level */
  recommendedReach: ContentReach;

  /** If different from requested, why */
  reachDelta?: {
    requested: ContentReach;
    recommended: ContentReach;
    reason: string;
  };

  /** Whether the content passed safety review */
  safetyCleared: boolean;

  /** Safety issues found (if any) */
  safetyIssues?: {
    severity: 'info' | 'warning' | 'critical';
    category: string;
    description: string;
  }[];

  /** Conditions for this reach (if any) */
  conditions: string[];

  /** Suggested attestation path to earn higher reach */
  attestationPath?: {
    currentReach: ContentReach;
    targetReach: ContentReach;
    requiredAttestations: ContentAttestationType[];
    estimatedTimeline?: string;
  };

  /** Full transparency report */
  transparencyReport: TransparencyReport;
}

/**
 * TransparencyReport - Full audit of the reach decision.
 *
 * The user always sees this before content leaves the device.
 * Expandable in the UI - shows exactly what context was considered
 * and how it influenced the decision.
 */
export interface TransparencyReport {
  /** What context layers were considered */
  layersConsidered: ContextLayerName[];

  /** Key factors in the decision */
  decisionFactors: {
    factor: string;
    influence: 'positive' | 'negative' | 'neutral';
    weight: number;
    evidence: string;
  }[];

  /** Author's trust standing summary */
  authorStanding: string;

  /** Community health at time of decision */
  communityHealth: string;

  /** Graph neighborhood influence */
  neighborhoodInfluence: string;
}

// ============================================================================
// Content Birth Context (Immutable Provenance)
// ============================================================================

/**
 * ContentBirthContext - Immutable provenance record embedded at creation.
 *
 * Captures the web of meaning that existed when "post" was clicked.
 * Signed into the author's source chain. Travels with the content.
 * Makes recontextualization detectable.
 *
 * Like database normalization (CreateDTM, UpdateDTM, audit trail) but
 * for MEMORY - not just "row inserted at 14:32 by user_id 7" but
 * "written during heated community discussion about water rights,
 * by an author with 3 years of trusted contributions, when the
 * constitutional stack was at version X."
 *
 * Privacy rule: Detail scales with reach. More reach requires more
 * transparency. Private content gets minimal birth context.
 * Commons content gets full provenance.
 *
 * Anti-weaponization properties:
 * - Screenshots without birth context are visibly incomplete
 * - Recontextualization is detectable (compare birth context to new framing)
 * - Historical context is immutable (signed into source chain)
 * - Chains of provenance (replies carry parentBirthContextHash)
 * - Trust degrades without context (protocol rewards keeping context intact)
 */
export interface ContentBirthContext {
  /** Hash of the full CreateContext this was derived from (for audit) */
  sourceContextHash: string;

  /** When the post was born (ISO timestamp, source-chain signed) */
  createdAt: string;

  // --- Who created this ---

  /** Author's trust standing at time of creation */
  authorTrustSnapshot: {
    trustScore: number;
    reachCeiling: ContentReach;
    totalContributions: number;
    activeAttestationTypes: ContentAttestationType[];
  };

  // --- Why they created it ---

  /** Author's stated intent (optional, author-provided) */
  authorIntent?: string;

  // --- What was going on ---

  /** Constitutional stack hash at time of creation (verifiable against DHT) */
  constitutionalStackHash: string;

  /** Community health signal at time of creation */
  communityMood?: {
    activityLevel: 'quiet' | 'normal' | 'active' | 'heated';
    recentFlagRate: number;
  };

  /** If replying: the parent's birth context hash (chain of provenance) */
  parentBirthContextHash?: string;

  /** Content graph neighborhood summary (shape, not full nodes) */
  neighborhoodSummary: {
    relatedNodeCount: number;
    averageNeighborhoodTrust: number;
    dominantReach: ContentReach;
  };

  /** Active governance context (e.g., "during municipal election period") */
  activeGovernanceMarkers?: string[];

  // --- How reach was decided ---

  /** Reach as negotiated */
  negotiatedReach: ContentReach;

  /** Reference to the reach negotiation request */
  reachNegotiationId?: string;

  /** Hash of the full reach negotiation reasoning (stored separately) */
  reachReasoningHash?: string;
}

/**
 * Derive a ContentBirthContext from a full CreateContext + negotiation result.
 *
 * Privacy gradient: detail scales with reach level.
 *   private       → minimal (just createdAt + stackHash)
 *   local/neighborhood → + author trust + community mood
 *   municipal+    → + neighborhood summary + governance markers
 *   commons       → full birth context (maximum transparency)
 */
export function deriveBirthContext(
  context: CreateContext,
  negotiationResult: ReachNegotiationResult,
  sourceContextHash: string,
  authorIntent?: string
): ContentBirthContext {
  const reach = negotiationResult.recommendedReach;

  // Minimal birth context for private content
  const birth: ContentBirthContext = {
    sourceContextHash,
    createdAt: context.assembledAt,
    authorTrustSnapshot: {
      trustScore: context.authorProfile.trustIndicators.trustScore,
      reachCeiling: context.authorProfile.reachCeiling,
      totalContributions: context.authorProfile.trustIndicators.totalContributions,
      activeAttestationTypes: context.authorProfile.trustIndicators.activeAttestationTypes,
    },
    constitutionalStackHash: context.constitutional.stackHash,
    neighborhoodSummary: {
      relatedNodeCount: context.graphNeighborhood.relatedNodes.length,
      averageNeighborhoodTrust:
        context.graphNeighborhood.neighborhoodAttestationSummary.averageTrustScore,
      dominantReach: context.graphNeighborhood.neighborhoodAttestationSummary.dominantReach,
    },
    negotiatedReach: reach,
  };

  // For private content, strip most context
  if (reach === 'private') {
    return {
      sourceContextHash,
      createdAt: context.assembledAt,
      authorTrustSnapshot: birth.authorTrustSnapshot,
      constitutionalStackHash: context.constitutional.stackHash,
      neighborhoodSummary: {
        relatedNodeCount: 0,
        averageNeighborhoodTrust: 0,
        dominantReach: 'private',
      },
      negotiatedReach: reach,
    };
  }

  // local/neighborhood: add community mood
  if (reach === 'local' || reach === 'neighborhood' || reach === 'invited') {
    birth.communityMood = context.graphNeighborhood.communitySentiment
      ? {
          activityLevel: context.graphNeighborhood.communitySentiment.recentActivityLevel,
          recentFlagRate: context.graphNeighborhood.communitySentiment.recentFlagRate,
        }
      : undefined;
    birth.authorIntent = authorIntent;
    return birth;
  }

  // municipal+: add governance markers
  birth.communityMood = context.graphNeighborhood.communitySentiment
    ? {
        activityLevel: context.graphNeighborhood.communitySentiment.recentActivityLevel,
        recentFlagRate: context.graphNeighborhood.communitySentiment.recentFlagRate,
      }
    : undefined;
  birth.authorIntent = authorIntent;
  birth.activeGovernanceMarkers = context.communityNorms.activeGovernanceContext;

  // Parent chain of provenance (for replies)
  if (context.payload.parentContentId) {
    birth.parentBirthContextHash = undefined; // Set by caller from parent's birth context
  }

  return birth;
}
