import { Injectable, inject } from '@angular/core';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

import type { IIntegrityAnchor } from './integrity-anchor.interface';
import type { DoorwayInfo } from '../../imagodei/models/doorway.model';

/**
 * FederationRegistryAnchor — DHT integrity point for doorway federation lookup.
 *
 * Calls the `get_doorways_by_region` zome function on the `infrastructure`
 * zome (infrastructure DNA role), returning all doorways registered in
 * the given region.
 *
 * Used by DoorwayRegistryService as a fallback behind HTTP; this anchor
 * names the integrity role of the on-chain federation registry.
 */
@Injectable({
  providedIn: 'root',
})
export class FederationRegistryAnchor implements IIntegrityAnchor<string, DoorwayInfo[]> {
  readonly zomeName = 'infrastructure';
  readonly fnName = 'get_doorways_by_region';

  private readonly holochainClient = inject(HolochainClientService);

  async verify(region: string): Promise<DoorwayInfo[]> {
    try {
      const result = await this.holochainClient.callZome<DoorwayInfo[]>({
        zomeName: this.zomeName,
        fnName: this.fnName,
        payload: region,
        roleName: 'infrastructure',
      });

      if (result.success && result.data) {
        return result.data;
      }

      return [];
    } catch (error) {
      if (error instanceof Error) {
        console.warn(
          '[FederationRegistryAnchor] Failed to retrieve doorways for region:',
          error.message
        );
      }
      return [];
    }
  }
}
