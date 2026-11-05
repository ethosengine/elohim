/**
 * Content-address VERIFICATION for a receiver.
 *
 * The compose-gate's `interface-first-reuse-ts` rule is right that TypeScript
 * receives computed identity and never mints it, so it is worth saying which
 * of the two this is. Minting is computing an address to NAME new content —
 * that stays in Rust (`elohim-address`, `elohim-epr`), and nothing here does
 * it. This module only ever answers "do these bytes I was handed match the
 * address I was told to expect?", which is the receiver half of the substrate
 * trust contract's verify-locally-then-serve invariant. A browser has no
 * conductor to delegate that to; if it cannot hash, it cannot check, and it
 * is reduced to trusting whoever answered.
 *
 * It delegates to `verifyAddress` (`./address`), the one verifier; it takes no
 * digest of its own and parses no address grammar of its own. There are no
 * legacy spellings: `sha256-<hex>`, `blake3-<hex>`, bare hex and every other
 * non-canonical form are not addresses and read as `unverifiable`.
 */
import { addressForm, verifyAddress } from './address.js';

/**
 * The three answers a content-address check can honestly give.
 *
 * `unverifiable` is not a soft `mismatch`: an address this verifier does not
 * accept (not a canonical byte, raw-sha2 or dag-cbor CIDv1) and a NULL/absent
 * pointer both mean "this substrate handed us no commitment to check against".
 * Collapsing those into `mismatch` reads every one of them as tampering.
 */
export type BlobHashVerdict = 'match' | 'mismatch' | 'unverifiable';

/**
 * Check bytes a peer handed us against the content address the row declared.
 *
 * This is the three-answer READER, not the admission verifier (rail R-T3): it
 * answers `unverifiable` where `verifyAddress` would only say `false`, so a
 * report can tell a tampered row from a row that names no checkable address.
 * `expected` is whatever the row carried — including `null`/`undefined` for a
 * row with no pointer at all, which is `unverifiable`, never `mismatch`.
 */
export async function verifyBlobHash(
  bytes: Uint8Array,
  expected: string | null | undefined,
): Promise<BlobHashVerdict> {
  if (expected === null || expected === undefined || expected.trim() === '') {
    return 'unverifiable';
  }
  const address = expected.trim();
  // `verifyAddress` is false both for a digest mismatch and for an address it refuses; the
  // reader needs to tell them apart, so it asks the verifier's own parser first.
  if (addressForm(address) === null) return 'unverifiable';
  return (await verifyAddress(address, bytes)) ? 'match' : 'mismatch';
}
