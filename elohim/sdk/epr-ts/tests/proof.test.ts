import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { verifyEd25519 } from '../src/proof';

function hexToBytes(hex: string): Uint8Array {
  return Uint8Array.from(Buffer.from(hex, 'hex'));
}

describe('Ed25519 verify', () => {
  it('RFC 8032 test 1', async () => {
    const pk = hexToBytes('d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a');
    const msg = new Uint8Array();
    const sig = hexToBytes(
      'e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b'
    );
    expect(await verifyEd25519(pk, msg, sig)).toBe(true);
  });

  it('verifies Rust-generated vectors', async () => {
    const path = join(process.cwd(), '../../epr/tests/vectors/signed_eprs.json');
    const vectors = JSON.parse(readFileSync(path, 'utf-8'));

    for (const v of vectors) {
      const pk = hexToBytes(v.public_key_hex);
      const canonical = hexToBytes(v.canonical_bytes_hex);
      const sig = hexToBytes(v.signature_hex);
      expect(await verifyEd25519(pk, canonical, sig)).toBe(true);
    }
  });

  it('rejects wrong signature', async () => {
    const pk = hexToBytes('d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a');
    const msg = new Uint8Array([1, 2, 3]);
    const badSig = new Uint8Array(64);
    expect(await verifyEd25519(pk, msg, badSig)).toBe(false);
  });
});
