/**
 * Household-mesh chaos, recovery and heal-on-read drills (Act I).
 *
 * Wires three destructive feature files against a mesh THIS RUN OWNS:
 *   - features/resilience/chaos-peer-churn.feature
 *   - features/federation/peer-recovery.feature
 *   - features/resilience/app-blob-heal-on-read.feature
 *
 * Process-control contract (the sharp edge in this file)
 * -----------------------------------------------------
 * Every kill is `process.kill(pid, …)` on an EXACT pid resolved through
 * `requireFixturePeerPid()`. There is deliberately no `pkill -f` / `pgrep -f`
 * anywhere here: a pattern like `elohim-storage` also matches the runner's own
 * shell and self-kills the run (2026-08-16). Restarts go back through
 * `app/elohim-app/scripts/hc-mesh.sh storage-restart <peer>`, which re-execs the
 * peer with the EXACT environment recovered from `/proc/<pid>/environ` — never
 * a hand-rolled re-exec, because a peer re-keyed by a chaos drill runs on an
 * AGENT_PUBKEY this suite has no way to know. Before killing, we copy that
 * environ into the same workdir `storage-restart` reads, so a peer killed here
 * is always restartable even though its /proc entry is gone by then.
 *
 * Every destructive step is held behind `A2O_ALLOW_DESTRUCTIVE=1` and answers
 * `skipped` when the switch is off (never `pending` — cucumber-js is strict and
 * a pending step reds the run exactly like a failure). Read-only probes are
 * never gated. `@requires:owned-substrate` is NOT that gate: it is declared
 * available in the Act I lane contract, so it says which substrate the drill
 * needs, not whether we may fire it today.
 *
 * Every destructive step registers an `onCleanup` restore, and the chaos file
 * additionally has a tag-scoped `After` hook, so a mid-scenario failure leaves
 * the mesh the way we found it: killed peers come back through the mesh script
 * and wiped blob bytes are copied back from the sidelined plane.
 *
 * Local-store probes read the FILESYSTEM, never `GET /blob/{hash}`.
 * `get_blob_or_heal` means an HTTP blob read HEALS on a local miss — probing
 * "is this peer missing the bytes" over HTTP would repair the very condition
 * the drill just created, and probing "does it hold them again" over HTTP would
 * be self-fulfilling. `elohim-storage` stores a blob at
 * `$STORAGE_DIR/blobs/blobs/<first-4-hex>/sha256-<hex>` (BlobStore::blob_path
 * under Config::blobs_dir), and `STORAGE_DIR` is read per peer from its own
 * `/proc/<pid>/environ`.
 */

import { strict as assert } from 'node:assert';
import { execFile } from 'node:child_process';
import { existsSync } from 'node:fs';
import { copyFile, mkdir, readdir, readFile, rename, rm } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { After, Given, Then, When } from '@cucumber/cucumber';

import {
  getRaw,
  probeContent,
  probeInventoryParity,
  probeP2PStatus,
} from '../../src/framework/dataplane/surfaces.js';
import {
  HouseholdMeshFixture,
  loadHouseholdMeshFixture,
  normalizeHouseholdFootprint,
  requireFixtureDoorwayUrl,
  requireFixturePeerPid,
  requireFixturePrimaryStorageUrl,
  requireFixtureStoragePeer,
} from '../../src/framework/fixtures/household-mesh.js';
import { retry } from '../../src/framework/utils/retry.js';
import { E2EWorld } from '../../src/framework/world.js';

const execFileAsync = promisify(execFile);

// ---------------------------------------------------------------------------
// Mesh topology + process control
// ---------------------------------------------------------------------------

const HOUSEHOLD_PEERS = ['matthew', 'jessica', 'james'] as const;
type HouseholdPeerName = (typeof HOUSEHOLD_PEERS)[number];

/**
 * Kill order for the cascade drills. `matthew` is last on purpose: doorway
 * alpha's primaryStorageUrl is matthew's, so keeping him standing is what makes
 * "the surviving peer still serves the content" a claim about custody rather
 * than about which pod the doorway happened to pick.
 */
const CASCADE_KILL_ORDER: readonly HouseholdPeerName[] = ['james', 'jessica', 'matthew'];

/**
 * hc-mesh.sh's own default (`MESH_DIR="${MESH_DIR:-/tmp/elohim-local-mesh}"`),
 * honoured through the same env var so the two agree. Only used to place the
 * `storage-restart` environ capture where that script looks for it.
 */
// Not a secret store: this is the local mesh's own working directory on a host this run
// owns, and the literal must stay byte-identical to hc-mesh.sh's default or the restart
// capture lands where that script will not look for it.
// eslint-disable-next-line sonarjs/publicly-writable-directories -- see above
const MESH_DIR = process.env['MESH_DIR'] ?? '/tmp/elohim-local-mesh';

const HERE = path.dirname(fileURLToPath(import.meta.url));
/** genesis/a2o/steps/mesh → repo root → app/elohim-app/scripts/hc-mesh.sh */
const HC_MESH_SH = path.resolve(HERE, '../../../../app/elohim-app/scripts/hc-mesh.sh');

const RESYNC_WINDOW_MS = Number(process.env['E2E_PEER_RESYNC_WINDOW_MS'] ?? 120_000);
const RECOVERY_WINDOW_MS = Number(process.env['E2E_CUSTODY_RECOVERY_WINDOW_MS'] ?? 300_000);
const SWEEP_WINDOW_MS = Number(process.env['E2E_CUSTODY_SWEEP_WINDOW_MS'] ?? 300_000);

interface PeerHandle {
  name: HouseholdPeerName;
  url: string;
  /** Transport cid as this peer reports it on /p2p/status. Measured lazily. */
  peerId?: string;
}

interface CustodyCredit {
  id: string;
  provider: string;
  blobHash: string;
}

interface WipedPlane {
  peer: HouseholdPeerName;
  liveDir: string;
  backupDir: string;
}

interface DrillState {
  peers: PeerHandle[];
  floor: number;
  /** Peers this scenario killed and has not yet restarted. */
  down: Set<HouseholdPeerName>;
  /** Single blob files moved aside, keyed `${peer}:${hex}`. */
  sidelined: Map<string, { from: string; to: string }>;
  /** Whole blob planes renamed aside. */
  wipedPlanes: WipedPlane[];
  /** Per-peer counters captured at the moment a drill perturbed the mesh. */
  baselines: Map<string, number>;
  /** Custody credits captured by "the mesh's custody record … is settled". */
  settledCredits?: CustodyCredit[];
  /** Blob hash (canonical `sha256-<hex>`) per content id resolved this run. */
  blobHashes: Map<string, string>;
  /** Bytes read from the doorway before a drill, for later byte-equality. */
  expectedBytes: Map<string, Uint8Array>;
  /** ISO timestamp the current drill started, to scope REA event queries. */
  drillStartedAt?: string;
  /** In-flight streaming read held open across a mid-transfer kill. */
  inFlight?: { contentId: string; response: Response; firstChunk: Uint8Array };
  /** Log offsets captured before a sweep, so we only read NEW log lines. */
  logOffsets: Map<HouseholdPeerName, number>;
  /** The custody-reconcile pass line captured after a sweep. */
  sweepPassLine?: string;
  /** Protection ladder declared by the feature's Background: copies → label. */
  ladder?: Map<number, string>;
}

const drillStates = new WeakMap<E2EWorld, DrillState>();

function fixture(): HouseholdMeshFixture {
  return loadHouseholdMeshFixture();
}

function drill(world: E2EWorld): DrillState {
  const existing = drillStates.get(world);
  if (existing) return existing;

  const manifest = fixture();
  const peers = HOUSEHOLD_PEERS.map(name => ({
    name,
    url: requireFixtureStoragePeer(manifest, name).url,
  }));
  const floor = manifest.connectedPeersFloor ?? peers.length - 1;
  assert.ok(
    Number.isInteger(floor) && floor >= 0,
    `household fixture connectedPeersFloor must be an integer, got ${String(floor)}`
  );

  const state: DrillState = {
    peers,
    floor,
    down: new Set(),
    sidelined: new Map(),
    wipedPlanes: [],
    baselines: new Map(),
    blobHashes: new Map(),
    expectedBytes: new Map(),
    logOffsets: new Map(),
  };
  drillStates.set(world, state);

  // After-safe restore: whatever this scenario killed or wiped goes back,
  // including on a mid-scenario failure.
  world.onCleanup(async () => {
    await restoreBlobs(state);
    await restartDownPeers(state);
  });
  return state;
}

function peerByName(state: DrillState, name: HouseholdPeerName): PeerHandle {
  const peer = state.peers.find(candidate => candidate.name === name);
  assert.ok(peer, `"${name}" is not a household mesh peer`);
  return peer;
}

/** Trim trailing slashes without a backtracking regex (sibling of the fixture's own). */
function withoutTrailingSlashes(value: string): string {
  let end = value.length;
  while (end > 0 && value[end - 1] === '/') end -= 1;
  return value.slice(0, end);
}

