/// <reference lib="webworker" />
/* eslint-disable no-console -- SW delivery evidence logging; parsed by a2o step definitions */
declare const self: ServiceWorkerGlobalScope;

import JSZip from 'jszip';

const CACHE_NAME = 'apps-v1';

// ---------------------------------------------------------------------------
// Lifecycle events
// ---------------------------------------------------------------------------

self.addEventListener('install', () => {
  // Activate immediately — don't wait for old SW to release
  self.skipWaiting();
});

self.addEventListener('activate', event => {
  // Claim all clients immediately so the SW controls the page on first visit
  event.waitUntil(self.clients.claim());
});

// ---------------------------------------------------------------------------
// Fetch interception — only for /apps/ requests
// ---------------------------------------------------------------------------

self.addEventListener('fetch', (event: FetchEvent) => {
  const url = new URL(event.request.url);
  if (!url.pathname.startsWith('/apps/')) return;
  if (url.pathname.endsWith('/_capability')) return; // don't cache probes

  event.respondWith(handleAppFetch(event.request));
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

interface DeliveryPeerResponse {
  peerId: string;
  multiaddrs: string[];
  network: string;
  capabilities: string[];
  lastSeen: number;
  httpPort: number;
}

function extractIpFromMultiaddrSw(addr: string): string {
  const match = addr.match(/\/ip4\/([^/]+)/);
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

/** Probe results cached per slug — cleared on invalidation or new blob_hash */
const deliveryCache = new Map<string, DeliveryInfo>();

async function probeCapability(slug: string): Promise<DeliveryInfo> {
  const cached = deliveryCache.get(slug);
  if (cached) return cached;

  try {
    const resp = await fetch(`/apps/${slug}/_capability`, { method: 'HEAD' });
    const info: DeliveryInfo = {
      deliveryMode: resp.headers.get('X-Delivery-Mode') || 'compressed',
      blobHash: resp.headers.get('X-Blob-Hash') || '',
      cacheTier: resp.headers.get('X-Cache-Tier') || 'unknown',
      ready:
        resp.headers.get('X-Ready') === 'true' || resp.headers.get('X-Projection-Ready') === 'true',
    };
    deliveryCache.set(slug, info);
    return info;
  } catch {
    return {
      deliveryMode: 'compressed',
      blobHash: '',
      cacheTier: 'unknown',
      ready: false,
    };
  }
}

// ---------------------------------------------------------------------------
// Cache-first fetch for extracted delivery (Task 8)
// ---------------------------------------------------------------------------

async function handleAppFetch(request: Request): Promise<Response> {
  const url = new URL(request.url);
  const pathParts = url.pathname.replace('/apps/', '').split('/');
  const identifier = pathParts[0]; // Could be slug or blob_hash
  const filePath = pathParts.slice(1).join('/');

  console.log(`[apps-sw] fetch: ${url.pathname}`);

  const cache = await caches.open(CACHE_NAME);

  // 1. Probe peer capability to get the blob_hash for CID-based caching
  const capability = await probeCapability(identifier);
  const blobHash = capability.blobHash;

  console.log(
    `[apps-sw] capability: ${identifier} deliveryMode=${capability.deliveryMode} ready=${capability.ready} blobHash=${capability.blobHash.slice(0, 12)}...`
  );

  // 2. Try cache under CID key first (immutable content address)
  if (blobHash) {
    const cidKey = new Request(`${self.location.origin}/apps/${blobHash}/${filePath}`);
    const cached = await cache.match(cidKey);
    if (cached) {
      console.log(`[apps-sw] cache-hit: ${filePath} key=${blobHash || identifier}`);
      return cached;
    }
  }

  // 3. Also check under the original URL (backwards compat)
  const cached = await cache.match(request);
  if (cached) {
    console.log(`[apps-sw] cache-hit: ${filePath} key=${blobHash || identifier}`);
    return cached;
  }

  // 4. Try LAN/WAN peers in scored order (best-effort P2P delivery)
  const peers = await getDeliveryPeers(blobHash);
  for (const peer of peers) {
    if (!peer.baseUrl) continue;
    try {
      console.log(
        `[apps-sw] peer-try: ${peer.peerId.slice(0, 12)} network=${peer.network} score=${peer.score}`
      );
      if (peer.servesExtracted && peer.warm) {
        const resp = await fetch(`${peer.baseUrl}/apps/${identifier}/${filePath}`);
        if (resp.ok) {
          console.log(`[apps-sw] peer-hit: ${peer.peerId.slice(0, 12)} ${identifier}/${filePath}`);
          const cacheKey = buildCacheKey(blobHash, identifier, filePath);
          cache.put(cacheKey, resp.clone());
          return resp;
        }
      }
    } catch {
      continue; // peer unreachable, try next
    }
  }

  // 5. Fall back to default path (doorway — the safety net)
  console.log(
    `[apps-sw] fallback: mode=${capability.deliveryMode} ready=${capability.ready} → ${capability.deliveryMode === 'extracted' || capability.ready ? 'fetch-individual' : 'fetch-zip'}`
  );
  if (capability.deliveryMode === 'extracted' || capability.ready) {
    // Doorway can serve individual files — fetch and cache under CID
    return fetchAndCacheByCid(cache, request, blobHash, identifier, filePath);
  } else {
    // Doorway serves compressed only — fetch ZIP, extract, serve from cache
    return fetchViaZip(cache, identifier, blobHash, filePath);
  }
}

/** Build a cache key Request — prefer CID (immutable) over slug */
function buildCacheKey(blobHash: string, slug: string, filePath: string): Request {
  const prefix = blobHash || slug;
  return new Request(`${self.location.origin}/apps/${prefix}/${filePath}`);
}

async function fetchAndCacheByCid(
  cache: Cache,
  request: Request,
  blobHash: string,
  slug: string,
  filePath: string
): Promise<Response> {
  try {
    const response = await fetch(request);
    if (response.ok) {
      // Read content address from response header (doorway sets this)
      const contentAddress = response.headers.get('X-Content-Address') || blobHash;
      const responseToCache = response.clone();

      // Cache under CID key (immutable) if we have a content address
      if (contentAddress) {
        const cidKey = new Request(`${self.location.origin}/apps/${contentAddress}/${filePath}`);
        await cache.put(cidKey, responseToCache);
      } else {
        await cache.put(request, responseToCache);
      }
      console.log(`[apps-sw] cached: ${slug}/${filePath} address=${contentAddress}`);
    }
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
  const extractionKey = blobHash || slug;

  // Ensure ZIP is extracted (deduplicate concurrent extractions)
  if (!zipExtracting.has(extractionKey)) {
    zipExtracting.set(extractionKey, extractZip(cache, slug, blobHash));
  }
  await zipExtracting.get(extractionKey);
  zipExtracting.delete(extractionKey);

  // Now serve from cache — look under CID key first, then slug
  const cachePrefix = blobHash || slug;
  const cached = await cache.match(
    new Request(`${self.location.origin}/apps/${cachePrefix}/${filePath}`)
  );
  if (cached) return cached;

  // Extraction didn't include this file — 404
  return new Response('File not found in app bundle', { status: 404 });
}

async function extractZip(cache: Cache, slug: string, blobHash: string): Promise<void> {
  // Fetch the raw ZIP blob
  const blobUrl = blobHash ? `/blob/${blobHash}` : `/apps/${slug}/`;
  const resp = await fetch(blobUrl);
  if (!resp.ok) return;

  const data = await resp.arrayBuffer();
  console.log(`[apps-sw] zip-fetch: ${blobUrl} size=${data.byteLength}`);
  const zip = await JSZip.loadAsync(data);

  // Cache under CID (immutable) when available, slug as fallback
  const cachePrefix = blobHash || slug;

  for (const [path, file] of Object.entries(zip.files)) {
    if (file.dir) continue;
    const content = await file.async('arraybuffer');
    const contentType = guessContentType(path);
    const response = new Response(content, {
      headers: { 'Content-Type': contentType },
    });
    await cache.put(new Request(`${self.location.origin}/apps/${cachePrefix}/${path}`), response);
    console.log(`[apps-sw] cache-put: ${cachePrefix}/${path} type=${contentType}`);
  }
}

function guessContentType(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() || '';
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

    // Clear delivery probe cache for this slug
    if (slug) deliveryCache.delete(slug);
    console.log(
      `[apps-sw] invalidated ${toDelete.length} files for ${[slug, blobHash].filter(Boolean).join(' / ')}`
    );
  }
};
