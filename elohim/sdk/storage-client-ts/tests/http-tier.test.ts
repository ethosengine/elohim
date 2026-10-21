// HttpTier over the batch route's wire format (interfaces spec §A.6, §H.2): a stub server speaks
// the CBOR sequence, and a TierChain over a MemoryTier is the requester, so verification and
// write-back are the chain's, as §H.2 says.
import { CborItemFramer, decodeCanonical, encodeCanonical } from '@elohim/epr';
import { absence, MemoryTier, TierChain, standingPolicy, type TierRecord } from '@elohim/eprfs';
import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';
import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  admissibleFromSecureContext,
  BATCH_ROUTE,
  DEFAULT_TIMEOUT_MS,
  HttpConnector,
  HttpTier,
  SIGNATURE_HEADERS,
} from '../src/tiers/http-tier';

const enc = new TextEncoder();
const HUB = 'agent:uhCAkhub';
const OTHER = 'agent:uhCAkother';

async function rawCid(bytes: Uint8Array): Promise<CID> {
  return CID.create(1, 0x55, await sha256.digest(bytes));
}

function record(id: string, base: string | null, klass: TierRecord['class'] = 'household'): TierRecord {
  return {
    id,
    class: klass,
    sources: [{ source: 'held-record', record: 'household-roster' }],
    endpoint: base === null ? null : { form: 'url', base },
    standing: null,
    proximity: null,
  };
}

function seq(...items: unknown[]): Uint8Array {
  const parts = items.map((i) => encodeCanonical(i));
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}

/** A body streamed in small chunks, so items split across reads. */
function chunked(bytes: Uint8Array, size = 7): ReadableStream<Uint8Array> {
  let at = 0;
  return new ReadableStream({
    pull(controller) {
      if (at >= bytes.length) {
        controller.close();
        return;
      }
      controller.enqueue(bytes.slice(at, at + size));
      at += size;
    },
  });
}

interface Seen {
  url: string;
  headers: Record<string, string>;
  body: { root: CID; items: { cid: CID; path: Uint8Array[] }[]; via: string[] };
}

function serve(answer: (seen: Seen) => Response | Promise<Response>): Seen[] {
  const seen: Seen[] = [];
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string, init: RequestInit) => {
      const s: Seen = {
        url,
        headers: init.headers as Record<string, string>,
        body: decodeCanonical(init.body as Uint8Array),
      };
      seen.push(s);
      return answer(s);
    }),
  );
  return seen;
}

function cborSeq(bytes: Uint8Array, status = 200): Response {
  return new Response(chunked(bytes), { status, headers: { 'Content-Type': 'application/cbor-seq' } });
}

const unansweredNotHeld = (tried: string[]) => ({
  why: { kind: 'not-held', tried },
  comeBack: { when: 'on-event', event: 'tier-reachable' },
  meanwhile: { local: 'nothing', because: 'no tier holds it' },
  escalation: null,
});

afterEach(() => {
  vi.unstubAllGlobals();
});

async function fixture() {
  const root = await rawCid(enc.encode('root-node-stand-in'));
  const a = enc.encode('alpha');
  const b = enc.encode('beta');
  return { root, a, b, aCid: await rawCid(a), bCid: await rawCid(b), pa: [enc.encode('a')], pb: [enc.encode('b')] };
}

