/// <reference lib="webworker" />
/* eslint-disable no-console -- SW delivery evidence logging; parsed by a2o step definitions */
/*
 * Why this worker exists: a small peer that cannot afford to extract or serve an
 * app hands over raw bytes, and the client does the work — unzip, cache, serve.
 * That is also why verification lives here: bytes prove themselves against their
 * content address before this worker caches them.
 */
declare const self: ServiceWorkerGlobalScope;

import JSZip from 'jszip';

import { verifyRawSha256 } from './app/elohim/utils/raw-cid-verify';

import type { DeliveryPeer } from '@elohim/storage-client/generated';

// Bumped v1→v2 (2026-05-30): the prior cache held a stale index.html keyed to an
// old bundle hash (zip-non-determinism rotated the blobHash), which referenced new
// chunk filenames that 404'd → white screen. A new cache name forces fresh fetches
// against the current bundle. (Deterministic-hash + auto cache-invalidation are Sprint 2.)
// Bumped v2→v3 (2026-09-26): v2 entries were cached without verification; v3 holds
// only bytes extracted from an archive that hashed to its content address.
const CACHE_NAME = 'apps-v3';

/*
 * Remembered heads — `/apps/_heads/{slug}` in the same cache holds the blobHash of the
 * last archive this worker verified and fully extracted for that slug. The leading
 * underscore reserves the path the way `_capability` is reserved: a slug never starts
 * with `_`, and CID-keyed entries live under `/apps/sha256-…/`, so no app path collides.
 * Pages cannot read it (the fetch handler answers 404); only this worker uses it.
 */
const HEADS_SEGMENT = '_heads';

function headKey(slug: string): Request {
  return new Request(`${self.location.origin}/apps/${HEADS_SEGMENT}/${slug}`);
}

/** The blobHash this worker last verified for `slug`, or '' when it never has. */
async function recallHead(cache: Cache, slug: string): Promise<string> {
  const remembered = await cache.match(headKey(slug));
  return remembered ? (await remembered.text()).trim() : '';
}

// ---------------------------------------------------------------------------
// Lifecycle events
// ---------------------------------------------------------------------------

self.addEventListener('install', () => {
  // Activate immediately — don't wait for old SW to release
  void self.skipWaiting();
});

self.addEventListener('activate', event => {
  // Claim all clients immediately so the SW controls the page on first visit, and drop
  // every older apps cache — they may hold bytes that never proved themselves.
  event.waitUntil(
    (async () => {
      const names = await caches.keys();
      await Promise.all(
        names
          .filter(name => name.startsWith('apps-') && name !== CACHE_NAME)
          .map(name => caches.delete(name))
      );
      await self.clients.claim();
    })()
  );
});

// ---------------------------------------------------------------------------
// Fetch interception — only for /apps/ requests
// ---------------------------------------------------------------------------

self.addEventListener('fetch', (event: FetchEvent) => {
  const url = new URL(event.request.url);
  if (!url.pathname.startsWith('/apps/')) return;
  if (url.pathname.endsWith('/_capability')) return; // don't cache probes
  if (url.pathname.startsWith(`/apps/${HEADS_SEGMENT}/`)) {
    // The worker's own memory, not an app path — never served to a page.
    event.respondWith(new Response('Not found', { status: 404 }));
    return;
  }

  event.respondWith(handleAppFetch(event));
});

// ---------------------------------------------------------------------------
// Multi-peer scoring (inline — SW can't import from elohim-service at runtime)
// ---------------------------------------------------------------------------

interface ScoredPeer {
  peerId: string;
  baseUrl: string;
  score: number;
  network: string;
  servesExtracted: boolean;
  servesCompressed: boolean;
  warm: boolean;
}

// Wire shape comes from the Rust struct via ts-rs; this file used to hand-copy
// it, and the copy in @elohim/service drifted (it invented a `network: 'relay'`
// variant the Rust side never writes).
type DeliveryPeerResponse = DeliveryPeer;

function extractIpFromMultiaddrSw(addr: string): string {
  const match = /\/ip4\/([^/]+)/.exec(addr);
  return match ? match[1] : '';
}

