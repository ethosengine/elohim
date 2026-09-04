/**
 * Mishpat `migrates-lineage` / `sunsets-lineage` / `revokes-commitment`
 * payload builders and the harness-side notarization calls that submit them
 * via `mishpat.create_commitment` (Holochain Evolution Epic, Task 10 Part 1
 * — the TypeScript half; the live mesh run is Task 11's).
 *
 * Payload contract (Task 2, `elohim/holochain/dna/mishpat/zomes/mishpat/src/
 * commitments.rs` `validate_migrates_lineage` / `validate_sunsets_lineage` /
 * `validate_lineage_signatures` / `validate_revokes_commitment`, read
 * 2026-09-04): `migrates-lineage` requires `action, role, from_dna_hash,
 * to_dna_hash, release_cid, constitution_root, roster_cid,
 * signing_payload_cid, signatures, evidence, window` (window:
 * `opens_at`/`revert_until`, both `Z`-suffixed RFC3339, opens < revert);
 * `sunsets-lineage` drops `release_cid`/`constitution_root`/`roster_cid` for
 * `migration_commitment_cid`, and its window carries `sunsets_at` instead;
 * `revokes-commitment` needs only `action`/`target_cid`/`signed_at`, PLUS
 * the full lineage-signature quorum when it names `target_action:
 * "migrates-lineage" | "sunsets-lineage"` (revoking a lineage commitment
 * takes the same signers a lineage commitment itself took — one signer's
 * say-so cannot pull a crossing back). Every signature verifies via
 * `verify_signature_raw(key, signature, signing_payload_cid.as_bytes())` —
 * the LITERAL UTF-8 bytes of that one string field, never a re-serialization
 * of the payload (the module comment there is explicit that
 * `verify_signature`, the msgpack-checking sibling, would never verify).
 *
 * ## Signing path — the gap this file documents rather than papers over
 *
 * The mishpat contract's own comment names the counterpart primitive:
 * "the counterpart to `keystore.sign(agent, bytes)`" — a raw-bytes signature
 * from the CONDUCTOR's lair keystore, under the agent's real on-DHT identity
 * key. Two places were checked for that primitive and neither has it:
 *
 *   1. `@holochain/client` (0.20.2, this repo's pinned version — checked
 *      `lib/api/zome-call-signing.d.ts` / `lib/hdk/capabilities.d.ts`
 *      2026-09-04): `generateSigningKeyPair` + `authorizeSigningCredentials`
 *      produce and authorize an EPHEMERAL keypair used only to sign the
 *      zome-CALL transport envelope (`CallZomeRequestSigned`). That key is
 *      not the agent's real `AgentPubKey` and its signature is not exposed
 *      to the caller as raw bytes over arbitrary content — it is consumed
 *      internally by `signZomeCall`. There is no public API in this client
 *      version for "sign these bytes with the agent's real key."
 *   2. The mishpat coordinator zome itself (grepped for `#[hdk_extern]` /
 *      `fn sign` across `zomes/mishpat/src/`, 2026-09-04): no extern signs
 *      raw bytes today. `hdk::prelude::sign` is available IN WASM (an
 *      extern could call it and return the `Signature`), but authoring one
 *      is a DNA-crate change — out of scope for this dispatch, which is
 *      explicitly TypeScript-only and frozen out of `elohim/holochain/**`.
 *
 * So `notarizeMigration` / `notarizeSunset` / `revokeMigration` below take
 * an ALREADY-SIGNED payload (its `signatures` array populated) and only
 * submit it via `create_commitment` — they do not sign anything themselves.
 * Producing that array is Task 11's problem to solve, with one of two
 * shapes once a mesh session un-freezes the DNA crate: (a) a new mishpat
 * coordinator extern (e.g. `sign_bytes`) that wraps `hdk::prelude::sign` and
 * returns `{ agent, signature }`, called through the same conductor rail as
 * `create_commitment`; or (b) some other keystore-backed signer this
 * dispatch did not have visibility into. This file's `ConductorRail` is
 * intentionally the same `{ call(fn_name, payload) }` shape
 * `scripts/release-ceremony.ts`'s local (unexported) `conductor()` rail
 * returns, so whichever shape Task 11 picks, submitting the finished
 * payload is a one-line `conductor.call('create_commitment', …)` — which is
 * exactly what these three functions already do.
 *
 * ## Content addressing — reimplemented, not imported
 *
 * `signingPayloadCid` uses the SAME CIDv1 raw/sha2-256/base32-lower encoding
 * `scripts/epr-release-package.ts`'s `blobCid` mints (that packager's own
 * doc: "the same `bafkrei…` form `elohim-storage/src/epr_codec.rs` mints").
 * The functions are reimplemented here rather than imported: reading that
 * script's own tail (2026-09-04) shows its whole CLI body runs at MODULE
 * EVALUATION time inside an unguarded top-level `try { … }` — no
 * `import.meta.url === …` guard — so `import { blobCid } from
 * '../../scripts/epr-release-package.js'` would execute `parseArgs
 * (process.argv.slice(2))` as an import side effect and, with no
 * `--artifact-class` on this test runner's argv, throw a `UsageError` that
 * sets `process.exitCode = 64` — silently failing the whole test process
 * regardless of what this file's own tests assert. `epr-release-package.spec
 * .ts` avoids the same trap by shelling the CLI out as a subprocess instead
 * of importing it; this file has no CLI to shell out to for a PURE payload
 * builder, so it duplicates the ~15-line encoding instead.
 *
 * The mishpat contract does not require `signing_payload_cid` to be any
 * PARTICULAR digest — `validate_lineage_signatures` reads it as an opaque
 * string and checks signatures against its literal bytes, never
 * recomputing it from the rest of the payload. Any stable content id of the
 * payload-without-`signing_payload_cid`-and-`signatures` satisfies the
 * contract; this file picks the packager's own scheme only for consistency
 * with the rest of the release-manifest tooling, not because the contract
 * demands it.
 */

