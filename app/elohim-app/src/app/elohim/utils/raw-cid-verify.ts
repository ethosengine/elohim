/**
 * Bytes prove themselves: check raw bytes against the byte CID they were
 * requested by, before anything caches or renders them.
 *
 * A small peer may hand over raw bytes it cannot afford to extract or vouch
 * for; the client does the work, so verification lives here. Pure — no
 * Angular, no DOM — so the apps service worker bundles it too.
 *
 * This is a thin wrapper over the one TypeScript verifier, `@elohim/epr`'s
 * `verifyAddress` (a FULL decode of the address, never a prefix check). The
 * byte plane admits exactly one spelling: a CIDv1 raw + BLAKE3-256 byte CID
 * (`bafkr4i…`, `addressForm === 'bytes'`). The other forms the verifier knows
 * (`raw` sha2-256 citation fingerprints, `dag-cbor` records) are real
 * addresses but not of bytes, so they are refused here; the legacy
 * `sha256-<hex>` / `sha256:<hex>` markers are not addresses at all.
 */
import { addressForm, verifyAddress } from '@elohim/epr';

/** True iff `address` is a canonical byte CID (the byte profile's one spelling). */
export function isByteCid(address: string): boolean {
  return addressForm(address) === 'bytes';
}

/**
 * True iff `bytes` hash (BLAKE3-256) to the digest the byte CID `address`
 * commits to. Any other address — wrong codec/hash, legacy marker, malformed —
 * returns false (fail closed).
 */
export async function verifyByteCid(
  bytes: ArrayBuffer | Uint8Array,
  address: string
): Promise<boolean> {
  return isByteCid(address) && (await verifyAddress(address, bytes));
}