function scorePeerForSw(peer: DeliveryPeerResponse, contentHash: string): ScoredPeer {
  let score = 0;

  // Network proximity (biggest factor)
  if (peer.network === 'lan') score += 1000;
  else if (peer.network === 'wan') score += 500;
  else score += 100; // relay

  const servesExtracted = peer.capabilities.includes('serves_extracted');
  const servesCompressed = peer.capabilities.includes('serves_compressed');
  const warm = peer.capabilities.includes(`warm:${contentHash}`);

  // Delivery capability
  if (servesExtracted) score += 200;
  if (servesCompressed) score += 50;

  // Warm cache for THIS content
  if (warm) score += 300;

  // Recency
  const age = Date.now() - peer.lastSeen;
  if (age < 30000) score += 100;
  else if (age < 90000) score += 50;

  // Construct baseUrl from multiaddr (extract IP) + httpPort
  const ip = extractIpFromMultiaddrSw(peer.multiaddrs[0] || '');
  const baseUrl = ip ? `http://${ip}:${peer.httpPort}` : '';

  return {
    peerId: peer.peerId,
    baseUrl,
    score,
    network: peer.network,
    servesExtracted,
    servesCompressed,
    warm,
  };
}

// ---------------------------------------------------------------------------
// Delivery peer discovery (30s cache)
// ---------------------------------------------------------------------------

let peerCache: { peers: ScoredPeer[]; fetchedAt: number } | null = null;
const PEER_CACHE_TTL = 30000; // 30s

async function getDeliveryPeers(blobHash: string): Promise<ScoredPeer[]> {
  if (peerCache && Date.now() - peerCache.fetchedAt < PEER_CACHE_TTL) {
    return peerCache.peers;
  }

  try {
    const resp = await fetch('/api/v1/peers/delivery');
    if (!resp.ok) return [];
    const peers: DeliveryPeerResponse[] = await resp.json();

    const scored = peers
      .map(p => scorePeerForSw(p, blobHash))
      .filter(p => p.baseUrl) // only peers with reachable URLs
      .sort((a, b) => b.score - a.score);

    peerCache = { peers: scored, fetchedAt: Date.now() };
    return scored;
  } catch {
    return [];
  }
}

// ---------------------------------------------------------------------------
// Capability probe (Task 7)
// ---------------------------------------------------------------------------

interface DeliveryInfo {
  deliveryMode: string; // 'extracted' | 'compressed' | 'blob-only'
  blobHash: string;
  cacheTier: string; // 'projection' | 'extraction' | 'blob-only'
  ready: boolean;
}

/**
 * Successful probe results cached per slug for this worker's lifetime — cleared on
 * invalidation. A failed probe (offline, error, non-OK, no address) is never memoized:
 * remembering a failure would pin the slug to "no address" after the network returns.
 */
const deliveryCache = new Map<string, DeliveryInfo>();

const NO_DELIVERY_INFO: DeliveryInfo = {
  deliveryMode: 'compressed',
  blobHash: '',
  cacheTier: 'unknown',
  ready: false,
};

async function probeCapability(slug: string): Promise<DeliveryInfo> {
  const cached = deliveryCache.get(slug);
  if (cached) return cached;

  try {
    const resp = await fetch(`/apps/${slug}/_capability`, { method: 'HEAD' });
    if (!resp.ok) return NO_DELIVERY_INFO;
    const info: DeliveryInfo = {
      deliveryMode: resp.headers.get('X-Delivery-Mode') ?? 'compressed',
      blobHash: resp.headers.get('X-Blob-Hash') ?? '',
      cacheTier: resp.headers.get('X-Cache-Tier') ?? 'unknown',
      ready:
        resp.headers.get('X-Ready') === 'true' || resp.headers.get('X-Projection-Ready') === 'true',
    };
    if (info.blobHash) deliveryCache.set(slug, info);
    return info;
  } catch {
    return NO_DELIVERY_INFO;
  }
}

// ---------------------------------------------------------------------------
// Cache-first fetch for extracted delivery (Task 8)
// ---------------------------------------------------------------------------

