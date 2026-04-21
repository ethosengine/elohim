import { describe, expect, it } from 'vitest';
import { encodeCanonical, decodeCanonical } from '../src/cbor';

describe('canonical CBOR', () => {
  it('roundtrips primitives', () => {
    for (const v of [null, true, false, 0, 42, -7, 'hello', new Uint8Array([1, 2, 3])]) {
      const bytes = encodeCanonical(v);
      const decoded = decodeCanonical(bytes);
      expect(decoded).toEqual(v);
    }
  });

  it('produces deterministic bytes regardless of map insertion order', () => {
    const a = encodeCanonical({ b: 2, a: 1 });
    const b = encodeCanonical({ a: 1, b: 2 });
    expect(a).toEqual(b);
  });
});
