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
 *   /db/content/{id}/head           → DeclaredHeadResult (notary HEAD: headActionHash + declared)
 *   / (+ /health, /status.json)     → classifyDoorwayState() DoorwayState ('serving' |
 *                                     'shedding' | 'dead') — doorway-failover concern
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
      // E2E_DOORWAY_B is already CI vocabulary (run-dataplane-validation.sh
      // passes it to the quiesce gate); wiring it here lets a local mesh
      // stand in a second doorway instead of every B-leg probing the LIVE
      // production doorway from the desk. Default unchanged.
      return process.env['E2E_DOORWAY_B'] ?? 'https://elohim.host';
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
 *
 * alpha-A also falls back to E2E_STORAGE_URL — the generic "the" storage var
 * several OTHER step files already read directly (resilience.steps.ts,
 * delivery-admin.steps.ts, acquisition-pins.steps.ts) — because the
 * Dataplane Validation stage (scripts/ci/run-dataplane-validation.sh) sets
 * ONLY E2E_STORAGE_URL (default: matthew-alpha's storage svc — matthew being
 * alpha-A's genesis/author peer), never the peer-alias-scoped
 * E2E_STORAGE_ALPHA. Without this fallback every alpha-A gauge-metric
 * assertion that routes through here (resiliency-saga chapters 2/7/8) hit
 * the "storage metrics URL not set" pending BEFORE ever reaching a probe —
 * an env-wiring gap, not the sweep-cadence timing this function's callers
 * were previously assumed to be racing.
 */
export function resolveStorageUrl(peerName: string): string | null {
  if (peerName === 'alpha-A') {
    return process.env['E2E_STORAGE_ALPHA'] ?? process.env['E2E_STORAGE_URL'] ?? null;
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

/**
 * Parse a SINGLE labelled Prometheus series matching `series{...labelKey="labelValue"...}`,
 * summing across any other label dimensions present on matching lines (mirrors
 * `scripts/look-doorway-membrane.ts`'s `readCounter` — ported here as a sibling of
 * `parsePrometheusMetrics` so a caller can pin one label combination instead of the
 * arbitrary first-seen value `parsePrometheusMetrics`' bare-name collapse returns for
 * a `*Vec` metric with multiple label sets).
 *
 * Returns `null` when the series name never appears in the scrape (metric not
 * emitted at all) OR appears but no line carries the requested label pair (that
 * exact label value has never been observed) — both are "absent", not "zero";
 * callers must treat `null` as pending/skip, never as a measured 0.
 */
export function parseLabeledPrometheusMetric(
  text: string,
  series: string,
  labelKey: string,
  labelValue: string
): number | null {
  let total: number | null = null;
  for (const rawLine of text.split('\n')) {
    const line = rawLine.trim();
    if (line.startsWith('#') || !line.startsWith(series)) continue;
    // The line is either `series value` (unlabelled) or `series{...} value`; a
    // longer metric name sharing this prefix (e.g. `series_total` vs `series`)
    // must not falsely match — the char right after the prefix must be a space
    // or the label-set opener.
    const after = line.slice(series.length);
    if (after.length > 0 && !after.startsWith(' ') && !after.startsWith('{')) continue;
    if (!after.includes(`${labelKey}="${labelValue}"`)) continue;
    const value = Number(line.trim().split(/\s+/).pop());
    if (!Number.isNaN(value)) total = (total ?? 0) + value;
  }
  return total;
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

/**
 * Raw GET — returns status + response body text. Never throws on non-2xx (a
 * non-2xx status is still a real answer); DOES throw on connect error/timeout
 * (there is no status to report). `opts.timeoutMs` overrides the default 15s
 * remote-peer allowance — callers doing bounded, no-retry classification
 * (classifyDoorwayState) pass a tighter budget.
 */
export async function getRaw(
  url: string,
  opts: { timeoutMs?: number } = {}
): Promise<{ status: number; text: string }> {
  const timeoutMs = opts.timeoutMs ?? 15_000;
  const { statusCode, body } = await request(url, {
    method: 'GET',
    bodyTimeout: timeoutMs,
    headersTimeout: timeoutMs,
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

// ---------------------------------------------------------------------------
// Doorway live-state classification (doorway-failover concern)
// ---------------------------------------------------------------------------

export type DoorwayState = 'serving' | 'shedding' | 'dead';

/** Per-request bound for classifyDoorwayState's probes — no retries; callers own polling. */
export const CLASSIFY_TIMEOUT_MS = 10_000;

/**
 * GET a URL bounded to `timeoutMs`, collapsing ANY thrown error (connect
 * refused, DNS failure, timeout) to `null` rather than propagating —
 * classification needs to tell "answered, badly" apart from "never answered"
 * without a try/catch at every call site.
 */
async function tryGetRaw(
  url: string,
  timeoutMs: number
): Promise<{ status: number; text: string } | null> {
  try {
    return await getRaw(url, { timeoutMs });
  } catch {
    return null;
  }
}

/** Minimal shape of GET /status.json this classifier reads — see doorway-catching-up-page.feature. */
interface StatusJsonShape {
  upstreams?: { circuit?: string }[];
  admission?: { shedTotal?: number };
}

/** True when a 503's own body on `/` is the legacy non-browser shed JSON. */
function isCatchingUpBody(text: string): boolean {
  try {
    const body = JSON.parse(text) as Record<string, unknown>;
    return body['status'] === 'catching-up';
  } catch {
    return false; // browser path gets a staged HTML page, not JSON
  }
}

/** True when GET /status.json names an open upstream circuit or rising shedTotal. */
async function statusJsonShowsShedding(peerUrl: string): Promise<boolean> {
  const statusJson = await tryGetRaw(`${peerUrl}/status.json`, CLASSIFY_TIMEOUT_MS);
  if (statusJson?.status !== 200) return false;
  try {
    const body = JSON.parse(statusJson.text) as StatusJsonShape;
    const openCircuit = (body.upstreams ?? []).some(u => u.circuit && u.circuit !== 'closed');
    const shedTotal = body.admission?.shedTotal ?? 0;
    return openCircuit || shedTotal > 0;
  } catch {
    return false; // status.json unreachable/unparseable — caller falls back to the tie-break
  }
}

/**
 * Confirm WHY a 503 on / is shedding — checked in order, stopping at the
 * first confirmation (shape 1, then shape 2, then the /health tie-break).
 * Every confirmed-503 path IS shedding by definition (see the doc comment on
 * classifyDoorwayState for why 'dead' can never apply here), so this returns
 * void rather than DoorwayState: the caller's outcome is always 'shedding'
 * regardless of which shape confirmed it. Split out of classifyDoorwayState
 * purely to keep that function's cognitive complexity in budget.
 */
async function confirm503Cause(peerUrl: string, rootText: string): Promise<void> {
  if (isCatchingUpBody(rootText)) return;
  if (await statusJsonShowsShedding(peerUrl)) return;
  // Tie-break: neither shed shape confirmed — ask /health anyway. Its result
  // doesn't change the caller's outcome (still 'shedding' either way); the
  // request exists to make the alive-but-unconfirmed-cause case observable.
  await tryGetRaw(`${peerUrl}/health`, CLASSIFY_TIMEOUT_MS);
}

/**
 * Classify a doorway's live state from its two public HTTP surfaces. Bounded:
 * 10s per request (CLASSIFY_TIMEOUT_MS), no retries — callers own polling.
 *
 *   serving  — GET / answers 200 (the shell rendered normally).
 *
 *   shedding — GET / answers 503 AND the response identifies as the specified
 *              catching-up contract: either the / body itself is
 *              `{"status":"catching-up",...}` (the legacy non-browser shed
 *              JSON — spec 2026-07-19-doorway-catching-up-page-design), or
 *              GET /status.json succeeds and shows an upstream with
 *              `circuit !== 'closed'` or `admission.shedTotal > 0` (the
 *              browser path gets a staged HTML recovery page instead of JSON
 *              on `/`, so status.json is the only way to read the cause off
 *              that path).
 *
 *              TIE-BREAK: a 503 whose body matches NEITHER shape above (e.g.
 *              status.json itself unreachable, or reachable but showing no
 *              open circuit / shedTotal yet — an admission-layer shed with no
 *              upstream signal recorded) still counts as shedding, never
 *              dead, as long as GET /health answers 200 — the process is
 *              alive and intentionally refusing, which is a materially
 *              different state from an outage even when the exact cause
 *              can't be pinned. A 503 that fails even the tie-break also
 *              defaults to shedding rather than dead: GET / produced a real
 *              HTTP response (not a connect error/timeout), which already
 *              disqualifies 'dead' by the definition below — a process that
 *              answers ANYTHING on / is not silently gone.
 *
 *   dead     — connect error / timeout on BOTH / and /health — nothing
 *              answered at all.
 */
export async function classifyDoorwayState(peerUrl: string): Promise<DoorwayState> {
  const root = await tryGetRaw(`${peerUrl}/`, CLASSIFY_TIMEOUT_MS);

  if (root === null) {
    // GET / itself never answered — confirm total silence via /health before
    // calling it dead (health is the cheaper, more permissive probe).
    const health = await tryGetRaw(`${peerUrl}/health`, CLASSIFY_TIMEOUT_MS);
    return health === null ? 'dead' : 'shedding';
  }

  if (root.status === 200) return 'serving';
  if (root.status === 503) {
    await confirm503Cause(peerUrl, root.text);
    return 'shedding';
  }

  // Some other status this contract doesn't specify — / answered (so not
  // dead), but the answer isn't a clean 200 or the documented 503. Treat as
  // shedding: the caller's binary "not dead" contract holds either way.
  return 'shedding';
}

// ---------------------------------------------------------------------------
// Bounded doorway-ready wait (resiliency-saga chapter 4 — deploy-window race)
// ---------------------------------------------------------------------------

/**
 * The edge pipeline's Dataplane Validation stage runs immediately after
 * "Deploy Edge Node - Alpha", so a raw GET / issued right at stage start can
 * race the doorway pod restart — green live, red in-window (builds
 * #1257/#1262). `waitForDoorwayReady` absorbs exactly that restart window: it
 * polls GET /health every `intervalMs` until it reports `healthy: true`, up to
 * `timeoutMs` total.
 *
 * Measurement hardening ONLY — this must never mask a genuinely-down doorway.
 * On deadline expiry it returns `false` and the caller is expected to proceed
 * to its real assertion anyway, so a truly-broken doorway still fails
 * honestly (the wait only absorbs the KNOWN, bounded restart race, never
 * substitutes for the assertion). A non-200 status, a non-`healthy` body, or
 * a thrown network error are all treated as "not ready yet" and re-polled —
 * mirrors pollForGauge's never-throw-mid-poll contract.
 */
export const DOORWAY_READY_POLL_INTERVAL_MS = 3_000;
export const DOORWAY_READY_POLL_TIMEOUT_MS = 90_000;

export async function waitForDoorwayReady(
  peerUrl: string,
  opts: { intervalMs?: number; timeoutMs?: number } = {}
): Promise<boolean> {
  const ready = await pollForGauge<true>(
    async () => {
      const { status, body } = await probeHealth(peerUrl);
      if (status !== 200 || body?.healthy !== true) return undefined;
      return true;
    },
    {
      intervalMs: opts.intervalMs ?? DOORWAY_READY_POLL_INTERVAL_MS,
      timeoutMs: opts.timeoutMs ?? DOORWAY_READY_POLL_TIMEOUT_MS,
    }
  );
  return ready === true;
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

// ---------------------------------------------------------------------------
// Catching-up-riding GET (resiliency-saga chapter 3 — mid-run admission shed)
// ---------------------------------------------------------------------------

/**
 * The quiesce gate proves the fleet converged at STAGE START, but a doorway can
 * flap back into the catching-up admission shed mid-run (503 +
 * `{"status":"catching-up","retryAfter":30}` while /health stays healthy —
 * runbook "Content view sheds 503 catching-up", churn invariant I6; edge #1360
 * redded chapter 3 on exactly this). Content-materialization assertions measure
 * a different plane than admission, so they ride the shed bounded instead of
 * failing on the first refusal.
 *
 * Measurement hardening ONLY — same contract as waitForDoorwayReady: only the
 * documented catching-up shed is ridden (matched by its own body signature);
 * every other answer (200, 404, plain 503, connect error) propagates
 * immediately, and on deadline expiry the last catching-up 503 is returned so
 * the caller's assertion still fails honestly.
 */
export const CATCHUP_RIDE_TIMEOUT_MS = 90_000;
export const CATCHUP_RIDE_MAX_INTERVAL_MS = 15_000;

export interface RiddenRawResponse {
  status: number;
  text: string;
  /** Total ms spent waiting through catching-up sheds; absent when none was hit. */
  rodeCatchUpMs?: number;
}

/** Extract the shed body's retryAfter (seconds) as ms; undefined when absent/invalid. */
function parseRetryAfterMs(text: string): number | undefined {
  try {
    const retryAfter = (JSON.parse(text) as Record<string, unknown>)['retryAfter'];
    if (typeof retryAfter === 'number' && retryAfter > 0) return retryAfter * 1000;
  } catch {
    // fall through — caller uses the default interval
  }
  return undefined;
}

async function defaultSleep(ms: number): Promise<void> {
  await new Promise<void>(resolve => setTimeout(resolve, ms));
}

export async function getRawRidingCatchUp(
  url: string,
  opts: {
    timeoutMs?: number;
    /** Injection seams for unit tests — production callers omit all three. */
    fetchFn?: (
      url: string,
      fetchOpts?: { timeoutMs?: number }
    ) => Promise<{ status: number; text: string }>;
    sleepFn?: (ms: number) => Promise<void>;
    nowFn?: () => number;
  } = {}
): Promise<RiddenRawResponse> {
  const timeoutMs = opts.timeoutMs ?? CATCHUP_RIDE_TIMEOUT_MS;
  const fetchFn = opts.fetchFn ?? getRaw;
  const sleepFn = opts.sleepFn ?? defaultSleep;
  const nowFn = opts.nowFn ?? Date.now;

  // timeoutMs is a TRUE wall-clock bound: every per-fetch timeout and every
  // sleep is clamped to the remaining budget, so the worst case is
  // ~timeoutMs total, never timeoutMs + stragglers (the pre-fix shape let a
  // last-moment fetch + sleep run ~30s past the deadline, blowing derived
  // step ceilings). rodeCatchUpMs reports WALL time spent riding, not the
  // sum of sleeps, so operators sizing ceilings from logs see the truth.
  const start = nowFn();
  let rode = false;
  for (;;) {
    const remainingForFetch = timeoutMs - (nowFn() - start);
    const res = await fetchFn(url, {
      timeoutMs: Math.max(1_000, Math.min(CATCHUP_RIDE_MAX_INTERVAL_MS, remainingForFetch)),
    });
    const shedding = res.status === 503 && isCatchingUpBody(res.text);
    if (!shedding) {
      return rode ? { ...res, rodeCatchUpMs: nowFn() - start } : res;
    }
    rode = true;
    const remaining = timeoutMs - (nowFn() - start);
    if (remaining <= 0) {
      return { ...res, rodeCatchUpMs: nowFn() - start };
    }
    const waitMs = Math.min(
      parseRetryAfterMs(res.text) ?? CATCHUP_RIDE_MAX_INTERVAL_MS,
      CATCHUP_RIDE_MAX_INTERVAL_MS,
      remaining
    );
    await sleepFn(waitMs);
  }
}

/**
 * Step-ceiling for any cucumber step whose body makes ONE ridden read: the
 * ride cap + one bounded fetch + assertion margin. Derive step timeouts from
 * this instead of hand-typing milliseconds — cucumber's 30s default kills the
 * ride mid-flight otherwise (the edge #1360 review class).
 */
export const CATCHUP_RIDE_STEP_TIMEOUT_MS =
  CATCHUP_RIDE_TIMEOUT_MS + CATCHUP_RIDE_MAX_INTERVAL_MS + 30_000;

/**
 * The one shared phrasing for "this read rode the shed" — appended to
 * failure messages. Names the runbook class ONLY when the FINAL answer is
 * still the catching-up shed; a genuine 404/500 after a transient shed says
 * so without misrouting triage to the admission runbook.
 */
export function describeCatchUpRide(res: RiddenRawResponse): string {
  if (res.rodeCatchUpMs === undefined) return '';
  const secs = Math.round(res.rodeCatchUpMs / 1000);
  return res.status === 503 && isCatchingUpBody(res.text)
    ? ` (still catching-up after riding the admission shed for ${secs}s — runbook "Content view sheds 503 catching-up")`
    : ` (after a transient catching-up shed of ${secs}s)`;
}

/**
 * GET /db/content/{id} on the given peer.
 * contentId is encoded with encodeDocId() which preserves ':' (namespace separator).
 * Rides the bounded catching-up admission shed (getRawRidingCatchUp) — this
 * surface asserts content materialization, not admission state.
 */
export async function probeContent(
  peerUrl: string,
  contentId: string
): Promise<ProbeResult<ContentItemSurface>> {
  const url = `${peerUrl}/db/content/${encodeDocId(contentId)}`;
  const res = await getRawRidingCatchUp(url);
  if (res.status < 200 || res.status >= 300) {
    throw new Error(
      `GET ${url} returned ${res.status}: ${res.text.slice(0, 200)}${describeCatchUpRide(res)}`
    );
  }
  return { status: res.status, body: JSON.parse(res.text) as ContentItemSurface };
}

/** Result of GET /db/content/{id}/head — the notary HEAD answer. */
export interface DeclaredHeadResult {
  headActionHash: string;
  declared: boolean;
}

/**
 * GET /db/content/{id}/head on a peer and extract the notary HEAD answer.
 * Shared fetch logic behind two call sites that both need "does this peer
 * have a canonical declared head, and what is it": the ch10 cross-doorway
 * comparator (steps/dataplane/resiliency-saga.steps.ts, "peer ... resolves
 * the declared head for content ... equal to peer ...") and the failover
 * concern's per-serving-peer check (steps/dataplane/failover.steps.ts,
 * "every serving doorway ... resolves the same declared head ..."). Throws
 * if the surface itself is unreachable/non-200/non-JSON, or if
 * `headActionHash` is missing/empty (the notary has no HEAD answer at all) —
 * callers decide what an unreachable HEAD means for their assertion.
 */
export async function probeDeclaredHead(
  peerUrl: string,
  contentId: string
): Promise<DeclaredHeadResult> {
  const { status, text } = await getRaw(`${peerUrl}/db/content/${encodeDocId(contentId)}/head`);
  if (status !== 200) {
    throw new Error(
      `GET ${peerUrl}/db/content/${contentId}/head: HTTP ${status} (body: ${text.slice(0, 120)})`
    );
  }
  let body: Record<string, unknown>;
  try {
    body = JSON.parse(text) as Record<string, unknown>;
  } catch {
    throw new Error(`GET ${peerUrl}/db/content/${contentId}/head: response is not valid JSON`);
  }
  const headActionHash = body['headActionHash'];
  if (typeof headActionHash !== 'string' || headActionHash.length === 0) {
    throw new Error(
      `headActionHash missing/empty for "${contentId}" at ${peerUrl} — the notary has no HEAD answer`
    );
  }
  return { headActionHash, declared: body['declared'] === true };
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
// Periodic-sweep gauge poll (resiliency-saga chapters 2, 7, 8)
// ---------------------------------------------------------------------------

/**
 * elohim-storage populates several Prometheus gauges (identity-fill,
 * custodian capacity) from a periodic background sweep with a ~5-minute
 * cadence — NOT on every /metrics scrape. (custody-class is the exception:
 * it has NO periodic loop — it is emitted only as a side-effect of serving
 * GET /api/v1/weave, so its chapter-7 scenario issues that GET itself
 * before polling; discovered 2026-07-29.) The Dataplane Validation
 * stage runs these gauge assertions early in its cucumber run, roughly 2
 * minutes after the edge restart — before the first sweep has had a chance
 * to run — so a single-shot probe reads a genuinely-not-yet-populated gauge
 * as a false pending. `pollForGauge` absorbs that startup race: it re-probes
 * every `intervalMs` (default 30s) until the deadline (`timeoutMs`, default
 * 6 minutes — one sweep cadence plus margin) instead of giving up after one
 * attempt. Once the cure for a chapter's sweep lands, the probe finds the
 * gauge on its first attempt and this adds zero cost — it only spends time
 * when the gauge is genuinely still absent, which is exactly the case that
 * used to false-pend instantly.
 *
 * A thrown probe error is treated the same as an absent value (keep
 * polling) — the caller decides what "still absent at the deadline" means
 * (pending, never a hard failure: an honest "cannot observe yet", never a
 * measured zero).
 */
export const GAUGE_SWEEP_POLL_INTERVAL_MS = 30_000;
export const GAUGE_SWEEP_POLL_TIMEOUT_MS = 6 * 60_000;

async function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

export async function pollForGauge<T>(
  probe: () => Promise<T | undefined | null>,
  opts: { intervalMs?: number; timeoutMs?: number } = {}
): Promise<T | undefined> {
  const intervalMs = opts.intervalMs ?? GAUGE_SWEEP_POLL_INTERVAL_MS;
  const deadline = Date.now() + (opts.timeoutMs ?? GAUGE_SWEEP_POLL_TIMEOUT_MS);
  for (;;) {
    let value: T | undefined | null;
    try {
      value = await probe();
    } catch {
      value = undefined;
    }
    if (value !== undefined && value !== null) return value;
    if (Date.now() >= deadline) return undefined;
    await sleep(Math.min(intervalMs, deadline - Date.now()));
  }
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
