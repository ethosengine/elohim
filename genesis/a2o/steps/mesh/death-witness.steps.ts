/* eslint-disable sonarjs/publicly-writable-directories -- this owned-substrate drill reads the local mesh's declared /tmp pid registry */
/**
 * Stations 1 and 2 of the runtime death-witness story.
 *
 * Station 1 controls only the conductor child already running under Jessica's
 * ark; station 2 only READS the peers that custody her spool. Neither starts
 * mesh processes: the operator owns the foreground mesh, and these steps refuse
 * a mesh that was not launched with ark parentage.
 */

import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { Given, Then, When } from '@cucumber/cucumber';

import { request } from 'undici';

import { getRaw } from '../../src/framework/dataplane/surfaces.js';
import {
  loadHouseholdMeshFixture,
  requireFixturePeerPid,
  requireFixtureStoragePeer,
} from '../../src/framework/fixtures/household-mesh.js';
import { E2EWorld } from '../../src/framework/world.js';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, '../../../..');
const LOCAL_DEV_DIR = resolve(REPO_ROOT, 'elohim/holochain/local-dev');
const MESH_DIR = process.env['MESH_DIR'] ?? '/tmp/elohim-local-mesh';
const ARK_BIN =
  process.env['ARK_BIN'] ?? '/projects/.cargo-target-pool/family/dev/elohim/dev/debug/ark';
const ARK_PRECONDITION = 'mesh is not ark-launched: start it with MESH_CONDUCTOR_LAUNCH=ark';
const HOUSEHOLD_PEERS = ['jessica', 'matthew', 'james'] as const;
const JESSICA = 'jessica';

type HouseholdPeer = (typeof HOUSEHOLD_PEERS)[number];

interface ProcessPassport {
  name: string;
  artifact_path: string;
  pid: number | null;
  ready: boolean;
}

interface Passport {
  processes: ProcessPassport[];
}

interface WitnessListRow {
  cid: string;
  incident: string;
  process: string;
  pid: number;
}

interface DeathWitness {
  artifact_path: string;
  artifact_sha256: string;
  uptime_ms: number;
  exit: {
    class: string;
    signal?: number;
  };
  last_stderr: string[];
  last_stdout: string[];
  last_intent: {
    action: unknown;
  } | null;
  passport: Passport;
}

interface DeathWitnessState {
  killedPid?: number;
  priorWitnessCount?: number;
  priorIncidents?: Set<string>;
  witnessRow?: WitnessListRow;
  witness?: DeathWitness;
  /** Station 2: the custodians whose pledge the Given step verified. */
  custodians?: CustodianHandle[];
  /** Station 2: Jessica's agent key, as the pledges name her. */
  wardAgent?: string;
  /** Station 2: the one sha256 digest behind every rendering of the witness. */
  witnessDigest?: string;
}

const scenarioStates = new WeakMap<E2EWorld, DeathWitnessState>();

function scenario(world: E2EWorld): DeathWitnessState {
  const existing = scenarioStates.get(world);
  if (existing) return existing;
  const created: DeathWitnessState = {};
  scenarioStates.set(world, created);
  return created;
}

function arkDir(peer: HouseholdPeer): string {
  return resolve(LOCAL_DEV_DIR, peer, 'ark');
}

function berthPath(peer: HouseholdPeer): string {
  return resolve(arkDir(peer), 'berth.json');
}

function passportPath(peer: HouseholdPeer): string {
  return resolve(arkDir(peer), 'passport.json');
}

function arkPidPath(peer: HouseholdPeer): string {
  return resolve(MESH_DIR, 'pids', `ark-${peer}`);
}

function readJson<T>(path: string, description: string): T {
  try {
    return JSON.parse(readFileSync(path, 'utf8')) as T;
  } catch (error) {
    throw new Error(`cannot read ${description} at ${path}: ${String(error)}`);
  }
}

function readPassport(peer: HouseholdPeer): Passport {
  return readJson<Passport>(passportPath(peer), `${peer}'s ark passport`);
}

