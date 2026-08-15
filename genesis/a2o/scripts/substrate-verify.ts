/**
 * `substrate-verify` — runtime-layer substrate validation, ported from
 * `genesis/scripts/ci/substrate-verify.sh` onto the typed `@elohim/storage-client`
 * dataplane facade.
 *
 * The bash script is the SPEC. Every assertion name, detail string, env var,
 * default, state-file handoff, report shape and exit code is reproduced
 * byte-for-byte wherever the facade allows it; each place it cannot is marked
 * with a `// parity:` comment naming the bash line it mirrors or diverges from.
 *
 * Subcommands (identical to bash):
 *   mesh         peer mesh formed: per-pod libp2p peer counts, adjacency,
 *                version parity, doorway pool health
 *   upload       stage blob-backed content onto the genesis peer via doorway,
 *                plus a BUILD-UNIQUE propagation probe blob
 *   propagation  cross-pod blob distribution: custody manifest, probe fetch,
 *                bytes persisted, manifest convergence
 *   delivery     serve-blob REA EconomicEvents emitted for this build
 *   projection   projector cursors + replication/pull/projection-reconcile
 *   federation   doorway federation membership + p2p bootstrap surface
 *   resilience   conductor-signal-driven posture (fleet view from ONE pod)
 *
 * Report: `$REPORT_DIR/substrate-verify-<cmd>.json`, written UNCONDITIONALLY
 * (bash's `finish` always calls `write_report` first, even on preflight fail).
 * The report carries one field bash does not: `runner: "facade"`. CI's measure
 * counts artifacts with `runner == "facade" && failed == 0`, so it is
 * load-bearing — do not drop it.
 *
 * Exit codes: 0 = no fail-status assertions, 1 = failures, 2 = usage/fatal.
 *
 * Design notes carried over from bash:
 *   - Blob presence checks use GET, never HEAD (head/get asymmetry: HEAD does
 *     not trigger heal-on-read and lies about fetchability). The facade's
 *     `getBlobStatus` is a real GET that discards the body.
 *   - Assertions accumulate; the report is always written; exit 1 iff any
 *     assertion FAILED. Stages wrap calls in catchError(UNSTABLE).
 *
 * Parity coverage lives in `scripts/substrate-verify.selftest.ts`.
 */

/*
 * eslint-disable-next-line is not enough here: EVERY `||` in this file encodes
 * bash's `${VAR:-default}` / jq's `// alt`, both of which fall back on an
 * EMPTY string as well as on unset. `??` would silently change that — an
 * exported-but-empty CONTENT_BLOB_HASH must fall through to the state file,
 * and a Jenkins `withEnv` export of an empty string is exactly how that
 * happens in practice.
 */
/* eslint-disable @typescript-eslint/prefer-nullish-coalescing */

import { createHash, randomBytes } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

// Type-only import: erased at runtime, so this file loads even before the
// facade package is built. The runtime binding is the dynamic import in
// `defaultFacade()`. NOTE: `pnpm typecheck` in genesis/a2o cannot pass until
// `@elohim/storage-client` exists on disk and `pnpm install` has run.
import type {
  DataplaneFleet,
  DoorwayClient,
  StorageClient,
  streamSyncState,
} from '@elohim/storage-client';

// ---------------------------------------------------------------------------
// Frozen facade contract — local structural mirrors
//
// The runner talks to these interfaces, never to concrete classes, so the
// selftest can inject a fixture-backed implementation without the package
// being built. `FacadeContractProof` below is a compile-time assertion that
// the real facade satisfies each mirror.
// ---------------------------------------------------------------------------

export type StreamName = 'replication' | 'pull' | 'projectionReconcile';
export type StreamState = 'caught-up' | 'behind' | 'idle' | 'not-computable';

export interface DataplaneApiLike {
  getP2pStatus(): Promise<unknown>;
  getVersion(): Promise<{ commit?: string; version?: string }>;
  listCommitments(q?: { action?: string; limit?: number }): Promise<unknown[]>;
  listEconomicEvents(q?: { action?: string; after?: string; limit?: number }): Promise<unknown[]>;
  getProjectorStatus(): Promise<unknown>;
  getInventoryParity(): Promise<{ filesystemCount: number | null }>;
  /** Real GET, body discarded; returns HTTP status, throws only on network error. */
  getBlobStatus(hash: string): Promise<number>;
  listConnectedPeers(): Promise<unknown>;
  listPeerStatuses(): Promise<unknown>;
  getNetworkPosture(): Promise<unknown>;
}

export interface StorageClientLike {
  dataplane: DataplaneApiLike;
}

export interface FleetLike {
  peers(): { name: string; client: StorageClientLike }[];
  peer(name: string): StorageClientLike | undefined;
  names(): string[];
}

export interface SeedBlobResult {
  ok: boolean;
  status: number;
  alreadyPresent: boolean;
  errorBody?: string;
}

export interface DoorwayLike {
  getHealth(): Promise<unknown>;
  seedBlob(args: { hash: string; data: Uint8Array; token?: string }): Promise<SeedBlobResult>;
  listFederationDoorways(): Promise<unknown>;
  listFederationP2pPeers(): Promise<unknown>;
}

export interface Facade {
  fleetFromCsv(csv: string, opts: { hAppId?: string; timeout?: number }): FleetLike;
  makeDoorway(baseUrl: string): DoorwayLike;
  streamSyncState(raw: unknown, stream: StreamName): StreamState;
}

type Assert<T extends true> = T;
type IsAny<T> = 0 extends 1 & T ? true : false;
/**
 * `T` satisfies `U` — vacuously true while `T` is `any`, which is what the
 * facade's exports resolve to until `@elohim/storage-client` is installed and
 * built into a2o's resolution graph. The proof is therefore INERT before the
 * workspace link exists and LOAD-BEARING the moment it does: a mismatch
 * between these mirrors and the real facade becomes a typecheck error rather
 * than a runtime surprise in CI.
 */
type Satisfies<T, U> = IsAny<T> extends true ? true : T extends U ? true : false;

/** Compile-time proof the local mirrors match the frozen facade interface. */
export type FacadeContractProof = [
  Assert<Satisfies<DataplaneFleet, FleetLike>>,
  Assert<Satisfies<StorageClient, StorageClientLike>>,
  Assert<Satisfies<DoorwayClient, DoorwayLike>>,
  Assert<Satisfies<typeof streamSyncState, (raw: unknown, s: StreamName) => StreamState>>,
];

// ---------------------------------------------------------------------------
// Report shape
// ---------------------------------------------------------------------------

export type AssertionStatus = 'pass' | 'warn' | 'fail';

export interface Assertion {
  status: AssertionStatus;
  name: string;
  detail: string;
}

export interface Report {
  schemaVersion: '1';
  /** Additive vs bash. CI counts artifacts with runner=="facade" && failed==0. */
  runner: 'facade';
  command: string;
  generatedAt: string;
  failed: number;
  assertions: Assertion[];
  context: Record<string, unknown>;
}

export interface RunContext {
  env: NodeJS.ProcessEnv;
  facade: Facade;
  /** Test seam: bash's `sleep <secs>`. Real runs use setTimeout. */
  sleep(seconds: number): Promise<void>;
  log(line: string): void;
  /** Working directory for relative CONTENT_PATH resolution (bash uses $PWD). */
  cwd: string;
}

export interface RunResult {
  exitCode: number;
  report: Report;
  reportPath: string;
}

const USAGE =
  'usage: substrate-verify.ts {mesh|upload|propagation|delivery|projection|federation|resilience}';

const HR = '═══════════════════════════════════════════════════════════';

const COMMANDS = [
  'mesh',
  'upload',
  'propagation',
  'delivery',
  'projection',
  'federation',
  'resilience',
] as const;
export type Command = (typeof COMMANDS)[number];

