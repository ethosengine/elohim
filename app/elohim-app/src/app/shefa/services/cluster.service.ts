import { HttpClient } from '@angular/common/http';
import { Injectable, inject, signal } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import type { MyClusterView } from '@app/generated/my-cluster-view';

@Injectable({ providedIn: 'root' })
export class ClusterService {
  private readonly http = inject(HttpClient);

  readonly cluster = signal<MyClusterView | null>(null);
  readonly loading = signal(false);
  readonly error = signal<string | null>(null);

  async getMyCluster(): Promise<MyClusterView> {
    this.loading.set(true);
    this.error.set(null);
    try {
      const view = await firstValueFrom(this.http.get<MyClusterView>('/api/v1/cluster'));
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

  startPolling(intervalMs = 5000): () => void {
    const id = setInterval(() => {
      void this.getMyCluster();
    }, intervalMs);
    return () => clearInterval(id);
  }
}
