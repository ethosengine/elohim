/**
 * EprRawNodeComponent — raw-node inspector for the universal EPR address.
 *
 * Renders an EPR AS AN ATOM (its own stored fields + provenance), distinct from
 * the rich pillar experience served by `/epr/{id}`. Mounted at
 * `/epr/{id}/raw`; the doorway serves the shell for this path (landed
 * separately). This is a blank-slate inspector: NO pillar chrome (no lamad
 * nav/mastery), NO blob-resolving/computed content path — it shows the atom as
 * stored, straight off the RAW wire shape from `GET /db/content/{id}`.
 *
 * The richer typed-relation neighborhood (closure walk) is a LATER slice;
 * relatedNodeIds (a flat sibling list) is enough here.
 *
 * Thin UI wrapper: the component shapes how the atom is SHOWN. It computes no
 * domain truth — identity (CID/anchor), reach grade, and validation status are
 * all backend-owned and displayed verbatim.
 */

import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, RouterModule } from '@angular/router';

import { catchError, map, of } from 'rxjs';

import { StorageClientService } from '../../services/storage-client.service';

// ── View model ──────────────────────────────────────────────────────────────
//
// The raw wire shape returned by GET /db/content/{id} is the protocol ContentView
// (camelCase, parsed JSON, proper booleans — delivered by the Rust boundary).
// We read it as delivered; no JSON.parse, no case conversion, no reshape. The
// inspector additionally reads `relatedNodeIds` defensively: it is a flat
// sibling list the storage layer may include on a node (present/empty).
//
// NOTE(wire-drift): the hand-written `StorageContentNode` interface that types
// `StorageClientService.getContent` has drifted from the authoritative
// `content-view.schema.json` / generated `ContentView` (it carries `metadataJson`
// + `tags` and lacks `metadata`/`dhtAnchorHash`). We read the live shape here
// rather than the stale interface. Reconciling the service interface to the
// generated `ContentView` is a separate, service-layer change.

interface RawNode {
  id: string;
  contentType: string;
  contentFormat: string;
  blobHash: string | null;
  blobCid: string | null;
  contentSizeBytes: number | null;
  metadata: Record<string, unknown> | null;
  reach: string | null;
  validationStatus: string | null;
  createdBy: string | null;
  createdAt: string;
  updatedAt: string;
  dhtAnchorHash: string | null;
  relatedNodeIds: string[];
}

interface MetaEntry {
  key: string;
  value: string;
}

type LoadState =
  | { status: 'loading' }
  | { status: 'loaded'; node: RawNode }
  | { status: 'not-found' }
  | { status: 'error' };

// ── Wire → view-model projection (read-as-delivered, no reshape) ─────────────

function toRawNode(raw: Record<string, unknown>): RawNode {
  const related = raw['relatedNodeIds'];
  return {
    id: String(raw['id'] ?? ''),
    contentType: String(raw['contentType'] ?? ''),
    contentFormat: String(raw['contentFormat'] ?? ''),
    blobHash: (raw['blobHash'] as string | null) ?? null,
    blobCid: (raw['blobCid'] as string | null) ?? null,
    contentSizeBytes: (raw['contentSizeBytes'] as number | null) ?? null,
    metadata: (raw['metadata'] as Record<string, unknown> | null) ?? null,
    reach: (raw['reach'] as string | null) ?? null,
    validationStatus: (raw['validationStatus'] as string | null) ?? null,
    createdBy: (raw['createdBy'] as string | null) ?? null,
    createdAt: String(raw['createdAt'] ?? ''),
    updatedAt: String(raw['updatedAt'] ?? ''),
    dhtAnchorHash: (raw['dhtAnchorHash'] as string | null) ?? null,
    relatedNodeIds: Array.isArray(related) ? (related as string[]) : [],
  };
}

function metaEntries(metadata: Record<string, unknown> | null): MetaEntry[] {
  if (!metadata) return [];
  return Object.entries(metadata).map(([key, value]) => ({
    key,
    value: typeof value === 'string' ? value : JSON.stringify(value),
  }));
}

@Component({
  selector: 'app-epr-raw-node',
  standalone: true,
  imports: [CommonModule, RouterModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  styleUrl: './epr-raw-node.component.css',
  templateUrl: './epr-raw-node.component.html',
})
export class EprRawNodeComponent {
  private readonly route = inject(ActivatedRoute);
  private readonly storage = inject(StorageClientService);

  /**
   * The requested EPR address/slug from the route. Read from the SNAPSHOT (not
   * the paramMap Observable) deliberately: this inspector is a dead-end surface —
   * relation links target the bare `/epr/{relId}` (the rich viewer), never a
   * raw→raw `/epr/{relId}/raw` neighbor — so the component is never reused across
   * a param change. If raw-to-raw navigation is ever added, switch this to the
   * paramMap Observable + restructure the state signal.
   */
  readonly resourceId = this.route.snapshot.paramMap.get('resourceId') ?? '';

  private readonly state = toSignal(
    this.storage.getContent(this.resourceId).pipe(
      map(
        (node): LoadState =>
          node === null
            ? { status: 'not-found' }
            : { status: 'loaded', node: toRawNode(node as unknown as Record<string, unknown>) }
      ),
      catchError(() => of<LoadState>({ status: 'error' }))
    ),
    { initialValue: { status: 'loading' } as LoadState }
  );

  readonly status = computed(() => this.state().status);
  readonly node = computed<RawNode | null>(() => {
    const s = this.state();
    return s.status === 'loaded' ? s.node : null;
  });
  readonly metadata = computed<MetaEntry[]>(() => metaEntries(this.node()?.metadata ?? null));

  /** Plain href to the canonical blob path (same-origin, served by storage/doorway). */
  blobHref(hash: string): string {
    return `/blob/${hash}`;
  }
}