function doorwayUrl(world: E2EWorld, id = 'alpha'): string {
  const registered = world.doorways.get(id)?.url;
  if (registered && /^https?:\/\//.test(registered)) return withoutTrailingSlashes(registered);
  return requireFixtureDoorwayUrl(fixture(), id);
}

/** The peer doorway `id` reads from first — the "serving peer" of the app path. */
function servingPeerName(doorwayId = 'alpha'): HouseholdPeerName {
  const manifest = fixture();
  const primary = requireFixturePrimaryStorageUrl(manifest, doorwayId);
  for (const name of HOUSEHOLD_PEERS) {
    if (requireFixtureStoragePeer(manifest, name).url === primary) return name;
  }
  assert.fail(
    `doorway "${doorwayId}" primary storage ${primary} is not one of the household peers ` +
      `(${HOUSEHOLD_PEERS.join(', ')}) — this drill assumes the doorway reads from the mesh`
  );
}

// ---------------------------------------------------------------------------
// Process control — EXACT pids only, restarts through hc-mesh.sh
// ---------------------------------------------------------------------------

/**
 * The one switch that actually holds the destructive leg.
 *
 * `@requires:owned-substrate` is declared AVAILABLE in the Act I lane contract,
 * so it gates nothing on this mesh: it says which substrate the drill needs,
 * not whether we may fire it today. Off by default; every kill, restart and
 * wipe below asks first, and answers `skipped` (not `pending` — cucumber-js is
 * strict, and a pending step reds the run exactly like a failure) when held.
 * Read-only probes are never gated.
 */
function destructiveAllowed(): boolean {
  return process.env['A2O_ALLOW_DESTRUCTIVE'] === '1';
}

function holdDestructive(description: string): 'skipped' | undefined {
  if (destructiveAllowed()) return undefined;
  process.stdout.write(
    `[a2o] destructive step HELD: ${description}. ` +
      'Set A2O_ALLOW_DESTRUCTIVE=1 to let this drill touch the mesh.\n'
  );
  return 'skipped';
}

/**
 * Resolve a peer's pid AND prove it still is that peer.
 *
 * `requireFixturePeerPid` checks only `pid > 1` and `pid !== process.pid`; it
 * does not check identity, and the manifest's pids are written once by the
 * prologue. A mesh regenerated since then leaves a stale pid that the kernel
 * may have recycled onto an unrelated process — signalling it would kill
 * something that has nothing to do with this suite. So every signal here goes
 * through this: /proc/<pid>/cmdline must name elohim-storage AND carry the
 * `--http-port` this peer's manifest URL declares. Never a pattern match over
 * the process table (a `pkill -f elohim-storage` also matches the runner's own
 * shell and self-kills the run).
 */
async function verifiedPeerPid(name: HouseholdPeerName): Promise<number> {
  const manifest = fixture();
  const pid = requireFixturePeerPid(manifest, name);
  assert.ok(pid !== process.pid, 'refusing to signal the test runner itself');
  const port = new URL(requireFixtureStoragePeer(manifest, name).url).port;
  assert.ok(port, `household peer "${name}" has no port in its manifest URL`);

  let cmdline: string;
  try {
    cmdline = await readFile(`/proc/${pid}/cmdline`, 'utf8');
  } catch (error) {
    assert.fail(
      `household fixture names pid ${pid} for peer "${name}", but /proc/${pid} is unreadable ` +
        `(${String(error)}) — the manifest is stale; re-run \`just mesh prologue\``
    );
  }
  const argv = cmdline.split('\0').filter(Boolean);
  assert.ok(
    argv.some(arg => arg.includes('elohim-storage')),
    `pid ${pid} is not an elohim-storage process (argv: ${argv.join(' ').slice(0, 200)}) — the ` +
      `fixture's pid for "${name}" is stale and the kernel recycled it`
  );
  assert.ok(
    argv.some((arg, index) => arg === '--http-port' && argv[index + 1] === port),
    `pid ${pid} is an elohim-storage process but not the one on port ${port} (argv: ` +
      `${argv.join(' ').slice(0, 200)}) — the fixture's pid for "${name}" points at another peer`
  );
  return pid;
}

/** Read one variable out of a peer's own live environment. */
async function peerEnv(name: HouseholdPeerName, key: string): Promise<string> {
  const pid = await verifiedPeerPid(name);
  let raw: string;
  try {
    raw = await readFile(`/proc/${pid}/environ`, 'utf8');
  } catch (error) {
    assert.fail(`cannot read /proc/${pid}/environ for household peer "${name}": ${String(error)}`);
  }
  const entry = raw
    .split('\0')
    .find(pair => pair.startsWith(`${key}=`))
    ?.slice(key.length + 1);
  assert.ok(entry, `household peer "${name}" (pid ${pid}) has no ${key} in its environment`);
  return entry;
}

async function peerStorageDir(name: HouseholdPeerName): Promise<string> {
  return peerEnv(name, 'STORAGE_DIR');
}

function peerLogPath(name: HouseholdPeerName): string {
  const peer = requireFixtureStoragePeer(fixture(), name) as { logPath?: string };
  assert.ok(
    peer.logPath,
    `household fixture has no logPath for peer "${name}" — the custody-sweep outcome is only ` +
      "observable in that peer's log (fallback_kicks has no HTTP surface)"
  );
  return peer.logPath;
}

/**
 * Kill ONE peer by its exact fixture pid, after preserving the environment
 * hc-mesh.sh needs to bring it back. Never a pattern match.
 */
async function killPeer(state: DrillState, name: HouseholdPeerName): Promise<void> {
  if (state.down.has(name)) return;
  const pid = await verifiedPeerPid(name);

  // hc-mesh.sh restart_storage recovers a dead peer from the last capture in
  // $MESH_DIR/storage-restart/<name>.environ. Capture it while /proc still
  // exists, or a killed peer becomes unrestartable.
  const workdir = path.join(MESH_DIR, 'storage-restart');
  await mkdir(workdir, { recursive: true });
  try {
    await copyFile(`/proc/${pid}/environ`, path.join(workdir, `${name}.environ`));
  } catch (error) {
    assert.fail(
      `refusing to kill household peer "${name}" (pid ${pid}): could not capture its environment ` +
        `for restart (${String(error)}) — it would not come back`
    );
  }

  process.kill(pid, 'SIGKILL');
  state.down.add(name);
}

async function killPeersTogether(
  state: DrillState,
  names: readonly HouseholdPeerName[]
): Promise<void> {
  // Capture every environ first, then fire the signals back-to-back so the
  // losses really are simultaneous rather than serialised behind file IO.
  const workdir = path.join(MESH_DIR, 'storage-restart');
  await mkdir(workdir, { recursive: true });
  const pids: { name: HouseholdPeerName; pid: number }[] = [];
  for (const name of names) {
    if (state.down.has(name)) continue;
    const pid = await verifiedPeerPid(name);
    await copyFile(`/proc/${pid}/environ`, path.join(workdir, `${name}.environ`));
    pids.push({ name, pid });
  }
  for (const { name, pid } of pids) {
    process.kill(pid, 'SIGKILL');
    state.down.add(name);
  }
}

/** Bring peers back the only supported way: the mesh script's own restart. */
async function restartPeers(state: DrillState, names: readonly HouseholdPeerName[]): Promise<void> {
  const targets = names.filter(name => state.down.has(name));
  if (targets.length === 0) return;
  await execFileAsync('bash', [HC_MESH_SH, 'storage-restart', ...targets], {
    timeout: 240_000,
    maxBuffer: 8 * 1024 * 1024,
  });
  for (const name of targets) state.down.delete(name);
}

async function restartDownPeers(state: DrillState): Promise<void> {
  await restartPeers(state, [...state.down]);
}

// ---------------------------------------------------------------------------
// Blob addressing + the local (filesystem) blob-store oracle
// ---------------------------------------------------------------------------

const BASE32_LOWER = 'abcdefghijklmnopqrstuvwxyz234567';

function decodeBase32Lower(input: string): Uint8Array {
  let bits = 0;
  let value = 0;
  const out: number[] = [];
  for (const char of input) {
    const index = BASE32_LOWER.indexOf(char);
    assert.ok(index >= 0, `"${char}" is not a lower-case base32 character`);
    value = (value << 5) | index;
    bits += 5;
    if (bits >= 8) {
      bits -= 8;
      out.push((value >>> bits) & 0xff);
    }
  }
  return Uint8Array.from(out);
}

/**
 * Reduce any wire blob address to the sha256 hex the filesystem is keyed on.
 * Freshly-ingested content carries a CIDv1 (`bafkrei…` = raw codec + sha2-256);
 * legacy rows still carry `sha256-<64hex>`. Mirrors
 * `BlobStore::parse_content_address`.
 */
function sha256HexOf(address: string): string {
  const prefixed = /^sha256-([0-9a-f]{64})$/.exec(address);
  if (prefixed) return prefixed[1];
  if (/^[0-9a-f]{64}$/.test(address)) return address;
  assert.match(
    address,
    /^b[a-z2-7]+$/,
    `blob address "${address}" is neither sha256-<hex> nor a base32 CIDv1`
  );
  const bytes = decodeBase32Lower(address.slice(1));
  assert.ok(
    bytes.length === 36 &&
      bytes[0] === 0x01 &&
      bytes[1] === 0x55 &&
      bytes[2] === 0x12 &&
      bytes[3] === 0x20,
    `CID "${address}" is not a v1 raw/sha2-256 content address`
  );
  return [...bytes.slice(4)].map(byte => byte.toString(16).padStart(2, '0')).join('');
}

function canonicalBlobHash(address: string): string {
  return `sha256-${sha256HexOf(address)}`;
}

/** `$STORAGE_DIR/blobs/blobs/<first-4-hex>/sha256-<hex>` */
function blobFilePath(storageDir: string, hex: string): string {
  return path.join(storageDir, 'blobs', 'blobs', hex.slice(0, 4), `sha256-${hex}`);
}

function blobPlaneDir(storageDir: string): string {
  return path.join(storageDir, 'blobs', 'blobs');
}

async function localBlobPresent(name: HouseholdPeerName, address: string): Promise<boolean> {
  const storageDir = await peerStorageDir(name);
  return existsSync(blobFilePath(storageDir, sha256HexOf(address)));
}

/** Move ONE blob's bytes out of a peer's local store; rows are left intact. */
async function sidelineBlob(
  state: DrillState,
  name: HouseholdPeerName,
  address: string
): Promise<void> {
  const hex = sha256HexOf(address);
  const key = `${name}:${hex}`;
  if (state.sidelined.has(key)) return;
  const storageDir = await peerStorageDir(name);
  const from = blobFilePath(storageDir, hex);
  assert.ok(
    existsSync(from),
    `household peer "${name}" does not hold sha256-${hex} locally — nothing to make missing ` +
      `(looked at ${from})`
  );
  const to = `${from}.a2o-sidelined`;
  await rename(from, to);
  state.sidelined.set(key, { from, to });
}

/**
 * Wipe a peer's whole blob byte-plane by renaming it aside — the recoverable
 * half of the RESET_STORAGE drill (bytes gone, rows and identity intact).
 */
async function wipeBlobPlane(state: DrillState, name: HouseholdPeerName): Promise<void> {
  const storageDir = await peerStorageDir(name);
  const liveDir = blobPlaneDir(storageDir);
  assert.ok(existsSync(liveDir), `household peer "${name}" has no blob plane at ${liveDir}`);
  const backupDir = `${liveDir}.a2o-wiped-${Date.now()}`;
  await rename(liveDir, backupDir);
  await mkdir(liveDir, { recursive: true });
  state.wipedPlanes.push({ peer: name, liveDir, backupDir });
}

async function listRelativeFiles(root: string): Promise<string[]> {
  const found: string[] = [];
  const walk = async (dir: string): Promise<void> => {
    let entries;
    try {
      entries = await readdir(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) await walk(full);
      else found.push(path.relative(root, full));
    }
  };
  await walk(root);
  return found;
}

/** Put back every byte this scenario removed, keeping anything the mesh healed. */
async function restoreBlobs(state: DrillState): Promise<void> {
  for (const [key, move] of state.sidelined) {
    try {
      if (!existsSync(move.from) && existsSync(move.to)) await rename(move.to, move.from);
      else if (existsSync(move.to)) await rm(move.to, { force: true });
    } catch {
      // best-effort restore; the cleanup hook must never mask the real failure
    }
    state.sidelined.delete(key);
  }
  for (const plane of state.wipedPlanes.splice(0)) {
    try {
      for (const relative of await listRelativeFiles(plane.backupDir)) {
        const target = path.join(plane.liveDir, relative);
        if (existsSync(target)) continue;
        await mkdir(path.dirname(target), { recursive: true });
        await copyFile(path.join(plane.backupDir, relative), target);
      }
      await rm(plane.backupDir, { recursive: true, force: true });
    } catch {
      // best-effort restore
    }
  }
}

// ---------------------------------------------------------------------------
// Live surfaces
// ---------------------------------------------------------------------------

interface PeerStatus {
  peerId: string;
  connectedPeers: number;
  reconcilePassesTotal: number;
  kicksFiredTotal: number;
  projectionReconcile?: {
    caughtUp?: boolean;
    peersAsked?: number;
    completed?: number;
    failed?: number;
    healedTotal?: number;
    sweeps?: number;
  };
}

async function peerStatus(peer: PeerHandle): Promise<PeerStatus> {
  const { body } = await probeP2PStatus(peer.url);
  const status = body as unknown as PeerStatus;
  assert.equal(typeof status.peerId, 'string', `${peer.name} /p2p/status has no peerId`);
  peer.peerId = status.peerId;
  return status;
}

async function measurePeerIds(state: DrillState): Promise<void> {
  for (const peer of state.peers) {
    if (state.down.has(peer.name)) continue;
    await peerStatus(peer);
  }
}

interface ParityReport {
  gossipedButMissing: string[];
  localButNotGossiped: string[];
  filesystemCount: number;
  gossipedCount: number;
}

async function inventoryParity(peer: PeerHandle): Promise<ParityReport> {
  const { body } = await probeInventoryParity(peer.url);
  const wire = body;
  const list = (snake: string, camel: string): string[] => {
    const value = wire[snake] ?? wire[camel];
    return Array.isArray(value) ? (value as string[]) : [];
  };
  const count = (snake: string, camel: string): number => {
    const value = wire[snake] ?? wire[camel];
    return typeof value === 'number' ? value : -1;
  };
  const report: ParityReport = {
    gossipedButMissing: list('gossiped_but_missing', 'gossipedButMissing'),
    localButNotGossiped: list('local_but_not_gossiped', 'localButNotGossiped'),
    filesystemCount: count('filesystem_count', 'filesystemCount'),
    gossipedCount: count('gossiped_count', 'gossipedCount'),
  };
  assert.ok(report.filesystemCount >= 0, `${peer.name} inventory parity omitted filesystemCount`);
  assert.ok(report.gossipedCount >= 0, `${peer.name} inventory parity omitted gossipedCount`);
  return report;
}

/** Every custody-blob commitment this peer's projection holds. */
async function custodyCredits(baseUrl: string): Promise<CustodyCredit[]> {
  const { status, text } = await getRaw(
    `${baseUrl}/api/v1/commitments?action=custody-blob&limit=500`
  );
  assert.equal(status, 200, `GET ${baseUrl}/api/v1/commitments?action=custody-blob → ${status}`);
  const rows = JSON.parse(text) as unknown;
  const list = Array.isArray(rows)
    ? (rows as Record<string, unknown>[])
    : (((rows as Record<string, unknown>)['items'] as Record<string, unknown>[] | undefined) ?? []);
  return list.map(row => {
    // resourceClassifiedAs is a JSON list by contract; a custody-blob row
    // carries the blob hash as the list's primary element (bare on old rows).
    const classified = row['resourceClassifiedAs'];
    const primary = Array.isArray(classified)
      ? String((classified as unknown[])[0] ?? '')
      : String(classified ?? '');
    return {
      id: String(row['id'] ?? ''),
      provider: String(row['provider'] ?? ''),
      blobHash: primary,
    };
  });
}

function creditsFor(credits: CustodyCredit[], address: string): CustodyCredit[] {
  const hex = sha256HexOf(address);
  return credits.filter(credit => {
    try {
      return credit.blobHash !== '' && sha256HexOf(credit.blobHash) === hex;
    } catch {
      return false;
    }
  });
}

interface ServeBlobEvent {
  provider: string;
  receiver: string;
  resourceInventoriedAs: string;
  hasPointInTime: string;
}

async function serveBlobEvents(baseUrl: string, address: string): Promise<ServeBlobEvent[]> {
  const { status, text } = await getRaw(
    `${baseUrl}/api/v1/economic-events?action=serve-blob&limit=500`
  );
  assert.equal(status, 200, `GET ${baseUrl}/api/v1/economic-events?action=serve-blob → ${status}`);
  const parsed = JSON.parse(text) as unknown;
  const rows = Array.isArray(parsed)
    ? (parsed as Record<string, unknown>[])
    : (((parsed as Record<string, unknown>)['items'] as Record<string, unknown>[] | undefined) ??
      []);
  const hex = sha256HexOf(address);
  return rows
    .map(row => ({
      provider: String(row['provider'] ?? ''),
      receiver: String(row['receiver'] ?? ''),
      resourceInventoriedAs: String(row['resourceInventoriedAs'] ?? ''),
      hasPointInTime: String(row['hasPointInTime'] ?? ''),
    }))
    .filter(event => {
      try {
        return sha256HexOf(event.resourceInventoriedAs) === hex;
      } catch {
        return false;
      }
    });
}

async function protectionStatus(world: E2EWorld, contentId: string): Promise<string> {
  const { status, text } = await getRaw(
    `${doorwayUrl(world)}/api/v1/resilience/${encodeURIComponent(contentId)}/household`
  );
  assert.equal(
    status,
    200,
    `GET /api/v1/resilience/${contentId}/household → ${status} — the felt status must answer, ` +
      `not fall silent (body: ${text.slice(0, 160)})`
  );
  const snapshot = JSON.parse(text) as Record<string, unknown>;
  const value = snapshot['protectionStatus'];
  assert.ok(
    typeof value === 'string' && value.length > 0,
    `household resilience snapshot for "${contentId}" carries no protectionStatus: ` +
      `${text.slice(0, 200)}`
  );
  return value;
}

async function blobHashFor(world: E2EWorld, state: DrillState, contentId: string): Promise<string> {
  const cached = state.blobHashes.get(contentId);
  if (cached) return cached;
  const { body } = await probeContent(doorwayUrl(world), contentId);
  const hash = body.blobHash;
  assert.ok(
    typeof hash === 'string' && hash.length > 0,
    `content "${contentId}" has no blobHash on the doorway projection — nothing to hold custody of`
  );
  const canonical = canonicalBlobHash(hash);
  state.blobHashes.set(contentId, canonical);
  return canonical;
}

async function doorwayBlobBytes(world: E2EWorld, address: string): Promise<Uint8Array> {
  const response = await fetch(`${doorwayUrl(world)}/blob/${encodeURIComponent(address)}`, {
    signal: AbortSignal.timeout(60_000),
  });
  assert.ok(response.ok, `GET doorway /blob/${address} → ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

async function assertSeesNeighbours(
  state: DrillState,
  peer: PeerHandle,
  expected: number
): Promise<void> {
  const status = await peerStatus(peer);
  assert.ok(
    status.connectedPeers >= expected,
    `${peer.name} sees ${status.connectedPeers} neighbours; a joined peer must see ${expected}`
  );
}

/** Read the peer log from a previously captured offset (new lines only). */
async function newLogLines(state: DrillState, name: HouseholdPeerName): Promise<string[]> {
  const text = await readFile(peerLogPath(name), 'utf8');
  const offset = state.logOffsets.get(name) ?? 0;
  return text.slice(offset).split('\n');
}

async function markLogOffset(state: DrillState, name: HouseholdPeerName): Promise<void> {
  try {
    const text = await readFile(peerLogPath(name), 'utf8');
    state.logOffsets.set(name, text.length);
  } catch {
    state.logOffsets.set(name, 0);
  }
}

function logFieldValue(line: string, field: string): number | undefined {
  const match = new RegExp(String.raw`${field}[=:]\s*"?(\d+)"?`).exec(line);
  return match ? Number(match[1]) : undefined;
}

// ===========================================================================
// features/resilience/chaos-peer-churn.feature
// ===========================================================================

Given(
  'the household mesh is three storage peers, each a dedicated drill fixture',
  { timeout: 60_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    assert.equal(
      state.peers.length,
      3,
      `this drill file is written for a three-peer household; the fixture resolved ` +
        `${state.peers.length}`
    );
    assert.equal(
      fixture().processControl,
      true,
      'chaos drills kill real processes — the fixture must declare processControl: true'
    );
    for (const peer of state.peers) {
      // A pid we cannot resolve is a peer we must not pretend to drill.
      await verifiedPeerPid(peer.name);
      const { status } = await getRaw(`${peer.url}/health`, { timeoutMs: 10_000 });
      assert.equal(status, 200, `household peer ${peer.name} is not serving (/health ${status})`);
    }
  }
);

Given(
  'a household peer counts as joined only while it sees {int} neighbours',
  function (this: E2EWorld, neighbours: number) {
    const state = drill(this);
    assert.equal(
      state.floor,
      neighbours,
      `the story defines a joined peer as one that sees ${neighbours} neighbours, but the ` +
        `fixture's connectedPeersFloor is ${state.floor} — reconcile them before trusting any ` +
        'health assertion in this file'
    );
  }
);

