/**
 * Typed HTTP probes for the P2P dataplane surfaces.
 *
 * Each function accepts a base URL and returns a typed result. All probes are
 * pure HTTP — no browser dependency — so they run in API mode without a Playwright
 * device. Step definitions in steps/dataplane.steps.ts delegate here.
 *
 * Surface coverage:
 *   /health                         → doorway HealthSurface
 *   /sync/v1/{hAppId}/docs          → SyncDocListSurface
 *   /sync/v1/{hAppId}/docs/{id}/heads → SyncDocHeadsSurface
 *   /blob/{hash}                    → HTTP status (200 present / 404 missing)
 *   /db/content/{id}                → ContentItemSurface (blobHash field)
 *   /p2p/status                     → P2PStatusSurface (direct storage access)
 *   /api/v1/epr/{id}/nav-context    → raw JSON
 *   /api/v1/diagnostics/inventory-parity → raw JSON
 *   /api/v1/status/arc-policy       → raw JSON
 *   /metrics                        → ParsedMetrics (Prometheus text parse)
 *
 * Peer resolution:
 *   'alpha-A'    → E2E_DOORWAY_ALPHA (default https://doorway-alpha.elohim.host)
 *   'elohim.host' → https://elohim.host
 *   'shem'       → E2E_SHEM_HOST (must be set; throws if absent)
 *   <other>      → process.env[name] if set, else throw
 *
 * NOTE: /p2p/status is NOT proxied by doorway (SPA fallthrough on alpha).
 * To probe it on alpha, set E2E_STORAGE_ALPHA to the direct storage URL.
 * The step layer returns 'pending' when the storage URL is unavailable.
 *
 * Source-of-truth Rust types:
 *   doorway /health     → doorway/doorway-service/src/routes/health.rs HealthResponse
 *   /sync/v1/*          → elohim/elohim-storage/src/http.rs handle_sync_*
 *   /p2p/status         → elohim/elohim-storage/src/p2p/mod.rs P2PStatusInfo
 *                         (all fields serialised with #[serde(rename_all = "camelCase")])
 */

import { request } from 'undici';

// ---------------------------------------------------------------------------
// Peer resolution
// ---------------------------------------------------------------------------

export type PeerAlias = 'alpha-A' | 'elohim.host' | 'shem';

/**
 * Resolve a peer name to its base URL.
 *
 * Accepts well-known aliases (alpha-A, elohim.host, shem) or env-var names.
 * For alpha-A, falls back to https://doorway-alpha.elohim.host when the env
 * var is not set — allowing local runs against the public alpha endpoint.
 */
export function resolvePeerUrl(peerName: string): string {
  switch (peerName) {
    case 'alpha-A':
      return process.env['E2E_DOORWAY_ALPHA'] ?? 'https://doorway-alpha.elohim.host';
    case 'elohim.host':
      return 'https://elohim.host';
    case 'shem': {
      const url = process.env['E2E_SHEM_HOST'];
      if (!url) throw new Error('E2E_SHEM_HOST not set — cannot resolve shem peer');
      return url;
    }
    default: {
      // Try direct env var lookup (e.g. 'E2E_DOORWAY_ALPHA' passed as peer name)
      const fromEnv = process.env[peerName];
      if (fromEnv) return fromEnv;
      throw new Error(
        `Unknown peer: "${peerName}". Known aliases: alpha-A, elohim.host, shem. ` +
          `Set process.env["${peerName}"] to use it as a URL.`
      );
    }
  }
}

/**
 * Resolve the direct elohim-storage URL for a peer.
 * Only available when E2E_STORAGE_ALPHA (or similar) is set; used for
 * /p2p/status and storage /metrics which are NOT proxied by doorway.
 */
export function resolveStorageUrl(peerName: string): string | null {
  if (peerName === 'alpha-A') {
    return process.env['E2E_STORAGE_ALPHA'] ?? null;
  }
  return process.env[`E2E_STORAGE_${peerName.toUpperCase().replace(/[^A-Z0-9]/g, '_')}`] ?? null;
}

// ---------------------------------------------------------------------------
// Common wire types
// ---------------------------------------------------------------------------

export interface ProbeResult<T> {
  status: number;
  body: T;
}

// ---------------------------------------------------------------------------
// /health surface types
// ---------------------------------------------------------------------------

/** From doorway health.rs P2PHealth #[serde(rename_all = "camelCase")] */
export interface HealthP2P {
  enabled: boolean;
  peerCount: number;
  peerId?: string;
  /** Projection-reconcile caught-up flag (from projectionReconcile state) */
  caughtUp?: boolean;
  /** Anchors that diverged during reconcile */
  divergentAnchor?: number;
}

/** From doorway health.rs ProjectionRole */
export interface HealthProjection {
  writer: boolean;
}

/**
 * From doorway health.rs ConductorHealth.
 * NOTE: ConductorHealth lacks #[serde(rename_all = "camelCase")] unlike its sibling
 * P2PHealth/ProjectionRole structs — so wire fields are snake_case, not camelCase.
 */
