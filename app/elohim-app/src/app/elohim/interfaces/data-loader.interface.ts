/**
 * IDataLoader — Abstract interface for orchestrated content loading.
 *
 * Decouples consumers from the concrete DataLoaderService which
 * coordinates multiple backends (Holochain, projection cache, storage).
 * Consumers inject the DATA_LOADER token; the default factory resolves
 * to DataLoaderService.
 *
 * Tests provide a mock IDataLoader via the same token — no concrete
 * service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class ContentViewService {
 *   private readonly loader = inject(DATA_LOADER);
 *
 *   loadContentNode(id: string): Observable<ContentWithTagsView | null> {
 *     return this.loader.loadContent(id);
 *   }
 * }
 * ```
 */

import { InjectionToken } from '@angular/core';

import { Observable } from 'rxjs';

import type { ContentFilters } from './storage-api.interface';
import type { ContentWithTagsView } from '@elohim/storage-client/generated';

/**
 * Abstract data loader — orchestrates content loading across backends.
 *
 * Unlike IStorageApi (which is a thin HTTP client), IDataLoader
 * handles caching, prefetching, fallback between projection and
 * conductor, and cross-backend coordination.
 */
export interface IDataLoader {
  /** Load a single content item by ID, using the best available backend. */
  loadContent(id: string): Observable<ContentWithTagsView | null>;

  /** Load multiple content items with optional filters. */
  loadContents(filters?: ContentFilters): Observable<ContentWithTagsView[]>;
}

/**
 * Injection token for the data loader.
 *
 * Default factory resolves to DataLoaderService which coordinates
 * projection cache, Holochain conductor, and storage API fallback.
 *
 * Override in tests:
 * ```typescript
 * { provide: DATA_LOADER, useValue: mockDataLoader }
 * ```
 */
export const DATA_LOADER = new InjectionToken<IDataLoader>('DataLoader');
