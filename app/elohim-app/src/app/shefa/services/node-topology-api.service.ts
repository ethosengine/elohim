/**
 * NodeTopologyApiService — Thin HTTP client for stewarded node topology.
 *
 * Reads from elohim-storage projection via `/db/nodes` — fast and queryable.
 * Writes should go through the Holochain conductor (zome calls) so records
 * carry a dht_anchor_hash. This service is read-only by design.
 */

import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';

import { Observable } from 'rxjs';

import {
  StewardedNode,
  NodeAvailability,
  calculateNodeAvailability,
} from '../models/shefa-dashboard.model';

@Injectable({ providedIn: 'root' })
export class NodeTopologyApiService {
  private readonly http = inject(HttpClient);

  /** List all stewarded nodes, optionally filtered by claim status. */
  listNodes(claimStatus?: StewardedNode['claimStatus']): Observable<StewardedNode[]> {
    const params: Record<string, string> = claimStatus ? { claimStatus } : {};
    return this.http.get<StewardedNode[]>('/db/nodes', { params });
  }

  /** Get a single node with its stewardship relationships. */
  getNode(nodeId: string): Observable<StewardedNode> {
    return this.http.get<StewardedNode>(`/db/nodes/${nodeId}`);
  }

  /** Calculate aggregate availability from a set of stewarded nodes. */
  getAvailability(nodes: StewardedNode[]): NodeAvailability {
    return calculateNodeAvailability(nodes);
  }
}
