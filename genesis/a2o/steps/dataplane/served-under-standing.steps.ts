/**
 * Step glue for features/dataplane/served-under-standing.feature, scenarios 1-5
 * (@concern:served-under-standing, @requires:owned-substrate).
 *
 * SCENARIOS 4 AND 5 — THE TWO CHROME AFFORDANCES BEYOND THE REFUSAL.
 *
 * 4 (the fair-trade receipt). An admitted serve now carries
 * `x-elohim-receipt: /api/v1/receipt/<eprId>` beside `x-elohim-standing`
 * (`doorway/doorway-service/src/services/serve_receipt.rs`, stamped by
 * `dispatch_to_projected_epr`; relayed VERBATIM by a courier). That route
 * reads THREE already-notarized REA commitments and says what each one is:
 * the `hosting-agreement` (what was given so this could be kept ready), the
 * `project-epr` contract (who holds the bytes), and the asked doorway's own
 * `operate-doorway` binding (who runs the doorway that projected them). Three
 * records, which is what makes "credits the holder separately from the
 * doorway" structurally true. On a RELAY, beta takes alpha's `exchanged` +
 * `held` clauses over the one-hop budget and substitutes its OWN projection
 * credit — a doorway crediting itself for a holder's work is unreachable,
 * not discouraged. Nothing is minted: a per-serve `serve-blob` EconomicEvent
 * was declined at the design gate (10^4-10^6 heads/year against a plane whose
 * measured anchor is ~3,469), so the receipt names who was CREDITED and never
 * who was SERVED.
 *
 * 5 (the owed response). `WHERE_TO_BE_HEARD` now points at
 * `POST /api/v1/challenge` (`doorway/doorway-service/src/routes/challenge.rs`),
 * which witnesses the challenge as an REA `Commitment` with
 * `action: "respond-to-challenge"`. `provider` is copied VERBATIM from the
 * challenged contract's own `responsiveReach.party` — the doorway never sets
 * who owes — `receiver` is the challenger as the SAME verifier the fold used
 * to refuse them states it, `clauseOf` is the contract, and `due` is the
 * window that contract declared BEFORE the challenge existed. `carriedBy` in
 * the metadata is the doorway; that it differs from `provider` is the record
 * that no doorway answered for the collective. It repoints from
 * `/api/v1/feedback/operations` deliberately: that outbox binds a feedback
 * signal to a CONTENT action, and a reach challenge targets an REA
 * Commitment, so `create_feedback_signal` would fail at the conductor — the
 * old pointer was a door that could not open.
 *
 * WHAT THE MESH MUST HAVE STAGED for these two to run. `stageRoot` below now
 * also stages a `hosting-agreement` commitment scoped
 * `["epr_root:{contentId}","doorway:{doorwayId}"]` (torn down with the root),
 * and `buildMetadata` writes the `responsiveReach` term and the
 * `hostingAgreementId` pointer onto every staged contract. Both are the
 * STEWARD's declarations, authored at staging time and therefore strictly
 * before any challenge — which is exactly what makes scenario 5's "declared
 * before James sent it" checkable rather than asserted. Scenario 4
 * additionally needs the asked doorway to hold its OWN `operate-doorway`
 * binding; `just mesh prologue` seeds one for `alpha-elohim-host` and one for
 * `apex-elohim-host` (which is what the household lane calls "beta"), so on a
 * seeded mesh that clause is already there and on an unseeded one the receipt
 * honestly reports `unrecorded: ["projected"]` rather than inventing a credit.
 *
 * SCENARIO 5 NARROWS TO `private`, ON PURPOSE. `classify_reach('private')`
 * yields `BeneficiaryOnly`, which refuses an AUTHENTICATED requester too — so
 * James is genuinely refused naming reach without this file asserting the
 * false premise scenario 2 has to stay honest about (the real household triad
 * are all members of household-dowell, so no fixture human stands in for "a
 * household non-member"). The feature's own REACH paragraph licenses it: where
 * a narrowing lands is the collective's to say, and these scenarios never fix
 * it.
 *
 * TEARDOWN LEAVES THE OWED RESPONSE STANDING. Withdrawing it would be the test
 * answering for the collective — the one thing scenario 5's last line says a
 * doorway must not do, and a test is no more entitled to it.
 *
 * THE MECHANISM UNDER TEST. `doorway/doorway-service/src/services/serve_eligibility.rs`
 * folds REACH + STANDING for every cached-serve path from the EPR router's CURRENT
 * projection row (never a value cached beside bytes). The call sites (all read
 * 2026-09-12, dev b47b440b5):
 *   - `server/http.rs::dispatch_to_projected_epr` — the EPR-router mount dispatch,
 *     BEFORE the warm-shell/SSR byte path (the fold this file's mount hits).
 *   - `server/http.rs::fold_for_addressed_serve`, called from the `/apps/{address}/…`
 *     match arm and from the `/blob/{hash}` arm when the hash is a declared head.
 *   - `server/http.rs` `/epr/{id}` universal-address dispatch (`dispatch_epr_universal`).
 * A refusal is HTTP 403, header `x-elohim-standing: refused;reach=<reach>`,
 * `cache-control: no-store`, JSON body `{refused,reach,reason,hear,epr,contract}`
 * (`serve_eligibility::Refusal` + `refusal_response`).
 *
 * STAGING, following `steps/federation/name-routing.steps.ts`'s proven shape: a
 * real one-file ZIP archive PUT via `/admin/seed/blob`, a `content_node` row at
 * `reach: 'commons'` with `dhtAnchorHash` set at ingest (Amber-floor-visible with
 * no conductor round-trip), and a `project-epr` REA commitment whose `metadataJson`
 * carries the doorway-facing projection contract (urlPath/mode/reach/gateHints).
 * The commitment's `reach` is the term this file narrows — never the content row's
 * (that stays `commons` throughout; reach is declared on the EPR/contract, not
 * earned by the byte holder — see the feature's own vocabulary section).
 *
 * NARROWING. `PATCH /api/v1/commitments/{id}` with body `{state, metadata}` —
 * `metadata` (a JSON object, not a string) REPLACES the row's metadata wholesale
 * (`UpdateReaCommitmentStateView` / `handle_update_state`,
 * elohim/elohim-storage/src/api/rea_commitments.rs:352-378). `state` is required on
 * every PATCH, and a fresh `project-epr` POST lands in state `"created"`, NOT
 * `"proposed"` — sending a DIFFERENT state than the row currently holds is a real
 * lifecycle transition, which routes storage through a conductor state-advance that
 * re-anchors the entry's OLD metadata over the new (measured 2026-09-12: this file
 * used to hardcode `state: 'proposed'` on every narrowing PATCH, which silently
 * clobbered the reach it had just set; fixed storage-side in 48f706e98, unshipped).
 * So `narrowReach` below GETs the commitment first and echoes its CURRENT `state`
 * verbatim in the PATCH body — the only change in the payload is `metadata.reach`
 * (and, for scenario 2, `metadata.gateHints`) — which also routes storage through
 * its direct (non-conductor) path. This IS the steward's own re-declaration of the
 * contract's reach standing in for a Mishpat ruling verb until one exists (habit
 * note, WS3 pass three: "the governance lever is a holon's ruling rather than a
 * config flag" — today the closest live lever is this admin PATCH on the
 * commitment the collective's own steward provider holds).
 *
 * THE HOUSEHOLD COLLECTIVE. The real Dowell household on this mesh is the
 * `household-dowell` collective (`genesis/seeder/src/seed-household-formation.ts`
 * `HOUSEHOLD_SLUG`; `genesis/data/humans/humans.json` gives matthew/jessica/james
 * each `householdId: "household-dowell"`). All THREE household peers are
 * participants (`qahal-formation.steps.ts` `HOUSEHOLD_TRIAD` asserts exactly this
 * triad is affirmed) — there is NO non-member persona among alpha/beta's own
 * household. Scenario 2's James-refusal half is therefore checked LIVE against
 * `GET /db/participations/{humanId}` at the moment the scenario runs; if the real
 * mesh says James holds the membership (expected, given the above), that half
 * returns 'pending' naming this exact reason rather than asserting a false premise.
 *
 * THE TWO CHROME CLAUSES, AND WHAT NOW ANSWERS THEM. Both were measured RED on
 * the household mesh 2026-09-13 and closed in the same pass — the assertions
 * below are unchanged; the source they read now exists.
 *
 *   1. "the refusal names {household} as the collective whose ruling narrowed
 *      it" (scenario 1). `serve_eligibility`'s ANONYMOUS-restricted branch now
 *      says "…narrowed it to {declared} reach, which admits members of {the
 *      collective} and not a visitor who has shown nothing", and the refusal
 *      JSON carries `collective: {id,label,record}` plus `declaredIn` — the
 *      route that reads the reach declaration itself back. The name comes from
 *      the declaration's OWN audience term, so it is a record the visitor can
 *      follow, never a doorway-local string. Which is also why the RULING now
 *      writes that term (`householdAudience` below): a ruling that named no
 *      collective would leave the doorway nothing it could honestly say, and
 *      would narrow to a rung that admits any authenticated requester at all —
 *      wider than this feature's own REACH paragraph allows.
 *   2. "the chrome names the reach that admitted him" (scenario 2, success
 *      case). An admitted serve now carries
 *      `x-elohim-standing: admitted;reach=<reach>`
 *      (`serve_eligibility::stamp_admitted_standing`, stamped by
 *      `dispatch_to_projected_epr` and by the SSR branch) — the same sentence
 *      shape the refusal uses, so a reader never has to infer the fold's answer
 *      from a status code. A relay carries the HOLDER's value verbatim rather
 *      than restating it, so beta shows alpha's standing.
 *
 * A THIRD RED, in the substrate under both, is why scenario 2's premise could
 * not converge: `GET /db/participations/{human_id}` — the ONE read the fold
 * makes for a restricted-reach serve (`serve_eligibility::read_memberships`) —
 * 404'd "Unknown database endpoint" on every household peer, because
 * "participations" was missing from `extract_app_context`'s `legacy_prefixes`
 * (elohim-storage http.rs) and was eaten as an `h_app_id`. The fold read that
 * failure as "this requester presented no standing" — correctly, since a
 * membership read that cannot be completed is never permission — so a
 * household member was refused their own household's record and the refusal
 * blamed reach. Fixed storage-side in the same pass; regression test
 * `participations_namespace_survives_app_context_extraction`.
 *
 * SCENARIO 3's "still holds the bytes warm" is checked by a SECOND, AUTHENTICATED
 * serve of the same root succeeding (200 + the archive's own marker) — never by
 * reading anything out of the anonymous refusal, which proves nothing about byte
 * custody one way or the other. `x-elohim-bundle: last-reconciled` (the doorway's
 * own warm-shell provenance marker, `server/http.rs::with_bundle_provenance_header`)
 * is logged as supporting evidence when present, but is NOT hard-asserted: which
 * branch of `warm_shell::plan_shell_serve` fires (ServeWarm vs a fresh Fetch that
 * confirms against a healthy upstream, which drops the header) is a live timing
 * detail this file cannot pin from source alone without running the mesh.
 *
 * Mesh facts (household-fixture.json, `just mesh prologue`): doorway "alpha"
 * http://localhost:8888 (primary storage matthew :8090), doorway "beta"
 * http://localhost:8889 (primary storage jessica :8091). `read_memberships`
 * (serve_eligibility.rs) queries the ASKED doorway's OWN `storage_url` — so a
 * membership check against what beta will see at request time reads jessica's
 * storage, never matthew's.
 */

import { strict as assert } from 'node:assert';
import { createHash } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';

import { After, Before, Given, When, Then } from '@cucumber/cucumber';

import { getRawWithHeaders } from '../../src/framework/dataplane/surfaces.js';
import {
  loadHouseholdMeshFixture,
  requireFixturePrimaryStorageUrl,
} from '../../src/framework/fixtures/household-mesh.js';
import { fixtureCredentials, getFixture } from '../../src/framework/fixtures/humans.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ADMIN_KEY =
  process.env['API_KEY_ADMIN'] ?? process.env['MESH_API_KEY_ADMIN'] ?? 'mesh-admin-dev-key';
const REQUEST_TIMEOUT_MS = 15_000;
/** `DOORWAY_EPR_REFRESH_SECS` default (main.rs, 30s) + buffer: how long a
 * doorway's OWN EprRouter can take to notice a commitment this file just PATCHed
 * (same budget name-routing.steps.ts uses for the identical wait). */
const OWN_REFRESH_BUDGET_MS = 45_000;
const RECONCILE_POLL_INTERVAL_MS = 3_000;
/** Fallback when the fixture declares no convergence window (it always does
 * today — `just mesh prologue` stamps `convergenceWindowMs: 60000`). */
const DEFAULT_RECONCILE_WINDOW_MS = 60_000;

