/**
 * Content Doc Sync Service
 *
 * Thin reactive wrapper over the SDK's {@link AutomergeSync} helper. It polls a
 * single content doc on the Automerge content-sync plane and surfaces its
 * current flat fields as an Angular signal so components can bind to a doc that
 * converges peer-to-peer over libp2p (projected into the DocStore by the
 * elohim-storage producer).
 *
 * SERVICE GRAVITY: this is a UX-surface / reactive-projection service. It binds
 * already-converged backend truth to UI state — it does NOT compute domain
 * truth. CRDT convergence, the projection producer, and the "elohim" sync
 * partition all live in Rust (elohim-storage). Nothing here decides what is
 * true; it only decides what to show.
 *
 * DATA IDENTITY: the sync-plane doc id is the backend's deterministic mapping
 * `node:{contentId}` (mirrors the Rust producer's `content_doc_id`). We do not
 * mint identity here — we derive the same operational key the backend writes.
 *
 * LIBRARY HOME: this service lives in `@elohim/service` (not elohim-app) so the
 * resilience snapshot card — which renders inside the lamad bundle — and the
 * elohim-app harness can both consume it. The connection seam is honoured via
 * the injectable {@link CONTENT_SYNC_STORAGE_BASE_URL} resolver: the library
 * never hardcodes an endpoint or branches on doorway-vs-Tauri; the host app
 * provides the base URL (elohim-app wires it to its connection strategy; the
 * default is origin-relative, which the dev proxy / doorway carry for `/sync`).
 */

import {
  DestroyRef,
  Injectable,
  InjectionToken,
  Signal,
  WritableSignal,
  inject,
  signal,
} from '@angular/core';

import { AutomergeSync, StorageClient } from '@elohim/storage-client';

/**
 * The sync-partition namespace. MUST equal the `h_app_id` the elohim-storage
 * producer writes under and the sync timer lists (`initiate_sync_round`,
 * p2p/mod.rs). A doc written under any other namespace sits inert forever.
 */
export const CONTENT_SYNC_NAMESPACE = 'elohim';

/** Default poll cadence for a watched doc. */
export const DEFAULT_SYNC_POLL_MS = 5000;

/**
 * Resolves the storage base URL the SDK client targets. Injectable so the host
 * app owns the connection seam (doorway-vs-Tauri-vs-direct) while the library
 * stays endpoint-agnostic. The default is origin-relative (`''`): in
 * browser/doorway/dev-proxy contexts that resolves `/sync/v1/...` against the
 * current origin, which the doorway carries. Hosts with a non-origin storage
 * endpoint (e.g. Tauri's local sidecar) override this from their connection
 * strategy.
 */
export type StorageBaseUrlResolver = () => string;

export const CONTENT_SYNC_STORAGE_BASE_URL = new InjectionToken<StorageBaseUrlResolver>(
  'ContentSyncStorageBaseUrl',
  {
    providedIn: 'root',
    factory: () => () => '',
  }
);

/**
 * Flat content-doc fields as projected by the elohim-storage producer
 * (one Automerge doc per content id). All optional: a doc converges
 * field-by-field over CRDT sync, so a field may be absent mid-convergence.
 * `metadata` is the producer's JSON string verbatim — surfaced as-is (the UI
 * layer never re-parses wire/sync payloads).
 */
export interface ContentDocFields {
  id?: string;
  title?: string;
  contentType?: string;
  contentFormat?: string;
  reach?: string;
  body?: string;
  metadata?: string;
  updatedAt?: string;
}

/** Options for a watched content doc. */
export interface WatchContentOptions {
  /** Poll cadence in ms (default {@link DEFAULT_SYNC_POLL_MS}). */
  pollMs?: number;
}

/**
 * Factory for the SDK sync helper, injectable so tests can substitute a fake
 * without touching automerge or the network.
 */
export type AutomergeSyncFactory = (client: StorageClient) => AutomergeSync;

export const AUTOMERGE_SYNC_FACTORY = new InjectionToken<AutomergeSyncFactory>(
  'AutomergeSyncFactory',
  {
    providedIn: 'root',
    factory: () => (client: StorageClient) => new AutomergeSync(client),
  }
);

/**
 * The Automerge doc handle is opaque to the UI layer — read like a plain
 * record. (The app resolves `@automerge/automerge` to its base64 browser entry
 * via a tsconfig alias, which carries no types, so we don't lean on the SDK's
 * `Doc<T>` typing here.)
 */
type SyncDocHandle = Record<string, unknown>;

/**
 * The narrow slice of {@link AutomergeSync} this service uses, re-typed locally
 * so the alias-degraded automerge types never leak into call sites. The single
 * cast boundary lives in `getHelper`.
 */