function conductor(passport: Passport, peer: HouseholdPeer): ProcessPassport {
  const process = passport.processes[0];
  assert.ok(process, `${peer}'s passport has no conductor process`);
  assert.equal(process.name, 'conductor', `${peer}'s first passport process is not conductor`);
  return process;
}

function requirePid(value: number | null | undefined, description: string): number {
  assert.ok(
    typeof value === 'number' && Number.isSafeInteger(value) && value > 1,
    `${description} is not a safe process id: ${String(value)}`
  );
  return value;
}

function recordedArkPid(peer: HouseholdPeer): number {
  const [rawPid] = readFileSync(arkPidPath(peer), 'utf8').trim().split(/\s+/);
  return requirePid(Number(rawPid), `${peer}'s recorded ark pid`);
}

function parentPid(childPid: number, peer: HouseholdPeer): number {
  const status = readFileSync(`/proc/${childPid}/status`, 'utf8');
  const match = /^PPid:\s+(\d+)$/m.exec(status);
  assert.ok(match, `${peer}'s conductor /proc status has no PPid field`);
  return requirePid(Number(match[1]), `${peer}'s conductor parent pid`);
}

function runArk(args: string[]): string {
  const result = spawnSync(ARK_BIN, args, {
    cwd: REPO_ROOT,
    encoding: 'utf8',
    env: process.env,
    maxBuffer: 4 * 1024 * 1024,
    timeout: 5_000,
  });
  assert.equal(
    result.error,
    undefined,
    `ark ${args.join(' ')} could not run: ${result.error?.message ?? 'unknown spawn error'}`
  );
  assert.equal(
    result.status,
    0,
    `ark ${args.join(' ')} exited ${String(result.status)}:\n${result.stderr ?? ''}`
  );
  return result.stdout ?? '';
}

function listWitnesses(peer: HouseholdPeer): WitnessListRow[] {
  const value = JSON.parse(runArk(['witness', 'ls', '--berth', berthPath(peer)])) as unknown;
  assert.ok(Array.isArray(value), `ark witness ls for ${peer} did not return a JSON array`);
  return value as WitnessListRow[];
}

function selectedWitness(world: E2EWorld): DeathWitness {
  const state = scenario(world);
  if (state.witness) return state.witness;
  assert.ok(state.witnessRow, 'no new witness was selected by the polling step');
  state.witness = JSON.parse(
    runArk(['witness', 'show', '--berth', berthPath(JESSICA), state.witnessRow.cid])
  ) as DeathWitness;
  return state.witness;
}

async function wait(milliseconds: number): Promise<void> {
  return new Promise(resolveWait => setTimeout(resolveWait, milliseconds));
}

function intentActionName(action: unknown): string | undefined {
  if (typeof action === 'string') return action;
  if (action && typeof action === 'object' && 'restart' in action) return 'restart';
  return undefined;
}

Given('the household mesh is three storage peers: Jessica, Matthew, and James', () => {
  const fixture = loadHouseholdMeshFixture();
  const peers = Object.entries(fixture.storagePeers ?? {})
    .filter(([, peer]) => typeof peer.url === 'string' && peer.url.length > 0)
    .map(([name]) => name)
    .sort((first, second) => first.localeCompare(second));
  assert.deepEqual(
    peers,
    [...HOUSEHOLD_PEERS].sort((first, second) => first.localeCompare(second)),
    `the household fixture must resolve exactly Jessica, Matthew, and James; got ${peers.join(', ')}`
  );
});

Given("each peer's conductor is running as a child of that peer's envelope", () => {
  for (const peer of HOUSEHOLD_PEERS) {
    if (!existsSync(passportPath(peer)) || !existsSync(arkPidPath(peer))) {
      throw new Error(ARK_PRECONDITION);
    }
    const process = conductor(readPassport(peer), peer);
    assert.equal(process.ready, true, `${peer}'s ark passport does not mark conductor ready`);
    const childPid = requirePid(process.pid, `${peer}'s conductor pid`);
    assert.equal(
      parentPid(childPid, peer),
      recordedArkPid(peer),
      `${peer}'s conductor is not a child of its recorded ark process`
    );
  }
});

