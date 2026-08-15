import type { StorageClient } from '../client';
import type { NetworkPostureView } from '../generated/NetworkPostureView';
import type { P2PStatusInfo } from '../generated/P2PStatusInfo';
import type { PeerListView } from '../generated/PeerListView';
import type { PeerStatusView } from '../generated/PeerStatusView';
import type { ProjectorStatusView } from '../generated/ProjectorStatusView';
import type { ReaCommitmentView } from '../generated/ReaCommitmentView';
import type { EconomicEventView } from '../generated/EconomicEventView';

/**
 * `GET /version` response. Hand-declared: elohim-storage's `/version` route
 * serializes `elohim_compute::BuildInfo` directly (no ts-rs-derived view, no
 * schema-owned wire contract), so there is no generated type to bind to.
 */
export interface VersionInfo {
  commit?: string;
  version?: string;
}

/**
 * Normalized `GET /api/v1/diagnostics/inventory-parity` projection.
 *
 * The Rust wire type (`ParityReport` in
 * `elohim-storage/src/p2p/inventory_broadcaster.rs`) is a plain
 * `#[derive(Serialize)]` struct with no `rename_all = "camelCase")]` and no
 * ts-rs binding, so it serializes as literal snake_case
 * (`filesystem_count`). `getInventoryParity()` normalizes that boundary so
 * callers never branch on which casing a given peer happens to answer with.
 */
export interface InventoryParity {
  /** `filesystem_count` | `filesystemCount`, whichever the peer sent. `null` when absent or non-numeric. */
  filesystemCount: number | null;
}

/**
 * Query params for `GET /api/v1/commitments`.
 *
 * Hand-declared here — see schema counterpart at
 * `elohim/sdk/schemas/v1/inputs/rea-commitment-query.schema.json`.
 * `storage-client-ts` is not currently wired into the schema codegen
 * consumer list (`INTERFACE_FILES` in `elohim/sdk/schemas/scripts/codegen-ts.mjs`),
 * so the schema documents the shape without generating it; keep the two in
 * sync by hand until that wiring is added.
 */
export interface ReaCommitmentQuery {
  action?: string;
  limit?: number;
}

/**
 * Query params for `GET /api/v1/economic-events`.
 *
 * Hand-declared here — see schema counterpart at
 * `elohim/sdk/schemas/v1/inputs/economic-event-query.schema.json`. Same
 * codegen-consumer caveat as {@link ReaCommitmentQuery}.
 */
export interface EconomicEventQuery {
  action?: string;
  after?: string;
  limit?: number;
}

/** Named `/p2p/status` sync streams that carry tri/quad-state progress. */
export type StreamName = 'replication' | 'pull' | 'projectionReconcile';

/**
 * Tri/quad-state verdict for a single `/p2p/status` sync stream.
 *
 * - `not-computable` — the stream object is missing/null (the reconcile
 *   task never ran, or the DB pool/query was unavailable) OR its
 *   `caughtUp`/`caught_up` field is absent — never read as "caught up".
 * - `idle` — `pull`-only: `total === 0`, i.e. the desired-set reconcile
 *   ran and found nothing to fetch. Resolved-empty and a broken loop that
 *   never enqueues anything are indistinguishable from this field alone;
 *   `idle` names that ambiguity rather than guessing which one it is.
 * - `caught-up` / `behind` — the stream's own `caughtUp`/`caught_up`
 *   boolean, read after normalizing casing.
 */
export type StreamSyncState = 'caught-up' | 'behind' | 'idle' | 'not-computable';

const STREAM_KEYS: Record<StreamName, [camel: string, snake: string]> = {
  replication: ['replication', 'replication'],
  pull: ['pull', 'pull'],
  projectionReconcile: ['projectionReconcile', 'projection_reconcile'],
};

function asRecord(value: unknown): Record<string, unknown> | undefined {
  if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return undefined;
}

function pick(obj: Record<string, unknown>, camelKey: string, snakeKey: string): unknown {
  if (camelKey in obj) return obj[camelKey];
  if (snakeKey !== camelKey && snakeKey in obj) return obj[snakeKey];
  return undefined;
}

/**
 * Normalizes mixed-case, tri-state stream sync fields from a raw
 * `/p2p/status` body.
 *
 * A mid-rollout fleet can answer with either camelCase (current) or
 * snake_case (older pods) field names for the same stream — this reads
 * both. Takes `unknown` deliberately: callers may be looking at a raw
 * fetch body from a doorway-proxied or cross-version peer, not necessarily
 * the typed {@link P2PStatusInfo} a healthy same-version pod returns from
 * `getP2pStatus()`.
 */
