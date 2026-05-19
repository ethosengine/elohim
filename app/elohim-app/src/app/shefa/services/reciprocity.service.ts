import { HttpClient } from '@angular/common/http';
import { Injectable, inject, signal } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import { AgentService } from '@app/elohim/services/agent.service';

import {
  VIEWER_RECIPROCITY_QUERY,
  type GraphQLEnvelope,
  type ViewerQueryVariables,
  type ViewerReciprocityResponse,
} from '@elohim/storage-client/graphql';

import { environment } from '../../../environments/environment';

import { adaptReciprocityResponse } from './topology-graphql.adapter';

import type { ReciprocityView } from '@app/generated/reciprocity-view';

@Injectable({ providedIn: 'root' })
export class ReciprocityService {
  private readonly http = inject(HttpClient);
  private readonly agentService = inject(AgentService);

  readonly reciprocity = signal<ReciprocityView | null>(null);
  readonly loading = signal(false);
  readonly error = signal<string | null>(null);

  /**
   * Fetch the viewer's reciprocity ledger. Stewardship-aligned name (no
   * `my*` prefix) — see the L6 viewer.* symmetry pass in
   * `genesis/docs/plans/2026-05-19-viewer-symmetry-reciprocity-qahal-substrate.md`.
   *
   * Routes through `/api/v1/graphql` (Viewer.reciprocity resolver) when
   * `environment.features.useGraphqlTopology` is true; otherwise falls back
   * to the legacy REST handler at `/api/v1/reciprocity`.
   */
  async getReciprocity(): Promise<ReciprocityView> {
    this.loading.set(true);
    this.error.set(null);
    try {
      const view = environment.features?.useGraphqlTopology
        ? await this.fetchViaGraphql()
        : await firstValueFrom(this.http.get<ReciprocityView>('/api/v1/reciprocity'));
      this.reciprocity.set(view);
      return view;
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'unknown';
      this.error.set(msg);
      throw e;
    } finally {
      this.loading.set(false);
    }
  }

  private async fetchViaGraphql(): Promise<ReciprocityView> {
    const agentCid = this.agentService.getCurrentAgentId();
    const variables: ViewerQueryVariables = { agentCid };
    const envelope = await firstValueFrom(
      this.http.post<GraphQLEnvelope<ViewerReciprocityResponse>>('/api/v1/graphql', {
        query: VIEWER_RECIPROCITY_QUERY,
        variables,
      }),
    );
    if (envelope.errors?.length) {
      throw new Error(`GraphQL error: ${envelope.errors.map(e => e.message).join('; ')}`);
    }
    if (!envelope.data) {
      throw new Error('GraphQL response contained no data');
    }
    return adaptReciprocityResponse(envelope.data);
  }

  startPolling(intervalMs = 30_000): () => void {
    const id = setInterval(() => {
      void this.getReciprocity();
    }, intervalMs);
    return () => clearInterval(id);
  }
}
