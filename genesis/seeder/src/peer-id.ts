/**
 * Peer ID Derivation & Resolution — Single Source of Truth
 *
 * The peer_id derivation formula is the load-bearing contract between:
 * - seed-agent-bindings: writes peer_identity_bindings DHT entries
 * - seed-commitments: writes rea_commitments with peer_id-keyed provider/receiver
 *
 * Drift here causes cluster_view + reciprocity_view SQL filters to silently
 * return empty results. Extract to a single shared utility to prevent divergence.
 *
 * **Stage 1**: Deterministic test-only peer_id. Not a valid libp2p PeerId
 * (suffix is hex, not base58btc) — opaque string at the projection layer.
 *
 * **Stage 2 (`resolvePeerId`)**: Fetch each peer's REAL libp2p peer id from
 * its storage pod (`GET http://<host>:8090/p2p/status` → `peerId`, camelCase
 * per `P2PStatusInfo`'s `#[serde(rename_all = "camelCase")]`). This matters
 * because elohim-storage's custody sweep kicks a blob fetch iff
 * `commitment.provider == self_cid` (the node's real libp2p peer id) — a
 * Stage-1 fake id can NEVER match, so commitments seeded with it leave the
 * custody/REA replication layer decorative.
 */

import { createHash } from 'node:crypto';

export type Archetype = 'node' | 'desktop' | 'mobile' | 'steward';

/**
 * Deterministic test-only peer_id. Stage 1: not a valid libp2p PeerId
 * (suffix is hex, not base58btc) — opaque string at the projection layer.
 *
 * Same (humanId, archetype) → same peer_id across re-runs. The 12D3KooW prefix
 * mimics libp2p shape for ergonomic logs only.
 *
 * **Single source of truth for the peer_id formula.** Both seed-agent-bindings
 * and seed-commitments must import this — drift means SQL view predicates
 * silently return empty.
 */
export function deterministicPeerId(humanId: string, archetype: Archetype): string {
  const digest = createHash('sha256')
    .update(`${humanId}:${archetype}`, 'utf8')
    .digest();
  // Lowercase hex suffix — deterministic, alphanumeric, and clearly not a
  // real libp2p PeerId. Stage 1 stores peer_id as an opaque string; Stage 2
  // resolves the real id from the live storage pod (resolvePeerId below).
  // Sliced to 38 chars to keep total length stable (12D3KooW + 38 = 46 chars).
  return `12D3KooW${digest.toString('hex').slice(0, 38)}`;
}

// =============================================================================
// Stage 2 — real peer-id resolution from live storage pods
// =============================================================================

/**
 * Default per-human storage URL template. Mirrors how genesis/Jenkinsfile
 * builds genesis-peer storage URLs (`elohim-<name>-alpha.elohim-alpha.svc.
 * cluster.local:8090`): `{name}` is the human's short name — the first
 * segment of the humanId after the `human-` prefix (`human-matthew-manager`
 * → `matthew`). The seeder runs from the jenkins namespace, which can reach
 * port 8090 on these cluster-local services. Override the whole template via
 * PEER_STORAGE_URL_TEMPLATE for non-alpha environments.
 */
const DEFAULT_STORAGE_URL_TEMPLATE =
  'http://elohim-{name}-alpha.elohim-alpha.svc.cluster.local:8090';

/** Default timeout for one /p2p/status probe. */
const DEFAULT_RESOLVE_TIMEOUT_MS = 5000;

/**
 * Storage-pod base URL for a human. Resolution order:
 *   1. PEER_STORAGE_URLS — the `name=host:port` CSV convention shared with
 *      substrate-verify / seed-provide-rows / the Jenkinsfile's
 *      peerStorageUrlsCsv(). Ports may differ per peer (the local hc-mesh
 *      runs matthew=8090, jessica=8091, james=8092), which a single {name}
 *      template cannot express.
 *   2. PEER_STORAGE_URL_TEMPLATE — `{name}`-substituted template.
 *   3. The alpha cluster-local default.
 */
export function storageUrlForHuman(humanId: string): string {
  const shortName = humanId.replace(/^human-/, '').split('-')[0];
  const csv = process.env.PEER_STORAGE_URLS;
  if (csv) {
    for (const entry of csv.split(',')) {
      const trimmed = entry.trim();
      const eq = trimmed.indexOf('=');
      if (eq <= 0) continue;
      if (trimmed.slice(0, eq) === shortName) {
        const hostPort = trimmed.slice(eq + 1);
        return hostPort.includes('://') ? hostPort : `http://${hostPort}`;
      }
    }
  }
  const template = process.env.PEER_STORAGE_URL_TEMPLATE || DEFAULT_STORAGE_URL_TEMPLATE;
  return template.replaceAll('{name}', shortName);
}

