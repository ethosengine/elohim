import { Injectable, InjectionToken, inject, signal } from '@angular/core';

import { OfflineViewRepository, decodeObject, verifyBundle } from './offline-view.repository';
import { presentation, type Reading } from './view-reading';

import type { Observation, SnapshotBundle } from './view-contract';

export interface ProjectionFeed {
  open(): Promise<void>;
  discover(): Promise<string[]>;
  capture(viewId: string): Promise<Observation>;
  history(viewId: string): Promise<Observation[]>;
  snapshot(viewId: string, cid: string): Promise<SnapshotBundle>;
}
@Injectable({ providedIn: 'root' })
export class HttpProjectionFeed implements ProjectionFeed {
  async open(): Promise<void> {
    const capability = new URLSearchParams(location.hash.slice(1)).get('capability');
    if (!capability) return;
    history.replaceState(null, '', location.pathname + location.search);
    // TODO(identity): replace launcher exchange with verified signed-in identity + owner binding.
    await this.request('/session', { method: 'POST', body: JSON.stringify({ capability }) });
  }
  discover(): Promise<string[]> {
    return this.request('');
  }
  capture(viewId: string): Promise<Observation> {
    return this.request(`/${encodeURIComponent(viewId)}/capture`, { method: 'POST' });
  }
  history(viewId: string): Promise<Observation[]> {
    return this.request(`/${encodeURIComponent(viewId)}/history`);
  }
  snapshot(viewId: string, cid: string): Promise<SnapshotBundle> {
    return this.request(`/${encodeURIComponent(viewId)}/snapshots/${encodeURIComponent(cid)}`);
  }
  private async request<T>(path: string, init: RequestInit = {}): Promise<T> {
    const response = await fetch(`/api/local-views${path}`, {
      ...init,
      credentials: 'same-origin',
      cache: 'no-store',
      headers: { 'Content-Type': 'application/json' },
    });
    if (!response.ok) {
      const diagnostic = (await response.json().catch(() => ({}))) as { error?: string };
      throw new Error(
        diagnostic.error?.slice(0, 1024) ??
          `Local reader unavailable (${response.status}). Reopen the private launcher link if your session expired.`
      );
    }
    return response.json() as Promise<T>;
  }
}
export const PROJECTION_FEED = new InjectionToken<ProjectionFeed>('ProjectionFeed', {
  providedIn: 'root',
  factory: () => inject(HttpProjectionFeed),
});
@Injectable()
export class DeclaredViewService {
  private readonly feed = inject(PROJECTION_FEED);
  private readonly repository = inject(OfflineViewRepository);
  readonly readings = signal<Reading[]>([]);
  readonly selected = signal<number>(0);
  readonly busy = signal(false);
  readonly offline = signal(false);
  readonly retained = signal(false);
  readonly retainedSequence = signal(0);
  readonly error = signal('');
  private bundles: Record<string, SnapshotBundle> = {};
  private viewId = '';
  private opened = false;
  async open(viewId: string): Promise<void> {
    if (typeof window === 'undefined' || this.opened) return;
    this.opened = true;
    this.viewId = viewId;
    try {
      const copy = await this.repository.load(viewId);
      if (copy) {
        await this.install(copy.observations, copy.bundles);
        this.retained.set(true);
        this.retainedSequence.set(copy.observations.at(-1)?.sequence ?? 0);
        this.offline.set(true);
      }
    } catch (error) {
      this.error.set(String(error));
    }
    // eslint-disable-next-line no-restricted-syntax -- open returned above when window is absent; this path is browser-only.
    if (!globalThis.navigator.onLine) {
      this.offline.set(true);
      return;
    }
    await this.refresh(true);
  }
  async refresh(opening = false): Promise<void> {
    if (this.busy()) return;
    this.busy.set(true);
    try {
      if (opening) await this.feed.open();
      let captureError = '';
      try {
        await this.feed.capture(this.viewId);
      } catch (error) {
        captureError = String(error);
      }
      const observations = await this.feed.history(this.viewId);
      const bundles = { ...this.bundles };
      for (const o of observations)
        bundles[o.snapshotCid] ??= await this.feed.snapshot(this.viewId, o.snapshotCid);
      await this.install(observations, bundles);
      this.offline.set(Boolean(captureError));
      this.error.set(captureError);
    } catch (error) {
      this.error.set(String(error));
      this.offline.set(true);
    } finally {
      this.busy.set(false);
    }
  }
  private async install(
    observations: Observation[],
    bundles: Record<string, SnapshotBundle>
  ): Promise<void> {
    const readings: Reading[] = [];
    for (const observation of observations) {
      const bundle = bundles[observation.snapshotCid];
      if (bundle?.snapshotCid !== observation.snapshotCid)
        throw new Error('Observation snapshot mismatch');
      const projection = await verifyBundle(bundle);
      if (projection.declaration_cid !== observation.declarationCid)
        throw new Error('Observation declaration mismatch');
      const declaration = JSON.parse(
        new TextDecoder().decode(decodeObject(bundle.objects[projection.declaration_cid] ?? ''))
      ) as { presentation?: Parameters<typeof presentation>[0] };
      readings.push({
        observation,
        projection,
        presentation: presentation(declaration.presentation),
      });
    }
    this.bundles = bundles;
    this.readings.set(readings);
    this.selected.set(readings.length - 1);
  }
  source(reading: Reading, path: string): string {
    const cid = reading.projection.sources[path];
    if (!cid) throw new Error('Source not retained');
    const encoded = this.bundles[reading.observation.snapshotCid].objects[cid];
    if (encoded === undefined) throw new Error('Source object missing');
    const bytes = decodeObject(encoded);
    try {
      return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    } catch {
      return `Binary source (${bytes.length} bytes)\nBase64: ${this.bundles[reading.observation.snapshotCid].objects[cid]}`;
    }
  }
  async keepOffline(): Promise<void> {
    if (this.busy() || !this.readings().length) return;
    this.busy.set(true);
    let candidate: string | undefined;
    try {
      candidate = await this.repository.pinBuild(this.viewId);
      await this.repository.save({
        viewId: this.viewId,
        observations: this.readings().map(r => r.observation),
        bundles: this.bundles,
        buildCache: candidate,
      });
      this.retained.set(true);
      this.retainedSequence.set(this.readings().at(-1)?.observation.sequence ?? 0);
      this.error.set('');
    } catch (error) {
      if (candidate) await caches.delete(candidate);
      this.error.set(String(error));
    } finally {
      this.busy.set(false);
    }
  }
  async forget(): Promise<void> {
    try {
      await this.repository.forget(this.viewId);
      this.retained.set(false);
    } catch (error) {
      this.error.set(String(error));
    }
  }
}
