import { HttpClient } from '@angular/common/http';
import { Injectable, inject, signal } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import type { PeerTopologyView } from '@app/generated/peer-topology-view';

@Injectable({ providedIn: 'root' })
export class PeerTopologyService {
  private readonly http = inject(HttpClient);

  readonly topology = signal<PeerTopologyView | null>(null);
  readonly loading = signal(false);

  async getMyPeerTopology(): Promise<PeerTopologyView> {
    this.loading.set(true);
    try {
      const view = await firstValueFrom(this.http.get<PeerTopologyView>('/api/v1/peer-topology'));
      this.topology.set(view);
      return view;
    } finally {
      this.loading.set(false);
    }
  }

  startPolling(intervalMs = 5000): () => void {
    const id = setInterval(() => {
      void this.getMyPeerTopology();
    }, intervalMs);
    return () => clearInterval(id);
  }
}
