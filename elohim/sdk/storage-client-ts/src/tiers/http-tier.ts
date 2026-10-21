/**
 * `HttpTier`: seam A's remote tier over HTTP (interfaces spec §H.2, §A.6), against
 * `@elohim/eprfs`'s `Tier` interface.
 *
 * It dials `record.endpoint` and asks the batch route, `POST /eprfs/v1/blobs`, with the root in
 * every request (A1, H3: never `/blob/{cid}`, so a tier can check membership under the root and
 * the requester's reach before it reads a byte). The answer is a CBOR sequence (RFC 8742) of
 * arrived items and then exactly one absent trailer, in the wire format §A.6 fixes; this tier
 * parses it as it streams.
 *
 * It never throws (decision 11): every requested item is answered, with bytes or a named absence.
 * A stream that fails, times out, is refused at the HTTP layer, or ends without its trailer
 * answers every item it had not yet answered as `unreachable` from this tier, so the walk reads it
 * as withheld and never as not held (B3). It rewrites any remote `own` to the id it dialled (A2).
 *
 * Verification stays in `TierChain` (§H.2): the chain hashes every byte before it returns or
 * writes it back, and counts a bad copy against this tier.
 */
import {
  absence,
  isItemAbsence,
  isTierId,
  isUnanswered,
  rewriteOwn,
  type BatchItem,
  type ItemAbsence,
  type NamedAbsence,
  type Tier,
  type TierAnswer,
  type TierConnector,
  type TierRecord,
  type TierSink,
  type Unanswered,
} from '@elohim/eprfs';
import { CborItemFramer, decodeCanonical, encodeCanonical } from '@elohim/epr';
import { CID } from 'multiformats/cid';

/** The batch route (§A.6). */
export const BATCH_ROUTE = '/eprfs/v1/blobs';

/**
 * Signs a direct request with the requester's agent key (A7: through a doorway the doorway's
 * validated `X-Agent-Cid` authenticates instead, so no signer is passed there).
 */
export interface AgentSigner {
  /** The agent key the signature is checked against (`uhCAk…`). */
  readonly agent: string;
  sign(message: Uint8Array): Promise<Uint8Array>;
}

export interface HttpTierOptions {
  signer?: AgentSigner;
  /**
   * Abort the whole request after this long; unanswered items then read as unreachable. Default
   * `DEFAULT_TIMEOUT_MS`; `Infinity` waits on the tier for as long as it keeps the stream open.
   */
  timeoutMs?: number;
  /**
   * The most bytes one sequence item may take. A tier whose next item has buffered past this
   * without completing answers the rest of the batch as unreachable, so a stalled or hostile tier
   * cannot grow this reader's memory without bound. Default `DEFAULT_MAX_ITEM_BYTES`.
   */
  maxItemBytes?: number;
}

/** A batch's default deadline, end to end. */
export const DEFAULT_TIMEOUT_MS = 120_000;
/** One item's default ceiling: a leaf blob plus its framing. */
export const DEFAULT_MAX_ITEM_BYTES = 64 * 1024 * 1024;

/**
 * The bytes received and not yet decoded. It grows geometrically and compacts in place, so a
 * large item arriving in many chunks is copied O(size) times in total, not once per chunk.
 */
class Pending {
  private buf = new Uint8Array(0);
  private start = 0;
  private end = 0;

  get length(): number {
    return this.end - this.start;
  }

  view(): Uint8Array {
    return this.buf.subarray(this.start, this.end);
  }

  push(chunk: Uint8Array): void {
    if (this.end + chunk.length > this.buf.length) {
      const live = this.length;
      const wanted = live + chunk.length;
      // Compact only into at most half the buffer, else double it: each byte is moved O(1)
      // times amortized, however the chunks fall.
      if (wanted <= this.buf.length / 2) {
        this.buf.copyWithin(0, this.start, this.end);
      } else {
        const grown = new Uint8Array(Math.max(wanted, 2 * this.buf.length, 64 * 1024));
        grown.set(this.view());
        this.buf = grown;
      }
      this.start = 0;
      this.end = live;
    }
    this.buf.set(chunk, this.end);
    this.end += chunk.length;
  }

  /** Drop the first `n` bytes; the caller decodes them first and keeps no view of them after. */
  consume(n: number): void {
    this.start += n;
    if (this.start === this.end) this.start = this.end = 0;
  }
}

/** Headers the signed form carries (A7). Named here; the serving half is a follow-on. */
export const SIGNATURE_HEADERS = {
  agent: 'X-Eprfs-Agent',
  nonce: 'X-Eprfs-Nonce',
  signature: 'X-Eprfs-Signature',
} as const;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isPath(value: unknown): value is Uint8Array[] {
  return Array.isArray(value) && value.every((c) => c instanceof Uint8Array);
}

function samePath(a: readonly Uint8Array[], b: readonly Uint8Array[]): boolean {
  return (
    a.length === b.length &&
    a.every((c, i) => c.length === b[i].length && c.every((byte, j) => byte === b[i][j]))
  );
}

