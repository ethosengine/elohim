/**
 * ObservationEmitterService — a person's attention on one content node,
 * witnessed as an agent-private observation on their own node.
 *
 * Ruling R-A3 of the post-station-4 sprint: `begin(refCid)` when a node is
 * shown, `noteScroll(pct)` as the person scrolls, `end(refCid)` when they
 * leave. `end` posts one `lamad:content-viewed` observation to
 * POST /api/v1/observations with the dwell and the deepest scroll reached.
 *
 * - Dwell counts only visible time: it pauses while
 *   `document.visibilityState === 'hidden'` (this service's own
 *   `visibilitychange` listener) and resumes when the tab returns.
 * - Scroll depth is a monotonic maximum, an integer clamped to 0..100; a
 *   non-finite report is ignored, so the payload never carries NaN. Callers
 *   derive a report from the page geometry with {@link scrollDepthPct}, which
 *   gives a page shorter than its viewport 100.
 * - No threshold lives here. Whether a view "counts" (the long-dwell lens) is
 *   the lifestream recipe's lens table, on the node.
 * - No `session_id`: the app's session id is a random UUID, not a CID.
 * - The observer is never named in the body: the node takes it from the
 *   caller's `X-Agent-Cid` header, and a body naming anyone else is refused.
 * - A failed post is swallowed — witnessing attention never breaks reading.
 * - The post goes to the storage base the host app resolves
 *   ({@link OBSERVATION_STORAGE_BASE_URL}, the same `getStorageBaseUrl()` the
 *   lifestream reads through), never a bare path on the serving origin (R-A11).
 * - A hard leave still witnesses (R-A8): on `pagehide` every open view is
 *   flushed with `navigator.sendBeacon`, or `fetch(…, {keepalive: true})` when
 *   there is no beacon or the browser refuses it. `visibilitychange` → hidden
 *   flushes only where `pagehide` does not exist; elsewhere hiding a tab just
 *   pauses dwell. A flushed view is closed, so the in-app `end` that may follow
 *   posts nothing more.
 */

import { DOCUMENT } from '@angular/common';
import { HttpClient } from '@angular/common/http';
import { Injectable, InjectionToken, OnDestroy, inject } from '@angular/core';

/** The observation write route, relative to the storage base. */
const OBSERVATIONS_PATH = '/api/v1/observations';

/**
 * Resolves the storage base URL observations are written to. The host app
 * provides the same resolution its storage client reads through
 * (`() => storageClient.getStorageBaseUrl()`), so a Tauri sidecar or a dev
 * proxy write lands where the lifestream reads. The default is origin-relative
 * (`''`), for hosts whose serving origin is the storage path.
 */
export const OBSERVATION_STORAGE_BASE_URL = new InjectionToken<() => string>(
  'ObservationStorageBaseUrl',
  {
    providedIn: 'root',
    factory: () => () => '',
  }
);

// ---------------------------------------------------------------------------
// Wire types (aligned with inputs/observation-intent.schema.json and
// views/observation-accepted-view.schema.json)
// ---------------------------------------------------------------------------

/** The manifest-declared observation kind for a content view. */
export const CONTENT_VIEWED_KIND = 'lamad:content-viewed';

/** Wire shape for the POST /api/v1/observations request body. */
export interface ObservationIntent {
  /** A manifest-declared observation kind, e.g. `lamad:content-viewed`. */
  observationKind: string;
  /** What was observed. */
  subjectCid?: string;
  /** What kind of thing the subject is. */
  subjectKind?: string;
  /** Unix epoch seconds; the node stamps it when absent. */
  observedAt?: number;
  /** The payload as a JSON string, shaped by the kind's declared field map. */
  payloadJson: string;
}

/** Response from POST /api/v1/observations (201). */
export interface ObservationAck {
  observerCid: string;
  observerCidNamespace: string;
  logCid: string;
  logOffset: number;
  seq: number;
  signed: string;
}

/** The declared payload of a `lamad:content-viewed` observation. */
interface ContentViewedPayload {
  ref_cid: string;
  dwell_ms: number;
  scroll_depth_pct: number;
}

// ---------------------------------------------------------------------------
// Scroll depth from page geometry
// ---------------------------------------------------------------------------

/**
 * The deepest point a viewport reaches, as an integer percentage of the page:
 * `(scrollTop + viewportHeight) / documentHeight`, clamped to 0..100.
 * A page no taller than its viewport is fully seen: 100. Never NaN.
 */
export function scrollDepthPct(
  scrollTop: number,
  viewportHeight: number,
  documentHeight: number
): number {
  if (!(documentHeight > viewportHeight)) {
    return 100;
  }
  const pct = ((scrollTop + viewportHeight) / documentHeight) * 100;
  return clampPct(pct) ?? 0;
}

