/**
 * The byte-profile address for a2o steps and scripts: CIDv1 raw (0x55) + BLAKE3-256 (0x1e),
 * base32-lower (`bafkr4i…`, 59 characters). Admission is by FULL decode — version, codec,
 * multihash code, digest length and canonical text — never by a spelling or prefix test.
 * Same mint as `scripts/ci/lib/byte-cid.mjs` and `elohim_address::ByteCid`; the kernel
 * vectors in `elohim/eprfs/fixtures/tree-vectors.json` pin both.
 */
import { blake3 } from '@noble/hashes/blake3';
import { CID } from 'multiformats/cid';
import { create as createDigest } from 'multiformats/hashes/digest';

const RAW_CODEC = 0x55;
const BLAKE3_256_CODE = 0x1e;

/** Mint the byte CID of `data`. */
export function mintByteCid(data: Uint8Array): string {
  return CID.createV1(
    RAW_CODEC,
    createDigest(BLAKE3_256_CODE, blake3(data, { dkLen: 32 }))
  ).toString();
}

/**
 * The canonical byte CID text of `address`, or `null` when it does not fully decode as a
 * CIDv1 raw + BLAKE3-256 address (a `sha256-` spelling, bare hex, a raw+sha2 CID, a record
 * CID, a non-canonical text — all `null`).
 */
export function parseByteCid(address: unknown): string | null {
  if (typeof address !== 'string') return null;
  let cid: CID;
  try {
    cid = CID.parse(address);
  } catch {
    return null;
  }
  if (cid.version !== 1 || cid.code !== RAW_CODEC) return null;
  if (cid.multihash.code !== BLAKE3_256_CODE || cid.multihash.digest.length !== 32) return null;
  return cid.toString() === address && address.length === 59 ? address : null;
}

export function isByteCid(address: unknown): address is string {
  return parseByteCid(address) !== null;
}

/** Throwing admission for step assertions. */
export function assertByteCid(address: unknown, what = 'address'): string {
  const cid = parseByteCid(address);
  if (cid === null) {
    throw new Error(
      `${what} "${String(address)}" is not a byte CID (raw + BLAKE3-256, bafkr4i…, 59 chars)`
    );
  }
  return cid;
}
