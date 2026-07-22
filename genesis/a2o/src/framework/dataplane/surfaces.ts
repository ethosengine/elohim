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
 *   /db/p2p/conductor-diagnostics   → ConductorDiagnosticsSurface (live agent set)
 *   /health/startup (→ /health fallback) → ServedHeadProbeResult (T4-1 servedBundleHeads
 *                                     attestation — Track-4 T4-2's "served" side, compared
 *                                     against ContentItemSurface.serverBlobHash, the "declared" side)
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
 * One entry from the T4-1 `servedBundleHeads[]` attestation (Track-4 T4-2
 * contract — deployed in parallel with this file; written to the CONTRACT,
 * not to whatever doorway code exists at write-time). Reports what the
 * RUNNING doorway process has actually materialized, distinct from the
 * content row's DECLARED head (`/db/content/{slug}.serverBlobHash` —
 * ContentItemSurface below). Absence of this field, or of an entry for a
 * given slug, means the T4-1 attestation is not deployed on that host yet —
 * see probeServedBundleHead()'s forward-compatible SKIP semantics.
 */
export interface ServedBundleHead {
  slug: string;
  serverBlobHash?: string;
  materializedAt?: string;
  status?: 'current' | 'stale' | 'refreshing' | 'failed' | string;
  declaredServerBlobHash?: string;
  [key: string]: unknown;
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
  /** T4-1 served-bundle-head attestation (see ServedBundleHead) — carried by
   *  /health/startup and, potentially, /health. Absent until T4-1 deploys. */
  servedBundleHeads?: ServedBundleHead[];
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
  /**
   * The DECLARED server (SSR) bundle hash — elohim-storage/src/ssr.rs
   * parse_server_blob_hash() reads this exact field from the same GET
   * /db/content/{slug} response. This is the "declared" side of the
   * served-vs-declared T4-2 comparison; ServedBundleHead.serverBlobHash
   * (from the health surface) is the "served" side.
   */
  serverBlobHash?: string | null;
  [key: string]: unknown;
}

// ---------------------------------------------------------------------------
// /db/p2p/conductor-diagnostics surface types
// ---------------------------------------------------------------------------

/**
 * One entry from GET /db/p2p/conductor-diagnostics `agents[]` — a projection
 * of an AgentInfoSigned the embedded conductor's peer store currently holds.
 * Source: elohim/elohim-storage/src/http.rs `project_agent_info`.
 *
 * IMPORTANT: `agent` is the RAW base64 core of the Holochain AgentPubKey —
 * it does NOT carry the `uhCAk` multibase prefix that `humans.agentPubKey`
 * (HumanView) carries. `uhCAk` = the multibase 'u' marker + the base64
 * encoding of the 3-byte Agent type-prefix; the conductor's admin API returns
 * only the remaining hash+loc bytes. Matching a `humans.agentPubKey` against
 * a diagnostics `agent` therefore needs `agentKeyMatchesDiagnosticAgent()`
 * below, never raw string equality.
 */
export interface ConductorDiagnosticsAgentEntry {
  agent: string | null;
  space?: string | null;
  url?: string | null;
  createdAt?: string | number | null;
  expiresAt?: string | number | null;
  isTombstone?: boolean | null;
  storageArc?: unknown;
  [key: string]: unknown;
}

/** Response from GET /db/p2p/conductor-diagnostics */
export interface ConductorDiagnosticsSurface {
  agentCount: number;
  agents: ConductorDiagnosticsAgentEntry[];
  transportStats?: unknown;
  networkMetrics?: unknown;
  [key: string]: unknown;
}

/**
 * Decode a base64/base64url string to bytes, tolerating either alphabet and
 * missing '=' padding. Returns null on anything that is not clean base64 —
 * a decode that silently drops invalid characters would defeat the byte-exact
 * comparison below.
 */
function base64ishToBytes(s: string): Buffer | null {
  // Unify to the url-safe alphabet; '=' is exclusively trailing padding, so a
  // global strip is equivalent to trimming the tail.
  const normalized = s.replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
  if (normalized.length === 0 || !/^[\w-]+$/.test(normalized)) return null;
  return Buffer.from(normalized, 'base64url');
}