Given(
  'the household protection status reads {string} at {int} custody copies, {string} at {int}, and {string} at {int}',
  function (
    this: E2EWorld,
    protectedLabel: string,
    protectedCopies: number,
    partialLabel: string,
    partialCopies: number,
    atRiskLabel: string,
    atRiskCopies: number
  ) {
    const state = drill(this);
    // FINDING: elohim-storage publishes no threshold table over HTTP — the
    // ladder is a Rust `match` in services/household_resilience.rs. So the
    // Background REGISTERS the mapping the story commits to, and every status
    // rung below resolves its expected label through it instead of hardcoding
    // a string. A ladder change now has exactly one place to be made.
    const ladder = new Map<number, string>([
      [protectedCopies, protectedLabel],
      [partialCopies, partialLabel],
      [atRiskCopies, atRiskLabel],
    ]);
    assert.equal(ladder.size, 3, 'the ladder declares two rungs at the same copy count');
    assert.equal(
      new Set([protectedLabel, partialLabel, atRiskLabel]).size,
      3,
      'the ladder gives two different copy counts the same label'
    );
    assert.ok(
      protectedCopies > partialCopies && partialCopies > atRiskCopies,
      'the ladder does not descend: fewer copies must never read as more protection'
    );
    assert.equal(
      protectedCopies,
      state.peers.length,
      `the ladder tops out at ${protectedCopies} custody copies but this household has ` +
        `${state.peers.length} peers — the top rung would be unreachable`
    );
    state.ladder = ladder;
  }
);