export function streamSyncState(status: unknown, stream: StreamName): StreamSyncState {
  const root = asRecord(status);
  if (!root) return 'not-computable';

  const [camelKey, snakeKey] = STREAM_KEYS[stream];
  const streamObj = asRecord(pick(root, camelKey, snakeKey));
  if (!streamObj) return 'not-computable';

  if (stream === 'pull') {
    const total = pick(streamObj, 'total', 'total');
    if (typeof total === 'number' && total === 0) {
      return 'idle';
    }
  }

  const caughtUp = pick(streamObj, 'caughtUp', 'caught_up');
  if (typeof caughtUp !== 'boolean') return 'not-computable';
  return caughtUp ? 'caught-up' : 'behind';
}

function buildQuery(params: Record<string, string | number | undefined>): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined) search.set(key, String(value));
  }
  const query = search.toString();
  return query ? `?${query}` : '';
}

/**
 * Dataplane read/diagnostics SDK methods, scoped to a single storage pod
 * (the client's configured `baseUrl` is one node, not the fleet — see
 * {@link DataplaneFleet} in `../fleet` for multi-peer composition).
 *
 * Accessed via `client.dataplane`:
 * ```typescript
 * const status = await client.dataplane.getP2pStatus();
 * const pull = streamSyncState(status, 'pull');
 * ```
 */
export class DataplaneApi {
  constructor(private readonly client: StorageClient) {}

  /** `GET /p2p/status`. */
  async getP2pStatus(): Promise<P2PStatusInfo> {
    return this.client.getJson<P2PStatusInfo>('/p2p/status');
  }

  /** `GET /version`. */
  async getVersion(): Promise<VersionInfo> {
    return this.client.getJson<VersionInfo>('/version');
  }

  /**
   * `GET /api/v1/commitments?action=&limit=`.
   *
   * Element type is the generated {@link ReaCommitmentView} — consumers
   * filtering on `.action` / `.resourceClassifiedAs` get real field types
   * rather than `unknown`.
   */
  async listCommitments(q: ReaCommitmentQuery = {}): Promise<ReaCommitmentView[]> {
    const query = buildQuery({ action: q.action, limit: q.limit });
    return this.client.getJson<ReaCommitmentView[]>(`/api/v1/commitments${query}`);
  }

  /**
   * `GET /api/v1/economic-events?action=&after=&limit=`.
   *
   * Element type is the generated {@link EconomicEventView}.
   */
  async listEconomicEvents(q: EconomicEventQuery = {}): Promise<EconomicEventView[]> {
    const query = buildQuery({ action: q.action, after: q.after, limit: q.limit });
    return this.client.getJson<EconomicEventView[]>(`/api/v1/economic-events${query}`);
  }

  /**
   * `GET /api/v1/status/projector`. Return type is the generated
   * {@link ProjectorStatusView} — consumers read `.lag[].lagSeconds`.
   */
  async getProjectorStatus(): Promise<ProjectorStatusView> {
    return this.client.getJson<ProjectorStatusView>('/api/v1/status/projector');
  }

  /**
   * `GET /api/v1/diagnostics/inventory-parity`, normalized to
   * {@link InventoryParity} (the raw wire body is snake_case).
   */
  async getInventoryParity(): Promise<InventoryParity> {
    const raw = await this.client.getJson<Record<string, unknown>>(
      '/api/v1/diagnostics/inventory-parity',
    );
    const value = pick(raw, 'filesystemCount', 'filesystem_count');
    return { filesystemCount: typeof value === 'number' ? value : null };
  }

  /**
   * Real `GET /blob/{hashOrCid}`, body discarded, HTTP status returned.
   *
   * NEVER implemented as HEAD: a GET on this route triggers heal-on-read
   * server-side (the peer will attempt to repair/refetch a missing shard
   * as a side effect of being asked), a HEAD does not. Never throws on a
   * non-2xx response — the status code IS the answer (404 = genuinely
   * absent, 200 = present). A network-level failure still throws.
   */
  async getBlobStatus(hashOrCid: string): Promise<number> {
    return this.client.getStatusDiscardingBody(`/blob/${encodeURIComponent(hashOrCid)}`);
  }

  /** Delegates to {@link StorageClient.listConnectedPeers} — `GET /p2p/peers`. */
  async listConnectedPeers(): Promise<PeerListView> {
    return this.client.listConnectedPeers();
  }

  /** Delegates to {@link StorageClient.listPeerStatuses} — `GET /api/v1/peer-statuses`. */
  async listPeerStatuses(): Promise<PeerStatusView[]> {
    return this.client.listPeerStatuses();
  }

  /** Delegates to {@link StorageClient.getNetworkPosture} — `GET /api/v1/network/posture`. */
  async getNetworkPosture(): Promise<NetworkPostureView> {
    return this.client.getNetworkPosture();
  }
}