import { createHash } from 'node:crypto';

// ---------------------------------------------------------------------------
// Content addressing (duplicated from epr-release-package.ts — see module doc)
// ---------------------------------------------------------------------------

const BASE32_ALPHABET = 'abcdefghijklmnopqrstuvwxyz234567';

function base32Encode(bytes: Uint8Array): string {
  let out = '';
  let buffer = 0;
  let bits = 0;
  for (const byte of bytes) {
    buffer = (buffer << 8) | byte;
    bits += 8;
    while (bits >= 5) {
      bits -= 5;
      out += BASE32_ALPHABET[(buffer >> bits) & 0x1f];
    }
  }
  if (bits > 0) out += BASE32_ALPHABET[(buffer << (5 - bits)) & 0x1f];
  return out;
}

/** CIDv1 raw (0x55) / sha2-256, multibase base32-lower ('b' prefix). */
function contentCid(bytes: Buffer): string {
  const digest = createHash('sha256').update(bytes).digest();
  const multihash = Buffer.concat([Buffer.from([0x12, 0x20]), digest]);
  const cidBytes = Buffer.concat([Buffer.from([0x01, 0x55]), multihash]);
  return `b${base32Encode(cidBytes)}`;
}

/** Deep-sorts object keys so field-insertion order never changes the digest. */
function canonicalize(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value !== null && typeof value === 'object') {
    const sorted: Record<string, unknown> = {};
    for (const key of Object.keys(value).sort((a, b) => a.localeCompare(b))) {
      sorted[key] = canonicalize((value as Record<string, unknown>)[key]);
    }
    return sorted;
  }
  return value;
}

/**
 * The content id `signatures` in a lineage commitment cover — computed over
 * every field of `payload` EXCEPT `signing_payload_cid` and `signatures`
 * themselves (stripped first, so calling this on either the unsigned draft
 * or the fully-signed payload yields the same value — see module doc).
 */
export function signingPayloadCid(payload: Record<string, unknown>): string {
  const rest = Object.fromEntries(
    Object.entries(payload).filter(([key]) => key !== 'signing_payload_cid' && key !== 'signatures')
  );
  const canonical = canonicalize(rest);
  return contentCid(Buffer.from(JSON.stringify(canonical), 'utf8'));
}

// ---------------------------------------------------------------------------
// Payload builders (pure)
// ---------------------------------------------------------------------------

