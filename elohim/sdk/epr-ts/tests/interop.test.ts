import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { verifyEpr, type Epr } from '../src/epr';
import type { Envelope } from '../src/generated/Envelope';

function hexToBytes(hex: string): Uint8Array {
  return Uint8Array.from(Buffer.from(hex, 'hex'));
}

describe('Rust ↔ TS interop', () => {
  it('verifies every signed Rust vector', async () => {
    const path = join(process.cwd(), '../../epr/tests/vectors/signed_eprs.json');
    const vectors = JSON.parse(readFileSync(path, 'utf-8'));

    for (const v of vectors) {
      const envelope = v.envelope as Envelope;
      const payload = hexToBytes(v.payload_hex);
      const publicKey = hexToBytes(v.public_key_hex);
      const epr: Epr = { envelope, payload };

      const result = await verifyEpr(epr, publicKey);
      if (!result.ok) {
        throw new Error(`vector "${v.label}" failed: ${result.error.kind} ${result.error.message}`);
      }
    }
  });

  it('rejects tampered payload', async () => {
    const path = join(process.cwd(), '../../epr/tests/vectors/signed_eprs.json');
    const vectors = JSON.parse(readFileSync(path, 'utf-8'));
    const v = vectors[0];

    const envelope = v.envelope as Envelope;
    const tampered = hexToBytes(v.payload_hex);
    if (tampered.length > 0) tampered[0] = tampered[0] ^ 0xff;
    const publicKey = hexToBytes(v.public_key_hex);

    const result = await verifyEpr({ envelope, payload: tampered }, publicKey);
    expect(result.ok).toBe(false);
  });
});
