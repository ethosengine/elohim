/**
 * Bytes prove themselves: check raw bytes against the content address they
 * were requested by, before anything caches or renders them.
 *
 * A small peer may hand over raw bytes it cannot afford to extract or vouch
 * for; the client does the work, so verification lives here. Pure — no
 * Angular, no DOM — so the apps service worker bundles it too.
 *
 * Two spellings of one address are accepted:
 * - `sha256-<64 lowercase hex>` (also the legacy `sha256:<hex>` separator)
 * - CIDv1 raw + sha2-256 (`bafkrei…`: version 1, codec 0x55, multihash 0x12, 32-byte digest)
 *
 * Anything else — dag-cbor CIDs, other hash functions, malformed strings — is
 * unverifiable here and fails closed.
 */
import { CID } from 'multiformats/cid';

const RAW_CODEC = 0x55;
const SHA2_256_CODE = 0x12;
const SHA256_BYTES = 32;
const HEX_DIGEST = /^[0-9a-f]{64}$/;

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

/**
 * The SHA-256 digest (lowercase hex) a raw content address commits to, or
 * null when the address is not a form whose bytes can be verified directly.
 */
export function expectedSha256Hex(address: string): string | null {
  const legacy = /^sha256[-:](.*)$/.exec(address);
  if (legacy) return HEX_DIGEST.test(legacy[1]) ? legacy[1] : null;

  let cid: CID;
  try {
    cid = CID.parse(address);
  } catch {
    return null;
  }
  if (
    cid.version !== 1 ||
    cid.code !== RAW_CODEC ||
    cid.multihash.code !== SHA2_256_CODE ||
    cid.multihash.size !== SHA256_BYTES
  ) {
    return null;
  }
  return toHex(cid.multihash.digest);
}

/**
 * True iff `bytes` hash to the digest `address` commits to. An address that
 * does not name a raw SHA-256 digest returns false (fail closed).
 */
export async function verifyRawSha256(
  bytes: ArrayBuffer | Uint8Array,
  address: string
): Promise<boolean> {
  const expected = expectedSha256Hex(address);
  if (expected === null) return false;
  const view = ArrayBuffer.isView(bytes) ? bytes : new Uint8Array(bytes);
  const digest = await crypto.subtle.digest('SHA-256', view as Uint8Array<ArrayBuffer>);
  return toHex(new Uint8Array(digest)) === expected;
}
