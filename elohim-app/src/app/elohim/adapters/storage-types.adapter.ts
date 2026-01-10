/**
 * Angular-specific adapters for generated wire types.
 *
 * The base types are generated from Rust Diesel models - see:
 *   holochain/sdk/storage-client-ts/src/generated/
 *
 * This file provides:
 * 1. camelCase view types for Angular templates
 * 2. Transformation functions (wire -> view)
 * 3. Type-safe transformer utilities
 *
 * WHY THIS EXISTS:
 * - Generated wire types use snake_case (Rust/SQLite convention)
 * - Angular templates use camelCase (TypeScript/JavaScript convention)
 * - This adapter bridges the two without modifying generated types
 */

import type {
  Relationship,
  HumanRelationship,
  ContributorPresence,
  EconomicEvent,
  ContentMastery,
  Content,
  Path,
  Step,
  Chapter,
} from '@elohim/storage-client/generated';

// =============================================================================
// Content Relationship View Types
// =============================================================================

/**
 * Content relationship - camelCase view model for Angular templates.
 */
export interface RelationshipView {
  id: string;
  appId: string;
  sourceId: string;
  targetId: string;
  relationshipType: string;
  confidence: number;
  inferenceSource: string;
  isBidirectional: boolean;
  inverseRelationshipId: string | null;
  provenanceChainJson: string | null;
  governanceLayer: string | null;
  reach: string;
  metadataJson: string | null;
  createdAt: string;
  updatedAt: string;
}

/**
 * Transform wire format to RelationshipView.
 */
export function transformRelationshipFromWire(wire: Relationship): RelationshipView {
  return {
    id: wire.id,
    appId: wire.app_id,
    sourceId: wire.source_id,
    targetId: wire.target_id,
    relationshipType: wire.relationship_type,
    confidence: wire.confidence,
    inferenceSource: wire.inference_source,
    isBidirectional: wire.is_bidirectional === 1,
    inverseRelationshipId: wire.inverse_relationship_id,
    provenanceChainJson: wire.provenance_chain_json,
    governanceLayer: wire.governance_layer,
    reach: wire.reach,
    metadataJson: wire.metadata_json,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
  };
}

// =============================================================================
// Human Relationship View Types
// =============================================================================

/**
 * Human relationship - camelCase view model for Angular templates.
 */
export interface HumanRelationshipView {
  id: string;
  appId: string;
  partyAId: string;
  partyBId: string;
  relationshipType: string;
  intimacyLevel: string;
  isBidirectional: boolean;
  consentGivenByA: boolean;
  consentGivenByB: boolean;
  custodyEnabledByA: boolean;
  custodyEnabledByB: boolean;
  autoCustodyEnabled: boolean;
  emergencyAccessEnabled: boolean;
  initiatedBy: string;
  verifiedAt: string | null;
  governanceLayer: string | null;
  reach: string;
  contextJson: string | null;
  createdAt: string;
  updatedAt: string;
  expiresAt: string | null;
  /** Derived: both parties have consented */
  isFullyConsented: boolean;
}

/**
 * Transform wire format to HumanRelationshipView.
 */
export function transformHumanRelationshipFromWire(wire: HumanRelationship): HumanRelationshipView {
  const consentA = wire.consent_given_by_a === 1;
  const consentB = wire.consent_given_by_b === 1;

  return {
    id: wire.id,
    appId: wire.app_id,
    partyAId: wire.party_a_id,
    partyBId: wire.party_b_id,
    relationshipType: wire.relationship_type,
    intimacyLevel: wire.intimacy_level,
    isBidirectional: wire.is_bidirectional === 1,
    consentGivenByA: consentA,
    consentGivenByB: consentB,
    custodyEnabledByA: wire.custody_enabled_by_a === 1,
    custodyEnabledByB: wire.custody_enabled_by_b === 1,
    autoCustodyEnabled: wire.auto_custody_enabled === 1,
    emergencyAccessEnabled: wire.emergency_access_enabled === 1,
    initiatedBy: wire.initiated_by,
    verifiedAt: wire.verified_at,
    governanceLayer: wire.governance_layer,
    reach: wire.reach,
    contextJson: wire.context_json,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
    expiresAt: wire.expires_at,
    isFullyConsented: consentA && consentB,
  };
}

// =============================================================================
// Contributor Presence View Types
// =============================================================================

/**
 * Contributor presence - camelCase view model for Angular templates.
 */
export interface ContributorPresenceView {
  id: string;
  appId: string;
  displayName: string;
  presenceState: string;
  externalIdentifiersJson: string | null;
  establishingContentIdsJson: string;
  affinityTotal: number;
  uniqueEngagers: number;
  citationCount: number;
  recognitionScore: number;
  recognitionByContentJson: string | null;
  lastRecognitionAt: string | null;
  stewardId: string | null;
  stewardshipStartedAt: string | null;
  stewardshipCommitmentId: string | null;
  stewardshipQualityScore: number | null;
  claimInitiatedAt: string | null;
  claimVerifiedAt: string | null;
  claimVerificationMethod: string | null;
  claimEvidenceJson: string | null;
  claimedAgentId: string | null;
  claimRecognitionTransferredValue: number | null;
  claimFacilitatedBy: string | null;
  image: string | null;
  note: string | null;
  metadataJson: string | null;
  createdAt: string;
  updatedAt: string;
  /** Derived: parsed establishing content IDs */
  establishingContentIds: string[];
}

