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
 * a `name_routing.rs` defect. Scenario 3 ("the holder is paused") does NOT depend
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
import { householdMeshDir } from '../../src/framework/fixtures/household-mesh.js';
import {
  assertStillOwnedProcess,
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
/** Gap between the first and second confirmation checks in `waitForOwnDispatchToDrop` —
 * a single 404 can be a transient between two independent SSE-triggered re-syncs (storage
 * can emit `projection.revoked` more than once for the same commitment, e.g. a retried
 * PATCH each taking its own conductor round trip), so one dropped answer is never trusted
 * alone; two dropped answers this far apart are. */
const CONFIRM_DROP_GAP_MS = 500;
/** After `waitForOwnDispatchToDrop` confirms the drop, how much longer the stale-registry
 * premise waits before letting the visitor's ask fire — clear of the revoke's own settle
 * window (SSE re-fetch / EprRouter install can still be completing a beat after the local
 * dispatch itself already answers 404 twice). */
const SETTLE_AFTER_DROP_MS = 1_000;
const PAUSE_SETTLE_MS = 5_000;
const RESTORE_BUDGET_MS = 30_000;
const HEALTH_TIMEOUT_MS = 10_000;

/** Scenario 4's own timing contract (see that Given step's doc): once the OTHER doorway's
 * registry has just relearned the holder from a tick, that knowledge is trustworthy only
 * until its NEXT ~60s tick re-syncs and drops the meanwhile-lapsed contract. Everything
 * between "confirm the tick" and "Jessica's ask lands" must fit inside this margin. */
const STALE_REGISTRY_SAFETY_MARGIN_MS = 45_000;
/** One federation-discovery interval (60s, main.rs) plus buffer for the coherence probe
 * itself to complete and be logged. */
const TICK_WAIT_BUDGET_MS = 90_000;
/** "retry the whole premise once" — this scenario's own stated timing contract. */
const STALE_REGISTRY_MAX_ATTEMPTS = 2;
/** Generous per-attempt ceiling: local-mount warm-up + one tick wait + a short registry
 * confirm + the own-dispatch-drop margin + buffer. */
const STALE_REGISTRY_ATTEMPT_BUDGET_MS =
  65_000 + TICK_WAIT_BUDGET_MS + 15_000 + STALE_REGISTRY_SAFETY_MARGIN_MS + 15_000;

/** Scenario 5's own timing contract ("A busy holder is set aside until the time it
 * named has passed"). Named so the doorway's federation heartbeat cadence
 * (`spawn_peer_discovery_task`, TICK_LOG_MESSAGE's own doc — 60s in main.rs) is
 * findable at the call site rather than a bare "120" appearing out of nowhere. */
const REGISTRY_REFRESH_CYCLE_SECS = 60;
/** Strictly greater than one refresh cycle — the scenario's own narrative
 * requirement ("busy for longer than one registry refresh cycle"), so a doorway
 * that only remembered a single reply would still be caught treating the busy
 * holder as available again after its first refresh. */
const BUSY_WINDOW_SECS = REGISTRY_REFRESH_CYCLE_SECS * 2;
/** "doorway X next refreshes its registry" budget: one 60s cadence plus buffer for
 * the tick itself to be logged (mirrors TICK_WAIT_BUDGET_MS's own reasoning, sized
 * a little more generously per this scenario's own stated timing contract). */
const NEXT_REFRESH_TICK_BUDGET_MS = 150_000;
/** Settle margin added on top of the declared busy window before asserting the
 * window has genuinely elapsed — clear of clock/timer-granularity noise, never a
 * substitute for the real elapsed-time assertion that follows it. */
const BUSY_WINDOW_SETTLE_MARGIN_MS = 2_000;

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
 * elohim-storage's `validate_project_epr_commitment` forbids one, except for `"/"`).
 *
 * Scoped by the SCENARIO NONCE, not just `root`: every scenario in this feature stages
 * the SAME literal root ("garden") by narrative design, and `EprRouter`'s table is keyed
 * by `RouteKey{host, path}` — NOT by commitment_id (`doorway/doorway-service/src/
 * projection/epr_router.rs`) — so two DIFFERENT commitments both claiming url_path
 * "/nrt-garden" silently coexist/last-write-win at the SAME route key. Two scenarios (or
 * two attempts of this Given step's own retry — see `STALE_REGISTRY_MAX_ATTEMPTS`'s
 * doc) that both stage "garden" around the same wall-clock moment therefore share a
 * mount, and a still-live OTHER contract at that shared mount can serve Jessica's ask in
 * place of THIS scenario's own (already-cancelled) one — measured live 2026-09-13: a
 * second `project-epr-nrt-*` commitment for "/nrt-garden", created 0.37s after this
 * scenario's own cancel and cancelled 35s later, served the relayed ask instead of this
 * scenario's contract, with no peer-staleness or conductor-write defect involved at all.
 * The nonce makes every scenario's (and every attempt's — the nonce is stable across a
 * retry of ONE scenario, see `scenarioNonce`'s doc) mount byte-for-byte unique, so no two
 * stagings of "garden" can ever share a route key again. */
function nrtMount(root: string): string {
  return `/nrt-${nrtSlug(root)}-${requireScenarioNonce()}`;
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

/** A raw response from the doorway's dev-gated busy/shed-override surface — status,
 * body text and (lower-cased) response headers, since `Retry-After` is read directly. */
interface ShedResponse {
  status: number;
  text: string;
  headers: Record<string, string | undefined>;
}

/**
 * `PUT {doorwayUrl}/admin/dev/shed` — the PROPOSED sibling of the already-live
 * `PUT /admin/dev/portal-health` (`doorway/doorway-service/src/routes/admin_dev.rs`):
 * same dev-mode-gated, doorway-local OPERATIONAL-state shape, same admin-key auth this
 * file's own `adminCall` uses for its writes. NOT YET BUILT as of this authoring —
 * every caller MUST check `shedRouteMissing` on the result before trusting anything
 * else about it (a 404/405 means "route absent on this build", never "the holder
 * genuinely refused").
 *
 * `{"retryAfterSecs": 0}` is this file's own convention for CLEARING a previously-set
 * override (the task's stated alternative, DELETE, is not exercised here — "support
 * PUT-with-0" is sufficient and keeps one call shape for both directions).
 */
async function putShedOverride(doorwayUrl: string, retryAfterSecs: number): Promise<ShedResponse> {
  const response = await fetch(`${doorwayUrl}/admin/dev/shed`, {
    method: 'PUT',
    headers: { Authorization: `Bearer ${ADMIN_KEY}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ retryAfterSecs }),
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
  });
  const text = await response.text();
  const headers: Record<string, string | undefined> = {};
  response.headers.forEach((value, key) => {
    headers[key.toLowerCase()] = value;
  });
  return { status: response.status, text, headers };
}

/** See `putShedOverride`'s doc: the honest "not built yet" reading of its result. */
function shedRouteMissing(res: ShedResponse): boolean {
  return res.status === 404 || res.status === 405;
}

/**
 * Marks the calling step PENDING (yellow — never a false green, never an opaque red)
 * with a message naming exactly which route is missing, mirroring this file's other
 * `this.attach?.(...)` + `return 'pending'` degrades (see `federation-epr.steps.ts` for
 * the same convention). Scenario 5 cannot arrange a real busy holder without this route;
 * every step that depends on it degrades through this one function so the message is
 * worded identically everywhere it appears in a report.
 */
function pendingShedRoute(world: E2EWorld, doorwayId: string, res: ShedResponse): 'pending' {
  world.attach?.(
    `PENDING: PUT ${world.getDoorway(doorwayId).url}/admin/dev/shed answered HTTP ${res.status} — ` +
      'this dev-gated busy/shed-override route is not built yet on this doorway (the proposed ' +
      'sibling of the live PUT /admin/dev/portal-health in ' +
      'doorway/doorway-service/src/routes/admin_dev.rs). Scenario "A busy holder is set aside ' +
      `until the time it named has passed" cannot arrange doorway "${doorwayId}" as a real busy ` +
      'holder without it.'
  );
  return 'pending';
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
  root: string,
  path: string
): Promise<void> {
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
 * "asks doorway" step fires (so a forwarded hop genuinely meets a local 404, not a stale 200).
 *
 * Requires TWO CONSECUTIVE dropped answers, `CONFIRM_DROP_GAP_MS` apart, before returning.
 * A single 404 is not trusted alone: storage can emit `projection.revoked` more than once for
 * the same commitment (e.g. a retried PATCH, each independently taking the conductor-write
 * round trip and its own write-then-emit — see the storage-side ordering investigation this
 * guards against), and the doorway's re-sync in between two such events can transiently
 * observe the row as still-active before the settled state lands. One 404 sandwiched between
 * two re-syncs must never read as "dropped" — only a confirmed, repeated drop does.
 */
async function waitForOwnDispatchToDrop(
  doorwayUrl: string,
  doorwayLabel: string,
  path: string,
  budgetMs: number
): Promise<void> {
  const deadline = Date.now() + budgetMs;
  let last: RawResponse | undefined;
  let consecutiveDrops = 0;
  for (;;) {
    last = await localOnlyGet(`${doorwayUrl}${path}`);
    const dropped = last.status === 404;
    if (dropped) {
      consecutiveDrops += 1;
      if (consecutiveDrops >= 2) return;
      // First drop observed — confirm it holds before trusting it.
      await delay(CONFIRM_DROP_GAP_MS);
      continue;
    }
    consecutiveDrops = 0;
    if (Date.now() >= deadline) {
      throw new Error(
        `doorway "${doorwayLabel}" has not refused the lapsed contract's entry at ${path} after ` +
          `${budgetMs}ms (HTTP ${last.status}) — its own EprRouter refresh (DOORWAY_EPR_REFRESH_SECS, ` +
          `default 30s) has not caught up with the cancellation yet`
      );
    }
    await delay(3_000);
  }
}

/** A single record from `GET /api/v1/commitments?action=project-epr` — the fields this
 * file's own assertion below reads. The endpoint returns a BARE JSON array of these
 * (`ReaCommitmentService::list` → `commitment_view`, elohim-storage), unfiltered by
 * url_path (the generic `ReaCommitmentQuery` has no such filter — see
 * `elohim-storage/src/db/rea_commitments.rs`'s `ReaCommitmentQuery`), so this file filters
 * client-side. */
interface CommitmentListRow {
  id: string;
  metadata?: { urlPath?: string } | null;
}

/** Regardless of how many attempts the premise took, exactly ONE project-epr commitment
 * must exist for `mount` on `doorwayUrl` by the time the premise is done arranging —
 * proof that no attempt (this file's own retry, or anything else) ever minted a SECOND
 * commitment sharing this route key. `nrtMount`'s own doc names the failure this guards:
 * two commitments at the SAME `RouteKey{host, path}` silently coexist in the doorway's
 * `EprRouter` (last-write-wins), so a stray second row is invisible until it happens to
 * win a dispatch race against the one this scenario is actually testing. A large limit
 * (well past this mesh's observed project-epr row count) is used so accumulated debris
 * from earlier runs never causes a false negative by paging past the match. */
async function assertExactlyOneCommitmentForMount(
  doorwayUrl: string,
  doorwayLabel: string,
  mount: string,
  expectedCommitmentId: string
): Promise<void> {
  const res = await rawGet(`${doorwayUrl}/api/v1/commitments?action=project-epr&limit=2000`);
  assert.equal(
    res.status,
    200,
    `listing project-epr commitments on "${doorwayLabel}" to verify mount uniqueness failed: ` +
      `HTTP ${res.status} ${res.text.slice(0, 300)}`
  );
  const rows = JSON.parse(res.text) as CommitmentListRow[];
  const matches = rows.filter(r => r.metadata?.urlPath === mount);
  assert.equal(
    matches.length,
    1,
    `doorway "${doorwayLabel}" has ${matches.length} project-epr commitment(s) at mount ` +
      `"${mount}" (ids: ${matches.map(r => r.id).join(', ') || 'none'}), expected exactly one — ` +
      'a second commitment sharing this route key can win a dispatch race against the one this ' +
      'scenario is testing (EprRouter is keyed by RouteKey{host, path}, not commitment_id)'
  );
  assert.equal(
    matches[0]?.id,
    expectedCommitmentId,
    `the one project-epr commitment at mount "${mount}" on doorway "${doorwayLabel}" is ` +
      `"${matches[0]?.id}", not this scenario's own "${expectedCommitmentId}"`
  );
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

/** The slice of `path` written strictly between two prior `logLength` offsets — the
 * BOUNDED counterpart to `logSince` (which reads to end-of-file). Scenario 5's "nothing
 * asked X while it was set aside" (step 6) needs exactly this bound: reading to
 * end-of-file at assertion time would also pick up the LEGITIMATE ask that follows
 * clearing the override (the busy holder is asked again, on purpose, once it is no
 * longer set aside), which is outside the window this check is about. Clamped so a
 * offset pair captured across a log rotation never throws. */
async function logBetween(path: string, fromOffset: number, toOffset: number): Promise<string> {
  const text = await readFile(path, 'utf8');
  const from = Math.min(fromOffset, text.length);
  const to = Math.min(Math.max(toOffset, from), text.length);
  return text.slice(from, to);
}

// ---------------------------------------------------------------------------
// The federation discovery loop's own tick — the timestamp anchor scenario 4's premise
// needs (see that Given step's own doc for why: there is no admin endpoint exposing "when
// did this doorway's name-route table last refresh", so the log line is the only observable
// proxy).
// ---------------------------------------------------------------------------

/** `spawn_peer_discovery_task`'s own marker (federation.rs `refresh_peer_cache`) — logged
 * once per loop iteration, gated on `peers.len() > 0`. The SAME iteration calls
 * `refresh_coherence` immediately afterward with no intervening sleep (see that function's
 * doc in federation.rs), so this line's timestamp is, for this test surface's purposes, the
 * moment the doorway's name-route table last refreshed too. */
const TICK_LOG_MESSAGE = 'Federation peer cache refreshed';

interface TimestampedLogLine {
  atMs: number;
  fields: LogLineFields;
}

/** Same JSON-line parsing as `parseLogLines`, additionally keeping the envelope's own
 * `timestamp` field (tracing_subscriber's `fmt::layer().json()` default, RFC3339). */
function parseTimestampedLogLines(text: string): TimestampedLogLine[] {
  const out: TimestampedLogLine[] = [];
  for (const line of text.split('\n')) {
    if (!line.trim()) continue;
    try {
      const parsed = JSON.parse(line) as { timestamp?: unknown; fields?: LogLineFields };
      if (!parsed.fields || typeof parsed.timestamp !== 'string') continue;
      const atMs = Date.parse(parsed.timestamp);
      if (!Number.isNaN(atMs)) out.push({ atMs, fields: parsed.fields });
    } catch {
      // Non-JSON line (startup banner, panic backtrace, …) — skip, matches parseLogLines.
    }
  }
  return out;
}

/** Timestamp (ms since epoch) of the most recent tick logged so far in `logPath`, or
 * `undefined` if the discovery loop has not logged one yet (fresh boot, or peers still
 * empty — the line itself is gated on `peers.len() > 0`). */
async function lastTickAt(logPath: string): Promise<number | undefined> {
  const text = await readFile(logPath, 'utf8').catch(() => '');
  let latest: number | undefined;
  for (const { atMs, fields } of parseTimestampedLogLines(text)) {
    if (fields.message === TICK_LOG_MESSAGE && (latest === undefined || atMs > latest)) {
      latest = atMs;
    }
  }
  return latest;
}

/** Poll `logPath` until a tick strictly after `afterMs` appears (the FIRST-ever tick counts
 * when `afterMs` is `undefined`), returning its timestamp — the moment this doorway's
 * registry last relearned who holds what. */
async function waitForNextTick(
  logPath: string,
  afterMs: number | undefined,
  budgetMs: number
): Promise<number> {
  const deadline = Date.now() + budgetMs;
  for (;;) {
    const latest = await lastTickAt(logPath);
    if (latest !== undefined && (afterMs === undefined || latest > afterMs)) return latest;
    if (Date.now() >= deadline) {
      throw new Error(
        `no "${TICK_LOG_MESSAGE}" tick observed in ${logPath} within ${budgetMs}ms (after ` +
          `${afterMs === undefined ? 'boot' : new Date(afterMs).toISOString()}) — the federation ` +
          'discovery loop (60s cadence, `spawn_peer_discovery_task`) does not appear to be running ' +
          'on this doorway process'
      );
    }
    await delay(2_000);
  }
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
    ['-n', `${householdMeshDir()}/a2o.lock`, '/bin/bash', '-c', 'echo locked; read -r _'],
    { stdio: 'pipe' }
  );
  leases.set(world, child);
  const granted = await Promise.race([
    once(child.stdout, 'data').then(([chunk]) => String(chunk).includes('locked')),
    once(child, 'exit').then(() => false),
  ]);
  assert.ok(
    granted,
    'household mesh is already in use by another fault-injecting scenario; refusing to pause'
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
  requestUrl: string;
  response: RawResponse;
  elapsedMs: number;
  /** New log lines written by the ASKED doorway during this ask (relay evidence). */
  askedNewLines: LogLineFields[];
  otherNewLines: LogLineFields[];
  /** New request-log hits against `path` on the OTHER doorway during this ask ("was it contacted"). */
  otherRequestHits: number;
  otherId: string;
}

/**
 * Scenario 5's own fault-injection record — the dev-gated shed override this file
 * PUTs on a holder, and the two log offsets that bound the window it is honestly
 * checkable over (see `logBetween`'s doc for why "to end-of-file" is the wrong bound).
 */
interface BusyOverrideState {
  doorwayId: string;
  doorwayUrl: string;
  retryAfterSecs: number;
  /** Wall-clock instant the busy window was declared — VERIFIED busy first (see the
   * "the household makes doorway ... answer that it is busy ..." step), so this is the
   * instant a real, confirmed busy window began, never merely the instant the PUT was
   * sent. */
  declaredAtMs: number;
  /** The doorway's own stdout log — the same file `doorwayLogPath` resolves for every
   * other request-log check in this file. */
  logPath: string;
  /** `logLength(logPath)` taken AFTER this step's own verify-busy probe — deliberately
   * excluding that probe's own request line from the "nothing asked it while set aside"
   * window (the same "the extra ... request is outside that capture" convention this
   * file's header describes for the advertised-address scenario). */
  logOffsetAtDeclare: number;
  /** `logLength(logPath)` taken BEFORE the clearing step's own verify probe — same
   * self-contamination guard, at the other end of the window. Set once "doorway ... is
   * no longer busy" has cleared the override. */
  logOffsetAtClear?: number;
  /** Set once "doorway ... is no longer busy" has cleared the override — the After
   * hook's own signal that no further best-effort clear is needed. */
  clearedAtMs?: number;
}

interface NameRoutingState {
  root: string;
  /** The project-epr commitment's `urlPath` (no trailing slash — `/nrt-garden`). */
  mount: string;
  /** The visitor's path: the mount root, or its explicit entry for the lapse scenario. */
  path: string;
  staged: StagedContract[];
  paused: Map<string, OwnedProcessHandle>; // fixture id -> handle, while SIGSTOP'd
  ask?: AskCapture;
  secondAsk?: AskCapture;
  staleRegistry?: { doorwayId: string; holderId: string; tickAt: number };
  /** Scenario 5's busy/shed-override record — see `BusyOverrideState`'s doc. */
  busyOverride?: BusyOverrideState;
  /** Scenario 5's evidence for "doorway ... was never restarted or reconfigured during
   * this scenario": every doorway id this scenario's OWN glue issued an admin write
   * against (the shed-override PUTs; background staging writes never target a doorway
   * this scenario asserts non-interference for) and each doorway's `/proc` start tick,
   * captured once at the earliest point this file's own scenario-5 glue controls. */
  adminWritesByDoorwayId: Set<string>;
  startTicksByDoorwayId: Map<string, string>;
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
    adminWritesByDoorwayId: new Set(),
    startTicksByDoorwayId: new Map(),
  };
  states.set(world, state);
  return state;
}

/** Records that THIS scenario's own glue issued an admin write against `doorwayId` —
 * scoped to the shed-override control surface Scenario 5 introduces (see
 * `NameRoutingState.adminWritesByDoorwayId`'s doc for why the pre-existing background
 * staging writes need no separate tracking here: they never target the doorway this
 * scenario asserts non-interference for). */
function recordAdminWrite(state: NameRoutingState, doorwayId: string): void {
  state.adminWritesByDoorwayId.add(doorwayId);
}

/** Captures `doorwayId`'s `/proc` start tick once, at the EARLIEST point this file's
 * own scenario-5 glue controls — before any fault injection. Idempotent: a second call
 * for the same (state, doorwayId) is a no-op, so it is safe to call from more than one
 * step without re-resolving the process each time. See the "doorway ... was never
 * restarted or reconfigured during this scenario" Then step for why this baseline
 * matters: an unnoticed restart mid-scenario must read as a named finding, not a
 * silent false-green from a step that only checks "does it answer /health now". */
async function captureBaselineStartTicks(
  state: NameRoutingState,
  doorwayId: string
): Promise<void> {
  if (state.startTicksByDoorwayId.has(doorwayId)) return;
  const handle = await resolveOwnedMeshProcess(
    'doorway',
    MESH_LETTER[doorwayId],
    `doorway ${doorwayId}`
  );
  state.startTicksByDoorwayId.set(doorwayId, handle.ticks);
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

/**
 * The stale-registry premise, made deterministic against doorway "other"'s own ~60s
 * coherence-discovery cadence rather than against wall-clock luck.
 *
 * WHY THIS EXISTED AS A FLAKE: the old arrangement staged the root, waited (via a real
 * relayed GET) for `other`'s registry to know the holder, then IMMEDIATELY lapsed the
 * contract. That "immediately" carried no relationship to `other`'s OWN tick clock — the
 * confirming GET could have succeeded off a tick that fired anywhere from 0ms to just under
 * 60s ago (`waitForRegistryToKnowHolder` only proves "true as of the LAST completed tick",
 * never "true as of NOW"). So the very next tick — due at any point in that unknown
 * remaining window — could (and, per live observation, often did) land before the "Jessica
 * asks" step's GET, silently re-syncing `other`'s registry past the lapse and erasing the
 * premise: `other` would then answer with an immediate local refusal (0 relay attempts),
 * not the one attempted-and-failed hop this scenario measures.
 *
 * THE FIX: anchor on `other`'s own tick log ("Federation peer cache refreshed" —
 * `federation.rs::refresh_peer_cache`, the SAME loop iteration that calls
 * `refresh_coherence`/`install_name_routes` right after, no intervening sleep — see that
 * function's doc). Wait for a tick strictly AFTER the one observed before staging (so it is
 * known to have read the just-warmed, still-live contract), confirm the registry picked it
 * up, THEN lapse and drop the holder's own local dispatch — all within
 * `STALE_REGISTRY_SAFETY_MARGIN_MS` of that tick, comfortably inside the ~60s before
 * `other`'s NEXT tick could re-sync. The subsequent "Jessica asks" step is a single bounded
 * GET issued immediately once this `Given` returns, so keeping this arrangement inside the
 * margin is what keeps that ask's premise real.
 *
 * A miss (own dispatch too slow to drop, or the margin otherwise blown) retries the WHOLE
 * premise once against the doorway's NEXT tick — reusing the SAME staged contract via its
 * own reactivation branch (deterministic id — see `stageRoot`'s doc), never minting a
 * duplicate. Two misses in a row fails naming the exact timing gap, rather than silently
 * asserting on a premise that was never actually true.
 */
When(
  'the household withdraws the contract for {string} on doorway {string} before doorway {string} next refreshes its registry',
  { timeout: STALE_REGISTRY_ATTEMPT_BUDGET_MS * STALE_REGISTRY_MAX_ATTEMPTS },
  async function (
    this: E2EWorld,
    root: string,
    holderId: string,
    relayingId: string
  ): Promise<void> {
    const state = getState(this);
    assert.equal(state.root, root);
    // The lapsed site's entry must be absent, not replaced by the unrelated
    // root SPA's extension-less route fallback. While this contract is live,
    // the same entry is served and relayed with this scenario's own marker.
    state.path = `${state.mount}/index.html`;
    const holder = this.getDoorway(holderId);
    const otherId = otherFixtureId(holderId);
    assert.equal(otherId, relayingId, 'the stale registry belongs to the other household doorway');
    const other = this.getDoorway(otherId);
    const otherLogPath = await doorwayLogPath(MESH_LETTER[otherId], `doorway ${otherId}`);

    const staged = state.staged.find(contract => contract.doorwayUrl === holder.url);
    assert.ok(staged, `no staged contract for ${root} on ${holderId}`);
    let lastFailure: unknown;

    for (let attempt = 1; attempt <= STALE_REGISTRY_MAX_ATTEMPTS; attempt += 1) {
      if (attempt > 1) {
        // A previous attempt's OUTCOME is unknown here: it may have already lapsed this
        // row and confirmed the drop (the common case — the margin check itself is what
        // failed), or it may have thrown before ever reaching `lapseContract` (the tick
        // wait or the registry confirm timed out), leaving the row genuinely still live.
        // Settle it to a KNOWN state — cancelled AND confirmed dropped from `holder`'s own
        // local dispatch, using the FULL budget rather than a tick-margin remainder —
        // before reactivating. Skipping this settle is exactly the reactivation-races-
        // reconciliation hazard `scenarioNonce`'s own doc names: PATCHing a row back to
        // "proposed" while its PRIOR cancel is still winding through the doorway's
        // SSE-driven refresh can leave `holder`'s local view stuck on neither state for
        // the remainder of `waitForLocalMount`'s budget.
        await lapseContract(staged);
        await waitForOwnDispatchToDrop(holder.url, holderId, state.path, OWN_REFRESH_BUDGET_MS);

        // Now bring it back live so THIS attempt's premise (a GENUINELY live contract at
        // the moment `other` ticks) is real, not a leftover from the previous attempt.
        const reactivated = await adminCall(
          'PATCH',
          `${staged.doorwayUrl}/api/v1/commitments/${staged.commitmentId}`,
          { state: 'proposed' }
        );
        assert.equal(
          reactivated.status,
          200,
          `re-activating ${staged.commitmentId} on "${holderId}" for a retry of the stale-registry ` +
            `premise failed: HTTP ${reactivated.status} ${reactivated.text.slice(0, 300)}`
        );
      }
      await waitForLocalMount(holder.url, holderId, state.path, root);

      try {
        // Anchor on `other`'s own cadence: wait for a tick strictly after the last one
        // observed now (i.e., after the contract above is confirmed genuinely live).
        const baseline = await lastTickAt(otherLogPath);
        const tickAt = await waitForNextTick(otherLogPath, baseline, TICK_WAIT_BUDGET_MS);

        // Confirm the tick really did pick up the still-live contract. Short budget: if
        // the tick fired, the registry write is synchronous with it (same loop
        // iteration) — this is a confirmation, not another open-ended wait.
        await waitForRegistryToKnowHolder(other.url, otherId, state.path, holder.url, 15_000);

        // From this instant, "`other` names `holder` as holder" is STALE — true only
        // until `other`'s NEXT tick, due at roughly tickAt + 60s.
        await lapseContract(staged);

        const remainingMs = tickAt + STALE_REGISTRY_SAFETY_MARGIN_MS - Date.now();
        if (remainingMs <= 0) {
          throw new Error(
            `confirming the stale holder for "${root}" left no safety margin before doorway ` +
              `"${otherId}"'s next tick (tick observed at ${new Date(tickAt).toISOString()}, margin ` +
              `${STALE_REGISTRY_SAFETY_MARGIN_MS}ms)`
          );
        }
        await waitForOwnDispatchToDrop(holder.url, holderId, state.path, remainingMs);

        const elapsedSinceTick = Date.now() - tickAt;
        if (elapsedSinceTick > STALE_REGISTRY_SAFETY_MARGIN_MS) {
          throw new Error(
            `doorway "${holderId}"'s own dispatch dropped ${elapsedSinceTick}ms after doorway ` +
              `"${otherId}"'s tick at ${new Date(tickAt).toISOString()} — past the ` +
              `${STALE_REGISTRY_SAFETY_MARGIN_MS}ms safety margin before its next tick would re-sync ` +
              "the registry and erase the stale-holder premise before Jessica's ask can land"
          );
        }

        // `waitForOwnDispatchToDrop` just confirmed the drop (two consecutive 404s); the
        // margin check above is measured against THAT confirmation instant, unchanged. What
        // follows is additional settle time so the visitor's ask (fired the instant this
        // `Given` returns) never lands inside the revoke's own settle window — the doorway's
        // broader re-sync (SSE re-fetch, EprRouter install) can still be finishing a beat
        // after the local dispatch itself already reads dropped.
        await delay(SETTLE_AFTER_DROP_MS);

        // Never let a second, unnoticed contract at this SAME mount decide the outcome
        // Jessica's ask is about to measure (see `nrtMount`'s doc for the mechanism this
        // guards against — measured live 2026-09-13).
        await assertExactlyOneCommitmentForMount(
          holder.url,
          holderId,
          state.mount,
          staged.commitmentId
        );
        state.staleRegistry = { doorwayId: otherId, holderId, tickAt };
        return;
      } catch (error) {
        lastFailure = error;
      }
    }

    throw new Error(
      `could not arrange the stale-registry premise for "${root}" on doorway "${holderId}" within ` +
        `${STALE_REGISTRY_MAX_ATTEMPTS} attempt(s) against doorway "${otherId}"'s ~60s coherence tick: ` +
        (lastFailure instanceof Error ? lastFailure.message : String(lastFailure))
    );
  }
);

Given(
  'doorway {string} has no contract to host {string} locally',
  { timeout: 15_000 },
  async function (this: E2EWorld, doorwayId: string, root: string): Promise<void> {
    const doorway = this.getDoorway(doorwayId);
    await assertNoLocalContract(doorway.url, doorwayId, root, getState(this).path);
  }
);

Given(
  'doorway {string} has observed the withdrawal of its local hosting contract for {string}',
  { timeout: 15_000 },
  async function (this: E2EWorld, doorwayId: string, root: string): Promise<void> {
    const doorway = this.getDoorway(doorwayId);
    await assertNoLocalContract(doorway.url, doorwayId, root, getState(this).path);
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

Then(
  'doorway {string} still resolves {string} to doorway {string} in its local registry',
  { timeout: 20_000 },
  async function (
    this: E2EWorld,
    relayingId: string,
    root: string,
    holderId: string
  ): Promise<void> {
    const state = getState(this);
    assert.equal(state.root, root);
    if (state.staleRegistry) {
      const stale = state.staleRegistry;
      assert.equal(stale.doorwayId, relayingId);
      assert.equal(stale.holderId, holderId);
      const log = await doorwayLogPath(MESH_LETTER[relayingId], `doorway ${relayingId}`);
      assert.equal(await lastTickAt(log), stale.tickAt, 'the registry refreshed after withdrawal');
      assert.ok(
        Date.now() - stale.tickAt < STALE_REGISTRY_SAFETY_MARGIN_MS,
        'the measured stale-registry window expired before the visitor request'
      );
      return;
    }
    await waitForRegistryToKnowHolder(
      this.getDoorway(relayingId).url,
      relayingId,
      state.path,
      this.getDoorway(holderId).url,
      5_000
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
  'Jessica asks doorway {string} for {string}( at its published entry document)',
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
      requestUrl: `${asked.url}${state.path}`,
      response,
      elapsedMs,
      askedNewLines,
      otherNewLines: parseLogLines(otherNewText),
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
  'the relayed page matches a direct request to doorway {string}',
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
  'doorway {string} logged a relay resolved through its hosting-contract registry',
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
  'the test client representing Jessica uses the returned origin to ask for {string} again',
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
      requestUrl: `${servedBy}${state.path}`,
      askedId: relayingId, // placeholder id label; the relaying doorway's log is what "forwarded nothing" reads
      response,
      elapsedMs,
      askedNewLines: newLines,
      otherNewLines: [],
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
    assert.equal(second.requestUrl, `${holder.url}${state.path}`);
    assert.ok(
      second.response.text.includes(nrtMarker(state.root)),
      'the direct holder address returned unrelated content instead of the staged site'
    );
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
// Scenario 3 — the holder is paused; the doorway Jessica reaches serves itself
// =============================================================================

When(
  'the household pauses doorway {string}',
  { timeout: PAUSE_SETTLE_MS + 20_000 },
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
    await delay(PAUSE_SETTLE_MS);
    await assertStillOwnedProcess(handle, `doorway ${doorwayId}`);
    const status = await readFile(`/proc/${handle.pid}/status`, 'utf8');
    assert.match(
      status,
      /^State:\s+T(?:\s|$)/m,
      `doorway "${doorwayId}" did not enter the stopped state`
    );
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
    const relayAttempts = ask.askedNewLines.filter(
      f => f['path'] === state.path && (f.message === MSG_RELAYED || f.message === MSG_ALL_FAILED)
    );
    assert.equal(
      relayAttempts.length,
      0,
      `doorway "${servingId}" logged a relay attempt before serving locally`
    );
  }
);

Then(
  'doorway {string} was never contacted for that request',
  function (this: E2EWorld, pausedId: string): void {
    const state = getState(this);
    const ask = requireAsk(state);
    assert.equal(
      ask.otherId,
      pausedId,
      `the captured ask tracked doorway "${ask.otherId}" as the sibling, not "${pausedId}"`
    );
    assert.equal(
      ask.otherRequestHits,
      0,
      `doorway "${pausedId}"'s access log gained ${ask.otherRequestHits} new request line(s) for ${state.path} ` +
        `during Jessica's ask — it WAS contacted`
    );
  }
);

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
    assert.equal(
      ask.otherRequestHits,
      1,
      `doorway "${holderId}" received ${ask.otherRequestHits} request(s) for ${state.path}; ` +
        `expected one forwarded request (visitor HTTP ${ask.response.status})`
    );
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
        `exactly one (visitor HTTP ${ask.response.status}; ` +
        `relay messages: ${JSON.stringify(ask.askedNewLines.filter(f => f['path'] === state.path))})`
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
  'doorway {string} received the forwarded request exactly once',
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
  'doorway {string} forwarded nothing onward for that request',
  function (this: E2EWorld, holderId: string): void {
    const state = getState(this);
    const ask = requireAsk(state);
    assert.equal(ask.otherId, holderId);
    const onward = ask.otherNewLines.filter(
      f => f['path'] === state.path && (f.message === MSG_RELAYED || f.message === MSG_ALL_FAILED)
    );
    assert.equal(
      onward.length,
      0,
      `doorway "${holderId}" attempted another relay for ${state.path}`
    );
  }
);

Then('Jessica received HTTP 404 for {string}', function (this: E2EWorld, root: string): void {
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
});

Then(
  'neither registered doorway served content for the lapsed site',
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

Then('Jessica received that refusal in less than 15 seconds', function (this: E2EWorld): void {
  const state = getState(this);
  const ask = requireAsk(state);
  assert.ok(
    ask.elapsedMs < REQUEST_TIMEOUT_MS,
    `Jessica's request took ${ask.elapsedMs}ms (bounded fetch is ${REQUEST_TIMEOUT_MS}ms) — this looks like a ` +
      'timeout, not a fast honest refusal'
  );
});

// =============================================================================
// Scenario 5 — a busy holder is set aside until the time it named has passed
//
// Two preconditions this scenario needs do NOT exist in the tree yet, and this
// file's glue is written to degrade HONESTLY against both rather than assume them:
//
//   (a) A third household doorway "gamma". `app/elohim-app/scripts/hc-mesh-prologue.sh`
//       declares it ABSENT with a named reason, and `household-mesh.ts`'s
//       `fixtureDoorwayUrl`/`requireFixtureDoorwayUrl` already throw that reason when
//       nothing overrides it. That throw happens inside the ALREADY-REGISTERED
//       `doorway {string} at {string}` step (`steps/mode-aware.steps.ts`) — this
//       scenario's own FIRST line — which this file must reuse unchanged and cannot
//       intercept. On a mesh with no `E2E_DOORWAY_GAMMA` set, the scenario therefore
//       fails at that first line with the absence reason named in the thrown message,
//       before any step in this section ever runs. None of the steps below can turn
//       that into a Cucumber PENDING status without editing mode-aware.steps.ts, which
//       is out of this file's scope — the honest degrade available from here is the
//       named, non-opaque error that step already throws.
//
//   (b) A dev-gated busy/shed override, `PUT {doorwayUrl}/admin/dev/shed` — the
//       proposed sibling of the already-live `PUT /admin/dev/portal-health`
//       (`doorway/doorway-service/src/routes/admin_dev.rs`). See `putShedOverride`'s
//       doc. Every step below that calls it checks `shedRouteMissing` FIRST and
//       returns Cucumber PENDING (via `pendingShedRoute`) when the route is absent —
//       never a false green, never an opaque assertion failure.
// =============================================================================

When(
  'the household makes doorway {string} answer that it is busy for longer than one registry refresh cycle',
  { timeout: REQUEST_TIMEOUT_MS * 2 + 15_000 },
  async function (this: E2EWorld, doorwayId: string): Promise<string | void> {
    const state = getState(this);
    const doorway = this.getDoorway(doorwayId);

    // Earliest point this file's own scenario-5 glue controls, before any fault
    // injection — the baseline "doorway ... was never restarted or reconfigured"
    // (step 7) compares against. Captured for "beta" here (not `doorwayId`, which is
    // the busy holder, "alpha") because "beta" is the doorway that step names, and
    // this is the first moment in the scenario this file's own glue runs at all.
    await captureBaselineStartTicks(state, 'beta');

    const declared = await putShedOverride(doorway.url, BUSY_WINDOW_SECS);
    if (shedRouteMissing(declared)) return pendingShedRoute(this, doorwayId, declared);
    assert.equal(
      declared.status,
      200,
      `PUT ${doorway.url}/admin/dev/shed {"retryAfterSecs":${BUSY_WINDOW_SECS}} failed: ` +
        `HTTP ${declared.status} ${declared.text.slice(0, 300)}`
    );
    recordAdminWrite(state, doorwayId);

    // VERIFY the holder really is busy — never proceed on an unverified premise. Forced
    // local-only (x-federation-hop:1) so this is unambiguously THIS doorway's own answer,
    // never an accidental relay.
    const probe = await localOnlyGet(`${doorway.url}${state.path}`);
    assert.equal(
      probe.status,
      503,
      `doorway "${doorwayId}" answered HTTP ${probe.status} to a forced local-only probe right ` +
        'after its shed override was set — expected 503 (busy)'
    );
    const retryAfter = probe.headers['retry-after'];
    assert.ok(
      retryAfter,
      `doorway "${doorwayId}"'s busy 503 carries no Retry-After header — the busy answer must ` +
        'name how long it expects to stay that way'
    );
    assert.equal(
      Number(retryAfter),
      BUSY_WINDOW_SECS,
      `doorway "${doorwayId}"'s Retry-After (${retryAfter}) does not match the declared busy ` +
        `window (${BUSY_WINDOW_SECS}s)`
    );

    const logPath = await doorwayLogPath(MESH_LETTER[doorwayId], `doorway ${doorwayId}`);
    state.busyOverride = {
      doorwayId,
      doorwayUrl: doorway.url,
      retryAfterSecs: BUSY_WINDOW_SECS,
      declaredAtMs: Date.now(),
      logPath,
      // Taken AFTER the verify-busy probe above — see `BusyOverrideState.logOffsetAtDeclare`'s
      // doc for why: that probe's own request line must never count as "something asked the
      // busy holder while it was set aside" (step 6).
      logOffsetAtDeclare: await logLength(logPath),
    };
  }
);

When(
  'doorway {string} next refreshes its registry',
  { timeout: NEXT_REFRESH_TICK_BUDGET_MS + 15_000 },
  async function (this: E2EWorld, doorwayId: string): Promise<void> {
    // Same anchor-on-the-doorway's-own-tick technique as "the household withdraws the
    // contract ... before doorway X next refreshes its registry" above: the tick log
    // line is the only observable proxy for "this doorway's name-route registry just
    // relearned who holds what" (see `TICK_LOG_MESSAGE`'s own doc).
    const logPath = await doorwayLogPath(MESH_LETTER[doorwayId], `doorway ${doorwayId}`);
    const baseline = await lastTickAt(logPath);
    await waitForNextTick(logPath, baseline, NEXT_REFRESH_TICK_BUDGET_MS);
  }
);

Then(
  'doorway {string} never asked doorway {string} for {string} on that request',
  function (this: E2EWorld, relayingId: string, holderId: string, root: string): void {
    const state = getState(this);
    assert.equal(root, state.root, `scenario staged root "${state.root}", not "${root}"`);
    // "on that request" — the MOST RECENT ask (see `requireLatestAsk`'s doc), not the
    // first: this step runs after the scenario's SECOND "Jessica asks doorway beta..."
    const ask = requireLatestAsk(state);
    assert.equal(
      ask.askedId,
      relayingId,
      `the most recent captured ask was against "${ask.askedId}", not "${relayingId}"`
    );
    // Reuses the exact mechanism "doorway ... was never contacted for that request"
    // (Scenario 3) verifies with — `ask.otherRequestHits`/`ask.otherId`, populated by
    // the shared "Jessica asks doorway ... for ..." step — parameterised here by the
    // NAMED holder rather than that step's own implicit "other" doorway. The equality
    // check below is what makes that parameterisation honest: it fails loudly, rather
    // than silently trusting the tracked sibling, if a future scenario ever asks with a
    // holder this ask capture was not scoped to.
    assert.equal(
      ask.otherId,
      holderId,
      `this ask's tracked sibling is doorway "${ask.otherId}", not the named holder ` +
        `"${holderId}" — the request-log evidence below would not be about "${holderId}"`
    );
    assert.equal(
      ask.otherRequestHits,
      0,
      `doorway "${holderId}"'s access log gained ${ask.otherRequestHits} new request line(s) for ` +
        `${state.path} during doorway "${relayingId}"'s most recent answer — it WAS asked`
    );
  }
);

When(
  'doorway {string} is no longer busy',
  { timeout: REQUEST_TIMEOUT_MS * 2 + 15_000 },
  async function (this: E2EWorld, doorwayId: string): Promise<string | void> {
    const state = getState(this);
    const busy = state.busyOverride;
    assert.ok(
      busy?.doorwayId === doorwayId,
      `doorway "${doorwayId}" was never declared busy by this scenario — "the household makes ` +
        'doorway ... answer that it is busy ..." must run first'
    );
    const doorway = this.getDoorway(doorwayId);

    const cleared = await putShedOverride(doorway.url, 0);
    if (shedRouteMissing(cleared)) return pendingShedRoute(this, doorwayId, cleared);
    assert.equal(
      cleared.status,
      200,
      `clearing doorway "${doorwayId}"'s shed override failed: HTTP ${cleared.status} ` +
        cleared.text.slice(0, 300)
    );
    recordAdminWrite(state, doorwayId);

    // Taken BEFORE the verify probe below — the same self-contamination guard as
    // `logOffsetAtDeclare`, at the other end of the window: our OWN verification
    // request must never count as evidence either way.
    busy.logOffsetAtClear = await logLength(busy.logPath);
    busy.clearedAtMs = Date.now();

    const probe = await localOnlyGet(`${doorway.url}${state.path}`);
    assert.equal(
      probe.status,
      200,
      `doorway "${doorwayId}" answered HTTP ${probe.status} to a forced local-only probe right ` +
        'after clearing its shed override — expected 200 (no longer busy)'
    );
    const marker = nrtMarker(state.root);
    assert.ok(
      probe.text.includes(marker),
      `doorway "${doorwayId}" answered 200 after clearing its shed override but without the ` +
        `staged archive's own marker (${marker}) — this looks like its own unrelated "/" landing ` +
        'page, not its staged mount'
    );
  }
);

When(
  'the time doorway {string} named has passed',
  { timeout: BUSY_WINDOW_SECS * 1000 + 30_000 },
  async function (this: E2EWorld, doorwayId: string): Promise<void> {
    const state = getState(this);
    const busy = state.busyOverride;
    assert.ok(
      busy?.doorwayId === doorwayId,
      `doorway "${doorwayId}" was never declared busy by this scenario`
    );
    const windowMs = busy.retryAfterSecs * 1000;
    const remaining = busy.declaredAtMs + windowMs + BUSY_WINDOW_SETTLE_MARGIN_MS - Date.now();
    if (remaining > 0) await delay(remaining);
    // Never merely sleep and claim it: measure the real elapsed time and assert it.
    const elapsed = Date.now() - busy.declaredAtMs;
    assert.ok(
      elapsed > windowMs,
      `only ${elapsed}ms elapsed since doorway "${doorwayId}" declared its ${windowMs}ms busy ` +
        'window — the passage of time this step asserts is not yet real'
    );
  }
);

Then(
  'nothing asked doorway {string} for {string} while it was set aside',
  { timeout: 15_000 },
  async function (this: E2EWorld, doorwayId: string, root: string): Promise<void> {
    const state = getState(this);
    assert.equal(root, state.root, `scenario staged root "${state.root}", not "${root}"`);
    const busy = state.busyOverride;
    assert.ok(
      busy?.doorwayId === doorwayId,
      `doorway "${doorwayId}" was never declared busy by this scenario`
    );
    assert.ok(
      busy.logOffsetAtClear !== undefined,
      `doorway "${doorwayId}"'s busy window was never cleared — "doorway ... is no longer busy" ` +
        'must run before this check'
    );
    // Bounded to [declared, cleared) — see `logBetween`'s doc: reading to end-of-file
    // here would also pick up the LEGITIMATE ask that follows clearing the override,
    // which is outside "while it was set aside". Scoped to `state.path` only: the
    // ordinary federation health probe legitimately contacts the holder on OTHER
    // paths, and asserting zero-hits-on-any-path would be false.
    const window = await logBetween(busy.logPath, busy.logOffsetAtDeclare, busy.logOffsetAtClear);
    const hits = requestLineHits(parseLogLines(window), state.path);
    assert.equal(
      hits,
      0,
      `doorway "${doorwayId}"'s access log gained ${hits} new request line(s) for ${state.path} ` +
        'between the moment it was declared busy and the moment it was cleared — something asked ' +
        'it while it was set aside'
    );
  }
);

Then(
  'doorway {string} was never restarted or reconfigured during this scenario',
  { timeout: 15_000 },
  async function (this: E2EWorld, doorwayId: string): Promise<void> {
    const state = getState(this);
    const baseline = state.startTicksByDoorwayId.get(doorwayId);
    assert.ok(
      baseline,
      `no baseline /proc start tick was captured for doorway "${doorwayId}" — this scenario's own ` +
        '"the household makes doorway ... answer that it is busy ..." step must run first'
    );
    const handle = await resolveOwnedMeshProcess(
      'doorway',
      MESH_LETTER[doorwayId],
      `doorway ${doorwayId}`
    );
    assert.equal(
      handle.ticks,
      baseline,
      `doorway "${doorwayId}"'s /proc start tick changed from ${baseline} to ${handle.ticks} ` +
        'during this scenario — it was restarted'
    );
    assert.ok(
      !state.adminWritesByDoorwayId.has(doorwayId),
      `this scenario issued an admin write to doorway "${doorwayId}" — it should never have been ` +
        'administratively touched'
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
    // Scenario 5's own fault injection: a shed override left set (a mid-scenario
    // failure never reaches "doorway ... is no longer busy") must never survive past
    // this scenario, or it silently poisons whatever runs against this doorway next.
    if (state.busyOverride && state.busyOverride.clearedAtMs === undefined) {
      try {
        await putShedOverride(state.busyOverride.doorwayUrl, 0);
      } catch {
        // best-effort — matches this hook's own swallow convention below
      }
    }
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