When("Jessica's conductor is killed with SIGKILL", function (this: E2EWorld) {
  const state = scenario(this);
  const childPid = requirePid(
    conductor(readPassport(JESSICA), JESSICA).pid,
    "Jessica's conductor pid"
  );
  const existing = listWitnesses(JESSICA);
  state.killedPid = childPid;
  state.priorWitnessCount = existing.length;
  state.priorIncidents = new Set(existing.map(row => row.incident));
  state.witnessRow = undefined;
  state.witness = undefined;
  process.kill(childPid, 'SIGKILL');
});

Then(
  "within 10 seconds Jessica's peer lists a death witness for a new incident",
  { timeout: 12_000 },
  async function (this: E2EWorld) {
    const state = scenario(this);
    assert.ok(state.priorWitnessCount !== undefined, 'the kill step recorded no witness baseline');
    assert.ok(state.priorIncidents, 'the kill step recorded no incident baseline');
    const deadline = Date.now() + 10_000;
    let latest: WitnessListRow[] = [];

    while (Date.now() <= deadline) {
      latest = listWitnesses(JESSICA);
      if (latest.length === state.priorWitnessCount + 1) {
        const row = latest.find(candidate => !state.priorIncidents?.has(candidate.incident));
        if (row) {
          state.witnessRow = row;
          return;
        }
      } else if (latest.length > state.priorWitnessCount + 1) {
        assert.fail(
          `Jessica gained ${latest.length - state.priorWitnessCount} witness rows; expected one death`
        );
      }
      await wait(Math.min(500, Math.max(1, deadline - Date.now())));
    }

    assert.fail(
      `Jessica did not gain exactly one witness for a new incident within 10 seconds ` +
        `(before ${state.priorWitnessCount}, after ${latest.length})`
    );
  }
);

Then(
  'the witness names the signal, how long the conductor ran, and its last stderr lines',
  function (this: E2EWorld) {
    const witness = selectedWitness(this);
    assert.equal(witness.exit.class, 'signaled', 'the witness does not classify a signal death');
    assert.equal(witness.exit.signal, 9, 'the witness does not name SIGKILL (signal 9)');
    assert.ok(witness.uptime_ms > 0, `the witness reports invalid uptime ${witness.uptime_ms}`);
    assert.ok(Array.isArray(witness.last_stderr), 'the witness has no stderr ring');
    assert.ok(Array.isArray(witness.last_stdout), 'the witness has no stdout ring');
    if (witness.last_stderr.length === 0) {
      assert.ok(
        witness.last_stdout.length > 0,
        'last_stderr was empty on this conductor build, and the stdout fallback was empty too'
      );
    }
  }
);

Then(
  "the witness carries the envelope's own last decision about that conductor",
  function (this: E2EWorld) {
    const action = selectedWitness(this).last_intent?.action;
    const actionName = intentActionName(action);
    assert.ok(
      actionName === 'spawn' || actionName === 'restart',
      `the witness last intent is neither spawn nor restart: ${JSON.stringify(action)}`
    );
  }
);

Then(
  'the witness names the hash of the conductor program the envelope actually started',
  function (this: E2EWorld) {
    const witness = selectedWitness(this);
    assert.equal(
      witness.artifact_sha256,
      runArk(['hash', witness.artifact_path]).trim(),
      'the witness artifact hash differs from the bytes at its recorded artifact path'
    );
  }
);

Then(
  "the witness carries Jessica's passport as it stood at the moment of death",
  function (this: E2EWorld) {
    const state = scenario(this);
    assert.ok(state.killedPid !== undefined, 'the kill step recorded no conductor pid');
    const process = selectedWitness(this).passport.processes[0];
    assert.ok(process, "the witness passport has no first process for Jessica's conductor");
    assert.equal(
      process.pid,
      state.killedPid,
      "the witness passport does not carry Jessica's killed conductor pid"
    );
  }
);

// ---------------------------------------------------------------------------
// Station 2 — the custodians Jessica already has hold the witness
// ---------------------------------------------------------------------------

