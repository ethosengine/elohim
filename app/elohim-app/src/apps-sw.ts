/// <reference lib="webworker" />
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

self.addEventListener('activate', (event) => {
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

  return { peerId: peer.peerId, baseUrl, score, network: peer.network, servesExtracted, servesCompressed, warm };
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

/** Probe results cached per app_id — cleared on invalidation or new blob_hash */
const deliveryCache = new Map<string, DeliveryInfo>();

async function probeCapability(appId: string): Promise<DeliveryInfo> {
  const cached = deliveryCache.get(appId);
  if (cached) return cached;

  try {
    const resp = await fetch(`/apps/${appId}/_capability`, { method: 'HEAD' });
    const info: DeliveryInfo = {
      deliveryMode: resp.headers.get('X-Delivery-Mode') || 'compressed',
      blobHash: resp.headers.get('X-Blob-Hash') || '',
      cacheTier: resp.headers.get('X-Cache-Tier') || 'unknown',
      ready:
        resp.headers.get('X-Ready') === 'true' ||
        resp.headers.get('X-Projection-Ready') === 'true',
    };
    deliveryCache.set(appId, info);
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
  const appId = pathParts[0];
  const filePath = pathParts.slice(1).join('/');

  // 1. Check local cache first
  const cache = await caches.open(CACHE_NAME);
  const cached = await cache.match(request);
  if (cached) return cached;

  // 2. Probe peer capability (cached per app load)
  const capability = await probeCapability(appId);

  // 3. Try LAN/WAN peers in scored order (best-effort P2P delivery)
  const peers = await getDeliveryPeers(capability.blobHash);
  for (const peer of peers) {
    if (!peer.baseUrl) continue;
    try {
      if (peer.servesExtracted && peer.warm) {
        const resp = await fetch(`${peer.baseUrl}/apps/${appId}/${filePath}`);
        if (resp.ok) {
          cache.put(request, resp.clone());
          return resp;
        }
      }
    } catch {
      continue; // peer unreachable, try next
    }
  }

  // 4. Fall back to default path (doorway — the safety net)
  if (capability.deliveryMode === 'extracted' || capability.ready) {
    // Doorway can serve individual files — fetch and cache
    return fetchAndCache(cache, request);
  } else {
    // Doorway serves compressed only — fetch ZIP, extract, serve from cache
    return fetchViaZip(cache, appId, capability.blobHash, filePath);
  }
}

async function fetchAndCache(
  cache: Cache,
  request: Request,
): Promise<Response> {
  try {
    const response = await fetch(request);
    if (response.ok) {
      cache.put(request, response.clone());
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
  appId: string,
  blobHash: string,
  filePath: string,
): Promise<Response> {
  // Ensure ZIP is extracted (deduplicate concurrent extractions)
  if (!zipExtracting.has(appId)) {
    zipExtracting.set(appId, extractZip(cache, appId, blobHash));
  }
  await zipExtracting.get(appId);
  zipExtracting.delete(appId);

  // Now serve from cache
  const cached = await cache.match(
    new Request(`${self.location.origin}/apps/${appId}/${filePath}`),
  );
  if (cached) return cached;

  // Extraction didn't include this file — 404
  return new Response('File not found in app bundle', { status: 404 });
}

async function extractZip(
  cache: Cache,
  appId: string,
  blobHash: string,
): Promise<void> {
  // Fetch the raw ZIP blob
  const blobUrl = blobHash ? `/blob/${blobHash}` : `/apps/${appId}/`;
  const resp = await fetch(blobUrl);
  if (!resp.ok) return;

  const data = await resp.arrayBuffer();
  const zip = await JSZip.loadAsync(data);

  for (const [path, file] of Object.entries(zip.files)) {
    if (file.dir) continue;
    const content = await file.async('arraybuffer');
    const contentType = guessContentType(path);
    const response = new Response(content, {
      headers: { 'Content-Type': contentType },
    });
    await cache.put(
      new Request(`${self.location.origin}/apps/${appId}/${path}`),
      response,
    );
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
  const { type, appId } = event.data;
  if (type === 'invalidate' && appId) {
    const cache = await caches.open(CACHE_NAME);
    const keys = await cache.keys();
    const prefix = `/apps/${appId}/`;
    const toDelete = keys.filter((req) =>
      new URL(req.url).pathname.startsWith(prefix),
    );
    await Promise.all(toDelete.map((key) => cache.delete(key)));
    deliveryCache.delete(appId);
    console.log(
      `[apps-sw] invalidated ${toDelete.length} files for ${appId}`,
    );
  }
};
