/**
 * `verifyAddress`: the one TypeScript verifier (rail R-T3, interfaces spec §1.3 and §H.1).
 *
 * Rust has one verifier, `eprfs_core::BlobCid::verifies`; TypeScript has this one. Both answer
 * the receiver's question "do these bytes match the address I was told to expect?" and never mint
 * an address (minting stays in Rust). The agreement is pinned by the shared vectors in
 * `elohim/eprfs/fixtures/tree-vectors.json`, which Rust wrote and `tests/address.test.ts` reads.
 *
 * Accepted:
 * - CIDv1 with a sha2-256 multihash (32-byte digest) and codec raw (`0x55`, `bafkrei…`) or
 *   DAG-CBOR (`0x71`, `bafyrei…`): the bytes verify when their sha2-256 digest equals the CID's.
 *   This is exactly `BlobCid::verifies`, which recomputes the CID under the declared codec.
 * - The legacy markers `sha256-<64 lowercase hex>` and `sha256:<64 lowercase hex>`, for reads
 *   only: an old row names its bytes this way, so a reader may check them, but nothing new is
 *   addressed by a legacy marker.
 *
 * Everything else fails closed: CIDv0, any other codec (dag-pb, dag-json, …), any other hash
 * function or digest length, uppercase or short hex, and strings that do not parse.
 *
 * `raw-cid-verify.ts` is a thin raw-only wrapper: its callers depend on a DAG-CBOR address
 * failing closed there, so it checks `addressForm(...)` before delegating here.
 */
import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';

/** Multicodec codes this verifier accepts. */
export const RAW_CODEC = 0x55;
export const DAG_CBOR_CODEC = 0x71;
const SHA2_256_CODE = 0x12;
const SHA2_256_BYTES = 32;
const LEGACY = /^sha256[-:]([0-9a-f]{64})$/;

/** Which accepted form an address takes, or `null` when it is not one this verifier checks. */
export type AddressForm = 'raw' | 'dag-cbor' | 'legacy';

interface Parsed {
  form: AddressForm;
  digest: Uint8Array;
}

function fromCid(cid: CID): Parsed | null {
  if (cid.version !== 1) return null;
  if (cid.multihash.code !== SHA2_256_CODE || cid.multihash.size !== SHA2_256_BYTES) return null;
  if (cid.multihash.digest.length !== SHA2_256_BYTES) return null;
  if (cid.code === RAW_CODEC) return { form: 'raw', digest: cid.multihash.digest };
  if (cid.code === DAG_CBOR_CODEC) return { form: 'dag-cbor', digest: cid.multihash.digest };
  return null;
}

function unhex(text: string): Uint8Array {
  const out = new Uint8Array(text.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = Number.parseInt(text.slice(2 * i, 2 * i + 2), 16);
  return out;
}

function parse(address: CID | string): Parsed | null {
  if (typeof address !== 'string') {
    // A CID instance from any copy of multiformats, read through `CID.asCID`, which accepts
    // structurally compatible objects and returns null for anything else. `asCID` rebuilds such an
    // object from its own `multihash` without checking it against its `bytes`, and `toString()`
    // (what a cache keys by and a log prints) derives from `bytes`. So the address is taken from
    // the bytes alone: an object whose `multihash` disagrees with its `bytes` fails closed, and
    // the digest checked is always the one the CID's text names.
    const cid = CID.asCID(address);
    if (cid === null) return null;
    let canon: CID;
    try {
      canon = CID.decode(cid.bytes);
    } catch {
      return null;
    }
    return canon.equals(cid) ? fromCid(canon) : null;
  }
  const legacy = LEGACY.exec(address);
  if (legacy) return { form: 'legacy', digest: unhex(legacy[1]) };
  let cid: CID;
  try {
    cid = CID.parse(address);
  } catch {
    return null;
  }
  return fromCid(cid);
}

/** The accepted form of `address`, or `null` when `verifyAddress` would fail it closed. */
export function addressForm(address: CID | string): AddressForm | null {
  return parse(address)?.form ?? null;
}

function viewOf(bytes: Uint8Array | ArrayBuffer): Uint8Array {
  return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
}

/**
 * True iff `bytes` hash to the digest `address` commits to under an accepted form. Never throws:
 * an address this verifier does not accept answers `false`.
 */
export async function verifyAddress(
  address: CID | string,
  bytes: Uint8Array | ArrayBuffer,
): Promise<boolean> {
  const parsed = parse(address);
  if (parsed === null) return false;
  const digest = await sha256.digest(viewOf(bytes));
  const got = digest.digest;
  if (got.length !== parsed.digest.length) return false;
  let diff = 0;
  for (let i = 0; i < got.length; i++) diff |= got[i] ^ parsed.digest[i];
  return diff === 0;
}