interface DocSyncHelper {
  load(docId: string): Promise<SyncDocHandle>;
  sync(
    docId: string,
    doc: SyncDocHandle
  ): Promise<{ doc: SyncDocHandle; changed: boolean; heads: string[] }>;
}

/** Derive the sync-plane doc id for a content id (mirror of the Rust producer). */
export function contentDocId(contentId: string): string {
  return `node:${contentId}`;
}

function readDocFields(doc: SyncDocHandle): ContentDocFields {
  const d = doc;
  const str = (v: unknown): string | undefined => (typeof v === 'string' ? v : undefined);
  return {
    id: str(d['id']),
    title: str(d['title']),
    contentType: str(d['contentType']),
    contentFormat: str(d['contentFormat']),
    reach: str(d['reach']),
    body: str(d['body']),
    metadata: str(d['metadata']),
    updatedAt: str(d['updatedAt']),
  };
}

@Injectable({ providedIn: 'root' })
export class ContentDocSyncService {
  private readonly resolveBaseUrl = inject(CONTENT_SYNC_STORAGE_BASE_URL);
  private readonly makeSync = inject(AUTOMERGE_SYNC_FACTORY);
  private readonly destroyRef = inject(DestroyRef);

  private helper?: DocSyncHelper;
  private readonly docs = new Map<string, SyncDocHandle>();
  private readonly fields = new Map<string, WritableSignal<ContentDocFields | null>>();
  private readonly timers = new Map<string, ReturnType<typeof setInterval>>();

  constructor() {
    // Safety net: tear down any poll loops when the (root) service is destroyed.
    this.destroyRef.onDestroy(() => this.stopAll());
  }

  /**
   * Begin watching a content doc and return its reactive fields signal.
   * Idempotent per content id: repeat calls return the same signal and do not
   * start a second poll loop. Kicks an immediate refresh, then polls.
   */
  watchContent(
    contentId: string,
    options: WatchContentOptions = {}
  ): Signal<ContentDocFields | null> {
    const docId = contentDocId(contentId);
    const sig = this.fieldSignal(docId);

    if (!this.timers.has(docId)) {
      const pollMs = options.pollMs ?? DEFAULT_SYNC_POLL_MS;
      void this.refreshContent(contentId);
      this.timers.set(
        docId,
        setInterval(() => void this.refreshContent(contentId), pollMs)
      );
    }

    return sig.asReadonly();
  }

  /**
   * Run one sync round for a content doc and push its fields into the signal.
   * Returns the fields (also useful for tests / one-shot reads).
   */
  async refreshContent(contentId: string): Promise<ContentDocFields | null> {
    const docId = contentDocId(contentId);
    const sig = this.fieldSignal(docId);
    try {
      const helper = this.getHelper();
      let doc = this.docs.get(docId);
      doc ??= await helper.load(docId);
      const result = await helper.sync(docId, doc);
      this.docs.set(docId, result.doc);
      const fields = readDocFields(result.doc);
      sig.set(fields);
      return fields;
    } catch (error) {
      // A failed round is a UI/transport concern, not a truth concern: keep the
      // last-known fields and let the next poll retry.
      console.warn('[ContentDocSyncService] sync round failed for', docId, error);
      return sig();
    }
  }

  /** Read the current fields signal for a content id (null until first round). */
  contentDoc(contentId: string): Signal<ContentDocFields | null> {
    return this.fieldSignal(contentDocId(contentId)).asReadonly();
  }

  /** Stop polling a single content doc (its last-known signal value is kept). */
  stopWatchingContent(contentId: string): void {
    const docId = contentDocId(contentId);
    const timer = this.timers.get(docId);
    if (timer !== undefined) {
      clearInterval(timer);
      this.timers.delete(docId);
    }
  }

  /** Stop all poll loops. */
  stopAll(): void {
    for (const timer of this.timers.values()) {
      clearInterval(timer);
    }
    this.timers.clear();
  }

  private fieldSignal(docId: string): WritableSignal<ContentDocFields | null> {
    let sig = this.fields.get(docId);
    if (sig === undefined) {
      sig = signal<ContentDocFields | null>(null);
      this.fields.set(docId, sig);
    }
    return sig;
  }

  private getHelper(): DocSyncHelper {
    if (this.helper === undefined) {
      const client = new StorageClient({
        baseUrl: this.resolveBaseUrl(),
        hAppId: CONTENT_SYNC_NAMESPACE,
      });
      // Single cast boundary: re-type the SDK helper to the local slice so the
      // alias-degraded automerge types stay out of this service's call sites.
      this.helper = this.makeSync(client) as unknown as DocSyncHelper;
    }
    return this.helper;
  }
}