/**
 * Ask the scored delivery peers for a file, best-effort.
 *
 * Returns the first successful peer response, or null when no peer serves it --
 * an unreachable or unhelpful peer is an ordinary outcome here, not an error, so
 * every failure just advances to the next candidate.
 */
async function tryDeliveryPeers(
  blobHash: string,
  identifier: string,
  filePath: string
): Promise<Response | null> {
  const peers = await getDeliveryPeers(blobHash);
  for (const peer of peers) {
    if (!peer.baseUrl) continue;
    if (!peer.servesExtracted || !peer.warm) continue;
    try {
      console.log(
        `[apps-sw] peer-try: ${peer.peerId.slice(0, 12)} network=${peer.network} score=${peer.score}`
      );
      const resp = await fetch(`${peer.baseUrl}/apps/${identifier}/${filePath}`);
      if (resp.ok) {
        console.log(`[apps-sw] peer-hit: ${peer.peerId.slice(0, 12)} ${identifier}/${filePath}`);
        // Served but not cached: one file's bytes cannot be checked against the
        // bundle's address, so peer bytes stay out of the cache until per-file digests exist.
        return resp;
      }
    } catch {
      continue; // peer unreachable, try the next one
    }
  }
  return null;
}

async function handleAppFetch(event: FetchEvent): Promise<Response> {
  const request = event.request;
  const url = new URL(request.url);
  const pathParts = url.pathname.replace('/apps/', '').split('/');
  const identifier = pathParts[0]; // Could be slug or blob_hash
  const filePath = pathParts.slice(1).join('/');

  console.log(`[apps-sw] fetch: ${url.pathname}`);

  const cache = await caches.open(CACHE_NAME);

  // 1. Probe peer capability to get the blob_hash for CID-based caching
  const capability = await probeCapability(identifier);
  let blobHash = capability.blobHash;

  console.log(
    `[apps-sw] capability: ${identifier} deliveryMode=${capability.deliveryMode} ready=${capability.ready} blobHash=${capability.blobHash.slice(0, 12)}...`
  );

  // 1b. No address from the network (offline, error, non-OK): fall back to the version
  // this worker last verified for the slug. The bytes under it proved themselves
  // against their address; the NAME slug → blobHash is only as good as the doorway's
  // X-Blob-Hash header at the time it was verified — the election is not consulted
  // yet (known gap), so this can serve a superseded version, never unverified bytes.
  const recalled = blobHash ? '' : await recallHead(cache, identifier);
  if (recalled) {
    blobHash = recalled;
    console.log(`[apps-sw] head-recalled: ${identifier} → ${recalled}`);
  }

  // 2. Try cache under CID key first (immutable content address)
  if (blobHash) {
    const cidKey = new Request(`${self.location.origin}/apps/${blobHash}/${filePath}`);
    const cached = await cache.match(cidKey);
    if (cached) {
      console.log(`[apps-sw] cache-hit: ${filePath} key=${blobHash}`);
      return cached;
    }
  }

  // 3. Also check under the original URL (backwards compat)
  const cached = await cache.match(request);
  if (cached) {
    console.log(`[apps-sw] cache-hit: ${filePath} key=${blobHash || identifier}`);
    return cached;
  }

  // A head is remembered only after the whole archive was extracted, so a miss under a
  // recalled head means the bundle holds no such file — answer that without the network.
  if (recalled) {
    console.log(`[apps-sw] recalled-miss: ${filePath} not in ${recalled}`);
    return new Response('File not found in app bundle', { status: 404 });
  }

  // 4. Try LAN/WAN peers in scored order (best-effort P2P delivery)
  const fromPeer = await tryDeliveryPeers(blobHash, identifier, filePath);
  if (fromPeer) return fromPeer;

  // 5. Fall back to default path (doorway — the safety net)
  console.log(
    `[apps-sw] fallback: mode=${capability.deliveryMode} ready=${capability.ready} → ${capability.deliveryMode === 'extracted' || capability.ready ? 'fetch-individual' : 'fetch-zip'}`
  );
  if (capability.deliveryMode === 'extracted' || capability.ready) {
    // Doorway can serve individual files — serve this one now, and fill the cache
    // from the verified archive in the background so the next load can go offline.
    if (blobHash) event.waitUntil(ensureVerifiedExtraction(cache, identifier, blobHash));
    return fetchIndividual(request, identifier, filePath);
  } else {
    // Doorway serves compressed only — fetch ZIP, extract, serve from cache
    return fetchViaZip(cache, identifier, blobHash, filePath);
  }
}

