/**
 * AUTO-GENERATED TypeScript types from Rust Diesel models
 *
 * DO NOT EDIT - regenerate with:
 *   cd holochain/elohim-storage && ./scripts/generate-types.sh
 *
 * Source: holochain/elohim-storage/src/db/models.rs
 *
 * These types use snake_case to match the wire format from the Rust backend.
 * Import as: import type { Content } from '@elohim/storage-client/generated';
 *
 * For Angular view models (camelCase), see:
 *   elohim-app/src/app/elohim/adapters/storage-types.adapter.ts
 */

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