/**
 * Assertion names reused across pass/warn/fail branches. Naming them keeps the
 * branches provably identical — the archived artifacts are diffed against the
 * bash runner's output, so a one-character drift is a real regression.
 */
const A = {
  meshAdjacency: 'mesh.adjacency',
  meshAdjacencyReverse: 'mesh.adjacency-reverse',
  meshReachable: 'mesh.reachable',
  meshVersionParity: 'mesh.version-parity',
  uploadPreflight: 'upload.preflight',
  uploadContent: 'upload.content',
  propagationPreflight: 'propagation.preflight',
  propagationCustodyManifest: 'propagation.custody-manifest',
  propagationSourceContent: 'propagation.source-content',
  propagationSourceProbe: 'propagation.source-probe',
  propagationParityBaseline: 'propagation.parity-baseline',
  propagationProbeFetch: 'propagation.probe-fetch',
  propagationContentFetch: 'propagation.content-fetch',
  propagationBytesPersisted: 'propagation.bytes-persisted',
  propagationCustodyConvergence: 'propagation.custody-convergence',
  deliveryPreflight: 'delivery.preflight',
  deliveryProbe: 'delivery.probe',
  deliveryServeBlobEvents: 'delivery.serve-blob-events',
  federationPreflight: 'federation.preflight',
  federationDoorways: 'federation.doorways',
  federationP2pPeers: 'federation.p2p-peers',
  federationShardCapability: 'federation.shard-capability',
  resilienceReachable: 'resilience.reachable',
} as const;

// ---------------------------------------------------------------------------
// Run — assertion plumbing (bash lines 51-126)
// ---------------------------------------------------------------------------

class Run {
  readonly assertions: Assertion[] = [];
  failCount = 0;
  /** Mirror of bash's `$STATE_DIR/.last_status` (see http_get, bash:96-108). */
  lastStatus = '000';

  readonly reportDir: string;
  readonly stateDir: string;
  readonly curlTimeoutSecs: number;

  private fleetCache: FleetLike | null = null;

  constructor(readonly ctx: RunContext) {
    this.reportDir = ctx.env['REPORT_DIR'] || '.';
    // sonarjs/publicly-writable-directories: /tmp/substrate-verify is the
    // documented cross-stage handoff location — `genesis/Jenkinsfile` `cat`s
    // these exact paths between stages. Changing it breaks the contract.
    // eslint-disable-next-line sonarjs/publicly-writable-directories
    this.stateDir = ctx.env['STATE_DIR'] || '/tmp/substrate-verify';
    this.curlTimeoutSecs = numEnv(ctx.env['CURL_TIMEOUT'], 10);
    mkdirSync(this.stateDir, { recursive: true }); // bash:46
  }

  // -- assertion recorders (bash:57-66) -------------------------------------

  note(detail: string): void {
    this.ctx.log(`   ${detail}`);
  }

  pass(name: string, detail: string): void {
    this.assertions.push({ status: 'pass', name, detail });
    this.ctx.log(`   ✅ ${name} — ${detail}`);
  }

  warn(name: string, detail: string): void {
    this.assertions.push({ status: 'warn', name, detail });
    this.ctx.log(`   ⚠️  ${name} — ${detail}`);
  }

  fail(name: string, detail: string): void {
    this.assertions.push({ status: 'fail', name, detail });
    this.ctx.log(`   ❌ ${name} — ${detail}`);
    this.failCount++;
  }

  // -- report (bash:68-91) --------------------------------------------------

  writeReport(command: string, context: Record<string, unknown> = {}): RunResult {
    const out = join(this.reportDir, `substrate-verify-${command}.json`);
    const report: Report = {
      schemaVersion: '1',
      runner: 'facade',
      command,
      generatedAt: isoUtcSeconds(),
      failed: this.assertions.filter(a => a.status === 'fail').length,
      assertions: this.assertions,
      context,
    };
    // parity: bash relies on $REPORT_DIR pre-existing (the jq redirect fails
    // silently otherwise, leaving no artifact). Creating it is strictly safer
    // and never changes an existing-directory run.
    mkdirSync(this.reportDir, { recursive: true });
    writeFileSync(out, JSON.stringify(report, null, 2));
    this.ctx.log('');
    this.ctx.log(`Report: ${out} (${this.failCount} failed assertion(s))`);
    return { exitCode: this.failCount === 0 ? 0 : 1, report, reportPath: out };
  }

  /** bash `finish` — write the report, then exit 0 iff nothing failed. */
  finish(command: string, context: Record<string, unknown> = {}): RunResult {
    return this.writeReport(command, context);
  }

  // -- facade plumbing ------------------------------------------------------

  /**
   * bash `peers()` / `split_peer()` (bash:114-118) — PEER_STORAGE_URLS is a
   * `name=host:port` CSV. Parsing belongs to the facade.
   */
  fleet(): FleetLike {
    this.fleetCache ??= this.ctx.facade.fleetFromCsv(this.ctx.env['PEER_STORAGE_URLS'] || '', {
      hAppId: 'elohim',
      // parity: CURL_TIMEOUT is seconds in bash (`curl -m`); the facade's
      // `timeout` unit is not pinned by the frozen interface — milliseconds
      // is the conventional reading for a TS client option.
      timeout: this.curlTimeoutSecs * 1000,
    });
    return this.fleetCache;
  }

  /**
   * Display-only mirror of the CSV parse. The facade owns the real parse (and
   * the clients); bash's detail strings quote the peer URL, which the frozen
   * `StorageClient` interface does not expose.
   */
  displayUrls(): Map<string, string> {
    const map = new Map<string, string>();
    for (const entry of (this.ctx.env['PEER_STORAGE_URLS'] || '').split(',')) {
      const trimmed = entry.trim();
      const eq = trimmed.indexOf('=');
      if (trimmed === '' || eq < 0) continue;
      map.set(trimmed.slice(0, eq), `http://${trimmed.slice(eq + 1)}`);
    }
    return map;
  }

  doorway(): DoorwayLike {
    return this.ctx.facade.makeDoorway(`http://${this.ctx.env['INTERNAL_DOORWAY_URL'] ?? ''}`);
  }

  /**
   * bash `http_get` semantics over a facade call: success ⇒ ok, any throw ⇒
   * not-ok with a status recorded for `last_status` interpolation.
   *
   * parity gap: bash records the real HTTP code for every route (curl -w
   * '%{http_code}'). The facade surfaces a status only for `getBlobStatus`;
   * for everything else a thrown error's `status`/`statusCode` is used when
   * present, and "000" (bash's transport-failure code) otherwise.
   */
  async call<T>(fn: () => Promise<T>): Promise<{ ok: true; value: T } | { ok: false }> {
    try {
      const value = await fn();
      this.lastStatus = '200';
      return { ok: true, value };
    } catch (e) {
      this.lastStatus = statusFromError(e);
      return { ok: false };
    }
  }

  /** bash `http_status_only` — a real GET whose body is discarded. */
  async blobStatus(client: StorageClientLike, hash: string): Promise<string> {
    try {
      return String(await client.dataplane.getBlobStatus(hash));
    } catch {
      return '000'; // bash: curl transport failure → "000"
    }
  }

  // -- state files (the Jenkins cross-stage handoff) ------------------------

  readState(file: string): string {
    const path = join(this.stateDir, file);
    if (!existsSync(path)) return '';
    // bash `$(cat file)` strips trailing newlines. Done without a regex so no
    // backtracking class exists on an adversarial file.
    let raw = readFileSync(path, 'utf8');
    while (raw.endsWith('\n')) raw = raw.slice(0, -1);
    return raw;
  }