/** Resolve the label the registered ladder gives to `copies` custody copies. */
function ladderLabel(state: DrillState, copies: number): string {
  const label = state.ladder?.get(copies);
  assert.ok(
    label,
    `no protection label is registered for ${copies} custody copies — run the Background ladder ` +
      'step first'
  );
  return label;
}

Given(
  'content {string} is under custody on every household peer',
  { timeout: 120_000 },
  async function (this: E2EWorld, contentId: string) {
    const state = drill(this);
    await assertUnderCustody(this, state, contentId, state.peers.length);
  }
);

Given(
  'content {string} is under custody on all {int} household peers',
  { timeout: 120_000 },
  async function (this: E2EWorld, contentId: string, expected: number) {
    const state = drill(this);
    await assertUnderCustody(this, state, contentId, expected);
  }
);

/**
 * "Under custody" as the feature defines it: the peer has promised to hold a
 * copy (a custody-blob commitment naming it as provider) AND it actually holds
 * the bytes. Both halves, or the mesh's belief is already corrupt at t=0.
 */
async function assertUnderCustody(
  world: E2EWorld,
  state: DrillState,
  contentId: string,
  expectedPeers: number
): Promise<void> {
  const hash = await blobHashFor(world, state, contentId);
  await measurePeerIds(state);

  const { status, text } = await getRaw(
    `${doorwayUrl(world)}/api/v1/resilience/${encodeURIComponent(contentId)}/household`
  );
  assert.equal(status, 200, `GET /api/v1/resilience/${contentId}/household → ${status}`);
  const footprint = normalizeHouseholdFootprint(
    JSON.parse(text) as Record<string, unknown>,
    contentId
  );
  assert.ok(
    footprint.commitmentBacked > 0,
    `"${contentId}" has no commitment-backed collectives — nothing credits its custody`
  );

  const credits = creditsFor(await custodyCredits(state.peers[0]?.url), hash);
  const providers = new Set(credits.map(credit => credit.provider));
  let holding = 0;
  for (const peer of state.peers) {
    assert.ok(
      peer.peerId && providers.has(peer.peerId),
      `no custody-blob commitment names ${peer.name} (${peer.peerId ?? 'unmeasured'}) as ` +
        `provider for ${hash} — providers on record: ${[...providers].join(', ') || 'none'}`
    );
    assert.ok(
      await localBlobPresent(peer.name, hash),
      `${peer.name} is credited with custody of ${hash} but does not hold the bytes`
    );
    holding += 1;
  }
  assert.equal(
    holding,
    expectedPeers,
    `expected ${expectedPeers} household peers under custody of "${contentId}", found ${holding}`
  );
  state.expectedBytes.set(contentId, await doorwayBlobBytes(world, hash));
}

Given(
  "the mesh's custody record for {string} is settled",
  { timeout: 120_000 },
  async function (this: E2EWorld, contentId: string) {
    const state = drill(this);
    const hash = await blobHashFor(this, state, contentId);

    // QUIESCENCE precondition, not a snapshot for its own sake: the mesh must
    // have finished making up its mind about this content BEFORE the churn, or
    // a duplicate found afterwards is attributable to a sweep that was still
    // mid-flight rather than to the drill. Settled = every peer agrees, AND the
    // agreed record does not move across a settle interval.
    const readAll = async (): Promise<Map<HouseholdPeerName, string>> => {
      const signatures = new Map<HouseholdPeerName, string>();
      for (const peer of state.peers) {
        const credits = creditsFor(await custodyCredits(peer.url), hash);
        signatures.set(
          peer.name,
          credits
            .map(credit => `${credit.provider}#${credit.id}`)
            .sort((first, second) => first.localeCompare(second))
            .join('|')
        );
      }
      return signatures;
    };

    const settleIntervalMs = Number(process.env['E2E_CUSTODY_SETTLE_MS'] ?? 15_000);
    const first = await readAll();
    assert.equal(
      new Set(first.values()).size,
      1,
      'household peers disagree about the custody record before the drill even starts: ' +
        [...first].map(([name, signature]) => `${name}=${signature || 'empty'}`).join(' / ')
    );
    await new Promise(resolve => setTimeout(resolve, settleIntervalMs));
    const second = await readAll();
    assert.deepEqual(
      [...second.entries()],
      [...first.entries()],
      `the custody record for "${contentId}" moved during a ${settleIntervalMs}ms settle window ` +
        '— the mesh has not made up its mind yet, so churn is not yet attributable'
    );

    const settled = creditsFor(
      await custodyCredits(peerByName(state, state.peers[0]?.name).url),
      hash
    );
    assert.ok(settled.length > 0, `no custody record exists for "${contentId}" to settle on`);
    state.settledCredits = settled;
  }
);

When(
  "Jessica's storage peer is killed and restarted {int} times, {int} seconds apart",
  { timeout: 900_000 },
  async function (this: E2EWorld, times: number, gapSeconds: number) {
    const held = holdDestructive(
      `SIGKILL Jessica's storage peer and restart it via hc-mesh.sh ${times} times, ` +
        `${gapSeconds}s apart`
    );
    if (held) return held;
    const state = drill(this);
    state.drillStartedAt = new Date().toISOString();
    for (let round = 0; round < times; round += 1) {
      await killPeer(state, 'jessica');
      await new Promise(resolve => setTimeout(resolve, gapSeconds * 1_000));
      await restartPeers(state, ['jessica']);
    }
  }
);

Then(
  "within {int} seconds of the final return Jessica's peer sees its {int} neighbours again",
  { timeout: 600_000 },
  async function (this: E2EWorld, seconds: number, neighbours: number) {
    const state = drill(this);
    await retry(async () => assertSeesNeighbours(state, peerByName(state, 'jessica'), neighbours), {
      maxAttempts: 200,
      initialDelayMs: 1_000,
      backoffFactor: 1.1,
      maxDelayMs: 5_000,
      timeoutMs: seconds * 1_000,
    });
  }
);

Then(
  "Jessica's peer holds the same inventory as the rest of the mesh",
  { timeout: RESYNC_WINDOW_MS + 15_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    await retry(
      async () => {
        const reports = new Map<HouseholdPeerName, ParityReport>();
        for (const peer of state.peers) reports.set(peer.name, await inventoryParity(peer));
        const jessica = reports.get('jessica');
        assert.ok(jessica);
        assert.deepEqual(
          jessica.gossipedButMissing,
          [],
          'Jessica is missing blobs her peers gossip as hers'
        );
        assert.deepEqual(
          jessica.localButNotGossiped,
          [],
          'Jessica holds blobs the mesh does not know about'
        );
        assert.equal(
          new Set([...reports.values()].map(report => report.filesystemCount)).size,
          1,
          'household inventories still differ: ' +
            [...reports].map(([name, r]) => `${name}=${r.filesystemCount}`).join(', ')
        );
      },
      {
        maxAttempts: 60,
        initialDelayMs: 1_000,
        backoffFactor: 1.2,
        maxDelayMs: 5_000,
        timeoutMs: RESYNC_WINDOW_MS,
      }
    );
  }
);

Then(
  'the mesh records exactly one custody copy of {string} per household peer',
  { timeout: 120_000 },
  async function (this: E2EWorld, contentId: string) {
    const state = drill(this);
    const hash = await blobHashFor(this, state, contentId);
    await measurePeerIds(state);
    for (const peer of state.peers) {
      const credits = creditsFor(await custodyCredits(peer.url), hash);
      const perProvider = new Map<string, number>();
      for (const credit of credits) {
        perProvider.set(credit.provider, (perProvider.get(credit.provider) ?? 0) + 1);
      }
      for (const household of state.peers) {
        assert.equal(
          perProvider.get(household.peerId ?? '') ?? 0,
          1,
          `${peer.name} records ${perProvider.get(household.peerId ?? '') ?? 0} custody copies of ` +
            `${hash} for ${household.name}; churn must be idempotent — exactly one`
        );
      }
    }
  }
);