export interface LineageSignature {
  /** base64 `AgentPubKey` of the signer. */
  agent: string;
  /** base64 of the raw 64-byte ed25519 signature over `signing_payload_cid`'s UTF-8 bytes. */
  signature: string;
}

export interface LineageEvidence {
  soak: unknown[];
  forecast: unknown;
  deliberation: unknown;
}

export interface LineageWindow {
  /** RFC3339 UTC, `Z`-suffixed. */
  opensAt: string;
  /** RFC3339 UTC, `Z`-suffixed; must be lexicographically AFTER `opensAt`. */
  revertUntil: string;
}

export interface MigratesLineageInput {
  role: string;
  fromDnaHash: string;
  toDnaHash: string;
  releaseCid: string;
  constitutionRoot: string;
  rosterCid: string;
  evidence: LineageEvidence;
  window: LineageWindow;
  /** k in the k-of-n quorum (spec default: 1). */
  requiredSignatures?: number;
  /** Populated once Task 11's signing step runs; defaults to none. */
  signatures?: LineageSignature[];
}

export interface MigratesLineagePayload {
  action: 'migrates-lineage';
  role: string;
  from_dna_hash: string;
  to_dna_hash: string;
  release_cid: string;
  constitution_root: string;
  roster_cid: string;
  signing_payload_cid: string;
  signatures: LineageSignature[];
  evidence: LineageEvidence;
  window: { opens_at: string; revert_until: string };
  required_signatures?: number;
}

/**
 * Builds a `migrates-lineage` commitment payload: every field
 * `validate_migrates_lineage` requires, `signing_payload_cid` computed over
 * everything else, and `signatures` defaulted to `[]` (the caller's own
 * signing step, or Task 11's, fills it in — see module doc).
 */
export function buildMigratesLineagePayload(input: MigratesLineageInput): MigratesLineagePayload {
  const unsigned = {
    action: 'migrates-lineage' as const,
    role: input.role,
    from_dna_hash: input.fromDnaHash,
    to_dna_hash: input.toDnaHash,
    release_cid: input.releaseCid,
    constitution_root: input.constitutionRoot,
    roster_cid: input.rosterCid,
    evidence: input.evidence,
    window: { opens_at: input.window.opensAt, revert_until: input.window.revertUntil },
    ...(input.requiredSignatures === undefined
      ? {}
      : { required_signatures: input.requiredSignatures }),
  };
  return {
    ...unsigned,
    signing_payload_cid: signingPayloadCid(unsigned),
    signatures: input.signatures ?? [],
  };
}

export interface SunsetsLineageInput {
  role: string;
  fromDnaHash: string;
  toDnaHash: string;
  migrationCommitmentCid: string;
  evidence: LineageEvidence;
  /** RFC3339 UTC, `Z`-suffixed. */
  sunsetsAt: string;
  requiredSignatures?: number;
  signatures?: LineageSignature[];
}

export interface SunsetsLineagePayload {
  action: 'sunsets-lineage';
  role: string;
  from_dna_hash: string;
  to_dna_hash: string;
  migration_commitment_cid: string;
  signing_payload_cid: string;
  signatures: LineageSignature[];
  evidence: LineageEvidence;
  window: { sunsets_at: string };
  required_signatures?: number;
}

/** Builds a `sunsets-lineage` commitment payload — closes a migration by naming the commitment it closes. */
export function buildSunsetsLineagePayload(input: SunsetsLineageInput): SunsetsLineagePayload {
  const unsigned = {
    action: 'sunsets-lineage' as const,
    role: input.role,
    from_dna_hash: input.fromDnaHash,
    to_dna_hash: input.toDnaHash,
    migration_commitment_cid: input.migrationCommitmentCid,
    evidence: input.evidence,
    window: { sunsets_at: input.sunsetsAt },
    ...(input.requiredSignatures === undefined
      ? {}
      : { required_signatures: input.requiredSignatures }),
  };
  return {
    ...unsigned,
    signing_payload_cid: signingPayloadCid(unsigned),
    signatures: input.signatures ?? [],
  };
}

// ---------------------------------------------------------------------------
// Station 10's real roster — an `author-lens` commitment, not a lineage one
// ---------------------------------------------------------------------------

