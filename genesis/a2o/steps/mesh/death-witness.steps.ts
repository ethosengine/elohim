/* eslint-disable sonarjs/publicly-writable-directories -- this owned-substrate drill reads the local mesh's declared /tmp pid registry */
/**
 * Station 1 of the runtime death-witness story.
 *
 * This file controls only the conductor child already running under Jessica's
 * ark. It never starts mesh processes: the operator owns the foreground mesh,
 * and these steps refuse a mesh that was not launched with ark parentage.
 */

import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { Given, Then, When } from '@cucumber/cucumber';

import { loadHouseholdMeshFixture } from '../../src/framework/fixtures/household-mesh.js';
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