  writeState(file: string, value: string): void {
    writeFileSync(join(this.stateDir, file), `${value}\n`);
  }
}

// ---------------------------------------------------------------------------
// small helpers
// ---------------------------------------------------------------------------

function isoUtcSeconds(): string {
  // bash: date -u +%Y-%m-%dT%H:%M:%SZ (no milliseconds)
  return new Date()
    .toISOString()
    .replace('.000Z', 'Z')
    .replace(/\.\d\d\dZ$/, 'Z');
}

function numEnv(raw: string | undefined, fallback: number): number {
  if (raw === undefined || raw === '') return fallback;
  const n = Number(raw);
  return Number.isFinite(n) ? n : fallback;
}

function statusFromError(e: unknown): string {
  const o = asObject(e);
  for (const key of ['status', 'statusCode']) {
    const v = o[key];
    if (typeof v === 'number' && Number.isFinite(v)) return String(v);
    if (typeof v === 'string' && /^\d{3}$/.test(v)) return v;
  }
  const rv = asObject(o['response'])['status'];
  return typeof rv === 'number' && Number.isFinite(rv) ? String(rv) : '000';
}

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

/** Always an object, so property reads mirror jq's forgiving `.foo` on null. */
function asObject(v: unknown): Record<string, unknown> {
  return isRecord(v) ? v : {};
}

/** jq's `x // "fallback"` — only null and false take the alternative. */
function jqAlt(v: unknown, fallback: string): string {
  return v === null || v === undefined || v === false ? fallback : String(v);
}

/** jq's `.a // .b // empty` over candidate keys. */
function firstString(o: Record<string, unknown>, ...keys: string[]): string {
  for (const k of keys) {
    const v = o[k];
    if (v !== null && v !== undefined && v !== false) return String(v);
  }
  return '';
}

/** `head -c N` is BYTES, not characters. */
function headBytes(s: string, n: number): string {
  const buf = Buffer.from(s, 'utf8');
  return buf.length <= n ? s : buf.subarray(0, n).toString('utf8');
}

function sha256Hex(data: Uint8Array): string {
  return createHash('sha256').update(data).digest('hex');
}

/**
 * bash's `[ "$x" -ge "$floor" ]` over a jq `// 0` default: a non-numeric value
 * makes the test ERROR OUT, which bash reads as false.
 */
function atLeast(raw: string, floor: number): boolean {
  const n = Number(raw);
  return Number.isFinite(n) && n >= floor;
}

/** bash: `[.[]? | select(.action=="custody-blob" and (.resourceClassifiedAs|index($h)))] | length >= 1` */
function hasCustodyBlobFor(rows: unknown, hash: string): boolean {
  if (!Array.isArray(rows)) return false; // jq `.[]?` on a non-array yields nothing
  return rows.some(row => {
    const o = asObject(row);
    if (o['action'] !== 'custody-blob') return false;
    const rc = o['resourceClassifiedAs'];
    return Array.isArray(rc) && rc.includes(hash);
  });
}

/** bash: `[(.peers // [])[] | (.peerId // .peer_id)]` */
function peerIdsOf(body: unknown): string[] {
  const list = asObject(body)['peers'];
  if (!Array.isArray(list)) return [];
  return list.map(p => firstString(asObject(p), 'peerId', 'peer_id')).filter(id => id !== '');
}

/** Map the facade's stream verdict onto bash's jq-rendered tri-state string. */
/**
 * Fail-closed override for a facade 'not-computable' verdict: if the stream
 * OBJECT is present (either casing) but the facade couldn't read a boolean
 * `caughtUp`/`caught_up` from it (schema rename / type drift), report
 * 'behind' → renders "false" → FAIL, matching bash's `// false | tostring`.
 * An absent/null stream object stays 'not-computable' → "null" → WARN.
 */
function failClosed(body: unknown, stream: StreamName, state: StreamState): StreamState {
  if (state !== 'not-computable') return state;
  const root = asObject(body);
  const key = stream === 'projectionReconcile' ? 'projectionReconcile' : stream;
  const snake = stream === 'projectionReconcile' ? 'projection_reconcile' : stream;
  let raw: unknown;
  if (key in root) {
    raw = root[key];
  } else if (snake in root) {
    raw = root[snake];
  }
  const present = raw !== null && raw !== undefined && typeof raw === 'object';
  return present ? 'behind' : 'not-computable';
}

function streamWord(state: StreamState): string {
  switch (state) {
    case 'caught-up':
      return 'true';
    case 'behind':
      return 'false';
    case 'idle':
      return 'idle';
    default:
      return 'null';
  }
}

// ===========================================================================
// mesh — peer mesh formed (bash:131-228)
// ===========================================================================

interface MeshPeerProbe {
  reachable: boolean;
  peerId: string;
  version: string;
  row?: Record<string, unknown>;
}

/**
 * One pod's mesh probe: /p2p/status → /p2p/peers → /version, plus the
 * per-pod connected assertion (bash:143-171).
 */
async function probeMeshPeer(
  run: Run,
  name: string,
  client: StorageClientLike,
  peerUrl: string,
  minConnected: number
): Promise<MeshPeerProbe> {
  const status = await run.call(async () => client.dataplane.getP2pStatus());
  if (!status.ok) {
    run.warn(
      `mesh.${name}.reachable`,
      `storage HTTP unreachable (${peerUrl}, status ${run.lastStatus})`
    );
    return { reachable: false, peerId: '', version: '' };
  }

  const s = asObject(status.value);
  const connected = jqAlt(s['connectedPeers'], '0');
  const nat = jqAlt(s['natStatus'], '?');

  // Archive the FULL connected-peer-id list, not just the count: the
  // 2026-06-11 #1119 partition (matthew↔jessica absent from each other's sets
  // while both held 10+ connections) was undiagnosable from counts alone —
  // the artifact must name WHO each pod is connected to.
  const peersRes = await run.call(async () => client.dataplane.listConnectedPeers());

  if (atLeast(connected, minConnected)) {
    run.pass(`mesh.${name}.connected`, `connectedPeers=${connected} nat=${nat}`);
  } else {
    run.fail(
      `mesh.${name}.connected`,
      `connectedPeers=${connected} < ${minConnected} (nat=${nat})`
    );
  }

  const verRes = await run.call(async () => client.dataplane.getVersion());

  return {
    reachable: true,
    peerId: firstString(s, 'peerId'),
    version: verRes.ok ? firstString(asObject(verRes.value), 'commit', 'version') : '',
    row: {
      name,
      peerId: s['peerId'] ?? null,
      connectedPeers: s['connectedPeers'] ?? null,
      natStatus: s['natStatus'] ?? null,
      relayMode: s['relayMode'] ?? null,
      connectedPeerIds: peersRes.ok ? peerIdsOf(peersRes.value) : [],
    },
  };
}

/**
 * One direction of the adjacency check (bash:182-206). Both directions share
 * this shape: guard on "URL resolvable AND the other pod's id known", then a
 * live /p2p/peers read.
 *
 * parity: bash pipes http_get into `jq -e` — a non-200 leaves jq with no
 * usable input, which exits non-zero and lands in the FAIL branch (not the
 * warn branch). A declared-but-DOWN peer therefore FAILS rather than warns;
 * only an unknown peer-id short-circuits to warn.
 */
async function assertAdjacency(
  run: Run,
  name: string,
  client: StorageClientLike | undefined,
  expectId: string,
  details: { pass: string; fail: string; warn: string }
): Promise<void> {
  if (!client || !expectId) {
    run.warn(name, details.warn);
    return;
  }
  const res = await run.call(async () => client.dataplane.listConnectedPeers());
  if (res.ok && peerIdsOf(res.value).includes(expectId)) run.pass(name, details.pass);
  else run.fail(name, details.fail);
}

