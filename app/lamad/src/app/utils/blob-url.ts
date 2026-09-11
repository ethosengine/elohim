/**
 * Blob URL resolution for the lamad bundle.
 *
 * ## The rule
 *
 * In **doorway mode** a blob URL is *origin-relative*: `/blob/{hash}` on the
 * origin that served the page. It is NEVER `environment.client.storageUrl` —
 * that address (`http://localhost:8090`) is the native storage sidecar and is
 * only reachable in **direct mode** (Tauri / native shell).
 *
 * ## Why this wrapper exists
 *
 * lamad is an independently served EPR app with SSR enabled. During a
 * server-side render the shared connection strategy auto-detects the runtime,
 * sees Node (`process.versions.node`) and resolves **direct** mode — so every
 * blob URL baked into the server-rendered HTML came out as
 * `http://localhost:8090/blob/{hash}`. That address is correct for the *server*
 * (elohim-storage really does answer there — the SSR transfer state shows a 200
 * from `http://localhost:8090/p2p/status`) but it is nonsense for the *browser*
 * that receives the HTML: every visitor got `ERR_CONNECTION_REFUSED` on
 * `<img src="http://localhost:8090/blob/…">`, plus a mixed-content warning.
 *
 * An origin-relative href is the only shape that is correct on both sides of
 * the SSR boundary, and it is what the doorway-failover invariant requires:
 * whichever doorway serves the page resolves the same declared head. So we
 * prefer the runtime origin over any build-time configuration.
 *
 * Only the *blob href* is rewritten here. `getStorageBaseUrl()` (which drives
 * `/db` and `/api` calls, including the SSR-side fetches that legitimately talk
 * to the local sidecar) is left on the delegate untouched.
 *
 * TODO(ssr-mode-detection): the general cure lives one layer down, in
 * `@elohim/service`'s `detectConnectionMode()` — it reads `process.versions.node`
 * and calls an SSR render "direct", which is true of the *renderer* and false of
 * the *browser being rendered for*. The same root cause also drifts Angular's
 * HTTP transfer-state keys (SSR caches under `http://localhost:8090/api/…`, the
 * browser looks up `https://<doorway>/api/…`, so the cache misses and every
 * hydrated page refetches). Fixing that needs an SSR-vs-native distinction in the
 * shared strategy and a sweep of both bundles; this wrapper closes the
 * visitor-visible hole in lamad without changing the shell's behaviour.
 */

import type { ILamadStorageClient } from '../interfaces/storage.interface';

/** Where blobs are served from for the current runtime. */
export type BlobConnectionMode = 'doorway' | 'direct';

export interface BlobUrlContext {
  /** How this runtime reaches storage. */
  mode: BlobConnectionMode;
  /** Serving origin of the page, when one is known (undefined under SSR). */
  origin?: string;
}

/**
 * Detect the blob connection mode and serving origin for the current runtime.
 *
 * `direct` is claimed only by a genuine native runtime — a Tauri shell, or an
 * explicit `__env.connectionMode = 'direct'` pin. Everything else (browser and
 * the SSR render of a page destined for a browser) is doorway-served.
 */
export function currentBlobUrlContext(): BlobUrlContext {
  const runtime = globalThis as Record<string, unknown>;
  const env = runtime['__env'] as Record<string, unknown> | undefined;

  const isNative = '__TAURI__' in runtime || env?.['connectionMode'] === 'direct';

  return {
    mode: isNative ? 'direct' : 'doorway',
    // eslint-disable-next-line no-restricted-syntax -- SSR-safe optional location access.
    origin: globalThis.location?.origin,
  };
}

/**
 * Resolve the href a browser should use to load a blob.
 *
 * @param blobHash Content address (`sha256-…`, `bafk…`)
 * @param sidecarClient Client that knows the native sidecar URL (direct mode only)
 * @param context Connection mode + serving origin; defaults to runtime detection
 */
export function resolveBlobHref(
  blobHash: string,
  sidecarClient: Pick<ILamadStorageClient, 'getBlobUrl'>,
  context: BlobUrlContext = currentBlobUrlContext()
): string {
  if (!blobHash) return '';

  // Native shell: the sidecar address is the right answer and the delegate owns it.
  if (context.mode === 'direct') {
    return sidecarClient.getBlobUrl(blobHash);
  }

  const path = `/blob/${blobHash}`;
  const origin = context.origin;

  // Browser: absolute-on-this-origin (an <img src> outside the Angular router
  // still resolves correctly). SSR: root-relative, resolved by the browser
  // against whichever doorway served the document.
  return origin && /^https?:\/\//.test(origin) ? `${origin}${path}` : path;
}

/**
 * Wrap a storage client so every blob href it hands out is origin-relative in
 * doorway mode. All other methods pass through to the delegate unchanged.
 */
export function withOriginRelativeBlobUrls(
  delegate: ILamadStorageClient,
  context: () => BlobUrlContext = currentBlobUrlContext
): ILamadStorageClient {
  return {
    getBlobUrl: (blobHash: string) => resolveBlobHref(blobHash, delegate, context()),
    getStorageBaseUrl: () => delegate.getStorageBaseUrl(),
    getContentEngagement: (contentId: string) => delegate.getContentEngagement(contentId),
  };
}
