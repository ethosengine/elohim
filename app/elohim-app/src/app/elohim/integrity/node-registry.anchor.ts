import { Injectable, inject } from '@angular/core';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

import type { IIntegrityAnchor } from './integrity-anchor.interface';

/** Raw node record as returned from the node_registry_coordinator zome (snake_case) */
export interface RawRegisteredNode {
  node_id: string;
  display_name?: string;
  node_type?: string;
  status?: string;
  last_heartbeat?: string;
  doorway_url?: string;
}

/**
 * NodeRegistryAnchor — DHT integrity point for node registry lookup.
 *
 * Calls the `get_my_nodes` zome function on the `node_registry_coordinator`
 * zome, returning all nodes registered to the current agent's identity.
 *
 * Used by RunningContextService to determine the user's compute context
 * (registered HoloPorts, self-hosted nodes, cloud nodes).
 */
@Injectable({
  providedIn: 'root',
})
export class NodeRegistryAnchor implements IIntegrityAnchor<null, RawRegisteredNode[]> {
  readonly zomeName = 'node_registry_coordinator';
  readonly fnName = 'get_my_nodes';

  private readonly holochainClient = inject(HolochainClientService);

  async verify(_input: null): Promise<RawRegisteredNode[]> {
    try {
      const result = await this.holochainClient.callZome<RawRegisteredNode[]>({
        zomeName: this.zomeName,
        fnName: this.fnName,
        payload: null,
      });

      if (!result.success || !result.data) {
        return [];
      }

      return result.data;
    } catch (error) {
      if (error instanceof Error) {
        console.warn('[NodeRegistryAnchor] Failed to retrieve registered nodes:', error.message);
      }
      return [];
    }
  }
}