/**
 * Minimal structural fetch shape — exactly what the resolver calls. Global
 * fetch satisfies it; tests can hand in a plain async mock without casts.
 */
export type FetchLike = (url: string, init?: { signal?: AbortSignal }) => Promise<Response>;

export interface ResolvePeerIdOptions {
  /** Injection point for tests; defaults to global fetch. */
  fetchImpl?: FetchLike;
  /** Per-probe timeout (ms). Default 5000. */
  timeoutMs?: number;
  /** Explicit storage base URL — overrides storageUrlForHuman(humanId). */
  storageUrl?: string;
}

/**
 * Per-host caches, scoped to one seeder run. Caching the *promise* (including
 * a null/failed outcome) guarantees every resolution against a host inside a
 * run sees the SAME answer — load-bearing for seed-commitments, where the
 * content-addressed commitment id is derived from the peer ids and the seed
 * and activate phases must compute identical ids.
 */
const peerIdCache = new Map<string, Promise<string | null>>();
const failureReasons = new Map<string, string>();
const warnedHosts = new Set<string>();

/** Reset the per-run caches (exported for tests). */
export function clearPeerIdCache(): void {
  peerIdCache.clear();
  failureReasons.clear();
  warnedHosts.clear();
}

async function fetchPeerIdOnce(
  storageUrl: string,
  fetchImpl: FetchLike,
  timeoutMs: number
): Promise<string | null> {
  const endpoint = `${storageUrl.replace(/\/+$/, '')}/p2p/status`;
  try {
    const response = await fetchImpl(endpoint, { signal: AbortSignal.timeout(timeoutMs) });
    if (!response.ok) {
      failureReasons.set(storageUrl, `HTTP ${response.status}`);
      return null;
    }
    // Wire casing: P2PStatusInfo is #[serde(rename_all = "camelCase")]
    // (elohim/elohim-storage/src/p2p/mod.rs) — `peer_id` serializes as `peerId`.
    const json = (await response.json()) as { peerId?: unknown };
    if (typeof json.peerId === 'string' && json.peerId.length > 0) {
      return json.peerId;
    }
    failureReasons.set(storageUrl, 'response missing peerId field');
    return null;
  } catch (err) {
    failureReasons.set(storageUrl, err instanceof Error ? err.message : String(err));
    return null;
  }
}

/**
 * Fetch the REAL libp2p peer id from a storage pod's /p2p/status, cached per
 * host within the run. Returns null on any failure (timeout, non-2xx, missing
 * field) — callers decide the fallback.
 */
export async function fetchRealPeerId(
  storageUrl: string,
  opts: ResolvePeerIdOptions = {}
): Promise<string | null> {
  let cached = peerIdCache.get(storageUrl);
  if (!cached) {
    cached = fetchPeerIdOnce(
      storageUrl,
      opts.fetchImpl ?? fetch,
      opts.timeoutMs ?? DEFAULT_RESOLVE_TIMEOUT_MS
    );
    peerIdCache.set(storageUrl, cached);
  }
  return cached;
}

/**
 * Stage 2 resolver: the human's REAL libp2p peer id from its live storage
 * pod, falling back to the Stage-1 deterministic id (loudly) when the pod is
 * unreachable. Use this for every peer_id that elohim-storage will compare
 * against a node's self_cid (custody-blob provider/receiver) — the custody
 * sweep only kicks blob fetches when `commitment.provider` equals the node's
 * real peer id.
 */
export async function resolvePeerId(
  humanId: string,
  archetype: Archetype,
  opts: ResolvePeerIdOptions = {}
): Promise<string> {
  const storageUrl = opts.storageUrl ?? storageUrlForHuman(humanId);
  const realPeerId = await fetchRealPeerId(storageUrl, opts);
  if (realPeerId) return realPeerId;

  if (!warnedHosts.has(storageUrl)) {
    warnedHosts.add(storageUrl);
    const reason = failureReasons.get(storageUrl) ?? 'unknown error';
    console.warn(
      `[peer-id] WARN: could not fetch real libp2p peerId from ${storageUrl} ` +
        `(${reason}; first hit: ${humanId}) — falling back to the deterministic ` +
        `Stage-1 id. Custody sweep will not match this peer: elohim-storage only ` +
        `kicks blob fetches when commitment.provider equals the node's REAL peer id.`
    );
  }
  return deterministicPeerId(humanId, archetype);
}
