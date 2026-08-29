import { CID } from 'multiformats/cid';
import { sha256 } from '@noble/hashes/sha2';
import { bytesToHex } from '@noble/hashes/utils';

/** Multihash code for sha2-256, per the multicodec table. */
const SHA2_256_CODE = 0x12;

/**
 * The three answers a content-address check can honestly give.
 *
 * `unverifiable` is not a soft `mismatch`: a blake3-prefixed address, a CID
 * wrapping a non-sha2-256 multihash, and a NULL/absent `blob_hash` all mean
 * "this substrate handed us no sha256 commitment to check against" — 27 of
 * 3442 rows on the live household mesh carry a NULL `blob_hash`. Collapsing
 * those into `mismatch` reads every one of them as tampering.
 */
export type BlobHashVerdict = 'match' | 'mismatch' | 'unverifiable';

/**
 * The sha256 hex digest a content address names, for any recognizable form:
 *
 * - CIDv1 (`bafkrei…` raw, `bafyrei…` dag-cbor — both carry a sha2-256
 *   multihash): the wrapped digest.
 * - `sha256-<64 hex>` / `sha256:<64 hex>` (the canonical legacy markers — the
 *   dash form on the blob wire, the colon form in the concern registries):
 *   the hex part.
 * - bare 64-hex: itself.
 * - `sha256-<CID>` (the double-wrapped seed defect — a CID-form blob hash
 *   wrapped in the legacy marker by an address constructor that assumed bare
 *   hex): the CID's wrapped digest.
 *
 * Returns `null` for anything else (blake3 addresses, slugs, garbage) so the
 * caller can report `unverifiable` rather than inventing a comparison.
 *
 * Ported from `elohim-storage`'s `content_address_hex`
 * (`elohim/elohim-storage/src/p2p/blob_fetch.rs`), with one deliberate
 * divergence: a CID whose multihash is not sha2-256 yields `null` here, since
 * this function's answer feeds a sha256 comparison.
 */
export function contentAddressHex(addr: string): string | null {
  // Hex forms FIRST: a bare 64-hex string starting with a multibase prefix
  // character (`f` = base16, `b` = base32) could otherwise be misread as a CID
  // by the parser.
  const bare = stripSha256Marker(addr);
  if (bare.length === 64 && isHex(bare)) {
    return bare.toLowerCase();
  }
  const direct = cidDigestHex(addr);
  if (direct !== null) return direct;
  // Double-wrapped `sha256-<cid>`: recover the digest from the inner CID.
  if (bare !== addr) {
    return cidDigestHex(bare);
  }
  return null;
}

/** The sha256 hex digest of `bytes` — the digest a content address commits to. */
export function sha256BlobHash(bytes: Uint8Array): string {
  return bytesToHex(sha256(bytes));
}

/**
 * Check bytes a peer handed us against the content address the row declared.
 *
 * `expected` is whatever the row carried — including `null`/`undefined` for a
 * row with no blob hash at all, which is `unverifiable`, never `mismatch`.
 */
export function verifyBlobHash(
  bytes: Uint8Array,
  expected: string | null | undefined,
): BlobHashVerdict {
  if (expected === null || expected === undefined || expected.trim() === '') {
    return 'unverifiable';
  }
  const wanted = contentAddressHex(expected.trim());
  if (wanted === null) return 'unverifiable';
  return sha256BlobHash(bytes) === wanted ? 'match' : 'mismatch';
}

/** Strip the legacy sha256 marker in either separator form. */
function stripSha256Marker(addr: string): string {
  if (addr.startsWith('sha256-')) return addr.slice('sha256-'.length);
  if (addr.startsWith('sha256:')) return addr.slice('sha256:'.length);
  return addr;
}

function isHex(s: string): boolean {
  return /^[0-9a-fA-F]+$/.test(s);
}

/** The sha2-256 digest a CID string wraps, or `null` if it is not one. */
function cidDigestHex(s: string): string | null {
  try {
    const cid = CID.parse(s);
    if (cid.multihash.code !== SHA2_256_CODE) return null;
    return bytesToHex(cid.multihash.digest);
  } catch {
    return null;
  }
}