/** Version parity (warn-only: partial rollouts are a real, deploy-owned state). */
function assertVersionParity(run: Run, versions: string): void {
  // bash: tr ' ' '\n' | cut -d= -f2 | sort -u | drop-empties | wc -l
  const distinct = new Set(
    versions
      .split(' ')
      .filter(tok => tok !== '')
      .map(tok => tok.split('=')[1] ?? '')
      .filter(v => v !== '')
  );
  if (distinct.size <= 1) {
    // bash `${versions# }` strips exactly ONE leading space.
    run.pass(
      A.meshVersionParity,
      `all reachable pods on one binary version (${versions.replace(/^ /, '')})`
    );
  } else {
    // bash interpolates $versions unstripped here — that leading space is
    // what puts a space after the colon.
    run.warn(
      A.meshVersionParity,
      `version skew across pods:${versions} — partial rollout? (DHT-partition risk class)`
    );
  }
}

/** Doorway view — informational only, never an assertion (bash:220-225). */
async function noteDoorwayHealth(run: Run): Promise<void> {
  if (!run.ctx.env['INTERNAL_DOORWAY_URL']) return;
  const health = await run.call(async () => run.doorway().getHealth());
  if (!health.ok) return;
  const h = asObject(health.value);
  const p2p = asObject(h['p2p']);
  const cond = asObject(h['conductor']);
  run.note(
    `doorway: p2p.peerCount=${jqAlt(p2p['peerCount'], '?')} ` +
      `pools=${jqAlt(cond['pools_healthy'], '?')}/${jqAlt(cond['pools_total'], '?')}`
  );
}

async function cmdMesh(run: Run): Promise<RunResult> {
  const env = run.ctx.env;
  const minReachable = numEnv(env['MESH_MIN_REACHABLE'], 2);
  const minConnected = numEnv(env['MESH_MIN_CONNECTED'], 1);
  const sourcePeer = env['SOURCE_PEER'] || 'matthew';
  const targetPeer = env['TARGET_PEER'] || 'jessica';

  run.ctx.log(HR);
  run.ctx.log('VERIFY PEER MESH — per-pod libp2p state, adjacency, parity');
  run.ctx.log(HR);

  const urls = run.displayUrls();
  const peerIds = new Map<string, string>();
  const peersJson: Record<string, unknown>[] = [];
  let reachable = 0;
  let versions = '';

  // parity: sequential per-peer iteration, exactly as the bash `for` loop.
  // Future improvement (deliberately NOT taken here — parity first): the
  // per-peer fan-out is independent and could run under Promise.all, which
  // would cut mesh/delivery/projection wall-clock roughly N-fold. It would
  // also reorder the console transcript and the `versions` accumulator, so it
  // needs its own parity pass against archived bash artifacts.
  for (const { name, client } of run.fleet().peers()) {
    const probe = await probeMeshPeer(run, name, client, urls.get(name) ?? '', minConnected);
    if (!probe.reachable) continue;
    reachable++;
    peerIds.set(name, probe.peerId);
    if (probe.row) peersJson.push(probe.row);
    versions = `${versions} ${name}=${probe.version}`;
  }

  if (reachable >= minReachable) {
    run.pass(A.meshReachable, `${reachable} peer pod(s) reachable (floor ${minReachable})`);
  } else {
    run.fail(A.meshReachable, `only ${reachable} peer pod(s) reachable; floor is ${minReachable}`);
  }

  // Adjacency: the propagation source must list the target in its live peer set
  // — precondition for the cross-pod fetch (heal-on-read races CONNECTED peers).
  const tgtId = peerIds.get(targetPeer) ?? '';
  await assertAdjacency(run, A.meshAdjacency, run.fleet().peer(sourcePeer), tgtId, {
    pass: `${sourcePeer} lists ${targetPeer} (${tgtId}) in its connected peer set`,
    fail: `${sourcePeer} does NOT list ${targetPeer} (${tgtId}) — cross-pod fetch will starve`,
    warn: `skipped — ${sourcePeer} or ${targetPeer} unreachable/unidentified`,
  });

  // Adjacency, reverse direction (Workstream C: bidirectional or it doesn't
  // count). The TARGET must also list the SOURCE: projection-reconcile
  // discovery and reverse heals run from the target's side, and a one-way
  // edge starves them quietly.
  const srcId = peerIds.get(sourcePeer) ?? '';
  await assertAdjacency(run, A.meshAdjacencyReverse, run.fleet().peer(targetPeer), srcId, {
    pass: `${targetPeer} lists ${sourcePeer} (${srcId}) in its connected peer set`,
    fail:
      `${targetPeer} does NOT list ${sourcePeer} (${srcId}) — reconcile discovery on ` +
      `${targetPeer} cannot reach the row/blob holder`,
    warn: `skipped — ${targetPeer} or ${sourcePeer} unreachable/unidentified`,
  });

  if (reachable >= 1) assertVersionParity(run, versions);
  await noteDoorwayHealth(run);

  return run.finish('mesh', { peers: peersJson });
}

// ===========================================================================
// upload — content blob + build-unique propagation probe blob (bash:233-292)
// ===========================================================================

interface UploadOutcome {
  hash: string;
  size: number;
}

async function uploadOne(run: Run, data: Uint8Array, label: string): Promise<UploadOutcome> {
  const size = data.byteLength;
  const hash = `sha256-${sha256Hex(data)}`;
  // Bearer-gate (jenkins-seed-bearer-gate plan, Task 1): non-dev doorway requires
  // an Admin bearer on PUT /admin/seed/blob. The Jenkinsfile mints SEED_DOORWAY_TOKEN
  // and exports it; dev/local runs leave it empty → NO Authorization header at all
  // (the facade omits it when `token` is undefined) → dev-mode doorway accepts.
  const token = run.ctx.env['SEED_DOORWAY_TOKEN'] || undefined;

  let result: SeedBlobResult;
  try {
    result = await run.doorway().seedBlob({ hash, data, token });
  } catch (e) {
    // parity: curl transport failure yields status "000" in bash.
    result = {
      ok: false,
      status: Number(statusFromError(e)),
      alreadyPresent: false,
      errorBody: '',
    };
  }

  const { status } = result;
  if (status === 200 || status === 201) {
    run.pass(`upload.${label}`, `${hash} (${size} bytes) → HTTP ${status}`);
  } else if (status === 409 || result.alreadyPresent) {
    run.pass(`upload.${label}`, `${hash} (${size} bytes) → HTTP 409 (already present, idempotent)`);
  } else {
    run.fail(`upload.${label}`, `HTTP ${status} — ${headBytes(result.errorBody ?? '', 300)}`);
  }
  return { hash, size };
}

/**
 * Build-unique probe blob: its hash has never existed before this build, so
 * the downstream cross-pod fetch can only be satisfied by live propagation.
 */
function buildProbeBlob(env: NodeJS.ProcessEnv, buildStartIso: string): Buffer {
  const build = env['BUILD_TAG'] || env['BUILD_NUMBER'] || 'local';
  const commit = env['GIT_COMMIT'] || 'unknown';
  return Buffer.from(
    'elohim substrate propagation probe\n' +
      `build: ${build}\n` +
      `commit: ${commit}\n` +
      `generated: ${buildStartIso}\n` +
      `nonce: ${randomBytes(16).toString('hex')}\n`,
    'utf8'
  );
}

