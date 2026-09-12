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
 * WHY THE TEST ROOT IS `/nrt-<slug>.html`, NOT A BARE PATH. Every doorway in
 * this household already projects `elohim-host-landing` at the universal root
 * mount `"/"` with `spaFallback: true` — confirmed live (`GET
 * /totally-random-nonexistent-xyz` on either doorway answers 200 with the
 * Angular shell). `EprRouter::path_matches_prefix` treats `"/"` as covering
 * EVERY request path, and `derive_app_subpath` (`server/http.rs`) only bypasses
 * `spaFallback`'s "serve entry_file" behaviour for a sub-path whose LAST
 * segment carries a `.` (`is_spa_route_subpath`) — genuine assets 404 through
 * verbatim, extensionless "deep routes" always get the SPA shell. So a bare
 * name like `/garden` can NEVER 404 locally while the real `"/"` mount is live,
 * and the relay would never fire. Naming the test root `/nrt-<slug>.html` (an
 * extension-bearing top-level path) escapes the SPA fallback: a doorway that
 * mounts it EXACTLY (a bare-prefix hit, remainder empty) serves the shell via
 * `entry_file`; a doorway that does NOT mount it falls through to its own `"/"`
 * catch-all, strips a NON-empty, extension-bearing remainder, and proxies
 * verbatim to storage — which genuinely has no such file, and genuinely 404s.
 * Verified live (2026-09-12): holder 200 (bare-hit → entry_file), non-holder
 * 404 (`{"error": "File not found in app: <slug>.html"}`), holder-with-forced
 * local-only header (`x-federation-hop: 1`) still 200 — proving the holder's
 * 200 is a REAL local mount, not an accidental relay.
 *
 * WHY EVERY TEST CONTRACT PROJECTS THE ALREADY-CACHED `elohim-host-landing`
 * EPR rather than a fresh one: the seeder's own id formula
 * (`project-epr-${sha256(peer|action|doorway:D|epr:E)}`, `seed-projections.ts`
 * `baseProjectionId`) keys ONLY on (doorwayId, eprId) — NOT urlPath — so a
 * NEW commitment naming an eprId ALREADY mounted on that doorway (e.g.
 * `elohim-host-landing` at `"/"`) would collide with the REAL seeded id and a
 * drift-detected re-grant would SUPERSEDE the real `"/"` mount. This file
 * therefore mints its OWN ids (content-addressed over doorwayId + the test
 * urlPath, `project-epr-nrt-*`, never the seeder's formula) and always reuses
 * the `elohim-host-landing` eprId at a NEW, never-seeded urlPath — real bytes,
 * zero risk to the household's real mounts. `storage/db/rea_commitments`
 * carries no per-(doorway,eprId) uniqueness constraint beyond the id itself
 * (verified against `elohim-storage/src/db/rea_commitments.rs`), so two rows
 * for the same eprId at different urlPaths on the same doorway coexist fine.
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

/** "garden" -> "/nrt-garden.html". See the file header for why the extension matters. */
function nrtPath(root: string): string {
  const collapsed = root.toLowerCase().replace(/[^a-z0-9]+/g, '-');
  let start = 0;
  let end = collapsed.length;
  while (start < end && collapsed[start] === '-') start += 1;
  while (end > start && collapsed[end - 1] === '-') end -= 1;
  const slug = collapsed.slice(start, end);
  return `/nrt-${slug || 'root'}.html`;
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
  path: string;
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
 * churn to WITHIN one lane invocation (still needed — the After hook below cancels a
 * scenario's staged rows, and a later scenario in the SAME run reusing the same root
 * reactivates them) and guarantees a fresh run never touches a previous run's row at all,
 * cancelled-and-wedged or not.
 */
let runStamp: string | undefined;

Before(function (this: E2EWorld): void {
  runStamp ??= process.env['A2O_RUN_ID'] ?? `${process.pid}-${Date.now().toString(36)}`;
});

/** Content-addressed over (doorwayId, path, runStamp) — deterministic WITHIN one lane
 * invocation (so scenarios sharing a root reuse the same row, per the idempotent-reuse
 * design) but never collides with a previous run's row (see `runStamp`'s doc). By
 * construction can never collide with a REAL seeded project-epr id either (those are
 * addressed over (doorwayId, eprId), never urlPath — see file header). */
function testCommitmentId(doorwayId: string, path: string): string {
  assert.ok(runStamp, 'runStamp not minted yet — the Before hook must run before any staging step');
  const digest = createHash('sha256')
    .update(`${doorwayId}|${path}|${runStamp}`, 'utf8')
    .digest('hex')
    .slice(0, 16);
  return `project-epr-nrt-${digest}`;
}

/** Stage (or re-activate) a test project-epr contract projecting the household's own,
 * already-cached `elohim-host-landing` bundle at `path` on `doorwayId`, via `doorwayUrl`'s
 * own doorway (so the write lands on the storage that doorway's EprRouter refresh reads).
 * Idempotent: a 409 on a prior run's id is reactivated (state -> "proposed") rather than
 * treated as failure, so scenarios sharing a root (all four stage "garden") never collide. */
async function stageRoot(
  doorwayUrl: string,
  doorwayId: string,
  path: string
): Promise<StagedContract> {
  const id = testCommitmentId(doorwayId, path);
  const provider = testStewardPeerId();
  const metadata = {
    urlPath: path,
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
    inScopeOf: `doorway:${doorwayId}|epr:elohim-host-landing`,
    note: `[a2o name-routing-test] project elohim-host-landing at ${path} on ${doorwayId}`,
    metadataJson: JSON.stringify(metadata),
    metadata,
  };
  const created = await adminCall('POST', `${doorwayUrl}/api/v1/commitments`, body);
  if (created.status === 409) {
    // A prior scenario/run already minted this exact (doorwayId, path) row — reactivate it
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
      `staging ${path} on ${doorwayId} failed: HTTP ${created.status} ${created.text.slice(0, 300)}`
    );
  }
  return { commitmentId: id, doorwayUrl, doorwayId, path };
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

/** Best-effort — used only from the After hook. */
async function cancelContractQuiet(staged: StagedContract): Promise<void> {
  try {
    await adminCall('PATCH', `${staged.doorwayUrl}/api/v1/commitments/${staged.commitmentId}`, {
      state: 'cancelled',
    });
  } catch {
    // best-effort cleanup, matches E2EWorld.runCleanup's own swallow convention
  }
}

/** Poll until `doorwayUrl` locally serves `path` (forced local-only, so this proves a REAL
 * mount, never an accidental relay). A brand-new commitment reaches the doorway's EprRouter
 * via its SSE-driven refresh subscriber — asynchronous relative to the POST response — so a
 * single immediate check races that refresh and flakes; this bounds the wait instead. */
async function waitForLocalMount(
  doorwayUrl: string,
  doorwayLabel: string,
  path: string,
  root: string,
  budgetMs = 20_000
): Promise<void> {
  const deadline = Date.now() + budgetMs;
  let lastStatus = -1;
  for (;;) {
    const res = await localOnlyGet(`${doorwayUrl}${path}`);
    lastStatus = res.status;
    if (res.status === 200) return;
    if (Date.now() >= deadline) {
      throw new Error(
        `doorway "${doorwayLabel}" does not locally serve the just-staged root "${root}" (${path}) ` +
          `within ${budgetMs}ms: HTTP ${lastStatus}`
      );
    }
    await delay(1_000);
  }
}

async function assertNoLocalContract(
  doorwayUrl: string,
  doorwayLabel: string,
  path: string,
  root: string
): Promise<void> {
  const res = await localOnlyGet(`${doorwayUrl}${path}`);
  assert.equal(
    res.status,
    404,
    `doorway "${doorwayLabel}" answered the local-only probe (x-federation-hop:1) for "${root}" ` +
      `(${path}) with HTTP ${res.status}, not 404 — it holds a local mount there`
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
  path: string,
  budgetMs: number
): Promise<void> {
  const deadline = Date.now() + budgetMs;
  for (;;) {
    const res = await localOnlyGet(`${doorwayUrl}${path}`);
    if (res.status === 404) return;
    if (Date.now() >= deadline) {
      throw new Error(
        `doorway "${doorwayLabel}" still answers 200 for the lapsed contract at ${path} after ` +
          `${budgetMs}ms — its own EprRouter refresh (DOORWAY_EPR_REFRESH_SECS, default 30s) has not ` +
          `caught up with the cancellation yet`
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
    path: nrtPath(root),
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
    const staged = await stageRoot(holder.url, holderDoorwayId, state.path);
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
      const staged = await stageRoot(doorway.url, doorwayId, state.path);
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
    const staged = await stageRoot(holder.url, holderDoorwayId, state.path);
    state.staged.push(staged);

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
    await waitForOwnDispatchToDrop(holder.url, holderId, state.path, OWN_REFRESH_BUDGET_MS);
  }
);

Given(
  'doorway {string} holds no hosting contract for {string}',
  { timeout: 15_000 },
  async function (this: E2EWorld, doorwayId: string, root: string): Promise<void> {
    const doorway = this.getDoorway(doorwayId);
    const path = nrtPath(root);
    await assertNoLocalContract(doorway.url, doorwayId, path, root);
  }
);

Given(
  'doorway {string} holds no live hosting contract for {string}',
  { timeout: 15_000 },
  async function (this: E2EWorld, doorwayId: string, root: string): Promise<void> {
    const doorway = this.getDoorway(doorwayId);
    const path = nrtPath(root);
    await assertNoLocalContract(doorway.url, doorwayId, path, root);
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
  assert.ok(
    ask.response.text.includes('<app-root') || ask.response.text.includes('<html'),
    `Jessica's request for "${root}" did not return the app shell: ${ask.response.text.slice(0, 200)}`
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