Then(
  'no custody record names a peer that no longer holds the bytes',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const settled = state.settledCredits;
    assert.ok(
      settled && settled.length > 0,
      'no settled custody record was captured — run the settled-record Given first'
    );
    await measurePeerIds(state);
    const byPeerId = new Map(state.peers.map(peer => [peer.peerId ?? '', peer.name]));
    for (const record of settled) {
      const name = byPeerId.get(record.provider);
      if (!name) continue; // custodian outside this household — not ours to assert about
      assert.ok(
        await localBlobPresent(name, record.blobHash),
        `custody record ${record.id} still names ${name} as holding ${record.blobHash}, but the ` +
          'bytes are gone from its local store — a phantom custody row'
      );
    }
  }
);

When('one custody peer is killed', { timeout: 60_000 }, async function (this: E2EWorld) {
  const held = holdDestructive(`SIGKILL household peer ${CASCADE_KILL_ORDER[0]}`);
  if (held) return held;
  const state = drill(this);
  state.drillStartedAt = new Date().toISOString();
  await killPeer(state, CASCADE_KILL_ORDER[0]);
});

When(
  'a second custody peer is killed, leaving one',
  { timeout: 60_000 },
  async function (this: E2EWorld) {
    const held = holdDestructive(`SIGKILL household peer ${CASCADE_KILL_ORDER[1]}`);
    if (held) return held;
    const state = drill(this);
    await killPeer(state, CASCADE_KILL_ORDER[1]);
    assert.equal(
      state.peers.filter(peer => !state.down.has(peer.name)).length,
      1,
      'the cascade was supposed to leave exactly one household peer standing'
    );
  }
);

/**
 * One parameterised definition serves the ladder's Given (S2 baseline) and its
 * literal Then (S4's "at-risk") — cucumber matches on text, not keyword.
 */
Then(
  'the household protection status for {string} reports {string}',
  { timeout: 60_000 },
  async function (this: E2EWorld, contentId: string, expected: string) {
    const state = drill(this);
    assert.ok(
      [...(state.ladder?.values() ?? [])].includes(expected),
      `"${expected}" is not a rung on the ladder this feature declared ` +
        `(${[...(state.ladder?.values() ?? [])].join(', ') || 'none registered'})`
    );
    assert.equal(
      await protectionStatus(this, contentId),
      expected,
      `the household was told the wrong thing about "${contentId}"`
    );
  }
);

Then(
  'within {int} seconds the household protection status for {string} reports {string}',
  { timeout: 600_000 },
  async function (this: E2EWorld, seconds: number, contentId: string, expected: string) {
    const state = drill(this);
    assert.ok(
      [...(state.ladder?.values() ?? [])].includes(expected),
      `"${expected}" is not a rung on the ladder this feature declared ` +
        `(${[...(state.ladder?.values() ?? [])].join(', ') || 'none registered'})`
    );
    await retry(
      async () => {
        const actual = await protectionStatus(this, contentId);
        assert.equal(actual, expected, `protection status is "${actual}", expected "${expected}"`);
      },
      {
        maxAttempts: 200,
        initialDelayMs: 1_000,
        backoffFactor: 1.1,
        maxDelayMs: 5_000,
        timeoutMs: seconds * 1_000,
      }
    );
  }
);

Then(
  'the household protection status for {string} reports "at-risk", not "protected" and not silence',
  { timeout: 60_000 },
  async function (this: E2EWorld, contentId: string) {
    const state = drill(this);
    // protectionStatus() fails on a non-200 or a missing field: silence is a
    // failure here, not a pass — that is the whole point of the rung. The
    // expected label comes from the Background ladder at ONE custody copy, so
    // the string in the Gherkin and the string asserted here cannot drift.
    const expected = ladderLabel(state, 1);
    const actual = await protectionStatus(this, contentId);
    assert.equal(
      actual,
      expected,
      `one custody peer left, but the household is being told "${actual}"`
    );
  }
);

Then(
  'fetching {string} through the doorway still returns the content',
  { timeout: 120_000 },
  async function (this: E2EWorld, contentId: string) {
    const state = drill(this);
    const hash = await blobHashFor(this, state, contentId);
    const bytes = await doorwayBlobBytes(this, hash);
    const expected = state.expectedBytes.get(contentId);
    assert.ok(bytes.length > 0, `doorway served zero bytes for "${contentId}"`);
    if (expected)
      assert.deepEqual(bytes, expected, 'doorway served different bytes after the loss');
  }
);

Then(
  'the surviving peer still serves the content',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const survivors = state.peers.filter(peer => !state.down.has(peer.name));
    assert.equal(survivors.length, 1, 'this rung expects exactly one surviving household peer');
    const survivor = survivors[0];
    const [contentId] = [...state.blobHashes.keys()];
    assert.ok(contentId, 'no content was resolved earlier in this scenario');
    const hash = state.blobHashes.get(contentId) as string;
    const response = await fetch(`${survivor.url}/blob/${encodeURIComponent(hash)}`, {
      signal: AbortSignal.timeout(60_000),
    });
    assert.ok(response.ok, `surviving peer ${survivor.name} served ${response.status} for ${hash}`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    const expected = state.expectedBytes.get(contentId);
    if (expected) assert.deepEqual(bytes, expected, 'the survivor served different bytes');
  }
);

Given(
  'a household member is loading {string} through the doorway, large enough that the transfer is still in flight a second later',
  { timeout: 120_000 },
  async function (this: E2EWorld, contentId: string) {
    const state = drill(this);
    const hash = await blobHashFor(this, state, contentId);
    state.expectedBytes.set(contentId, await doorwayBlobBytes(this, hash));

    // Start the read and hold the stream open — the body is deliberately NOT
    // drained, so the transfer is genuinely mid-flight when the kill lands.
    const response = await fetch(`${doorwayUrl(this)}/blob/${encodeURIComponent(hash)}`, {
      signal: AbortSignal.timeout(180_000),
    });
    assert.ok(response.ok, `doorway refused the read: ${response.status}`);
    assert.ok(response.body, 'doorway returned no readable stream');
    const reader = response.body.getReader();
    const first = await reader.read();
    assert.ok(first.value && first.value.length > 0, 'no bytes arrived before the kill');
    reader.releaseLock();
    const expected = state.expectedBytes.get(contentId) as Uint8Array;
    assert.ok(
      first.value.length < expected.length,
      `"${contentId}" arrived in one chunk (${expected.length} bytes) — it is not large enough ` +
        'for a mid-transfer kill to mean anything'
    );
    state.inFlight = { contentId, response, firstChunk: first.value };
  }
);

When(
  'the peer serving that read is killed mid-transfer',
  { timeout: 60_000 },
  async function (this: E2EWorld) {
    const held = holdDestructive(
      `SIGKILL the doorway's primary storage peer (${servingPeerName('alpha')}) mid-transfer`
    );
    if (held) return held;
    const state = drill(this);
    assert.ok(state.inFlight, 'no read is in flight');
    state.drillStartedAt = new Date().toISOString();
    // The doorway reads its pool primary first; that is the peer serving this
    // transfer. Asserted rather than assumed by servingPeerName().
    await killPeer(state, servingPeerName('alpha'));
  }
);

Then(
  'the load completes from another custody peer without the member doing anything',
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const inFlight = state.inFlight;
    assert.ok(inFlight, 'no read is in flight');
    const expected = state.expectedBytes.get(inFlight.contentId) as Uint8Array;

    // No retry, no re-request: the SAME read the member started must finish.
    const chunks: Uint8Array[] = [inFlight.firstChunk];
    const reader = (inFlight.response.body as ReadableStream<Uint8Array>).getReader();
    for (;;) {
      const next = await reader.read();
      if (next.done) break;
      if (next.value) chunks.push(next.value);
    }
    const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
    const merged = new Uint8Array(total);
    let offset = 0;
    for (const chunk of chunks) {
      merged.set(chunk, offset);
      offset += chunk.length;
    }
    assert.deepEqual(
      merged,
      expected,
      `the in-flight read finished with ${merged.length} of ${expected.length} bytes after its ` +
        'serving peer died'
    );
  }
);

Then(
  'the household peer that supplied the rescue bytes is credited for serving them',
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const inFlight = state.inFlight;
    assert.ok(inFlight, 'no read is in flight');
    const hash = state.blobHashes.get(inFlight.contentId) as string;
    const since = state.drillStartedAt ?? '';
    await measurePeerIds(state);

    const found: string[] = [];
    for (const peer of state.peers) {
      if (state.down.has(peer.name)) continue;
      for (const event of await serveBlobEvents(peer.url, hash)) {
        if (event.hasPointInTime >= since) found.push(`${peer.name}←${event.provider}`);
      }
    }
    assert.ok(
      found.length > 0,
      `no serve-blob economic event was booked for ${hash} after ${since} on any surviving ` +
        'household peer — the rescue happened but nobody was credited for it'
    );
  }
);

Given(
  'every household peer sees its {int} neighbours',
  { timeout: 120_000 },
  async function (this: E2EWorld, neighbours: number) {
    const state = drill(this);
    for (const peer of state.peers) await assertSeesNeighbours(state, peer, neighbours);
  }
);

When(
  'two storage peers are killed at the same moment',
  { timeout: 60_000 },
  async function (this: E2EWorld) {
    const held = holdDestructive(
      `SIGKILL household peers ${CASCADE_KILL_ORDER[0]} and ${CASCADE_KILL_ORDER[1]} together`
    );
    if (held) return held;
    const state = drill(this);
    state.drillStartedAt = new Date().toISOString();
    await killPeersTogether(state, [CASCADE_KILL_ORDER[0], CASCADE_KILL_ORDER[1]]);
  }
);

