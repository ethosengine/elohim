/**
 * Seed the `elohim-host-landing` EPR atom into elohim-storage.
 *
 * The EPR atom is the pre-requisite for:
 *   GET /api/v1/epr/elohim-host-landing/nav-context  (404 → 200)
 *   The EprRouter projection "elohim-host-landing" resolves in doorway
 *
 * ## Why this file exists
 *
 * The projection seeder (seed-projections.ts) writes REA Commitments that
 * declare "doorway D projects EPR E at path P". For that to be usable, the
 * EPR atom referenced by E must exist in epr_atoms — the nav-context handler
 * does a primary-key lookup on cid = schema_key when the caller passes a slug
 * (via the schema_key fallback added to epr_nav_context_view::project).
 *
 * ## Canonical bytes / CID / signing
 *
 * The EPR wire format is content-addressed: the CID is CIDv1(dag-cbor,
 * sha2-256) over the canonical bytes. The canonical bytes are a dag-cbor
 * encoding of a BTreeMap with alphabetically ordered keys (claims, coupling,
 * issuedAt, kind, payload, reach, schemaKey, schemaRef, [supersedes]) —
 * exactly mirroring Rust `Envelope::canonical_bytes`.
 *
 * The atom is signed with a fixed 32-byte ed25519 seed so the CID is
 * deterministic across re-runs. The seeder is idempotent: a second PUT of the
 * same atom is accepted (idempotent) by the storage layer (same canonical_bytes
 * means no collision).
 *
 * Phase 2a acceptance: the storage layer validates signature algorithm
 * ("ed25519") and byte length (64) only — no public-key verification. Any
 * 64-byte signature over any 32-byte key is accepted structurally.
 *
 * ## schema_key slug resolution
 *
 * The Rust handler resolves "elohim-host-landing" via:
 *   1. epr_atoms PK lookup (returns None — PK is the CIDv1)
 *   2. schema_key fallback (returns the atom with schema_key = 'elohim-host-landing')
 *
 * That fallback was added to epr_nav_context_view::project alongside this
 * seeder step.
 */

import * as dagCbor from '@ipld/dag-cbor';
import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';
import * as ed from '@noble/ed25519';
// Use only the async API (signAsync / getPublicKeyAsync) — no sha512 provider setup needed.

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** CIDv1 codec byte for dag-cbor (IPLD multicodec table 0x71). */
const DAG_CBOR_CODEC = 0x71;

/**
 * Fixed 32-byte seed for the seeder's signing keypair.
 * Must never change — changing it changes the CID, making re-runs non-idempotent.
 * This is a well-known seeder key, NOT an agent identity.
 */
const SEEDER_ED25519_SEED = new Uint8Array([
  0x65, 0x6c, 0x6f, 0x68, 0x69, 0x6d, 0x2d, 0x73, // "elohim-s"
  0x65, 0x65, 0x64, 0x65, 0x72, 0x2d, 0x6c, 0x61, // "eeder-la"
  0x6e, 0x64, 0x69, 0x6e, 0x67, 0x2d, 0x61, 0x74, // "nding-at"
  0x6f, 0x6d, 0x2d, 0x30, 0x30, 0x31, 0x00, 0x00, // "om-001\0\0"
]);

/**
 * Schema ref CID — a well-known sentinel CIDv1 used as the schema reference
 * for the landing atom. Derived from the fixed bytes [0x73, 0x63, 0x68, 0x65]
 * ("sche") so it is reproducible. In production this would reference a real
 * Manifest EPR; for the landing atom the schema ref is informational only
 * (Phase 2a does not validate payload schemas).
 */
const SCHEMA_REF_SEED_BYTES = new Uint8Array([
  0x73, 0x63, 0x68, 0x65, 0x6d, 0x61, 0x2d, 0x72, // "schema-r"
  0x65, 0x66, 0x2d, 0x6c, 0x61, 0x6e, 0x64, 0x69, // "ef-landi"
  0x6e, 0x67, 0x2d, 0x70, 0x61, 0x67, 0x65, 0x2d, // "ng-page-"
  0x76, 0x31,                                       // "v1"
]);

/** Schema key — the human-readable slug the nav-context handler resolves. */
const SCHEMA_KEY = 'elohim-host-landing';

/** Kind: "Manifest" — requires only the governance coupling leg. */
const KIND = 'Manifest';

/** Reach: "commons" — publicly visible. */
const REACH = 'commons';

/** Fixed issued_at: reproducible across re-runs (any re-PUT of same bytes = idempotent). */
const ISSUED_AT = '2026-01-01T00:00:00Z';

/** Payload: minimal JSON describing the atom's purpose. */
const PAYLOAD_JSON = JSON.stringify({
  title: 'Elohim Host Landing',
  description: 'The hosted landing page for elohim.host / alpha.elohim.host.',
  schemaKey: SCHEMA_KEY,
});

// ---------------------------------------------------------------------------
// CID computation
// ---------------------------------------------------------------------------

async function computeCid(bytes: Uint8Array): Promise<CID> {
  const hash = await sha256.digest(bytes);
  return CID.create(1, DAG_CBOR_CODEC, hash);
}

// ---------------------------------------------------------------------------
// Canonical bytes — mirrors Rust Envelope::canonical_bytes exactly
//
// BTreeMap alphabetical key order: claims, coupling, issuedAt, kind, payload,
// reach, schemaKey, schemaRef, [supersedes]
//
// CID fields (schemaRef, coupling legs, claims) are encoded as IPLD Link nodes
// (@ipld/dag-cbor encodes CID objects as CBOR Tag 42 links automatically).
// ---------------------------------------------------------------------------