/**
 * Serve one file from the network without caching it. The doorway's
 * X-Content-Address names the bundle, not this file, so a single file's bytes
 * cannot prove themselves; only bytes extracted from a verified archive are cached.
 */
async function fetchIndividual(
  request: Request,
  slug: string,
  filePath: string
): Promise<Response> {
  try {
    const response = await fetch(request);
    if (response.ok) console.log(`[apps-sw] served-uncached: ${slug}/${filePath}`);
    return response;
  } catch {
    // Network failure — return offline error
    return new Response('Offline — app not cached', { status: 503 });
  }
}

// ---------------------------------------------------------------------------
// ZIP extraction (Task 9)
// ---------------------------------------------------------------------------

/** Track ZIP extraction state to avoid duplicate concurrent downloads */
const zipExtracting = new Map<string, Promise<void>>();

async function fetchViaZip(
  cache: Cache,
  slug: string,
  blobHash: string,
  filePath: string
): Promise<Response> {
  // Without a content address nothing can be verified, so nothing is cached:
  // pass the request through to the network instead of failing the app.
  if (!blobHash) {
    console.warn(`[apps-sw] verify-skip: no address for ${slug} — serving uncached`);
    return fetchIndividual(new Request(`/apps/${slug}/${filePath}`), slug, filePath);
  }

  await ensureVerifiedExtraction(cache, slug, blobHash);

  // Now serve from cache under the CID key
  const cached = await cache.match(
    new Request(`${self.location.origin}/apps/${blobHash}/${filePath}`)
  );
  if (cached) return cached;

  // Extraction didn't include this file — 404
  return new Response('File not found in app bundle', { status: 404 });
}

/** Extract a bundle's verified archive into the cache once, deduplicating concurrent callers. */
async function ensureVerifiedExtraction(
  cache: Cache,
  slug: string,
  blobHash: string
): Promise<void> {
  // Already verified and extracted for this slug — nothing to fetch again.
  if ((await recallHead(cache, slug)) === blobHash) return;
  let pending = zipExtracting.get(blobHash);
  if (!pending) {
    pending = extractZip(cache, slug, blobHash).finally(() => zipExtracting.delete(blobHash));
    zipExtracting.set(blobHash, pending);
  }
  await pending;
}

