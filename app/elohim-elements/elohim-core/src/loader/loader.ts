/**
 * Loader — transport-agnostic content-addressed resolution.
 *
 * The protocol is peer-native first. Bundles, elements, and arbitrary
 * EPR content all resolve via CID. The Loader tries a configured list
 * of transports in order:
 *
 *   1. localCache  — service-worker / IndexedDB cache
 *   2. tauri       — direct HTTP to localhost:8090 storage instance
 *   3. doorway     — projection cache via current origin's doorway
 *   4. peer (future) — WebTransport / WebRTC to other peers
 *
 * Each transport returns either bytes, null (unavailable for this CID),
 * or throws (transport-level error — try next). When all transports
 * fail, returns { bytes: null, source: 'unresolved' }.
 *
 * Optional CID verification re-hashes the returned bytes and refuses
 * to return them if the hash doesn't match the requested CID. ALWAYS
 * enabled in production (the default); disabled in tests where computing
 * real SHAs is unnecessary friction.
 *
 * The actual transport implementations are NOT in this module — just the
 * Loader orchestrator and the LoaderTransport interface. Concrete transports
 * are injected per-environment:
 *
 *   Browser-via-doorway:  [DoorwayTransport]
 *   Tauri:                [LocalCacheTransport, TauriDirectTransport]
 *   Production browser:   [LocalCacheTransport, DoorwayTransport]
 *   (P2P transport added later — does not require a Loader change)
 */

export interface LoaderTransport {
  /** Source name for telemetry and debugging. Appears in LoaderResolution.source. */
  readonly name: string;
  /**
   * Attempt to fetch content for the given CID.
   *
   * - Returns the bytes if content is available on this transport.
   * - Returns null if this transport does not have content for this CID.
   * - Throws on transport-level errors (network failure, timeout, …);
   *   the Loader treats a throw as "try next transport".
   */
  fetch(cid: string): Promise<Uint8Array | null>;
  /** Whether this transport is currently reachable. Returning false skips fetch entirely. */
  isAvailable(): boolean;
}

export interface LoaderResolution {
  /** Resolved bytes, or null when no transport succeeded. */
  bytes: Uint8Array | null;
  /** Name of the transport that resolved the content, or 'unresolved'. */
  source: string;
  /** The CID that was requested. */
  cid: string;
}

export interface LoaderOptions {
  /**
   * When true (the default), re-hash the bytes returned by the transport and
   * reject the promise if the hash does not match the CID. A mismatch is a
   * hard failure — the Loader does NOT fall through to the next transport,
   * because the data is corrupt regardless of which transport provided it.
   */
  verifyCid?: boolean;
}

export class Loader {
  private readonly verifyCid: boolean;

  constructor(
    private readonly transports: LoaderTransport[],
    options: LoaderOptions = {}
  ) {
    // Safe default: always verify unless caller explicitly opts out.
    this.verifyCid = options.verifyCid !== false;
  }

  async resolve(cid: string): Promise<LoaderResolution> {
    for (const transport of this.transports) {
      if (!transport.isAvailable()) continue;

      let bytes: Uint8Array | null;
      try {
        bytes = await transport.fetch(cid);
      } catch {
        // Transport-level error (network failure, timeout, etc.) — try next.
        continue;
      }

      if (bytes === null) continue;

      // CID verification: hard failure — propagate immediately.
      // Do NOT catch and fall through; a CID mismatch means the bytes are
      // corrupt regardless of transport. The caller must decide what to do.
      if (this.verifyCid) {
        await this.verifyCidMatches(cid, bytes);
      }

      return { bytes, source: transport.name, cid };
    }

    return { bytes: null, source: 'unresolved', cid };
  }

  private async verifyCidMatches(cid: string, bytes: Uint8Array): Promise<void> {
    const dashIndex = cid.indexOf('-');
    if (dashIndex === -1) {
      throw new Error(`Malformed CID (no algorithm prefix): ${cid}`);
    }
    const algo = cid.slice(0, dashIndex);
    const expected = cid.slice(dashIndex + 1);

    if (algo !== 'sha256') {
      throw new Error(`Unsupported CID algo: ${algo}`);
    }

    const hashBuffer = await crypto.subtle.digest('SHA-256', bytes);
    const actual = Array.from(new Uint8Array(hashBuffer))
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');

    if (actual !== expected) {
      throw new Error(`CID mismatch: expected ${expected}, got ${actual}`);
    }
  }
}
