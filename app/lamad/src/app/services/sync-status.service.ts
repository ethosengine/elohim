import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable, of } from 'rxjs';
import { catchError, timeout } from 'rxjs/operators';

import { LAMAD_STORAGE_CLIENT } from '../interfaces/storage.interface';

import type { P2PStatusView } from '../../generated/p2p-status-view';

/**
 * Honest sync phases for the learner-facing progress strip.
 * - waiting: no peers reachable / nothing discovered yet — keep waiting, never "done"
 * - syncing: content is converging (show have/total)
 * - ready: the driving stream reports caughtUp
 * - unreachable: /p2p/status could not be fetched (never rendered as "done")
 */
export type SyncPhase = 'waiting' | 'syncing' | 'ready' | 'unreachable';

export interface SyncProgress {
  phase: SyncPhase;
  /** items fetched/completed so far */
  fetched: number;
  /** total items to converge (0 when not computable) */
  total: number;
  pending: number;
  failed: number;
  /** completion %, or null when indeterminate (total unknown) */
  percent: number | null;
  /** connected peers right now */
  peers: number;
  /** human-legible one-liner */
  label: string;
}

/**
 * Pure derivation of an honest {@link SyncProgress} from a P2P status snapshot.
 *
 * `null` (fetch failed) → `unreachable`. We prefer the explicit `pull` rollup
 * (it carries a real `total`); when it is absent we fall back to `replication`
 * (deriving total = completed + pending + failed). Per the wire contract, a null
 * `pull` must NEVER read as caught up — so readiness keys off whichever stream is
 * actually present, and a peerless/empty snapshot stays `waiting`, not `ready`.
 *
 * subject: p2p-status-view (Operational/Category-C, source of truth in elohim-storage)
 */
export function deriveSyncProgress(status: P2PStatusView | null): SyncProgress {
  if (status == null) {
    return {
      phase: 'unreachable',
      fetched: 0,
      total: 0,
      pending: 0,
      failed: 0,
      percent: null,
      peers: 0,
      label: 'Sync status unavailable',
    };
  }

  const peers = status.connectedPeers ?? 0;
  const pull = status.pull ?? null;

  let fetched: number;
  let total: number;
  let pending: number;
  let failed: number;
  let caughtUp: boolean;

  if (pull) {
    ({ fetched, total, pending, failed, caughtUp } = pull);
  } else {
    const r = status.replication;
    fetched = r.completed;
    pending = r.pending;
    failed = r.failed;
    total = r.completed + r.pending + r.failed;
    caughtUp = r.caughtUp;
  }

  if (caughtUp) {
    return {
      phase: 'ready',
      fetched,
      total,
      pending,
      failed,
      percent: 100,
      peers,
      label: total > 0 ? `Synced · ${total} items` : 'Synced',
    };
  }

  const percent = total > 0 ? Math.round((fetched / total) * 100) : null;
  // Nothing discovered and no peers yet ⇒ genuinely waiting, not mid-sync.
  const phase: SyncPhase = total === 0 && peers === 0 ? 'waiting' : 'syncing';
  const peerLabel = `${peers} peer${peers === 1 ? '' : 's'}`;
  const label =
    phase === 'waiting'
      ? 'Waiting for peers…'
      : total > 0
        ? `Syncing · ${fetched}/${total} items · ${peerLabel}`
        : `Syncing · ${peerLabel}`;

  return { phase, fetched, total, pending, failed, percent, peers, label };
}

/**
 * Reads the node's live P2P sync status (`GET /p2p/status`). Node-local,
 * reconstructed per request (Operational/Category-C) — a sensing surface over
 * backend-authoritative sync state; it never mutates anything.
 */
@Injectable({ providedIn: 'root' })
export class SyncStatusService {
  private readonly http = inject(HttpClient);
  private readonly storageClient = inject(LAMAD_STORAGE_CLIENT);

  /** One snapshot; `null` on any error/timeout (caller treats null as unreachable). */
  fetch(): Observable<P2PStatusView | null> {
    const baseUrl = this.storageClient.getStorageBaseUrl();
    return this.http.get<P2PStatusView>(`${baseUrl}/p2p/status`).pipe(
      timeout(5000),
      catchError(() => of(null))
    );
  }
}
