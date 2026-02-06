/**
 * AUTO-GENERATED TypeScript types from Rust Diesel models
 *
 * DO NOT EDIT - regenerate with:
 *   cd holochain/elohim-storage && cargo test export_bindings
 *
 * Source: holochain/elohim-storage/src/db/models.rs (Wire types)
 *         holochain/elohim-storage/src/views.rs (View types)
 *
 * Wire types (snake_case) - match SQLite/Diesel models
 * View types (camelCase) - match HTTP API responses, ready for Angular templates
 *
 * Import as: import type { PathView } from '@elohim/storage-client/generated';
 */

// =============================================================================
// Utility Types
// =============================================================================

// JSON value type for parsed metadata fields
export * from './JsonValue';

// =============================================================================
// Wire Types (snake_case) - Database models
// =============================================================================

// App registry
export * from './App';

// Content types
export * from './Content';
export * from './ContentTag';
export * from './ContentWithTags';

// Path types
export * from './Path';
export * from './PathTag';
export * from './PathAttestation';
export * from './Chapter';
export * from './Step';

// Composite path types
export * from './ChapterWithSteps';
export * from './PathWithDetails';
export * from './PathWithSteps';

// Relationship types (content graph)
export * from './Relationship';
export * from './RelationshipWithContent';

// Human relationship types (social graph)
export * from './HumanRelationship';

// Contributor presence (stewardship)
export * from './ContributorPresence';

// Economic events (hREA/ValueFlows)
export * from './EconomicEvent';

// Content mastery (Bloom's taxonomy)
export * from './ContentMastery';

// Stewardship allocations
export * from './StewardshipAllocation';
export * from './StewardshipAllocationWithPresence';
export * from './ContentStewardship';

// Local sessions
export * from './LocalSession';

// =============================================================================
// View Types (camelCase) - HTTP API responses
// =============================================================================

// App view
export * from './AppView';

// Content views
export * from './ContentView';
export * from './ContentWithTagsView';

// Path views
export * from './PathView';
export * from './PathAttestationView';
export * from './ChapterView';
export * from './StepView';

// Composite path views
export * from './ChapterWithStepsView';
export * from './PathWithDetailsView';
export * from './PathWithStepsView';

// Relationship views (content graph)
export * from './RelationshipView';
export * from './RelationshipWithContentView';

// Human relationship views (social graph)
export * from './HumanRelationshipView';

// Contributor presence views (stewardship)
export * from './ContributorPresenceView';

// Economic event views (hREA/ValueFlows)
export * from './EconomicEventView';

// Content mastery views (Bloom's taxonomy)
export * from './ContentMasteryView';

// Stewardship allocation views
export * from './StewardshipAllocationView';
export * from './StewardshipAllocationWithPresenceView';
export * from './ContentStewardshipView';

// Local session views
export * from './LocalSessionView';

// =============================================================================
// Input View Types (camelCase) - HTTP API request bodies
// =============================================================================

// Content input
export * from './CreateContentInputView';

// Path inputs
export * from './CreatePathInputView';
export * from './CreateChapterInputView';
export * from './CreateStepInputView';

// Relationship inputs (content graph)
export * from './CreateRelationshipInputView';

// Human relationship inputs (social graph)
export * from './CreateHumanRelationshipInputView';

// Contributor presence inputs (stewardship)
export * from './CreateContributorPresenceInputView';
export * from './InitiateClaimInputView';

// Economic event inputs (hREA/ValueFlows)
export * from './CreateEconomicEventInputView';

// Stewardship allocation inputs
export * from './CreateAllocationInputView';
export * from './UpdateAllocationInputView';