/**
 * Transform wire format to ContributorPresenceView.
 */
export function transformContributorPresenceFromWire(wire: ContributorPresence): ContributorPresenceView {
  let establishingContentIds: string[] = [];
  try {
    establishingContentIds = JSON.parse(wire.establishing_content_ids_json) || [];
  } catch {
    // Ignore parse errors
  }

  return {
    id: wire.id,
    appId: wire.app_id,
    displayName: wire.display_name,
    presenceState: wire.presence_state,
    externalIdentifiersJson: wire.external_identifiers_json,
    establishingContentIdsJson: wire.establishing_content_ids_json,
    affinityTotal: wire.affinity_total,
    uniqueEngagers: wire.unique_engagers,
    citationCount: wire.citation_count,
    recognitionScore: wire.recognition_score,
    recognitionByContentJson: wire.recognition_by_content_json,
    lastRecognitionAt: wire.last_recognition_at,
    stewardId: wire.steward_id,
    stewardshipStartedAt: wire.stewardship_started_at,
    stewardshipCommitmentId: wire.stewardship_commitment_id,
    stewardshipQualityScore: wire.stewardship_quality_score,
    claimInitiatedAt: wire.claim_initiated_at,
    claimVerifiedAt: wire.claim_verified_at,
    claimVerificationMethod: wire.claim_verification_method,
    claimEvidenceJson: wire.claim_evidence_json,
    claimedAgentId: wire.claimed_agent_id,
    claimRecognitionTransferredValue: wire.claim_recognition_transferred_value,
    claimFacilitatedBy: wire.claim_facilitated_by,
    image: wire.image,
    note: wire.note,
    metadataJson: wire.metadata_json,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
    establishingContentIds,
  };
}

// =============================================================================
// Economic Event View Types
// =============================================================================

/**
 * Economic event - camelCase view model for Angular templates.
 */
export interface EconomicEventView {
  id: string;
  appId: string;
  action: string;
  provider: string;
  receiver: string;
  resourceConformsTo: string | null;
  resourceInventoriedAs: string | null;
  resourceClassifiedAsJson: string | null;
  resourceQuantityValue: number | null;
  resourceQuantityUnit: string | null;
  effortQuantityValue: number | null;
  effortQuantityUnit: string | null;
  hasPointInTime: string;
  hasDuration: string | null;
  inputOf: string | null;
  outputOf: string | null;
  lamadEventType: string | null;
  contentId: string | null;
  contributorPresenceId: string | null;
  pathId: string | null;
  triggeredBy: string | null;
  state: string;
  note: string | null;
  metadataJson: string | null;
  createdAt: string;
}

/**
 * Transform wire format to EconomicEventView.
 */
export function transformEconomicEventFromWire(wire: EconomicEvent): EconomicEventView {
  return {
    id: wire.id,
    appId: wire.app_id,
    action: wire.action,
    provider: wire.provider,
    receiver: wire.receiver,
    resourceConformsTo: wire.resource_conforms_to,
    resourceInventoriedAs: wire.resource_inventoried_as,
    resourceClassifiedAsJson: wire.resource_classified_as_json,
    resourceQuantityValue: wire.resource_quantity_value,
    resourceQuantityUnit: wire.resource_quantity_unit,
    effortQuantityValue: wire.effort_quantity_value,
    effortQuantityUnit: wire.effort_quantity_unit,
    hasPointInTime: wire.has_point_in_time,
    hasDuration: wire.has_duration,
    inputOf: wire.input_of,
    outputOf: wire.output_of,
    lamadEventType: wire.lamad_event_type,
    contentId: wire.content_id,
    contributorPresenceId: wire.contributor_presence_id,
    pathId: wire.path_id,
    triggeredBy: wire.triggered_by,
    state: wire.state,
    note: wire.note,
    metadataJson: wire.metadata_json,
    createdAt: wire.created_at,
  };
}

// =============================================================================
// Content Mastery View Types
// =============================================================================

/**
 * Content mastery - camelCase view model for Angular templates.
 */
export interface ContentMasteryView {
  id: string;
  appId: string;
  humanId: string;
  contentId: string;
  masteryLevel: string;
  masteryLevelIndex: number;
  freshnessScore: number;
  needsRefresh: boolean;
  engagementCount: number;
  lastEngagementType: string | null;
  lastEngagementAt: string | null;
  levelAchievedAt: string | null;
  contentVersionAtMastery: string | null;
  assessmentEvidenceJson: string | null;
  privilegesJson: string | null;
  createdAt: string;
  updatedAt: string;
}