export interface MintRosterInput {
  /** Agent strings (base64 `AgentPubKey`, `u`-prefixed) — the earned electorate. */
  members: string[];
  constitutionRoot: string;
  /** Only presence/non-emptiness is validated (`validate_author_lens`) — any
   * stable fixture-scoped slug satisfies the contract. */
  governsEpr?: string;
  school?: string;
}

export interface RosterMintPayload {
  action: 'author-lens';
  governs_epr: string;
  school: string;
  role: 'floor';
  rule: Record<string, never>;
  telos: Record<string, never>;
  members: string[];
  constitution_root: string;
}

const DEFAULT_ROSTER_GOVERNS_EPR = 'a2o-happ-lineage-migration-roster';
const DEFAULT_ROSTER_SCHOOL = 'a2o-fixture';

/**
 * Builds an `author-lens` payload, `role: "floor"`, whose `rule`/`telos` are
 * `{}` — `validate_author_lens` (`elohim/holochain/dna/mishpat/zomes/mishpat/
 * src/commitments.rs`) only checks that both fields ARE objects, never that
 * they hold anything in particular, so an empty bounds body satisfies the
 * arm. `members`/`constitution_root` are extra fields the validator never
 * reads — but `verify_path`'s own `read_roster`
 * (`elohim/elohim-storage/src/services/release_adoption/path_evidence.rs`)
 * reads them off ANY commitment body at the cid a path names as its
 * `roster_cid`, which is what makes this payload a real, gossip-checkable
 * roster rather than a slug nothing on the DHT ever resolves.
 */
export function buildRosterMintPayload(input: MintRosterInput): RosterMintPayload {
  return {
    action: 'author-lens',
    governs_epr: input.governsEpr ?? DEFAULT_ROSTER_GOVERNS_EPR,
    school: input.school ?? DEFAULT_ROSTER_SCHOOL,
    role: 'floor',
    rule: {},
    telos: {},
    members: input.members,
    constitution_root: input.constitutionRoot,
  };
}

export interface RevocationExtra {
  /** Present only when the target is itself a lineage commitment. */
  targetAction?: 'migrates-lineage' | 'sunsets-lineage';
  signedAt?: string;
  requiredSignatures?: number;
  signatures?: LineageSignature[];
}

export interface RevocationPayload {
  action: 'revokes-commitment';
  target_cid: string;
  signed_at: string;
  target_action?: 'migrates-lineage' | 'sunsets-lineage';
  signing_payload_cid?: string;
  signatures?: LineageSignature[];
  required_signatures?: number;
}

/**
 * Builds a `revokes-commitment` payload naming `targetCid`. When
 * `extra.targetAction` names a lineage action (`migrates-lineage` |
 * `sunsets-lineage`), `validate_revokes_commitment` additionally runs the
 * FULL `validate_lineage_signatures` quorum check on this payload — so this
 * builder adds `signing_payload_cid`/`signatures` in that case, exactly as
 * the two lineage builders above do. Revoking a plain (non-lineage)
 * commitment needs none of that; the base three fields
 * (`action`/`target_cid`/`signed_at`) are all `validate_revokes_commitment`
 * requires there.
 */
export function buildRevocationPayload(
  targetCid: string,
  extra: RevocationExtra = {}
): RevocationPayload {
  const base = {
    action: 'revokes-commitment' as const,
    target_cid: targetCid,
    signed_at: extra.signedAt ?? new Date().toISOString(),
    ...(extra.targetAction ? { target_action: extra.targetAction } : {}),
  };
  if (extra.targetAction) {
    return {
      ...base,
      signing_payload_cid: signingPayloadCid(base),
      signatures: extra.signatures ?? [],
      ...(extra.requiredSignatures === undefined
        ? {}
        : { required_signatures: extra.requiredSignatures }),
    };
  }
  return base;
}

// ---------------------------------------------------------------------------
// Live notarization (NOT exercised by this dispatch's tests — see module doc
// "Signing path" section for why; the mesh is another session's tonight)
// ---------------------------------------------------------------------------

