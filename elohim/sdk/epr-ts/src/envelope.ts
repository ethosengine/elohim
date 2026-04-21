import { CID } from 'multiformats/cid';
import { encodeCanonical } from './cbor';
import type { Envelope } from './generated/Envelope';
import type { Coupling } from './generated/Coupling';

/**
 * Parse a CID that may arrive as:
 *   - a string (multibase-encoded, e.g. "bafyrei...")
 *   - a number array (serde JSON byte serialization of Cid with serde-codec)
 *   - a Uint8Array (raw bytes)
 *
 * The Rust cid crate with `serde-codec` feature serializes CID to JSON as a
 * byte array (the raw CID multihash bytes), NOT as a multibase string.
 */
function parseCid(raw: unknown): CID {
  if (typeof raw === 'string') {
    return CID.parse(raw);
  }
  if (raw instanceof Uint8Array) {
    return CID.decode(raw);
  }
  if (Array.isArray(raw)) {
    return CID.decode(new Uint8Array(raw));
  }
  throw new Error(`Cannot parse CID from ${JSON.stringify(raw)}`);
}

/**
 * Compute the canonical bytes that get hashed for CID and signed for proof.
 * Mirrors Rust Envelope::canonical_bytes: excludes cid, proof, supersededBy.
 * Alphabetical map keys: claims, coupling, issuedAt, kind, payload, reach,
 * schemaKey, schemaRef, [supersedes].
 */
export async function canonicalEnvelopeBytes(env: Envelope, payload: Uint8Array): Promise<Uint8Array> {
  // The Envelope type says CIDs are strings, but when deserialized from the
  // Rust-generated JSON vectors, they arrive as number arrays (serde-codec byte form).
  // parseCid handles both transparently.
  const claimsAsCids = (env.claims as unknown as unknown[]).map((c) => parseCid(c));

  const map: Record<string, unknown> = {
    claims: claimsAsCids,
    coupling: couplingToMap(env.coupling),
    issuedAt: env.issuedAt,
    kind: env.kind,
    payload,
    reach: env.reach,
    schemaKey: env.schemaKey,
    schemaRef: parseCid(env.schemaRef as unknown),
  };
  if (env.supersedes !== null && env.supersedes !== undefined) {
    map.supersedes = parseCid(env.supersedes as unknown);
  }
  return encodeCanonical(map);
}

function couplingToMap(c: Coupling): Record<string, unknown> {
  const m: Record<string, unknown> = {};
  // coupling fields may also arrive as number arrays from the Rust JSON vectors
  if (c.knowledge !== null && c.knowledge !== undefined) {
    m.knowledge = parseCid(c.knowledge as unknown);
  }
  if (c.value !== null && c.value !== undefined) {
    m.value = parseCid(c.value as unknown);
  }
  if (c.governance !== null && c.governance !== undefined) {
    m.governance = parseCid(c.governance as unknown);
  }
  return m;
}
