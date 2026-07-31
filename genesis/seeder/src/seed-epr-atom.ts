/**
 * Seed the `elohim-host-landing` EPR atom GRAPH into elohim-storage.
 *
 * The EPR atoms are the pre-requisite for:
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
 * ## What changed (2026-07-31): the backing claims became REAL
 *
 * The landing atom used to ship `claims: []` and a governance coupling leg
 * whose CID was `sha256("elohim-host-governance-v1")` — a hash of a literal
 * string, self-described as "informational only". Nothing was on the other end
 * of that leg. The landing surface had a fully instrumented p2p/resiliency
 * plane and a governance leg pointing into the void.
 *
 * Now the seeder mints a small ACYCLIC GRAPH, in dependency order, and every
 * CID it references resolves to something that actually exists:
 *
 *   elohim-host-landing-schema        Manifest  ─┐ schemaRef of everything below
 *   elohim-host-landing-governance    Manifest  ─┼─ landing.coupling.governance
 *   elohim-host-landing-stewardship   Claim     ─┼─ landing.claims[0]
 *   elohim-host-landing-custody       Claim     ─┼─ landing.claims[1]
 *   elohim-host-landing               Manifest  ─┘
 *
 * ## Where the graph terminates (the three non-atom anchors)
 *
 * EPR coupling requirements (elohim/epr/src/kind.rs) make a self-contained
 * atom graph IMPOSSIBLE to root: `Manifest` requires a governance leg,
 * `Claim` requires a knowledge leg, and the only leg-free kinds
 * (AttentionTending, WitnessedInteraction) are semantically wrong here. Some
 * leg must therefore point at a CID that is not one of these atoms. Coupling
 * legs are opaque CIDs, so that is legal — but each terminal is a REAL,
 * content-derived address of bytes that exist, never a hash of a made-up
 * sentinel string:
 *
 *   MANIFESTO_ANCHOR  = deriveContentAnchor(content/manifesto.json)
 *       This is byte-for-byte the `dht_anchor_hash` column seed-sqlite writes
 *       on the seeded `manifesto` content row. The manifesto is the corpus's
 *       constitutional document (its governance state file records
 *       status = "constitutional"), so it is the honest governance terminal.
 *
 *   LANDING_ANCHOR    = deriveContentAnchor(content/elohim-host-landing.json)
 *       The `dht_anchor_hash` of the seeded landing content row — the
 *       knowledge terminal, i.e. "the thing these claims are about".
 *
 *   LAMAD_MANIFEST_CID = CIDv1(raw, sha2-256) over the bytes of
 *       elohim/sdk/domains/lamad/manifest.json — the repo's authoritative
 *       content-vocabulary schema. It is the schemaRef of the schema atom
 *       only; every other atom's schemaRef is the schema atom's own CID.
 *
 * ## Canonical bytes / CID / signing
 *
 * The EPR wire format is content-addressed: the CID is CIDv1(dag-cbor,
 * sha2-256) over the canonical bytes, which are a dag-cbor encoding of the map
 * {claims, coupling, issuedAt, kind, payload, reach, schemaKey, schemaRef,
 * [supersedes]} — mirroring Rust `Envelope::canonical_bytes`. Storage RECOMPUTES
 * this and rejects a mismatch (`epr_service::ingest` stage 1), so the encoding
 * has to agree byte-for-byte with the Rust side.
 *
 * Map key order is NOT insertion order on either side and does not need to be
 * managed here: `@ipld/dag-cbor` (cborg) and `serde_ipld_dagcbor` both emit
 * RFC-8949 §4.2.1 deterministic maps — length-first, then bytewise — and
 * `serde_ipld_dagcbor` explicitly re-sorts serialized entries
 * (ser.rs `self.entries.sort_unstable()`) regardless of BTreeMap order.
 * Verified empirically: cborg emits kind < reach < claims < payload <
 * coupling < issuedAt < schemaKey < schemaRef, independent of insertion order.
 *
 * Absent coupling legs are OMITTED from the coupling map (Rust `coupling_ipld`
 * only inserts present legs) — a `null` would change the bytes.
 *
 * Atoms are signed with a fixed 32-byte ed25519 seed so the CIDs are
 * deterministic across re-runs.
 *
 * ## Idempotency
 *
 * Content-addressed PUT is naturally idempotent: the CID is derived from the
 * canonical bytes, and `LocalEprStore::put` short-circuits when the CID already
 * exists AND the inbound canonical bytes match the stored ones (it errors only
 * on a genuine collision — same CID, different bytes, which cannot happen from
 * a deterministic seed). Re-running this seeder therefore re-PUTs the same five
 * CIDs and creates no duplicates. Changing a payload (e.g. editing the
 * governance state file) mints a NEW CID rather than mutating the old atom —
 * the old atom is left in place, not superseded (a supersedence pass is out of
 * scope here, same posture as seed-commitments' custody drift guard).
 *
 * Phase 2a acceptance: the storage layer validates signature algorithm
 * ("ed25519") and byte length (64) only — no public-key verification.
 *
 * ## schema_key slug resolution
 *
 * The Rust handler resolves "elohim-host-landing" via:
 *   1. epr_atoms PK lookup (returns None — PK is the CIDv1)
 *   2. schema_key fallback (returns the atom with schema_key = 'elohim-host-landing')
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

import * as dagCbor from '@ipld/dag-cbor';
import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';
import * as raw from 'multiformats/codecs/raw';
import * as ed from '@noble/ed25519';
// Use only the async API (signAsync / getPublicKeyAsync) — no sha512 provider setup needed.

import { contentAnchorInput, deriveContentAnchor } from './seed-sqlite.js';
import {
  buildContentIndex,
  buildPresenceIdByHumanId,
  loadAllPresences,
  resolveStewardship,
  type StewardshipProvenance,
} from './seed-stewardship.js';
import { LANDING_SELF_CUSTODY_PLEDGE } from './seed-commitments.js';

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const GENESIS_DIR = path.resolve(__dirname, '../..');
const REPO_ROOT = path.resolve(GENESIS_DIR, '..');
const CONTENT_DIR = path.join(GENESIS_DIR, 'data', 'lamad', 'content');
const GOVERNANCE_DIR = path.join(GENESIS_DIR, 'data', 'lamad', 'governance');
const LAMAD_MANIFEST_PATH = path.join(REPO_ROOT, 'elohim', 'sdk', 'domains', 'lamad', 'manifest.json');

const LANDING_CONTENT_ID = 'elohim-host-landing';
const CONSTITUTIONAL_CONTENT_ID = 'manifesto';
const GOVERNANCE_STATE_FILE = `state-content-${LANDING_CONTENT_ID}.json`;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** CIDv1 codec byte for dag-cbor (IPLD multicodec table 0x71). */
const DAG_CBOR_CODEC = 0x71;