export interface HealthConductor {
  connected: boolean;
  connected_workers: number;
  total_workers: number;
  pool_size: number;
  pools_healthy: number;
  pools_total: number;
}

/**
 * Shape of the doorway GET /health response.
 * Source: doorway/doorway-service/src/routes/health.rs HealthResponse.
 * Note: node_id is serialised as "node_id" (no rename attr on that field).
 */
export interface HealthSurface {
  healthy: boolean;
  status: string;
  registrationOpen: boolean;
  version: string;
  uptime: number;
  cacheEnabled: boolean;
  conductor: HealthConductor;
  projection: HealthProjection;
  p2p?: HealthP2P;
  discoveryComplete: boolean;
  error?: string;
}

// ---------------------------------------------------------------------------
// /sync/v1/* surface types
// ---------------------------------------------------------------------------

/** One document entry from GET /sync/v1/{hAppId}/docs */
export interface SyncDocEntry {
  docId: string;
  docType: string;
  changeCount: number;
  lastModified: string;
  heads: string[];
}

/** Response from GET /sync/v1/{hAppId}/docs */
export interface SyncDocListSurface {
  hAppId: string;
  documents: SyncDocEntry[];
  total: number;
  offset: number;
  limit: number;
}

/** Response from GET /sync/v1/{hAppId}/docs/{docId}/heads */
export interface SyncDocHeadsSurface {
  hAppId: string;
  docId: string;
  heads: string[];
}

// ---------------------------------------------------------------------------
// /p2p/status surface types
// ---------------------------------------------------------------------------

/**
 * Pull-queue rollup from acquisition.rs PullStatusInfo
 * #[serde(rename_all = "camelCase")]
 */
export interface P2PPullStatus {
  total: number;
  fetched: number;
  pending: number;
  failed: number;
  caughtUp: boolean;
}

/**
 * P2P status from elohim-storage GET /p2p/status.
 * Source: elohim/elohim-storage/src/p2p/mod.rs P2PStatusInfo
 * All fields use #[serde(rename_all = "camelCase")].
 * Not all fields are listed — only the ones the step library asserts on.
 */
export interface P2PStatusSurface {
  peerId: string;
  connectedPeers: number;
  syncDocuments: number;
  natStatus: string;
  reconcilePassesTotal: number;
  kicksFiredTotal: number;
  placementGapsEmittedTotal: number;
  dedupUniqueLen: number;
  dedupTotalSeen: number;
  pull?: P2PPullStatus;
  syncPaused: boolean;
  [key: string]: unknown;
}

// ---------------------------------------------------------------------------
// /db/content/{id} surface types
// ---------------------------------------------------------------------------

/** Subset of the content DB response — the blobHash field the step library checks */
export interface ContentItemSurface {
  id: string;
  /** Present and non-null when the content has an associated blob (sha256-… format) */
  blobHash?: string | null;
  [key: string]: unknown;
}

// ---------------------------------------------------------------------------
// /metrics surface types (Prometheus text format)
// ---------------------------------------------------------------------------

/** Flat map of metric_name → current numeric value */
export type ParsedMetrics = Map<string, number>;

/**
 * Parse Prometheus text-format metrics into a flat name→value map.
 * Takes only the first value seen for any metric name (handles label variants
 * by picking the first labelled series).
 * Lines starting with '#' are skipped (comments / type/help lines).
 */
