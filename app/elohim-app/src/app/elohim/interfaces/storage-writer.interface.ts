/**
 * IStorageWriter — Abstract interface for elohim-storage write operations.
 *
 * Decouples domain services from the concrete HTTP client (StorageApiService).
 * Consumers inject the STORAGE_WRITER token; the default factory resolves
 * to StorageApiService.
 *
 * Tests provide a mock IStorageWriter via the same token — no concrete
 * service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class StewardshipService {
 *   private readonly writer = inject(STORAGE_WRITER);
 *
 *   allocate(input: CreateAllocationInput): Observable<StewardshipAllocationView> {
 *     return this.writer.createStewardshipAllocation(input);
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { Observable } from 'rxjs';

import {
  HumanRelationshipView,
  ContributorPresenceView,
} from '@app/elohim/adapters/storage-types.adapter';
import { CreateHumanRelationshipInput } from '@app/imagodei/models/human-relationship.model';
import { CreateRelationshipInput } from '@app/lamad/models/content-node.model';
import {
  CreateAllocationInput,
  UpdateAllocationInput,
  FileDisputeInput,
  ResolveDisputeInput,
  BulkAllocationResult,
} from '@app/lamad/models/stewardship-allocation.model';

import { StorageApiService } from '../services/storage-api.service';

import type {
  RelationshipView,
  EconomicEventView,
  ContentMasteryView,
  StewardshipAllocationView,
} from '@elohim/storage-client/generated';
// TODO: MasteryLevel and EngagementType need reconciliation between
// mastery.service.ts values (aware/understanding/applying) and schema
// values (seen/remember/understand). Discovered during schema IoC enforcement.
// Until reconciled, these stay as string at this boundary.

// ============================================================================
// Input Types (moved from storage-api.service.ts)
// ============================================================================

/**
 * Input for creating a contributor presence.
 */
export interface CreatePresenceInput {
  displayName: string;
  establishingContentIds: string[];
  externalIdentifiers?: { platform: string; identifier: string }[];
}

/**
 * Input for creating/updating mastery.
 */
export interface CreateMasteryInput {
  humanId: string;
  contentId: string;
  masteryLevel?: string; // TODO: reconcile with schema MasteryLevel
  engagementType?: string; // TODO: reconcile with schema EngagementType
}

/**
 * Input for creating an economic event.
 */
export interface CreateEventInput {
  action: string;
  provider: string;
  receiver: string;
  resourceConformsTo?: string;
  resourceQuantity?: { value: number; unit: string };
  lamadEventType?: string;
  contentId?: string;
  pathId?: string;
  contributorPresenceId?: string;
  note?: string;
  metadata?: Record<string, unknown>;
}

// ============================================================================
// Interface
// ============================================================================

/**
 * Abstract storage writer — write-side contract for elohim-storage.
 *
 * Covers all mutation operations: content relationships, human relationships,
 * contributor presences, economic events, mastery, and stewardship allocations.
 */
export interface IStorageWriter {
  // --------------------------------------------------------------------------
  // Content Relationships
  // --------------------------------------------------------------------------

  /** Create a new content relationship. */
  createRelationship(input: CreateRelationshipInput): Observable<RelationshipView>;

  // --------------------------------------------------------------------------
  // Human Relationships
  // --------------------------------------------------------------------------

  /** Create a new human relationship. */
  createHumanRelationship(input: CreateHumanRelationshipInput): Observable<HumanRelationshipView>;

  /** Update consent on a human relationship. */
  updateHumanRelationshipConsent(relationshipId: string, consent: boolean): Observable<void>;

  /** Update custody setting on a human relationship. */
  updateHumanRelationshipCustody(
    relationshipId: string,
    enabled: boolean,
    autoCustody?: boolean
  ): Observable<void>;

  // --------------------------------------------------------------------------
  // Contributor Presences
  // --------------------------------------------------------------------------

  /** Create a new contributor presence. */
  createContributorPresence(input: CreatePresenceInput): Observable<ContributorPresenceView>;

  /** Initiate stewardship of a contributor presence. */
  initiatePresenceStewardship(presenceId: string, stewardId: string): Observable<void>;

  /** Initiate a claim on a contributor presence. */
  initiatePresenceClaim(presenceId: string, agentId: string): Observable<void>;

  /** Verify a presence claim with evidence. */
  verifyPresenceClaim(
    presenceId: string,
    evidence: { method: string; data: Record<string, unknown> }
  ): Observable<void>;

  // --------------------------------------------------------------------------
  // Economic Events
  // --------------------------------------------------------------------------

  /** Create a new economic event. */
  createEconomicEvent(input: CreateEventInput): Observable<EconomicEventView>;

  // --------------------------------------------------------------------------
  // Content Mastery
  // --------------------------------------------------------------------------

  /** Create or update a mastery record. */
  upsertMastery(input: CreateMasteryInput): Observable<ContentMasteryView>;

  // --------------------------------------------------------------------------
  // Stewardship Allocations
  // --------------------------------------------------------------------------

  /** Create a new stewardship allocation. */
  createStewardshipAllocation(input: CreateAllocationInput): Observable<StewardshipAllocationView>;

  /** Update a stewardship allocation. */
  updateStewardshipAllocation(
    allocationId: string,
    input: UpdateAllocationInput
  ): Observable<StewardshipAllocationView>;

  /** Delete a stewardship allocation. */
  deleteStewardshipAllocation(allocationId: string): Observable<void>;

  /** File a dispute on a stewardship allocation. */
  fileAllocationDispute(
    allocationId: string,
    input: FileDisputeInput
  ): Observable<StewardshipAllocationView>;

  /** Resolve a dispute on a stewardship allocation (Elohim ratification). */
  resolveAllocationDispute(
    allocationId: string,
    input: ResolveDisputeInput
  ): Observable<StewardshipAllocationView>;

  /** Bulk create stewardship allocations. */
  bulkCreateAllocations(inputs: CreateAllocationInput[]): Observable<BulkAllocationResult>;
}

// ============================================================================
// Injection Token
// ============================================================================

/**
 * Injection token for the storage writer.
 *
 * Default factory resolves to StorageApiService which provides
 * direct HTTP access to elohim-storage mutation endpoints.
 *
 * Override in tests:
 * ```typescript
 * { provide: STORAGE_WRITER, useValue: mockStorageWriter }
 * ```
 */
export const STORAGE_WRITER = new InjectionToken<IStorageWriter>('StorageWriter', {
  providedIn: 'root',
  factory: () => inject(StorageApiService),
});