/**
 * The peers who stand as custodians for Jessica in this drill. Named here, not
 * derived from the fixture, because the scenario names them: a mesh that grew a
 * fourth peer must not silently change who the story is about.
 */
const CUSTODIANS: readonly HouseholdPeer[] = ['matthew', 'james'];

/** The station's own wall-clock budget and poll interval — real time, no fake clock. */
const CUSTODY_WINDOW_MS = 60_000;
const CUSTODY_POLL_MS = 1_000;

/**
 * The sentence a reader must be given when the household never made the
 * promise this station measures. Absent pledges are a MESH SETUP fact, not a
 * custody failure, and the two must not look alike in triage.
 */
const SPOOL_PRECONDITION = 'run `just mesh prologue` — no custody-spool rows';

/** Commitment states that retire a pledge (mirrors `spool_custody_author::RETIRED_STATES`). */
const RETIRED_COMMITMENT_STATES = new Set([
  'cancelled',
  'superseded',
  'revoked',
  'completed',
  'rejected',
]);

const STORAGE_DIR_KEY = 'STORAGE_DIR=';

interface CustodianHandle {
  name: HouseholdPeer;
  url: string;
  /** The Holochain agent key a `custody-spool`/`custody-blob` row names as provider. */
  agentPubKey: string;
}

function meshPeer(name: HouseholdPeer): { url: string; agentPubKey?: string } {
  const peer = requireFixtureStoragePeer(loadHouseholdMeshFixture(), name);
  return { url: peer.url, agentPubKey: peer.agentPubKey };
}

/**
 * A peer's Holochain agent key as the household fixture records it
 * (`hc-mesh.sh refresh_fixture_pids` stamps it from the running process).
 * Commitments are written in THIS namespace; a transport id is a different
 * namespace and is never compared against one of these.
 */
function requireAgentKey(name: HouseholdPeer): string {
  const key = meshPeer(name).agentPubKey;
  assert.ok(
    key,
    `the household fixture records no agentPubKey for "${name}", so no commitment row can be ` +
      'attributed to it — restart the mesh so hc-mesh.sh restamps the fixture'
  );
  return key;
}

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
 * The one sha256 digest behind every rendering of this witness.
 *
 * A death witness wears three addresses for one set of bytes: the witness CID
 * the ark wrote (`bafyrei…`, dag-cbor codec `0x71`), the pantry blob's own CID
 * (`bafkrei…`, raw codec `0x55`), and the legacy `sha256-<hex>` marker the
 * custody and inventory planes speak. They differ in CODEC, never in digest —
 * so "the same content hash" is a comparison of DIGESTS. Comparing the CID
 * strings of two codecs would report a false mismatch on identical bytes.
 *
 * Mirrors `BlobStore::parse_content_address`, which keys on the multihash
 * digest and accepts all three.
 */
function witnessDigestOf(address: string): string {
  const prefixed = /^sha256-([0-9a-f]{64})$/.exec(address);
  if (prefixed) return prefixed[1];
  if (/^[0-9a-f]{64}$/.test(address)) return address;
  assert.match(
    address,
    /^b[a-z2-7]+$/,
    `witness address "${address}" is neither sha256-<hex> nor a base32 CIDv1`
  );
  const bytes = decodeBase32Lower(address.slice(1));
  assert.ok(
    bytes.length === 36 &&
      bytes[0] === 0x01 &&
      (bytes[1] === 0x55 || bytes[1] === 0x71) &&
      bytes[2] === 0x12 &&
      bytes[3] === 0x20,
    `CID "${address}" is not a v1 raw-or-dag-cbor CID over sha2-256`
  );
  return [...bytes.slice(4)].map(byte => byte.toString(16).padStart(2, '0')).join('');
}

function digestOrUndefined(address: string): string | undefined {
  if (address === '') return undefined;
  try {
    return witnessDigestOf(address);
  } catch {
    return undefined;
  }
}

function sha256HexOfBytes(bytes: Uint8Array): string {
  return createHash('sha256').update(bytes).digest('hex');
}

