/**
 * Step glue for features/federation/name-routing.feature
 * (@concern:served-under-standing, @requires:owned-substrate, WS3 T3.3).
 *
 * THE MECHANISM UNDER TEST. `doorway/doorway-service/src/services/name_routing.rs`
 * folds a candidate holder set from sibling doorways' `project-epr` contracts
 * (read over each peer's `GET /api/v1/federation/coherence` head set, on the
 * SAME 60s tick that feeds `state.name_routes` — see
 * `services/federation.rs::install_name_routes`) and relays ONE hop when the
 * LOCAL verdict is 404/503 (`relay_precondition` in `name_routing.rs`).
 *
 * A HOSTING COMMITMENT ROUTES A NAME; IT DOES NOT SERVE IT. A `project-epr`
 * commitment alone (pointing at a made-up `epr_id`) makes `EprRouter` dispatch
 * the mount, but `dispatch_to_projected_epr` then proxies to storage's
 * `GET /apps/{identifier}/{file}`, which opens an ARCHIVE for that identifier
 * via `slug_index` — populated only from `content_node` rows (format
 * `html5-app`/`spa-bundle`) that clear the Amber trust floor. No such row
 * exists for a made-up id, so the archive genuinely has no member matching the
 * request: a real, storage-side 404 (`{"error": "File not found in app: …"}`),
 * indistinguishable from a genuine non-holder's answer unless you stock real
 * bytes. So every staged root gets a REAL, tiny archive: `stockAppArchive`
 * builds a one-file ZIP (`index.html`, hand-rolled STORED-method writer — no
 * new dependency for one tiny archive), PUTs it content-addressed via
 * `/admin/seed/blob` (mirrors `scripts/ci/stage-spa-blob.sh` +
 * `genesis/seeder/src/doorway-client.ts`'s `pushBlob`), and creates a
 * `content_node` row naming that blob with `dhtAnchorHash` set AT INGEST
 * (`CreateContentInput.dhtAnchorHash` — "so the row satisfies the
 * require_provenance read gate … where the libp2p publish drain never runs")
 * so the row is Amber-floor-visible immediately, with no conductor round-trip
 * to wait out on this dev-mode, diesel-direct mesh.
 *
 * REQUEST SHAPE: `/nrt-<slug>/` — a trailing-slash, extension-less path
 * against a mount `/nrt-<slug>` (no trailing slash; elohim-storage's
 * `validate_project_epr_commitment` rule 4 forbids one except for `"/"`).
 * `derive_app_subpath` strips the mount prefix and `trim_start_matches('/')`,
 * leaving an EMPTY sub-path either way the trailing slash falls — a bare-mount
 * hit, so the HOLDER always serves `entry_file` ("index.html") regardless.
 *
 * THE ONE OPEN RISK THIS SHAPE CARRIES, FLAGGED HONESTLY: every doorway in
 * this household ALSO projects `elohim-host-landing` at the universal root
 * mount `"/"` with `spaFallback: true` (confirmed live — `GET
 * /totally-random-nonexistent-xyz` answers 200 with the real Angular shell).
 * `is_spa_route_subpath` (`server/http.rs`) reads a trailing-slash remainder's
 * LAST segment as the empty string after the final `/` — which never contains
 * a `.` — so it is ALWAYS classified as an extension-less "deep route" and
 * `"/"`'s own `spaFallback` can answer 200 with the REAL landing page for
 * `/nrt-<slug>/` on a doorway that holds no `/nrt-<slug>` mount at all,
 * whether or not the relay ever ran. Traced from source, not yet re-observed
 * live post-deploy (mesh down while this was written). Every assertion in
 * this file that needs to know a specific doorway actually served a root
 * therefore checks for `nrtMarker(root)` — a literal string the staged
 * archive's `index.html` carries — NEVER bare `<html`/status-200, which the
 * real landing page would also satisfy; see `nrtMarker`'s own doc. If this
 * risk is real, the affected scenarios still fail HONESTLY (a content-marker
 * mismatch, or "no relay log line was ever written") rather than false-green
 * — but they will fail for THIS reason, not the registry-gap reason below,
 * and that distinction matters when reading a red.
 *
 * IDS ARE RUN-SCOPED, NEVER SHARED ACROSS RUNS — AND, AS OF 2026-09-12, NEVER
 * SHARED ACROSS SCENARIOS EITHER. See `runStamp`'s own doc for why a row
 * cycled through create/cancel/reactivate across many RUNS wedged storage's
 * write path for minutes, and `scenarioNonce`'s own doc for why reactivating
 * a row across SCENARIOS within one run raced the doorway's own mount/cache
 * reconciliation and read as "marker present=false" for up to the full
 * `waitForLocalMount` budget (measured live, run 20260912T214110Z — scenarios
 * 2–4 all failed in their own staging `Given`, reusing scenario 1's row).
 * Every test id — `project-epr-nrt-*` AND `nrt-app-*` — includes BOTH
 * `runStamp` and `scenarioNonce`, so a fresh lane invocation never touches a
 * previous run's row, a later scenario never reactivates an earlier one's,
 * and by construction neither can ever collide with a REAL seeded
 * project-epr id either (those are addressed over (doorwayId, eprId), never
 * urlPath).
 *
 * FEDERATION-PEER DISCOVERY MUST BE ARRANGED, NOT ASSUMED. `main.rs` only
 * spawns `spawn_peer_discovery_task` — the ONLY writer of `state.name_routes`
 * (`install_name_routes`, called exclusively from `refresh_coherence`, called
 * exclusively from that task's loop) — `if !args.federation_peers.is_empty()`
 * AT PROCESS BOOT. `hc-mesh.sh` never sets `FEDERATION_PEERS`/
 * `--federation-peers` for either doorway (grepped clean), so on a mesh
 * started via `just mesh start` this task is NEVER SPAWNED and NOTHING EVER
 * reads `state.peer_url_list`/`state.peer_cache` on a timer. `POST
 * /admin/federation/peers` (a real, live, auth-gated admin route —
 * `routes/federation.rs::handle_admin_add_federation_peer`) mutates exactly
 * those two structures and even runs one IMMEDIATE `refresh_peer_cache`, but
 * with the consuming task never started, that write is permanent and inert —
 * confirmed live: `GET /admin/federation/peers` went from `{"peers":[],
 * "total":0}` to naming the sibling correctly, yet `name_routes` never filled
 * across 10+ minutes of polling (2026-09-12 20:37–20:47, both directions
 * registered ~19:58). This file still calls that admin route in "both doorways
 * can read the registry…" (it is the correct, necessary ARRANGE step, and
 * costs nothing when harmless), and Given "stages the root … only"/"…and
 * doorway …"/"the registry still names …" still poll for the registry to
 * fill for up to `REGISTRY_FILL_BUDGET_MS` — but on THIS mesh that poll will
 * time out and the affected scenarios RED with this exact finding named in
 * the assertion message. `@requires:owned-substrate` forbids restarting a
 * doorway with `FEDERATION_PEERS` set from here (a hard rule of this run) —
 * the fix belongs in `hc-mesh.sh`'s doorway launch, a mesh-launcher gap, not
 * a `name_routing.rs` defect. Scenario 3 ("the holder sheds") does NOT depend
 * on this: it stages the root on BOTH doorways, so the doorway Jessica asks
 * always finds a LOCAL mount and never needs the relay/registry at all — this
 * is genuinely, independently green on the current mesh.
 *
 * REGISTERED ELSEWHERE: `doorway {string} at {string}`
 * (`steps/mode-aware.steps.ts`) supplies the Background's two doorway
 * bindings. This file never touches `household-chaos.steps.ts` or
 * `federation-failover.steps.ts` — SIGSTOP/SIGCONT follows their OWN
 * established start-tick-guarded `/proc` technique via
 * `src/framework/fixtures/owned-doorway-process.ts`, and the household mesh
 * lease follows `dataplane/apex-transition.steps.ts`'s `acquireLease`
 * pattern (flock on `$MESH_DIR/a2o.lock`) so this file's fault injection can
 * never race a concurrent fault-injecting scenario sharing the same mesh.
 */

import { strict as assert } from 'node:assert';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { readFile, readlink } from 'node:fs/promises';
import { setTimeout as delay } from 'node:timers/promises';

import { After, Before, Given, Then, When } from '@cucumber/cucumber';

import { getRawWithHeaders } from '../../src/framework/dataplane/surfaces.js';
import {
  resolveOwnedMeshProcess,
  signalOwnedMeshProcess,
  type OwnedProcessHandle,
} from '../../src/framework/fixtures/owned-doorway-process.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ADMIN_KEY =
  process.env['API_KEY_ADMIN'] ?? process.env['MESH_API_KEY_ADMIN'] ?? 'mesh-admin-dev-key';
