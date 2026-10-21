// verifyAddress agrees with Rust's BlobCid::verifies on the shared vectors (interfaces spec §H.1
// contract, rail R-T3). The dag-cbor tree vectors and the raw link vector were written by
// eprfs-core/tests/vectors.rs.
import { readFileSync } from 'node:fs';

import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';
import { identity } from 'multiformats/hashes/identity';
import { describe, expect, it } from 'vitest';

import { addressForm, DAG_CBOR_CODEC, RAW_CODEC, verifyAddress } from '../src/address';
import { CborItemFramer, cborItemLength, decodeCanonicalSequence, encodeCanonical } from '../src/cbor';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const vectors: any = JSON.parse(
  readFileSync(new URL('../../../eprfs/fixtures/tree-vectors.json', import.meta.url), 'utf8'),
);

function unhex(text: string): Uint8Array {
  const out = new Uint8Array(text.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = Number.parseInt(text.slice(2 * i, 2 * i + 2), 16);
  return out;
}

function hex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

const HELLO_WORLD = new TextEncoder().encode('hello world');

describe('verifyAddress — the shared vectors', () => {
  it('verifies every dag-cbor tree vector Rust wrote, as a string and as a CID', async () => {
    expect(vectors.trees.length).toBeGreaterThan(0);
    for (const t of vectors.trees) {
      const bytes = unhex(t.bytesHex);
      expect(await verifyAddress(t.cid, bytes), t.name).toBe(true);
      expect(await verifyAddress(CID.parse(t.cid), bytes), t.name).toBe(true);
      expect(addressForm(t.cid)).toBe('dag-cbor');
    }
  });

  it('verifies the raw link vector (sha2-256 of "hello world")', async () => {
    const cid = vectors.link.cid as string;
    expect(addressForm(cid)).toBe('raw');
    expect(await verifyAddress(cid, HELLO_WORLD)).toBe(true);
    expect(await verifyAddress(cid, HELLO_WORLD.buffer.slice(0) as ArrayBuffer)).toBe(true);
  });

  it('refuses a tampered byte under every accepted form', async () => {
    for (const t of vectors.trees) {
      const bytes = unhex(t.bytesHex);
      bytes[bytes.length - 1] ^= 1;
      expect(await verifyAddress(t.cid, bytes), t.name).toBe(false);
    }
    const tampered = HELLO_WORLD.slice();
    tampered[0] ^= 1;
    expect(await verifyAddress(vectors.link.cid, tampered)).toBe(false);
  });

  it('checks the codec the CID declares, as BlobCid::recompute_for_codec does', async () => {
    // The same digest under raw and under dag-cbor: both verify the bytes, because each recomputes
    // the CID under its own declared codec and the digest is the same.
    const digest = await sha256.digest(HELLO_WORLD);
    const raw = CID.create(1, RAW_CODEC, digest);
    const dagCbor = CID.create(1, DAG_CBOR_CODEC, digest);
    expect(await verifyAddress(raw, HELLO_WORLD)).toBe(true);
    expect(await verifyAddress(dagCbor, HELLO_WORLD)).toBe(true);
  });
});

describe('verifyAddress — legacy markers, for reads only', () => {
  const digestHex = 'b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9';

  it('accepts sha256-<hex> and sha256:<hex>', async () => {
    expect(addressForm(`sha256-${digestHex}`)).toBe('legacy');
    expect(await verifyAddress(`sha256-${digestHex}`, HELLO_WORLD)).toBe(true);
    expect(await verifyAddress(`sha256:${digestHex}`, HELLO_WORLD)).toBe(true);
    // The legacy digest is the raw link vector's digest.
    expect(hex(CID.parse(vectors.link.cid).multihash.digest)).toBe(digestHex);
  });

  it('fails closed on uppercase, short, bare or wrapped-CID legacy forms', async () => {
    for (const bad of [
      `sha256-${digestHex.toUpperCase()}`,
      `sha256-${digestHex.slice(2)}`,
      digestHex,
      `sha256-${vectors.link.cid}`,
      `blake3-${digestHex}`,
    ]) {
      expect(addressForm(bad), bad).toBeNull();
      expect(await verifyAddress(bad, HELLO_WORLD), bad).toBe(false);
    }
  });
});

describe('verifyAddress — everything else fails closed', () => {
  it('refuses CIDv0, other codecs, other hashes, and garbage', async () => {
    const digest = await sha256.digest(HELLO_WORLD);
    const v0 = CID.createV0(digest);
    const dagPb = CID.create(1, 0x70, digest);
    const dagJson = CID.create(1, 0x0129, digest);
    const ident = CID.create(1, RAW_CODEC, identity.digest(HELLO_WORLD));
    for (const bad of [v0, dagPb, dagJson, ident]) {
      expect(addressForm(bad)).toBeNull();
      expect(await verifyAddress(bad, HELLO_WORLD)).toBe(false);
      expect(await verifyAddress(bad.toString(), HELLO_WORLD)).toBe(false);
    }
    for (const bad of ['', 'not-a-cid', 'bafy', 'Qm']) {
      expect(await verifyAddress(bad, HELLO_WORLD)).toBe(false);
    }
  });

  it('takes a CID-like object at its bytes: a multihash that disagrees with them fails closed', async () => {
    // `CID.asCID` rebuilds a structurally compatible object (`'/' === bytes`, as survives a
    // structured clone) from its own `multihash` without checking it against `bytes`, while
    // `toString()` derives from `bytes`. A forged object would otherwise verify one digest and
    // be keyed, served and printed as another.
    const target = CID.create(1, RAW_CODEC, await sha256.digest(HELLO_WORLD));
    const forged = new TextEncoder().encode('forged');
    const fake = {
      version: 1,
      code: RAW_CODEC,
      multihash: await sha256.digest(forged),
      bytes: target.bytes,
      '/': target.bytes,
    };
    const rebuilt = CID.asCID(fake);
    expect(rebuilt?.toString()).toBe(target.toString()); // the hazard: same text, other digest
    expect(addressForm(fake as unknown as CID)).toBeNull();
    expect(await verifyAddress(fake as unknown as CID, forged)).toBe(false);
    expect(await verifyAddress(fake as unknown as CID, HELLO_WORLD)).toBe(false);
    // A consistent CID-like object still verifies.
    const honest = { version: 1, code: RAW_CODEC, multihash: target.multihash, bytes: target.bytes, '/': target.bytes };
    expect(await verifyAddress(honest as unknown as CID, HELLO_WORLD)).toBe(true);
  });
});

describe('cborItemLength / decodeCanonicalSequence', () => {
  it('splits a CBOR sequence of maps with tag-42 links and byte strings', () => {
    const link = CID.parse(vectors.link.cid);
    const a = encodeCanonical({ cid: link, paths: [[new Uint8Array([0x61])]], bytes: HELLO_WORLD });
    const b = encodeCanonical({ absent: [] });
    const seq = new Uint8Array([...a, ...b]);
    expect(cborItemLength(seq, 0)).toBe(a.length);
    expect(cborItemLength(seq, a.length)).toBe(b.length);
    const items = decodeCanonicalSequence<Record<string, unknown>>(seq);
    expect(items).toHaveLength(2);
    expect(CID.asCID(items[0].cid)?.toString()).toBe(vectors.link.cid);
    expect(items[0].bytes).toEqual(HELLO_WORLD);
    expect(items[1]).toEqual({ absent: [] });
  });

  it('answers null for every truncated prefix and frames the vector link exactly', () => {
    const link = unhex(vectors.link.dagCborHex);
    expect(cborItemLength(link)).toBe(link.length);
    for (let cut = 0; cut < link.length; cut++) {
      expect(cborItemLength(link.subarray(0, cut))).toBeNull();
    }
    expect(() => decodeCanonicalSequence(link.subarray(0, link.length - 1))).toThrow();
  });

  it('CborItemFramer resumed byte by byte agrees with a fresh walk at every prefix', () => {
    const link = CID.parse(vectors.link.cid);
    const item = encodeCanonical({
      cid: link,
      paths: [[new Uint8Array([0x61]), new Uint8Array(300)]],
      bytes: HELLO_WORLD,
      n: [1.5, 2n ** 60n, -1, null, true],
    });
    const framer = new CborItemFramer();
    for (let cut = 0; cut <= item.length; cut++) {
      expect(framer.frame(item.subarray(0, cut))).toBe(cborItemLength(item.subarray(0, cut)));
    }
    framer.reset();
    expect(framer.frame(item)).toBe(item.length);
  });

  it('CborItemFramer visits each head once across chunks: linear, not quadratic, in a tiny-element array', () => {
    const n = 1 << 18;
    const item = new Uint8Array(5 + n);
    item.set([0x9a, (n >>> 24) & 0xff, (n >>> 16) & 0xff, (n >>> 8) & 0xff, n & 0xff]);
    item.fill(0x01, 5);
    const framer = new CborItemFramer();
    let calls = 0;
    let length: number | null = null;
    for (let end = 1024; length === null; end = Math.min(end + 1024, item.length)) {
      calls++;
      length = framer.frame(item.subarray(0, end));
    }
    expect(length).toBe(item.length);
    expect(calls).toBeGreaterThan(250);
    expect(framer.visits).toBeLessThanOrEqual(n + 1 + calls);
  });

  it('frames float64 items and integers at or above 2^53, whose argument is a value', () => {
    const float = encodeCanonical({ x: 1.5 });
    expect(Array.from(float.subarray(3, 4))).toEqual([0xfb]); // DAG-CBOR writes every float as 64-bit
    const big = encodeCanonical({ n: 2n ** 60n });
    const neg = encodeCanonical({ n: -(2n ** 63n) });
    const max = encodeCanonical(2n ** 64n - 1n);
    const tagged = encodeCanonical([1.5, -0.25, 2n ** 60n]);
    for (const item of [float, big, neg, max, tagged]) {
      expect(cborItemLength(item)).toBe(item.length);
      for (let cut = 0; cut < item.length; cut++) expect(cborItemLength(item.subarray(0, cut))).toBeNull();
    }
    const seq = new Uint8Array([...float, ...big, ...neg, ...max, ...tagged]);
    expect(decodeCanonicalSequence(seq)).toEqual([{ x: 1.5 }, { n: 2n ** 60n }, { n: -(2n ** 63n) }, 2n ** 64n - 1n, [1.5, -0.25, 2n ** 60n]]);
  });

  it('still refuses a byte-string length or element count past 2^53', () => {
    const head = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
    expect(() => cborItemLength(new Uint8Array([0x5b, ...head]))).toThrow(/length too large/);
    expect(() => cborItemLength(new Uint8Array([0x9b, ...head]))).toThrow(/length too large/);
  });

  it('refuses indefinite lengths, which DAG-CBOR forbids', () => {
    expect(() => cborItemLength(new Uint8Array([0x9f, 0x01, 0xff]))).toThrow();
    expect(() => cborItemLength(new Uint8Array([0x5f]))).toThrow();
  });
});
