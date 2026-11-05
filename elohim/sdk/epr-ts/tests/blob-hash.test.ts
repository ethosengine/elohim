import { describe, expect, it } from 'vitest';
import { verifyBlobHash } from '../src/blob-hash';

/**
 * Cross-language vector set over `b"hello world"`: the byte CID (raw 0x55 + blake3-256 0x1e) and the
 * citation-class raw+sha2 CID. Derivations are recorded in `address.test.ts`.
 */
const BYTES = new TextEncoder().encode('hello world');
const BYTE_CID = 'bafkr4igxjga67jykbseaxdmmdgc5a5o3zp3htom2l6mrjznk7fvyggu6eq';
const SHA2_RAW_CID = 'bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e';
const SHA2_HEX = 'b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9';

describe('verifyBlobHash — match', () => {
  it('accepts the byte CID', async () => {
    expect(await verifyBlobHash(BYTES, BYTE_CID)).toBe('match');
  });

  it('accepts a citation-class raw+sha2 CID over the same bytes', async () => {
    expect(await verifyBlobHash(BYTES, SHA2_RAW_CID)).toBe('match');
  });

  it('trims surrounding whitespace', async () => {
    expect(await verifyBlobHash(BYTES, `  ${BYTE_CID}\n`)).toBe('match');
  });
});

describe('verifyBlobHash — mismatch', () => {
  it('rejects tampered bytes', async () => {
    const tampered = new Uint8Array(BYTES);
    tampered[0] = tampered[0] ^ 0xff;
    expect(await verifyBlobHash(tampered, BYTE_CID)).toBe('mismatch');
  });

  it('rejects a well-formed byte CID naming a different digest', async () => {
    expect(await verifyBlobHash(BYTES, 'bafkr4ideg6z2yocgkez77nr3outtvdnvjdcvqrs5phnqh7jvtrwnlpm5qu')).toBe(
      'mismatch',
    );
  });
});

describe('verifyBlobHash — unverifiable', () => {
  it('reports every retired spelling as unverifiable, not tampering', async () => {
    for (const retired of [
      `sha256-${SHA2_HEX}`,
      `sha256:${SHA2_HEX}`,
      SHA2_HEX,
      `blake3-${'b'.repeat(64)}`,
      `sha256-${SHA2_RAW_CID}`,
    ]) {
      expect(await verifyBlobHash(BYTES, retired), retired).toBe('unverifiable');
    }
  });

  it('reports garbage as unverifiable', async () => {
    expect(await verifyBlobHash(BYTES, 'sha256-shardA')).toBe('unverifiable');
  });

  it('reports an empty address as unverifiable', async () => {
    expect(await verifyBlobHash(BYTES, '')).toBe('unverifiable');
  });

  it('reports a NULL pointer as unverifiable', async () => {
    expect(await verifyBlobHash(BYTES, null)).toBe('unverifiable');
    expect(await verifyBlobHash(BYTES, undefined)).toBe('unverifiable');
  });
});