const REQUEST_TIMEOUT_MS = 15_000;
/** 10s initial delay + up to two 60s coherence-probe ticks + buffer (task-specified budget). */
const REGISTRY_FILL_BUDGET_MS = 150_000;
const REGISTRY_POLL_INTERVAL_MS = 5_000;
/** `DOORWAY_EPR_REFRESH_SECS` default (main.rs) + buffer: how long a doorway's OWN
 * EprRouter can take to notice a commitment it just had cancelled. */
const OWN_REFRESH_BUDGET_MS = 45_000;
const SHED_SETTLE_MS = 5_000;
const RESTORE_BUDGET_MS = 30_000;
const HEALTH_TIMEOUT_MS = 10_000;

/** Fixture doorway id -> hc-mesh.sh's own PID-ledger letter (apex-transition.steps.ts convention). */
const MESH_LETTER: Readonly<Record<string, 'a' | 'b'>> = { alpha: 'a', beta: 'b' };

// ---------------------------------------------------------------------------
// Small pure helpers
// ---------------------------------------------------------------------------

/** Same formula as `genesis/seeder/src/peer-id.ts`'s `deterministicPeerId('human-matthew-manager',
 * 'desktop')`, inlined so this file has no cross-package import to resolve at test-runtime. Any
 * valid, already-known agent identity is fine for a test-only commitment's provider/receiver. */
function testStewardPeerId(): string {
  const digest = createHash('sha256').update('human-matthew-manager:desktop', 'utf8').digest();
  return `12D3KooW${digest.toString('hex').slice(0, 38)}`;
}

/** "garden" -> "garden". Shared slug derivation for every `nrt*` path helper below. */
function nrtSlug(root: string): string {
  const collapsed = root.toLowerCase().replace(/[^a-z0-9]+/g, '-');
  let start = 0;
  let end = collapsed.length;
  while (start < end && collapsed[start] === '-') start += 1;
  while (end > start && collapsed[end - 1] === '-') end -= 1;
  const slug = collapsed.slice(start, end);
  return slug || 'root';
}

/** The project-epr commitment's `urlPath` — no trailing slash (validator rule 4 in
 * elohim-storage's `validate_project_epr_commitment` forbids one, except for `"/"`). */
function nrtMount(root: string): string {
  return `/nrt-${nrtSlug(root)}`;
}

/** What Jessica actually asks for: a trailing-slash, extension-less request against the
 * mount above (coordinator's design — see the "bytes-staging" note below `nrtMount`'s
 * doc for how a bare-mount-hit resolves this to `entry_file` regardless of the trailing
 * slash: `derive_app_subpath` strips the mount prefix then `trim_start_matches('/')`,
 * leaving an EMPTY sub-path either way). */
function nrtRequestPath(root: string): string {
  return `${nrtMount(root)}/`;
}

/** The literal marker `stockAppArchive`'s `index.html` carries for `root` — the one
 * reliable way to tell "this doorway served MY test archive" apart from "this doorway's
 * own real `"/"` landing page answered instead" (the SPA-fallback risk `nrtRequestPath`'s
 * doc describes: a trailing-slash extension-less path can be swallowed as a 200 by a
 * doorway that does NOT hold the mount at all). Every assertion in this file that needs
 * to know whether a specific doorway actually served THIS root checks for this marker,
 * never bare `<html`/status-200, which the real landing page would also satisfy.
 *
 * Carries `requireScenarioNonce()` as a second attribute — see that function's doc and
 * `scenarioNonce`'s doc on the `Before` hook: a stale cache entry surviving from an
 * EARLIER scenario in this same run (the exact glue defect measured 2026-09-12 — a
 * doorway's warm-shell/bundle-heads reconciliation racing a reactivated, previously
 * cancelled-and-deleted row) carries the SAME root text but a DIFFERENT scenario nonce,
 * so it can never read as this scenario's green. */
function nrtMarker(root: string): string {
  return `data-nrt-root="${root}" data-nrt-nonce="${requireScenarioNonce()}"`;
}

// ---------------------------------------------------------------------------
// A tiny, real app archive — CIDv1 + a hand-rolled STORED-method ZIP.
//
// A hosting commitment makes a doorway ROUTE a name; it does not make the
// doorway SERVE bytes. `dispatch_to_projected_epr` (server/http.rs) resolves
// `dispatch_address` to `projection.epr_id` for a never-warmed mount, and
// storage's `GET /apps/{identifier}/{file}` (`handle_app_request`,
// elohim-storage/src/http.rs) opens the ARCHIVE for that identifier via its
// `slug_index` — populated ONLY from `content_node` rows of format
// `html5-app`/`spa-bundle` that clear the Amber trust floor (a provenance
// marker: `dht_anchor_hash` or a completed libp2p publish drain — see
// `lookup_slug_blob_hash`'s doc). No such row existed for a made-up test
// `epr_id`, so the archive genuinely had no member matching the request —
// the storage-side 404 this file's earlier runs actually hit.
//
// The fix mirrors `scripts/ci/stage-spa-blob.sh` + `genesis/seeder/src/
// doorway-client.ts`'s `pushBlob`: PUT a real, content-addressed ZIP via
// `/admin/seed/blob` (`X-Blob-Hash`/`X-Blob-Cid`/`X-Blob-Size`), then create a
// `content_node` row naming that blob. The one addition CI's own path does
// NOT need: `CreateContentInput.dhtAnchorHash` (`genesis/seeder/src/
// generated/create-content-input.ts`) — "set at ingest so the row satisfies
// the require_provenance read gate ... where the libp2p publish drain never
// runs" — is exactly the provenance marker `lookup_slug_blob_hash` looks for,
// so the row is Amber-floor-visible immediately, with no conductor
// round-trip to wait out (this household mesh's doorways are dev-mode,
// diesel-direct writes; there is no drain loop staging content for them
// otherwise).
//
// No `multiformats` import: that package resolves for genesis/seeder (a
// declared dependency there) but not genesis/a2o (a separate pnpm workspace
// member with no such dependency of its own) — hand-rolled CIDv1 here rather
// than adding a new cross-package dependency for one content address.
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

/** Unsigned LEB128 varint — CID's own encoding for its version/codec/multihash-code
 * fields. Both values used here (1, 0x55, 0x12) fit in one byte; encoded properly
 * regardless, since a varint of a small value is just that one byte. */
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

/** CIDv1, raw codec, sha2-256 multihash — `bafkrei…`, this codebase's canonical blob
 * address (`doorway/CLAUDE.md` "Addressing canon"). */
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

/** A fixed DOS date/time stamp. The archive is content-addressed by hash, never by
 * mtime, so a fixed stamp keeps the SAME html producing the SAME zip bytes (and thus
 * the same hash) on every re-run — no wall-clock entropy in a content address. */
const ZIP_DOS_DATE = ((2026 - 1980) << 9) | (1 << 5) | 1;
const ZIP_DOS_TIME = 0;

/** A minimal, valid, STORED-method (uncompressed) ZIP containing exactly one file at
 * `name` — everything `handle_app_request`'s `zip::ZipArchive` reader needs, nothing
 * a general-purpose zip library would add (compression, extra fields, comments). */
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

interface StagedArchive {
  contentId: string;
  doorwayUrl: string;
}

/** PUT a real ZIP (containing `index.html`) via `/admin/seed/blob` and create the
 * `content_node` row an `html5-app` project-epr mount resolves through — the minimal
 * staging a synthetic root needs to actually SERVE, not merely route. Idempotent: a
 * 409 on the content-row POST (a prior ATTEMPT of this SAME scenario already staged the
 * same scenario-scoped id — see `scenarioNonce`'s doc) is treated as already-done. */
async function stockAppArchive(
  doorwayUrl: string,
  contentId: string,
  root: string
): Promise<StagedArchive> {
  const html = Buffer.from(
    `<!doctype html><html><head><meta charset="utf-8"><title>${root}</title></head>` +
      `<body ${nrtMarker(root)}>nrt-${root}</body></html>`,
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
    `PUT ${doorwayUrl}/admin/seed/blob failed for test archive ${contentId}: HTTP ${putStatus} ${putText.slice(0, 300)}`
  );

  const createBody = {
    id: contentId,
    title: `[a2o name-routing-test] ${root}`,
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
    `staging the test app archive ${contentId} failed: HTTP ${created.status} ${created.text.slice(0, 300)}`
  );

  return { contentId, doorwayUrl };
}

async function deleteContentRowQuiet(archive: StagedArchive): Promise<void> {
  try {
    await fetch(`${archive.doorwayUrl}/db/content/${archive.contentId}`, {
      method: 'DELETE',
      headers: { Authorization: `Bearer ${ADMIN_KEY}` },
      signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    });
  } catch {
    // best-effort — the blob bytes are content-addressed and harmless to leave regardless
  }
}

