import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { computeCid } from '../src/cid';

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