function base64url(bytes: Uint8Array): string {
  let text = '';
  for (const b of bytes) text += String.fromCharCode(b);
  return btoa(text).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function absent(item: BatchItem, unanswered: Unanswered<ItemAbsence>): NamedAbsence {
  return { kind: 'absent', cid: item.cid.toString(), path: item.path, unanswered };
}

/** The wire `unanswered` of a trailer entry, with every remote `own` rewritten, or null if malformed. */
function readUnanswered(value: unknown, dialled: string): Unanswered<ItemAbsence> | null {
  if (!isRecord(value) || !isRecord(value.why) || typeof value.why.kind !== 'string') return null;
  const why = rewriteOwn(value.why as unknown as ItemAbsence, dialled);
  const candidate = { ...value, why };
  return isUnanswered(candidate, isItemAbsence) ? (candidate as unknown as Unanswered<ItemAbsence>) : null;
}

export class HttpTier implements Tier {
  private readonly signer: AgentSigner | undefined;
  private readonly timeoutMs: number;
  private readonly maxItemBytes: number;

  constructor(
    readonly record: TierRecord,
    opts: HttpTierOptions = {},
  ) {
    this.signer = opts.signer;
    this.timeoutMs = opts.timeoutMs ?? DEFAULT_TIMEOUT_MS;
    this.maxItemBytes = opts.maxItemBytes ?? DEFAULT_MAX_ITEM_BYTES;
  }

  /** The batch route's URL, or null when the record names no URL endpoint. */
  get url(): string | null {
    const ep = this.record.endpoint;
    if (ep === null || ep.form !== 'url') return null;
    return `${ep.base.replace(/\/+$/, '')}${BATCH_ROUTE}`;
  }

  /** A one-item batch (H3). */
  async fetchAt(root: CID, item: BatchItem, via: readonly string[], signal?: AbortSignal): Promise<TierAnswer> {
    let answer: TierAnswer | undefined;
    await this.fetchBlobs(root, [item], via, (_i, r) => (answer ??= r), signal);
    return answer ?? absent(item, absence.unreachable(this.record.id));
  }

  async fetchBlobs(
    root: CID,
    items: readonly BatchItem[],
    via: readonly string[],
    sink: TierSink,
    signal?: AbortSignal,
  ): Promise<void> {
    if (items.length === 0) return;
    const answered = new Set<number>();
    const deliver = (index: number, r: TierAnswer) => {
      if (answered.has(index)) return;
      answered.add(index);
      sink(items[index], r);
    };
    const unreachableRest = () => {
      const why = absence.unreachable(this.record.id);
      for (let i = 0; i < items.length; i++) if (!answered.has(i)) deliver(i, absent(items[i], why));
    };

    const url = this.url;
    if (url === null) {
      unreachableRest();
      return;
    }

    const controller = new AbortController();
    const onAbort = () => controller.abort();
    if (signal?.aborted) controller.abort();
    signal?.addEventListener('abort', onAbort, { once: true });
    const timer = Number.isFinite(this.timeoutMs)
      ? setTimeout(() => controller.abort(), this.timeoutMs)
      : undefined;

    try {
      const body = encodeCanonical({
        root,
        items: items.map((i) => ({ cid: i.cid, path: [...i.path] })),
        via: via.filter(isTierId),
      });
      const headers: Record<string, string> = {
        'Content-Type': 'application/cbor',
        Accept: 'application/cbor-seq',
      };
      if (this.signer !== undefined) {
        const nonce = base64url(crypto.getRandomValues(new Uint8Array(16)));
        const message = encodeCanonical({ root, items: items.map((i) => i.cid), nonce });
        headers[SIGNATURE_HEADERS.agent] = this.signer.agent;
        headers[SIGNATURE_HEADERS.nonce] = nonce;
        headers[SIGNATURE_HEADERS.signature] = base64url(await this.signer.sign(message));
      }
      const response = await fetch(url, { method: 'POST', headers, body, signal: controller.signal });
      if (!response.ok || response.body === null) return; // unreachable below

      const trailer = await this.readSequence(response.body, items, deliver);
      if (trailer !== null) {
        for (const { index, unanswered } of trailer) deliver(index, absent(items[index], unanswered));
      }
    } catch {
      // Transport failure, abort or timeout: nothing is established; unreachable below.
    } finally {
      if (timer !== undefined) clearTimeout(timer);
      signal?.removeEventListener('abort', onAbort);
      unreachableRest();
    }
  }

  /**
   * Reads the CBOR sequence: arrived items are delivered as they decode; the trailer is returned
   * and delivered only once the stream has ended cleanly after it. Returns null when the stream
   * breaks the framing (no trailer, a malformed item, anything after the trailer, or an item that
   * outgrows `maxItemBytes`), in which case the caller answers every undelivered item as
   * unreachable and the stream is cancelled.
   */
  private async readSequence(
    stream: ReadableStream<Uint8Array>,
    items: readonly BatchItem[],
    deliver: (index: number, r: TierAnswer) => void,
  ): Promise<{ index: number; unanswered: Unanswered<ItemAbsence> }[] | null> {
    const byCid = new Map<string, number[]>();
    items.forEach((item, index) => {
      const key = item.cid.toString();
      byCid.set(key, [...(byCid.get(key) ?? []), index]);
    });

    const reader = stream.getReader();
    const pending = new Pending();
    // One resumable framer for the item at the buffer's head: it keeps its place across chunks,
    // so each head is read once however the item arrives (an item of many tiny elements would
    // otherwise be re-walked from its start on every read, quadratic in its size).
    const framer = new CborItemFramer();
    let trailer: { index: number; unanswered: Unanswered<ItemAbsence> }[] | null = null;
    let clean = false;
    try {
      for (;;) {
        const { value, done } = await reader.read();
        if (value !== undefined && value.length > 0) pending.push(value);
        // Decode every complete item in the buffer. The framer resumes where the last read left
        // it, so an incomplete item costs only the heads that arrived since.
        for (;;) {
          if (pending.length === 0) break;
          if (trailer !== null) return null; // anything after the trailer is a protocol error
          const length = framer.frame(pending.view());
          if (length === null) {
            if (pending.length > this.maxItemBytes) return null; // an item past the ceiling
            break;
          }
          if (length > this.maxItemBytes) return null;
          // `decodeCanonical` copies byte strings out, so consuming the bytes after is safe.
          const item = decodeCanonical<unknown>(pending.view().subarray(0, length));
          pending.consume(length);
          framer.reset();
          if (!isRecord(item)) return null;
          if ('bytes' in item) {
            const cid = CID.asCID(item.cid);
            if (cid === null || !(item.bytes instanceof Uint8Array) || !Array.isArray(item.paths)) return null;
            // Items with unrequested CIDs are dropped (contract 10); one copy answers every path.
            for (const index of byCid.get(cid.toString()) ?? []) deliver(index, item.bytes);
          } else if ('absent' in item) {
            if (!Array.isArray(item.absent)) return null;
            trailer = [];
            for (const entry of item.absent) {
              if (!isRecord(entry)) return null;
              const cid = CID.asCID(entry.cid);
              const unanswered = readUnanswered(entry.unanswered, this.record.id);
              if (cid === null || !isPath(entry.path) || unanswered === null) return null;
              const path = entry.path;
              const indices = (byCid.get(cid.toString()) ?? []).filter((i) => samePath(items[i].path, path));
              for (const index of indices) trailer.push({ index, unanswered });
            }
          } else {
            return null;
          }
        }
        if (done) {
          clean = true;
          break;
        }
      }
    } finally {
      // Stop the transfer when this reader gave up on it; a cleanly ended stream has nothing left.
      if (!clean) await reader.cancel().catch(() => undefined);
      reader.releaseLock();
    }
    // A stream ending without a trailer, or inside an item, means the tier failed partway (B3).
    return pending.length === 0 ? trailer : null;
  }
}

/** Is `base` a URL a secure page may dial (H11)? Loopback http is admitted for dev origins. */
export function admissibleFromSecureContext(base: string): boolean {
  let url: URL;
  try {
    url = new URL(base);
  } catch {
    return false;
  }
  if (url.protocol === 'https:') return true;
  if (url.protocol !== 'http:') return false;
  const host = url.hostname;
  return host === 'localhost' || host === '127.0.0.1' || host === '[::1]' || host.endsWith('.localhost');
}

export interface HttpConnectorOptions {
  signer?: AgentSigner;
  /** Per batch; default `DEFAULT_TIMEOUT_MS` (see `HttpTierOptions.timeoutMs`). */
  timeoutMs?: number;
  /** See `HttpTierOptions.maxItemBytes`. */
  maxItemBytes?: number;
  /**
   * Whether the page is a secure context. Defaults to `globalThis.isSecureContext`. From a secure
   * origin an `http://<lan-ip>` endpoint is mixed content, so the connector does not connect it
   * (H11): it is used only on loopback and dev origins until a household serves HTTPS under a name.
   */
  secureContext?: boolean;
}

/** Rust's `TierConnector` over HTTP: records the page holds become tiers (H8). */
export class HttpConnector implements TierConnector {
  constructor(private readonly opts: HttpConnectorOptions = {}) {}

  connect(record: TierRecord): Tier | null {
    const ep = record.endpoint;
    if (ep === null || ep.form !== 'url') return null;
    const secure =
      this.opts.secureContext ?? (globalThis as { isSecureContext?: boolean }).isSecureContext ?? false;
    if (secure && !admissibleFromSecureContext(ep.base)) return null;
    return new HttpTier(record, {
      signer: this.opts.signer,
      timeoutMs: this.opts.timeoutMs,
      maxItemBytes: this.opts.maxItemBytes,
    });
  }
}