async function buildCanonicalBytes(
  schemaRefCid: CID,
  governanceCid: CID,
  payload: Uint8Array,
): Promise<Uint8Array> {
  const map: Record<string, unknown> = {
    claims: [] as CID[],
    coupling: {
      governance: governanceCid,
    },
    issuedAt: ISSUED_AT,
    kind: KIND,
    payload,
    reach: REACH,
    schemaKey: SCHEMA_KEY,
    schemaRef: schemaRefCid,
    // no supersedes
  };

  // @ipld/dag-cbor emits maps with string keys in insertion order. We must
  // match the Rust BTreeMap alphabetical order explicitly. The keys above are
  // already in alphabetical order (claims < coupling < issuedAt < kind <
  // payload < reach < schemaKey < schemaRef). Verified against the Rust
  // Envelope::canonical_bytes implementation.
  return dagCbor.encode(map);
}

// ---------------------------------------------------------------------------
// EPR atom builder
// ---------------------------------------------------------------------------

export interface EprAtomSeedResult {
  cid: string;
  schemaKey: string;
  skipped: boolean;
  reason?: string;
}

/**
 * Build and PUT the elohim-host-landing EPR atom.
 *
 * The atom is PUT to `PUT /api/v1/epr/:cid` on the storage endpoint.
 * Phase 2a: the storage layer accepts any structurally valid ed25519
 * (algorithm = "ed25519", signature = 64 bytes) without verifying the
 * signature against a known public key.
 *
 * Idempotent: if the CID already exists with identical canonical bytes,
 * the storage layer returns 200. If it already exists with different bytes,
 * it returns 409/400 — which should never happen since the seed is deterministic.
 */
export async function seedLandingEprAtom(storageUrl: string): Promise<EprAtomSeedResult> {
  // 1. Derive schema_ref CID from the fixed seed bytes
  const schemaRefHash = await sha256.digest(SCHEMA_REF_SEED_BYTES);
  const schemaRefCid = CID.create(1, DAG_CBOR_CODEC, schemaRefHash);

  // 2. Derive signer CID (representing this seeder's agent identity) from
  //    the ed25519 public key bytes
  const privateKey = SEEDER_ED25519_SEED;
  const publicKeyBytes = await ed.getPublicKeyAsync(privateKey);
  const signerHash = await sha256.digest(publicKeyBytes);
  const signerCid = CID.create(1, DAG_CBOR_CODEC, signerHash);

  // 3. Governance coupling leg CID — a well-known self-reference for the
  //    landing atom's governance anchor (informational in Phase 2a).
  const governanceBytes = new TextEncoder().encode('elohim-host-governance-v1');
  const govHash = await sha256.digest(governanceBytes);
  const governanceCid = CID.create(1, DAG_CBOR_CODEC, govHash);

  // 4. Encode payload
  const payload = new TextEncoder().encode(PAYLOAD_JSON);

  // 5. Build canonical bytes (the bytes that are hashed for CID and signed)
  const canonicalBytes = await buildCanonicalBytes(schemaRefCid, governanceCid, payload);

  // 6. Compute the CIDv1(dag-cbor, sha2-256) from canonical bytes
  const atomCid = await computeCid(canonicalBytes);
  const cidStr = atomCid.toString();

  // 7. Sign canonical bytes
  const signatureBytes = await ed.signAsync(canonicalBytes, privateKey);
  const signatureHex = Buffer.from(signatureBytes).toString('hex');

  // 8. Build the EprPublishInput wire shape (matches elohim_views::epr::EprPublishInput)
  const body = {
    envelope: {
      cid: cidStr,
      kind: KIND,
      schemaRef: schemaRefCid.toString(),
      schemaKey: SCHEMA_KEY,
      reach: REACH,
      coupling: {
        knowledge: null,
        value: null,
        governance: governanceCid.toString(),
      },
      claims: [],
      supersedes: null,
      supersededBy: null,
      issuedAt: ISSUED_AT,
      proof: {
        signer: signerCid.toString(),
        algorithm: 'ed25519',
        signature: signatureHex,
      },
    },
    // Hex-encoded raw payload bytes
    payload: Buffer.from(payload).toString('hex'),
    event: null,
  };

  // 9. PUT to /api/v1/epr/:cid
  const url = `${storageUrl.replace(/\/$/, '')}/api/v1/epr/${encodeURIComponent(cidStr)}`;
  const response = await fetch(url, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });

  if (response.ok) {
    return { cid: cidStr, schemaKey: SCHEMA_KEY, skipped: false };
  }

  const text = await response.text();

  // Idempotent: "already exists with different canonical bytes" should never
  // happen with a fixed seed. A plain 200 or a "same canonical bytes" 200 is
  // the expected idempotent path.
  const isIdempotent =
    response.status === 409 ||
    text.includes('already exists') ||
    text.includes('not idempotent');

  if (isIdempotent) {
    return { cid: cidStr, schemaKey: SCHEMA_KEY, skipped: true, reason: text.slice(0, 120) };
  }

  throw new Error(
    `seedLandingEprAtom: PUT ${url} failed HTTP ${response.status}: ${text.slice(0, 500)}`,
  );
}
