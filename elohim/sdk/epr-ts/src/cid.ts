import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';

import { addressForm, DAG_CBOR_CODEC, verifyAddress } from './address.js';

/** Compute CIDv1(dag-cbor, sha2-256) over canonical bytes. */
export async function computeCid(canonicalBytes: Uint8Array): Promise<CID> {
  const hash = await sha256.digest(canonicalBytes);
  return CID.create(1, DAG_CBOR_CODEC, hash);
}

/**
 * Verify a claimed dag-cbor CID matches the hash of the given canonical bytes.
 *
 * @deprecated Use `verifyAddress` (rail R-T3: one TypeScript verifier). This is `verifyAddress`
 * narrowed to the dag-cbor codec, which is what it always checked: a raw (`bafkrei…`) CID over
 * the same digest answers `false` here and `true` from `verifyAddress`.
 */
export async function verifyCid(claimed: CID, canonicalBytes: Uint8Array): Promise<boolean> {
  return addressForm(claimed) === 'dag-cbor' && verifyAddress(claimed, canonicalBytes);
}