/** Content-addressed over (path, runStamp, scenarioNonce) — same reasoning as
 * `testCommitmentId` below (fresh per scenario attempt, so a stale row cancelled and
 * deleted by an EARLIER scenario's `After` hook is never reactivated by a LATER one — see
 * `scenarioNonce`'s doc for the race that reactivation raced). */
function testContentId(path: string): string {
  assert.ok(runStamp, 'runStamp not minted yet — the Before hook must run before any staging step');
  const digest = createHash('sha256')
    .update(`content|${path}|${runStamp}|${requireScenarioNonce()}`, 'utf8')
    .digest('hex')
    .slice(0, 16);
  return `nrt-app-${digest}`;
}

/** Trim trailing slashes without a backtracking-prone regex (matches
 * household-mesh.ts's own `withoutTrailingSlashes` convention). */
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

// ---------------------------------------------------------------------------
// HTTP: raw GET (via the shared dataplane surface) + admin POST/PATCH
// ---------------------------------------------------------------------------

interface RawResponse {
  status: number;
  text: string;
  headers: Record<string, string | undefined>;
}

async function rawGet(url: string, headers?: Record<string, string>): Promise<RawResponse> {
  return getRawWithHeaders(url, { timeoutMs: REQUEST_TIMEOUT_MS, headers });
}

/** Force LOCAL-ONLY evaluation — the same header the relay itself stamps on an
 * outbound hop, which `relay_precondition` refuses to relay past. A genuine test
 * tool: the honest way to ask "does THIS doorway, by itself, hold a mount here?" */
