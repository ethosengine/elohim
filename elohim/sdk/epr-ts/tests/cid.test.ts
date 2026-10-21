import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { CID } from 'multiformats/cid';
import { computeCid, verifyCid } from '../src/cid';

function hexToBytes(hex: string): Uint8Array {
  return Uint8Array.from(Buffer.from(hex, 'hex'));
}

describe('CID derivation', () => {
  it('stable across calls', async () => {
    const bytes = new Uint8Array([1, 2, 3, 4]);
    const a = await computeCid(bytes);
    const b = await computeCid(bytes);
    expect(a.toString()).toBe(b.toString());
  });

  it('matches Rust vectors', async () => {
    // Relative path: cwd is elohim/sdk/epr-ts when vitest runs
    const path = join(process.cwd(), '../../epr/tests/vectors/signed_eprs.json');
    const raw = readFileSync(path, 'utf-8');
    const vectors = JSON.parse(raw);

    for (const v of vectors) {
      const canonical = hexToBytes(v.canonical_bytes_hex);
      const cid = await computeCid(canonical);
      expect(cid.toString()).toBe(v.cid);
    }
  });
});

describe('verifyCid (deprecated: verifyAddress narrowed to dag-cbor)', () => {
  it('accepts the dag-cbor CID of the bytes and refuses a raw CID over the same digest', async () => {
    const bytes = new Uint8Array([1, 2, 3, 4]);
    const cid = await computeCid(bytes);
    expect(await verifyCid(cid, bytes)).toBe(true);
    expect(await verifyCid(cid, new Uint8Array([1, 2, 3, 5]))).toBe(false);
    const raw = CID.create(1, 0x55, cid.multihash);
    expect(await verifyCid(raw, bytes)).toBe(false);
  });
});
