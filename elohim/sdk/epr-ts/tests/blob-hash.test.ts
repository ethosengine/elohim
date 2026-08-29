import { describe, expect, it } from 'vitest';
import { contentAddressHex, sha256BlobHash, verifyBlobHash } from '../src/blob-hash';

/**
 * The same fixture the Rust unit tests use
 * (`elohim/elohim-storage/src/p2p/blob_fetch.rs`, `known_blob()`), so the two
 * suites form one cross-language vector set over `b"hello world"`.
 */
const BYTES = new TextEncoder().encode('hello world');
const HEX = 'b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9';
/** CIDv1(raw, sha2-256) over the same digest — the `bafkrei…` form on the wire. */
const CID_FORM = 'bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e';

describe('sha256BlobHash', () => {
  it('matches the Rust fixture digest', () => {
    expect(sha256BlobHash(BYTES)).toBe(HEX);
  });
});

describe('verifyBlobHash — match', () => {
  it('accepts bare hex', () => {
    expect(verifyBlobHash(BYTES, HEX)).toBe('match');
  });

  it('accepts the sha256: marker form', () => {
    expect(verifyBlobHash(BYTES, `sha256:${HEX}`)).toBe('match');
  });

  it('accepts the sha256- marker form', () => {
    expect(verifyBlobHash(BYTES, `sha256-${HEX}`)).toBe('match');
  });

  it('accepts uppercase hex', () => {
    expect(verifyBlobHash(BYTES, HEX.toUpperCase())).toBe('match');
  });

  it('accepts CID form', () => {
    expect(verifyBlobHash(BYTES, CID_FORM)).toBe('match');
  });

  it('accepts the double-wrapped sha256-<CID> seed defect', () => {
    expect(verifyBlobHash(BYTES, `sha256-${CID_FORM}`)).toBe('match');
  });
});

describe('verifyBlobHash — mismatch', () => {
  it('rejects tampered bytes', () => {
    const tampered = new Uint8Array(BYTES);
    tampered[0] = tampered[0] ^ 0xff;
    expect(verifyBlobHash(tampered, HEX)).toBe('mismatch');
  });

  it('rejects a well-formed address naming a different digest', () => {
    expect(verifyBlobHash(BYTES, 'a'.repeat(64))).toBe('mismatch');
  });
});

describe('verifyBlobHash — unverifiable', () => {
  it('reports a blake3-prefixed address as unverifiable, not tampering', () => {
    expect(verifyBlobHash(BYTES, `blake3-${'b'.repeat(64)}`)).toBe('unverifiable');
  });

  it('reports garbage as unverifiable', () => {
    expect(verifyBlobHash(BYTES, 'sha256-shardA')).toBe('unverifiable');
  });

  it('reports an empty address as unverifiable', () => {
    expect(verifyBlobHash(BYTES, '')).toBe('unverifiable');
  });

  it('reports a NULL blob_hash as unverifiable', () => {
    // 27 of 3442 rows on the live household mesh carry a NULL blob_hash.
    expect(verifyBlobHash(BYTES, null)).toBe('unverifiable');
    expect(verifyBlobHash(BYTES, undefined)).toBe('unverifiable');
  });
});

describe('contentAddressHex', () => {
  it('parses hex before CID so a b-leading bare hex is not read as a CID', () => {
    // This fixture's own digest starts with `b` — the precedence trap.
    expect(HEX.startsWith('b')).toBe(true);
    expect(contentAddressHex(HEX)).toBe(HEX);
  });

  it('unwraps a production corpus CID to its multihash digest', () => {
    expect(contentAddressHex('bafkreidjt7llifx7bw362aytrbsd3iymo77tjqrxbqnrvabfry2w5z4lwq')).toBe(
      '699fd6b416ff0db7ed031388643da30c77ff34c2370c1b1a80258e356ee78bb4',
    );
  });

  it('returns null for a non-address string', () => {
    expect(contentAddressHex('elohim-host-landing')).toBeNull();
  });
});
