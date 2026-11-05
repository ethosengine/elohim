/**
 * `verifyAddress`: the one TypeScript verifier (rail R-T3, interfaces spec §1.3 and §H.1), the
 * twin of the Rust kernel `elohim-address` (`ByteCid::parse` + `verify_cid_any`).
 *
 * Both answer the receiver's question "do these bytes match the address I was told to expect?"
 * and never mint an address (minting stays in Rust). The agreement is pinned by the kernel's
 * vectors, `crates/elohim-address/vectors/address-vectors.json`, which `tests/address.test.ts`
 * reads: every positive row verifies here and every negative row is refused.
 *
 * Admitted — exactly three `(codec, multihash, digest length)` triples, each a CIDv1:
 * - `(raw 0x55, BLAKE3-256 0x1e, 32)`, form `bytes`: the byte profile, `bafkr4i…`, 59 characters.
 * - `(raw 0x55, sha2-256 0x12, 32)`, form `raw`: citation fingerprints, `bafkrei…`.
 * - `(dag-cbor 0x71, sha2-256 0x12, 32)`, form `dag-cbor`: records, `bafyrei…`.
 *
 * Admission is a FULL DECODE, never a prefix check: the text must parse as a CID; its binary
 * form must be the canonical encoding of its own parts (minimal varints — a version spelled
 * `81 00` is refused — and no bytes past the digest), so a byte CID is exactly
 * `01 55 1e 20 ‖ digest`; and the canonical base32-lower text of those bytes must equal the input
 * (non-zero padding bits, upper case, other multibases are refused). The legacy markers
 * `sha256-<hex>`, `sha256:<hex>` and `blake3-<hex>` are not addresses and are refused.
 *
 * Everything else fails closed: CIDv0, any other codec or hash, any other digest length, and
 * strings that do not parse. A digest match proves the bytes are the named bytes; it never grants
 * authorization.
 */
import { blake3 } from "@noble/hashes/blake3";
import { base32 } from "multiformats/bases/base32";
import { CID } from "multiformats/cid";
import { create as createDigest } from "multiformats/hashes/digest";
import { from as hasherFrom } from "multiformats/hashes/hasher";
import { sha256 } from "multiformats/hashes/sha2";

/** Multicodec codes this verifier accepts. */
export const RAW_CODEC = 0x55;
export const DAG_CBOR_CODEC = 0x71;
export const SHA2_256_CODE = 0x12;
export const BLAKE3_256_CODE = 0x1e;
const DIGEST_BYTES = 32;

/** BLAKE3-256 (unkeyed, 32-byte output) as a multiformats hasher, multihash code `0x1e`. */
const blake3Hasher = hasherFrom({
  name: "blake3",
  code: BLAKE3_256_CODE,
  encode: (input: Uint8Array) => blake3(input, { dkLen: DIGEST_BYTES }),
});

/**
 * Which accepted form an address takes, or `null` when it is not one this verifier checks:
 * `bytes` (raw + BLAKE3-256), `raw` (raw + sha2-256, citations), `dag-cbor` (records).
 */
export type AddressForm = "bytes" | "raw" | "dag-cbor";

interface Parsed {
  form: AddressForm;
  digest: Uint8Array;
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a[i] ^ b[i];
  return diff === 0;
}

/**
 * The CID's form, when its binary form is the canonical encoding of one of the three triples.
 * multiformats keeps the bytes it decoded (its varint reader admits `81 00` for 1), so the CID is
 * rebuilt from its decoded parts and the bytes compared.
 */
function fromCid(cid: CID): Parsed | null {
  if (cid.version !== 1) return null;
  const { code: mh, size, digest } = cid.multihash;
  if (size !== DIGEST_BYTES || digest.length !== DIGEST_BYTES) return null;
  let form: AddressForm;
  if (cid.code === RAW_CODEC && mh === BLAKE3_256_CODE) form = "bytes";
  else if (cid.code === RAW_CODEC && mh === SHA2_256_CODE) form = "raw";
  else if (cid.code === DAG_CBOR_CODEC && mh === SHA2_256_CODE)
    form = "dag-cbor";
  else return null;
  const canonical = CID.createV1(cid.code, createDigest(mh, digest));
  if (!sameBytes(canonical.bytes, cid.bytes)) return null;
  return { form, digest };
}

function parse(address: CID | string): Parsed | null {
  if (typeof address !== "string") {
    // A CID instance from any copy of multiformats, read through `CID.asCID`, which accepts
    // structurally compatible objects and returns null for anything else. `asCID` rebuilds such an
    // object from its own `multihash` without checking it against its `bytes`, and `toString()`
    // (what a cache keys by and a log prints) derives from `bytes`. So the address is taken from
    // the bytes alone: an object whose `multihash` disagrees with its `bytes` fails closed, and
    // the digest checked is always the one the CID's text names.
    const cid = CID.asCID(address);
    if (cid === null) return null;
    let decoded: CID;
    try {
      decoded = CID.decode(cid.bytes);
    } catch {
      return null;
    }
    return decoded.equals(cid) ? fromCid(decoded) : null;
  }
  let cid: CID;
  try {
    cid = CID.parse(address);
  } catch {
    return null;
  }
  const parsed = fromCid(cid);
  if (parsed === null) return null;
  // Canonical text equality: the one accepted spelling is the base32-lower text of the canonical
  // binary form. `cid.toString()` is not used — multiformats caches the parsed string as the
  // CID's text, so comparing against it would be vacuous.
  return base32.encode(cid.bytes) === address ? parsed : null;
}

/** The accepted form of `address`, or `null` when `verifyAddress` would fail it closed. */
export function addressForm(address: CID | string): AddressForm | null {
  return parse(address)?.form ?? null;
}

function viewOf(bytes: Uint8Array | ArrayBuffer): Uint8Array {
  return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
}

/**
 * True iff `bytes` hash, under the hash `address` itself declares, to the digest it commits to.
 * Never throws: an address this verifier does not accept answers `false`.
 */
export async function verifyAddress(
  address: CID | string,
  bytes: Uint8Array | ArrayBuffer,
): Promise<boolean> {
  const parsed = parse(address);
  if (parsed === null) return false;
  const input = viewOf(bytes);
  const hashed =
    parsed.form === "bytes"
      ? await blake3Hasher.digest(input)
      : await sha256.digest(input);
  return sameBytes(hashed.digest, parsed.digest);
}