/** GET a JSON list surface that answers either as a bare array or as `{items}`. */
async function listRows(url: string, what: string): Promise<Record<string, unknown>[]> {
  const { status, text } = await getRaw(url);
  assert.equal(status, 200, `GET ${url} → ${status} (${what})`);
  const parsed = JSON.parse(text) as unknown;
  if (Array.isArray(parsed)) return parsed as Record<string, unknown>[];
  const items = (parsed as Record<string, unknown>)['items'];
  return Array.isArray(items) ? (items as Record<string, unknown>[]) : [];
}

/** `resourceClassifiedAs` is a JSON list by contract; older rows carry it bare. */
function classifications(row: Record<string, unknown>): string[] {
  const value = row['resourceClassifiedAs'];
  if (Array.isArray(value)) return value.map(entry => String(entry));
  return value === undefined || value === null ? [] : [String(value)];
}

/**
 * A peer's live `STORAGE_DIR`, read from the running process rather than
 * reconstructed from a convention, so the drill looks where the peer actually
 * writes. Mirrors household-chaos.steps.ts's `peerStorageDir`.
 */
async function peerStorageDir(name: HouseholdPeer): Promise<string> {
  const pid = requireFixturePeerPid(loadHouseholdMeshFixture(), name);
  let cmdline: string;
  let environ: string;
  try {
    cmdline = await readFile(`/proc/${pid}/cmdline`, 'utf8');
    environ = await readFile(`/proc/${pid}/environ`, 'utf8');
  } catch (error) {
    throw new Error(`cannot read /proc/${pid} for household peer "${name}": ${String(error)}`);
  }
  assert.ok(
    cmdline.includes('elohim-storage'),
    `the fixture's pid ${pid} for "${name}" is not an elohim-storage process — it is stale`
  );
  const value = environ
    .split('\0')
    .find(pair => pair.startsWith(STORAGE_DIR_KEY))
    ?.slice(STORAGE_DIR_KEY.length);
  assert.ok(value, `household peer "${name}" (pid ${pid}) has no STORAGE_DIR in its environment`);
  return value;
}

/** `$STORAGE_DIR/blobs/blobs/<first-4-hex>/sha256-<hex>` */
function blobFilePath(storageDir: string, digest: string): string {
  return join(storageDir, 'blobs', 'blobs', digest.slice(0, 4), `sha256-${digest}`);
}

function requireCustodians(state: DeathWitnessState): CustodianHandle[] {
  assert.ok(
    state.custodians?.length === CUSTODIANS.length,
    'no custodian pledge was resolved — the Given step for this scenario did not run'
  );
  return state.custodians;
}

/**
 * The witness this station is about, discovered inside the station's own
 * budget. Station 1's polling step may have already selected it (when both
 * stations run in one scenario); station 2 stands alone, so it looks for the
 * new incident itself against the baseline the kill step recorded.
 */
async function awaitWitnessDigest(state: DeathWitnessState, deadline: number): Promise<string> {
  assert.ok(state.priorIncidents, 'the kill step recorded no incident baseline');
  while (state.witnessRow === undefined) {
    const row = listWitnesses(JESSICA).find(
      candidate => !state.priorIncidents?.has(candidate.incident)
    );
    if (row) {
      state.witnessRow = row;
      break;
    }
    if (Date.now() > deadline) {
      assert.fail(
        "Jessica's ark wrote no witness for a new incident, so there is nothing for a custodian " +
          'to hold — station 1 is the failure, not custody'
      );
    }
    await wait(CUSTODY_POLL_MS);
  }
  const digest = witnessDigestOf(state.witnessRow.cid);
  state.witnessDigest = digest;
  return digest;
}

/**
 * Does this custodian's own peer PROVIDE a `custody-blob` for the witness?
 * `provider` is the custodian's agent key by the T16 convention (custodian is
 * provider, the content's steward is receiver).
 */
