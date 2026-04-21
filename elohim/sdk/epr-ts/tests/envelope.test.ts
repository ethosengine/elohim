import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { canonicalEnvelopeBytes } from '../src/envelope';
import type { Envelope } from '../src/generated/Envelope';

function hexToBytes(hex: string): Uint8Array {
  return Uint8Array.from(Buffer.from(hex, 'hex'));
}

describe('canonical envelope bytes', () => {
  it('matches Rust canonical_bytes_hex for every vector', async () => {
    const path = join(process.cwd(), '../../epr/tests/vectors/signed_eprs.json');
    const vectors = JSON.parse(readFileSync(path, 'utf-8'));

    for (const v of vectors) {
      const env: Envelope = v.envelope;
      const payload = hexToBytes(v.payload_hex);
      const derived = await canonicalEnvelopeBytes(env, payload);
      const derivedHex = Buffer.from(derived).toString('hex');
      expect(derivedHex).toBe(v.canonical_bytes_hex);
    }
  });
});