/**
 * The `{ call(fn_name, payload) }` shape `scripts/release-ceremony.ts`'s
 * local `conductor()` rail returns after admin-connect ->
 * `authorizeSigningCredentials` -> app-connect (that file's module doc,
 * "The admin/app WS connect + cell resolution + zome-call-signing rail").
 * Not imported from there — that function is unexported (the file is a
 * standalone CLI with no exports) — so any caller building one copies its
 * SHAPE, the same way that file's own doc says every driver in this repo
 * copies it rather than re-deriving it.
 */
export interface ConductorRail {
  call: (fnName: string, payload: unknown) => Promise<unknown>;
}

export interface NotarizeResult {
  /** The commitment's `entry_hash`, base64 — the commitment cid (never `action_hash`; see `project_mishpat_commitment_cid_is_entry_hash`). */
  cid: string;
}

interface CommitmentOutputWire {
  entry_hash?: unknown;
  action_hash?: unknown;
}

/**
 * base64-encodes a Holochain hash. `@holochain/client`'s `encodeHashToBase64`
 * is deliberately NOT imported for this one call — pulling in the client
 * here only to encode 39 bytes would make this module depend on a live
 * websocket library it otherwise never touches, for a function this file's
 * own tests (which never reach a conductor) would never exercise anyway.
 * Holochain's `HoloHash` base64 form is `'u' + base64url(no padding)`.
 */
function encodeHoloHashBase64(hash: Uint8Array): string {
  return `u${Buffer.from(hash).toString('base64url')}`;
}

/**
 * The mishpat extern that SELF-SIGNS: `create_lineage_commitment` appends the
 * CALLING agent's own `{agent, signature}` over `payload_json
 * .signing_payload_cid` (via in-zome `sign_raw`, the literal-bytes counterpart
 * of the validator's `verify_signature_raw`) and then takes the ordinary
 * `create_commitment` path. Landed 2026-09-04 (`f508e75c9`) and hot-swapped
 * onto the household mesh — it closes exactly the gap this module's "Signing
 * path" section documents: `@holochain/client` cannot sign raw bytes with the
 * agent's real conductor-held key, so the harness never constructs a signature.
 */
const SELF_SIGNING_EXTERN = 'create_lineage_commitment';
const PLAIN_EXTERN = 'create_commitment';

async function createCommitment(
  conductor: ConductorRail,
  action: string,
  payload: object,
  signedAt: string,
  extern: string = PLAIN_EXTERN
): Promise<NotarizeResult> {
  const raw = await conductor.call(extern, {
    action,
    payload_json: JSON.stringify(payload),
    signed_at: signedAt,
  });
  const output = raw as CommitmentOutputWire;
  const entryHash = output.entry_hash;
  if (!(entryHash instanceof Uint8Array)) {
    throw new Error(
      `create_commitment(${action}) returned no entry_hash (Uint8Array) — got: ${JSON.stringify(raw)}`
    );
  }
  return { cid: encodeHoloHashBase64(entryHash) };
}

/**
 * Submits a `migrates-lineage` payload.
 *
 * `selfSign` (the default, and what Task 11's live stations use) routes through
 * [`SELF_SIGNING_EXTERN`], so the payload goes in with `signatures: []` and
 * comes back notarized under the CALLING agent's own key — the harness supplies
 * no key and no signature, ever. Pass `selfSign: false` to submit an
 * already-signed payload through the plain extern (the shape this module
 * originally had, kept because Station 10's negative branch needs a commitment
 * that the self-signing path would never produce).
 */
export async function notarizeMigration(opts: {
  conductor: ConductorRail;
  actingPeer: string;
  payload: MigratesLineagePayload;
  signedAt?: string;
  selfSign?: boolean;
}): Promise<NotarizeResult> {
  const extern = opts.selfSign === false ? PLAIN_EXTERN : SELF_SIGNING_EXTERN;
  console.error(
    `[lineage-commitments] ${opts.actingPeer} notarizing migrates-lineage via ${extern}`
  );
  return createCommitment(
    opts.conductor,
    'migrates-lineage',
    opts.payload,
    opts.signedAt ?? new Date().toISOString(),
    extern
  );
}