/**
 * Fixed 32-byte seed for the seeder's signing keypair.
 * Must never change — changing it changes nothing about the CIDs (the proof is
 * NOT in the canonical bytes) but does change signer_cid on every atom.
 * This is a well-known seeder key, NOT an agent identity.
 */
const SEEDER_ED25519_SEED = new Uint8Array([
  0x65, 0x6c, 0x6f, 0x68, 0x69, 0x6d, 0x2d, 0x73, // "elohim-s"
  0x65, 0x65, 0x64, 0x65, 0x72, 0x2d, 0x6c, 0x61, // "eeder-la"
  0x6e, 0x64, 0x69, 0x6e, 0x67, 0x2d, 0x61, 0x74, // "nding-at"
  0x6f, 0x6d, 0x2d, 0x30, 0x30, 0x31, 0x00, 0x00, // "om-001\0\0"
]);

/** Schema key — the human-readable slug the nav-context handler resolves. */
export const SCHEMA_KEY = LANDING_CONTENT_ID;
export const SCHEMA_ATOM_KEY = `${LANDING_CONTENT_ID}-schema`;
export const GOVERNANCE_ATOM_KEY = `${LANDING_CONTENT_ID}-governance`;
export const STEWARDSHIP_CLAIM_KEY = `${LANDING_CONTENT_ID}-stewardship-claim`;
export const CUSTODY_CLAIM_KEY = `${LANDING_CONTENT_ID}-custody-claim`;