async function cmdUpload(run: Run): Promise<RunResult> {
  const env = run.ctx.env;
  const contentPath = env['CONTENT_PATH'] || 'genesis/docs/content/elohim-protocol/manifesto.md';

  run.ctx.log(HR);
  run.ctx.log('UPLOAD BLOB-BACKED CONTENT — genesis peer ingest via doorway');
  run.ctx.log(HR);

  if (!env['INTERNAL_DOORWAY_URL']) {
    run.fail(A.uploadPreflight, 'INTERNAL_DOORWAY_URL unset');
    return run.finish('upload');
  }

  // bash writes build_start_iso BEFORE the content-file check — a missing
  // content file still leaves a usable build-start timestamp behind.
  const buildStartIso = isoUtcSeconds();
  run.writeState('build_start_iso', buildStartIso);

  const absContent = resolve(run.ctx.cwd, contentPath);
  if (!existsSync(absContent)) {
    run.fail(A.uploadContent, `${contentPath} missing`);
    return run.finish('upload');
  }

  const contentUp = await uploadOne(run, readFileSync(absContent), 'content');
  run.writeState('content_blob_hash', contentUp.hash);
  run.writeState('content_blob_size', String(contentUp.size));

  const probeBytes = buildProbeBlob(env, buildStartIso);
  // bash keeps the probe on disk at $STATE_DIR/propagation-probe.txt.
  writeFileSync(join(run.stateDir, 'propagation-probe.txt'), probeBytes);

  const probeUp = await uploadOne(run, probeBytes, 'probe');
  run.writeState('probe_blob_hash', probeUp.hash);
  run.writeState('probe_blob_size', String(probeUp.size));
  run.note(
    'probe blob is build-unique — a fossil copy on the target is impossible by construction'
  );

  return run.finish('upload', {
    contentBlobHash: contentUp.hash,
    probeBlobHash: probeUp.hash,
    buildStartIso,
  });
}

// ===========================================================================
// propagation — cross-pod distribution via the p2p data plane (bash:297-401)
// ===========================================================================

interface PropagationTargets {
  ok: boolean;
  src?: StorageClientLike;
  tgt?: StorageClientLike;
  contentHash: string;
  probeHash: string;
}

/** All four preflight gates (bash:307-310); records at most one failure. */
function propagationPreflight(
  run: Run,
  sourcePeer: string,
  targetPeer: string
): PropagationTargets {
  const env = run.ctx.env;
  // Env override wins; the STATE_DIR file is the cross-stage fallback.
  const contentHash = env['CONTENT_BLOB_HASH'] || run.readState('content_blob_hash');
  const probeHash = env['PROBE_BLOB_HASH'] || run.readState('probe_blob_hash');
  const blocked = { ok: false, contentHash, probeHash };

  if (!contentHash) {
    run.fail(A.propagationPreflight, 'CONTENT_BLOB_HASH unset');
    return blocked;
  }
  if (!probeHash) {
    run.fail(A.propagationPreflight, 'PROBE_BLOB_HASH unset');
    return blocked;
  }
  const src = run.fleet().peer(sourcePeer);
  if (!src) {
    run.fail(A.propagationPreflight, `no PEER_STORAGE_URLS entry for ${sourcePeer}`);
    return blocked;
  }
  const tgt = run.fleet().peer(targetPeer);
  if (!tgt) {
    run.fail(A.propagationPreflight, `no PEER_STORAGE_URLS entry for ${targetPeer}`);
    return blocked;
  }
  return { ok: true, src, tgt, contentHash, probeHash };
}

/**
 * Manifest layer + source custody (bash:314-328): a custody-blob commitment
 * for the content blob must exist (three-surface model: manifest / reality /
 * diff), and the source must actually hold both blobs.
 */
async function assertSourceHoldsBlobs(
  run: Run,
  src: StorageClientLike,
  sourcePeer: string,
  contentHash: string,
  probeHash: string
): Promise<void> {
  const commitments = await run.call(async () => src.dataplane.listCommitments({ limit: 200 }));
  if (commitments.ok && hasCustodyBlobFor(commitments.value, contentHash)) {
    run.pass(
      A.propagationCustodyManifest,
      `custody-blob commitment row present for ${contentHash} on ${sourcePeer}`
    );
  } else {
    run.fail(
      A.propagationCustodyManifest,
      `no custody-blob commitment for ${contentHash} on ${sourcePeer} — ` +
        `Seed Custody Commitments leg broken`
    );
  }

  // GET, not HEAD — head/get asymmetry.
  const contentStatus = await run.blobStatus(src, contentHash);
  if (contentStatus === '200') {
    run.pass(A.propagationSourceContent, `${sourcePeer} serves content blob`);
  } else {
    run.fail(
      A.propagationSourceContent,
      `${sourcePeer} GET content blob → ${contentStatus} (upload leg broken)`
    );
  }

  const probeStatus = await run.blobStatus(src, probeHash);
  if (probeStatus === '200') {
    run.pass(A.propagationSourceProbe, `${sourcePeer} serves probe blob`);
  } else {
    run.fail(
      A.propagationSourceProbe,
      `${sourcePeer} GET probe blob → ${probeStatus} (upload leg broken)`
    );
  }
}

/** Reality-layer baseline on the target (bash:331-336); null ⇒ unprovable. */
async function readParityBaseline(
  run: Run,
  tgt: StorageClientLike,
  targetPeer: string
): Promise<number | null> {
  const parity = await run.call(async () => tgt.dataplane.getInventoryParity());
  const fsBefore =
    parity.ok && typeof parity.value.filesystemCount === 'number'
      ? parity.value.filesystemCount
      : null;
  if (fsBefore === null) {
    run.warn(
      A.propagationParityBaseline,
      `inventory-parity unavailable on ${targetPeer} (older image?) — ` +
        `byte-persistence delta unprovable this run`
    );
  } else {
    run.note(`${targetPeer} filesystem_count before: ${fsBefore}`);
  }
  return fsBefore;
}

interface ProbePoll {
  status: string;
  waited: number;
  attempts: number;
}

/**
 * The probe fetch: poll — each GET arms heal-on-read; the custody sweep tick
 * (120s) and inventory gossip need real time after a fresh upload.
 *
 * `waited` is the FIRST half of a SINGLE budget shared with the
 * custody-convergence poll below (bash reuses the same variable), so the
 * second poll gets the REMAINDER, never a fresh PROPAGATION_TIMEOUT_SECS.
 */
async function pollProbeFetch(
  run: Run,
  tgt: StorageClientLike,
  probeHash: string,
  timeoutSecs: number,
  pollSecs: number
): Promise<ProbePoll> {
  let waited = 0;
  let attempts = 0;
  let status = '000';
  while (waited <= timeoutSecs) {
    attempts++;
    status = await run.blobStatus(tgt, probeHash);
    if (status === '200') break;
    // bash sleeps even on the final iteration, so a failed poll reports
    // `waited` one poll interval PAST the timeout (e.g. 315s at a 300s budget).
    await run.ctx.sleep(pollSecs);
    waited += pollSecs;
  }
  return { status, waited, attempts };
}

/**
 * Manifest convergence: the custody-blob commitment for the content blob must
 * be visible in EVERY reachable pod's projection — not just the pod that
 * received the POST. This is the DHT-leg assertion (anchor → conductor gossip
 * → projection_reconcile). Consumes whatever remains of the shared budget.
 */
async function pollCustodyConvergence(
  run: Run,
  contentHash: string,
  timeoutSecs: number,
  pollSecs: number,
  waitedIn: number
): Promise<{ missing: string[]; waited: number }> {
  let waited = waitedIn;
  let missing: string[] = [];
  for (;;) {
    missing = [];
    for (const { name, client } of run.fleet().peers()) {
      const res = await run.call(async () =>
        client.dataplane.listCommitments({ action: 'custody-blob', limit: 200 })
      );
      if (!(res.ok && hasCustodyBlobFor(res.value, contentHash))) missing.push(name);
    }
    if (missing.length === 0 || waited >= timeoutSecs) break;
    await run.ctx.sleep(pollSecs);
    waited += pollSecs;
  }
  return { missing, waited };
}