/**
 * Transform wire format to ContentMasteryView.
 */
export function transformContentMasteryFromWire(wire: ContentMastery): ContentMasteryView {
  return {
    id: wire.id,
    appId: wire.app_id,
    humanId: wire.human_id,
    contentId: wire.content_id,
    masteryLevel: wire.mastery_level,
    masteryLevelIndex: wire.mastery_level_index,
    freshnessScore: wire.freshness_score,
    needsRefresh: wire.needs_refresh === 1,
    engagementCount: wire.engagement_count,
    lastEngagementType: wire.last_engagement_type,
    lastEngagementAt: wire.last_engagement_at,
    levelAchievedAt: wire.level_achieved_at,
    contentVersionAtMastery: wire.content_version_at_mastery,
    assessmentEvidenceJson: wire.assessment_evidence_json,
    privilegesJson: wire.privileges_json,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
  };
}

// =============================================================================
// Content View Types
// =============================================================================

/**
 * Content - camelCase view model for Angular templates.
 */
export interface ContentView {
  id: string;
  appId: string;
  title: string;
  description: string | null;
  contentType: string;
  contentFormat: string;
  blobHash: string | null;
  blobCid: string | null;
  contentSizeBytes: number | null;
  metadataJson: string | null;
  reach: string;
  validationStatus: string;
  createdBy: string | null;
  createdAt: string;
  updatedAt: string;
  contentBody: string | null;
}

/**
 * Transform wire format to ContentView.
 */
export function transformContentFromWire(wire: Content): ContentView {
  return {
    id: wire.id,
    appId: wire.app_id,
    title: wire.title,
    description: wire.description,
    contentType: wire.content_type,
    contentFormat: wire.content_format,
    blobHash: wire.blob_hash,
    blobCid: wire.blob_cid,
    contentSizeBytes: wire.content_size_bytes,
    metadataJson: wire.metadata_json,
    reach: wire.reach,
    validationStatus: wire.validation_status,
    createdBy: wire.created_by,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
    contentBody: wire.content_body,
  };
}

// =============================================================================
// Path View Types
// =============================================================================

/**
 * Path - camelCase view model for Angular templates.
 */
export interface PathView {
  id: string;
  appId: string;
  title: string;
  description: string | null;
  pathType: string;
  difficulty: string | null;
  estimatedDuration: string | null;
  thumbnailUrl: string | null;
  thumbnailAlt: string | null;
  metadataJson: string | null;
  visibility: string;
  createdBy: string | null;
  createdAt: string;
  updatedAt: string;
}

/**
 * Transform wire format to PathView.
 */
export function transformPathFromWire(wire: Path): PathView {
  return {
    id: wire.id,
    appId: wire.app_id,
    title: wire.title,
    description: wire.description,
    pathType: wire.path_type,
    difficulty: wire.difficulty,
    estimatedDuration: wire.estimated_duration,
    thumbnailUrl: wire.thumbnail_url,
    thumbnailAlt: wire.thumbnail_alt,
    metadataJson: wire.metadata_json,
    visibility: wire.visibility,
    createdBy: wire.created_by,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
  };
}

// =============================================================================
// Step View Types
// =============================================================================

/**
 * Step - camelCase view model for Angular templates.
 */
export interface StepView {
  id: string;
  appId: string;
  pathId: string;
  chapterId: string | null;
  title: string;
  description: string | null;
  stepType: string;
  resourceId: string | null;
  resourceType: string | null;
  orderIndex: number;
  estimatedDuration: string | null;
  metadataJson: string | null;
}

/**
 * Transform wire format to StepView.
 */
export function transformStepFromWire(wire: Step): StepView {
  return {
    id: wire.id,
    appId: wire.app_id,
    pathId: wire.path_id,
    chapterId: wire.chapter_id,
    title: wire.title,
    description: wire.description,
    stepType: wire.step_type,
    resourceId: wire.resource_id,
    resourceType: wire.resource_type,
    orderIndex: wire.order_index,
    estimatedDuration: wire.estimated_duration,
    metadataJson: wire.metadata_json,
  };
}

// =============================================================================
// Chapter View Types
// =============================================================================

/**
 * Chapter - camelCase view model for Angular templates.
 */
export interface ChapterView {
  id: string;
  appId: string;
  pathId: string;
  title: string;
  description: string | null;
  orderIndex: number;
  estimatedDuration: string | null;
}

/**
 * Transform wire format to ChapterView.
 */
export function transformChapterFromWire(wire: Chapter): ChapterView {
  return {
    id: wire.id,
    appId: wire.app_id,
    pathId: wire.path_id,
    title: wire.title,
    description: wire.description,
    orderIndex: wire.order_index,
    estimatedDuration: wire.estimated_duration,
  };
}

// =============================================================================
// Re-export wire types for convenience
// =============================================================================

export type {
  Relationship,
  HumanRelationship,
  ContributorPresence,
  EconomicEvent,
  ContentMastery,
  Content,
  Path,
  Step,
  Chapter,
} from '@elohim/storage-client/generated';