/** Reach: "commons" — publicly visible. */
const REACH = 'commons';

/** Fixed issued_at: reproducible across re-runs (any re-PUT of same bytes = idempotent). */
const ISSUED_AT = '2026-01-01T00:00:00Z';

type EprKindName = 'Manifest' | 'Claim';

// ---------------------------------------------------------------------------
// CID computation
// ---------------------------------------------------------------------------

async function dagCborCid(bytes: Uint8Array): Promise<CID> {
  const hash = await sha256.digest(bytes);
  return CID.create(1, DAG_CBOR_CODEC, hash);
}

/** CIDv1(raw, sha2-256) — the same addressing `deriveContentAnchor` uses. */
async function rawCid(bytes: Uint8Array): Promise<CID> {
  const hash = await sha256.digest(bytes);
  return CID.create(1, raw.code, hash);
}

// ---------------------------------------------------------------------------
// Canonical bytes — mirrors Rust Envelope::canonical_bytes exactly
// ---------------------------------------------------------------------------

export interface AtomCoupling {
  knowledge?: CID;
  value?: CID;
  governance?: CID;
}

export interface AtomSpec {
  schemaKey: string;
  kind: EprKindName;
  schemaRef: CID;
  coupling: AtomCoupling;
  claims: CID[];
  payload: Uint8Array;
}

/**
 * Only PRESENT legs are encoded. Rust's `coupling_ipld` builds a BTreeMap and
 * inserts a key only when the leg is `Some`, so an explicit `null` here would
 * produce different canonical bytes and a CID the storage layer rejects.
 */
function couplingMap(coupling: AtomCoupling): Record<string, CID> {
  const map: Record<string, CID> = {};
  if (coupling.governance) map.governance = coupling.governance;
  if (coupling.knowledge) map.knowledge = coupling.knowledge;
  if (coupling.value) map.value = coupling.value;
  return map;
}

/** Every field Rust's `Envelope::canonical_bytes` folds into the signed bytes. */
export interface CanonicalEnvelopeFields {
  claims: CID[];
  coupling: AtomCoupling;
  issuedAt: string;
  kind: string;
  payload: Uint8Array;
  reach: string;
  schemaKey: string;
  schemaRef: CID;
  supersedes?: CID;
}

/**
 * The byte-parity surface with Rust. Kept generic (rather than folding the
 * seeder's fixed reach/issuedAt in) so the unit test can drive it with the
 * Rust-generated vectors at elohim/epr/tests/vectors/signed_eprs.json and prove
 * the encodings agree — the storage layer recomputes these bytes and rejects a
 * CID mismatch, so parity is not optional.
 */
export function encodeCanonicalEnvelope(fields: CanonicalEnvelopeFields): Uint8Array {
  const map: Record<string, unknown> = {
    claims: fields.claims,
    coupling: couplingMap(fields.coupling),
    issuedAt: fields.issuedAt,
    kind: fields.kind,
    payload: fields.payload,
    reach: fields.reach,
    schemaKey: fields.schemaKey,
    schemaRef: fields.schemaRef,
  };
  // Rust inserts `supersedes` only when Some — an explicit null would change
  // the bytes.
  if (fields.supersedes) map.supersedes = fields.supersedes;
  return dagCbor.encode(map);
}

export function buildCanonicalBytes(spec: AtomSpec): Uint8Array {
  return encodeCanonicalEnvelope({
    claims: spec.claims,
    coupling: spec.coupling,
    issuedAt: ISSUED_AT,
    kind: spec.kind,
    payload: spec.payload,
    reach: REACH,
    schemaKey: spec.schemaKey,
    schemaRef: spec.schemaRef,
  });
}

// ---------------------------------------------------------------------------
// Atom finalization (CID + signature + wire body)
// ---------------------------------------------------------------------------