Then(
  'the surviving peer reports no connected peers rather than a stale count',
  { timeout: 180_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const survivors = state.peers.filter(peer => !state.down.has(peer.name));
    assert.equal(survivors.length, 1, 'this rung expects exactly one surviving household peer');
    const survivor = survivors[0];
    await retry(
      async () => {
        const status = await peerStatus(survivor);
        assert.equal(
          status.connectedPeers,
          0,
          `${survivor.name} still reports ${status.connectedPeers} connected peers after both ` +
            'neighbours died — a stale count is the dishonesty this rung exists to catch'
        );
      },
      {
        maxAttempts: 60,
        initialDelayMs: 1_000,
        backoffFactor: 1.2,
        maxDelayMs: 5_000,
        timeoutMs: 150_000,
      }
    );
  }
);

Then(
  'the surviving peer still serves {string} from its own store',
  { timeout: 120_000 },
  async function (this: E2EWorld, contentId: string) {
    const state = drill(this);
    const survivors = state.peers.filter(peer => !state.down.has(peer.name));
    assert.equal(survivors.length, 1, 'this rung expects exactly one surviving household peer');
    const survivor = survivors[0];
    const hash = state.blobHashes.get(contentId) ?? (await blobHashFor(this, state, contentId));

    // "from its own store" is a claim about local bytes, so check the disk
    // first: an HTTP 200 alone could have been a heal from a peer.
    assert.ok(
      await localBlobPresent(survivor.name, hash),
      `${survivor.name} no longer holds ${hash} locally — it cannot be serving from its own store`
    );
    const response = await fetch(`${survivor.url}/blob/${encodeURIComponent(hash)}`, {
      signal: AbortSignal.timeout(60_000),
    });
    assert.ok(response.ok, `surviving peer ${survivor.name} served ${response.status}`);
    const expected = state.expectedBytes.get(contentId);
    if (expected) {
      assert.deepEqual(new Uint8Array(await response.arrayBuffer()), expected);
    }
  }
);

/**
 * TEARDOWN for the chaos drills — load-bearing, not a nicety.
 *
 * chaos-peer-churn deliberately ends its last scenario while two peers are
 * dead: a closing "everyone sees 2 neighbours again" rung would be the exact
 * post-return assertion three peers cannot make honestly, and a reader of the
 * run report would take it for a healing claim. So the restore lives here,
 * where it hands the mesh back intact without the drill claiming it healed
 * anything. It runs after EVERY scenario in this feature — including a failed
 * one — restarting only the peers this run stopped, and it waits for the mesh
 * to be joined again before the next scenario starts.
 *
 * Scoped to @chaos (unique to chaos-peer-churn.feature) so it cannot fire for
 * unrelated scenarios. The other two features restore through the same helpers
 * via world.onCleanup.
 */
After({ tags: '@chaos', timeout: 900_000 }, async function (this: E2EWorld) {
  const state = drillStates.get(this);
  if (!state) return;
  const touched = state.down.size > 0 || state.sidelined.size > 0 || state.wipedPlanes.length > 0;
  await restoreBlobs(state);
  await restartDownPeers(state);
  // Nothing was stopped (the destructive gate held the drill), so there is
  // nothing to wait for and no reason to spend the re-sync window probing.
  if (!touched) return;
  await retry(
    async () => {
      for (const peer of state.peers) await assertSeesNeighbours(state, peer, state.floor);
    },
    {
      maxAttempts: 120,
      initialDelayMs: 1_000,
      backoffFactor: 1.15,
      maxDelayMs: 5_000,
      timeoutMs: RESYNC_WINDOW_MS,
    }
  );
});

// ===========================================================================
// features/federation/peer-recovery.feature
// ===========================================================================

const MANIFESTO = 'manifesto';

Given(
  'human {string} has a storage peer in the household mesh',
  { timeout: 60_000 },
  async function (this: E2EWorld, human: string) {
    const state = drill(this);
    const name = human.toLowerCase() as HouseholdPeerName;
    const peer = peerByName(state, name);
    // A peer we cannot address by a VERIFIED pid is not one we can drill.
    await verifiedPeerPid(name);
    const { status } = await getRaw(`${peer.url}/health`, { timeoutMs: 10_000 });
    assert.equal(status, 200, `${human}'s storage peer is not serving (/health ${status})`);
    await peerStatus(peer);
  }
);

Given(
  "a custody-blob commitment names Jessica's peer as provider for the manifesto blob",
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const hash = await blobHashFor(this, state, MANIFESTO);
    const jessica = peerByName(state, 'jessica');
    await peerStatus(jessica);
    const credits = creditsFor(await custodyCredits(jessica.url), hash);
    assert.ok(
      credits.some(credit => credit.provider === jessica.peerId),
      `no custody-blob commitment names Jessica (${jessica.peerId ?? 'unmeasured'}) as provider ` +
        `for ${hash} — recovery has no promise to act on. Providers on record: ` +
        `${[...new Set(credits.map(c => c.provider))].join(', ') || 'none'}`
    );
  }
);

Given(
  "Jessica's peer serves the manifesto blob",
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const hash = await blobHashFor(this, state, MANIFESTO);
    assert.ok(
      await localBlobPresent('jessica', hash),
      `Jessica does not hold ${hash} locally, so there is nothing for a wipe to take away`
    );
    const jessica = peerByName(state, 'jessica');
    const response = await fetch(`${jessica.url}/blob/${encodeURIComponent(hash)}`, {
      signal: AbortSignal.timeout(60_000),
    });
    assert.ok(response.ok, `Jessica served ${response.status} for ${hash}`);
    state.expectedBytes.set(MANIFESTO, new Uint8Array(await response.arrayBuffer()));
  }
);

When(
  "Jessica's peer content store is wiped while the mesh stays up",
  { timeout: 180_000 },
  async function (this: E2EWorld) {
    const held = holdDestructive(
      "rename Jessica's whole blob byte-plane aside (rows and identity intact; the After-safe " +
        'restore copies back anything the mesh did not heal)'
    );
    if (held) return held;
    const state = drill(this);
    const jessica = peerByName(state, 'jessica');
    const before = await peerStatus(jessica);
    state.baselines.set('jessica:kicks', before.kicksFiredTotal);
    state.baselines.set('jessica:passes', before.reconcilePassesTotal);
    state.drillStartedAt = new Date().toISOString();
    await markLogOffset(state, 'jessica');

    await wipeBlobPlane(state, 'jessica');

    const hash = await blobHashFor(this, state, MANIFESTO);
    assert.equal(
      await localBlobPresent('jessica', hash),
      false,
      'the wipe did not actually remove the manifesto bytes from Jessica'
    );
    // "while the mesh stays up" is half the claim: the other peers must be
    // serving, or this is a mesh outage rather than one device losing a disk.
    for (const peer of state.peers) {
      if (peer.name === 'jessica') continue;
      const { status } = await getRaw(`${peer.url}/health`, { timeoutMs: 10_000 });
      assert.equal(status, 200, `${peer.name} went down with the wipe (/health ${status})`);
    }
  }
);

Then(
  "within the recovery window Jessica's peer serves the manifesto blob again",
  { timeout: RECOVERY_WINDOW_MS + 30_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const hash = await blobHashFor(this, state, MANIFESTO);
    // Filesystem, NOT GET /blob — an HTTP read heals on miss and would make
    // this assertion prove itself.
    await retry(
      async () => {
        assert.ok(
          await localBlobPresent('jessica', hash),
          `Jessica still has no local bytes for ${hash}; nothing put them back`
        );
      },
      {
        maxAttempts: 200,
        initialDelayMs: 2_000,
        backoffFactor: 1.1,
        maxDelayMs: 15_000,
        timeoutMs: RECOVERY_WINDOW_MS,
      }
    );
  }
);

Then(
  "the custody sweep on Jessica's peer reports a kick for the manifesto blob",
  { timeout: SWEEP_WINDOW_MS + 30_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const jessica = peerByName(state, 'jessica');
    const baseline = state.baselines.get('jessica:kicks') ?? 0;
    await retry(
      async () => {
        const status = await peerStatus(jessica);
        assert.ok(
          status.kicksFiredTotal > baseline,
          `Jessica's kicksFiredTotal is still ${status.kicksFiredTotal} (was ${baseline}) — the ` +
            'custody sweep fired nothing for the blob it promised to hold'
        );
      },
      {
        maxAttempts: 200,
        initialDelayMs: 2_000,
        backoffFactor: 1.1,
        maxDelayMs: 15_000,
        timeoutMs: SWEEP_WINDOW_MS,
      }
    );
  }
);

Then(
  'a {string} economic event records which household peer aided the recovery',
  { timeout: 180_000 },
  async function (this: E2EWorld, action: string) {
    assert.equal(action, 'serve-blob', `this step reads serve-blob events, not "${action}"`);
    const state = drill(this);
    const hash = await blobHashFor(this, state, MANIFESTO);
    const jessica = peerByName(state, 'jessica');
    await peerStatus(jessica);
    const since = state.drillStartedAt ?? '';
    const events = (await serveBlobEvents(jessica.url, hash)).filter(
      event => event.hasPointInTime >= since
    );
    assert.ok(
      events.length > 0,
      `Jessica recovered ${hash} but booked no serve-blob event after ${since} — the peer that ` +
        'aided her is unrecognized'
    );
    for (const event of events) {
      assert.ok(event.provider.length > 0, 'a serve-blob event names no provider');
      assert.notEqual(
        event.provider,
        jessica.peerId,
        'the serve-blob event credits Jessica for serving herself'
      );
    }
  }
);