async function extractZip(cache: Cache, slug: string, blobHash: string): Promise<void> {
  // Nothing is cached until the archive proves itself against its content address,
  // so without an address there is nothing worth fetching.
  if (!blobHash) {
    console.warn(`[apps-sw] verify-skip: no address for ${slug} — not caching`);
    return;
  }
  // Fetch the raw ZIP blob — unreachable is an ordinary outcome (offline), not an error.
  const blobUrl = `/blob/${blobHash}`;
  let resp: Response;
  try {
    resp = await fetch(blobUrl);
  } catch {
    console.warn(`[apps-sw] zip-unreachable: ${blobUrl} — not caching`);
    return;
  }
  if (!resp.ok) return;

  const data = await resp.arrayBuffer();
  console.log(`[apps-sw] zip-fetch: ${blobUrl} size=${data.byteLength}`);
  if (!(await verifyRawSha256(data, blobHash))) {
    console.warn(`[apps-sw] verify-fail: ${blobUrl} does not hash to ${blobHash} — not caching`);
    return;
  }
  console.log(`[apps-sw] verify-ok: ${blobHash}`);
  // Archive-expansion review: this archive is fetched from the household's own
  // blob store by content address, for an app the peer explicitly installed, and
  // is expanded into a private Cache Storage entry rather than onto a filesystem.
  // A hostile archive would have to be admitted to the blob store first, which is
  // a separate boundary with its own controls.
  // eslint-disable-next-line sonarjs/no-unsafe-unzip -- see review note above
  const zip = await JSZip.loadAsync(data);

  // Cache under CID (immutable) when available, slug as fallback
  const cachePrefix = blobHash || slug;

  // Bundles are often zipped with one top-level folder (`evolution-of-trust/index.html`)
  // while pages ask for `/apps/{slug}/index.html`; the peer's server matches by suffix.
  // Strip a root every entry shares so the cache keys are the paths pages request.
  const entries = Object.entries(zip.files).filter(([, file]) => !file.dir);
  const root = sharedRootDir(entries.map(([path]) => path));

  for (const [entryPath, file] of entries) {
    const path = entryPath.slice(root.length);
    const content = await file.async('arraybuffer');
    const contentType = guessContentType(path);
    const response = new Response(content, {
      headers: { 'Content-Type': contentType },
    });
    await cache.put(new Request(`${self.location.origin}/apps/${cachePrefix}/${path}`), response);
    console.log(`[apps-sw] cache-put: ${cachePrefix}/${path} type=${contentType}`);
  }

  // Remember the head only once every entry is in the cache: a recalled head then names
  // a complete, verified bundle. A request made by CID has no slug to remember.
  if (slug !== blobHash) {
    await cache.put(
      headKey(slug),
      new Response(blobHash, { headers: { 'Content-Type': 'text/plain; charset=utf-8' } })
    );
    console.log(`[apps-sw] head-remembered: ${slug} → ${blobHash}`);
  }
}

/** `dir/` when every entry sits under one shared top-level directory, else ''. */
function sharedRootDir(paths: string[]): string {
  const first = paths[0]?.split('/')[0];
  if (!first || paths.some(path => !path.startsWith(`${first}/`))) return '';
  return `${first}/`;
}

function guessContentType(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() ?? '';
  const types: Record<string, string> = {
    html: 'text/html; charset=utf-8',
    js: 'application/javascript; charset=utf-8',
    mjs: 'application/javascript; charset=utf-8',
    css: 'text/css; charset=utf-8',
    json: 'application/json; charset=utf-8',
    png: 'image/png',
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    gif: 'image/gif',
    svg: 'image/svg+xml',
    woff: 'font/woff',
    woff2: 'font/woff2',
    ttf: 'font/ttf',
    ico: 'image/x-icon',
    wasm: 'application/wasm',
    mp3: 'audio/mpeg',
    ogg: 'audio/ogg',
    mp4: 'video/mp4',
    webp: 'image/webp',
    webm: 'video/webm',
    xml: 'application/xml',
    txt: 'text/plain; charset=utf-8',
  };
  return types[ext] || 'application/octet-stream';
}

// ---------------------------------------------------------------------------
// Cache invalidation via BroadcastChannel (Task 10)
// ---------------------------------------------------------------------------

const channel = new BroadcastChannel('apps-sw');
channel.onmessage = async (event: MessageEvent) => {
  const { type, slug, blobHash } = event.data;
  if (type === 'invalidate' && (slug || blobHash)) {
    const cache = await caches.open(CACHE_NAME);
    const keys = await cache.keys();

    // Build prefix list — clear both slug-keyed and CID-keyed entries
    const prefixes: string[] = [];
    if (slug) prefixes.push(`/apps/${slug}/`);
    if (blobHash) prefixes.push(`/apps/${blobHash}/`);

    const toDelete = keys.filter(req => {
      const pathname = new URL(req.url).pathname;
      return prefixes.some(prefix => pathname.startsWith(prefix));
    });
    await Promise.all(toDelete.map(async key => cache.delete(key)));
    // Forget the remembered head too, so an offline load cannot recall evicted bytes.
    if (slug) await cache.delete(headKey(slug));

    // Clear delivery probe cache for this slug
    if (slug) deliveryCache.delete(slug);
    console.log(
      `[apps-sw] invalidated ${toDelete.length} files for ${[slug, blobHash].filter(Boolean).join(' / ')}`
    );
  }
};