export function parsePrometheusMetrics(text: string): ParsedMetrics {
  const metrics = new Map<string, number>();
  for (const rawLine of text.split('\n')) {
    const line = rawLine.trim();
    if (line.startsWith('#') || line === '') continue;

    // Format: `metric_name[{labels}] value [timestamp]`
    // Find the first space after the label-set close (or after the metric name when no labels),
    // then take only the first whitespace-separated token from the remainder as the value.
    // lastIndexOf(' ') is wrong here: when a line has a trailing timestamp it picks the
    // timestamp token instead of the value token.
    const braceEnd = line.indexOf('}');
    const searchFrom = braceEnd === -1 ? 0 : braceEnd + 1;
    const firstSpace = line.indexOf(' ', searchFrom);
    if (firstSpace === -1) continue;

    const nameAndLabels = line.slice(0, firstSpace);
    // Take only the first whitespace-separated token after metric+labels; optional timestamp ignored.
    const valueStr = line
      .slice(firstSpace + 1)
      .trimStart()
      .split(/\s+/)[0];

    // Strip label set to get the base metric name
    const braceIdx = nameAndLabels.indexOf('{');
    const metricName = braceIdx === -1 ? nameAndLabels : nameAndLabels.slice(0, braceIdx);

    const value = Number.parseFloat(valueStr);
    if (!Number.isNaN(value) && !metrics.has(metricName)) {
      metrics.set(metricName, value);
    }
  }
  return metrics;
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

/** Raw GET — returns status + response body text. Never throws on non-2xx. */
export async function getRaw(url: string): Promise<{ status: number; text: string }> {
  const { statusCode, body } = await request(url, {
    method: 'GET',
    // 15 s timeout — remote alpha peers can be slow
    bodyTimeout: 15_000,
    headersTimeout: 15_000,
  });
  const text = await body.text();
  return { status: statusCode, text };
}

/** GET + JSON parse. Throws on non-2xx status codes. */
async function getJson<T>(url: string): Promise<ProbeResult<T>> {
  const { status, text } = await getRaw(url);
  if (status < 200 || status >= 300) {
    throw new Error(`GET ${url} returned ${status}: ${text.slice(0, 200)}`);
  }
  return { status, body: JSON.parse(text) as T };
}

// ---------------------------------------------------------------------------
// Surface probe functions
// ---------------------------------------------------------------------------

/**
 * GET /health on the given peer base URL.
 * The peer URL is typically the doorway URL (alpha-A = E2E_DOORWAY_ALPHA).
 */
export async function probeHealth(peerUrl: string): Promise<ProbeResult<HealthSurface>> {
  return getJson<HealthSurface>(`${peerUrl}/health`);
}

/**
 * GET /sync/v1/{hAppId}/docs on the given peer.
 * The sync API is proxied by doorway (post-2026-06-27 manifest flip).
 */
export async function probeSyncDocs(
  peerUrl: string,
  hAppId: string
): Promise<ProbeResult<SyncDocListSurface>> {
  return getJson<SyncDocListSurface>(`${peerUrl}/sync/v1/${encodeURIComponent(hAppId)}/docs`);
}

/**
 * GET /sync/v1/{hAppId}/docs/{docId}/heads on the given peer.
 */
export async function probeSyncDocHeads(
  peerUrl: string,
  hAppId: string,
  docId: string
): Promise<ProbeResult<SyncDocHeadsSurface>> {
  return getJson<SyncDocHeadsSurface>(
    `${peerUrl}/sync/v1/${encodeURIComponent(hAppId)}/docs/${encodeURIComponent(docId)}/heads`
  );
}

/**
 * GET /blob/{hash} on the given peer.
 * Returns the HTTP status code only — 200 means present, 404 means missing.
 * Never throws; errors surface as 5xx status codes.
 */
export async function probeBlob(peerUrl: string, hash: string): Promise<number> {
  try {
    const { status } = await getRaw(`${peerUrl}/blob/${encodeURIComponent(hash)}`);
    return status;
  } catch {
    return 0; // network error
  }
}

/**
 * GET /db/content/{id} on the given peer.
 */
export async function probeContent(
  peerUrl: string,
  contentId: string
): Promise<ProbeResult<ContentItemSurface>> {
  return getJson<ContentItemSurface>(`${peerUrl}/db/content/${encodeURIComponent(contentId)}`);
}

/**
 * GET /p2p/status on the given STORAGE URL (not doorway URL).
 * This endpoint is NOT proxied by doorway — callers must supply the direct
 * storage base URL (e.g. from E2E_STORAGE_ALPHA).
 */
export async function probeP2PStatus(storageUrl: string): Promise<ProbeResult<P2PStatusSurface>> {
  return getJson<P2PStatusSurface>(`${storageUrl}/p2p/status`);
}

/**
 * GET /api/v1/epr/{id}/nav-context.
 * Proxied by doorway; use the doorway peer URL.
 */
export async function probeEprNavContext(
  peerUrl: string,
  eprId: string
): Promise<ProbeResult<Record<string, unknown>>> {
  return getJson<Record<string, unknown>>(
    `${peerUrl}/api/v1/epr/${encodeURIComponent(eprId)}/nav-context`
  );
}

/**
 * GET /api/v1/diagnostics/inventory-parity.
 * Proxied by doorway; use the doorway peer URL.
 */
export async function probeInventoryParity(
  peerUrl: string
): Promise<ProbeResult<Record<string, unknown>>> {
  return getJson<Record<string, unknown>>(`${peerUrl}/api/v1/diagnostics/inventory-parity`);
}

/**
 * GET /api/v1/status/arc-policy.
 * Proxied by doorway; use the doorway peer URL.
 */
export async function probeArcPolicy(
  peerUrl: string
): Promise<ProbeResult<Record<string, unknown>>> {
  return getJson<Record<string, unknown>>(`${peerUrl}/api/v1/status/arc-policy`);
}

/**
 * GET /metrics from the given base URL.
 * Doorway metrics are at port 8080 of the doorway host; storage metrics are
 * at the default storage port. Both serve Prometheus text format.
 *
 * For doorway metrics: derive the metrics URL by replacing the scheme/port
 * of the doorway URL (caller responsibility, or pass the metrics base URL directly).
 * For storage metrics: pass the E2E_STORAGE_ALPHA URL directly.
 */
export async function probeMetrics(metricsBaseUrl: string): Promise<ParsedMetrics> {
  const { status, text } = await getRaw(`${metricsBaseUrl}/metrics`);
  if (status !== 200) {
    throw new Error(`GET ${metricsBaseUrl}/metrics returned ${status}`);
  }
  return parsePrometheusMetrics(text);
}
