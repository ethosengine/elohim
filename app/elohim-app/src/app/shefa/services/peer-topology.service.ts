import { HttpClient } from '@angular/common/http';
import { Injectable, inject, signal } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import { AgentService } from '@app/elohim/services/agent.service';

import { VIEWER_PEERS_QUERY } from '@elohim/storage-client/graphql';

import { environment } from '../../../environments/environment';

import { adaptPeersResponse } from './topology-graphql.adapter';

import type { PeerTopologyView } from '@app/generated/peer-topology-view';
import type {
  GraphQLEnvelope,
  ViewerPeersResponse,
  ViewerQueryVariables,
} from '@elohim/storage-client/graphql';

@Injectable({ providedIn: 'root' })
export class PeerTopologyService {
  private readonly http = inject(HttpClient);
  private readonly agentService = inject(AgentService);

  readonly topology = signal<PeerTopologyView | null>(null);
  readonly loading = signal(false);

  async getMyPeerTopology(): Promise<PeerTopologyView> {
    this.loading.set(true);
    try {
      const view = environment.features?.useGraphqlTopology
        ? await this.fetchViaGraphql()
        : await firstValueFrom(this.http.get<PeerTopologyView>('/api/v1/peer-topology'));
      this.topology.set(view);
      return view;
    } finally {
      this.loading.set(false);
    }
  }

  private async fetchViaGraphql(): Promise<PeerTopologyView> {
    const agentCid = this.agentService.getCurrentAgentId();
    const variables: ViewerQueryVariables = { agentCid };
    const envelope = await firstValueFrom(
      this.http.post<GraphQLEnvelope<ViewerPeersResponse>>('/api/v1/graphql', {
        query: VIEWER_PEERS_QUERY,
        variables,
      })
    );
    if (envelope.errors?.length) {
      throw new Error(`GraphQL error: ${envelope.errors.map(e => e.message).join('; ')}`);
    }
    if (!envelope.data) {
      throw new Error('GraphQL response contained no data');
    }
    return adaptPeersResponse(envelope.data);
  }

  startPolling(intervalMs = 5000): () => void {
    const id = setInterval(() => {
      void this.getMyPeerTopology();
    }, intervalMs);
    return () => clearInterval(id);
  }
}