describe('HttpTier — the request', () => {
  it('posts the root, items and via to the batch route, never /blob/{cid}', async () => {
    const f = await fixture();
    const seen = serve(() => cborSeq(seq({ absent: [] })));
    const tier = new HttpTier(record(HUB, 'https://hub.dowell.example/'));
    await tier.fetchBlobs(f.root, [{ cid: f.aCid, path: f.pa }], [OTHER, 'own'], () => {});
    expect(seen).toHaveLength(1);
    expect(seen[0].url).toBe(`https://hub.dowell.example${BATCH_ROUTE}`);
    expect(seen[0].url).not.toContain('/blob/');
    expect(seen[0].headers['Content-Type']).toBe('application/cbor');
    expect(CID.asCID(seen[0].body.root)?.toString()).toBe(f.root.toString());
    expect(CID.asCID(seen[0].body.items[0].cid)?.toString()).toBe(f.aCid.toString());
    expect(seen[0].body.items[0].path).toEqual(f.pa);
    // `own` is indexical and never crosses the wire.
    expect(seen[0].body.via).toEqual([OTHER]);
  });

  it('signs a direct request over (root, item CIDs, nonce) when given a signer (A7)', async () => {
    const f = await fixture();
    const seen = serve(() => cborSeq(seq({ absent: [] })));
    const signed: Uint8Array[] = [];
    const tier = new HttpTier(record(HUB, 'https://hub.example'), {
      signer: { agent: 'uhCAkme', sign: async (m) => (signed.push(m), new Uint8Array([1, 2, 3])) },
    });
    await tier.fetchAt(f.root, { cid: f.aCid, path: f.pa }, []);
    const h = seen[0].headers;
    expect(h[SIGNATURE_HEADERS.agent]).toBe('uhCAkme');
    expect(h[SIGNATURE_HEADERS.signature]).toBe('AQID');
    const message = decodeCanonical<{ root: CID; items: CID[]; nonce: string }>(signed[0]);
    expect(CID.asCID(message.root)?.toString()).toBe(f.root.toString());
    expect(message.items.map((c) => CID.asCID(c)?.toString())).toEqual([f.aCid.toString()]);
    expect(message.nonce).toBe(h[SIGNATURE_HEADERS.nonce]);
  });
});

describe('HttpTier — the answer', () => {
  it('delivers arrived bytes for every requested path, and the trailer rewritten from own', async () => {
    const f = await fixture();
    serve(() =>
      cborSeq(
        seq(
          { cid: f.aCid, paths: [f.pa], bytes: f.a },
          { absent: [{ cid: f.bCid, path: f.pb, unanswered: unansweredNotHeld(['own', OTHER]) }] },
        ),
      ),
    );
    const tier = new HttpTier(record(HUB, 'https://hub.example'));
    const got = new Map<string, unknown>();
    await tier.fetchBlobs(
      f.root,
      [
        { cid: f.aCid, path: f.pa },
        { cid: f.bCid, path: f.pb },
      ],
      [],
      (item, r) => got.set(item.cid.toString(), r),
    );
    expect(got.get(f.aCid.toString())).toEqual(f.a);
    const b = got.get(f.bCid.toString()) as { kind: string; unanswered: { why: { tried: string[] } } };
    expect(b.kind).toBe('absent');
    expect(b.unanswered.why.tried).toEqual([HUB, OTHER]);
  });

  it('answers unreachable, never not-held, when the stream ends without its trailer (B3)', async () => {
    const f = await fixture();
    serve(() => cborSeq(seq({ cid: f.aCid, paths: [f.pa], bytes: f.a })));
    const tier = new HttpTier(record(HUB, 'https://hub.example'));
    const got = new Map<string, unknown>();
    await tier.fetchBlobs(
      f.root,
      [
        { cid: f.aCid, path: f.pa },
        { cid: f.bCid, path: f.pb },
      ],
      [],
      (item, r) => got.set(item.cid.toString(), r),
    );
    expect(got.get(f.aCid.toString())).toEqual(f.a);
    expect(got.get(f.bCid.toString())).toMatchObject({
      kind: 'absent',
      unanswered: { why: { kind: 'unreachable', tier: HUB } },
    });
  });

  it('reads anything after the trailer as a protocol error: the trailer establishes nothing', async () => {
    const f = await fixture();
    serve(() =>
      cborSeq(
        seq(
          { absent: [{ cid: f.bCid, path: f.pb, unanswered: unansweredNotHeld([]) }] },
          { cid: f.aCid, paths: [f.pa], bytes: f.a },
        ),
      ),
    );
    const r = await new HttpTier(record(HUB, 'https://hub.example')).fetchAt(f.root, { cid: f.bCid, path: f.pb }, []);
    expect(r).toMatchObject({ kind: 'absent', unanswered: { why: { kind: 'unreachable', tier: HUB } } });
  });

  it('answers unreachable for an HTTP error, a transport failure, a malformed trailer and a timeout', async () => {
    const f = await fixture();
    const item = { cid: f.aCid, path: f.pa };
    const tier = new HttpTier(record(HUB, 'https://hub.example'), { timeoutMs: 20 });
    const unreachable = { kind: 'absent', unanswered: { why: { kind: 'unreachable', tier: HUB } } };

    serve(() => new Response('no', { status: 503 }));
    expect(await tier.fetchAt(f.root, item, [])).toMatchObject(unreachable);

    serve(() => Promise.reject(new TypeError('connection refused')));
    expect(await tier.fetchAt(f.root, item, [])).toMatchObject(unreachable);

    serve(() => cborSeq(seq({ absent: [{ cid: f.aCid, path: f.pa, unanswered: { why: 'nope' } }] })));
    expect(await tier.fetchAt(f.root, item, [])).toMatchObject(unreachable);

    vi.stubGlobal(
      'fetch',
      vi.fn(
        (_url: string, init: RequestInit) =>
          new Promise((_resolve, reject) => {
            init.signal?.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')));
          }),
      ),
    );
    expect(await tier.fetchAt(f.root, item, [])).toMatchObject(unreachable);
  });

  it('a record without a URL endpoint answers unreachable without dialling', async () => {
    const f = await fixture();
    const seen = serve(() => cborSeq(seq({ absent: [] })));
    const r = await new HttpTier(record(HUB, null)).fetchAt(f.root, { cid: f.aCid, path: f.pa }, []);
    expect(r).toMatchObject({ kind: 'absent', unanswered: { why: { kind: 'unreachable' } } });
    expect(seen).toHaveLength(0);
  });
});