export interface FinalizedAtom {
  schemaKey: string;
  kind: EprKindName;
  cid: CID;
  /** The `EprPublishInput` wire shape (elohim_views::epr::EprPublishInput). */
  body: Record<string, unknown>;
}

async function finalize(spec: AtomSpec): Promise<FinalizedAtom> {
  const canonicalBytes = buildCanonicalBytes(spec);
  const cid = await dagCborCid(canonicalBytes);

  const publicKeyBytes = await ed.getPublicKeyAsync(SEEDER_ED25519_SEED);
  const signerCid = await dagCborCid(publicKeyBytes);
  const signatureBytes = await ed.signAsync(canonicalBytes, SEEDER_ED25519_SEED);

  return {
    schemaKey: spec.schemaKey,
    kind: spec.kind,
    cid,
    body: {
      envelope: {
        cid: cid.toString(),
        kind: spec.kind,
        schemaRef: spec.schemaRef.toString(),
        schemaKey: spec.schemaKey,
        reach: REACH,
        coupling: {
          knowledge: spec.coupling.knowledge?.toString() ?? null,
          value: spec.coupling.value?.toString() ?? null,
          governance: spec.coupling.governance?.toString() ?? null,
        },
        claims: spec.claims.map((c) => c.toString()),
        supersedes: null,
        supersededBy: null,
        issuedAt: ISSUED_AT,
        proof: {
          signer: signerCid.toString(),
          algorithm: 'ed25519',
          signature: Buffer.from(signatureBytes).toString('hex'),
        },
      },
      // Hex-encoded raw payload bytes
      payload: Buffer.from(spec.payload).toString('hex'),
      event: null,
    },
  };
}

// ---------------------------------------------------------------------------
// Real backing facts, read off the same files the rest of the seed path reads
// ---------------------------------------------------------------------------

function readJson<T>(file: string): T {
  return JSON.parse(fs.readFileSync(file, 'utf-8')) as T;
}

/** The custody-blob pledge that seed-commitments seeds for the landing surface. */
export function landingCustodyFacts(): {
  action: string;
  contentId: string;
  artifactRole: string;
  providerHumanId: string;
  receiverHumanId: string;
  selfCustody: boolean;
} {
  const pair = LANDING_SELF_CUSTODY_PLEDGE;
  return {
    action: 'custody-blob',
    contentId: pair.contentId ?? LANDING_CONTENT_ID,
    artifactRole: pair.artifactRole ?? 'content',
    providerHumanId: pair.providerHumanId,
    receiverHumanId: pair.receiverHumanId,
    selfCustody: pair.providerHumanId === pair.receiverHumanId,
  };
}

/** The stewardship the stewardship seeder will allocate for the landing surface. */
export function landingStewardshipFacts(): {
  contentId: string;
  provenance: StewardshipProvenance;
  stewards: Array<{ presenceId: string; ratio: number; contributionType: string }>;
} {
  const presences = loadAllPresences();
  const index = buildContentIndex(CONTENT_DIR);
  const facts = index.get(LANDING_CONTENT_ID);
  const { stewards, provenance } = resolveStewardship(
    facts?.category,
    facts?.stewardedBy,
    buildPresenceIdByHumanId(presences)
  );
  return {
    contentId: LANDING_CONTENT_ID,
    provenance,
    stewards: stewards.map((s) => ({
      presenceId: s.presenceId,
      ratio: s.ratio,
      contributionType: s.contributionType ?? 'curator',
    })),
  };
}

// ---------------------------------------------------------------------------
// Graph construction
// ---------------------------------------------------------------------------

export interface LandingAtomGraph {
  /** Non-atom terminals, surfaced so callers can log/verify what the legs hit. */
  anchors: {
    manifestoAnchor: string;
    landingAnchor: string;
    lamadManifestCid: string;
  };
  /** In dependency order — PUT them in this order. */
  atoms: FinalizedAtom[];
  /** The atom the EprRouter / nav-context resolve by slug. */
  landing: FinalizedAtom;
}

