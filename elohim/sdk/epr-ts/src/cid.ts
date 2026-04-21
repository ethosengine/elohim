import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';

/** CIDv1 codec byte for dag-cbor per the IPLD multicodec table. */
const DAG_CBOR_CODEC = 0x71;

/** Compute CIDv1(dag-cbor, sha2-256) over canonical bytes. */
export async function computeCid(canonicalBytes: Uint8Array): Promise<CID> {
  const hash = await sha256.digest(canonicalBytes);
  return CID.create(1, DAG_CBOR_CODEC, hash);
}

/** Verify a claimed CID matches the hash of the given canonical bytes. */
export async function verifyCid(claimed: CID, canonicalBytes: Uint8Array): Promise<boolean> {
  const derived = await computeCid(canonicalBytes);
  return derived.toString() === claimed.toString();
}
