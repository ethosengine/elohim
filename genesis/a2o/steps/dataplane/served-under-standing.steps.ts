/**
 * Step glue for features/dataplane/served-under-standing.feature, scenarios 1-3
 * (@concern:served-under-standing, @requires:owned-substrate).
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
 * every PATCH; this file always resends `'proposed'`, the state the commitment was
 * created with, so a reach-only narrowing never touches lifecycle. This IS the
 * steward's own re-declaration of the contract's reach standing in for a Mishpat
 * ruling verb until one exists (habit note, WS3 pass three: "the governance lever
 * is a holon's ruling rather than a config flag" — today the closest live lever is
 * this admin PATCH on the commitment the collective's own steward provider holds).
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
 * TWO GAPS THIS FILE MEASURES HONESTLY RED, TRACED FROM SOURCE (never faked
 * green — matches the habit's own "the anonymous-after-narrowing scenario must
 * FAIL on today's binaries, or it is not measuring the gap it was written for"):
 *
 *   1. `serve_eligibility::serve_eligibility`'s ANONYMOUS-restricted refusal
 *      reason ("Its stewards narrowed it to {declared}, which does not admit a
 *      visitor who has shown nothing. If you are one of the people it now names,
 *      sign in and ask again.") never names the collective — only the
 *      AUTHENTICATED-wrong-membership branch says "which admits {audience}". So
 *      "the refusal names {household} as the collective whose ruling narrowed
 *      it" (scenario 1) has no source text to satisfy for an ANONYMOUS refusal
 *      and is measured as a real assertion that fails today, not skipped.
 *   2. `dispatch_to_projected_epr`'s Serve branch (`ServeEligibility::Serve`)
 *      returns `None` and falls through to the ordinary byte path with NO extra
 *      header — `x-elohim-standing` is minted ONLY by `refusal_response`. So
 *      "the chrome names the reach that admitted him" (scenario 2, success case)
 *      has no header to read and is measured as a real assertion that fails
 *      today.
 *
 * Both are the fold's chrome/governance-mark surface, not its reach/standing
 * decision — the habit's own checks list assigns the five chrome affordances to
 * `epr-atom-home.habit.md` "deliberately not re-registered here". This file still
 * asserts them (rather than omitting the clause) because a silently-omitted
 * assertion is a false green; a written, failing assertion is the honest measure
 * this habit's evidence ledger runs on.
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
import { fixtureCredentials } from '../../src/framework/fixtures/humans.js';
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

function buildMetadata(
  mount: string,
  reach: string,
  gateHints: GateHint[]
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
  };
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
  const metadata = buildMetadata(mount, 'commons', []);
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
  };
  await waitForLocalServe(staged, 65_000);
  return staged;
}

/** Re-declare `staged`'s reach (and, optionally, its gate hints) — the
 * steward's own re-declaration of the contract, standing in for a Mishpat
 * ruling verb until one exists (file header). */
async function narrowReach(
  staged: StagedRoot,
  reach: string,
  gateHints: GateHint[] = []
): Promise<void> {
  const metadata = buildMetadata(staged.mount, reach, gateHints);
  const res = await adminCall(
    'PATCH',
    `${staged.doorwayUrl}/api/v1/commitments/${staged.commitmentId}`,
    {
      state: 'proposed',
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
      metadata: buildMetadata(staged.mount, 'commons', []),
    });
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
}