const utf8 = (s: string) => new TextEncoder().encode(s);

/**
 * Build the whole landing atom graph without touching the network.
 * Deterministic: same repo bytes in, same CIDs out.
 */
export async function buildLandingAtomGraph(): Promise<LandingAtomGraph> {
  // --- Terminals ------------------------------------------------------------
  const landingJson = readJson<Parameters<typeof contentAnchorInput>[0]>(
    path.join(CONTENT_DIR, `${LANDING_CONTENT_ID}.json`)
  );
  const manifestoJson = readJson<Parameters<typeof contentAnchorInput>[0]>(
    path.join(CONTENT_DIR, `${CONSTITUTIONAL_CONTENT_ID}.json`)
  );

  const landingAnchor = CID.parse(await deriveContentAnchor(contentAnchorInput(landingJson)));
  const manifestoAnchor = CID.parse(await deriveContentAnchor(contentAnchorInput(manifestoJson)));
  const lamadManifestCid = await rawCid(fs.readFileSync(LAMAD_MANIFEST_PATH));

  // --- 1. Schema manifest ---------------------------------------------------
  // The schemaRef every other atom below points at. Its own schemaRef is the
  // lamad manifest file's CID — the bootstrap terminal (see file header).
  const schemaAtom = await finalize({
    schemaKey: SCHEMA_ATOM_KEY,
    kind: 'Manifest',
    schemaRef: lamadManifestCid,
    coupling: { governance: manifestoAnchor },
    claims: [],
    payload: utf8(
      JSON.stringify({
        title: 'Elohim Host Landing — atom family schema',
        description:
          'Declares the payload shapes carried by the elohim-host-landing atom family. ' +
          'Payload schema validation is deferred (EPR stage 4 needs the manifest resolver), ' +
          'so this atom is the resolvable schemaRef the family points at rather than an ' +
          'enforced validator.',
        schemaKey: SCHEMA_ATOM_KEY,
        schemaSource: 'elohim/sdk/domains/lamad/manifest.json',
        members: {
          [SCHEMA_KEY]: 'Manifest — the landing surface projection',
          [GOVERNANCE_ATOM_KEY]: 'Manifest — verbatim governance state document',
          [STEWARDSHIP_CLAIM_KEY]: 'Claim — who stewards the landing content, and on what provenance',
          [CUSTODY_CLAIM_KEY]: 'Claim — the custody-blob pledge held for the landing artifact',
        },
      })
    ),
  });

  // --- 2. Governance state --------------------------------------------------
  // Payload is the governance state document VERBATIM: the atom IS the file,
  // content-addressed. Editing the file mints a new CID, which is the point.
  const governanceStateBytes = fs.readFileSync(path.join(GOVERNANCE_DIR, GOVERNANCE_STATE_FILE));
  const governanceAtom = await finalize({
    schemaKey: GOVERNANCE_ATOM_KEY,
    kind: 'Manifest',
    schemaRef: schemaAtom.cid,
    coupling: {
      // Governed under the constitutional corpus…
      governance: manifestoAnchor,
      // …about the landing content.
      knowledge: landingAnchor,
    },
    claims: [],
    payload: new Uint8Array(governanceStateBytes),
  });

  // --- 3. Stewardship claim -------------------------------------------------
  const stewardship = landingStewardshipFacts();
  const stewardshipClaim = await finalize({
    schemaKey: STEWARDSHIP_CLAIM_KEY,
    kind: 'Claim',
    schemaRef: schemaAtom.cid,
    coupling: { knowledge: landingAnchor },
    claims: [],
    payload: utf8(
      JSON.stringify({
        claim: 'stewardship',
        ...stewardship,
        source: 'genesis/seeder/src/seed-stewardship.ts (resolveStewardship)',
        note:
          'The stewards and ratios this claim asserts are computed by the same resolver ' +
          'the stewardship seeder uses to write stewardship_allocations rows, from the ' +
          "content's own stewardedBy declaration.",
      })
    ),
  });

  // --- 4. Custody claim -----------------------------------------------------
  const custody = landingCustodyFacts();
  const custodyClaim = await finalize({
    schemaKey: CUSTODY_CLAIM_KEY,
    kind: 'Claim',
    schemaRef: schemaAtom.cid,
    coupling: { knowledge: landingAnchor },
    claims: [],
    payload: utf8(
      JSON.stringify({
        claim: 'custody',
        ...custody,
        source: 'genesis/seeder/src/seed-commitments.ts (LANDING_SELF_CUSTODY_PLEDGE)',
        note:
          'Asserts that a custody-blob REA commitment is pledged for this content and ' +
          'artifact role. The resolved blob hash is deliberately NOT part of this claim: ' +
          'it is minted at deploy time (serverBlobHash), so binding it here would make the ' +
          'claim CID move on every SSR bundle rebuild. The commitment row carries the hash.',
      })
    ),
  });

  // --- 5. The landing atom --------------------------------------------------
  const landing = await finalize({
    schemaKey: SCHEMA_KEY,
    kind: 'Manifest',
    schemaRef: schemaAtom.cid,
    coupling: {
      // A real atom, minted above — not a hash of a literal string.
      governance: governanceAtom.cid,
      // Cheap, real knowledge leg: the seeded content row's own anchor.
      knowledge: landingAnchor,
      // No value leg: `Manifest` does not require one and there is no real
      // value-side CID to point at (the custody commitment is an REA row, not
      // an EPR atom). Absent beats fabricated.
    },
    claims: [stewardshipClaim.cid, custodyClaim.cid],
    payload: utf8(
      JSON.stringify({
        title: 'Elohim Host Landing',
        description: 'The hosted landing page for elohim.host / alpha.elohim.host.',
        schemaKey: SCHEMA_KEY,
        contentId: LANDING_CONTENT_ID,
      })
    ),
  });

  return {
    anchors: {
      manifestoAnchor: manifestoAnchor.toString(),
      landingAnchor: landingAnchor.toString(),
      lamadManifestCid: lamadManifestCid.toString(),
    },
    atoms: [schemaAtom, governanceAtom, stewardshipClaim, custodyClaim, landing],
    landing,
  };
}