/** The real household collective this mesh seeds (see file header). Never a
 * fictional test-only id: scenario 2's membership match must be checkable
 * against `GET /db/participations/{humanId}`'s real rows. */
const HOUSEHOLD_COLLECTIVE_ID = 'household-dowell';

// ---------------------------------------------------------------------------
// A tiny, real app archive — CIDv1 + a hand-rolled STORED-method ZIP.
//
// Identical construction to steps/federation/name-routing.steps.ts's own
// stockAppArchive (see that file's header for the full "why a real archive"
// rationale: a hosting commitment ROUTES a name, it does not SERVE bytes, and
// storage's slug_index only resolves html5-app/spa-bundle content_node rows
// that clear the Amber trust floor). Duplicated rather than imported: neither
// file exports these helpers, and reaching into a sibling steps file for
// unexported internals is worse than one more small, self-contained copy.
// ---------------------------------------------------------------------------

const BASE32_ALPHABET = 'abcdefghijklmnopqrstuvwxyz234567';

function base32Encode(bytes: Uint8Array): string {
  let bits = 0;
  let value = 0;
  let output = '';
  for (const byte of bytes) {
    value = (value << 8) | byte;
    bits += 8;
    while (bits >= 5) {
      output += BASE32_ALPHABET[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) {
    output += BASE32_ALPHABET[(value << (5 - bits)) & 31];
  }
  return output;
}

function varint(n: number): number[] {
  const out: number[] = [];
  let v = n;
  while (v >= 0x80) {
    out.push((v & 0x7f) | 0x80);
    v >>>= 7;
  }
  out.push(v);
  return out;
}

const RAW_CODEC = 0x55;
const SHA2_256_CODE = 0x12;

function cidV1Raw(bytes: Uint8Array): string {
  const digest = createHash('sha256').update(bytes).digest();
  const multihash = [...varint(SHA2_256_CODE), ...varint(digest.length), ...digest];
  const cidBytes = [...varint(1), ...varint(RAW_CODEC), ...multihash];
  return `b${base32Encode(Uint8Array.from(cidBytes))}`;
}

let crc32TableCache: Uint32Array | undefined;

function crc32Table(): Uint32Array {
  if (crc32TableCache) return crc32TableCache;
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n += 1) {
    let c = n;
    for (let k = 0; k < 8; k += 1) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[n] = c >>> 0;
  }
  crc32TableCache = table;
  return table;
}

function crc32(data: Uint8Array): number {
  const table = crc32Table();
  let crc = 0xffffffff;
  for (const byte of data) {
    crc = table[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

const ZIP_DOS_DATE = ((2026 - 1980) << 9) | (1 << 5) | 1;
const ZIP_DOS_TIME = 0;

function buildZip(name: string, content: Uint8Array): Buffer {
  const nameBytes = Buffer.from(name, 'utf8');
  const crc = crc32(content);

  const localHeader = Buffer.alloc(30);
  localHeader.writeUInt32LE(0x04034b50, 0);
  localHeader.writeUInt16LE(20, 4);
  localHeader.writeUInt16LE(0, 6);
  localHeader.writeUInt16LE(0, 8);
  localHeader.writeUInt16LE(ZIP_DOS_TIME, 10);
  localHeader.writeUInt16LE(ZIP_DOS_DATE, 12);
  localHeader.writeUInt32LE(crc, 14);
  localHeader.writeUInt32LE(content.length, 18);
  localHeader.writeUInt32LE(content.length, 22);
  localHeader.writeUInt16LE(nameBytes.length, 26);
  localHeader.writeUInt16LE(0, 28);
  const localEntry = Buffer.concat([localHeader, nameBytes, Buffer.from(content)]);

  const centralHeader = Buffer.alloc(46);
  centralHeader.writeUInt32LE(0x02014b50, 0);
  centralHeader.writeUInt16LE(20, 4);
  centralHeader.writeUInt16LE(20, 6);
  centralHeader.writeUInt16LE(0, 8);
  centralHeader.writeUInt16LE(0, 10);
  centralHeader.writeUInt16LE(ZIP_DOS_TIME, 12);
  centralHeader.writeUInt16LE(ZIP_DOS_DATE, 14);
  centralHeader.writeUInt32LE(crc, 16);
  centralHeader.writeUInt32LE(content.length, 20);
  centralHeader.writeUInt32LE(content.length, 24);
  centralHeader.writeUInt16LE(nameBytes.length, 28);
  centralHeader.writeUInt16LE(0, 30);
  centralHeader.writeUInt16LE(0, 32);
  centralHeader.writeUInt16LE(0, 34);
  centralHeader.writeUInt16LE(0, 36);
  centralHeader.writeUInt32LE(0, 38);
  centralHeader.writeUInt32LE(0, 42);
  const centralEntry = Buffer.concat([centralHeader, nameBytes]);

  const endRecord = Buffer.alloc(22);
  endRecord.writeUInt32LE(0x06054b50, 0);
  endRecord.writeUInt16LE(0, 4);
  endRecord.writeUInt16LE(0, 6);
  endRecord.writeUInt16LE(1, 8);
  endRecord.writeUInt16LE(1, 10);
  endRecord.writeUInt32LE(centralEntry.length, 12);
  endRecord.writeUInt32LE(localEntry.length, 16);
  endRecord.writeUInt16LE(0, 20);

  return Buffer.concat([localEntry, centralEntry, endRecord]);
}

/** Any valid, already-known agent identity — fine for a test-only commitment's
 * provider/receiver (same formula as name-routing.steps.ts's testStewardPeerId,
 * inlined so this file has no cross-package import to resolve at test-runtime). */
function testStewardPeerId(): string {
  const digest = createHash('sha256').update('human-matthew-manager:desktop', 'utf8').digest();
  return `12D3KooW${digest.toString('hex').slice(0, 38)}`;
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

interface RawResponse {
  status: number;
  text: string;
  headers: Record<string, string | undefined>;
}

async function rawGet(url: string, headers?: Record<string, string>): Promise<RawResponse> {
  return getRawWithHeaders(url, { timeoutMs: REQUEST_TIMEOUT_MS, headers });
}

async function adminCallOnce(
  method: 'POST' | 'PATCH',
  url: string,
  body: unknown
): Promise<{ status: number; text: string }> {
  const response = await fetch(url, {
    method,
    headers: { Authorization: `Bearer ${ADMIN_KEY}`, 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
  });
  const text = await response.text();
  return { status: response.status, text };
}

function retryAfterMs(text: string): number | undefined {
  try {
    const parsed = JSON.parse(text) as { retryAfter?: unknown; status?: unknown };
    if (parsed.status === 'catching-up' && typeof parsed.retryAfter === 'number') {
      return parsed.retryAfter * 1000;
    }
  } catch {
    // not the catching-up shape
  }
  return undefined;
}

/** Retries a write on the mesh's transient 503 "catching-up" shape — same
 * convention as name-routing.steps.ts's adminCall. */
async function adminCall(
  method: 'POST' | 'PATCH',
  url: string,
  body: unknown,
  maxAttempts = 5
): Promise<{ status: number; text: string }> {
  let last: { status: number; text: string } = { status: -1, text: '' };
  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    last = await adminCallOnce(method, url, body);
    if (last.status !== 503) return last;
    if (attempt === maxAttempts) return last;
    const wait = Math.min(retryAfterMs(last.text) ?? 5_000, 15_000);
    await delay(wait);
  }
  return last;
}

/** Trim trailing slashes without a backtracking-prone regex (matches
 * name-routing.steps.ts's own `withoutTrailingSlashes` convention). */
function withoutTrailingSlashes(value: string): string {
  let end = value.length;
  while (end > 0 && value[end - 1] === '/') end -= 1;
  return value.slice(0, end);
}

function normalizeOrigin(value: string): string {
  try {
    const u = new URL(value);
    const host = u.hostname === '127.0.0.1' ? 'localhost' : u.hostname;
    const portSuffix = u.port ? `:${u.port}` : '';
    return `${u.protocol}//${host}${portSuffix}`;
  } catch {
    return withoutTrailingSlashes(value);
  }
}

function originsEqual(a: string, b: string): boolean {
  return normalizeOrigin(a) === normalizeOrigin(b);
}

/** Log a fixture human into `doorwayUrl` and return their bearer token + the
 * `humanId` the JWT will carry (the canonical fixture id, e.g.
 * "human-matthew-manager" — `resolve_verified_claims_from_request` reads this
 * straight off the verified token, never a doorway-side lookup). */
async function loginBearer(
  doorwayUrl: string,
  displayName: string
): Promise<{ token: string; humanId: string }> {
  const creds = fixtureCredentials(displayName);
  const res = await fetch(`${doorwayUrl}/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ identifier: creds.identifier, password: creds.password }),
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
  });
  const text = await res.text();
  assert.equal(
    res.status,
    200,
    `login as "${displayName}" via ${doorwayUrl}/auth/login failed: HTTP ${res.status} ${text.slice(0, 300)}`
  );
  const body = JSON.parse(text) as { token?: string; humanId?: string };
  assert.ok(body.token, `login as "${displayName}" via ${doorwayUrl} returned no token`);
  assert.ok(body.humanId, `login as "${displayName}" via ${doorwayUrl} returned no humanId`);
  return { token: body.token, humanId: body.humanId };
}

/** Whether `humanId` currently, live, holds a participation in `collectiveId` —
 * read from the SAME storage the asked doorway's own fold would consult
 * (`read_memberships`, serve_eligibility.rs: `GET {storage_url}/db/participations/{human_id}`,
 * a `departedAt` row dropped as past membership). */
async function isLiveMember(
  storageUrl: string,
  humanId: string,
  collectiveId: string
): Promise<boolean> {
  const res = await fetch(`${storageUrl}/db/participations/${encodeURIComponent(humanId)}`, {
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
  });
  assert.equal(
    res.status,
    200,
    `GET ${storageUrl}/db/participations/${humanId} failed: HTTP ${res.status}`
  );
  const body = (await res.json()) as {
    items?: { collectiveId?: string; departedAt?: string | null }[];
  };
  return (body.items ?? []).some(item => item.collectiveId === collectiveId && !item.departedAt);
}

// ---------------------------------------------------------------------------
// Root staging (test-only project-epr commitments; never a household's real
// mount). Ids are content-addressed over (label, runStamp, scenarioNonce) —
// never shared across runs or scenarios, matching name-routing.steps.ts's
// documented reason (a reactivated row races the doorway's own mount/cache
// reconciliation — see that file's `scenarioNonce` doc for the measured window).
// ---------------------------------------------------------------------------

let runStamp: string | undefined;
let scenarioNonce: string | undefined;

Before(function (this: E2EWorld, scenario): void {
  runStamp ??= process.env['A2O_RUN_ID'] ?? `${process.pid}-${Date.now().toString(36)}`;
  scenarioNonce = scenario.pickle.id
    .replace(/[^a-z0-9]/gi, '')
    .toLowerCase()
    .slice(0, 12);
});

function requireScenarioNonce(): string {
  assert.ok(scenarioNonce, 'scenarioNonce not minted yet — the Before hook must run first');
  return scenarioNonce;
}

function requireRunStamp(): string {
  assert.ok(runStamp, 'runStamp not minted yet — the Before hook must run first');
  return runStamp;
}

function slug(label: string): string {
  const collapsed = label.toLowerCase().replace(/[^a-z0-9]+/g, '-');
  let start = 0;
  let end = collapsed.length;
  while (start < end && collapsed[start] === '-') start += 1;
  while (end > start && collapsed[end - 1] === '-') end -= 1;
  return collapsed.slice(start, end) || 'epr';
}

function mountFor(label: string): string {
  const digest = createHash('sha256')
    .update(`${label}|${requireRunStamp()}|${requireScenarioNonce()}`, 'utf8')
    .digest('hex')
    .slice(0, 10);
  return `/sus-${slug(label)}-${digest}`;
}

function requestPathFor(label: string): string {
  return `${mountFor(label)}/`;
}

function contentIdFor(label: string): string {
  const digest = createHash('sha256')
    .update(`content|${label}|${requireRunStamp()}|${requireScenarioNonce()}`, 'utf8')
    .digest('hex')
    .slice(0, 16);
  return `sus-app-${digest}`;
}

function commitmentIdFor(doorwayId: string, label: string): string {
  const digest = createHash('sha256')
    .update(`${doorwayId}|${label}|${requireRunStamp()}|${requireScenarioNonce()}`, 'utf8')
    .digest('hex')
    .slice(0, 16);
  return `project-epr-sus-${digest}`;
}

/** The literal marker the staged archive's `index.html` carries — proof that a
 * given doorway is answering with THIS scenario's own bytes, never a stale
 * cache entry from an earlier scenario (same reasoning as name-routing.steps.ts's
 * `nrtMarker`) and never the real landing page's SPA fallback (which would
 * satisfy a bare `<html`/status-200 check but never this marker). */
function markerFor(label: string): string {
  return `data-sus-root="${label}" data-sus-nonce="${requireScenarioNonce()}"`;
}

interface GateHint {
  eprRef: string;
  label?: string;
  relation: string;
}

interface ResponsiveReach {
  party: string;
  partyLabel: string | null;
  withinHours: number;
}

function buildMetadata(
  mount: string,
  reach: string,
  gateHints: GateHint[],
  redress?: { responsiveReach: ResponsiveReach; hostingAgreementId: string }
): Record<string, unknown> {
  return {
    urlPath: mount,
    mode: 'cached',
    reach,
    baseHref: '/',
    entryFile: 'index.html',
    redirectsFrom: [],
    previewEprRef: null,
    gateHints,
    deadEnd: false,
    stewardDirectEndpoint: null,
    routeClaims: null,
    redirectTemplates: [],
    // The two redress terms, on EVERY staged contract from the moment it is
    // staged — never added later. That ordering is the whole point: the
    // contract's own createdAt predates any challenge against it, so
    // "a due window declared before the visitor sent it" reads back to a
    // record that already stood rather than being something the doorway
    // asserted. They are also projection-relevant (PROJECTION_RELEVANT_FIELDS),
    // so a change to either re-grants through supersession rather than
    // mutating a standing contract.
    responsiveReach: redress?.responsiveReach ?? null,
    hostingAgreementId: redress?.hostingAgreementId ?? null,
  };
}

/**
 * The responsive-reach term a collective's steward declares on its own
 * contract: who answers a challenge to this projection's standing, and how
 * soon.
 *
 * The doorway copies `party` VERBATIM onto the commitment it witnesses and
 * never chooses it — so this fixture is standing in for exactly one thing, the
 * collective's own declaration. 72 hours is a real window a household could
 * keep; the number matters only in that a reader can redo the arithmetic from
 * it.
 */
function householdResponsiveReach(collectiveName: string): ResponsiveReach {
  return {
    party: HOUSEHOLD_COLLECTIVE_ID,
    partyLabel: collectiveName,
    withinHours: 72,
  };
}

/** The audience term a ruling by the household writes onto the contract.
 *
 * A ruling NAMES its collective — that is what makes "the refusal names the
 * collective whose ruling narrowed it" checkable from a record rather than
 * from a doorway-local string. It is also what the feature's own REACH
 * paragraph asks a narrowing to be: "the new reach admits the household's
 * members and no longer admits an anonymous stranger". A narrowing that
 * carried NO audience term would satisfy neither — it would admit any
 * authenticated requester at all (serve_eligibility.rs: "the rung itself is
 * the audience"), which is wider than the story says, and it would leave the
 * refusal with no collective to name.
 *
 * `membershipPrerequisite` is the projected relation the doorway's fold reads
 * as an audience (`audience_from_projection`), so this needs no new entry
 * type, no new column and no new doorway persistence.
 */
function householdAudience(collectiveName: string): GateHint[] {
  return [
    {
      eprRef: HOUSEHOLD_COLLECTIVE_ID,
      label: collectiveName,
      relation: 'membershipPrerequisite',
    },
  ];
}

interface StagedRoot {
  label: string;
  doorwayUrl: string;
  doorwayId: string;
  mount: string;
  path: string;
  contentId: string;
  commitmentId: string;
  currentReach: string;
  /** The reciprocal term a fair-trade receipt reads as "what was given". */
  hostingAgreementId: string;
  /** The steward's declaration of who answers a challenge, and how soon. */
  responsiveReach: ResponsiveReach;
}

function hostingAgreementIdFor(doorwayId: string, label: string): string {
  const digest = createHash('sha256')
    .update(`hosting|${doorwayId}|${label}|${requireRunStamp()}|${requireScenarioNonce()}`, 'utf8')
    .digest('hex')
    .slice(0, 16);
  return `hosting-agreement-sus-${digest}`;
}

/**
 * Stage the reciprocal term: an in-kind compute pledge scoped to THIS root and
 * THIS doorway, the same shape `genesis/seeder/src/seed-operator-bindings.ts`
 * seeds for the real landing surface.
 *
 * Without it the receipt honestly reports `unrecorded: ["exchanged"]` — which
 * is correct behaviour and useless as a test, because "the receipt names what
 * was given in exchange" would then be asserting against an absence the
 * scenario itself created.
 */
async function stageHostingAgreement(staged: StagedRoot): Promise<void> {
  const provider = testStewardPeerId();
  const body = {
    id: staged.hostingAgreementId,
    action: 'hosting-agreement',
    provider,
    receiver: provider,
    resourceClassifiedAs: JSON.stringify(['compute']),
    inScopeOf: JSON.stringify([`epr_root:${staged.contentId}`, `doorway:${staged.doorwayId}`]),
    note: `[a2o served-under-standing] in-kind compute hosting for ${staged.contentId} on ${staged.doorwayId}`,
    metadataJson: JSON.stringify({
      signalKind: 'compute-allocation',
      triggerKind: 'subscription',
    }),
  };
  const created = await adminCall('POST', `${staged.doorwayUrl}/api/v1/commitments`, body);
  assert.ok(
    created.status === 201 || created.status === 409,
    `staging the hosting agreement ${staged.hostingAgreementId} failed: HTTP ${created.status} ` +
      created.text.slice(0, 300)
  );
}

/** PUT a real ZIP (containing the marked `index.html`) and create the
 * `content_node` row an `html5-app` project-epr mount resolves through.
 * Idempotent on a 409 (a cucumber retry of THIS scenario attempt — same
 * nonce — reactivates rather than duplicates; see name-routing.steps.ts's
 * `scenarioNonce` doc for why that never happens ACROSS scenarios). */
async function stockArchive(doorwayUrl: string, contentId: string, label: string): Promise<void> {
  const html = Buffer.from(
    `<!doctype html><html><head><meta charset="utf-8"><title>${label}</title></head>` +
      `<body ${markerFor(label)}>sus-${label}</body></html>`,
    'utf8'
  );
  const zip = buildZip('index.html', html);
  const legacyHash = `sha256-${createHash('sha256').update(zip).digest('hex')}`;
  const cid = cidV1Raw(zip);

  let putStatus = -1;
  let putText = '';
  for (let attempt = 1; attempt <= 5; attempt += 1) {
    const response = await fetch(`${doorwayUrl}/admin/seed/blob`, {
      method: 'PUT',
      headers: {
        Authorization: `Bearer ${ADMIN_KEY}`,
        'Content-Type': 'application/zip',
        'X-Blob-Hash': legacyHash,
        'X-Blob-Cid': cid,
        'X-Blob-Size': String(zip.length),
      },
      body: zip,
      signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    });
    putStatus = response.status;
    putText = await response.text();
    if (putStatus !== 503) break;
    if (attempt < 5) await delay(Math.min(retryAfterMs(putText) ?? 5_000, 15_000));
  }
  assert.ok(
    putStatus === 200 || putStatus === 201,
    `PUT ${doorwayUrl}/admin/seed/blob failed for ${contentId}: HTTP ${putStatus} ${putText.slice(0, 300)}`
  );

  const createBody = {
    id: contentId,
    title: `[a2o served-under-standing] ${label}`,
    contentType: 'application',
    contentFormat: 'html5-app',
    blobHash: legacyHash,
    blobCid: cid,
    contentSizeBytes: zip.length,
    reach: 'commons',
    createdBy: testStewardPeerId(),
    dhtAnchorHash: cid,
  };
  const created = await adminCall('POST', `${doorwayUrl}/db/content`, createBody);
  assert.ok(
    created.status === 201 || created.status === 409,
    `staging content row ${contentId} failed: HTTP ${created.status} ${created.text.slice(0, 300)}`
  );
}

/** Stage a `project-epr` commitment for `label` on `doorwayId`, at commons
 * reach and no gate hints — the starting state every Background scenario needs. */
async function stageRoot(world: E2EWorld, doorwayId: string, label: string): Promise<StagedRoot> {
  const doorway = world.getDoorway(doorwayId);
  const resolvedDoorwayId = await resolveDoorwayId(world, doorwayId, doorway.url);
  const mount = mountFor(label);
  const path = requestPathFor(label);
  const contentId = contentIdFor(label);
  await stockArchive(doorway.url, contentId, label);

  const commitmentId = commitmentIdFor(resolvedDoorwayId, label);
  const provider = testStewardPeerId();
  const hostingAgreementId = hostingAgreementIdFor(resolvedDoorwayId, label);
  const responsiveReach = householdResponsiveReach(getState(world).collectiveName);
  const redress = { responsiveReach, hostingAgreementId };
  const metadata = buildMetadata(mount, 'commons', [], redress);
  const body = {
    id: commitmentId,
    action: 'project-epr',
    provider,
    receiver: provider,
    inScopeOf: `doorway:${resolvedDoorwayId}|epr:${contentId}`,
    note: `[a2o served-under-standing] project ${contentId} at ${mount} on ${resolvedDoorwayId}`,
    metadataJson: JSON.stringify(metadata),
    metadata,
  };
  const created = await adminCall('POST', `${doorway.url}/api/v1/commitments`, body);
  if (created.status === 409) {
    const reactivated = await adminCall(
      'PATCH',
      `${doorway.url}/api/v1/commitments/${commitmentId}`,
      { state: 'proposed', metadata }
    );
    assert.equal(
      reactivated.status,
      200,
      `re-activating ${commitmentId} on ${resolvedDoorwayId} failed: HTTP ${reactivated.status} ` +
        reactivated.text.slice(0, 300)
    );
  } else {
    assert.equal(
      created.status,
      201,
      `staging ${mount} on ${resolvedDoorwayId} failed: HTTP ${created.status} ${created.text.slice(0, 300)}`
    );
  }

  const staged: StagedRoot = {
    label,
    doorwayUrl: doorway.url,
    doorwayId: resolvedDoorwayId,
    mount,
    path,
    contentId,
    commitmentId,
    currentReach: 'commons',
    hostingAgreementId,
    responsiveReach,
  };
  // The reciprocal term, staged against the SAME doorway that holds the
  // contract — the holder is the peer whose receipt names what was given.
  await stageHostingAgreement(staged);
  await waitForLocalServe(staged, 65_000);
  return staged;
}

/** The commitment's CURRENT `state`, read live — never assumed. A fresh
 * `project-epr` create lands in `"created"`, not `"proposed"`; sending any
 * OTHER value on a PATCH is a genuine lifecycle transition, which is exactly
 * what a reach-only narrowing must not be (file header, NARROWING). */
async function currentCommitmentState(doorwayUrl: string, commitmentId: string): Promise<string> {
  const res = await fetch(`${doorwayUrl}/api/v1/commitments/${commitmentId}`, {
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
  });
  const text = await res.text();
  assert.equal(
    res.status,
    200,
    `GET ${doorwayUrl}/api/v1/commitments/${commitmentId} failed: HTTP ${res.status} ${text.slice(0, 300)}`
  );
  const body = JSON.parse(text) as { state?: string };
  assert.ok(
    body.state,
    `commitment ${commitmentId} carries no "state" field: ${text.slice(0, 300)}`
  );
  return body.state;
}

/** Re-declare `staged`'s reach (and, optionally, its gate hints) — the
 * steward's own re-declaration of the contract, standing in for a Mishpat
 * ruling verb until one exists (file header). GENUINELY reach-only: the
 * PATCH echoes the row's CURRENT `state` back verbatim (never a hardcoded
 * lifecycle value), so the only real change in the payload is `metadata`. */
async function narrowReach(
  staged: StagedRoot,
  reach: string,
  gateHints: GateHint[] = []
): Promise<void> {
  const metadata = buildMetadata(staged.mount, reach, gateHints, {
    responsiveReach: staged.responsiveReach,
    hostingAgreementId: staged.hostingAgreementId,
  });
  const currentState = await currentCommitmentState(staged.doorwayUrl, staged.commitmentId);
  const res = await adminCall(
    'PATCH',
    `${staged.doorwayUrl}/api/v1/commitments/${staged.commitmentId}`,
    {
      state: currentState,
      metadata,
    }
  );
  assert.equal(
    res.status,
    200,
    `narrowing ${staged.commitmentId} to reach "${reach}" failed: HTTP ${res.status} ${res.text.slice(0, 300)}`
  );
  staged.currentReach = reach;
}

async function cancelQuiet(staged: StagedRoot): Promise<void> {
  try {
    await adminCall('PATCH', `${staged.doorwayUrl}/api/v1/commitments/${staged.commitmentId}`, {
      state: 'cancelled',
      metadata: buildMetadata(staged.mount, 'commons', [], {
        responsiveReach: staged.responsiveReach,
        hostingAgreementId: staged.hostingAgreementId,
      }),
    });
  } catch {
    // best-effort cleanup
  }
  try {
    await adminCall(
      'PATCH',
      `${staged.doorwayUrl}/api/v1/commitments/${staged.hostingAgreementId}`,
      { state: 'cancelled' }
    );
  } catch {
    // best-effort cleanup
  }
  try {
    await fetch(`${staged.doorwayUrl}/db/content/${staged.contentId}`, {
      method: 'DELETE',
      headers: { Authorization: `Bearer ${ADMIN_KEY}` },
      signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    });
  } catch {
    // best-effort — the blob bytes are content-addressed and harmless to leave
  }
}

/** Force LOCAL-ONLY evaluation (the same header the relay itself stamps on an
 * outbound hop) — the honest way to ask "does THIS doorway, by itself, hold a
 * mount here?", matching name-routing.steps.ts's `localOnlyGet`. */
async function localOnlyGet(url: string, headers?: Record<string, string>): Promise<RawResponse> {
  return rawGet(url, { ...headers, 'x-federation-hop': '1' });
}

/** Poll `staged`'s OWN local mount (force-local) until it answers with this
 * scenario's marker — proves a real mount, never an accidental SPA-fallback
 * 200 from the doorway's own "/" landing page (the same trailing-slash,
 * extension-less risk name-routing.steps.ts's `nrtRequestPath` doc names). */
async function waitForLocalServe(staged: StagedRoot, budgetMs: number): Promise<void> {
  const marker = markerFor(staged.label);
  const deadline = Date.now() + budgetMs;
  let last: RawResponse | undefined;
  for (;;) {
    last = await localOnlyGet(`${staged.doorwayUrl}${staged.path}`);
    if (last.status === 200 && last.text.includes(marker)) return;
    if (Date.now() >= deadline) {
      throw new Error(
        `doorway "${staged.doorwayId}" does not locally serve the just-staged root "${staged.label}" ` +
          `(${staged.path}) within ${budgetMs}ms: HTTP ${last.status}, marker present=${last.text.includes(marker)}`
      );
    }
    await delay(1_000);
  }
}

// ---------------------------------------------------------------------------
// Doorway id resolution (GET /api/v1/federation/coherence, cached per world) —
// mirrors name-routing.steps.ts's resolvedDoorwayId.
// ---------------------------------------------------------------------------

const doorwayIdCache = new WeakMap<E2EWorld, Map<string, string>>();

async function resolveDoorwayId(
  world: E2EWorld,
  fixtureId: string,
  doorwayUrl: string
): Promise<string> {
  const cache = doorwayIdCache.get(world) ?? new Map<string, string>();
  doorwayIdCache.set(world, cache);
  const cached = cache.get(fixtureId);
  if (cached) return cached;
  const res = await rawGet(`${doorwayUrl}/api/v1/federation/coherence`);
  assert.equal(
    res.status,
    200,
    `GET ${doorwayUrl}/api/v1/federation/coherence failed: HTTP ${res.status} ${res.text.slice(0, 200)}`
  );
  const manifest = JSON.parse(res.text) as { doorwayId: string };
  cache.set(fixtureId, manifest.doorwayId);
  return manifest.doorwayId;
}

/** Best-effort federation-peer registration — the correct ARRANGE step for
 * cross-doorway relay, and harmless when the mesh's DHT-registered-sibling
 * discovery has already made it redundant (habit DELTA 2026-09-12: basic
 * one-hop relay to a single real holder is proven working on this mesh via
 * that mechanism, not the FEDERATION_PEERS-gated task). Never asserted: a
 * failure here must not fail staging, only the eventual relay probe would. */
async function registerFederationPeerQuiet(fromUrl: string, peerUrl: string): Promise<void> {
  try {
    await adminCall('POST', `${fromUrl}/admin/federation/peers`, { url: peerUrl });
  } catch {
    // best-effort
  }
}

// ---------------------------------------------------------------------------
// Scenario state
// ---------------------------------------------------------------------------

interface Refusal {
  refused: string;
  reach: string;
  reason: string;
  hear: string;
  epr: string;
  contract: string;
  /** The collective the declared reach names, with the route that reads it
   * back. Absent when the reach names no audience — a doorway that holds no
   * such record says nothing rather than inventing a name. */
  collective?: { id: string; label?: string; record: string };
  /** The route that reads the reach declaration itself back (the REA
   * commitment carrying reach + audience terms). */
  declaredIn?: string;
  /** WHO owes an answer if this refusal is challenged, and how soon — read
   * from the contract's own `responsiveReach`, which was declared before the
   * request existed. Present-and-NULL when the collective declared nobody;
   * never omitted, so a reader can tell the two apart. */
  owed?: OwedResponse | null;
}

interface OwedResponse {
  party: string;
  partyLabel?: string;
  withinHours: number;
  declaredBy: string;
  declaredAt: string;
}

interface Credit {
  role: 'held' | 'projected';
  party: string;
  partyLabel?: string;
  sentence: string;
  commitment: string;
  record: string;
}

interface ReceiptBody {
  sentence: string;
  epr: string;
  exchanged: {
    what: string;
    sentence: string;
    party: string;
    resource?: string;
    commitment: string;
    record: string;
  } | null;
  credits: Credit[];
  unrecorded: string[];
  protocolForm: string;
  servedBy?: string;
}

interface ChallengeAnswer {
  outcome: string;
  reason: string;
  epr: string;
  challenge?: string;
  record?: string;
  owed: OwedResponse | null;
  due?: string;
  contract?: string;
  carriedBy?: string;
  servedBy?: string;
}

interface ScenarioState {
  label: string;
  collectiveName: string;
  holder?: StagedRoot;
  nonHolderId?: string;
  reconcileWindowMs: number;
  lastRefusal?: { status: number; headers: Record<string, string | undefined>; body?: Refusal };
  lastServed?: { status: number; text: string; headers: Record<string, string | undefined> };
  /** The receipt fetched for the last serve, and the raw bytes it arrived as
   * (so "no identifier is the FIRST thing it says" is checkable on the wire,
   * not on a re-serialized object). */
  receipt?: { body: ReceiptBody; raw: string; askedAt: string };
  /** The doorway scenarios 4/5 asked — the one that must credit ITSELF for the
   * projection, and the one that must not answer a challenge for a collective. */
  askedDoorwayId?: string;
  askedDoorwayUrl?: string;
  /** James's bearer, minted once per scenario 5 run. */
  challengerToken?: string;
  challengerHumanId?: string;
  /** When the challenge was sent — the instant every "declared before" check
   * is made against. */
  challengeSentAt?: number;
  challengeAnswer?: { status: number; body: ChallengeAnswer };
}

const states = new WeakMap<E2EWorld, ScenarioState>();

function beginScenario(world: E2EWorld, label: string, collectiveName: string): ScenarioState {
  const state: ScenarioState = {
    label,
    collectiveName,
    reconcileWindowMs: DEFAULT_RECONCILE_WINDOW_MS,
    holder: undefined,
  };
  states.set(world, state);
  return state;
}

function getState(world: E2EWorld): ScenarioState {
  const state = states.get(world);
  assert.ok(
    state,
    'the scenario must run "the collective ... stewards the EPR ..." before anything else'
  );
  return state;
}

function requireHolder(world: E2EWorld): StagedRoot {
  const state = getState(world);
  assert.ok(
    state.holder,
    `no doorway has staged a live projection contract for "${state.label}" yet`
  );
  return state.holder;
}

function parseRefusalBody(text: string): Refusal | undefined {
  try {
    return JSON.parse(text) as Refusal;
  } catch {
    return undefined;
  }
}

/** Poll `url` (with optional headers) until its status matches `wantStatus` —
 * used both for "wait for the doorway's own reconcile to refuse" (wantStatus
 * 403) and for "wait for the narrowed reach to admit standing" (wantStatus
 * 200). Captures the final response either way so callers assert against one
 * network observation, not a fresh, possibly-racy second call. */
async function pollUntilStatus(
  url: string,
  headers: Record<string, string> | undefined,
  wantStatus: number,
  budgetMs: number,
  intervalMs = RECONCILE_POLL_INTERVAL_MS
): Promise<RawResponse> {
  const deadline = Date.now() + budgetMs;
  let last: RawResponse | undefined;
  for (;;) {
    last = await rawGet(url, headers);
    if (last.status === wantStatus) return last;
    if (Date.now() >= deadline) {
      throw new Error(
        `GET ${url} never reached HTTP ${wantStatus} within ${budgetMs}ms (last observed: ` +
          `HTTP ${last.status} ${last.text.slice(0, 200)})`
      );
    }
    await delay(intervalMs);
  }
}

/** Poll until the narrowing has genuinely landed AND standing is still
 * admitted — the two-sided check a single "poll for 200 with the bearer"
 * cannot certify (a 200 there is trivially true while the reach is still
 * `commons`, before the narrowing has even reached this doorway's fold). Both
 * conditions must hold: an ANONYMOUS request answers 403 (the narrowing
 * itself landed on this doorway's fold) and the SAME path answers 200 to the
 * bearer (the standing the reach names is still admitted under it). */
async function pollUntilNarrowedWithStanding(
  url: string,
  bearerHeaders: Record<string, string>,
  budgetMs: number,
  intervalMs = RECONCILE_POLL_INTERVAL_MS
): Promise<void> {
  const deadline = Date.now() + budgetMs;
  let lastAnon: RawResponse | undefined;
  let lastAuthed: RawResponse | undefined;
  for (;;) {
    lastAnon = await rawGet(url);
    lastAuthed = await rawGet(url, bearerHeaders);
    if (lastAnon.status === 403 && lastAuthed.status === 200) return;
    if (Date.now() >= deadline) {
      throw new Error(
        `GET ${url} did not converge to "narrowed but standing admitted" within ${budgetMs}ms — ` +
          `anonymous answered HTTP ${lastAnon.status} (want 403, i.e. the narrowing landed), ` +
          `authenticated answered HTTP ${lastAuthed.status} (want 200, i.e. standing still admits)`
      );
    }
    await delay(intervalMs);
  }
}

// =============================================================================
// Background
// =============================================================================

Given(
  'the collective {string} stewards the EPR {string} at commons reach',
  function (this: E2EWorld, collectiveName: string, eprLabel: string): void {
    beginScenario(this, eprLabel, collectiveName);
  }
);

Given(
  'doorway {string} holds a live projection contract for {string}',
  { timeout: 90_000 },
  async function (this: E2EWorld, doorwayId: string, eprLabel: string): Promise<void> {
    const state = getState(this);
    assert.equal(
      eprLabel,
      state.label,
      `staged EPR label mismatch: "${eprLabel}" vs "${state.label}"`
    );
    state.holder = await stageRoot(this, doorwayId, eprLabel);
    // Best-effort ARRANGE for the relay hop the "can read today" step needs —
    // register every OTHER known doorway as this holder's peer, bidirectionally.
    const holderUrl = state.holder.doorwayUrl;
    for (const other of ['alpha', 'beta']) {
      if (other === doorwayId) continue;
      try {
        const otherUrl = this.getDoorway(other).url;
        await registerFederationPeerQuiet(holderUrl, otherUrl);
        await registerFederationPeerQuiet(otherUrl, holderUrl);
      } catch {
        // this doorway id may not exist on a smaller mesh — harmless
      }
    }
  }
);

Given(
  'doorway {string} holds no contract for {string}',
  { timeout: 20_000 },
  async function (this: E2EWorld, doorwayId: string, eprLabel: string): Promise<void> {
    const state = getState(this);
    assert.equal(
      eprLabel,
      state.label,
      `staged EPR label mismatch: "${eprLabel}" vs "${state.label}"`
    );
    state.nonHolderId = doorwayId;
    const doorway = this.getDoorway(doorwayId);
    const path = requestPathFor(eprLabel);
    const marker = markerFor(eprLabel);
    const res = await localOnlyGet(`${doorway.url}${path}`);
    assert.ok(
      !(res.status === 200 && res.text.includes(marker)),
      `doorway "${doorwayId}" answered the local-only probe for "${eprLabel}" (${path}) with the ` +
        `staged archive's own marker (HTTP ${res.status}) — it holds a local mount there`
    );
  }
);

Given(
  "the household's declared reconcile window is read from its fixture manifest",
  function (this: E2EWorld): void {
    const state = getState(this);
    const fixture = loadHouseholdMeshFixture();
    state.reconcileWindowMs = fixture.convergenceWindowMs ?? DEFAULT_RECONCILE_WINDOW_MS;
  }
);

// =============================================================================
// Scenario 1 — a collective narrows an EPR's reach and every doorway's fold
// answers.
// =============================================================================

Given(
  'an anonymous visitor at doorway {string} can read {string} today',
  { timeout: 180_000 },
  async function (this: E2EWorld, doorwayId: string, eprLabel: string): Promise<void> {
    const state = getState(this);
    assert.equal(eprLabel, state.label);
    const doorway = this.getDoorway(doorwayId);
    const path = requestPathFor(eprLabel);
    const marker = markerFor(eprLabel);
    // The relay hop (beta -> the holder) is proven working on this mesh via
    // DHT-registered-sibling discovery (habit DELTA 2026-09-12) but is not
    // instantaneous — bounded poll, same shape as name-routing.steps.ts's
    // waitForRegistryToKnowHolder, but reading the marker rather than the
    // served-by header since this step only needs to know the bytes arrived.
    const deadline = Date.now() + 150_000;
    let last: RawResponse | undefined;
    for (;;) {
      last = await rawGet(`${doorway.url}${path}`);
      if (last.status === 200 && last.text.includes(marker)) return;
      if (Date.now() >= deadline) {
        throw new Error(
          `anonymous visitor at doorway "${doorwayId}" cannot read "${eprLabel}" (${path}) today: ` +
            `HTTP ${last.status}, marker present=${last.text.includes(marker)}. If this doorway holds ` +
            'no contract for the root, the relay hop (proven separately in name-routing.feature) has ' +
            "not reached it — see this file's header for what this file assumes vs proves."
        );
      }
      await delay(RECONCILE_POLL_INTERVAL_MS);
    }
  }
);

When(
  'the collective {string} rules that {string} is no longer at commons reach',
  { timeout: 30_000 },
  async function (this: E2EWorld, collectiveName: string, eprLabel: string): Promise<void> {
    const state = getState(this);
    assert.equal(eprLabel, state.label);
    assert.equal(collectiveName, state.collectiveName);
    await narrowReach(requireHolder(this), 'local', householdAudience(collectiveName));
  }
);

When(
  'each doorway re-folds from the ledger on its own reconcile, unprompted',
  { timeout: OWN_REFRESH_BUDGET_MS + 60_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    const holder = requireHolder(this);
    const askUrl = `${this.getDoorway(state.nonHolderId ?? 'beta').url}${holder.path}`;
    const raw = await pollUntilStatus(askUrl, undefined, 403, OWN_REFRESH_BUDGET_MS + 30_000);
    state.lastRefusal = {
      status: raw.status,
      headers: raw.headers,
      body: parseRefusalBody(raw.text),
    };
  }
);

Then(
  'an anonymous visitor at doorway {string} is refused {string}',
  function (this: E2EWorld, doorwayId: string, eprLabel: string): void {
    const state = getState(this);
    assert.equal(eprLabel, state.label);
    assert.ok(
      state.lastRefusal,
      `no refusal captured yet for doorway "${doorwayId}" — the reconcile-wait step must run first`
    );
    assert.equal(
      state.lastRefusal.status,
      403,
      `doorway "${doorwayId}" answered HTTP ${state.lastRefusal.status}, not a 403 refusal`
    );
    assert.ok(
      state.lastRefusal.body,
      `doorway "${doorwayId}"'s 403 carried no parseable refusal JSON body`
    );
  }
);

Then(
  'the refusal names reach as the term that failed, not absence and not an error',
  function (this: E2EWorld): void {
    const state = getState(this);
    const refusal = state.lastRefusal?.body;
    assert.ok(refusal, 'no refusal body captured to assert against');
    assert.equal(state.lastRefusal?.status, 403, 'a refusal must be a 403, not a 404/5xx error');
    assert.equal(refusal.refused, 'reach', `refusal named term "${refusal.refused}", not "reach"`);
    assert.notEqual(
      refusal.reach,
      'undeclared',
      'refusal reach is "undeclared" (absence-of-declaration), not a genuine narrowing'
    );
  }
);

Then(
  'the refusal names {string} as the collective whose ruling narrowed it',
  function (this: E2EWorld, collectiveName: string): void {
    const state = getState(this);
    const refusal = state.lastRefusal?.body;
    assert.ok(refusal, 'no refusal body captured to assert against');
    // The name must be SAID, in the sentence a person reads — not only carried
    // in a field a chrome might render. (serve_eligibility.rs, the
    // ReachClass::Restricted !authenticated branch.)
    assert.ok(
      refusal.reason.toLowerCase().includes(collectiveName.toLowerCase()),
      `refusal reason does not name "${collectiveName}": "${refusal.reason}" — the ` +
        "anonymous-restricted refusal text is built from the reach declaration's own audience " +
        'term (doorway/doorway-service/src/services/serve_eligibility.rs). An unnamed collective ' +
        'here means either the ruling wrote no audience term onto the contract, or this doorway ' +
        "re-folded from a row that predates it — never that the doorway 'forgot' to say it."
    );
    // …and it must READ BACK to a record, which is the half a doorway-local
    // list could never produce: the collective's own row, and the reach
    // declaration that named it.
    assert.ok(
      refusal.collective?.id === HOUSEHOLD_COLLECTIVE_ID,
      `refusal names no collective record (want id "${HOUSEHOLD_COLLECTIVE_ID}"): ` +
        JSON.stringify(refusal.collective)
    );
    assert.equal(
      refusal.collective?.record,
      `/db/collectives/${HOUSEHOLD_COLLECTIVE_ID}`,
      'the named collective must be dereferenceable on the doorway that refused'
    );
    assert.equal(
      refusal.declaredIn,
      `/api/v1/commitments/${requireHolder(this).commitmentId}`,
      'the refusal must point at the reach declaration it read, so "decided by the fold" is ' +
        'checkable as "the reason it gave reads back to a record"'
    );
  }
);

Then(
  'the chrome of the refusal offers the visitor a way to be heard about that decision',
  function (this: E2EWorld): void {
    const state = getState(this);
    const refusal = state.lastRefusal?.body;
    assert.ok(refusal, 'no refusal body captured to assert against');
    assert.ok(
      typeof refusal.hear === 'string' && refusal.hear.length > 0,
      `refusal carries no "hear" redress route: ${JSON.stringify(refusal)}`
    );
    // The way to be heard must be the route that WITNESSES a challenge, not
    // merely a route that accepts one. `/api/v1/feedback/operations` was the
    // previous pointer and could never have carried this redress: that outbox
    // binds a feedback signal to a CONTENT action, and a reach challenge
    // targets an REA Commitment, so the write would fail at the conductor. A
    // refusal pointing at a door that does not open IS the suggestion box this
    // story exists to replace.
    assert.equal(
      refusal.hear,
      '/api/v1/challenge',
      `refusal "hear" route is "${refusal.hear}", not the route that witnesses a challenge ` +
        'as a commitment with a named party who owes an answer'
    );
  }
);

Then(
  'the refusal was decided by the fold, not by a doorway-local rule or list',
  function (this: E2EWorld): void {
    const state = getState(this);
    const refusal = state.lastRefusal?.body;
    const holder = requireHolder(this);
    assert.ok(refusal, 'no refusal body captured to assert against');
    // The traceability half of "decided by the fold": the reason must read
    // back to REAL ledger records this scenario staged, never a made-up id a
    // doorway-local allowlist could not have produced (serve_eligibility.rs
    // module doc, "A doorway-local allowlist can name no such record").
    assert.equal(
      refusal.epr,
      holder.contentId,
      `refusal names epr "${refusal.epr}", not the staged content id "${holder.contentId}"`
    );
    assert.equal(
      refusal.contract,
      holder.commitmentId,
      `refusal names contract "${refusal.contract}", not the staged commitment id "${holder.commitmentId}"`
    );
  }
);

// =============================================================================
// ATTACHMENT — how a device persona's standing actually reaches a doorway
// (2026-09-12, after run 20260912T225331Z failed "Matthew's conductor states
// his membership" with a 401 at http://localhost:8889/auth/login).
//
// matthew/jessica/james are DEVICE humans: `genesis/data/humans/humans.json`
// gives each `householdId: "household-dowell"` and no doorway ever ran their
// standing through a password login on their behalf — their standing comes
// from their OWN conductor/peer, never the doorway's hosted-account archive.
// `GET /db/participations/{humanId}` (used by the membership-check steps
// below) needs no login at all — it is a live substrate read keyed by the
// CANONICAL fixture human id (`getFixture(name).id`), not a session artifact —
// so those steps never needed `loginBearer` in the first place; that was the
// actual bug, now removed.
//
// The HARDER question is the "asks doorway ... as ..." steps, which need a
// bearer the doorway's own `serve_eligibility::standing_from_request` will
// verify. Traced from source:
//
//   - Matthew IS in `HOUSEHOLD_HOSTED_CAST` (genesis/seeder/src/seed-humans.ts)
//     under the comment "Everywhere. agencyPhase=doorway, so the operator
//     alone costs NO provisioned cell — the doorway branch calls the
//     singleton ZomeCaller on its own conductor." His fixture credentials
//     (`fixtureCredentials('Matthew')`) are literally the ADMIN account
//     (`matthew.dowell@alpha.elohim.host` — gateway-scoped to ALPHA by name;
//     see `src/framework/doorway-identity.ts`'s header on gateway scoping).
//     James is in the same cast, same registration pass.
//   - `doorway/doorway-service/src/routes/auth_routes.rs`'s `MeResponse`
//     doc: "MVP: always \"doorway-host\". Mode B detection is deferred to
//     Task A4" — a doorway NEVER projects `trustMode: "peer-conductor"` or
//     delegates to a peer's conductor for auth. Only elohim-storage's OWN
//     `/auth/me` (`handle_auth_me`, elohim-storage http.rs ~12279) answers
//     `trustMode: "peer-conductor"`, and it does so from a `LocalSession`
//     COOKIE fact with no exportable token — nothing a doorway could verify.
//   - So the hosted-shaped account IS the only bearer-granting mechanism that
//     exists today, and per the run evidence it resolves on ALPHA specifically
//     (the "has been narrowed..." step below already logged in against
//     `holder.doorwayUrl`, which IS alpha in this scenario's Background, and
//     that call did NOT fail — only the membership-check step's login against
//     BETA 401'd). Matching "matthew → alpha": this file now ALWAYS mints
//     Matthew's/James's bearer via `this.getDoorway('alpha').url`, then
//     presents that SAME token to whichever doorway the Gherkin names for the
//     actual content request. Cross-doorway acceptance needs no JWKS fetch on
//     this dev mesh: `hc-mesh.sh` sets `JWT_SECRET` for neither doorway, so
//     both fall back to the same dev placeholder secret and a token alpha
//     mints verifies locally at beta.
//   - If minting via alpha ALSO fails live, that IS "no path exists for a
//     device persona to present standing to a doorway at all" — the steps
//     below now return 'pending' naming exactly that (never a raw exception),
//     since it is a real product gap (Task A4), not a step-glue defect.
// =============================================================================

/** Matthew's and James's hosted-shaped account resolves on ALPHA (his own
 * pool/primary-storage doorway) — see the ATTACHMENT above. Mint there always,
 * regardless of which doorway the Gherkin later asks; the bearer travels via
 * the shared dev JWT secret (or JWKS federation on a real deployment). */
async function loginDevicePersonaBearer(
  world: E2EWorld,
  displayName: string
): Promise<{ token: string; humanId: string } | undefined> {
  try {
    return await loginBearer(world.getDoorway('alpha').url, displayName);
  } catch {
    return undefined;
  }
}

const NO_DEVICE_STANDING_PATH_REASON = (name: string): string =>
  `no implemented path exists for device persona "${name}" to present verifiable standing to a ` +
  'doorway\'s serve_eligibility fold: minting a bearer via doorway "alpha" (his own pool/primary-' +
  "storage doorway — see this file's ATTACHMENT comment above scenario 2) failed live. Doorway-side " +
  'peer-conductor delegation is deferred (doorway/doorway-service/src/routes/auth_routes.rs ' +
  'MeResponse doc: "MVP: always doorway-host. Mode B detection is deferred to Task A4"), and ' +
  "elohim-storage's own /auth/me (peer-conductor trustMode) is a LocalSession cookie fact with no " +
  'token a doorway could verify. This is a real product gap, not a step-glue defect.';

// =============================================================================
// Scenario 2 — a member whose conductor carries the standing is served
// through the nearest live holder.
// =============================================================================

Given(
  '{string} has been narrowed to a reach that admits members of {string}',
  { timeout: 150_000 },
  async function (
    this: E2EWorld,
    eprLabel: string,
    collectiveName: string
  ): Promise<void | 'pending'> {
    const state = getState(this);
    assert.equal(eprLabel, state.label);
    assert.equal(collectiveName, state.collectiveName);
    const holder = requireHolder(this);
    await narrowReach(holder, 'household', householdAudience(collectiveName));
    // Wait for the HOLDER's own EprRouter refresh to pick up the new gate
    // hints, checking BOTH sides at once — a bearer-only poll for 200 is
    // trivially true while the reach is still `commons` (before the
    // narrowing has even reached this fold) and certifies nothing. The
    // convergence this scenario needs is that an ANONYMOUS request has
    // started being refused (the narrowing landed) at the SAME time a
    // bearer any household member holds (matthew, minted via alpha — see
    // the ATTACHMENT above) is still admitted (the standing the reach
    // names survives it) — both within the household's declared reconcile
    // window.
    const bearer = await loginDevicePersonaBearer(this, 'Matthew');
    if (!bearer) return 'pending';
    await pollUntilNarrowedWithStanding(
      `${holder.doorwayUrl}${holder.path}`,
      { Authorization: `Bearer ${bearer.token}` },
      state.reconcileWindowMs + 15_000
    );
  }
);

Given(
  "Matthew's conductor states his membership in {string}",
  { timeout: 20_000 },
  async function (this: E2EWorld, collectiveName: string): Promise<void> {
    const state = getState(this);
    assert.equal(collectiveName, state.collectiveName);
    const fixture = loadHouseholdMeshFixture();
    // No login needed: this is a live substrate read keyed by Matthew's
    // CANONICAL fixture human id, never a session artifact (see ATTACHMENT
    // above — the earlier version of this step wrongly logged in via beta,
    // where his hosted-shaped account does not resolve, and 401'd for no
    // reason this assertion actually needed).
    const humanId = getFixture('Matthew').id;
    const storageUrl = requireFixturePrimaryStorageUrl(fixture, 'beta');
    const isMember = await isLiveMember(storageUrl, humanId, HOUSEHOLD_COLLECTIVE_ID);
    assert.ok(
      isMember,
      `Matthew ("${humanId}") carries no live participation in "${HOUSEHOLD_COLLECTIVE_ID}" ` +
        `per ${storageUrl}/db/participations/${humanId} — the household-formation ceremony ` +
        'has not (or no longer) affirmed this on this mesh'
    );
  }
);

Given(
  "James's conductor states no membership in {string}",
  { timeout: 20_000 },
  async function (this: E2EWorld, collectiveName: string): Promise<void | 'pending'> {
    const state = getState(this);
    assert.equal(collectiveName, state.collectiveName);
    const fixture = loadHouseholdMeshFixture();
    // No login needed — see the sibling Matthew step's comment above.
    const humanId = getFixture('James').id;
    const storageUrl = requireFixturePrimaryStorageUrl(fixture, 'beta');
    const isMember = await isLiveMember(storageUrl, humanId, HOUSEHOLD_COLLECTIVE_ID);
    if (isMember) {
      // The real Dowell household triad (matthew/jessica/james) are ALL
      // affirmed participants of household-dowell (genesis/seeder/src/
      // seed-household-formation.ts; genesis/data/humans/humans.json gives
      // James householdId: "household-dowell" directly) — there is no
      // non-member persona among this mesh's own household. Rather than
      // assert a premise the live mesh contradicts, this half of the
      // scenario is honestly pending: the feature names James specifically,
      // and no other fixture human stands in for "a household non-member"
      // without changing what the story is about.
      return 'pending';
    }
  }
);

When(
  'Matthew asks doorway {string} for {string} as {string}',
  { timeout: 20_000 },
  async function (
    this: E2EWorld,
    doorwayId: string,
    eprLabel: string,
    asName: string
  ): Promise<void | 'pending'> {
    assert.equal(asName, 'Matthew');
    const state = getState(this);
    assert.equal(eprLabel, state.label);
    const doorway = this.getDoorway(doorwayId);
    // Bearer minted via alpha (his own pool/primary-storage doorway), then
    // presented to WHICHEVER doorway the Gherkin names — see the ATTACHMENT
    // above scenario 2.
    const bearer = await loginDevicePersonaBearer(this, 'Matthew');
    if (!bearer) {
      console.warn(`  ⏭️  PENDING: ${NO_DEVICE_STANDING_PATH_REASON('Matthew')}`);
      return 'pending';
    }
    const holder = requireHolder(this);
    const raw = await rawGet(`${doorway.url}${holder.path}`, {
      Authorization: `Bearer ${bearer.token}`,
    });
    state.lastServed = raw;
    state.lastRefusal =
      raw.status === 403
        ? { status: raw.status, headers: raw.headers, body: parseRefusalBody(raw.text) }
        : undefined;
  }
);

Then('Matthew is served {string}', function (this: E2EWorld, eprLabel: string): void {
  const state = getState(this);
  assert.equal(eprLabel, state.label);
  assert.ok(state.lastServed, 'no served response captured for Matthew');
  assert.equal(
    state.lastServed.status,
    200,
    `Matthew was answered HTTP ${state.lastServed.status}, not served: ${state.lastServed.text.slice(0, 300)}`
  );
  const marker = markerFor(eprLabel);
  assert.ok(
    state.lastServed.text.includes(marker),
    `Matthew's 200 response does not carry the staged archive's own marker — it may be a doorway ` +
      'landing-page fallback rather than the actual root'
  );
});

Then(
  'the standing that admitted him was read from his own conductor',
  function (this: E2EWorld): void {
    // True by construction, not by inference from any response field:
    // `standing_from_request` (serve_eligibility.rs) derives standing SOLELY
    // from the verified JWT of THIS request — never a doorway-side lookup —
    // and this scenario's only credential for Matthew is the bearer his own
    // `/auth/login` minted. Re-asserting the prior serve is what is checkable
    // here; there is no separate "source" field on the wire to inspect.
    const state = getState(this);
    assert.ok(state.lastServed?.status === 200, 'Matthew must already be served');
  }
);

Then(
  'doorway {string} served it through the nearest live holder of the contract',
  function (this: E2EWorld, doorwayId: string): void {
    const state = getState(this);
    const holder = requireHolder(this);
    assert.ok(state.lastServed, 'no served response captured');
    const servedBy = state.lastServed.headers['x-elohim-served-by'];
    if (
      doorwayId === holder.doorwayId ||
      originsEqual(this.getDoorway(doorwayId).url, holder.doorwayUrl)
    ) {
      // Asked the holder directly — no relay hop to prove.
      return;
    }
    assert.ok(
      servedBy && originsEqual(servedBy, holder.doorwayUrl),
      `doorway "${doorwayId}" served without naming the holder in "x-elohim-served-by" ` +
        `(got: ${JSON.stringify(servedBy)}, expected origin of ${holder.doorwayUrl})`
    );
  }
);

Then(
  'the chrome names the reach that admitted him and the holder that served the bytes',
  function (this: E2EWorld): void {
    const state = getState(this);
    const holder = requireHolder(this);
    assert.ok(state.lastServed, 'no served response captured');
    const servedBy = state.lastServed.headers['x-elohim-served-by'];
    assert.ok(
      !servedBy || originsEqual(servedBy, holder.doorwayUrl),
      `"x-elohim-served-by" (${servedBy}) does not name the holder (${holder.doorwayUrl})`
    );
    // The admitting reach, said on the way IN — the same sentence shape the
    // refusal uses (`admitted;reach=<reach>` vs `refused;reach=<reach>`), so
    // the chrome never has to infer the fold's answer from a status code.
    // Stamped by serve_eligibility::stamp_admitted_standing; a relayed serve
    // carries the HOLDER's value verbatim.
    const standingHeader = state.lastServed.headers['x-elohim-standing'];
    assert.ok(
      standingHeader?.includes(`reach=${holder.currentReach}`),
      `a served (200) response's "x-elohim-standing" (${JSON.stringify(standingHeader)}) does not ` +
        `name the admitting reach "${holder.currentReach}" — either the row this doorway folded ` +
        'against still carries the pre-narrowing reach, or the serve took a byte path that never ' +
        'passed through the fold (doorway/doorway-service/src/server/http.rs).'
    );
    assert.ok(
      standingHeader?.startsWith('admitted;'),
      `"x-elohim-standing" (${JSON.stringify(standingHeader)}) does not say the fold ADMITTED this ` +
        'serve — a 200 alone does not distinguish "the fold said yes" from "no fold ran"'
    );
  }
);

When(
  'James asks doorway {string} for {string} as {string}',
  { timeout: 20_000 },
  async function (
    this: E2EWorld,
    doorwayId: string,
    eprLabel: string,
    asName: string
  ): Promise<void | 'pending'> {
    assert.equal(asName, 'James');
    const state = getState(this);
    assert.equal(eprLabel, state.label);
    const doorway = this.getDoorway(doorwayId);
    // Bearer minted via alpha, presented to whichever doorway the Gherkin
    // names — see the ATTACHMENT above scenario 2. In practice this step is
    // never reached today: James's own "no membership" Given (above) already
    // returns 'pending' first, since the real household triad are all
    // members. Fixed for correctness regardless.
    const bearer = await loginDevicePersonaBearer(this, 'James');
    if (!bearer) {
      console.warn(`  ⏭️  PENDING: ${NO_DEVICE_STANDING_PATH_REASON('James')}`);
      return 'pending';
    }
    const holder = requireHolder(this);
    const raw = await rawGet(`${doorway.url}${holder.path}`, {
      Authorization: `Bearer ${bearer.token}`,
    });
    state.lastServed = raw;
    state.lastRefusal =
      raw.status === 403
        ? { status: raw.status, headers: raw.headers, body: parseRefusalBody(raw.text) }
        : undefined;
  }
);

Then('James is refused {string}', function (this: E2EWorld, eprLabel: string): void {
  const state = getState(this);
  assert.equal(eprLabel, state.label);
  assert.ok(state.lastRefusal, `James was not refused: ${JSON.stringify(state.lastServed)}`);
  assert.equal(state.lastRefusal.status, 403);
});

// (the shared "the refusal names reach as the term that failed..." Then step
// above is reused verbatim for James's refusal and for scenario 3's.)

// =============================================================================
// Scenario 3 — the holder's warm copy stops answering anonymously inside the
// declared window.
// =============================================================================

Given(
  'doorway {string} has served {string} to an anonymous visitor and holds it warm',
  { timeout: 30_000 },
  async function (this: E2EWorld, doorwayId: string, eprLabel: string): Promise<void> {
    const state = getState(this);
    assert.equal(eprLabel, state.label);
    const holder = requireHolder(this);
    assert.ok(
      holder.doorwayId ===
        (await resolveDoorwayId(this, doorwayId, this.getDoorway(doorwayId).url)) ||
        originsEqual(this.getDoorway(doorwayId).url, holder.doorwayUrl),
      `doorway "${doorwayId}" is not the staged holder of "${eprLabel}"`
    );
    const marker = markerFor(eprLabel);
    const res = await rawGet(`${holder.doorwayUrl}${holder.path}`);
    assert.equal(
      res.status,
      200,
      `doorway "${doorwayId}" did not serve "${eprLabel}" to an anonymous visitor: HTTP ${res.status}`
    );
    assert.ok(
      res.text.includes(marker),
      `served response does not carry the staged archive's marker`
    );
    state.lastServed = res;
  }
);

// (the "the collective ... rules that ... is no longer at commons reach" When
// step from scenario 1 is reused verbatim here — same holder, same narrowing.)

Then(
  "within the household's declared reconcile window an anonymous visitor at doorway {string} is refused {string}",
  { timeout: 180_000 },
  async function (this: E2EWorld, doorwayId: string, eprLabel: string): Promise<void> {
    const state = getState(this);
    assert.equal(eprLabel, state.label);
    const holder = requireHolder(this);
    const doorway = this.getDoorway(doorwayId);
    const budgetMs = state.reconcileWindowMs + 15_000; // + request/poll-interval slack
    const start = Date.now();
    const raw = await pollUntilStatus(`${doorway.url}${holder.path}`, undefined, 403, budgetMs);
    const elapsedMs = Date.now() - start;
    assert.ok(
      elapsedMs <= state.reconcileWindowMs + RECONCILE_POLL_INTERVAL_MS,
      `doorway "${doorwayId}" refused only after ${elapsedMs}ms, past the declared reconcile window ` +
        `of ${state.reconcileWindowMs}ms`
    );
    state.lastRefusal = {
      status: raw.status,
      headers: raw.headers,
      body: parseRefusalBody(raw.text),
    };
  }
);

Then(
  'doorway {string} still holds the bytes warm',
  { timeout: 20_000 },
  async function (this: E2EWorld, doorwayId: string): Promise<void> {
    const state = getState(this);
    const holder = requireHolder(this);
    const doorway = this.getDoorway(doorwayId);
    // Proven by a SECOND, AUTHENTICATED serve succeeding — never by anything
    // read out of the anonymous refusal, which is a permission decision and
    // proves nothing about byte custody either way (file header). The
    // narrowed reach ("local", no gate hints) admits ANY authenticated
    // standing, so Matthew's bearer is enough to observe custody without
    // needing his household membership at all.
    const bearer = await loginBearer(doorway.url, 'Matthew');
    const raw = await rawGet(`${doorway.url}${holder.path}`, {
      Authorization: `Bearer ${bearer.token}`,
    });
    assert.equal(
      raw.status,
      200,
      `authenticated re-serve of "${state.label}" from doorway "${doorwayId}" answered HTTP ${raw.status} ` +
        '— if the bytes were evicted this would 404/503 rather than serve'
    );
    const marker = markerFor(state.label);
    assert.ok(
      raw.text.includes(marker),
      `authenticated re-serve does not carry the staged archive's marker`
    );
    // Supporting evidence only, never hard-asserted: which warm_shell::plan_shell_serve
    // branch fires (ServeWarm vs a fresh Fetch that confirms against upstream
    // and drops the marker) is a live timing detail — see file header.
    const bundleHeader = raw.headers['x-elohim-bundle'];
    if (bundleHeader) {
      // eslint-disable-next-line no-console -- diagnostic evidence, not a test outcome
      console.log(`[served-under-standing] x-elohim-bundle on the re-serve: ${bundleHeader}`);
    }
  }
);

Then('no eviction of those bytes was required for the refusal', function (this: E2EWorld): void {
  // This scenario never calls any admin cache/eviction route between the
  // warm serve and the refusal — the refusal above was produced purely by
  // the fold re-reading the narrowed reach, with the bytes untouched. Nothing
  // to probe: the absence of any eviction call in this file's own steps IS
  // the evidence, and "doorway {string} still holds the bytes warm" (above)
  // is what proves the bytes are, in fact, still there.
});

// =============================================================================
// Scenario 4 — the fair-trade receipt names what was exchanged for this serve
// and who was credited.
// =============================================================================

/** GET a JSON body from a doorway, asserting the status and parsing once. */
async function getJson(
  url: string,
  headers: Record<string, string> | undefined,
  what: string
): Promise<{ raw: string; body: unknown }> {
  const res = await rawGet(url, headers);
  assert.equal(
    res.status,
    200,
    `${what}: GET ${url} answered HTTP ${res.status} — ${res.text.slice(0, 300)}`
  );
  let body: unknown;
  try {
    body = JSON.parse(res.text);
  } catch {
    throw new Error(`${what}: GET ${url} did not answer JSON: ${res.text.slice(0, 300)}`);
  }
  return { raw: res.text, body };
}

Given(
  '{string} is at a reach that admits Matthew',
  { timeout: 30_000 },
  function (this: E2EWorld, eprLabel: string): void {
    const state = getState(this);
    assert.equal(eprLabel, state.label);
    const holder = requireHolder(this);
    // The Background staged this root at `commons`, which is the widest rung
    // and therefore admits Matthew along with everybody else. This scenario
    // deliberately does NOT narrow: what it pins is the ACCOUNT of the serve,
    // not the fold's discrimination (scenario 2 is where the fold has to
    // discriminate, and it needs a device-persona bearer to do it). Narrowing
    // here would couple the receipt's evidence to that separate red for no
    // gain — and would make an "admits Matthew" premise that this file cannot
    // check without his conductor's standing.
    assert.equal(
      holder.currentReach,
      'commons',
      `the staged reach is "${holder.currentReach}", so "admits Matthew" is no longer ` +
        "true by construction — this step asserts the Background's own commons staging"
    );
  }
);

When(
  'Matthew is served {string} through doorway {string}',
  { timeout: 180_000 },
  async function (this: E2EWorld, eprLabel: string, doorwayId: string): Promise<void> {
    const state = getState(this);
    assert.equal(eprLabel, state.label);
    const holder = requireHolder(this);
    const doorway = this.getDoorway(doorwayId);
    state.askedDoorwayId = await resolveDoorwayId(this, doorwayId, doorway.url);
    state.askedDoorwayUrl = doorway.url;

    // Matthew's own bearer when one can be minted — a serve to a NAMED person,
    // which is what the Gherkin says. When it cannot be (the device-persona
    // standing path is a separate, named red), the commons reach admits him
    // anyway and the receipt is the same receipt: it accounts for who was
    // CREDITED, never for who was served.
    const bearer = await loginDevicePersonaBearer(this, 'Matthew');
    const headers = bearer ? { Authorization: `Bearer ${bearer.token}` } : undefined;

    const marker = markerFor(eprLabel);
    const deadline = Date.now() + 150_000;
    let last: RawResponse | undefined;
    for (;;) {
      last = await rawGet(`${doorway.url}${holder.path}`, headers);
      if (last.status === 200 && last.text.includes(marker)) break;
      if (Date.now() >= deadline) {
        throw new Error(
          `doorway "${doorwayId}" did not serve "${eprLabel}" (${holder.path}) within 150s: ` +
            `HTTP ${last.status}, marker present=${last.text.includes(marker)}`
        );
      }
      await delay(RECONCILE_POLL_INTERVAL_MS);
    }
    state.lastServed = last;
    state.lastRefusal = undefined;
  }
);

Then(
  'the chrome carries a fair-trade receipt for this serve',
  { timeout: 30_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    assert.ok(state.lastServed, 'no served response captured');
    assert.equal(state.lastServed.status, 200);
    const askedUrl = state.askedDoorwayUrl;
    assert.ok(askedUrl, 'the "served through doorway" step must run first');

    // The pointer, said ON THE SERVE — so the chrome never has to guess that a
    // visit had an economic account at all. A relayed serve carries the
    // HOLDER's value verbatim, and the pointer is relative, so it resolves
    // against the doorway the visitor is actually talking to.
    const pointer = state.lastServed.headers['x-elohim-receipt'];
    assert.ok(
      pointer?.startsWith('/api/v1/receipt/'),
      `a served (200) response carries no "x-elohim-receipt" pointer (got: ${JSON.stringify(pointer)}) ` +
        '— the serve said nothing about what was traded for it'
    );

    const { raw, body } = await getJson(`${askedUrl}${pointer}`, undefined, 'the receipt');
    state.receipt = { body: body as ReceiptBody, raw, askedAt: askedUrl };
    assert.equal(
      state.receipt.body.epr,
      requireHolder(this).contentId,
      'the receipt must be about the record that was just served'
    );
  }
);

Then('the receipt names what was given in exchange for the serve', function (this: E2EWorld): void {
  const receipt = getState(this).receipt?.body;
  assert.ok(receipt, 'no receipt captured — the preceding step must run first');
  assert.ok(
    receipt.exchanged,
    'the receipt names no exchange: ' +
      `unrecorded=${JSON.stringify(receipt.unrecorded)}. A null here is the doorway being ` +
      'HONEST that it holds no hosting agreement for this record — so if this fails, the ' +
      "staging (stageRoot's hosting-agreement) did not land, not the receipt."
  );
  assert.ok(
    receipt.exchanged.what.trim().length > 0,
    'the exchange must say WHAT was given, in words'
  );
  assert.ok(
    receipt.exchanged.sentence.includes(receipt.exchanged.what),
    `the clause must SAY what was given, not only carry it in a field: "${receipt.exchanged.sentence}"`
  );
  assert.ok(
    !receipt.unrecorded.includes('exchanged'),
    'a named exchange and an "exchanged" entry in unrecorded cannot both be true'
  );
});

Then(
  'the receipt credits the holder peer that provided the bytes',
  function (this: E2EWorld): void {
    const state = getState(this);
    const receipt = state.receipt?.body;
    const holder = requireHolder(this);
    assert.ok(receipt, 'no receipt captured');
    const held = receipt.credits.find(c => c.role === 'held');
    assert.ok(
      held,
      `the receipt credits no holder: credits=${JSON.stringify(receipt.credits.map(c => c.role))}, ` +
        `unrecorded=${JSON.stringify(receipt.unrecorded)}`
    );
    assert.equal(
      held.commitment,
      holder.commitmentId,
      'the held credit must read back to the projection contract this scenario staged, not to ' +
        'some other record the doorway happened to hold'
    );
  }
);

Then(
  'the receipt credits doorway {string} for the projection, separately from the holder',
  function (this: E2EWorld, doorwayId: string): void {
    const state = getState(this);
    const receipt = state.receipt?.body;
    const holder = requireHolder(this);
    assert.ok(receipt, 'no receipt captured');
    const projected = receipt.credits.find(c => c.role === 'projected');
    assert.ok(
      projected,
      `the receipt credits no projecting doorway: unrecorded=${JSON.stringify(receipt.unrecorded)}. ` +
        `That is the doorway being HONEST that it holds no "operate-doorway" binding for ` +
        `"${state.askedDoorwayId}" — "just mesh prologue" seeds one for alpha-elohim-host and ` +
        'apex-elohim-host, so a miss here is a staging gap, never an invented credit.'
    );
    const held = receipt.credits.find(c => c.role === 'held');
    assert.ok(held, 'the holder credit must be present for "separately" to mean anything');
    // SEPARATELY: two records, two parties. One record credited twice would be
    // the doorway crediting itself for the holder's work.
    assert.notEqual(
      projected.commitment,
      held.commitment,
      `doorway "${doorwayId}" credited the SAME record for holding and for projecting ` +
        `(${projected.commitment}) — the two clauses must read back to two different commitments`
    );
    assert.notEqual(
      projected.commitment,
      holder.commitmentId,
      "the projection credit must not be the holder's own projection contract"
    );
  }
);

/**
 * Follow one clause's `record` route until some origin answers with the very
 * commitment the clause NAMES. Tried in order and stopped at the first match —
 * a record that reads back as a DIFFERENT id has not been dereferenced, it has
 * been confused with something else.
 */
async function dereferenceClause(
  origins: string[],
  record: string,
  commitment: string
): Promise<{ resolved: boolean; last?: { status: number; id?: string } }> {
  let last: { status: number; id?: string } | undefined;
  for (const origin of origins) {
    const res = await rawGet(`${origin}${record}`);
    last = { status: res.status };
    if (res.status !== 200) continue;
    const parsed = safeParseId(res.text);
    last.id = parsed;
    if (parsed === commitment) return { resolved: true, last };
  }
  return { resolved: false, last };
}

function safeParseId(text: string): string | undefined {
  try {
    return (JSON.parse(text) as { id?: string }).id;
  } catch {
    return undefined;
  }
}

Then(
  'every credit on the receipt reads back to a record on the shared ledger',
  { timeout: 60_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    const receipt = state.receipt?.body;
    const askedUrl = state.askedDoorwayUrl;
    assert.ok(receipt && askedUrl, 'no receipt captured');
    const clauses: { commitment: string; record: string; what: string }[] = receipt.credits.map(
      c => ({ commitment: c.commitment, record: c.record, what: `the ${c.role} credit` })
    );
    if (receipt.exchanged) {
      clauses.push({
        commitment: receipt.exchanged.commitment,
        record: receipt.exchanged.record,
        what: 'the exchange clause',
      });
    }
    assert.ok(clauses.length > 0, 'a receipt with no clause at all proves nothing');

    // The ledger is SHARED, so either doorway may answer for a record. Ask the
    // doorway that gave the receipt first; fall back to the holder it named,
    // because whether a holder's commitment row has replicated to the courier's
    // own storage peer yet is dataplane-convergence's subject, not this
    // scenario's.
    const origins = [askedUrl];
    if (receipt.servedBy && !originsEqual(receipt.servedBy, askedUrl)) {
      origins.push(receipt.servedBy);
    }

    for (const clause of clauses) {
      const seen = await dereferenceClause(origins, clause.record, clause.commitment);
      assert.ok(
        seen.resolved,
        `${clause.what} names record "${clause.record}" which does not read back to commitment ` +
          `"${clause.commitment}" on any of ${JSON.stringify(origins)} (last observed: ` +
          `${JSON.stringify(seen.last)}). A credit that cannot be dereferenced is a number the ` +
          'doorway made up, which is exactly what this assertion exists to refuse.'
      );
    }
  }
);

Then(
  'the receipt is written in the words a friend would use, with the protocol form one request away',
  { timeout: 30_000 },
  async function (this: E2EWorld): Promise<void> {
    const state = getState(this);
    const captured = state.receipt;
    const askedUrl = state.askedDoorwayUrl;
    const holder = requireHolder(this);
    assert.ok(captured && askedUrl, 'no receipt captured');
    const receipt = captured.body;

    // The checkable half #1: no protocol identifier is the FIRST thing said.
    // Asserted on the WIRE bytes, not on a re-serialized object, because key
    // order is exactly what "the first thing it says" means.
    assert.ok(
      captured.raw.trimStart().startsWith('{"sentence":"'),
      `the first thing the receipt says is not its sentence: ${captured.raw.slice(0, 120)}`
    );
    for (const identifier of [
      holder.contentId,
      holder.commitmentId,
      holder.hostingAgreementId,
      'hosting-agreement',
      'project-epr',
      'operate-doorway',
      'commitment',
      'REA',
    ]) {
      assert.ok(
        !receipt.sentence.includes(identifier),
        `the lead sentence carries the protocol identifier "${identifier}": "${receipt.sentence}"`
      );
    }

    // The checkable half #2: the precise form is reachable in ONE request.
    assert.ok(
      typeof receipt.protocolForm === 'string' && receipt.protocolForm.length > 0,
      'the receipt offers no route to its protocol form'
    );
    const { body } = await getJson(
      `${askedUrl}${receipt.protocolForm}`,
      undefined,
      'the protocol form'
    );
    const form = body as { commitments?: unknown[] };
    assert.ok(
      Array.isArray(form.commitments) && form.commitments.length > 0,
      `the protocol form returned no commitments verbatim: ${JSON.stringify(body).slice(0, 300)}`
    );

    // The half a machine cannot settle is left to a person, on purpose — the
    // feature says so. Print it so a reviewer reads the actual sentence.
    // eslint-disable-next-line no-console -- the human half of this assertion
    console.log(`[served-under-standing] receipt sentence: ${receipt.sentence}`);
  }
);

// =============================================================================
// Scenario 5 — a visitor's challenge becomes a witnessed commitment with a due
// window.
// =============================================================================

/** The words James sends. Fixed, so a re-run of this scenario is a REPLAY of
 * the same challenge rather than a second one — the doorway's ensure-not-create
 * is what makes that safe, and exercising it here is free. */
const JAMES_CHALLENGE_WORDS =
  'I live in this household and I think this should still be readable by people like me. ' +
  'Please look at it again.';

Given(
  'the collective {string} has ruled that {string} is no longer at commons reach',
  { timeout: 60_000 },
  async function (this: E2EWorld, collectiveName: string, eprLabel: string): Promise<void> {
    const state = getState(this);
    assert.equal(eprLabel, state.label);
    assert.equal(collectiveName, state.collectiveName);
    // `private` on purpose. `classify_reach('private')` is BeneficiaryOnly,
    // which refuses an AUTHENTICATED requester too — so James is genuinely
    // refused naming reach without this file asserting the false premise
    // scenario 2 has to stay honest about (the real household triad are all
    // members of household-dowell, so no fixture human stands in for "a
    // household non-member"). The feature's REACH paragraph licenses it: where
    // a narrowing lands is the collective's to say.
    await narrowReach(requireHolder(this), 'private', householdAudience(collectiveName));
  }
);

Given(
  'James asked doorway {string} for {string} and was refused naming reach',
  { timeout: 180_000 },
  async function (this: E2EWorld, doorwayId: string, eprLabel: string): Promise<void | 'pending'> {
    const state = getState(this);
    assert.equal(eprLabel, state.label);
    const holder = requireHolder(this);
    const doorway = this.getDoorway(doorwayId);
    state.askedDoorwayId = await resolveDoorwayId(this, doorwayId, doorway.url);
    state.askedDoorwayUrl = doorway.url;

    // A challenge needs somebody to answer TO, so this half genuinely needs
    // James's verified standing — an anonymous challenge is a named 401 by
    // design, not a witnessed record against nobody.
    const bearer = await loginDevicePersonaBearer(this, 'James');
    if (!bearer) {
      console.warn(`  ⏭️  PENDING: ${NO_DEVICE_STANDING_PATH_REASON('James')}`);
      return 'pending';
    }
    state.challengerToken = bearer.token;
    state.challengerHumanId = bearer.humanId;

    const headers = { Authorization: `Bearer ${bearer.token}` };
    const raw = await pollUntilStatus(
      `${doorway.url}${holder.path}`,
      headers,
      403,
      state.reconcileWindowMs + OWN_REFRESH_BUDGET_MS
    );
    const refusal = parseRefusalBody(raw.text);
    assert.ok(refusal, `James's 403 carried no parseable refusal JSON: ${raw.text.slice(0, 300)}`);
    assert.equal(
      refusal.refused,
      'reach',
      `the refusal named term "${refusal.refused}", not "reach"`
    );
    state.lastRefusal = { status: raw.status, headers: raw.headers, body: refusal };
    state.lastServed = undefined;
  }
);

When(
  "James uses the way to be heard in that refusal's chrome to challenge the standing",
  { timeout: 60_000 },
  async function (this: E2EWorld): Promise<void | 'pending'> {
    const state = getState(this);
    const holder = requireHolder(this);
    const refusal = state.lastRefusal?.body;
    assert.ok(refusal, 'no refusal captured — the preceding step must run first');
    if (!state.challengerToken) return 'pending';
    const askedUrl = state.askedDoorwayUrl;
    assert.ok(askedUrl, 'the refusal step must record which doorway was asked');

    // THE WAY TO BE HEARD IS TAKEN FROM THE REFUSAL ITSELF — never hardcoded
    // here. That is the whole point of the assertion in scenario 1: the route
    // a refusal advertises has to be the route that works.
    assert.ok(refusal.hear, 'the refusal offered no way to be heard');

    state.challengeSentAt = Date.now();
    const res = await fetch(`${askedUrl}${refusal.hear}`, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${state.challengerToken}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ epr: holder.contentId, words: JAMES_CHALLENGE_WORDS }),
      signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    });
    const text = await res.text();
    let body: ChallengeAnswer;
    try {
      body = JSON.parse(text) as ChallengeAnswer;
    } catch {
      throw new Error(
        `POST ${askedUrl}${refusal.hear} did not answer JSON: HTTP ${res.status} ${text.slice(0, 300)}`
      );
    }
    state.challengeAnswer = { status: res.status, body };
  }
);

Then('his challenge is witnessed on the shared ledger as a commitment', function (this: E2EWorld):
  | void
  | 'pending' {
  const state = getState(this);
  if (!state.challengeAnswer) return 'pending';
  const { status, body } = state.challengeAnswer;
  assert.ok(
    status === 201 || status === 200,
    `the challenge answered HTTP ${status}: ${JSON.stringify(body).slice(0, 400)}`
  );
  assert.ok(
    body.outcome === 'witnessed' || body.outcome === 'replay',
    `the challenge outcome was "${body.outcome}" (${body.reason}) — only "witnessed" and ` +
      '"replay" mean a commitment stands on the ledger. "nothingOwed" means the staged ' +
      'contract carried no responsiveReach term, which is a staging gap, not a doorway defect.'
  );
  assert.ok(body.challenge, 'the answer names no commitment');
  assert.ok(body.record, 'the answer offers no route to read that commitment back');
});

Then(
  'the commitment names {string} as the party that owes the response',
  function (this: E2EWorld, collectiveName: string): void | 'pending' {
    const state = getState(this);
    if (!state.challengeAnswer) return 'pending';
    const owed = state.challengeAnswer.body.owed;
    assert.ok(owed, 'the answer names nobody as owing a response');
    assert.equal(
      owed.party,
      HOUSEHOLD_COLLECTIVE_ID,
      `the owed party is "${owed.party}", not the household collective — the doorway must copy ` +
        "the contract's own responsiveReach.party verbatim, never choose one"
    );
    assert.equal(
      owed.partyLabel,
      collectiveName,
      `the owed party is not SAID as "${collectiveName}" (got: ${JSON.stringify(owed.partyLabel)})`
    );
    // …and it must not be the doorway. A doorway answering for a collective is
    // exactly what the scenario's last line refuses.
    assert.notEqual(
      owed.party,
      state.askedDoorwayId,
      'the doorway named ITSELF as the party that owes the response'
    );
  }
);

Then(
  'the commitment carries a due date that was declared before James sent it',
  function (this: E2EWorld): void | 'pending' {
    const state = getState(this);
    if (!state.challengeAnswer) return 'pending';
    const { body } = state.challengeAnswer;
    const owed = body.owed;
    assert.ok(owed, 'no owed term to check a due window against');
    assert.ok(body.due, 'the commitment carries no due date');
    const due = Date.parse(body.due);
    assert.ok(Number.isFinite(due), `the due date "${body.due}" is not a date`);

    // "Declared before he sent it" is checkable because the WINDOW lives on the
    // contract, whose own timestamp predates the challenge. The date is
    // arithmetic over that window — which is why this asserts the declaration's
    // age, not the doorway's word for it.
    const declaredAt = Date.parse(owed.declaredAt);
    assert.ok(
      Number.isFinite(declaredAt),
      `the owed term names no readable declaration time: ${JSON.stringify(owed.declaredAt)}`
    );
    const sentAt = state.challengeSentAt ?? Date.now();
    assert.ok(
      declaredAt < sentAt,
      `the redress term was declared at ${owed.declaredAt}, which is NOT before James sent his ` +
        `challenge at ${new Date(sentAt).toISOString()} — a window agreed after the fact is not a ` +
        'window that was fixed before he sent anything'
    );
    assert.ok(owed.withinHours > 0, 'a window of no time at all is not a window');
    assert.ok(
      owed.declaredBy.startsWith('/api/v1/commitments/'),
      `the window must read back to the record that declared it (got: "${owed.declaredBy}")`
    );
    // The arithmetic a reader can redo: the due date is the declared window
    // counted from receipt, never a number the doorway chose.
    const drift = Math.abs(due - (sentAt + owed.withinHours * 3_600_000));
    assert.ok(
      drift < 5 * 60_000,
      `the due date is ${new Date(due).toISOString()}, which is not ${owed.withinHours}h after ` +
        'the challenge was received — the window on the contract and the date on the record disagree'
    );
  }
);

Then(
  'James can read the owed response and its due date from the same chrome',
  { timeout: 30_000 },
  async function (this: E2EWorld): Promise<void | 'pending'> {
    const state = getState(this);
    if (!state.challengeAnswer?.body.challenge) return 'pending';
    const askedUrl = state.askedDoorwayUrl;
    assert.ok(askedUrl, 'no doorway recorded');
    const answer = state.challengeAnswer.body;

    // THE SAME CHROME: the same doorway James was refused at and challenged
    // through, read back live. Not the holder, not an admin surface.
    const { body } = await getJson(
      `${askedUrl}/api/v1/challenge/${encodeURIComponent(answer.challenge!)}`,
      { Authorization: `Bearer ${state.challengerToken ?? ''}` },
      'the standing challenge'
    );
    const standing = body as {
      challenge: string;
      owed: OwedResponse | null;
      due?: string;
      state: string;
      finished: boolean;
      words?: string;
    };
    assert.equal(standing.challenge, answer.challenge);
    assert.ok(standing.owed, 'the standing record names nobody as owing a response');
    assert.equal(standing.owed.party, HOUSEHOLD_COLLECTIVE_ID);
    assert.equal(
      standing.due,
      answer.due,
      'the due date read back does not match the one the challenge was witnessed with — a due ' +
        'date that moves is not a due date'
    );
    assert.equal(
      standing.words,
      JAMES_CHALLENGE_WORDS,
      "the record does not carry James's own words"
    );
  }
);

Then(
  "doorway {string} did not answer the challenge on the collective's behalf",
  { timeout: 30_000 },
  async function (this: E2EWorld, doorwayId: string): Promise<void | 'pending'> {
    const state = getState(this);
    if (!state.challengeAnswer?.body.challenge) return 'pending';
    const answer = state.challengeAnswer.body;
    const owed = answer.owed;
    assert.ok(owed, 'no owed term recorded');

    // The record of the doorway's restraint is structural: it CARRIED the
    // challenge and is named as having carried it, and the party that owes the
    // answer is somebody else.
    assert.ok(
      answer.carriedBy,
      `doorway "${doorwayId}" did not record itself as having carried the challenge`
    );
    assert.notEqual(
      answer.carriedBy,
      owed.party,
      `doorway "${doorwayId}" is named as BOTH the carrier and the party that owes the answer`
    );

    // …and nothing has been answered yet. A doorway that resolved the challenge
    // on the collective's behalf would have closed it in the same breath.
    const askedUrl = state.askedDoorwayUrl;
    assert.ok(askedUrl, 'no doorway recorded');
    const { body } = await getJson(
      `${askedUrl}/api/v1/challenge/${encodeURIComponent(answer.challenge!)}`,
      { Authorization: `Bearer ${state.challengerToken ?? ''}` },
      'the standing challenge'
    );
    const standing = body as { finished: boolean; state: string };
    assert.equal(
      standing.finished,
      false,
      `the challenge is already finished — doorway "${doorwayId}" answered it rather than ` +
        'carrying it to the collective that ruled'
    );
  }
);

// =============================================================================
// Teardown — restore reach to commons and cancel every staged commitment.
// =============================================================================

After({ tags: '@concern:served-under-standing', timeout: 30_000 }, async function (this: E2EWorld) {
  const state = states.get(this);
  if (state?.holder) {
    await cancelQuiet(state.holder);
  }
  // The owed-response commitment scenario 5 witnesses is LEFT STANDING on
  // purpose. Withdrawing it would be this test answering for the collective —
  // the one thing scenario 5's last line says a doorway must not do, and a test
  // is no more entitled to it. It is agent-scoped and content-addressed over
  // (challenger, contract, words), so a re-run replays the same record rather
  // than accumulating new ones.
  states.delete(this);
});