interface ScenarioState {
  label: string;
  collectiveName: string;
  holder?: StagedRoot;
  nonHolderId?: string;
  reconcileWindowMs: number;
  lastRefusal?: { status: number; headers: Record<string, string | undefined>; body?: Refusal };
  lastServed?: { status: number; text: string; headers: Record<string, string | undefined> };
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
    await narrowReach(requireHolder(this), 'local');
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
    // HONEST RED (file header, gap 1): serve_eligibility.rs's anonymous-
    // restricted refusal reason ("Its stewards narrowed it to {declared}...")
    // never names the collective — only the authenticated-wrong-membership
    // branch says "which admits {audience}". This assertion measures that
    // gap rather than skipping it.
    assert.ok(
      refusal.reason.toLowerCase().includes(collectiveName.toLowerCase()),
      `refusal reason does not name "${collectiveName}": "${refusal.reason}" — ` +
        "serve_eligibility.rs's anonymous-restricted refusal text never names the collective " +
        '(doorway/doorway-service/src/services/serve_eligibility.rs, the ReachClass::Restricted ' +
        '!authenticated branch); this is a real, currently-unmet clause of the habit, not a test defect.'
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
    assert.equal(
      refusal.hear,
      '/api/v1/feedback/operations',
      `refusal "hear" route is "${refusal.hear}", not the declared feedback-operations outbox`
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
// Scenario 2 — a member whose conductor carries the standing is served
// through the nearest live holder.
// =============================================================================

Given(
  '{string} has been narrowed to a reach that admits members of {string}',
  { timeout: OWN_REFRESH_BUDGET_MS + 60_000 },
  async function (this: E2EWorld, eprLabel: string, collectiveName: string): Promise<void> {
    const state = getState(this);
    assert.equal(eprLabel, state.label);
    assert.equal(collectiveName, state.collectiveName);
    const holder = requireHolder(this);
    await narrowReach(holder, 'household', [
      {
        eprRef: HOUSEHOLD_COLLECTIVE_ID,
        label: collectiveName,
        relation: 'membershipPrerequisite',
      },
    ]);
    // Wait for the HOLDER's own EprRouter refresh to pick up the new gate
    // hints — probed with a bearer any household member holds (matthew), so
    // the wait genuinely observes the fold admitting standing under the new
    // reach, not merely "the row changed".
    const bearer = await loginBearer(holder.doorwayUrl, 'Matthew');
    await pollUntilStatus(
      `${holder.doorwayUrl}${holder.path}`,
      { Authorization: `Bearer ${bearer.token}` },
      200,
      OWN_REFRESH_BUDGET_MS + 30_000
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
    const matthew = await loginBearer(this.getDoorway('beta').url, 'Matthew');
    const storageUrl = requireFixturePrimaryStorageUrl(fixture, 'beta');
    const isMember = await isLiveMember(storageUrl, matthew.humanId, HOUSEHOLD_COLLECTIVE_ID);
    assert.ok(
      isMember,
      `Matthew ("${matthew.humanId}") carries no live participation in "${HOUSEHOLD_COLLECTIVE_ID}" ` +
        `per ${storageUrl}/db/participations/${matthew.humanId} — the household-formation ceremony ` +
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
    const james = await loginBearer(this.getDoorway('beta').url, 'James');
    const storageUrl = requireFixturePrimaryStorageUrl(fixture, 'beta');
    const isMember = await isLiveMember(storageUrl, james.humanId, HOUSEHOLD_COLLECTIVE_ID);
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
  ): Promise<void> {
    assert.equal(asName, 'Matthew');
    const state = getState(this);
    assert.equal(eprLabel, state.label);
    const doorway = this.getDoorway(doorwayId);
    const bearer = await loginBearer(doorway.url, 'Matthew');
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
    // HONEST RED (file header, gap 2): dispatch_to_projected_epr's Serve
    // branch returns None and falls through to the ordinary byte path with
    // no extra header — x-elohim-standing is minted ONLY by refusal_response.
    // There is today no response signal naming the reach that admitted a
    // successful serve; this assertion measures that absence rather than
    // silently passing on the holder-half alone.
    const standingHeader = state.lastServed.headers['x-elohim-standing'];
    assert.ok(
      standingHeader?.includes(`reach=${holder.currentReach}`),
      'a served (200) response carries no "x-elohim-standing" header naming the admitting reach — ' +
        "doorway/doorway-service/src/server/http.rs::dispatch_to_projected_epr's Serve branch adds " +
        'no such header today (only refusal_response does); the chrome cannot yet show this from the ' +
        'wire, so this is a real gap, not a test defect.'
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
  ): Promise<void> {
    assert.equal(asName, 'James');
    const state = getState(this);
    assert.equal(eprLabel, state.label);
    const doorway = this.getDoorway(doorwayId);
    const bearer = await loginBearer(doorway.url, 'James');
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
// Teardown — restore reach to commons and cancel every staged commitment.
// =============================================================================

After({ tags: '@concern:served-under-standing', timeout: 30_000 }, async function (this: E2EWorld) {
  const state = states.get(this);
  if (state?.holder) {
    await cancelQuiet(state.holder);
  }
  states.delete(this);
});