// ---------------------------------------------------------------------------
// Seeding
// ---------------------------------------------------------------------------

export interface EprAtomSeedResult {
  cid: string;
  schemaKey: string;
  skipped: boolean;
  reason?: string;
}

async function putAtom(storageUrl: string, atom: FinalizedAtom): Promise<EprAtomSeedResult> {
  const cidStr = atom.cid.toString();
  const url = `${storageUrl.replace(/\/$/, '')}/api/v1/epr/${encodeURIComponent(cidStr)}`;
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  const apiKey = process.env.DOORWAY_API_KEY || process.env.STORAGE_API_KEY;
  if (apiKey) headers['X-API-Key'] = apiKey;

  const response = await fetch(url, {
    method: 'PUT',
    headers,
    body: JSON.stringify(atom.body),
  });

  if (response.ok) {
    return { cid: cidStr, schemaKey: atom.schemaKey, skipped: false };
  }

  const text = await response.text();

  // A CID collision — an existing row under this CID whose stored canonical
  // bytes DIFFER from ours (storage answers 400 "… already exists with
  // different canonical bytes — not idempotent") — is the storage layer's
  // guard against cross-language canonical-encoding drift. It must fail the
  // seed loudly, never read as "already seeded".
  if (text.includes('different canonical bytes')) {
    throw new Error(
      `seedLandingEprAtoms: PUT ${url} (${atom.schemaKey}) hit a canonical-bytes collision — ` +
        `encoder drift between this seeder and elohim-storage? HTTP ${response.status}: ${text.slice(0, 500)}`
    );
  }

  // Content-addressed idempotency: a re-PUT of identical canonical bytes is
  // accepted (200). A 409 / "already exists" is still a success for our
  // purposes — the atom is present with the bytes we intended.
  const isIdempotent = response.status === 409 || text.includes('already exists');

  if (isIdempotent) {
    return { cid: cidStr, schemaKey: atom.schemaKey, skipped: true, reason: text.slice(0, 120) };
  }

  throw new Error(
    `seedLandingEprAtoms: PUT ${url} (${atom.schemaKey}) failed HTTP ${response.status}: ${text.slice(0, 500)}`
  );
}