async function custodyBlobMissing(
  custodian: CustodianHandle,
  digest: string
): Promise<string | undefined> {
  const rows = await listRows(
    `${custodian.url}/api/v1/commitments?action=custody-blob&limit=500`,
    `${custodian.name}'s custody-blob commitments`
  );
  const named = rows.some(
    row =>
      String(row['provider'] ?? '') === custodian.agentPubKey &&
      classifications(row).some(marker => digestOrUndefined(marker) === digest)
  );
  return named
    ? undefined
    : `no custody-blob commitment on ${custodian.name} names it as provider for sha256-${digest}`;
}

/** Does the custodian hold bytes that hash to the witness digest? */
async function custodyBytesMissing(
  custodian: CustodianHandle,
  digest: string
): Promise<string | undefined> {
  const file = blobFilePath(await peerStorageDir(custodian.name), digest);
  if (!existsSync(file)) {
    return `${custodian.name} is credited with custody of sha256-${digest} but holds no bytes at ${file}`;
  }
  const held = sha256HexOfBytes(await readFile(file));
  return held === digest
    ? undefined
    : `${custodian.name}'s copy at ${file} hashes to sha256-${held}, not sha256-${digest}`;
}

/**
 * What the custodian's own byte route says about the witness.
 *
 * TWO answers are correct here and the difference is a reach fact, not a
 * custody fact. A witness content row is written at `reach: private`
 * (`spool_ingest`), and `private` sits above `community`, so once the
 * replication cycle has carried that row to this custodian
 * `blob_reach::blob_serve_verdict` returns `Authorize` — and `http.rs` fails
 * that branch CLOSED for every caller, identified or not. That refusal IS the
 * contract (station 3b names it), so a 403 carrying `requiredReach` is
 * accepted here and the digest is proven from the bytes on disk instead. A 200
 * is accepted only when the served bytes hash to the witness digest; anything
 * else — 404, 5xx, or a 200 over different bytes — is a real failure.
 */
async function blobRouteDisagrees(
  custodian: CustodianHandle,
  cid: string,
  digest: string
): Promise<string | undefined> {
  const url = `${custodian.url}/blob/${encodeURIComponent(cid)}`;
  const { statusCode, body } = await request(url, {
    method: 'GET',
    headersTimeout: 15_000,
    bodyTimeout: 15_000,
  });
  const bytes = new Uint8Array(await body.arrayBuffer());
  if (statusCode === 200) {
    const served = sha256HexOfBytes(bytes);
    return served === digest
      ? undefined
      : `GET ${url} served bytes hashing to sha256-${served}, not sha256-${digest}`;
  }
  const text = Buffer.from(bytes).toString('utf8');
  if (statusCode === 403 && text.includes('requiredReach')) return undefined;
  return `GET ${url} → ${statusCode} (${text.slice(0, 160)})`;
}

/** Everything station 2 claims about ONE custodian, or the first thing missing. */
async function custodyShortfall(
  custodian: CustodianHandle,
  cid: string,
  digest: string
): Promise<string | undefined> {
  return (
    (await custodyBlobMissing(custodian, digest)) ??
    (await custodyBytesMissing(custodian, digest)) ??
    (await blobRouteDisagrees(custodian, cid, digest))
  );
}

/**
 * Every id Jessica answers to. A `serve-blob` event names its provider with
 * whatever id the fetching plane knew — the libp2p transport id on the libp2p
 * plane, the iroh node id on the iroh plane — while commitments name the
 * Holochain agent key. Resolving Jessica to the SET of her ids is how the
 * drill avoids comparing an agent key against a transport id.
 */
async function wardIdentities(): Promise<Set<string>> {
  const ids = new Set<string>([requireAgentKey(JESSICA)]);
  const { url } = meshPeer(JESSICA);
  const { status, text } = await getRaw(`${url}/p2p/status`);
  assert.equal(status, 200, `GET ${url}/p2p/status → ${status} (Jessica's transport ids)`);
  const wire = JSON.parse(text) as Record<string, unknown>;
  for (const key of ['peerId', 'irohNodeId']) {
    const value = wire[key];
    if (typeof value === 'string' && value.length > 0) ids.add(value);
  }
  return ids;
}

