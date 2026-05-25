/**
 * Stewardship Allocation Model - One-to-Many Content Stewardship
 *
 * From the Manifesto (Part IV-C):
 * "Content isn't ever owned by who might create it, it's stewarded by whoever
 * has the most relational connection to the content itself."
 *
 * Key concepts:
 * - StewardshipAllocation: Individual steward-content relationship with allocation ratio
 * - ContentStewardship: Aggregate view of all stewards for a piece of content
 * - Governance lifecycle: active → disputed → resolved (or superseded)
 *
 * Allocation ratios sum to 1.0 per content piece. Ratios are computed by Elohim
 * based on contribution evidence, but stewards can dispute "hostile takeovers".
 */

import type { JsonValue } from '@elohim/storage-client/generated';

// ============================================================================
// Allocation Methods
// ============================================================================

/**
 * AllocationMethod - How the allocation ratio was determined.
 */
export type AllocationMethod = 'manual' | 'computed' | 'negotiated';

export const ALLOCATION_METHOD_DESCRIPTIONS: Record<AllocationMethod, string> = {
  manual: 'Manually assigned by system or admin',
  computed: 'Auto-computed from contribution evidence',
  negotiated: 'Negotiated through Elohim governance process',
};

// ============================================================================
// Contribution Types
// ============================================================================

/**
 * ContributionType - How the steward contributed to the content.
 */
export type ContributionType =
  | 'original_creator'
  | 'author'
  | 'editor'
  | 'curator'
  | 'maintainer'
  | 'translator'
  | 'reviewer'
  | 'inherited';

export const CONTRIBUTION_TYPE_DESCRIPTIONS: Record<ContributionType, string> = {
  original_creator: 'Original creator of the content',
  author: 'Primary author of the content',
  editor: 'Made significant editorial changes',
  curator: 'Organized and structured the content',
  maintainer: 'Keeps the content up-to-date',
  translator: 'Translated to another language',
  reviewer: 'Provided expert review/validation',
  inherited: 'Inherited stewardship (bootstrap or transfer)',
};

// ============================================================================
// Governance States
// ============================================================================

/**
 * GovernanceState - Lifecycle state of a stewardship allocation.
 */
export type GovernanceState = 'active' | 'disputed' | 'pending_review' | 'superseded';

export const GOVERNANCE_STATE_DESCRIPTIONS: Record<GovernanceState, string> = {
  active: 'Currently active allocation',
  disputed: 'Under dispute - awaiting Elohim resolution',
  pending_review: 'Awaiting governance review',
  superseded: 'Replaced by a newer allocation',
};

// ============================================================================
// Stewardship Allocation
// ============================================================================

/**
 * StewardshipAllocation - Individual steward-content relationship.
 *
 * Tracks:
 * - Which steward (ContributorPresence) stewards which content
 * - Their allocation ratio (share of recognition flows)
 * - How the allocation was determined
 * - Governance lifecycle (disputes, ratification)
 */
export interface StewardshipAllocation {
  id: string;
  contentId: string;
  stewardPresenceId: string;

  // Allocation details
  allocationRatio: number; // 0.0-1.0, all for content sum to 1.0
  allocationMethod: AllocationMethod;
  contributionType: ContributionType;
  contributionEvidence: JsonValue | null;

  // Governance
  governanceState: GovernanceState;
  disputeId: string | null;
  disputeReason: string | null;
  disputedAt: string | null;
  disputedBy: string | null;
  negotiationSessionId: string | null;
  elohimRatifiedAt: string | null;
  elohimRatifierId: string | null;

  // Temporal
  effectiveFrom: string;
  effectiveUntil: string | null;
  supersededBy: string | null;

  // Recognition tracking
  recognitionAccumulated: number;
  lastRecognitionAt: string | null;

  // Metadata
  note: string | null;
  metadata: JsonValue | null;
  createdAt: string;
  updatedAt: string;
}

/**
 * StewardshipAllocationView - View model for UI display.
 */
export interface StewardshipAllocationView extends StewardshipAllocation {
  // Populated from join with ContributorPresence
  stewardDisplayName?: string;
  stewardImage?: string;
}

// ============================================================================
// Content Stewardship (Aggregate)
// ============================================================================

/**
 * ContentStewardship - Aggregate view of all stewards for a content piece.
 *
 * Provides:
 * - All active allocations with steward details
 * - Primary steward identification
 * - Dispute status
 */
export interface ContentStewardship {
  contentId: string;
  allocations: StewardshipAllocationWithPresence[];
  totalAllocation: number; // Should sum to 1.0
  hasDisputes: boolean;
  primarySteward: StewardshipAllocation | null;
}

/**
 * StewardshipAllocationWithPresence - Allocation with hydrated steward presence.
 */
export interface StewardshipAllocationWithPresence {
  allocation: StewardshipAllocation;
  steward: ContributorPresenceRef | null;
}

/**
 * ContributorPresenceRef - Minimal reference to a ContributorPresence.
 */
export interface ContributorPresenceRef {
  id: string;
  displayName: string;
  image: string | null;
  presenceState: string;
}

// ============================================================================
// Input Types
// ============================================================================

/**
 * CreateAllocationInput - Input for creating a stewardship allocation.
 */
export interface CreateAllocationInput {
  contentId: string;
  stewardPresenceId: string;
  allocationRatio?: number; // default: 1.0
  allocationMethod?: AllocationMethod; // default: 'manual'
  contributionType?: ContributionType; // default: 'inherited'
  contributionEvidence?: JsonValue;
  note?: string;
  metadata?: JsonValue;
}

/**
 * UpdateAllocationInput - Input for updating an allocation.
 */
export interface UpdateAllocationInput {
  allocationRatio?: number;
  allocationMethod?: AllocationMethod;
  contributionType?: ContributionType;
  contributionEvidence?: JsonValue;
  governanceState?: GovernanceState;
  disputeId?: string;
  disputeReason?: string;
  elohimRatifiedAt?: string;
  elohimRatifierId?: string;
  note?: string;
}

/**
 * FileDisputeInput - Input for filing a dispute on an allocation.
 */
export interface FileDisputeInput {
  disputedBy: string;
  reason: string;
}

/**
 * ResolveDisputeInput - Input for resolving a dispute (Elohim ratification).
 */
export interface ResolveDisputeInput {
  ratifierId: string;
  newState: GovernanceState;
}

// ============================================================================
// Query Types
// ============================================================================

/**
 * AllocationQuery - Query parameters for listing allocations.
 */
export interface AllocationQuery {
  contentId?: string;
  stewardPresenceId?: string;
  governanceState?: GovernanceState;
  activeOnly?: boolean;
  limit?: number;
  offset?: number;
}

// ============================================================================
// Bulk Operations
// ============================================================================

/**
 * BulkAllocationResult - Result of a bulk allocation operation.
 */
export interface BulkAllocationResult {
  created: number;
  failed: number;
  errors: string[];
}