/** Reality layer: bytes persisted, not just streamed (bash:360-368). */
async function assertBytesPersisted(
  run: Run,
  tgt: StorageClientLike,
  targetPeer: string,
  fsBefore: number,
  probeStatus: string
): Promise<void> {
  const parityAfter = await run.call(async () => tgt.dataplane.getInventoryParity());
  const fsAfter =
    parityAfter.ok && typeof parityAfter.value.filesystemCount === 'number'
      ? parityAfter.value.filesystemCount
      : null;
  if (fsAfter !== null && fsAfter > fsBefore) {
    run.pass(
      A.propagationBytesPersisted,
      `${targetPeer} filesystem_count ${fsBefore} → ${fsAfter} (replica on disk)`
    );
  } else if (probeStatus === '200') {
    run.warn(
      A.propagationBytesPersisted,
      `probe served but filesystem_count did not grow (${fsBefore} → ${fsAfter ?? '?'}) — ` +
        `streamed-not-persisted, or parity lag`
    );
  }
  // else: bash records nothing.
}

async function cmdPropagation(run: Run): Promise<RunResult> {
  const env = run.ctx.env;
  const sourcePeer = env['SOURCE_PEER'] || 'matthew';
  const targetPeer = env['TARGET_PEER'] || 'jessica';
  const timeoutSecs = numEnv(env['PROPAGATION_TIMEOUT_SECS'], 300);
  const pollSecs = numEnv(env['PROPAGATION_POLL_SECS'], 15);

  run.ctx.log(HR);
  run.ctx.log(
    `VERIFY SUBSTRATE PROPAGATION — ${targetPeer} must serve blobs staged on ${sourcePeer}`
  );
  run.ctx.log(HR);

  const t = propagationPreflight(run, sourcePeer, targetPeer);
  if (!t.ok || !t.src || !t.tgt) return run.finish('propagation');
  const { src, tgt, contentHash, probeHash } = { ...t, src: t.src, tgt: t.tgt };

  await assertSourceHoldsBlobs(run, src, sourcePeer, contentHash, probeHash);
  const fsBefore = await readParityBaseline(run, tgt, targetPeer);

  const probe = await pollProbeFetch(run, tgt, probeHash, timeoutSecs, pollSecs);
  if (probe.status === '200') {
    run.pass(
      A.propagationProbeFetch,
      `${targetPeer} served build-unique probe blob after ${probe.waited}s / ` +
        `${probe.attempts} attempt(s) — p2p data plane moved bytes THIS build`
    );
  } else {
    run.fail(
      A.propagationProbeFetch,
      `${targetPeer} GET probe blob → ${probe.status} after ${probe.waited}s / ` +
        `${probe.attempts} attempts — substrate did not propagate ` +
        `(check inventory gossip + heal-on-read race + adjacency)`
    );
  }

  // Content blob on the target (may be served from a prior build's replica —
  // that's persistence, also worth proving; the probe above is the live test).
  const s = await run.blobStatus(tgt, contentHash);
  if (s === '200') {
    run.pass(
      A.propagationContentFetch,
      `${targetPeer} serves content blob (live or persisted replica)`
    );
  } else {
    run.fail(A.propagationContentFetch, `${targetPeer} GET content blob → ${s}`);
  }

  if (fsBefore !== null) {
    await assertBytesPersisted(run, tgt, targetPeer, fsBefore, probe.status);
  }

  const custody = await pollCustodyConvergence(
    run,
    contentHash,
    timeoutSecs,
    pollSecs,
    probe.waited
  );
  if (custody.missing.length === 0) {
    run.pass(
      A.propagationCustodyConvergence,
      `custody-blob commitment for ${contentHash} visible on every pod ` +
        `(manifest converged by ${custody.waited}s)`
    );
  } else {
    // bash's $manifest_missing accumulates " $name" per pod, so the rendered
    // detail reads "missing on: jessica james".
    const missingList = custody.missing.map(n => ` ${n}`).join('');
    run.fail(
      A.propagationCustodyConvergence,
      `custody-blob commitment for ${contentHash} missing on:${missingList} ` +
        `after ${custody.waited}s — DHT leg (anchor → conductor gossip → projection_reconcile) ` +
        `not converging; correlate per-peer 'ProjectionInventory: serving local inventory' ` +
        `vs 'peer inventory received' Loki lines`
    );
  }

  return run.finish('propagation', {
    contentBlobHash: contentHash,
    probeBlobHash: probeHash,
    waitedSecs: custody.waited,
    attempts: probe.attempts,
    custodyManifestMissingOn: custody.missing,
  });
}

// ===========================================================================
// delivery — serve-blob REA events emitted for this build (bash:406-459)
// ===========================================================================

/**
 * Synthetic post-propagation heal (genesis #1182 Cluster F). A serve-blob
 * EconomicEvent is emitted ONLY when a peer actually FETCHES a blob
 * (blob_fetch's atomic pair), so a 0-event read may be incidental rather than
 * a regression. Force the proof: GET the probe blob from each peer (a real
 * heal-on-read GET, never HEAD) — a peer that lacks it fetches cross-pod and
 * the SERVING peer emits the serve-blob event NOW, inside the after= window.
 */
async function syntheticHeal(run: Run, probe: string, settleSecs: number): Promise<void> {
  run.ctx.log(
    `   synthetic heal: GET probe blob ${probe} from each peer to force a serve-blob emit`
  );
  for (const { name, client } of run.fleet().peers()) {
    run.note(`${name}: heal GET /blob/${probe} → ${await run.blobStatus(client, probe)}`);
  }
  // heal-on-read + the atomic event write need a moment to project.
  await run.ctx.sleep(settleSecs);
}

async function countServeBlobEvents(
  run: Run,
  after: string
): Promise<{ perPeer: Record<string, number>; total: number }> {
  const perPeer: Record<string, number> = {};
  let total = 0;
  for (const { name, client } of run.fleet().peers()) {
    let count = 0;
    const res = await run.call(async () =>
      client.dataplane.listEconomicEvents({ action: 'serve-blob', after, limit: 50 })
    );
    if (res.ok) {
      count = Array.isArray(res.value) ? res.value.length : 0;
    } else {
      run.warn(`delivery.${name}`, `economic-events query unreachable/non-200 (${run.lastStatus})`);
    }
    perPeer[name] = count;
    run.note(`${name}: ${count} serve-blob event(s) since ${after}`);
    total += count;
  }
  return { perPeer, total };
}

async function cmdDelivery(run: Run): Promise<RunResult> {
  const env = run.ctx.env;
  const after = env['SUBSTRATE_BUILD_START_ISO'] || run.readState('build_start_iso');
  const minEvents = numEnv(env['DELIVERY_MIN_EVENTS'], 1);

  run.ctx.log(HR);
  run.ctx.log(`VERIFY DELIVERY EVENTS — serve-blob REA events since ${after}`);
  run.ctx.log(HR);

  if (!after) {
    run.fail(A.deliveryPreflight, 'no build-start timestamp (upload stage skipped?)');
    return run.finish('delivery');
  }

  // parity: bash reads PROBE_BLOB_HASH from the ENVIRONMENT ONLY here
  // (`local probe="${PROBE_BLOB_HASH:-}"`, bash:425) — unlike cmd_propagation
  // it does NOT fall back to $STATE_DIR/probe_blob_hash. Reproduced as-is.
  const probe = env['PROBE_BLOB_HASH'] || '';
  if (probe) {
    await syntheticHeal(run, probe, numEnv(env['DELIVERY_HEAL_SETTLE_SECS'], 12));
  } else {
    run.warn(
      A.deliveryProbe,
      'PROBE_BLOB_HASH unset — cannot force a synthetic heal; counting incidental events only'
    );
  }

  const { perPeer, total } = await countServeBlobEvents(run, after);

  if (total >= minEvents) {
    run.pass(
      A.deliveryServeBlobEvents,
      `${total} serve-blob event(s) across peers — transfers leave an REA delivery trail`
    );
  } else {
    // parity: bash hardcodes "0 serve-blob events" in the failure detail even
    // when total > 0 with a raised DELIVERY_MIN_EVENTS floor. Reproduced.
    const despite = probe ? ` despite a forced synthetic heal of ${probe}` : '';
    const ruledOut = probe
      ? " (a real fetch was forced, so 'no transfer happened' is ruled out)"
      : '';
    run.fail(
      A.deliveryServeBlobEvents,
      `0 serve-blob events since ${after}${despite} — the event-emission leg ` +
        `(blob_fetch atomic pair) regressed${ruledOut}`
    );
  }

  return run.finish('delivery', { perPeer, after });
}