Given(
  "Jessica's peer was restarted and its gossip inventory is empty",
  { timeout: 600_000 },
  async function (this: E2EWorld) {
    const held = holdDestructive("SIGKILL Jessica's storage peer and restart it via hc-mesh.sh");
    if (held) return held;
    const state = drill(this);
    await killPeer(state, 'jessica');
    await restartPeers(state, ['jessica']);
    const jessica = peerByName(state, 'jessica');
    const status = await peerStatus(jessica);
    state.baselines.set('jessica:kicks', status.kicksFiredTotal);
    state.baselines.set('jessica:passes', status.reconcilePassesTotal);
    await markLogOffset(state, 'jessica');
    const parity = await inventoryParity(jessica);
    assert.equal(
      parity.gossipedCount,
      0,
      `Jessica's gossip inventory already carries ${parity.gossipedCount} entries after the ` +
        'restart — this scenario needs the inventory-blind starting condition'
    );
  }
);

Given(
  "the manifesto blob is missing from Jessica's local store",
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const held = holdDestructive("move the manifesto blob out of Jessica's local store");
    if (held) return held;
    const state = drill(this);
    const hash = await blobHashFor(this, state, MANIFESTO);
    await sidelineBlob(state, 'jessica', hash);
    assert.equal(await localBlobPresent('jessica', hash), false, 'the blob is still present');
    state.drillStartedAt = new Date().toISOString();
  }
);

When(
  "the custody sweep runs on Jessica's peer",
  { timeout: SWEEP_WINDOW_MS + 30_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const jessica = peerByName(state, 'jessica');
    const baseline = state.baselines.get('jessica:passes') ?? 0;
    // The sweep is timer-driven; wait for a pass to complete rather than
    // pretending to trigger one.
    await retry(
      async () => {
        const status = await peerStatus(jessica);
        assert.ok(
          status.reconcilePassesTotal > baseline,
          `no custody reconcile pass has completed on Jessica since the drill started ` +
            `(reconcilePassesTotal still ${status.reconcilePassesTotal})`
        );
      },
      {
        maxAttempts: 200,
        initialDelayMs: 2_000,
        backoffFactor: 1.1,
        maxDelayMs: 15_000,
        timeoutMs: SWEEP_WINDOW_MS,
      }
    );
  }
);

Then(
  'the sweep falls back to racing the connected household peers',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    // FINDING: ReconcileOutcome.fallback_kicks has NO HTTP surface — /p2p/status
    // exposes reconcilePassesTotal / kicksFiredTotal / placementGapsEmittedTotal
    // only. The per-pass "T23: custody reconcile pass completed" INFO line
    // (elohim-storage p2p/mod.rs) is the sole place it is reported, so this
    // assertion reads the peer's own log.
    const lines = (await newLogLines(state, 'jessica')).filter(line =>
      line.includes('T23: custody reconcile pass completed')
    );
    assert.ok(
      lines.length > 0,
      "no custody reconcile pass line appeared in Jessica's log since the drill started"
    );
    const withFallback = lines.find(line => (logFieldValue(line, 'fallback_kicks') ?? 0) > 0);
    assert.ok(
      withFallback,
      'every reconcile pass reported fallback_kicks=0 — the sweep never raced the connected ' +
        `household peers. Last pass line: ${lines.at(-1)?.slice(0, 240)}`
    );
    assert.ok(
      (logFieldValue(withFallback, 'connected_peers') ?? 0) > 0,
      'the fallback fired with no connected peers to race — it cannot have been a real fallback'
    );
    state.sweepPassLine = withFallback;
  }
);

Then(
  'the sweep outcome reports the kick as an inventory-blind fallback',
  function (this: E2EWorld) {
    const state = drill(this);
    const line = state.sweepPassLine;
    assert.ok(line, 'no reconcile pass line was captured by the previous step');
    const fallback = logFieldValue(line, 'fallback_kicks') ?? 0;
    const kicks = logFieldValue(line, 'kicks_fired') ?? 0;
    assert.ok(
      fallback > 0,
      `the pass reported fallback_kicks=${fallback}: the kick was not an inventory-blind fallback`
    );
    assert.ok(
      kicks >= fallback,
      `the pass reported ${kicks} kicks but ${fallback} inventory-blind ones — the outcome does ` +
        'not add up'
    );
  }
);

Given(
  "Jessica's peer projection holds no custody-blob commitment rows",
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const jessica = peerByName(state, 'jessica');
    const credits = await custodyCredits(jessica.url);
    // FINDING: elohim-storage exposes no admin/test verb that clears a peer's
    // custody-blob projection, so this precondition cannot be MADE from a2o —
    // only observed. A non-empty projection fails here naming that gap rather
    // than letting the scenario pass against an already-converged peer.
    assert.equal(
      credits.length,
      0,
      `Jessica's projection already holds ${credits.length} custody-blob rows. This drill needs a ` +
        'wiped projection, and elohim-storage has no verb to clear one (no DELETE on ' +
        '/db/rea_commitments, no /admin projection-reset) — establish it out of band or add the verb'
    );
  }
);

Given(
  "another household peer's projection holds the custody-blob commitment rows",
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const others = state.peers.filter(peer => peer.name !== 'jessica');
    const counts: string[] = [];
    let holder = 0;
    for (const peer of others) {
      const credits = await custodyCredits(peer.url);
      counts.push(`${peer.name}=${credits.length}`);
      if (credits.length > 0) holder += 1;
    }
    assert.ok(
      holder > 0,
      `no other household peer holds custody-blob commitment rows (${counts.join(', ')}) — there ` +
        'is nothing for Jessica to discover her gaps from'
    );
  }
);

When(
  "the projection reconcile sweep runs on Jessica's peer",
  { timeout: SWEEP_WINDOW_MS + 30_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const jessica = peerByName(state, 'jessica');
    const before = await peerStatus(jessica);
    const baseline = before.projectionReconcile?.sweeps ?? 0;
    state.baselines.set('jessica:healed', before.projectionReconcile?.healedTotal ?? 0);
    await retry(
      async () => {
        const status = await peerStatus(jessica);
        const sweeps = status.projectionReconcile?.sweeps;
        assert.ok(
          typeof sweeps === 'number',
          'Jessica reports no projectionReconcile block on /p2p/status — the reconcile task is ' +
            'not running (missing lamad HcClient, pool, or disabled)'
        );
        assert.ok(
          sweeps > baseline,
          `no projection reconcile sweep completed on Jessica since the drill (sweeps=${sweeps})`
        );
      },
      {
        maxAttempts: 200,
        initialDelayMs: 2_000,
        backoffFactor: 1.1,
        maxDelayMs: 15_000,
        timeoutMs: SWEEP_WINDOW_MS,
      }
    );
  }
);

Then(
  "Jessica's peer discovers the missing commitment ids from a household peer",
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const status = await peerStatus(peerByName(state, 'jessica'));
    const peersAsked = status.projectionReconcile?.peersAsked ?? 0;
    assert.ok(
      peersAsked > 0,
      'the reconcile sweep asked no peer for an inventory — discovery never happened, so a ' +
        'caught-up claim would be vacuous'
    );
  }
);

Then(
  "each discovered commitment is healed from Jessica's own conductor",
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const status = await peerStatus(peerByName(state, 'jessica'));
    const reconcile = status.projectionReconcile;
    assert.ok(reconcile, 'Jessica reports no projectionReconcile block');
    const healedBefore = state.baselines.get('jessica:healed') ?? 0;
    assert.ok(
      (reconcile.healedTotal ?? 0) > healedBefore,
      `Jessica healed nothing this drill (healedTotal ${reconcile.healedTotal ?? 0}, was ` +
        `${healedBefore}) — discovery without healing is not recovery`
    );
    assert.equal(
      reconcile.failed ?? 0,
      0,
      `${reconcile.failed} discovered commitments were invisible to Jessica's own conductor — ` +
        'the heal source is the conductor, so a failure there is the whole claim failing'
    );
  }
);

Then(
  "Jessica's peer projection reconcile reports caught up with a non-zero local total",
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const jessica = peerByName(state, 'jessica');
    const status = await peerStatus(jessica);
    assert.equal(
      status.projectionReconcile?.caughtUp,
      true,
      'Jessica does not report caughtUp after the sweep'
    );
    const credits = await custodyCredits(jessica.url);
    assert.ok(
      credits.length > 0,
      'Jessica reports caught up on an EMPTY projection — local_total=0 is exactly the 2026-06-10 ' +
        'shape where convergence existed on paper only'
    );
  }
);

// ===========================================================================
// features/resilience/app-blob-heal-on-read.feature
// ===========================================================================

/** The doorway's own primary peer — the one an /apps read is served from. */
function servingPeerFor(world: E2EWorld): PeerHandle {
  return peerByName(drill(world), servingPeerName('alpha'));
}

Given(
  'content {string} is an {string} with its ZIP blob held by at least one peer',
  { timeout: 120_000 },
  async function (this: E2EWorld, contentId: string, format: string) {
    const state = drill(this);
    let body: Record<string, unknown>;
    try {
      ({ body } = await probeContent(doorwayUrl(this), contentId));
    } catch (error) {
      // A 404 here is a FIXTURE gap, not a heal defect, and the two reds look
      // nothing alike in triage. Say which one this is.
      throw new Error(
        `"${contentId}" is not seeded on this mesh, so the heal-on-read drill has nothing to ` +
          'break and nothing to repair. This scenario names a drill fixture the household seed ' +
          'does not create; either seed an ' +
          format +
          ` under that id, or point the scenario at an ${format} this mesh really carries. ` +
          `Underlying: ${String(error)}`
      );
    }
    assert.equal(
      body['contentFormat'],
      format,
      `"${contentId}" is a ${String(body['contentFormat'])}, not an ${format}`
    );
    const hash = await blobHashFor(this, state, contentId);
    const holders: string[] = [];
    for (const peer of state.peers) {
      if (await localBlobPresent(peer.name, hash)) holders.push(peer.name);
    }
    assert.ok(
      holders.length > 0,
      `no household peer holds the ZIP ${hash} for "${contentId}" — nothing to heal from`
    );
    state.baselines.set(`holders:${contentId}`, holders.length);
  }
);

