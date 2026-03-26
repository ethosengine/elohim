/**
 * IStorageApi — Abstract interface for elohim-storage DB endpoints.
 *
 * Decouples domain services from the concrete HTTP client (StorageApiService).
 * Consumers inject the STORAGE_API token; the default factory resolves
 * to StorageApiService.
 *
 * Tests provide a mock IStorageApi via the same token — no concrete
 * service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class MyService {
 *   private readonly storage = inject(STORAGE_API);
 *
 *   getRelated(id: string): Observable<RelationshipView[]> {
 *     return this.storage.getRelationships({ sourceId: id });
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { Observable } from 'rxjs';

import { StorageApiService } from '../services/storage-api.service';

import type { ContentFormat, ContentType, Reach } from '@app/generated/schema-enums';
import type {
  ContentMasteryView,
  ContentStewardshipView,
  ContentWithTagsView,
  RelationshipView,
  RelationshipWithContentView,
} from '@elohim/storage-client/generated';

/**
 * Query filters for content listing.
 *
 * Maps to the /db/content endpoint query parameters.
 */
export interface ContentFilters {
  contentType?: ContentType;
  tags?: string[];
  reach?: Reach;
  search?: string;
  limit?: number;
  offset?: number;
}

/**
 * Query filters for relationship listing.
 *
 * Maps to the /db/relationships endpoint query parameters.
 */
export interface RelationshipFilters {
  sourceId?: string;
  targetId?: string;
  relationshipType?: string;
  minConfidence?: number;
  inferenceSource?: string;
  bidirectionalOnly?: boolean;
  limit?: number;
  offset?: number;
}

/**
 * Partial update input for PATCH /db/content/{id}.
 * All fields are optional — only provided fields are applied server-side.
 * `metadata` is shallow-merged with the existing metadata object.
 */
export interface UpdateContentPatch {
  title?: string;
  description?: string | null;
  contentBody?: string;
  contentFormat?: ContentFormat;
  /** Shallow-merged server-side — only the keys you provide are overwritten. */
  metadata?: Record<string, unknown>;
  /** If provided, replaces all existing tags. */
  tags?: string[];
  reach?: Reach;
}

/**
 * Abstract storage API — read-oriented contract for elohim-storage.
 *
 * Exposes the common read surface shared by StorageApiService and
 * ProjectionAPIService. Write operations remain on the concrete service
 * until the write contract stabilizes.
 */
export interface IStorageApi {
  /** Get a single content item by ID, with tags. */
  getContent(id: string): Observable<ContentWithTagsView | null>;

  /** List content items with optional filters. */
  getContents(filters?: ContentFilters): Observable<ContentWithTagsView[]>;

  /** List content relationships with optional filters. */
  getRelationships(filters?: RelationshipFilters): Observable<RelationshipView[]>;

  /** Get relationships for a content node, with joined content. */
  getRelatedContent(id: string): Observable<RelationshipWithContentView[]>;

  /** Get mastery state for a human on specific content. */
  getMastery(humanId: string, contentId: string): Observable<ContentMasteryView | null>;

  /** Get stewardship data for a content piece. */
  getStewardship(contentId: string): Observable<ContentStewardshipView | null>;
}

/**
 * Injection token for the storage API.
 *
 * Default factory resolves to StorageApiService which provides
 * direct HTTP access to elohim-storage DB endpoints.
 *
 * Override in tests:
 * ```typescript
 * { provide: STORAGE_API, useValue: mockStorageApi }
 * ```
 */
export const STORAGE_API = new InjectionToken<IStorageApi>('StorageApi', {
  providedIn: 'root',
  factory: () => inject(StorageApiService),
});