/**
 * Build and PUT the whole landing atom graph, in dependency order.
 *
 * Returns one result per atom. Ordering matters only for readability of the
 * output — the CIDs are computed locally, so a referenced atom's CID is known
 * before it is stored — but seeding in dependency order means a partial failure
 * never leaves a dangling reference from an atom that DID land.
 */
export async function seedLandingEprAtoms(storageUrl: string): Promise<EprAtomSeedResult[]> {
  const graph = await buildLandingAtomGraph();
  const results: EprAtomSeedResult[] = [];
  for (const atom of graph.atoms) {
    results.push(await putAtom(storageUrl, atom));
  }
  return results;
}

/**
 * Back-compatible single-atom entry point: seeds the graph and returns the
 * landing atom's result.
 */
export async function seedLandingEprAtom(storageUrl: string): Promise<EprAtomSeedResult> {
  const results = await seedLandingEprAtoms(storageUrl);
  const landing = results.find((r) => r.schemaKey === SCHEMA_KEY);
  if (!landing) throw new Error('seedLandingEprAtom: landing atom missing from seed results');
  return landing;
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  const dryRun = process.argv.includes('--dry-run');
  const storageUrl =
    process.env.STORAGE_URL || process.env.DOORWAY_URL || 'http://localhost:8090';

  console.log('='.repeat(60));
  console.log('EPR Atom Seeder — elohim-host-landing');
  console.log(dryRun ? '  (DRY RUN — no atoms will be written)' : `  Target: ${storageUrl}`);
  console.log('='.repeat(60));

  const graph = await buildLandingAtomGraph();

  console.log();
  console.log('Non-atom anchors (real content-derived addresses):');
  console.log(`   manifesto content anchor : ${graph.anchors.manifestoAnchor}`);
  console.log(`   landing content anchor   : ${graph.anchors.landingAnchor}`);
  console.log(`   lamad manifest bytes     : ${graph.anchors.lamadManifestCid}`);
  console.log();
  console.log('Atoms (dependency order):');
  for (const atom of graph.atoms) {
    console.log(`   ${atom.kind.padEnd(9)} ${atom.schemaKey.padEnd(38)} ${atom.cid.toString()}`);
  }
  console.log();
  console.log('Landing atom couplings + claims:');
  const env = graph.landing.body.envelope as {
    coupling: Record<string, string | null>;
    claims: string[];
  };
  console.log(`   governance -> ${env.coupling.governance}`);
  console.log(`   knowledge  -> ${env.coupling.knowledge}`);
  console.log(`   value      -> ${env.coupling.value ?? '(absent — not required for Manifest)'}`);
  for (const claim of env.claims) console.log(`   claim      -> ${claim}`);
  console.log();

  if (dryRun) {
    console.log('Dry run complete. Run without --dry-run to write atoms.');
    return;
  }

  let failed = 0;
  for (const atom of graph.atoms) {
    try {
      const result = await putAtom(storageUrl, atom);
      console.log(
        `   ${result.skipped ? 'exists ' : 'wrote  '} ${atom.schemaKey.padEnd(38)} ${result.cid}`
      );
    } catch (error) {
      failed += 1;
      console.error(`   FAILED  ${atom.schemaKey}: ${error}`);
    }
  }

  console.log();
  console.log('='.repeat(60));
  if (failed > 0) {
    console.error(`EPR atom seeding: ${failed} of ${graph.atoms.length} atoms failed`);
    process.exit(1);
  }
  console.log(`EPR atom seeding complete — ${graph.atoms.length} atoms present`);
}

// Standalone execution only — importing this module (unit tests, other seeders)
// must not hit the network. Mirrors seed-sqlite.ts / seed-commitments.ts.
const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  main().catch((error) => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}