Given(
  "Matthew and James have each counter-signed a commitment to custody Jessica's witnesses",
  { timeout: 30_000 },
  async function (this: E2EWorld) {
    const state = scenario(this);
    const ward = requireAgentKey(JESSICA);
    const classification = `spool:witness:${ward}`;
    const custodians: CustodianHandle[] = [];

    for (const name of CUSTODIANS) {
      const { url } = meshPeer(name);
      const agentPubKey = requireAgentKey(name);
      const rows = await listRows(
        `${url}/api/v1/commitments?action=custody-spool&limit=500`,
        `${name}'s custody-spool commitments`
      );
      // Authorship on the custodian's OWN conductor is the counter-signature:
      // provider == that peer's agent key means that peer, not Jessica, wrote
      // the row. A pledge Jessica authored about herself would prove nothing.
      const pledged = rows.some(
        row =>
          String(row['provider'] ?? '') === agentPubKey &&
          String(row['receiver'] ?? '') === ward &&
          classifications(row).includes(classification) &&
          !RETIRED_COMMITMENT_STATES.has(String(row['state'] ?? ''))
      );
      assert.ok(
        pledged,
        `${name} has not counter-signed custody of Jessica's witnesses on its own peer ` +
          `(no live custody-spool row with provider ${agentPubKey}, receiver ${ward}, ` +
          `classified "${classification}"): ${SPOOL_PRECONDITION}`
      );
      custodians.push({ name, url, agentPubKey });
    }

    state.wardAgent = ward;
    state.custodians = custodians;
  }
);

Then(
  'within 60 seconds Matthew and James each hold a copy of the witness with the same content hash',
  { timeout: 90_000 },
  async function (this: E2EWorld) {
    const state = scenario(this);
    const custodians = requireCustodians(state);
    const deadline = Date.now() + CUSTODY_WINDOW_MS;
    const digest = await awaitWitnessDigest(state, deadline);
    const cid = state.witnessRow?.cid ?? '';
    const pending = new Map<HouseholdPeer, string>(
      custodians.map(custodian => [custodian.name, 'not yet checked'])
    );

    while (Date.now() <= deadline) {
      for (const custodian of custodians) {
        if (!pending.has(custodian.name)) continue;
        const shortfall = await custodyShortfall(custodian, cid, digest);
        if (shortfall === undefined) pending.delete(custodian.name);
        else pending.set(custodian.name, shortfall);
      }
      if (pending.size === 0) return;
      await wait(Math.min(CUSTODY_POLL_MS, Math.max(1, deadline - Date.now())));
    }

    const shortfalls = [...pending].map(([name, why]) => `${name}: ${why}`).join('; ');
    assert.fail(
      `Jessica's witness ${cid} (sha256-${digest}) was not held by every custodian within ` +
        `60 seconds — ${shortfalls}`
    );
  }
);

Then(
  'Matthew and James each record on their own peer that they received that witness from Jessica',
  { timeout: 30_000 },
  async function (this: E2EWorld) {
    const state = scenario(this);
    const custodians = requireCustodians(state);
    const digest = state.witnessDigest;
    assert.ok(
      digest,
      'no witness digest was resolved — the custody step for this scenario did not run'
    );
    const jessica = await wardIdentities();

    for (const custodian of custodians) {
      const rows = await listRows(
        `${custodian.url}/api/v1/economic-events?action=serve-blob&limit=500`,
        `${custodian.name}'s serve-blob events`
      );
      const receipts = rows.filter(
        row => digestOrUndefined(String(row['resourceInventoriedAs'] ?? '')) === digest
      );
      assert.ok(
        receipts.length > 0,
        `${custodian.name} holds the witness but records no serve-blob event for ` +
          `sha256-${digest} — it cannot say where the bytes came from`
      );
      const providers = new Set(receipts.map(row => String(row['provider'] ?? '')));
      assert.ok(
        [...providers].some(provider => jessica.has(provider)),
        `${custodian.name} records receiving sha256-${digest} from ` +
          `${[...providers].join(', ') || 'nobody'}, none of which is Jessica ` +
          `(${[...jessica].join(' | ')})`
      );
    }
  }
);