// ===========================================================================
// projection — projector cursors + sync streams caught up (bash:464-521)
// ===========================================================================

type LagEntry = Record<string, unknown>;

/**
 * bash: `[.lag[]? | select(.lagSeconds != null and .lagSeconds > $m)]`
 * `parseable: false` means jq would have ERRORED (non-object body) — the
 * caller then skips silently, matching bash.
 */
function laggyCursors(body: unknown, maxLag: number): { parseable: boolean; entries: LagEntry[] } {
  // jq: `null | .lag[]?` → empty, no error.
  if (body === null || body === undefined) return { parseable: true, entries: [] };
  // jq: `.lag` on an array/string/number → error.
  if (!isRecord(body)) return { parseable: false, entries: [] };
  const lag = body['lag'];
  // jq `[]?` swallows "not iterable".
  if (!Array.isArray(lag)) return { parseable: true, entries: [] };
  const entries = (lag as LagEntry[]).filter(entry => {
    const v = asObject(entry)['lagSeconds'];
    return typeof v === 'number' && v > maxLag;
  });
  return { parseable: true, entries };
}

async function assertProjectorLag(
  run: Run,
  name: string,
  client: StorageClientLike,
  maxLag: number
): Promise<void> {
  const projector = await run.call(async () => client.dataplane.getProjectorStatus());
  if (!projector.ok) {
    run.warn(
      `projection.${name}.projector`,
      `status/projector unavailable (${run.lastStatus}) — pod-direct route, older image?`
    );
    return;
  }
  const { parseable, entries } = laggyCursors(projector.value, maxLag);
  // parity: bash skips silently on parse failure. `jq '[.lag[]?|…]'` errors on
  // a non-object body, the `|| echo ""` leaves $laggy empty, and NEITHER
  // branch records an assertion. Reproduced: no assertion, no output.
  if (!parseable) return;
  if (entries.length === 0) {
    run.pass(`projection.${name}.lag`, `no cursor lag > ${maxLag}s`);
  } else {
    run.fail(
      `projection.${name}.lag`,
      `${entries.length} cursor(s) lag > ${maxLag}s: ${JSON.stringify(entries)}`
    );
  }
}

/** One /p2p/status read rendered as bash's `detail` line. */
function streamDetail(run: Run, body: unknown): { detail: string; ok: boolean } {
  // parity: replication keeps bash's RAW `.replication.caughtUp //
  // .replication.caught_up // empty` rather than streamSyncState — bash
  // renders an ABSENT replication block as the empty string (not "null"), and
  // that empty rendering is load-bearing in the detail line. The two
  // tri-state streams below use the facade helper.
  const repl = firstString(asObject(asObject(body)['replication']), 'caughtUp', 'caught_up');
  // pull tri-state: acquisition rollup() is spec-R-A "never false-complete" —
  // caught_up = total>0 && fetched==total, so total==0 (NO acquisition
  // workload) reports caughtUp=false BY DESIGN. But total==0 cannot
  // distinguish resolved-empty from an acquisition loop that never registered
  // pins — so "idle" is WARN-class, never a clean pass.
  //
  // FAIL-CLOSED PARITY: bash renders a stream object that is PRESENT but
  // carries no `caughtUp`/`caught_up` field as `false` (`// false | tostring`)
  // — a FAIL; its own note says "a schema rename must fail closed via the
  // caughtUp path". The facade's `streamSyncState` collapses that shape into
  // 'not-computable' (same word as stream-absent), which would degrade a live
  // field RENAME to warn-class. Restore bash's distinction locally: a stream
  // key that EXISTS but yields 'not-computable' renders "false" (fail-closed),
  // while a genuinely absent/null stream keeps "null" (warn — not computable).
  const pull = streamWord(failClosed(body, 'pull', run.ctx.facade.streamSyncState(body, 'pull')));
  const projr = streamWord(
    failClosed(
      body,
      'projectionReconcile',
      run.ctx.facade.streamSyncState(body, 'projectionReconcile')
    )
  );
  return {
    detail: `replication=${repl} pull=${pull} projection_reconcile=${projr}`,
    ok: repl === 'true' && pull !== 'false' && projr !== 'false',
  };
}

/** Sync streams: retry briefly — right after seeding the drain is legitimate. */
async function assertSyncStreams(
  run: Run,
  name: string,
  client: StorageClientLike,
  retries: number
): Promise<void> {
  let attempt = 0;
  let ok = false;
  let detail = '';
  while (attempt < retries) {
    attempt++;
    const statusRes = await run.call(async () => client.dataplane.getP2pStatus());
    if (statusRes.ok) {
      const rendered = streamDetail(run, statusRes.value);
      detail = rendered.detail;
      if (rendered.ok) {
        ok = true;
        break;
      }
    } else {
      detail = `p2p/status unreachable (${run.lastStatus})`;
    }
    // bash sleeps 10s at the end of every non-breaking iteration, including
    // the last one before the loop condition fails.
    await run.ctx.sleep(10);
  }

  const assertionName = `projection.${name}.streams`;
  if (!ok) {
    run.fail(assertionName, `${detail} after ${retries} attempt(s)`);
  } else if (detail.includes('null')) {
    // bash `case "$detail"` — null is tested BEFORE idle.
    run.warn(
      assertionName,
      `${detail} — null streams are 'not computable', NEVER caught-up (spec §4.3); ` +
        `not failing, but not proven`
    );
  } else if (detail.includes('idle')) {
    run.warn(
      assertionName,
      `${detail} — pull total==0: resolved-empty OR an acquisition loop that never ` +
        `registered pins; not failing, but not proven (cross-check seeded pin expectations)`
    );
  } else {
    run.pass(assertionName, detail);
  }
}

async function cmdProjection(run: Run): Promise<RunResult> {
  const env = run.ctx.env;
  const maxLag = numEnv(env['PROJECTION_MAX_LAG_SECS'], 120);
  const retries = numEnv(env['PROJECTION_RETRIES'], 3);

  run.ctx.log(HR);
  run.ctx.log('VERIFY PROJECTION SYNC — cursors + replication/pull streams');
  run.ctx.log(HR);

  for (const { name, client } of run.fleet().peers()) {
    await assertProjectorLag(run, name, client, maxLag);
    await assertSyncStreams(run, name, client, retries);
  }

  return run.finish('projection');
}

// ===========================================================================
// federation — doorway membership + p2p bootstrap surface (bash:526-565)
// ===========================================================================

