/**
 * The ONE byte-address mint in the seeder (regenesis-1 address crossing).
 *
 * Every blob byte string is addressed by a CIDv1 over the raw codec (0x55) and a
 * BLAKE3-256 multihash (0x1e): `bafkr4i…`, 59 chars. This is the same profile the
 * kernel (`elohim-epr` `BYTE_PROFILE_0`) declares; the seeder mirrors it rather
 * than inventing a spelling. No `sha256-` / `blake3-` / `bafkrei` mint exists here.
 *
 * Record hashes (`seed-hash.ts`) and cite fingerprints are a different class and
 * do not use this helper.
 */
import { blake3 } from '@noble/hashes/blake3';
import { CID } from 'multiformats/cid';
import * as Digest from 'multiformats/hashes/digest';

export const BYTE_CODEC = 0x55;
export const BYTE_MULTIHASH = 0x1e;
export const BYTE_CID_PREFIX = 'bafkr4i';
export const BYTE_CID_LEN = 59;

const BYTE_CID_RE = /^bafkr4i[a-z2-7]{52}$/;

/** CIDv1 (raw + blake3-256) of `bytes`, as its canonical base32-lower string. */
export function byteCid(bytes: Uint8Array): string {
  return CID.createV1(BYTE_CODEC, Digest.create(BYTE_MULTIHASH, blake3(bytes))).toString();
}

/** True iff `s` is spelled as a byte-profile CID (length, prefix, alphabet). */
export function isByteCid(s: unknown): s is string {
  return typeof s === 'string' && s.length === BYTE_CID_LEN && BYTE_CID_RE.test(s);
}
