import { HttpClient } from '@angular/common/http';
import { Injectable, inject, signal } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import { AgentService } from '@app/elohim/services/agent.service';

import { VIEWER_HUB_QUERY } from '@elohim/storage-client/graphql';

import { environment } from '../../../environments/environment';

import { adaptHubResponse } from './topology-graphql.adapter';

import type { MyClusterView } from '@app/generated/my-cluster-view';
import type {
  GraphQLEnvelope,
  ViewerHubResponse,
  ViewerQueryVariables,
} from '@elohim/storage-client/graphql';

@Injectable({ providedIn: 'root' })
export class ClusterService {
  private readonly http = inject(HttpClient);
  private readonly agentService = inject(AgentService);

  readonly cluster = signal<MyClusterView | null>(null);
  readonly loading = signal(false);
  readonly error = signal<string | null>(null);

  async getMyCluster(): Promise<MyClusterView> {
    this.loading.set(true);
    this.error.set(null);
    try {
      const view = environment.features?.useGraphqlTopology
        ? await this.fetchViaGraphql()
        : await firstValueFrom(this.http.get<MyClusterView>('/api/v1/cluster'));
      this.cluster.set(view);
      return view;
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'unknown';
      this.error.set(msg);
      throw e;
    } finally {
      this.loading.set(false);
    }
  }

  private async fetchViaGraphql(): Promise<MyClusterView> {
    const agentCid = this.agentService.getCurrentAgentId();
    const variables: ViewerQueryVariables = { agentCid };
    const envelope = await firstValueFrom(
      this.http.post<GraphQLEnvelope<ViewerHubResponse>>('/api/v1/graphql', {
        query: VIEWER_HUB_QUERY,
        variables,
      })
    );
    if (envelope.errors?.length) {
      throw new Error(`GraphQL error: ${envelope.errors.map(e => e.message).join('; ')}`);
    }
    if (!envelope.data) {
      throw new Error('GraphQL response contained no data');
    }
    return adaptHubResponse(envelope.data);
  }

  startPolling(intervalMs = 5000): () => void {
    const id = setInterval(() => {
      void this.getMyCluster();
    }, intervalMs);
    return () => clearInterval(id);
  }
}