async function assertFederationDoorways(run: Run, doorway: DoorwayLike): Promise<void> {
  const res = await run.call(async () => doorway.listFederationDoorways());
  if (!res.ok) {
    run.fail(A.federationDoorways, `GET /api/v1/federation/doorways → ${run.lastStatus}`);
    return;
  }
  // parity: `jq -r '.doorways | length' || echo 0` — a null/absent `.doorways`
  // errors in jq and falls back to 0.
  const raw = asObject(res.value)['doorways'];
  const list = Array.isArray(raw) ? raw : [];
  const n = list.length;
  const online = list.filter(d => asObject(d)['status'] === 'online').length;
  if (online >= 1) run.pass(A.federationDoorways, `${n} doorway(s) known, ${online} online`);
  else run.fail(A.federationDoorways, `${n} doorway(s) known but none online`);

  // Cross-doorway breadth needs shem (@requires:shem) — report, don't assert.
  if (n >= 2) run.note(`cross-doorway breadth present (${n}) — shem-dependent, informational`);
  else run.note('single-doorway federation (cross-doorway breadth needs shem)');
}

async function assertFederationP2pPeers(run: Run, doorway: DoorwayLike): Promise<void> {
  const res = await run.call(async () => doorway.listFederationP2pPeers());
  if (!res.ok) {
    run.fail(A.federationP2pPeers, `GET /api/v1/federation/p2p-peers → ${run.lastStatus}`);
    return;
  }
  const body = asObject(res.value);
  const totalRaw = body['total'];
  // parity: jq's `.total >= 1` uses jq's total ordering, where ANY string
  // sorts above every number. A string-typed `total` would therefore pass in
  // bash and fail here — a wire-shape corruption we prefer to surface.
  const total = typeof totalRaw === 'number' ? totalRaw : Number.NaN;
  if (total >= 1) {
    run.pass(A.federationP2pPeers, `bootstrap surface lists ${total} peer(s)`);
  } else {
    // bash: `jq -c '{total}'` → {"total":0} / {"total":null}
    run.fail(
      A.federationP2pPeers,
      `bootstrap surface empty: ${JSON.stringify({ total: totalRaw ?? null })}`
    );
  }

  const peers = Array.isArray(body['peers']) ? (body['peers'] as unknown[]) : [];
  const shard = peers.some(p => {
    const caps = asObject(p)['capabilities'];
    return Array.isArray(caps) && caps.includes('shard');
  });
  if (shard) {
    run.pass(
      A.federationShardCapability,
      'at least one advertised peer carries the shard capability'
    );
  } else {
    run.warn(A.federationShardCapability, 'no advertised peer carries shard capability');
  }
}

async function cmdFederation(run: Run): Promise<RunResult> {
  run.ctx.log(HR);
  run.ctx.log('VERIFY FEDERATION LAYER — doorway membership + bootstrap surface');
  run.ctx.log(HR);

  if (!run.ctx.env['INTERNAL_DOORWAY_URL']) {
    run.fail(A.federationPreflight, 'INTERNAL_DOORWAY_URL unset');
    return run.finish('federation');
  }
  const doorway = run.doorway();
  await assertFederationDoorways(run, doorway);
  await assertFederationP2pPeers(run, doorway);

  return run.finish('federation');
}

// ===========================================================================
// resilience — conductor-signal-driven posture (bash:570-604)
// ===========================================================================

async function assertNetworkPosture(
  run: Run,
  name: string,
  client: StorageClientLike
): Promise<void> {
  const posture = await run.call(async () => client.dataplane.getNetworkPosture());
  // bash records nothing when /api/v1/network/posture is unreachable.
  if (!posture.ok) return;

  const p = asObject(posture.value);
  const active = jqAlt(p['activePeers'], '0');
  if (atLeast(active, 1)) {
    const summary = JSON.stringify({
      totalPeers: p['totalPeers'] ?? null,
      stalePeers: p['stalePeers'] ?? null,
      householdsReciprocating: p['householdsReciprocating'] ?? null,
    });
    run.pass(`resilience.${name}.posture`, `activePeers=${active} ${summary}`);
  } else {
    // parity: bash hardcodes "activePeers=0:" here regardless of the read
    // value (a non-numeric activePeers makes the `-ge` test error out and
    // fall into this branch with the literal text). `head -c 200` bytes.
    run.fail(
      `resilience.${name}.posture`,
      `activePeers=0: ${headBytes(JSON.stringify(posture.value), 200)}`
    );
  }
}

async function cmdResilience(run: Run): Promise<RunResult> {
  run.ctx.log(HR);
  run.ctx.log('VERIFY RESILIENCE SIGNALS — peer-status projections + network posture');
  run.ctx.log(HR);

  // Stage-level `when` gates on CONDUCTOR_SEEDING_READY: these projections are
  // written ONLY by conductor-originated PeerStatusRecorded signals, so before
  // conductor seeding runs they are empty BY DESIGN.
  //
  // parity: this loop checks ONLY the FIRST peer that answers, then STOPS —
  // one healthy pod's projection answers for the fleet view. It is NOT a
  // fan-out aggregate.
  let checked = false;
  for (const { name, client } of run.fleet().peers()) {
    const statuses = await run.call(async () => client.dataplane.listPeerStatuses());
    if (!statuses.ok) continue;
    checked = true;

    const n = Array.isArray(statuses.value) ? statuses.value.length : 0;
    if (n >= 1) {
      run.pass(`resilience.${name}.peer-statuses`, `${n} peer-status record(s) projected`);
    } else {
      run.fail(
        `resilience.${name}.peer-statuses`,
        'no peer-status records — PeerStatusRecorded signals not flowing despite conductor seeding'
      );
    }

    await assertNetworkPosture(run, name, client);
    break; // one healthy pod's projection answers for the fleet view
  }
  if (!checked) run.fail(A.resilienceReachable, 'no peer pod answered /api/v1/peer-statuses');

  return run.finish('resilience');
}

// ---------------------------------------------------------------------------
// entry points
// ---------------------------------------------------------------------------

const HANDLERS: Record<Command, (run: Run) => Promise<RunResult>> = {
  mesh: cmdMesh,
  upload: cmdUpload,
  propagation: cmdPropagation,
  delivery: cmdDelivery,
  projection: cmdProjection,
  federation: cmdFederation,
  resilience: cmdResilience,
};

export function isCommand(v: string): v is Command {
  return (COMMANDS as readonly string[]).includes(v);
}

/** Run one subcommand against an injected facade. Exported for the selftest. */
export async function runCommand(command: Command, ctx: RunContext): Promise<RunResult> {
  return HANDLERS[command](new Run(ctx));
}

export async function defaultSleep(seconds: number): Promise<void> {
  await new Promise(res => setTimeout(res, Math.max(0, seconds) * 1000));
}

/** Bind the real `@elohim/storage-client` facade at runtime. */
export async function defaultFacade(): Promise<Facade> {
  const mod = (await import('@elohim/storage-client')) as unknown as {
    DataplaneFleet: {
      fromCsv(csv: string, opts: { hAppId?: string; timeout?: number }): FleetLike;
    };
    DoorwayClient: new (opts: { baseUrl: string }) => DoorwayLike;
    streamSyncState: (raw: unknown, stream: StreamName) => StreamState;
  };
  return {
    fleetFromCsv: (csv, opts) => mod.DataplaneFleet.fromCsv(csv, opts),
    makeDoorway: baseUrl => new mod.DoorwayClient({ baseUrl }),
    streamSyncState: mod.streamSyncState,
  };
}

async function main(): Promise<void> {
  const command = process.argv[2] ?? '';
  if (!isCommand(command)) {
    console.log(USAGE);
    process.exitCode = 2;
    return;
  }
  const { exitCode } = await runCommand(command, {
    env: process.env,
    facade: await defaultFacade(),
    sleep: defaultSleep,
    log: line => console.log(line),
    cwd: process.cwd(),
  });
  process.exitCode = exitCode;
}

// Run only when invoked directly (not when imported by the selftest).
if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch(e => {
    console.error(e instanceof Error ? e.message : String(e));
    process.exitCode = 2;
  });
}