/** An integer in 0..100, or null for a non-finite report. */
function clampPct(pct: number): number | null {
  if (Number.isNaN(pct)) {
    return null;
  }
  return Math.round(Math.min(100, Math.max(0, pct)));
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/** One open view: visible time banked so far, and the running visible stretch. */
interface OpenView {
  bankedMs: number;
  visibleSince: number | null;
  maxDepthPct: number;
}

@Injectable({ providedIn: 'root' })
export class ObservationEmitterService implements OnDestroy {
  private readonly http = inject(HttpClient);
  private readonly document = inject(DOCUMENT);
  private readonly resolveBaseUrl = inject(OBSERVATION_STORAGE_BASE_URL);

  /** Open views keyed by the content's ref CID. */
  private readonly open = new Map<string, OpenView>();

  private readonly onVisibilityChange = (): void => {
    const now = Date.now();
    const hidden = this.isHidden();
    if (hidden && !this.hasPageHide) {
      this.flushAll();
      return;
    }
    for (const view of this.open.values()) {
      if (hidden) {
        this.bank(view, now);
      } else if (view.visibleSince === null) {
        view.visibleSince = now;
      }
    }
  };

  /** The page is going away (tab close, full navigation): flush every open view. */
  private readonly onPageHide = (): void => {
    this.flushAll();
  };

  /** Where `pagehide` does not exist, hidden is the last reliable moment. */
  private readonly hasPageHide: boolean;

  constructor() {
    this.document.addEventListener('visibilitychange', this.onVisibilityChange);
    const win = this.document.defaultView;
    this.hasPageHide = !!win && 'onpagehide' in win;
    win?.addEventListener('pagehide', this.onPageHide);
  }

  /**
   * The person is now looking at `refCid`. A second `begin` for a ref that is
   * already open keeps the first start.
   */
  begin(refCid: string): void {
    if (!refCid || this.open.has(refCid)) {
      return;
    }
    this.open.set(refCid, {
      bankedMs: 0,
      visibleSince: this.isHidden() ? null : Date.now(),
      maxDepthPct: 0,
    });
  }

  /**
   * The deepest scroll reached so far, as a percentage of the page. Applies to
   * every open view; depth only ever grows. Non-finite reports are ignored.
   */
  noteScroll(pct: number): void {
    const depth = clampPct(pct);
    if (depth === null) {
      return;
    }
    for (const view of this.open.values()) {
      view.maxDepthPct = Math.max(view.maxDepthPct, depth);
    }
  }

  /**
   * The person has left `refCid`: post one `lamad:content-viewed` observation.
   * A ref that was never begun (or already ended) posts nothing.
   */
  end(refCid: string): void {
    const intent = this.close(refCid);
    if (!intent) {
      return;
    }
    this.http.post<ObservationAck>(this.url(), intent).subscribe({
      error: () => {
        // Swallowed: a missed witness never interrupts reading.
      },
    });
  }

  ngOnDestroy(): void {
    this.document.removeEventListener('visibilitychange', this.onVisibilityChange);
    this.document.defaultView?.removeEventListener('pagehide', this.onPageHide);
    this.open.clear();
  }

  /** The write URL on the host's storage base. */
  private url(): string {
    return `${this.resolveBaseUrl()}${OBSERVATIONS_PATH}`;
  }

  /** Close `refCid`'s open view and shape its intent; null when none is open. */
  private close(refCid: string): ObservationIntent | null {
    const view = this.open.get(refCid);
    if (!view) {
      return null;
    }
    this.open.delete(refCid);
    this.bank(view, Date.now());

    const payload: ContentViewedPayload = {
      ref_cid: refCid,
      dwell_ms: Math.max(0, Math.round(view.bankedMs)),
      scroll_depth_pct: view.maxDepthPct,
    };
    return {
      observationKind: CONTENT_VIEWED_KIND,
      subjectCid: refCid,
      subjectKind: 'content',
      payloadJson: JSON.stringify(payload),
    };
  }

  /**
   * Post every open view with a transport that outlives the page. Each view is
   * closed first, so a later `end` for the same ref is a no-op.
   */
  private flushAll(): void {
    for (const refCid of [...this.open.keys()]) {
      const intent = this.close(refCid);
      if (intent) {
        this.sendOutlivingPage(JSON.stringify(intent));
      }
    }
  }

  /** `sendBeacon` when the browser has and accepts it, else a keepalive fetch. */
  private sendOutlivingPage(body: string): void {
    const url = this.url();
    const nav = this.document.defaultView?.navigator;
    try {
      if (
        typeof nav?.sendBeacon === 'function' &&
        nav.sendBeacon(url, new Blob([body], { type: 'application/json' }))
      ) {
        return;
      }
    } catch {
      // A beacon the browser rejects outright falls through to fetch.
    }
    if (typeof fetch !== 'function') {
      return;
    }
    fetch(url, {
      method: 'POST',
      keepalive: true,
      headers: { 'Content-Type': 'application/json' },
      body,
    }).catch(() => {
      // Swallowed: a missed witness never interrupts leaving.
    });
  }

  private isHidden(): boolean {
    return this.document.visibilityState === 'hidden';
  }

  /** Move the running visible stretch into the bank and pause. */
  private bank(view: OpenView, now: number): void {
    if (view.visibleSince !== null) {
      view.bankedMs += Math.max(0, now - view.visibleSince);
      view.visibleSince = null;
    }
  }
}
