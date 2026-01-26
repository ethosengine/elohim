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
  contributionEvidenceJson: string | null;

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
  metadataJson: string | null;
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
  contributionEvidenceJson?: string;
  note?: string;
  metadataJson?: string;
}

/**
 * UpdateAllocationInput - Input for updating an allocation.
 */
export interface UpdateAllocationInput {
  allocationRatio?: number;
  allocationMethod?: AllocationMethod;
  contributionType?: ContributionType;
  contributionEvidenceJson?: string;
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
  disputeId: string;
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

// ============================================================================
// Wire Format Transformers
// ============================================================================

/**
 * Transform API response (camelCase View) to TypeScript domain model.
 * API now returns camelCase with parsed JSON objects.
 */
export function fromWireStewardshipAllocation(
  view: Record<string, unknown>
): StewardshipAllocation {
  // API returns parsed JSON objects; stringify for domain model compatibility
  const contributionEvidence = view['contributionEvidence'];
  const metadata = view['metadata'];

  return {
    id: view['id'] as string,
    contentId: view['contentId'] as string,
    stewardPresenceId: view['stewardPresenceId'] as string,
    allocationRatio: view['allocationRatio'] as number,
    allocationMethod: view['allocationMethod'] as AllocationMethod,
    contributionType: view['contributionType'] as ContributionType,
    contributionEvidenceJson: contributionEvidence ? JSON.stringify(contributionEvidence) : null,
    governanceState: view['governanceState'] as GovernanceState,
    disputeId: view['disputeId'] as string | null,
    disputeReason: view['disputeReason'] as string | null,
    disputedAt: view['disputedAt'] as string | null,
    disputedBy: view['disputedBy'] as string | null,
    negotiationSessionId: view['negotiationSessionId'] as string | null,
    elohimRatifiedAt: view['elohimRatifiedAt'] as string | null,
    elohimRatifierId: view['elohimRatifierId'] as string | null,
    effectiveFrom: view['effectiveFrom'] as string,
    effectiveUntil: view['effectiveUntil'] as string | null,
    supersededBy: view['supersededBy'] as string | null,
    recognitionAccumulated: view['recognitionAccumulated'] as number,
    lastRecognitionAt: view['lastRecognitionAt'] as string | null,
    note: view['note'] as string | null,
    metadataJson: metadata ? JSON.stringify(metadata) : null,
    createdAt: view['createdAt'] as string,
    updatedAt: view['updatedAt'] as string,
  };
}

/**
 * Transform ContentStewardship from API response (camelCase View).
 */
export function fromWireContentStewardship(view: Record<string, unknown>): ContentStewardship {
  const allocations = ((view['allocations'] as Record<string, unknown>[]) ?? []).map(a => ({
    allocation: fromWireStewardshipAllocation(a['allocation'] as Record<string, unknown>),
    steward: a['steward']
      ? fromWireContributorPresenceRef(a['steward'] as Record<string, unknown>)
      : null,
  }));

  return {
    contentId: view['contentId'] as string,
    allocations,
    totalAllocation: view['totalAllocation'] as number,
    hasDisputes: view['hasDisputes'] as boolean,
    primarySteward: view['primarySteward']
      ? fromWireStewardshipAllocation(view['primarySteward'] as Record<string, unknown>)
      : null,
  };
}

/**
 * Transform ContributorPresenceRef from API response (camelCase View).
 */
function fromWireContributorPresenceRef(view: Record<string, unknown>): ContributorPresenceRef {
  return {
    id: view['id'] as string,
    displayName: view['displayName'] as string,
    image: view['image'] as string | null,
    presenceState: view['presenceState'] as string,
  };
}
