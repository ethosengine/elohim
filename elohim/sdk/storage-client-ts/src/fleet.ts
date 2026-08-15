import { StorageClient } from './client';

/** One named fleet member: its CSV-declared name and its bound {@link StorageClient}. */
export interface FleetPeer {
  name: string;
  client: StorageClient;
}

/**
 * A fixed-size named set of {@link StorageClient} instances, one per
 * dataplane peer — the multi-peer counterpart to a single `StorageClient`
 * (which always targets one storage pod).
 *
 * Built from a compact CSV env-var shape so operator/CI configuration
 * doesn't need one env var per peer:
 * ```
 * ELOHIM_DATAPLANE_PEERS="matthew=storage-a.example:8090,adam=storage-b.example:8090"
 * ```
 */
export class DataplaneFleet {
  private readonly order: string[] = [];
  private readonly byName = new Map<string, StorageClient>();

  private constructor() {}

  /**
   * Parse `"name=host:port,name2=host2:port2"` into a fleet.
   *
   * - Scheme-less entries: `http://` is prepended. An entry already
   *   carrying `http://`/`https://` is left as-is.
   * - Empty segments (stray commas, whitespace-only) are skipped.
   * - A segment missing `=` (or with an empty name/host side) is skipped —
   *   malformed input degrades to "peer absent", never a thrown error.
   * - `hAppId` defaults to `'elohim'` when not given in `opts`.
   */
  static fromCsv(csv: string, opts: { hAppId?: string; timeout?: number } = {}): DataplaneFleet {
    const fleet = new DataplaneFleet();
    const hAppId = opts.hAppId ?? 'elohim';

    for (const rawSegment of csv.split(',')) {
      const segment = rawSegment.trim();
      if (!segment) continue;

      const eqIndex = segment.indexOf('=');
      if (eqIndex === -1) continue;

      const name = segment.slice(0, eqIndex).trim();
      const hostPort = segment.slice(eqIndex + 1).trim();
      if (!name || !hostPort) continue;

      const baseUrl = /^https?:\/\//i.test(hostPort) ? hostPort : `http://${hostPort}`;

      fleet.order.push(name);
      fleet.byName.set(name, new StorageClient({ baseUrl, hAppId, timeout: opts.timeout }));
    }

    return fleet;
  }

  /** All fleet members, in CSV declaration order. */
  peers(): FleetPeer[] {
    return this.order.map((name) => ({ name, client: this.byName.get(name)! }));
  }

  /** The client bound to `name`, or `undefined` if no such peer was declared. */
  peer(name: string): StorageClient | undefined {
    return this.byName.get(name);
  }

  /** Fleet member names, in CSV declaration order. */
  names(): string[] {
    return [...this.order];
  }
}