/**
 * Mints Station 10's real roster commitment (see `buildRosterMintPayload`'s
 * own doc for the arm and why `{}` bounds satisfy it) and returns its ENTRY
 * hash — `roster_cid` in a `migrates-lineage` payload names a commitment by
 * that same hash everywhere else on this substrate
 * (`project_mishpat_commitment_cid_is_entry_hash`), so this is the value a
 * caller drops straight into `MigratesLineageInput.rosterCid`.
 *
 * Submitted through the PLAIN `create_commitment` extern, NEVER the
 * self-signing `create_lineage_commitment` one `notarizeMigration` defaults
 * to: `create_lineage_commitment` requires `payload_json.signing_payload_cid`
 * to be a non-empty string UNCONDITIONALLY, before it ever reads `action`
 * (`commitments.rs`, `create_lineage_commitment`'s first checks) — an
 * `author-lens` body carries no such field and would be refused by that
 * check before `validate_author_lens` ever ran. `author-lens` needs no
 * signature at all, so the plain extern is also the CORRECT rail, not merely
 * the one that does not error.
 */
export async function mintRoster(opts: {
  conductor: ConductorRail;
  actingPeer: string;
  members: string[];
  constitutionRoot: string;
  governsEpr?: string;
  school?: string;
  signedAt?: string;
}): Promise<NotarizeResult> {
  const payload = buildRosterMintPayload({
    members: opts.members,
    constitutionRoot: opts.constitutionRoot,
    governsEpr: opts.governsEpr,
    school: opts.school,
  });
  console.error(
    `[lineage-commitments] ${opts.actingPeer} minting roster (author-lens, role: floor) — ` +
      `${opts.members.length} member(s) under root "${opts.constitutionRoot}"`
  );
  return createCommitment(
    opts.conductor,
    'author-lens',
    payload,
    opts.signedAt ?? new Date().toISOString(),
    PLAIN_EXTERN
  );
}

/** Submits an already-built, already-signed `sunsets-lineage` payload. */
export async function notarizeSunset(opts: {
  conductor: ConductorRail;
  actingPeer: string;
  payload: SunsetsLineagePayload;
  signedAt?: string;
  selfSign?: boolean;
}): Promise<NotarizeResult> {
  const extern = opts.selfSign === false ? PLAIN_EXTERN : SELF_SIGNING_EXTERN;
  console.error(
    `[lineage-commitments] ${opts.actingPeer} notarizing sunsets-lineage via ${extern}`
  );
  return createCommitment(
    opts.conductor,
    'sunsets-lineage',
    opts.payload,
    opts.signedAt ?? new Date().toISOString(),
    extern
  );
}

/**
 * Submits a `revokes-commitment` payload.
 *
 * `selfSign` (the default) routes through [`SELF_SIGNING_EXTERN`] exactly as
 * `notarizeMigration`/`notarizeSunset` do: when `payload.target_action` names a
 * lineage action, `validate_revokes_commitment` runs the SAME
 * `validate_lineage_signatures` quorum check the original crossing took, so a
 * revocation naming a lineage target needs its own quorum-worth of signatures
 * — never the target commitment's. `create_lineage_commitment` appends the
 * CALLING agent's own signature over `payload.signing_payload_cid`, which is
 * exactly the one-of-one bootstrap-steward quorum this household declares (the
 * same acting peer `notarizeMigrationPath` used to open the crossing signs its
 * reversal). Pass `selfSign: false` to submit an already-signed payload (or a
 * plain, non-lineage revocation, which carries no `signing_payload_cid` at
 * all) through the plain extern instead.
 */
export async function revokeMigration(opts: {
  conductor: ConductorRail;
  actingPeer: string;
  payload: RevocationPayload;
  signedAt?: string;
  selfSign?: boolean;
}): Promise<NotarizeResult> {
  const extern = opts.selfSign === false ? PLAIN_EXTERN : SELF_SIGNING_EXTERN;
  console.error(
    `[lineage-commitments] ${opts.actingPeer} revoking ${opts.payload.target_cid} via ${extern}`
  );
  return createCommitment(
    opts.conductor,
    'revokes-commitment',
    opts.payload,
    opts.signedAt ?? new Date().toISOString(),
    extern
  );
}
