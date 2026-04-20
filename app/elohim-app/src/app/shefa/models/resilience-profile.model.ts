/**
 * Resilience Profile Model - P2P Data Protection Projection
 *
 * This is a shefa projection, NOT a new protocol primitive. It composes
 * existing primitives — ShardManifest, MutualAidContext, Commitment,
 * CustodianNode — into a human-legible resilience view.
 *
 * Core Principles:
 * - Protection is per-content: medical records vs shared movies need
 *   different protection levels. A single global score is insufficient.
 * - Attestation through use, not ceremony: every shard fetch is a
 *   heartbeat. Liveness is proven by actual data flows, not pledges.
 * - Data has a lifecycle: the right to be forgotten is a resilience
 *   concern, not just a privacy concern. Deletion must propagate.
 * - Elohim are LLMs: they need narrative memory alongside quantitative
 *   scores. A number alone cannot capture "this person lost two peers
 *   last month and their medical records are under-replicated."
 * - Elohim can help set the score at higher abstract levels requiring
 *   discernment — e.g., whether a peer's declining availability is
 *   temporary or signals network attrition.
 */

import type { AgentRef } from '@app/elohim/models/coordination-envelope.model';
import type { ReachLevel } from '@app/elohim/models/protocol-core.model';

// =============================================================================
// Core Profile
// =============================================================================

/**
 * ProtectionStatus - Overall protection classification for a human's data
 */
export type ProtectionStatus = 'at-risk' | 'partial' | 'protected';

/**
 * ResilienceProfile - The primary projection summarizing a human's P2P
 * data protection posture across all content they steward.
 */
export interface ResilienceProfile {
  humanId: string;
  overallScore: number; // 0-1 normalized
  protectionStatus: ProtectionStatus;
  shardHealth: ShardHealthSummary;
  commitmentHealth: CommitmentHealthSummary;
  trustCircleDepth: TrustCircleDepth;
  contentRiskBreakdown: ContentRiskBucket[];
  nextAction?: ResilienceAction;
  elohimAssessment?: ElohimResilienceAssessment;
  lastComputedAt: string; // ISO 8601
}

// =============================================================================
// Shard Health
// =============================================================================

/**
 * ShardHealthSummary - Aggregate health of shard distribution across peers
 *
 * Reflects the physical redundancy of a human's data in the network.
 * A healthy shard distribution means content survives peer churn.
 */
export interface ShardHealthSummary {
  totalBlobs: number;
  totalShards: number;
  distinctPeers: number;
  averageShardsPerBlob: number;
  encodingBreakdown: {
    single: number;
    chunked: number;
    reedSolomon: number;
  };
  singlePointOfFailureCount: number;
  lastAccessVerifiedAt: string;
}

// =============================================================================
// Commitment Health
// =============================================================================

/**
 * CommitmentHealthSummary - Health of mutual aid commitments
 *
 * Commitments are the social layer of resilience: peers who have agreed
 * to steward each other's data. Reciprocation and coverage matter.
 */
export interface CommitmentHealthSummary {
  activeCommitments: number;
  reciprocatedCommitments: number;
  expiringSoon: number;
  totalPeersCommitted: number;
  commitmentCoverage: number; // 0-1
}

// =============================================================================
// Trust Circle Depth
// =============================================================================

/**
 * TrustCircleDepth - Distribution of peers across trust tiers
 *
 * Resilience improves with diversity across trust levels. Relying only
 * on household peers is fragile (correlated failure); institutional
 * peers provide uncorrelated redundancy but less intimacy.
 */
export interface TrustCircleDepth {
  householdPeers: number;
  friendPeers: number;
  communityPeers: number;
  institutionalPeers: number;
  totalCircles: number;
}

// =============================================================================
// Content Risk Buckets
// =============================================================================

/**
 * ContentRiskBucket - Risk assessment grouped by content reach level
 *
 * Different content needs different protection. A personal medical record
 * (reach: 'self') needs aggressive replication; a widely-shared article
 * (reach: 'network') is naturally resilient through popularity.
 */
export interface ContentRiskBucket {
  reach: ReachLevel; // from protocol-core
  contentCount: number;
  shardDistribution: number;
  adequacy: number; // 0-1
  exemplar?: string;
}

// =============================================================================
// Resilience Actions
// =============================================================================

/**
 * ResilienceActionType - Categories of actions to improve resilience
 */
export type ResilienceActionType = 'connect' | 'diversify' | 'renew' | 'review' | 'release';

/**
 * ResilienceAction - A concrete next step to improve data protection
 */
export interface ResilienceAction {
  type: ResilienceActionType;
  description: string;
  suggestedPeerIds?: string[];
  urgency: 'whenever' | 'soon' | 'now';
}

// =============================================================================
// Elohim Assessment
// =============================================================================

/**
 * ElohimResilienceAssessment - LLM-generated resilience assessment
 *
 * Elohim agents produce narrative assessments that capture nuance a
 * numeric score cannot. The assessment includes memories (persistent
 * context across sessions), concerns (actionable issues), and
 * attestations (references to constitutional or governance basis).
 */
export interface ElohimResilienceAssessment {
  assessedAt: string;
  assessedBy: AgentRef; // from coordination-envelope
  overallAdequacy: number;
  narrative: string;
  memories: ResilienceMemory[];
  concerns: ResilienceConcern[];
  attestations: string[];
  constitutionalBasis?: string;
}

/**
 * ResilienceMemory - Persistent narrative context for elohim agents
 *
 * Memories allow elohim to track resilience context across sessions.
 * They can be superseded (not deleted) to maintain audit trail.
 */
export interface ResilienceMemory {
  id: string;
  recordedAt: string;
  updatedAt: string;
  content: string; // Free text
  relevance: 'active' | 'background' | 'resolved';
  relatedContentIds?: string[];
  relatedHumanIds?: string[];
  supersededBy?: string;
}

/**
 * ResilienceConcern - A specific issue identified by an elohim agent
 */
export interface ResilienceConcern {
  severity: 'informational' | 'concerning' | 'critical';
  description: string;
  affectedContentIds?: string[];
  suggestedAction?: string;
}