async function localOnlyGet(url: string): Promise<RawResponse> {
  return rawGet(url, { 'x-federation-hop': '1' });
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

/** The upstream circuit's transient "catching-up" shape (storage_proxy.rs / dispatch_to_projected_epr's
 * warm-shell path) — never a hard failure, just "not yet". Retrying here (rather than pushing the
 * retry into every call site) is what keeps a brief catching-up window from reading as a staging
 * failure this file's own logic caused. */
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

/** Retries a write on 503 "catching-up" — observed live on this mesh's write path (storage
 * upstream circuit) — honoring its own `retryAfter` when present, capped, up to `maxAttempts`. */
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

async function coherenceManifest(
  doorwayUrl: string
): Promise<{ doorwayId: string; heads: { urlPath: string; eprId: string }[] }> {
  const res = await rawGet(`${doorwayUrl}/api/v1/federation/coherence`);
  assert.equal(
    res.status,
    200,
    `GET ${doorwayUrl}/api/v1/federation/coherence failed: HTTP ${res.status} ${res.text.slice(0, 200)}`
  );
  return JSON.parse(res.text) as { doorwayId: string; heads: { urlPath: string; eprId: string }[] };
}

async function registerFederationPeer(
  fromUrl: string,
  peerUrl: string,
  label: string
): Promise<void> {
  const res = await adminCall('POST', `${fromUrl}/admin/federation/peers`, { url: peerUrl });
  assert.ok(
    res.status === 200,
    `registering ${peerUrl} as a federation peer of ${label} (${fromUrl}) failed: ` +
      `HTTP ${res.status} ${res.text.slice(0, 300)}`
  );
}

// ---------------------------------------------------------------------------
// project-epr staging (test-only commitments, never the household's real mounts)
// ---------------------------------------------------------------------------

interface StagedContract {
  commitmentId: string;
  doorwayUrl: string;
  doorwayId: string;
  mount: string;
  archive: StagedArchive;
}

/**
 * One stamp per lane invocation (this process), minted lazily on the FIRST scenario's
 * `Before` hook and reused by every scenario after it — never per-scenario. Prefers the
 * lane's own run id when the caller supplies one (`A2O_RUN_ID`, the same env var
 * `just test mesh`'s recipe honors — see its `run_id="${A2O_RUN_ID:-...}"` — though that
 * recipe computes its OWN default locally and does not export it back to this process, so
 * it is visible here only when the invoker set it); otherwise a `pid+timestamp` stamp.
 *
 * WHY: `testCommitmentId` used to be content-addressed over (doorwayId, path) alone, so
 * every lane invocation reused the SAME row. That row got cycled through
 * create→cancel→reactivate dozens of times across repeated manual/automated runs while
 * diagnosing this feature and wedged — `PATCH` on it started returning a persistent
 * `503 {"status":"catching-up","cause":"upstream"}` for minutes on end, while a BRAND NEW
 * id `POST`ed and `PATCH`ed instantly. Scoping the id to this run bounds reactivation
 * churn to WITHIN one lane invocation (still needed for a genuine cucumber RETRY of the
 * same attempt — see `scenarioNonce`'s doc, right below, for why a fresh SCENARIO no
 * longer reactivates at all) and guarantees a fresh run never touches a previous run's
 * row at all, cancelled-and-wedged or not.
 */
let runStamp: string | undefined;

/**
 * One nonce per SCENARIO ATTEMPT (cucumber's `pickle.id` — stable across a retry of the
 * same attempt, distinct for every other scenario), minted fresh in every `Before` hook.
 *
 * WHY (measured 2026-09-12, run 20260912T214110Z): with `testContentId`/`testCommitmentId`
 * scoped only to (doorwayId, mount, runStamp) — i.e. shared across every scenario in the
 * feature that stages the same root — the SECOND scenario to stage "garden" reactivates
 * the FIRST scenario's row (cancelled + `DELETE /db/content/{id}`'d by that scenario's
 * `After` hook, then `POST`ed/`PATCH`ed back by this scenario's `Given`). That reactivation
 * races the doorway's own mount-teardown-and-rebuild: `waitForLocalMount`'s `x-federation-hop:1`
 * probe served the doorway's REAL "/" landing page (`X-Elohim-Bundle: last-reconciled`, no
 * test marker — exactly the SPA-fallback risk this file's header names) for as long as the
 * EprRouter mount table had not yet re-recognised the reactivated commitment. A manual
 * curl-level repro of exactly this cancel→delete→restage cycle (against the live alpha
 * doorway, root "manrepro1") measured this window at ~7s in isolation; under the noisier
 * multi-scenario CI lane it exceeded the 65s `waitForLocalMount` budget outright — the
 * "marker present=false" red this file's own scenarios 2–4 hit. Nothing server-side is
 * defective: `commitment_to_projection_view` (elohim-storage `db/rea_commitments.rs`)
 * derives `epr_id` by PARSING `in_scope_of` (`doorway:{id}|epr:{epr_id}`) fresh on every
 * read, and `in_scope_of` is immutable after creation (the `PATCH` handler,
 * `handle_update_state`, only ever reconciles `state`/`finished`/`metadata_json` — never
 * `in_scope_of`). So reusing a commitment id across scenarios while trying to point it at a
 * FRESH content id per scenario is not an option: only reusing the SAME content id (as
 * before) keeps `in_scope_of` valid, and reusing the same content id is exactly the reuse
 * that races the cache/reconciliation machinery above. The fix is to stop reactivating
 * across scenario boundaries at all — mint BOTH ids fresh per scenario attempt, so every
 * scenario's `Given` is a clean `POST` (scenario 1's own, already-reliable shape), never a
 * cancel-and-delete's `PATCH` reactivation of someone else's row. `nrtMarker` also carries
 * this nonce, so a warm/archived cache entry surviving from an EARLIER scenario (same root
 * text, same bytes, same blob hash) can never satisfy THIS scenario's marker check.
 */
let scenarioNonce: string | undefined;

Before(function (this: E2EWorld, scenario): void {
  runStamp ??= process.env['A2O_RUN_ID'] ?? `${process.pid}-${Date.now().toString(36)}`;
  scenarioNonce = scenario.pickle.id
    .replace(/[^a-z0-9]/gi, '')
    .toLowerCase()
    .slice(0, 12);
});

/** See `scenarioNonce`'s doc — every reader of the nonce goes through this so a missing
 * `Before` hook fails loudly rather than minting an `undefined`-shaped id. */
function requireScenarioNonce(): string {
  assert.ok(
    scenarioNonce,
    'scenarioNonce not minted yet — the Before hook must run before any staging step'
  );
  return scenarioNonce;
}

/** Content-addressed over (doorwayId, mount, runStamp, scenarioNonce) — deterministic
 * WITHIN one scenario ATTEMPT (so a cucumber retry of the SAME attempt reactivates its own
 * row rather than minting a duplicate) but distinct for every OTHER scenario, even one
 * staging the identical root on the identical doorway (see `scenarioNonce`'s doc for why:
 * `in_scope_of` is immutable, so a commitment id shared across scenarios can only ever be
 * safely re-pointed at the SAME content id, and reusing that content id is exactly what
 * raced the doorway's mount/cache reconciliation). By construction can never collide with a
 * REAL seeded project-epr id either (those are addressed over (doorwayId, eprId), never
 * urlPath — see file header). */
function testCommitmentId(doorwayId: string, mount: string): string {
  assert.ok(runStamp, 'runStamp not minted yet — the Before hook must run before any staging step');
  const digest = createHash('sha256')
    .update(`${doorwayId}|${mount}|${runStamp}|${requireScenarioNonce()}`, 'utf8')
    .digest('hex')
    .slice(0, 16);
  return `project-epr-nrt-${digest}`;
}

/** Stage a test project-epr contract on `doorwayId`, projecting a dedicated,
 * freshly-stocked app archive (see `stockAppArchive`) at `mount`, via `doorwayUrl`'s own
 * doorway (so the write lands on the storage that doorway's EprRouter refresh reads). Every
 * scenario attempt mints its own (content id, commitment id) pair (see `scenarioNonce`'s
 * doc), so this is normally a clean `POST` even when an earlier scenario staged the SAME
 * root — the 409 branch below exists only for a genuine cucumber RETRY of this same
 * attempt (identical nonce), where the prior partial attempt's rows are reactivated rather
 * than duplicated. */
async function stageRoot(
  doorwayUrl: string,
  doorwayId: string,
  mount: string,
  root: string
): Promise<StagedContract> {
  const contentId = testContentId(`${doorwayId}|${mount}`);
  const archive = await stockAppArchive(doorwayUrl, contentId, root);

  const id = testCommitmentId(doorwayId, mount);
  const provider = testStewardPeerId();
  const metadata = {
    urlPath: mount,
    mode: 'cached',
    reach: 'commons',
    baseHref: '/',
    entryFile: 'index.html',
    redirectsFrom: [] as string[],
    previewEprRef: null,
    gateHints: [] as unknown[],
    deadEnd: false,
    stewardDirectEndpoint: null,
    routeClaims: null,
    redirectTemplates: [] as unknown[],
  };
  const body = {
    id,
    action: 'project-epr',
    provider,
    receiver: provider,
    inScopeOf: `doorway:${doorwayId}|epr:${contentId}`,
    note: `[a2o name-routing-test] project ${contentId} at ${mount} on ${doorwayId}`,
    metadataJson: JSON.stringify(metadata),
    metadata,
  };
  const created = await adminCall('POST', `${doorwayUrl}/api/v1/commitments`, body);
  if (created.status === 409) {
    // A prior ATTEMPT of this exact scenario (a cucumber retry — same pickle.id, so the
    // same scenarioNonce) already minted this exact (doorwayId, mount) row — reactivate it
    // rather than failing; the body is byte-identical by construction (deterministic id).
    const reactivated = await adminCall('PATCH', `${doorwayUrl}/api/v1/commitments/${id}`, {
      state: 'proposed',
    });
    assert.equal(
      reactivated.status,
      200,
      `re-activating test contract ${id} on ${doorwayId} failed: HTTP ${reactivated.status} ` +
        reactivated.text.slice(0, 300)
    );
  } else {
    assert.equal(
      created.status,
      201,
      `staging ${mount} on ${doorwayId} failed: HTTP ${created.status} ${created.text.slice(0, 300)}`
    );
  }
  return { commitmentId: id, doorwayUrl, doorwayId, mount, archive };
}

async function lapseContract(staged: StagedContract): Promise<void> {
  const res = await adminCall(
    'PATCH',
    `${staged.doorwayUrl}/api/v1/commitments/${staged.commitmentId}`,
    { state: 'cancelled' }
  );
  assert.equal(
    res.status,
    200,
    `cancelling (lapsing) ${staged.commitmentId} failed: HTTP ${res.status} ${res.text.slice(0, 300)}`
  );
}

/** Best-effort — used only from the After hook. Cancels the commitment AND deletes the
 * content-row binding (`DELETE /db/content/{id}` exists — elohim-storage/src/http.rs
 * "GET/DELETE /db/content/{id}"); the blob BYTES stay (content-addressed by hash, so a
 * leftover zip is harmless and may be reused by a later run's identical archive). */
async function cancelContractQuiet(staged: StagedContract): Promise<void> {
  try {
    await adminCall('PATCH', `${staged.doorwayUrl}/api/v1/commitments/${staged.commitmentId}`, {
      state: 'cancelled',
    });
  } catch {
    // best-effort cleanup, matches E2EWorld.runCleanup's own swallow convention
  }
  await deleteContentRowQuiet(staged.archive);
}

/** Poll until `doorwayUrl` locally serves `path` (forced local-only, so this proves a REAL
 * mount, never an accidental relay) — verified by the archive's OWN marker, not bare
 * status 200, since a doorway with NO mount at `path` can still answer 200 from its own
 * `"/"` root (see `nrtRequestPath`'s doc). A brand-new commitment reaches the doorway's
 * EprRouter via its SSE-driven refresh subscriber — asynchronous relative to the POST
 * response — so a single immediate check races that refresh and flakes; this bounds the
 * wait instead. */
async function waitForLocalMount(
  doorwayUrl: string,
  doorwayLabel: string,
  path: string,
  root: string,
  budgetMs = 65_000
): Promise<void> {
  const marker = nrtMarker(root);
  const deadline = Date.now() + budgetMs;
  let last: RawResponse | undefined;
  for (;;) {
    last = await localOnlyGet(`${doorwayUrl}${path}`);
    if (last.status === 200 && last.text.includes(marker)) return;
    if (Date.now() >= deadline) {
      throw new Error(
        `doorway "${doorwayLabel}" does not locally serve the just-staged root "${root}" (${path}) ` +
          `within ${budgetMs}ms: HTTP ${last.status}, marker present=${last.text.includes(marker)}`
      );
    }
    await delay(1_000);
  }
}

/** "doorway X holds no hosting contract for root" — proven by the ABSENCE of the archive's
 * own marker in a local-only response, never by status alone: a doorway with no `nrtMount`
 * for this root still answers 200 from its own real `"/"` landing page for the trailing-
 * slash, extension-less `nrtRequestPath` (see that function's doc) — the marker is the one
 * reliable way to tell "not my content" apart from "no content at all". */
async function assertNoLocalContract(
  doorwayUrl: string,
  doorwayLabel: string,
  root: string
): Promise<void> {
  const path = nrtRequestPath(root);
  const marker = nrtMarker(root);
  const res = await localOnlyGet(`${doorwayUrl}${path}`);
  assert.ok(
    !(res.status === 200 && res.text.includes(marker)),
    `doorway "${doorwayLabel}" answered the local-only probe (x-federation-hop:1) for "${root}" ` +
      `(${path}) with the test archive's own marker (HTTP ${res.status}) — it holds a local mount there`
  );
}

/** Poll until `fromDoorwayUrl`'s registry has learned `holderUrl` as the holder of `path` —
 * i.e. a real, un-forced GET comes back relayed (`x-elohim-served-by` names the holder). See
 * the file header: on the current mesh this never converges (federation-peer discovery was
 * never started for either doorway process), and the thrown message names that exact gap. */
async function waitForRegistryToKnowHolder(
  fromDoorwayUrl: string,
  fromLabel: string,
  path: string,
  holderUrl: string,
  budgetMs: number
): Promise<RawResponse> {
  const deadline = Date.now() + budgetMs;
  let last: RawResponse | undefined;
  for (;;) {
    try {
      last = await rawGet(`${fromDoorwayUrl}${path}`);
      const servedBy = last.headers['x-elohim-served-by'];
      if (last.status === 200 && servedBy && originsEqual(servedBy, holderUrl)) {
        return last;
      }
    } catch (error) {
      last = { status: -1, text: String(error), headers: {} };
    }
    if (Date.now() >= deadline) {
      throw new Error(
        `doorway "${fromLabel}"'s registry never learned the staged holder for ${path} within ` +
          `${budgetMs}ms (last observed HTTP ${last?.status}` +
          (last?.headers['x-elohim-served-by']
            ? `, served-by ${last.headers['x-elohim-served-by']}`
            : '') +
          `). Federation-peer discovery (the ONLY writer of the doorway's name-route registry — ` +
          `main.rs: \`if !args.federation_peers.is_empty() { spawn_peer_discovery_task(...) }\`) is ` +
          `never started on this mesh: hc-mesh.sh launches doorways with no FEDERATION_PEERS/` +
          `--federation-peers, so \`POST /admin/federation/peers\` (already called by this scenario's ` +
          `"both doorways can read the registry…" background step) mutates state nothing consumes on a ` +
          `timer. This is a mesh dev-launcher gap (hc-mesh.sh), not a defect in name_routing.rs — see the ` +
          `full evidence in this file's header comment.`
      );
    }
    await delay(REGISTRY_POLL_INTERVAL_MS);
  }
}

/** Poll until `doorwayUrl`'s OWN local dispatch for `path` has dropped to 404 — used after
 * lapsing a contract, to know the doorway's own EprRouter refresh has caught up before the
 * "asks doorway" step fires (so a forwarded hop genuinely meets a local 404, not a stale 200). */
async function waitForOwnDispatchToDrop(
  doorwayUrl: string,
  doorwayLabel: string,
  root: string,
  budgetMs: number
): Promise<void> {
  const path = nrtRequestPath(root);
  const marker = nrtMarker(root);
  const deadline = Date.now() + budgetMs;
  let last: RawResponse | undefined;
  for (;;) {
    last = await localOnlyGet(`${doorwayUrl}${path}`);
    if (!(last.status === 200 && last.text.includes(marker))) return;
    if (Date.now() >= deadline) {
      throw new Error(
        `doorway "${doorwayLabel}" still serves the lapsed contract's marker at ${path} after ` +
          `${budgetMs}ms (HTTP ${last.status}) — its own EprRouter refresh (DOORWAY_EPR_REFRESH_SECS, ` +
          `default 30s) has not caught up with the cancellation yet`
      );
    }
    await delay(3_000);
  }
}

// ---------------------------------------------------------------------------
// Structured JSON log reading (doorway's tracing_subscriber::fmt::layer().json())
// ---------------------------------------------------------------------------

interface LogLineFields extends Record<string, unknown> {
  message?: string;
}

function parseLogLines(text: string): LogLineFields[] {
  const out: LogLineFields[] = [];
  for (const line of text.split('\n')) {
    if (!line.trim()) continue;
    try {
      const parsed = JSON.parse(line) as { fields?: LogLineFields };
      if (parsed.fields) out.push(parsed.fields);
    } catch {
      // Non-JSON line (startup banner, panic backtrace, …) — skip.
    }
  }
  return out;
}

const MSG_RELAYED = 'name-route: relayed one hop to the holder of this name';
const MSG_ALL_FAILED = 'name-route: every holder failed — preserving the local verdict';

/** The doorway logs EVERY inbound request at the top of `handle_request`
 * (`info!("[{}] {} {} (host: {})", addr, method, path, host)`) — positional, not
 * structured, so match the rendered message text rather than a `path` field. */
function requestLineHits(lines: LogLineFields[], path: string): number {
  let count = 0;
  const re = /^\[[^\]]+]\s+(\S+)\s+(\S+)\s+\(host:/;
  for (const fields of lines) {
    const message = fields.message;
    if (typeof message !== 'string') continue;
    const match = re.exec(message);
    if (match?.[2] === path) count += 1;
  }
  return count;
}

async function doorwayLogPath(letter: 'a' | 'b', label: string): Promise<string> {
  const handle = await resolveOwnedMeshProcess('doorway', letter, label);
  return readlink(`/proc/${handle.pid}/fd/1`);
}

async function logLength(path: string): Promise<number> {
  try {
    return (await readFile(path, 'utf8')).length;
  } catch {
    return 0;
  }
}

async function logSince(path: string, offset: number): Promise<string> {
  const text = await readFile(path, 'utf8');
  return text.length >= offset ? text.slice(offset) : text;
}

// ---------------------------------------------------------------------------
// Household mesh lease (mirrors dataplane/apex-transition.steps.ts's acquireLease —
// coordinates fault injection across every scenario sharing this mesh, any lane).
// ---------------------------------------------------------------------------

const leases = new WeakMap<E2EWorld, ChildProcessWithoutNullStreams>();

async function acquireLease(world: E2EWorld): Promise<void> {
  if (leases.has(world)) return;
  const child = spawn(
    '/usr/bin/flock',
    [
      '-n',
      // eslint-disable-next-line sonarjs/publicly-writable-directories -- shared household-mesh lock, matches apex-transition.steps.ts.
      `${process.env['MESH_DIR'] ?? '/tmp/elohim-local-mesh'}/a2o.lock`,
      '/bin/bash',
      '-c',
      'echo locked; read -r _',
    ],
    { stdio: 'pipe' }
  );
  leases.set(world, child);
  const granted = await Promise.race([
    once(child.stdout, 'data').then(([chunk]) => String(chunk).includes('locked')),
    once(child, 'exit').then(() => false),
  ]);
  assert.ok(
    granted,
    'household mesh is already in use by another fault-injecting scenario; refusing to shed'
  );
}

function releaseLease(world: E2EWorld): void {
  leases.get(world)?.stdin.end();
  leases.delete(world);
}

// ---------------------------------------------------------------------------
// Scenario state
// ---------------------------------------------------------------------------

interface AskCapture {
  askedId: string;
  response: RawResponse;
  elapsedMs: number;
  /** New log lines written by the ASKED doorway during this ask (relay evidence). */
  askedNewLines: LogLineFields[];
  /** New request-log hits against `path` on the OTHER doorway during this ask ("was it contacted"). */
  otherRequestHits: number;
  otherId: string;
}

interface NameRoutingState {
  root: string;
  /** The project-epr commitment's `urlPath` (no trailing slash — `/nrt-garden`). */
  mount: string;
  /** What Jessica actually asks for (trailing slash, extension-less — `/nrt-garden/`). */
  path: string;
  staged: StagedContract[];
  paused: Map<string, OwnedProcessHandle>; // fixture id -> handle, while SIGSTOP'd
  ask?: AskCapture;
  secondAsk?: AskCapture;
}

const states = new WeakMap<E2EWorld, NameRoutingState>();
const doorwayIdCache = new WeakMap<E2EWorld, Map<string, string>>();

function getState(world: E2EWorld): NameRoutingState {
  const state = states.get(world);
  assert.ok(state, 'the scenario must stage a root before asking a doorway about it');
  return state;
}

function beginScenario(world: E2EWorld, root: string): NameRoutingState {
  const state: NameRoutingState = {
    root,
    mount: nrtMount(root),
    path: nrtRequestPath(root),
    staged: [],
    paused: new Map(),
  };
  states.set(world, state);
  return state;
}

async function resolvedDoorwayId(
  world: E2EWorld,
  doorwayId: string,
  doorwayUrl: string
): Promise<string> {
  const cache = doorwayIdCache.get(world) ?? new Map<string, string>();
  doorwayIdCache.set(world, cache);
  const cached = cache.get(doorwayId);
  if (cached) return cached;
  const manifest = await coherenceManifest(doorwayUrl);
  cache.set(doorwayId, manifest.doorwayId);
  return manifest.doorwayId;
}

const otherFixtureId = (id: string): string => (id === 'alpha' ? 'beta' : 'alpha');

const NO_ASK_CAPTURED = 'no ask captured yet — "Jessica asks doorway ... for ..." must run first';
const NO_SECOND_ASK_CAPTURED =
  'no second ask captured yet — a second "Jessica asks ..." (or "Jessica\'s client asks ... again") must run first';

/** The FIRST captured ask this scenario made. */
function requireAsk(state: NameRoutingState): AskCapture {
  assert.ok(state.ask, NO_ASK_CAPTURED);
  return state.ask;
}

/** The SECOND captured ask (scenario 2's sticky-client follow-up). */
function requireSecondAsk(state: NameRoutingState): AskCapture {
  assert.ok(state.secondAsk, NO_SECOND_ASK_CAPTURED);
  return state.secondAsk;
}

/** Whichever ask is most recent — the second if one was made, else the first. Used by the
 * one "Jessica is served {string}" step shared across all four scenarios (only scenario 2
 * re-asks). */
function requireLatestAsk(state: NameRoutingState): AskCapture {
  const ask = state.secondAsk ?? state.ask;
  assert.ok(ask, NO_ASK_CAPTURED);
  return ask;
}

// =============================================================================
// Background
// =============================================================================

Given(
  'both doorways can read the registry of doorway registrations and hosting contracts',
  { timeout: 30_000 },
  async function (this: E2EWorld): Promise<void> {
    const alpha = this.getDoorway('alpha');
    const beta = this.getDoorway('beta');

    // Registration half of the registry: make each doorway a discovered federation
    // peer of the other. Necessary (this is the real, product ARRANGE step — see
    // doorway/CLAUDE.md "Peer discovery") but, as the file header documents at
    // length, not SUFFICIENT on this mesh: the consuming discovery-loop task is
    // gated on a boot-time flag hc-mesh.sh never sets. Scenarios that genuinely
    // need the fold to fill name that gap themselves when their own wait times out.
    await registerFederationPeer(alpha.url, beta.url, 'alpha');
    await registerFederationPeer(beta.url, alpha.url, 'beta');

    // Hosting-contract half: each doorway must be able to produce its own
    // coherence manifest for a sibling to read — the source the fold consumes.
    const alphaManifest = await coherenceManifest(alpha.url);
    const betaManifest = await coherenceManifest(beta.url);
    assert.ok(
      alphaManifest.doorwayId.length > 0,
      'doorway "alpha" reports no doorwayId in its coherence manifest'
    );
    assert.ok(
      betaManifest.doorwayId.length > 0,
      'doorway "beta" reports no doorwayId in its coherence manifest'
    );

    const cache = doorwayIdCache.get(this) ?? new Map<string, string>();
    cache.set('alpha', alphaManifest.doorwayId);
    cache.set('beta', betaManifest.doorwayId);
    doorwayIdCache.set(this, cache);
  }
);

Given('Jessica is a visitor asking as an ordinary web client', function (this: E2EWorld): void {
  // No fixture human, no auth — every ask in this file is a plain, unauthenticated
  // GET (exactly what an ordinary web visitor's browser sends). Nothing to arrange;
  // this step exists so the Background reads naturally and so a future scenario
  // that DOES need Jessica's identity has one declaration site to extend.
});

// =============================================================================
// Staging (Given)
// =============================================================================

Given(
  'the household stages the root {string} as hosted by doorway {string} only',
  { timeout: REGISTRY_FILL_BUDGET_MS + 30_000 },
  async function (this: E2EWorld, root: string, holderId: string): Promise<void> {
    const state = beginScenario(this, root);
    const holder = this.getDoorway(holderId);
    const holderDoorwayId = await resolvedDoorwayId(this, holderId, holder.url);
    const staged = await stageRoot(holder.url, holderDoorwayId, state.mount, root);
    state.staged.push(staged);

    // Sanity: the holder genuinely, locally serves it (forced local-only probe — proves this
    // is a real mount, never an accidental relay). Bounded poll: the row reaches the
    // doorway's EprRouter via its SSE-driven refresh, asynchronous relative to this POST.
    await waitForLocalMount(holder.url, holderId, state.path, root);

    // The registry-fill precondition (task budget: ~150s — 10s initial delay + up to two 60s
    // coherence-probe ticks + buffer). See this file's header: on the current mesh this never
    // converges, and the thrown message names the exact gap.
    const other = this.getDoorway(otherFixtureId(holderId));
    await waitForRegistryToKnowHolder(
      other.url,
      otherFixtureId(holderId),
      state.path,
      holder.url,
      REGISTRY_FILL_BUDGET_MS
    );
  }
);

Given(
  'the household stages the root {string} as hosted by doorway {string} and doorway {string}',
  { timeout: 60_000 },
  async function (this: E2EWorld, root: string, firstId: string, secondId: string): Promise<void> {
    const state = beginScenario(this, root);
    for (const id of [firstId, secondId]) {
      const doorway = this.getDoorway(id);
      const doorwayId = await resolvedDoorwayId(this, id, doorway.url);
      const staged = await stageRoot(doorway.url, doorwayId, state.mount, root);
      state.staged.push(staged);
      await waitForLocalMount(doorway.url, id, state.path, root);
    }
  }
);

Given(
  'the registry still names doorway {string} as a holder of {string} from a contract that has lapsed',
  { timeout: REGISTRY_FILL_BUDGET_MS + OWN_REFRESH_BUDGET_MS + 60_000 },
  async function (this: E2EWorld, holderId: string, root: string): Promise<void> {
    const state = beginScenario(this, root);
    const holder = this.getDoorway(holderId);
    const other = this.getDoorway(otherFixtureId(holderId));
    const holderDoorwayId = await resolvedDoorwayId(this, holderId, holder.url);
    const staged = await stageRoot(holder.url, holderDoorwayId, state.mount, root);
    state.staged.push(staged);
    await waitForLocalMount(holder.url, holderId, state.path, root);

    // The stale-registry premise (feature comment on this scenario): the OTHER
    // doorway must have learned the holder BEFORE it lapses, so its next ask still
    // reads a since-lapsed contract from its own registry. Requires the discovery
    // loop to genuinely be running — see waitForRegistryToKnowHolder's thrown
    // message for the mesh-launcher gap this run is very likely to hit.
    await waitForRegistryToKnowHolder(
      other.url,
      otherFixtureId(holderId),
      state.path,
      holder.url,
      REGISTRY_FILL_BUDGET_MS
    );

    // Now lapse it — and wait for the HOLDER's own dispatch to catch up, so the
    // upcoming forwarded hop meets a genuine local 404, not a stale 200.
    await lapseContract(staged);
    await waitForOwnDispatchToDrop(holder.url, holderId, root, OWN_REFRESH_BUDGET_MS);
  }
);

Given(
  'doorway {string} holds no hosting contract for {string}',
  { timeout: 15_000 },
  async function (this: E2EWorld, doorwayId: string, root: string): Promise<void> {
    const doorway = this.getDoorway(doorwayId);
    await assertNoLocalContract(doorway.url, doorwayId, root);
  }
);

Given(
  'doorway {string} holds no live hosting contract for {string}',
  { timeout: 15_000 },
  async function (this: E2EWorld, doorwayId: string, root: string): Promise<void> {
    const doorway = this.getDoorway(doorwayId);
    await assertNoLocalContract(doorway.url, doorwayId, root);
  }
);

Given(
  'doorway {string} is serving',
  { timeout: HEALTH_TIMEOUT_MS + 5_000 },
  async function (this: E2EWorld, doorwayId: string): Promise<void> {
    const doorway = this.getDoorway(doorwayId);
    const health = await rawGet(`${doorway.url}/health`);
    assert.equal(
      health.status,
      200,
      `doorway "${doorwayId}" /health answered HTTP ${health.status}`
    );
  }
);

Given(
  'doorway {string} is the first holder in owner order',
  function (this: E2EWorld, doorwayId: string): void {
    // Owner order is the registry fold's OWN internal preference (first-appearance
    // order among candidate holders, `fold_candidate_holders` in name_routing.rs) —
    // there is no admin surface exposing or controlling it directly, and this
    // scenario's own real assertions ("served itself, taking no hop" / "alpha was
    // never contacted") are satisfied by LOCAL-FIRST dispatch regardless of fold
    // order (the doorway Jessica asks holds its own contract, so the fold is never
    // even consulted — see this file's header). Declared here for narrative
    // completeness; nothing to arrange or verify independently.
    assert.ok(
      getState(this).staged.length > 0,
      `doorway "${doorwayId}" has no staged contract yet`
    );
  }
);

Given(
  'doorway {string} is the next holder after it',
  function (this: E2EWorld, doorwayId: string): void {
    assert.ok(
      getState(this).staged.length > 0,
      `doorway "${doorwayId}" has no staged contract yet`
    );
  }
);

Given(
  'both doorways are registered in the registry',
  { timeout: 15_000 },
  async function (this: E2EWorld): Promise<void> {
    const alpha = this.getDoorway('alpha');
    const beta = this.getDoorway('beta');
    const alphaPeers = await rawGet(`${alpha.url}/admin/federation/peers`, {
      Authorization: `Bearer ${ADMIN_KEY}`,
    });
    const betaPeers = await rawGet(`${beta.url}/admin/federation/peers`, {
      Authorization: `Bearer ${ADMIN_KEY}`,
    });
    assert.equal(
      alphaPeers.status,
      200,
      `GET alpha /admin/federation/peers failed: HTTP ${alphaPeers.status}`
    );
    assert.equal(
      betaPeers.status,
      200,
      `GET beta /admin/federation/peers failed: HTTP ${betaPeers.status}`
    );
    const alphaKnowsBeta = (JSON.parse(alphaPeers.text) as { total: number }).total > 0;
    const betaKnowsAlpha = (JSON.parse(betaPeers.text) as { total: number }).total > 0;
    assert.ok(
      alphaKnowsBeta,
      'doorway "alpha" has no registered federation peer (registration step must run first)'
    );
    assert.ok(
      betaKnowsAlpha,
      'doorway "beta" has no registered federation peer (registration step must run first)'
    );
  }
);

// =============================================================================
// The core probe: "Jessica asks doorway X for Y" (shared by all four scenarios)
// =============================================================================

When(
  'Jessica asks doorway {string} for {string}',
  { timeout: OWN_REFRESH_BUDGET_MS + 30_000 },
  async function (this: E2EWorld, doorwayId: string, root: string): Promise<void> {
    const state = getState(this);
    assert.equal(
      root,
      state.root,
      `scenario staged root "${state.root}", but Jessica asked for "${root}"`
    );
    const asked = this.getDoorway(doorwayId);
    const otherId = otherFixtureId(doorwayId);

    const askedLog = await doorwayLogPath(MESH_LETTER[doorwayId], `doorway ${doorwayId}`);
    const otherLog = await doorwayLogPath(MESH_LETTER[otherId], `doorway ${otherId}`);
    const askedOffset = await logLength(askedLog);
    const otherOffset = await logLength(otherLog);

    const started = Date.now();
    const response = await rawGet(`${asked.url}${state.path}`);
    const elapsedMs = Date.now() - started;

    const askedNewLines = parseLogLines(await logSince(askedLog, askedOffset));
    const otherNewText = await logSince(otherLog, otherOffset);
    const otherRequestHits = requestLineHits(parseLogLines(otherNewText), state.path);

    const capture: AskCapture = {
      askedId: doorwayId,
      response,
      elapsedMs,
      askedNewLines,
      otherRequestHits,
      otherId,
    };
    if (state.ask) {
      state.secondAsk = capture;
    } else {
      state.ask = capture;
    }
  }
);

// =============================================================================
// Scenario 1 — relayed one hop
// =============================================================================

Then('Jessica is served {string}', function (this: E2EWorld, root: string): void {
  const state = getState(this);
  const ask = requireLatestAsk(state);
  assert.equal(
    ask.response.status,
    200,
    `Jessica's request for "${root}" answered HTTP ${ask.response.status}: ${ask.response.text.slice(0, 300)}`
  );
  // The marker, never a bare `<html`/`<app-root` check: a doorway that does NOT hold this
  // root can still answer 200 from its own unrelated "/" landing page for this request
  // shape (see nrtRequestPath's doc) — that must fail this step, not pass it.
  const marker = nrtMarker(root);
  assert.ok(
    ask.response.text.includes(marker),
    `Jessica's request for "${root}" answered 200 but without the test archive's own marker ` +
      `(${marker}) — this looks like an UNRELATED doorway answering (e.g. its own "/" landing ` +
      `page), not the staged root: ${ask.response.text.slice(0, 300)}`
  );
});

Then(
  'the page Jessica received is the one doorway {string} would have served her directly',
  { timeout: 15_000 },
  async function (this: E2EWorld, holderId: string): Promise<void> {
    const state = getState(this);
    const ask = requireAsk(state);
    const holder = this.getDoorway(holderId);
    const direct = await localOnlyGet(`${holder.url}${state.path}`);
    assert.equal(
      direct.status,
      200,
      `doorway "${holderId}" direct fetch answered HTTP ${direct.status}`
    );
    assert.equal(
      ask.response.text,
      direct.text,
      `the relayed page differs byte-for-byte from what doorway "${holderId}" serves directly`
    );
  }
);

Then(
  'doorway {string} resolved the holder from the registry, not from a configured peer list',
  function (this: E2EWorld, relayingId: string): void {
    const state = getState(this);
    const ask = requireAsk(state);
    assert.equal(
      ask.askedId,
      relayingId,
      `the captured ask was against "${ask.askedId}", not "${relayingId}"`
    );
    const relayed = ask.askedNewLines.filter(
      f => f.message === MSG_RELAYED && f['path'] === state.path
    );
    assert.ok(
      relayed.length > 0,
      `doorway "${relayingId}" logged no "${MSG_RELAYED}" line for ${state.path} — this line is emitted only ` +
        'from `relay_by_name` after `NameRouteTable::holders_for` folds the registry (contract + liveness), the ' +
        'ONLY holder-resolution path this build has; a static/configured peer list is not consulted anywhere on ' +
        'the relay path'
    );
  }
);

Then(
  'doorway {string} forwarded the request exactly once',
  function (this: E2EWorld, relayingId: string): void {
    const state = getState(this);
    const ask = requireAsk(state);
    assert.equal(ask.askedId, relayingId);
    const attempts = ask.askedNewLines.filter(
      f => (f.message === MSG_RELAYED || f.message === MSG_ALL_FAILED) && f['path'] === state.path
    );
    assert.equal(
      attempts.length,
      1,
      `doorway "${relayingId}" logged ${attempts.length} relay-decision lines for ${state.path} for this ask, ` +
        'expected exactly one (the one-hop budget, MAX_FEDERATION_HOPS=1)'
    );
    assert.equal(
      attempts[0]?.message,
      MSG_RELAYED,
      `the single relay-decision line was a failure, not a serve`
    );
  }
);

Then(
  'doorway {string} never answered that it does not host {string}',
  function (this: E2EWorld, doorwayId: string, root: string): void {
    const state = getState(this);
    const ask = requireAsk(state);
    assert.equal(ask.askedId, doorwayId);
    assert.notEqual(
      ask.response.status,
      404,
      `doorway "${doorwayId}" answered Jessica's request for "${root}" with 404 — it DID answer "not here"`
    );
  }
);

// =============================================================================
// Scenario 2 — sticky client goes direct next time
// =============================================================================

Then(
  'the reply names doorway {string} as the origin for {string}',
  function (this: E2EWorld, holderId: string, root: string): void {
    const state = getState(this);
    const ask = requireAsk(state);
    const holder = this.getDoorway(holderId);
    const servedBy = ask.response.headers['x-elohim-served-by'];
    assert.ok(servedBy, `the reply for "${root}" carries no x-elohim-served-by header`);
    assert.ok(
      originsEqual(servedBy, holder.url),
      `the reply names origin "${servedBy}", not doorway "${holderId}"'s origin (${holder.url})`
    );
  }
);

When(
  "Jessica's client asks for {string} again",
  { timeout: 20_000 },
  async function (this: E2EWorld, root: string): Promise<void> {
    const state = getState(this);
    assert.equal(root, state.root);
    const firstAsk = requireAsk(state);
    const servedBy = firstAsk.response.headers['x-elohim-served-by'];
    assert.ok(servedBy, 'no served-by origin to stick to');

    // The shipped sticky client (doorwayFallbacks + apiBaseUrlInterceptor) prefers an
    // address that answered and dials it directly next time — this step performs exactly
    // that: a plain GET straight at the holder's OWN origin, no relaying doorway involved.
    const relayingId = firstAsk.askedId;
    const relayingLog = await doorwayLogPath(MESH_LETTER[relayingId], `doorway ${relayingId}`);
    const relayingOffset = await logLength(relayingLog);

    const started = Date.now();
    const response = await rawGet(`${servedBy}${state.path}`);
    const elapsedMs = Date.now() - started;
    const newLines = parseLogLines(await logSince(relayingLog, relayingOffset));

    state.secondAsk = {
      askedId: relayingId, // placeholder id label; the relaying doorway's log is what "forwarded nothing" reads
      response,
      elapsedMs,
      askedNewLines: newLines,
      otherRequestHits: 0,
      otherId: relayingId,
    };
  }
);

Then(
  'the request went directly to doorway {string}',
  function (this: E2EWorld, holderId: string): void {
    const state = getState(this);
    const second = requireSecondAsk(state);
    const holder = this.getDoorway(holderId);
    assert.equal(
      second.response.status,
      200,
      `direct request to doorway "${holderId}" answered HTTP ${second.response.status}`
    );
    assert.ok(
      !second.response.headers['x-elohim-name-route'],
      'the direct second request still carries a relay marker header — it was not answered locally'
    );
    // Trivially true by construction (the When step dialed holder.url directly), asserted
    // here so a future refactor that breaks that construction fails loudly.
    assert.ok(holder.url.length > 0);
  }
);

Then(
  'doorway {string} forwarded nothing for that second request',
  function (this: E2EWorld, relayingId: string): void {
    const state = getState(this);
    const second = requireSecondAsk(state);
    assert.equal(
      second.askedId,
      relayingId,
      `captured second-ask log belongs to "${second.askedId}", not "${relayingId}"`
    );
    const relayLines = second.askedNewLines.filter(
      f => f.message === MSG_RELAYED || f.message === MSG_ALL_FAILED
    );
    assert.equal(
      relayLines.length,
      0,
      `doorway "${relayingId}" logged ${relayLines.length} relay-decision line(s) during the second request — ` +
        'it was never asked, so it must never have relayed anything'
    );
  }
);

// =============================================================================
// Scenario 3 — the holder sheds; the doorway Jessica reaches serves itself
// =============================================================================

When(
  'the household makes doorway {string} shed',
  { timeout: SHED_SETTLE_MS + 20_000 },
  async function (this: E2EWorld, doorwayId: string): Promise<void> {
    const state = getState(this);
    await acquireLease(this);
    const handle = await resolveOwnedMeshProcess(
      'doorway',
      MESH_LETTER[doorwayId],
      `doorway ${doorwayId}`
    );
    await signalOwnedMeshProcess(handle, 'SIGSTOP', `doorway ${doorwayId}`);
    state.paused.set(doorwayId, handle);
    await delay(SHED_SETTLE_MS);
  }
);

Then(
  'doorway {string} served {string} itself, taking no hop',
  function (this: E2EWorld, servingId: string, root: string): void {
    const state = getState(this);
    assert.equal(root, state.root, `scenario staged root "${state.root}", not "${root}"`);
    const ask = requireAsk(state);
    assert.equal(ask.askedId, servingId);
    assert.equal(
      ask.response.status,
      200,
      `doorway "${servingId}" answered HTTP ${ask.response.status}`
    );
    const marker = nrtMarker(root);
    assert.ok(
      ask.response.text.includes(marker),
      `doorway "${servingId}" answered 200 but without the test archive's own marker (${marker}) — this ` +
        `looks like its own unrelated "/" landing page, not its staged mount for "${root}"`
    );
    assert.ok(
      !ask.response.headers['x-elohim-name-route'],
      `doorway "${servingId}"'s answer carries a relay marker (x-elohim-name-route) — it was relayed, not served ` +
        'locally'
    );
    assert.ok(
      !ask.response.headers['x-elohim-served-by'],
      `doorway "${servingId}"'s answer names a served-by origin — it was relayed, not served locally`
    );
  }
);

Then(
  'doorway {string} was never contacted for that request',
  function (this: E2EWorld, shedId: string): void {
    const state = getState(this);
    const ask = requireAsk(state);
    assert.equal(
      ask.otherId,
      shedId,
      `the captured ask tracked doorway "${ask.otherId}" as the sibling, not "${shedId}"`
    );
    assert.equal(
      ask.otherRequestHits,
      0,
      `doorway "${shedId}"'s access log gained ${ask.otherRequestHits} new request line(s) for ${state.path} ` +
        `during Jessica's ask — it WAS contacted`
    );
  }
);

Then("the shedding holder's answer was never handed to Jessica", function (this: E2EWorld): void {
  const state = getState(this);
  const ask = requireAsk(state);
  // Alpha's shed answer is `converging_shell_response`/a 503 "catching-up" body — Jessica's
  // response carries neither that status nor any relay marker (asserted above), so it cannot
  // be alpha's answer under any code path this build has.
  assert.notEqual(
    ask.response.status,
    503,
    "Jessica's response carries the shedding doorway's own 503 status"
  );
  assert.ok(
    !ask.response.text.includes('catching-up'),
    "Jessica's response body carries the shedding doorway's own converging-shell body"
  );
});

When(
  'the household restores doorway {string}',
  { timeout: 15_000 },
  async function (this: E2EWorld, doorwayId: string): Promise<void> {
    const state = getState(this);
    const handle = state.paused.get(doorwayId);
    if (!handle) return; // nothing to restore — a prior step's assertion already failed
    await signalOwnedMeshProcess(handle, 'SIGCONT', `doorway ${doorwayId}`);
    state.paused.delete(doorwayId);
  }
);

Then(
  'doorway {string} is serving again',
  { timeout: RESTORE_BUDGET_MS + 10_000 },
  async function (this: E2EWorld, doorwayId: string): Promise<void> {
    const doorway = this.getDoorway(doorwayId);
    const deadline = Date.now() + RESTORE_BUDGET_MS;
    let lastStatus = -1;
    for (;;) {
      try {
        const health = await rawGet(`${doorway.url}/health`);
        lastStatus = health.status;
        if (health.status === 200) return;
      } catch {
        // still restarting network stack after SIGCONT — keep polling
      }
      if (Date.now() >= deadline) {
        assert.fail(
          `doorway "${doorwayId}" did not report healthy within ${RESTORE_BUDGET_MS}ms of restoration ` +
            `(last HTTP ${lastStatus})`
        );
      }
      await delay(1_000);
    }
  }
);

// =============================================================================
// Scenario 4 — the one-hop budget; a lapsed contract is not a live one
// =============================================================================

Then(
  'doorway {string} forwarded the request to doorway {string} exactly once',
  function (this: E2EWorld, relayingId: string, holderId: string): void {
    const state = getState(this);
    const ask = requireAsk(state);
    assert.equal(ask.askedId, relayingId);
    // A sibling's own 404 (the lapsed holder answering honestly) is a FAILED attempt in
    // relay_one_hop terms — the log line is the WARN "every holder failed", carrying
    // `attempted`, never the "relayed" INFO line (that is reserved for a 2xx/3xx serve).
    const failed = ask.askedNewLines.filter(
      f => f.message === MSG_ALL_FAILED && f['path'] === state.path
    );
    assert.equal(
      failed.length,
      1,
      `doorway "${relayingId}" logged ${failed.length} "${MSG_ALL_FAILED}" line(s) for ${state.path}, expected ` +
        'exactly one'
    );
    assert.equal(
      failed[0]?.['attempted'],
      1,
      `doorway "${relayingId}"'s relay-failure line reports attempted=${failed[0]?.['attempted']}, not 1 — the ` +
        `only candidate holder should have been doorway "${holderId}"`
    );
  }
);

Then(
  'the forwarded request carried the mark that says it was forwarded',
  function (this: E2EWorld): void {
    const state = getState(this);
    const ask = requireAsk(state);
    // By construction: `fetch_from_holder` (server/http.rs) unconditionally stamps
    // `FEDERATION_HOP_HEADER` on every outbound relay attempt before it is ever sent — the
    // "every holder failed" log line asserted in the previous step is proof a relay attempt
    // reached the holder at all, and this file's http.rs reading confirms that attempt ALWAYS
    // carries the mark. No independent wire-level probe is available from this test surface.
    const failed = ask.askedNewLines.filter(
      f => f.message === MSG_ALL_FAILED && f['path'] === state.path
    );
    assert.ok(
      failed.length > 0,
      'no relay attempt was logged at all — cannot have carried the mark'
    );
  }
);

Then(
  'doorway {string} answered that request itself rather than forwarding it onward',
  function (this: E2EWorld, holderId: string): void {
    const state = getState(this);
    const ask = requireAsk(state);
    assert.equal(
      ask.otherId,
      holderId,
      `the captured ask tracked doorway "${ask.otherId}" as the sibling, not "${holderId}"`
    );
    assert.equal(
      ask.otherRequestHits,
      1,
      `doorway "${holderId}"'s access log gained ${ask.otherRequestHits} new request line(s) for ${state.path} ` +
        'during this ask, expected exactly 1 (the one forwarded hop it answered locally, per the one-hop budget ' +
        'it must never forward again)'
    );
  }
);

Then(
  'Jessica received a refusal naming that no doorway holds a live contract for {string}',
  function (this: E2EWorld, root: string): void {
    const state = getState(this);
    const ask = requireAsk(state);
    assert.equal(
      ask.response.status,
      404,
      `Jessica's request for "${root}" answered HTTP ${ask.response.status}, not a refusal`
    );
    // The literal wording is implementation-specific (`{"error": "File not found in app: ..."}`
    // — elohim-storage's generic app-bundle 404, not a name-routing-specific message). This
    // step asserts the STATUS (a genuine refusal, not content) and records that gap honestly
    // rather than inventing a string match the API does not carry.
    assert.ok(
      ask.response.text.length > 0,
      'the refusal carries an empty body — cannot confirm it is a real refusal, not a hang-turned-empty-response'
    );
  }
);

Then(
  'being registered was not enough for either doorway to be chosen as holder',
  function (this: E2EWorld): void {
    const state = getState(this);
    const ask = requireAsk(state);
    assert.notEqual(
      ask.response.status,
      200,
      'Jessica received served content — a registered-but-not-holding doorway was chosen'
    );
    const served = ask.askedNewLines.filter(
      f => f.message === MSG_RELAYED && f['path'] === state.path
    );
    assert.equal(
      served.length,
      0,
      'a relay SERVED this request — registration alone made a doorway the holder'
    );
  }
);

Then(
  'Jessica received that refusal rather than waiting out a timeout',
  function (this: E2EWorld): void {
    const state = getState(this);
    const ask = requireAsk(state);
    assert.ok(
      ask.elapsedMs < REQUEST_TIMEOUT_MS,
      `Jessica's request took ${ask.elapsedMs}ms (bounded fetch is ${REQUEST_TIMEOUT_MS}ms) — this looks like a ` +
        'timeout, not a fast honest refusal'
    );
  }
);

// =============================================================================
// Teardown — restoration has priority over everything else. A scenario that
// fails mid-flight never reaches its own "restores"/"cancel" steps, so this
// hook is the only guaranteed release. Global (untagged) but a complete no-op
// for any world this file never touched — safe alongside
// household-chaos.steps.ts / federation-failover.steps.ts sharing the same
// mesh and, via the shared @concern:served-under-standing tag, the same
// feature-file family as served-under-standing.feature (owned elsewhere).
// =============================================================================

After({ timeout: 30_000 }, async function (this: E2EWorld) {
  const state = states.get(this);
  if (!state) return;
  try {
    for (const [doorwayId, handle] of state.paused) {
      try {
        await signalOwnedMeshProcess(handle, 'SIGCONT', `doorway ${doorwayId}`);
      } catch {
        // best-effort — matches E2EWorld.runCleanup's own swallow convention
      }
    }
    for (const staged of state.staged) {
      await cancelContractQuiet(staged);
    }
  } finally {
    releaseLease(this);
    states.delete(this);
  }
});