describe('HttpTier inside a TierChain', () => {
  const levels = { household: 2, public: 6 };

  it('verifies, writes back and serves offline afterwards; a bad copy is integrity, never cached', async () => {
    const f = await fixture();
    const bad = enc.encode('forged');
    serve(() =>
      cborSeq(
        seq({ cid: f.aCid, paths: [f.pa], bytes: f.a }, { cid: f.bCid, paths: [f.pb], bytes: bad }, { absent: [] }),
      ),
    );
    const own = new MemoryTier();
    const connector = new HttpConnector({ secureContext: false });
    const chain = new TierChain(own, [record(HUB, 'http://192.168.1.20:8090')], connector, standingPolicy(levels), {
      rootReach: () => levels.household,
    });
    const got = new Map<string, unknown>();
    await chain.fetchBlobs(
      f.root,
      [
        { cid: f.aCid, path: f.pa },
        { cid: f.bCid, path: f.pb },
      ],
      [],
      (item, r) => got.set(item.cid.toString(), r),
    );
    expect(got.get(f.aCid.toString())).toEqual(f.a);
    expect(own.blobs.get(f.aCid.toString())).toEqual(f.a);
    expect(got.get(f.bCid.toString())).toMatchObject({ kind: 'absent', unanswered: { why: { kind: 'integrity' } } });
    expect(own.blobs.has(f.bCid.toString())).toBe(false);
    expect(chain.stats().perTier[HUB]).toEqual({ asked: 1, hit: 1, integrity: 1 });

    // Offline: the verified file is answered from the own tier with no request.
    const fetchSpy = vi.fn(() => Promise.reject(new TypeError('offline')));
    vi.stubGlobal('fetch', fetchSpy);
    expect(await chain.fetchAt(f.root, { cid: f.aCid, path: f.pa }, [])).toEqual(f.a);
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('a not-held trailer names the tiers tried', async () => {
    const f = await fixture();
    serve(() => cborSeq(seq({ absent: [{ cid: f.aCid, path: f.pa, unanswered: unansweredNotHeld([]) }] })));
    const chain = new TierChain(
      new MemoryTier(),
      [record(HUB, 'https://hub.example')],
      new HttpConnector({ secureContext: true }),
      standingPolicy(levels),
      { rootReach: () => levels.household },
    );
    const r = await chain.fetchAt(f.root, { cid: f.aCid, path: f.pa }, []);
    expect(r).toEqual({
      kind: 'absent',
      cid: f.aCid.toString(),
      path: f.pa,
      unanswered: absence.notHeld([HUB]),
    });
  });
});

describe('HttpConnector — mixed content (H11)', () => {
  it('connects https anywhere, and http only on loopback, from a secure context', () => {
    const secure = new HttpConnector({ secureContext: true });
    expect(secure.connect(record(HUB, 'https://hub.example'))).toBeInstanceOf(HttpTier);
    expect(secure.connect(record(HUB, 'http://192.168.1.20:8090'))).toBeNull();
    expect(secure.connect(record(HUB, 'http://localhost:8090'))).toBeInstanceOf(HttpTier);
    expect(secure.connect(record(HUB, null))).toBeNull();
    expect(
      secure.connect({ ...record(HUB, null), endpoint: { form: 'multiaddr', addr: '/ip4/1.2.3.4/tcp/1' } }),
    ).toBeNull();
    const dev = new HttpConnector({ secureContext: false });
    expect(dev.connect(record(HUB, 'http://192.168.1.20:8090'))).toBeInstanceOf(HttpTier);
    expect(admissibleFromSecureContext('ftp://x')).toBe(false);
    expect(admissibleFromSecureContext('http://127.0.0.1:1')).toBe(true);
  });
});

describe('HttpTier — bounded reading', () => {
  it('reassembles a large leaf from many small chunks, and a later compaction leaves delivered bytes intact', async () => {
    const f = await fixture();
    const big = new Uint8Array(300_000).map((_, i) => (i * 31) & 0xff);
    const bigCid = await rawCid(big);
    const body = seq(
      { cid: f.aCid, paths: [f.pa], bytes: f.a },
      { cid: bigCid, paths: [f.pb], bytes: big },
      { absent: [] },
    );
    let reads = 0;
    let at = 0;
    const stream = new ReadableStream<Uint8Array>({
      pull(controller) {
        if (at >= body.length) return controller.close();
        reads++;
        controller.enqueue(body.slice(at, at + 4096));
        at += 4096;
      },
    });
    serve(() => new Response(stream, { status: 200 }));
    const got = new Map<string, unknown>();
    await new HttpTier(record(HUB, 'https://hub.example')).fetchBlobs(
      f.root,
      [
        { cid: f.aCid, path: f.pa },
        { cid: bigCid, path: f.pb },
      ],
      [],
      (item, r) => got.set(item.cid.toString(), r),
    );
    expect(reads).toBeGreaterThan(70);
    expect(got.get(f.aCid.toString())).toEqual(f.a);
    expect(got.get(bigCid.toString())).toEqual(big);
  });

  it('gives up on an item that outgrows maxItemBytes, cancels the stream, and answers unreachable', async () => {
    const f = await fixture();
    // A byte-string head claiming 2^40 bytes inside an arrived item, then filler without end.
    const head = new Uint8Array([0xa3, 0x63, 0x63, 0x69, 0x64, 0x5b, 0, 0, 0x01, 0, 0, 0, 0, 0]);
    let sent = 0;
    let cancelled = false;
    const stream = new ReadableStream<Uint8Array>({
      pull(controller) {
        controller.enqueue(sent === 0 ? head : new Uint8Array(1024));
        sent++;
      },
      cancel() {
        cancelled = true;
      },
    });
    serve(() => new Response(stream, { status: 200 }));
    const tier = new HttpTier(record(HUB, 'https://hub.example'), { maxItemBytes: 8 * 1024 });
    const r = await tier.fetchAt(f.root, { cid: f.aCid, path: f.pa }, []);
    expect(r).toMatchObject({ kind: 'absent', unanswered: { why: { kind: 'unreachable', tier: HUB } } });
    expect(cancelled).toBe(true);
    expect(sent).toBeLessThan(16);
  });

  it('frames an item of many tiny elements in linear work however it is chunked', async () => {
    // An arrived item whose `paths` array holds 2^20 one-byte elements, streamed in 4 KiB chunks
    // and then ended: re-walking from the item's start on every read would visit ~2^20 * 256
    // heads; the resumable framer visits each once, plus one partial head per read.
    const f = await fixture();
    const n = 1 << 20;
    const prefix = new Uint8Array([0xa1, 0x65, ...enc.encode('paths'), 0x9a, (n >>> 24) & 0xff, (n >>> 16) & 0xff, (n >>> 8) & 0xff, n & 0xff]);
    const body = new Uint8Array(prefix.length + n);
    body.set(prefix);
    body.fill(0x01, prefix.length);
    let reads = 0;
    let at = 0;
    const stream = new ReadableStream<Uint8Array>({
      pull(controller) {
        if (at >= body.length - 1) return controller.close(); // one element short: never completes
        reads++;
        controller.enqueue(body.slice(at, Math.min(at + 4096, body.length - 1)));
        at += 4096;
      },
    });
    const frame = CborItemFramer.prototype.frame;
    let visits = 0;
    const spy = vi.spyOn(CborItemFramer.prototype, 'frame').mockImplementation(function (this: CborItemFramer, bytes) {
      const out = frame.call(this, bytes);
      visits = Math.max(visits, this.visits);
      return out;
    });
    try {
      serve(() => new Response(stream, { status: 200 }));
      const tier = new HttpTier(record(HUB, 'https://hub.example'), { maxItemBytes: 4 * n });
      const r = await tier.fetchAt(f.root, { cid: f.aCid, path: f.pa }, []);
      expect(r).toMatchObject({ kind: 'absent', unanswered: { why: { kind: 'unreachable', tier: HUB } } });
      expect(reads).toBeGreaterThan(250);
      expect(visits).toBeGreaterThan(n - 1);
      expect(visits).toBeLessThanOrEqual(n + 3 + reads);
    } finally {
      spy.mockRestore();
    }
  });

  it('a connector with no timeout still ends a stalled tier at the finite default', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    try {
      const f = await fixture();
      vi.stubGlobal(
        'fetch',
        vi.fn(
          (_url: string, init: RequestInit) =>
            new Promise((_resolve, reject) => {
              init.signal?.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')));
            }),
        ),
      );
      expect(Number.isFinite(DEFAULT_TIMEOUT_MS)).toBe(true);
      const tier = new HttpConnector({ secureContext: true }).connect(record(HUB, 'https://hub.example'));
      const pending = tier!.fetchAt(f.root, { cid: f.aCid, path: f.pa }, []);
      await vi.advanceTimersByTimeAsync(DEFAULT_TIMEOUT_MS);
      expect(await pending).toMatchObject({ kind: 'absent', unanswered: { why: { kind: 'unreachable', tier: HUB } } });
    } finally {
      vi.useRealTimers();
    }
  });
});

describe('one TypeScript verifier (R-T3)', () => {
  it('this package takes no sha2-256 digest of its own: verification is the chain\'s, through @elohim/epr', async () => {
    const { readdirSync, readFileSync } = await import('node:fs');
    const { join } = await import('node:path');
    const files = (dir: string): string[] =>
      readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
        e.isDirectory() ? files(join(dir, e.name)) : e.name.endsWith('.ts') ? [join(dir, e.name)] : [],
      );
    const digest = /\bsha256\.digest\(|\bsha256\(|subtle\.digest\(/;
    const hashing = files(join(process.cwd(), 'src')).filter((f) => digest.test(readFileSync(f, 'utf8')));
    expect(hashing).toEqual([]);
  });
});
