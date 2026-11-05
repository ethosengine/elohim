/**
 * Test-only byte-CID minting: a REAL address from REAL bytes, never a header-swapped string.
 *
 * Production TypeScript only verifies addresses (`@elohim/epr` `verifyAddress`); minting stays in
 * Rust. Specs need fixtures, so this mints them the one way the byte profile defines: CIDv1,
 * codec raw (0x55), multihash BLAKE3-256 (0x1e), 32-byte digest -> `bafkr4i…` (59 characters).
 */
import { blake3 } from '@noble/hashes/blake3';
import { CID } from 'multiformats/cid';
import { create as createDigest } from 'multiformats/hashes/digest';

const RAW = 0x55;
const BLAKE3_256 = 0x1e;

/** The byte CID of `bytes`. */
export function byteCidOf(bytes: Uint8Array): CID {
  // Node-realm copy: jsdom's TextEncoder output fails multiformats' instanceof checks.
  const input = new Uint8Array(bytes);
  return CID.createV1(RAW, createDigest(BLAKE3_256, blake3(input, { dkLen: 32 })));
}

/** The byte CID text of the UTF-8 encoding of `text`. */
export function byteCidOfText(text: string): string {
  return byteCidOf(new Uint8Array(new TextEncoder().encode(text))).toString();
}
