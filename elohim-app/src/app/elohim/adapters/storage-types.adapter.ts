/**
 * Angular-specific adapters for generated view types.
 *
 * The view types are generated from Rust with camelCase serialization - see:
 *   holochain/sdk/storage-client-ts/src/generated/*View.ts
 *
 * This file provides:
 * 1. Re-exports of generated View types
 * 2. Extended types with derived/computed fields
 * 3. Simple helpers to add derived fields
 *
 * WHY THIS EXISTS:
 * - Generated View types are camelCase (from Rust #[serde(rename_all = "camelCase")])
 * - Some Angular components need computed/derived fields not stored in DB
 * - This adapter adds those derived fields without duplicating type definitions
 */

// =============================================================================
// Re-export generated View types
// =============================================================================

export type {
  RelationshipView,
  HumanRelationshipView as HumanRelationshipViewBase,
  ContributorPresenceView as ContributorPresenceViewBase,
  EconomicEventView,
  ContentMasteryView,
  ContentView,
  PathView,
  StepView,
  ChapterView,
  ContentWithTagsView,
  PathWithDetailsView,
  PathWithStepsView,
  ChapterWithStepsView,
  RelationshipWithContentView,
  StewardshipAllocationView,
  StewardshipAllocationWithPresenceView,
  ContentStewardshipView,
  LocalSessionView,
  PathAttestationView,
} from '@elohim/storage-client/generated';

// Import base types for extending
import type {
  HumanRelationshipView as HumanRelationshipViewBase,
  ContributorPresenceView as ContributorPresenceViewBase,
} from '@elohim/storage-client/generated';

// @coverage: 0.0% (2026-02-05)

// =============================================================================
// Extended View Types with Derived Fields
// =============================================================================

/**
 * HumanRelationshipView with computed `isFullyConsented` field.
 * The base type comes from Rust, this adds Angular-specific derived state.
 */
export interface HumanRelationshipView extends HumanRelationshipViewBase {
  /** Derived: both parties have consented */
  isFullyConsented: boolean;
}

/**
 * ContributorPresenceView with computed `establishingContentIds` field.
 * The base type comes from Rust, this adds parsed JSON array.
 */
export interface ContributorPresenceView extends ContributorPresenceViewBase {
  /** Derived: parsed establishing content IDs from JSON */
  establishingContentIds: string[];
}

// =============================================================================
// Lightweight Adapters for Derived Fields
// =============================================================================

/**
 * Add derived `isFullyConsented` field to HumanRelationshipView.
 * Call this when you need the computed field for UI logic.
 */
export function withFullyConsentedFlag(view: HumanRelationshipViewBase): HumanRelationshipView {
  return {
    ...view,
    isFullyConsented: view.consentGivenByA && view.consentGivenByB,
  };
}

/**
 * Add derived `establishingContentIds` field to ContributorPresenceView.
 * Extracts the typed array from the already-parsed JSON value.
 */
export function withEstablishingContentIds(
  view: ContributorPresenceViewBase
): ContributorPresenceView {
  const establishingContentIds = Array.isArray(view.establishingContentIds)
    ? (view.establishingContentIds as string[])
    : [];
  return {
    ...view,
    establishingContentIds,
  };
}

// =============================================================================
// Batch Helpers
// =============================================================================

/**
 * Add derived fields to an array of HumanRelationshipViews.
 */
export function withFullyConsentedFlags(
  views: HumanRelationshipViewBase[]
): HumanRelationshipView[] {
  return views.map(withFullyConsentedFlag);
}

/**
 * Add derived fields to an array of ContributorPresenceViews.
 */
export function withEstablishingContentIdsArray(
  views: ContributorPresenceViewBase[]
): ContributorPresenceView[] {
  return views.map(withEstablishingContentIds);
}