/**
 * True when a `humans.agentPubKey` (uhCAk-prefixed multibase form) refers to
 * the SAME live agent as a conductor-diagnostics `agent` entry (raw base64 of
 * the 32-byte key core). String containment can NEVER establish this — the two
 * encodings are byte-misaligned:
 *
 *   humans.agentPubKey  = 'u' + base64url(0x84 0x20 0x24 ‖ core[32] ‖ loc[4])
 *   diagnostics agent   =       base64url(core[32])
 *
 * The 3-byte holo type-prefix shifts every subsequent base64 character on the
 * humans side, and the diagnostics string's LAST character encodes only the
 * core's final bits (zero-padded at the 32-byte boundary) while the humans
 * string continues into the 4 DHT-location bytes — so even the shared 32-byte
 * core yields divergent character tails. Worked example (real alpha keys):
 *
 *   human uhCAkQte6fxZXuJtHlLBb8L87RjsVdKimUsQhdYVAMMLGZG2bt69n
 *              → strip 'u', base64url-decode → 39 bytes; bytes[3..35] = core
 *   diag        Qte6fxZXuJtHlLBb8L87RjsVdKimUsQhdYVAMMLGZG0
 *              → base64url-decode → exactly those 32 core bytes
 *   (tails diverge: …GZG0 vs …GZG2bt69n — string containment is FALSE while
 *   byte equality of the core HOLDS)
 *
 * So: decode BOTH sides and compare bytes — humanBytes[3..35] === diagBytes.
 * Unit-pinned with the real key pair above in
 * src/framework/dataplane/__tests__/surfaces.test.ts.
 */
