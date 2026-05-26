/**
 * Lamad-local storage interface + injection tokens.
 *
 * P-disposition: These tokens decouple lamad services from concrete
 * StorageApiService and StorageClientService implementations in elohim-app.
 * The concrete implementations are registered in app.config.ts; lamad services
 * inject via token, not via direct class import (which would be cross-pillar).
 *
 * Slice 2.1c: Cross-pillar import cleanup.
 */

import { InjectionToken } from '@angular/core';

import { Observable } from 'rxjs';

import type {
  RelationshipView,
  ContentMasteryView,
  ContentStewardshipView,
  StewardshipAllocationView,
} from '@elohim/storage-client/generated';

// Input types live in lamad models — bundle-relative imports are fine
import type { CreateRelationshipInput } from '../models/content-node.model';
import type {
  CreateAllocationInput,
  UpdateAllocationInput,
  FileDisputeInput,
  ResolveDisputeInput,
  BulkAllocationResult,
} from '../models/stewardship-allocation.model';

// ============================================================================
// Narrow filter/query types (subset of StorageApiService's full contract)
// These duplicate only what lamad actually uses — no imagodei types involved.
// ============================================================================

export interface LamadRelationshipFilters {
  sourceId?: string;
  targetId?: string;
  relationshipType?: string;
  minConfidence?: number;
  inferenceSource?: string;
  limit?: number;
  offset?: number;
}

export interface LamadMasteryQuery {
  humanId?: string;
  contentId?: string;
  minLevel?: string;
  needsRefresh?: boolean;
  limit?: number;
  offset?: number;
}

export interface LamadMasteryUpsertInput {
  humanId: string;
  contentId: string;
  masteryLevel?: string;
  engagementType?: string;
}

export interface LamadAllocationQuery {
  stewardPresenceId?: string;
  contentId?: string;
  governanceState?: string;
  limit?: number;
  offset?: number;
}

// ============================================================================
// ILamadStorageApi — read+write contract for what lamad actually uses
// ============================================================================

export interface ILamadStorageApi {
  // ---- Relationships ----
  getRelationships(query?: LamadRelationshipFilters): Observable<RelationshipView[]>;
  createRelationship(input: CreateRelationshipInput): Observable<RelationshipView>;

  // ---- Mastery ----
  getMasteryRecords(query?: LamadMasteryQuery): Observable<ContentMasteryView[]>;

  // ---- Mastery (write) ----
  upsertMastery(input: LamadMasteryUpsertInput): Observable<ContentMasteryView>;

  // ---- Stewardship ----
  getContentStewardship(contentId: string): Observable<ContentStewardshipView>;
  getAllocationsForSteward(stewardPresenceId: string): Observable<StewardshipAllocationView[]>;
  getStewardshipAllocations(query?: LamadAllocationQuery): Observable<StewardshipAllocationView[]>;
  createStewardshipAllocation(input: CreateAllocationInput): Observable<StewardshipAllocationView>;
  updateStewardshipAllocation(
    allocationId: string,
    input: UpdateAllocationInput
  ): Observable<StewardshipAllocationView>;
  deleteStewardshipAllocation(allocationId: string): Observable<void>;
  fileAllocationDispute(
    allocationId: string,
    input: FileDisputeInput
  ): Observable<StewardshipAllocationView>;
  resolveAllocationDispute(
    allocationId: string,
    input: ResolveDisputeInput
  ): Observable<StewardshipAllocationView>;
  bulkCreateAllocations(inputs: CreateAllocationInput[]): Observable<BulkAllocationResult>;
}

/**
 * Injection token for the lamad storage API.
 *
 * No factory — registered in lamad's app.config.ts:
 *   { provide: LAMAD_STORAGE_API, useExisting: StorageApiService }
 *
 * In tests:
 *   { provide: LAMAD_STORAGE_API, useValue: mockStorageApi }
 */
export const LAMAD_STORAGE_API = new InjectionToken<ILamadStorageApi>('LamadStorageApi');

// ============================================================================
// ILamadStorageClient — blob URL surface that lamad uses
// ============================================================================

export interface ILamadStorageClient {
  getBlobUrl(blobHash: string): string;
  getStorageBaseUrl(): string;
}

/**
 * Injection token for the lamad storage client (blob + base URL access).
 *
 * No factory — registered in lamad's app.config.ts:
 *   { provide: LAMAD_STORAGE_CLIENT, useExisting: StorageClientService }
 *
 * In tests:
 *   { provide: LAMAD_STORAGE_CLIENT, useValue: mockClient }
 */
export const LAMAD_STORAGE_CLIENT = new InjectionToken<ILamadStorageClient>('LamadStorageClient');
