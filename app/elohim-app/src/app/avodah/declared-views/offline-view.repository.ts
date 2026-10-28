import { Injectable } from '@angular/core';

import { verifyRawSha256 } from '@app/elohim/utils/raw-cid-verify';

import type { Observation, Projection, SnapshotBundle } from './view-contract';

export interface OfflineCopy {
  viewId: string;
  observations: Observation[];
  bundles: Record<string, SnapshotBundle>;
  buildCache: string;
}
export interface BuildManifest {
  id: string;
  assets: { url: string; address: string }[];
}
export function openViewDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open('declared-local-views', 1);
    request.onupgradeneeded = () =>
      request.result.createObjectStore('copies', { keyPath: 'viewId' });
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}
export function decodeObject(encoded: string): Uint8Array {
  return Uint8Array.from(atob(encoded), c => c.codePointAt(0) ?? 0);
}
export async function verifyBundle(bundle: SnapshotBundle): Promise<Projection> {
  if (!(await verifyRawSha256(new TextEncoder().encode(bundle.envelope), bundle.snapshotCid))) {
    throw new Error('Corrupt snapshot envelope');
  }
  const snapshot = JSON.parse(bundle.envelope) as { version: number; projection: Projection };
  if (snapshot.version !== 1 || ![1, 2].includes(snapshot.projection.version))
    throw new Error('Unsupported snapshot');
  const p = snapshot.projection;
  if ((p.version === 2) !== Boolean(p.composition)) throw new Error('Invalid composition envelope');
  if (p.sources[p.declaration] !== p.declaration_cid)
    throw new Error('Declaration version mismatch');
  for (const [path, cid] of Object.entries(p.sources)) {
    if (!cid) throw new Error(`Missing source address: ${path}`);
    const object = bundle.objects[cid];
    if (
      object === undefined ||
      p.input_versions[path] !== cid ||
      !(await verifyRawSha256(decodeObject(object), cid))
    ) {
      throw new Error(`Corrupt or missing captured source: ${path}`);
    }
  }
  const ids = new Set<string>();
  for (const subject of p.subjects) {
    if (ids.has(subject.identity) || p.sources[subject.source] !== subject.version)
      throw new Error('Invalid subject source');
    ids.add(subject.identity);
  }
  return p;
}
@Injectable({ providedIn: 'root' })
export class OfflineViewRepository {
  private async read(viewId: string): Promise<OfflineCopy | undefined> {
    const db = await openViewDatabase();
    return new Promise<OfflineCopy | undefined>((resolve, reject) => {
      const req = db.transaction('copies').objectStore('copies').get(viewId);
      req.onsuccess = () => resolve(req.result as OfflineCopy | undefined);
      req.onerror = () => reject(req.error);
    }).finally(() => db.close());
  }
  async load(viewId: string): Promise<OfflineCopy | undefined> {
    const copy = await this.read(viewId);
    if (copy) await this.verify(copy);
    return copy;
  }
  async verify(copy: OfflineCopy): Promise<void> {
    for (const [i, observation] of copy.observations.entries()) {
      const bundle = copy.bundles[observation.snapshotCid];
      if (bundle?.snapshotCid !== observation.snapshotCid || observation.sequence !== i + 1)
        throw new Error('Incomplete observation history');
      const projection = await verifyBundle(bundle);
      if (projection.declaration_cid !== observation.declarationCid)
        throw new Error('Observation declaration mismatch');
    }
  }
  async save(copy: OfflineCopy): Promise<void> {
    await this.verify(copy);
    const previous = await this.read(copy.viewId);
    const db = await openViewDatabase();
    try {
      await new Promise<void>((resolve, reject) => {
        // One commit replaces the entire copy. Abort/quota leaves the old generation intact.
        const tx = db.transaction('copies', 'readwrite');
        tx.objectStore('copies').put(copy);
        tx.oncomplete = () => resolve();
        tx.onabort = () => reject(tx.error ?? new Error('Offline save interrupted'));
        tx.onerror = () => reject(tx.error);
      });
    } finally {
      db.close();
    }
    if (previous && previous.buildCache !== copy.buildCache)
      await caches.delete(previous.buildCache).catch(() => false);
  }
  async forget(viewId: string): Promise<void> {
    const old = await this.read(viewId);
    const db = await openViewDatabase();
    try {
      await new Promise<void>((resolve, reject) => {
        const tx = db.transaction('copies', 'readwrite');
        tx.objectStore('copies').delete(viewId);
        tx.oncomplete = () => resolve();
        tx.onabort = () => reject(tx.error);
      });
    } finally {
      db.close();
    }
    if (old) await caches.delete(old.buildCache);
  }
  async pinBuild(viewId: string): Promise<string> {
    const response = await fetch('/avodah/views/build.json', { cache: 'no-store' });
    if (!response.ok) throw new Error('A pinned production build is required for offline use');
    const manifest = (await response.json()) as BuildManifest;
    if (!manifest.assets.length || manifest.assets.length > 10000)
      throw new Error('Invalid build manifest');
    // eslint-disable-next-line no-restricted-syntax -- explicit Keep offline browser gesture; never called during SSR.
    const registration = await globalThis.navigator.serviceWorker.register(
      '/avodah/views/offline-worker.js',
      {
        scope: '/avodah/views/',
      }
    );
    if (!registration.active)
      await new Promise<void>((resolve, reject) => {
        const worker = registration.installing ?? registration.waiting;
        if (!worker) {
          reject(new Error('Offline worker unavailable'));
          return;
        }
        worker.addEventListener('statechange', () => {
          if (worker.state === 'activated') resolve();
          if (worker.state === 'redundant') reject(new Error('Offline worker installation failed'));
        });
      });
    const name = `declared-view-${viewId}-${manifest.id}-${crypto.randomUUID()}`;
    try {
      const cache = await caches.open(name);
      for (const asset of manifest.assets) {
        const url = new URL(asset.url, location.origin);
        if (url.origin !== location.origin || url.pathname.startsWith('/api/'))
          throw new Error('Invalid build asset');
        const fetched = await fetch(url, {
          cache: 'no-store',
          headers: { 'X-Local-View-Pin': '1' },
        });
        if (
          !fetched.ok ||
          !(await verifyRawSha256(await fetched.clone().arrayBuffer(), asset.address))
        )
          throw new Error('Build asset verification failed');
        await cache.put(url, fetched);
        const stored = await cache.match(url);
        if (!stored || !(await verifyRawSha256(await stored.arrayBuffer(), asset.address)))
          throw new Error('Build persistence failed');
      }
      return name;
    } catch (error) {
      await caches.delete(name);
      throw error;
    }
  }
}