export function agentKeyMatchesDiagnosticAgent(
  humanAgentPubKey: string,
  diagnosticAgent: string
): boolean {
  if (!humanAgentPubKey || !diagnosticAgent) return false;
  // Multibase: leading 'u' = base64url-no-padding. Strip ONLY the marker —
  // the 3-byte holo type-prefix is part of the decoded payload, not the string.
  const humanB64 = humanAgentPubKey.startsWith('u') ? humanAgentPubKey.slice(1) : humanAgentPubKey;
  const humanBytes = base64ishToBytes(humanB64);
  const diagBytes = base64ishToBytes(diagnosticAgent);
  if (!humanBytes || !diagBytes) return false;
  // AgentPubKey = 3-byte type prefix + 32-byte core + 4-byte DHT location (39
  // bytes); diagnostics carries exactly the 32-byte core. Guard both shapes
  // before slicing so a malformed value can never alias into a false match.
  if (humanBytes.length < 35 || diagBytes.length !== 32) return false;
  return humanBytes.subarray(3, 35).equals(diagBytes);
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
// Path-segment encoding helpers
// ---------------------------------------------------------------------------

/**
 * Encode a doc/entity id for use in a URL path segment. Unlike encodeURIComponent,
 * this preserves ':' because doc ids use a 'type:uuid' namespace syntax and the
 * Rust storage handler receives the raw path segment (the colon is not decoded by
 * the Axum extractor — percent-encoding '%3A' and a literal ':' are treated as
 * different keys). Every other special character is still percent-encoded.
 */
function encodeDocId(id: string): string {
  return encodeURIComponent(id).replace(/%3A/gi, ':');
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

/**
 * Raw POST — returns status + response body text. Never throws on non-2xx.
 * Unauthenticated by default (no auth header): a caller that presents no
 * identity is, definitionally, a non-author — the exact "any peer on the wire"
 * case the notary must refuse. Used by the notary-authority concern's guarded
 * HEAD-move refusal probe. Sends an optional JSON body.
 */
export async function postRaw(
  url: string,
  jsonBody?: unknown
): Promise<{ status: number; text: string }> {
  const hasBody = jsonBody !== undefined;
  const { statusCode, body } = await request(url, {
    method: 'POST',
    headers: hasBody ? { 'content-type': 'application/json' } : {},
    body: hasBody ? JSON.stringify(jsonBody) : undefined,
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
 * docId is encoded with encodeDocId() which preserves ':' (namespace separator).
 */
export async function probeSyncDocHeads(
  peerUrl: string,
  hAppId: string,
  docId: string
): Promise<ProbeResult<SyncDocHeadsSurface>> {
  return getJson<SyncDocHeadsSurface>(
    `${peerUrl}/sync/v1/${encodeURIComponent(hAppId)}/docs/${encodeDocId(docId)}/heads`
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
 * contentId is encoded with encodeDocId() which preserves ':' (namespace separator).
 */
export async function probeContent(
  peerUrl: string,
  contentId: string
): Promise<ProbeResult<ContentItemSurface>> {
  return getJson<ContentItemSurface>(`${peerUrl}/db/content/${encodeDocId(contentId)}`);
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
 * GET /db/p2p/conductor-diagnostics on the given peer.
 * Proxied by doorway (same class as other /db/p2p/* diagnostic routes); use
 * the doorway peer URL. Returns 503 when the peer's embedded conductor admin
 * connection is unavailable — callers should treat that as "not observable"
 * rather than a hard failure (mirrors the /p2p/status pending convention).
 */
export async function probeConductorDiagnostics(
  peerUrl: string
): Promise<ProbeResult<ConductorDiagnosticsSurface>> {
  return getJson<ConductorDiagnosticsSurface>(`${peerUrl}/db/p2p/conductor-diagnostics`);
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

// ---------------------------------------------------------------------------
// Served-vs-declared projected-head probe (Track-4 T4-2)
// ---------------------------------------------------------------------------

/** Result of probing a peer for its T4-1 served-bundle-head attestation. */
export interface ServedHeadProbeResult {
  /** Which health surface answered 200, or 'unreachable' if neither did. */
  source: 'startup' | 'health' | 'unreachable';
  /** True once ANY health surface responded 200 (independent of whether the
   *  servedBundleHeads field, or an entry for the requested slug, was present). */
  reachable: boolean;
  /**
   * The matching servedBundleHeads[] entry for the requested slug, or
   * undefined when the T4-1 attestation is not deployed on this peer yet
   * (servedBundleHeads absent, or no entry for this slug). Forward-compatible:
   * callers must treat `entry === undefined` as a SKIP, not a failure —
   * mirrors scripts/ci/verify-projected-head.sh's FIELD-ABSENT semantics.
   */
  entry?: ServedBundleHead;
}

/** GET a URL and parse as JSON, never throwing — non-200 or invalid JSON both yield a null body. */
async function tryHealthSurface(
  url: string
): Promise<{ status: number; body: HealthSurface | null }> {
  try {
    const { status, text } = await getRaw(url);
    if (status !== 200) return { status, body: null };
    try {
      return { status, body: JSON.parse(text) as HealthSurface };
    } catch {
      return { status, body: null };
    }
  } catch {
    return { status: 0, body: null };
  }
}

/**
 * Probe the T4-1 served-bundle-head attestation for one slug on a peer.
 * Tries GET /health/startup first, falling back to GET /health — mirrors
 * the CI probe (scripts/ci/verify-projected-head.sh) exactly, so the same
 * "servedBundleHeads absent = not yet deployed, not broken" semantics hold
 * in both the a2o world and CI. Never throws.
 */
export async function probeServedBundleHead(
  peerUrl: string,
  slug: string
): Promise<ServedHeadProbeResult> {
  const startup = await tryHealthSurface(`${peerUrl}/health/startup`);
  if (startup.body) {
    return {
      source: 'startup',
      reachable: true,
      entry: startup.body.servedBundleHeads?.find(h => h.slug === slug),
    };
  }
  const health = await tryHealthSurface(`${peerUrl}/health`);
  if (health.body) {
    return {
      source: 'health',
      reachable: true,
      entry: health.body.servedBundleHeads?.find(h => h.slug === slug),
    };
  }
  return { source: 'unreachable', reachable: false, entry: undefined };
}
