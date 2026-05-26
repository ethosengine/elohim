import { Injectable, Injector, inject } from '@angular/core';

import type { IIntegrityAnchor } from './integrity-anchor.interface';
import type { BlobsForContentOutput } from './blob-manager.service';

/**
 * BlobMetadataAnchor — DHT integrity point for blob metadata retrieval.
 *
 * P-migrated from @app/elohim/integrity/BlobMetadataAnchor (Slice 2.1c).
 * Moved here because its only consumers are in lamad and its only
 * cross-pillar dep (HolochainClientService) is lazily imported to
 * avoid circular dependencies.
 *
 * Calls the `get_blobs_by_content_id` zome function on the `content_store`
 * zome, returning all blob metadata anchored to a given content ID.
 */
@Injectable({
  providedIn: 'root',
})
export class BlobMetadataAnchor implements IIntegrityAnchor<string, BlobsForContentOutput | null> {
  readonly zomeName = 'content_store';
  readonly fnName = 'get_blobs_by_content_id';

  private readonly injector = inject(Injector);

  async verify(contentId: string): Promise<BlobsForContentOutput | null> {
    try {
      // Lazily import HolochainClientService to avoid circular dependency.
      // The LAMAD_HOLOCHAIN_CLIENT token cannot be used here because DI is
      // unavailable in async context; Injector.get() is the correct pattern.
      const HolochainClientService = (
        await import('@app/elohim/services/holochain-client.service')
      ).HolochainClientService;
      const holochainClient = this.injector.get(HolochainClientService);

      const result = await holochainClient.callZome<BlobsForContentOutput>({
        zomeName: this.zomeName,
        fnName: this.fnName,
        payload: { content_id: contentId },
      });

      if (!result.success || !result.data) {
        return null;
      }

      return result.data;
    } catch (error) {
      if (error instanceof Error) {
        console.warn('[BlobMetadataAnchor] Failed to retrieve blobs for content:', error.message);
      }
      return null;
    }
  }
}