Given(
  "the serving peer's local blob store is missing the ZIP for {string}",
  { timeout: 120_000 },
  async function (this: E2EWorld, contentId: string) {
    const held = holdDestructive(
      `move the "${contentId}" ZIP out of the doorway's serving peer local store`
    );
    if (held) return held;
    const state = drill(this);
    const hash = await blobHashFor(this, state, contentId);
    const serving = servingPeerFor(this);
    const others = state.peers.filter(peer => peer.name !== serving.name);
    const survivors: string[] = [];
    for (const peer of others) {
      if (await localBlobPresent(peer.name, hash)) survivors.push(peer.name);
    }
    assert.ok(
      survivors.length > 0,
      `only the serving peer ${serving.name} holds ${hash}; removing it would leave no peer to ` +
        'race-fetch from and the scenario would fail for the wrong reason'
    );
    await sidelineBlob(state, serving.name, hash);
    assert.equal(
      await localBlobPresent(serving.name, hash),
      false,
      `${serving.name} still holds ${hash}`
    );
    state.drillStartedAt = new Date().toISOString();
  }
);

Then(
  "the serving peer's local blob store now holds the ZIP for {string}",
  { timeout: 180_000 },
  async function (this: E2EWorld, contentId: string) {
    const state = drill(this);
    const hash = await blobHashFor(this, state, contentId);
    const serving = servingPeerFor(this);
    // finalize_fetch_success persists filesystem-first; the disk is the honest
    // oracle for "now holds", and it cannot be satisfied by another HTTP read.
    await retry(
      async () => {
        assert.ok(
          await localBlobPresent(serving.name, hash),
          `${serving.name} still has no local bytes for ${hash} — the read did not heal it`
        );
      },
      {
        maxAttempts: 60,
        initialDelayMs: 1_000,
        backoffFactor: 1.2,
        maxDelayMs: 10_000,
        timeoutMs: 150_000,
      }
    );
  }
);

Given(
  'the serving peer healed {string} from a peer race-fetch',
  { timeout: 180_000 },
  async function (this: E2EWorld, contentId: string) {
    const state = drill(this);
    const hash = await blobHashFor(this, state, contentId);
    const serving = servingPeerFor(this);
    assert.ok(
      await localBlobPresent(serving.name, hash),
      `${serving.name} does not hold ${hash} — no heal has happened to book an event for`
    );
    const events = await serveBlobEvents(serving.url, hash);
    assert.ok(
      events.length > 0,
      `${serving.name} holds ${hash} but has no serve-blob event for it — these bytes did not ` +
        'arrive by race-fetch'
    );
    state.blobHashes.set(contentId, hash);
  }
);

When(
  'I list economic events with action {string} for the healed blob hash',
  { timeout: 120_000 },
  async function (this: E2EWorld, action: string) {
    const state = drill(this);
    const [contentId] = [...state.blobHashes.keys()];
    assert.ok(contentId, 'no healed content was named earlier in this scenario');
    const hash = state.blobHashes.get(contentId) as string;
    const serving = servingPeerFor(this);
    const { status, text } = await getRaw(
      `${serving.url}/api/v1/economic-events?action=${encodeURIComponent(action)}&limit=500`
    );
    assert.equal(status, 200, `listing ${action} events returned ${status}`);
    state.baselines.set(`events:${hash}`, (await serveBlobEvents(serving.url, hash)).length);
    assert.ok(text.length > 0, 'the delivery ledger returned an empty body');
  }
);

Then(
  'at least one event names the source peer as provider',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const [contentId] = [...state.blobHashes.keys()];
    assert.ok(contentId, 'no healed content was named earlier in this scenario');
    const hash = state.blobHashes.get(contentId) as string;
    const serving = servingPeerFor(this);
    await peerStatus(serving);
    const events = await serveBlobEvents(serving.url, hash);
    assert.ok(events.length > 0, `no serve-blob event for ${hash}`);
    const named = events.filter(
      event => event.provider.length > 0 && event.provider !== serving.peerId
    );
    assert.ok(
      named.length > 0,
      `every serve-blob event for ${hash} names ${serving.name} as its own provider — the steward ` +
        'who actually served the bytes is unrecognized'
    );
  }
);

Given(
  'content {string} with a blob held by a peer the steward replicates from',
  { timeout: 120_000 },
  async function (this: E2EWorld, contentId: string) {
    const state = drill(this);
    const hash = await blobHashFor(this, state, contentId);
    const steward = servingPeerFor(this);
    const holders: string[] = [];
    for (const peer of state.peers) {
      if (peer.name === steward.name) continue;
      if (await localBlobPresent(peer.name, hash)) holders.push(peer.name);
    }
    assert.ok(
      holders.length > 0,
      `no peer other than the steward ${steward.name} holds ${hash} — there is no source to draw ` +
        'from'
    );
  }
);

Given(
  "the steward's local blob store is missing the blob for {string}",
  { timeout: 120_000 },
  async function (this: E2EWorld, contentId: string) {
    const held = holdDestructive(`move the "${contentId}" blob out of the steward's local store`);
    if (held) return held;
    const state = drill(this);
    const hash = await blobHashFor(this, state, contentId);
    const steward = servingPeerFor(this);
    await sidelineBlob(state, steward.name, hash);
    assert.equal(await localBlobPresent(steward.name, hash), false);
    state.drillStartedAt = new Date().toISOString();
  }
);

When('the steward proactively draws the blob during content replication', function () {
  // PENDING — named gap: the proactive quilt-draw (finalize_quilt_draw) is
  // driven by the replication loop's own timer. elohim-storage exposes no verb
  // that triggers ONE draw on demand (/api/v1/pins/{epr}/pull is the pin
  // acquisition path, not the quilt draw), so a2o cannot make this When happen
  // rather than wait for it. Scenario stays @wip until such a verb exists.
  return 'pending';
});

Then(
  "the steward's local blob store now holds the blob for {string}",
  { timeout: 180_000 },
  async function (this: E2EWorld, contentId: string) {
    const state = drill(this);
    const hash = await blobHashFor(this, state, contentId);
    const steward = servingPeerFor(this);
    await retry(
      async () => {
        assert.ok(
          await localBlobPresent(steward.name, hash),
          `${steward.name} still has no local bytes for ${hash}`
        );
      },
      {
        maxAttempts: 60,
        initialDelayMs: 1_000,
        backoffFactor: 1.2,
        maxDelayMs: 10_000,
        timeoutMs: 150_000,
      }
    );
  }
);

Then(
  'listing economic events with action {string} for that blob hash returns at least one event',
  { timeout: 120_000 },
  async function (this: E2EWorld, action: string) {
    assert.equal(action, 'serve-blob', `this step reads serve-blob events, not "${action}"`);
    const state = drill(this);
    const [contentId] = [...state.blobHashes.keys()];
    assert.ok(contentId, 'no content was resolved earlier in this scenario');
    const hash = state.blobHashes.get(contentId) as string;
    const steward = servingPeerFor(this);
    const since = state.drillStartedAt ?? '';
    const events = (await serveBlobEvents(steward.url, hash)).filter(
      event => event.hasPointInTime >= since
    );
    assert.ok(
      events.length > 0,
      `the proactive draw moved ${hash} but booked no serve-blob event after ${since} — the ` +
        'replication path is back on a bare blob_store.store()'
    );
  }
);

Then(
  'that event names the peer it drew from as provider',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const state = drill(this);
    const [contentId] = [...state.blobHashes.keys()];
    assert.ok(contentId, 'no content was resolved earlier in this scenario');
    const hash = state.blobHashes.get(contentId) as string;
    const steward = servingPeerFor(this);
    await peerStatus(steward);
    const since = state.drillStartedAt ?? '';
    const events = (await serveBlobEvents(steward.url, hash)).filter(
      event => event.hasPointInTime >= since
    );
    assert.ok(events.length > 0, `no serve-blob event for ${hash} after ${since}`);
    for (const event of events) {
      assert.ok(event.provider.length > 0, 'a serve-blob event names no provider');
      assert.notEqual(
        event.provider,
        steward.peerId,
        'the draw credited the steward for serving itself'
      );
    }
  }
);

Given(
  'content {string} is an {string} whose ZIP blob no peer holds',
  { timeout: 120_000 },
  async function (this: E2EWorld, contentId: string, format: string) {
    const state = drill(this);
    let body: Record<string, unknown>;
    try {
      ({ body } = await probeContent(doorwayUrl(this), contentId));
    } catch (error) {
      // A 404 here is a FIXTURE gap, not a heal defect, and the two reds look
      // nothing alike in triage. Say which one this is.
      throw new Error(
        `"${contentId}" is not seeded on this mesh, so the heal-on-read drill has nothing to ` +
          'break and nothing to repair. This scenario names a drill fixture the household seed ' +
          'does not create; either seed an ' +
          format +
          ` under that id, or point the scenario at an ${format} this mesh really carries. ` +
          `Underlying: ${String(error)}`
      );
    }
    assert.equal(
      body['contentFormat'],
      format,
      `"${contentId}" is a ${String(body['contentFormat'])}, not an ${format}`
    );
    const hash = await blobHashFor(this, state, contentId);
    for (const peer of state.peers) {
      assert.equal(
        await localBlobPresent(peer.name, hash),
        false,
        `${peer.name} holds ${hash}; this scenario needs an orphaned blob so the 404 is the ` +
          'honest answer'
      );
    }
  }
);

Then(
  'the doorway response body contains {string}',
  async function (this: E2EWorld, expected: string) {
    // The doorway response is captured by steps/protocol/landing-page-dogfood's
    // `I GET {string} from the doorway`, which stores the undrained Response on
    // the world. Reading its body here is the first consumption.
    const captured = (this as E2EWorld & { doorwayResponse?: Response }).doorwayResponse;
    assert.ok(captured, 'no doorway response captured — run the GET step first');
    const text = await captured.text();
    assert.ok(
      text.includes(expected),
      `doorway body did not name "${expected}"; it said: ${text.slice(0, 240)}`
    );
  }
);
