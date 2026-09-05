/**
 * Steps for `features/delivery/workspace-to-fleet-release.feature` — the rung-5
 * crossing from a developer's own workstation peer to the deployed fleet
 * (plan: the 2026-09-05 rung-5 workspace orchestration plan, Task 4).
 *
 * ## Rails composed, never re-implemented
 *
 *   T1  steps/delivery/coordinator-candidate.ts  — mint a fresh coordinator-only
 *       candidate `.happ` from the bundle the FLEET installed
 *       (`elohim/holochain/local-dev/deployed-bundles/elohim.happ`, the bundle
 *       `hc-start.sh --conductor` joins alpha with), not the household mesh's.
 *   T2  scripts/epr-release-package.ts           — package it, deriving `appliesTo`
 *       from the WORKSPACE peer's own `GET /version` passport (Task 1 of this plan
 *       made that derivation read a crossed role's AUTHORING cell).
 *   T3  scripts/release-ceremony.ts              — `channel create` / `publish` /
 *       `status` / `attestations`, all acting through the workspace conductor.
 *   T4  the workspace peer's own `GET /admin/adoption`                — OUR peer's
 *       controller verdict. This is the developer's own runtime on his own
 *       workstation; no deployed machine is contacted over HTTP anywhere in this
 *       file.
 *
 * ## Why the attestation READ is `release-ceremony.ts attestations`, not the probe
 *
 * The task brief names `scripts/release-attestation-probe.ts` as the observation
 * rail. That script's `main()` AUTHORS attestations on three peers (it is the
 * builder-exclusion proof for the household mesh) and needs three admin-websocket
 * peers — both disqualifying against a live fleet we are forbidden to write to
 * beyond one throwaway channel. What this file composes is that probe's leg 4, the
 * READ: the same `get_attestations_for_subject` link walk plus entry read, applying
 * the same reader's rule, exposed read-only as `release-ceremony.ts attestations`
 * (`countQualifying`). Same rail, same conductor-native path, zero writes.
 *
 * ## Fleet-write guard
 *
 * Every step that writes anything a fleet peer can see — the blob PUT the packager
 * performs, the channel root, the version, the head declaration — is guarded by
 * `A2O_ALLOW_FLEET_WRITE=1`. Without it those steps report `pending` and name the
 * variable. The feature is additionally `@requires:shem`, so the substrate-scope
 * `Before` gate in `steps/common.steps.ts` skips the whole story when the fleet
 * capability is declared unavailable. Both must be true for a single byte to move.
 */

/* eslint-disable sonarjs/no-os-command-from-path --
   this file shells out to `pnpm exec tsx` for the two drivers (T2/T3), the same
   composition posture steps/delivery/runtime-upgrade-propagation.steps.ts uses. */

import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import * as path from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { mintCoordinatorCandidate } from './coordinator-candidate.js';

// ---------------------------------------------------------------------------
// Paths, ports and constants
// ---------------------------------------------------------------------------

/** genesis/a2o/steps/delivery -> genesis/a2o (2 levels up). */
const A2O_ROOT = fileURLToPath(new URL('../../', import.meta.url));
/** genesis/a2o/steps/delivery -> repo root (4 levels up). */
const REPO_ROOT = fileURLToPath(new URL('../../../../', import.meta.url));

/**
 * The channel the fleet's own deployment data enrols every active peer on, in
 * `observe` (Task 3 of this plan). It is a throwaway by construction: no peer is
 * armed to apply it and no promotion ceremony is ever run on it here.
 */
const CHANNEL_ID =
  process.env['A2O_WORKSPACE_CHANNEL_ID'] ?? 'runtime:coordinators:elohim:workspace';

/** The name this file gives the one configured conductor — the developer's own. */
const WORKSPACE_PEER = 'workspace';

/** `hc-start.sh` records the join-alpha conductor's live ports here. */
const HC_PORTS_FILE = path.join(REPO_ROOT, 'elohim/holochain/local-dev/.hc_ports');

/**
 * `hc-start.sh`: the join-alpha profile pins the app port to 4485 precisely so a
 * workspace peer joined to the fleet is never confused with the household mesh's
 * own conductors (4445/4455/4465). It is the one structural signal available
 * without touching the conductor, so it is what the Background asserts.
 */
const JOIN_ALPHA_APP_PORT = Number(process.env['A2O_WORKSPACE_APP_PORT'] ?? 4485);

/** The workspace peer's own elohim-storage (`hc-start.sh --conductor`, STORAGE_PORT). */
const WORKSPACE_STORAGE_URL = process.env['E2E_WORKSPACE_STORAGE_URL'] ?? 'http://127.0.0.1:8090';

/** The bundle `hc-start.sh` joins alpha with — the fleet's installed reality. */
const FLEET_BASELINE_HAPP =
  process.env['E2E_WORKSPACE_BASELINE_HAPP'] ??
  path.join(REPO_ROOT, 'elohim/holochain/local-dev/deployed-bundles/elohim.happ');

const DEPLOYMENTS_JSON = path.join(REPO_ROOT, 'genesis/orchestrator/data/deployments.json');
const RECIPE_PATH = path.join(A2O_ROOT, 'scripts/workspace-release.md');

const RUN_STAMP =
  process.env['A2O_WORKSPACE_RUN_STAMP'] ??
  new Date().toISOString().replace(/\D/g, '').slice(0, 14);
const REPORT_DIR = path.join(
  A2O_ROOT,
  'reports',
  'workspace-release',
  new Date().toISOString().slice(0, 10)
);
const MANIFEST_PATH = path.join(REPORT_DIR, `workspace-release-${RUN_STAMP}.json`);

/** Discipline is declared, never defaulted (the packager refuses a number nobody typed). */
const SOAK_SECS = 30;
const ATTESTATION_THRESHOLD = 1;

/**
 * How long the observing peers are given after the head moves. The controller's
 * sweep interval is 60s, so anything under two sweeps plus gossip would be the
 * fixture's arithmetic failing, not the fleet's.
 */
const OBSERVATION_WINDOW_MS = Number(process.env['A2O_WORKSPACE_OBSERVE_MS'] ?? 180_000);
const POLL_INTERVAL_MS = 10_000;

const ADOPTION_PATH = '/admin/adoption';
const VERSION_PATH = '/version';
const LAMAD_ROLE = 'lamad';
const RELEASE_CHANNELS_KEY = 'ELOHIM_RELEASE_CHANNELS';
const OBSERVE_MODE = 'observe';

const FLEET_WRITE_ENV = 'A2O_ALLOW_FLEET_WRITE';
const PENDING = 'pending' as const;

// ---------------------------------------------------------------------------
// Wire shapes (no `any` — this file's lint gate is 0 errors)
// ---------------------------------------------------------------------------

interface AdoptionVerdict {
  state: string;
  ok?: boolean;
  releaseCid?: string;
}

interface AdoptionChannelRow {
  channelId: string;
  mode: string;
  resolvedHead: { cid: string; tier: string } | null;
  verdict: AdoptionVerdict | null;
  appliedRelease: { cid: string } | null;
}

interface AdoptionReport {
  controller: { running: boolean };
  channels: AdoptionChannelRow[];
}

interface VersionRole {
  role: string;
  dnaHash: string;
  coordinatorWasmHashes: Record<string, string>;
}

interface VersionResponse {
  passport?: { happ?: { roles?: VersionRole[] } };
}

interface RoleBinding {
  dnaHash?: string;
}

interface ReleaseManifestFile {
  channelId?: string;
  artifactClass?: string;
  artifacts?: { blobCid: string; sha256: string; bytes: number }[];
  appliesTo?: { roles?: Record<string, RoleBinding> };
}

interface PublishResult {
  releaseCid: string;
  tier: string;
  actingPeer: string;
  canonical?: boolean;
}

interface StatusPeerRow {
  peer: string;
  reachable: boolean;
  tier?: string;
  /** `resolveElectionOnPeer` in release-ceremony.ts: the elected head's action hash. */
  headActionHash?: string | null;
}

interface StatusReport {
  channelId: string;
  peers: StatusPeerRow[];
}

interface AttestationEvidence {
  releaseCid: string;
  readFrom: string;
  linkedCount: number;
  qualifying: number;
  total: number;
}

interface DeployedHuman {
  name: string;
  suspended?: boolean;
  runtimeConfig?: Record<string, string>;
}

interface DeploymentsFile {
  humans: DeployedHuman[];
}

// ---------------------------------------------------------------------------
// Run state — module level, so the stations compose in file order while each
// `ensure*` stays idempotent and self-contained (same shape as the sibling
// runtime-upgrade-propagation steps).
// ---------------------------------------------------------------------------

interface RunState {
  admin?: number;
  app?: number;
  candidateHapp?: string;
  manifest?: ReleaseManifestFile;
  releaseCid?: string;
  publish?: PublishResult;
  status?: StatusReport;
  adoptionRow?: AdoptionChannelRow;
  attestations?: AttestationEvidence;
  recipe?: string;
  recipeCommands?: string[];
}

const state: RunState = {};

// ---------------------------------------------------------------------------
// Guards — nothing here writes without both an operator env flip and the
// feature's own `@requires:shem` gate.
// ---------------------------------------------------------------------------

function fleetWriteAllowed(): boolean {
  return process.env[FLEET_WRITE_ENV] === '1';
}

/** Reads `hc-start.sh`'s live port record; null when no workspace conductor is up. */
function readConductorPorts(): { admin: number; app: number } | null {
  if (!existsSync(HC_PORTS_FILE)) return null;
  const text = readFileSync(HC_PORTS_FILE, 'utf8');
  const admin = Number(/admin_port=(\d+)/.exec(text)?.[1] ?? Number.NaN);
  const app = Number(/app_port=(\d+)/.exec(text)?.[1] ?? Number.NaN);
  if (!Number.isFinite(admin) || !Number.isFinite(app)) return null;
  return { admin, app };
}

function conductorCsv(): string {
  assert.ok(state.admin && state.app, 'workspace conductor ports were never resolved');
  return `${WORKSPACE_PEER}=${state.admin}:${state.app}`;
}

// ---------------------------------------------------------------------------
// Driver composition (T2/T3 — shell out, never re-implement)
// ---------------------------------------------------------------------------

interface DriverResult {
  status: number;
  stdout: string;
  stderr: string;
}

function runDriver(scriptRelPath: string, args: string[], timeoutMs = 120_000): DriverResult {
  const result = spawnSync('pnpm', ['exec', 'tsx', scriptRelPath, ...args], {
    cwd: A2O_ROOT,
    encoding: 'utf8',
    timeout: timeoutMs,
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error) {
    throw new Error(`failed to spawn "pnpm exec tsx ${scriptRelPath}": ${result.error.message}`);
  }
  return { status: result.status ?? 1, stdout: result.stdout ?? '', stderr: result.stderr ?? '' };
}

/** Every driver verb used here prints exactly one pretty-printed JSON object last. */
function extractLastJson<T>(stdout: string): T {
  const starts = [...stdout.matchAll(/^\{$/gm)].map(match => match.index ?? -1).filter(i => i >= 0);
  assert.ok(starts.length > 0, `no JSON object in driver output: ${stdout.slice(0, 400)}`);
  return JSON.parse(stdout.slice(starts.at(-1) ?? 0)) as T;
}

async function getJson<T>(base: string, routePath: string): Promise<T> {
  const response = await request(`${base}${routePath}`, { method: 'GET' });
  assert.equal(
    response.statusCode,
    200,
    `GET ${base}${routePath} answered ${response.statusCode}, expected 200`
  );
  return (await response.body.json()) as T;
}

async function workspaceAdoptionRow(): Promise<AdoptionChannelRow | undefined> {
  const report = await getJson<AdoptionReport>(WORKSPACE_STORAGE_URL, ADOPTION_PATH);
  return report.channels.find(row => row.channelId === CHANNEL_ID);
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

Given(
  "the workspace peer's conductor is joined to the fleet's network",
  { timeout: 30_000 },
  function () {
    const ports = readConductorPorts();
    if (!ports) {
      // No conductor of our own — the story cannot be told from here at all.
      return PENDING;
    }
    assert.equal(
      ports.app,
      JOIN_ALPHA_APP_PORT,
      `the running conductor's app port is ${ports.app}, not the join-alpha port ${JOIN_ALPHA_APP_PORT} — ` +
        'this looks like an isolated conductor; start the fleet-joined one with `just dev conductor alpha`'
    );
    state.admin = ports.admin;
    state.app = ports.app;
    return undefined;
  }
);

Given(
  "the fleet's deployment data enrols every active peer in release channel {string} in observe mode",
  function (channelId: string) {
    assert.equal(channelId, CHANNEL_ID, 'the story and the steps must name the same channel');
    const deployments = JSON.parse(readFileSync(DEPLOYMENTS_JSON, 'utf8')) as DeploymentsFile;
    const active = deployments.humans.filter(human => human.suspended !== true);
    assert.ok(active.length > 0, 'no active humans in deployments.json');
    const missing = active
      .filter(
        human => human.runtimeConfig?.[RELEASE_CHANNELS_KEY] !== `${channelId}=${OBSERVE_MODE}`
      )
      .map(human => human.name);
    assert.deepEqual(
      missing,
      [],
      `these active peers are not enrolled as ${channelId}=${OBSERVE_MODE}: ${missing.join(', ')}`
    );
  }
);

Given('the workspace peer follows that same channel in observe mode', async function () {
  let row: AdoptionChannelRow | undefined;
  try {
    row = await workspaceAdoptionRow();
  } catch {
    // The developer's own storage is not up (or not following anything) — the
    // recipe's first command has not been run.
    return PENDING;
  }
  if (!row) return PENDING;
  assert.equal(
    row.mode,
    OBSERVE_MODE,
    `the workspace peer follows ${CHANNEL_ID} in "${row.mode}" mode; this story requires observe — ` +
      `restart it with CONDUCTOR_RELEASE_CHANNELS=${CHANNEL_ID}=${OBSERVE_MODE}`
  );
  return undefined;
});

// ---------------------------------------------------------------------------
// Station 1 — mint and package, against this peer's own installed reality
// ---------------------------------------------------------------------------

function ensureCandidate(): string | undefined {
  if (state.candidateHapp) return state.candidateHapp;
  if (!existsSync(FLEET_BASELINE_HAPP)) return undefined;
  mkdirSync(REPORT_DIR, { recursive: true });
  const minted = mintCoordinatorCandidate(
    FLEET_BASELINE_HAPP,
    REPORT_DIR,
    RUN_STAMP,
    `${CHANNEL_ID}#${RUN_STAMP}`,
    'workspace'
  );
  state.candidateHapp = minted.happPath;
  return minted.happPath;
}

Given(
  'matthew has a coordinator change he wants the fleet to see',
  { timeout: 180_000 },
  function () {
    const happ = ensureCandidate();
    if (!happ) return PENDING;
    return undefined;
  }
);

function packageRelease(): void {
  if (state.manifest) return;
  const happ = ensureCandidate();
  assert.ok(happ, `no candidate bundle — the fleet baseline is missing: ${FLEET_BASELINE_HAPP}`);
  const result = runDriver(
    'scripts/epr-release-package.ts',
    [
      '--artifact',
      happ,
      '--artifact-class',
      'coordinator-bundle',
      '--channel-id',
      CHANNEL_ID,
      '--applies-to-from',
      WORKSPACE_STORAGE_URL,
      '--peer',
      WORKSPACE_STORAGE_URL,
      '--soak-secs',
      String(SOAK_SECS),
      '--attestation-threshold',
      String(ATTESTATION_THRESHOLD),
      '--notes',
      'workspace-to-fleet crossing, observe only — no peer is armed to apply this',
      '--out',
      MANIFEST_PATH,
    ],
    300_000
  );
  assert.equal(
    result.status,
    0,
    `epr-release-package failed (${result.status}): ${result.stderr.slice(0, 800)}`
  );
  state.manifest = JSON.parse(readFileSync(MANIFEST_PATH, 'utf8')) as ReleaseManifestFile;
}

When(
  "he packages it as a release manifest against his own peer's installed reality",
  { timeout: 360_000 },
  function () {
    if (!fleetWriteAllowed()) return PENDING;
    packageRelease();
    return undefined;
  }
);

Then(
  'the manifest declares the workspace channel and the same validation-rule identity his peer runs',
  { timeout: 60_000 },
  async function () {
    const manifest = state.manifest;
    if (!manifest) return PENDING;
    assert.equal(manifest.channelId, CHANNEL_ID);
    const version = await getJson<VersionResponse>(WORKSPACE_STORAGE_URL, VERSION_PATH);
    const lamad = version.passport?.happ?.roles?.find(role => role.role === LAMAD_ROLE);
    assert.ok(lamad, `the workspace peer's passport declares no ${LAMAD_ROLE} role`);
    assert.equal(
      manifest.appliesTo?.roles?.[LAMAD_ROLE]?.dnaHash,
      lamad.dnaHash,
      'the manifest binds to a different validation-rule identity than this peer runs'
    );
    return undefined;
  }
);

Then(
  "the artifact is addressed by its own content and served straight from his peer's content-addressed store, so any peer can fetch the exact bytes without asking a pipeline for them",
  { timeout: 60_000 },
  async function () {
    const artifact = state.manifest?.artifacts?.[0];
    if (!artifact) return PENDING;
    assert.match(
      artifact.sha256,
      /^[0-9a-f]{64}$/,
      'the artifact is not addressed by a sha256 digest'
    );
    assert.ok(artifact.bytes > 0, 'the artifact declares zero bytes');
    const response = await request(`${WORKSPACE_STORAGE_URL}/blob/${artifact.blobCid}`, {
      method: 'GET',
    });
    assert.equal(
      response.statusCode,
      200,
      `the workspace peer does not serve its own artifact ${artifact.blobCid} (${response.statusCode})`
    );
    await response.body.dump();
    return undefined;
  }
);

// ---------------------------------------------------------------------------
// Station 2 — publish, by the developer's own peer, with no pipeline in the path
// ---------------------------------------------------------------------------

Given(
  'matthew has packaged the release for channel {string}',
  { timeout: 360_000 },
  function (channelId: string) {
    assert.equal(channelId, CHANNEL_ID);
    if (!fleetWriteAllowed()) return PENDING;
    packageRelease();
    return undefined;
  }
);

function publishRelease(): void {
  if (state.publish) return;
  packageRelease();
  // The channel root is idempotent in intent, not in the zome: a second
  // `channel create` on an existing id is a refusal we deliberately tolerate,
  // because a throwaway channel outlives one run and the fleet's data already
  // names it.
  runDriver('scripts/release-ceremony.ts', [
    'channel',
    'create',
    CHANNEL_ID,
    '--as',
    WORKSPACE_PEER,
    '--conductors',
    conductorCsv(),
  ]);
  const result = runDriver(
    'scripts/release-ceremony.ts',
    [
      'publish',
      MANIFEST_PATH,
      '--as',
      WORKSPACE_PEER,
      '--conductors',
      conductorCsv(),
      '--adoption-url',
      WORKSPACE_STORAGE_URL,
    ],
    180_000
  );
  assert.equal(
    result.status,
    0,
    `release-ceremony publish failed (${result.status}): ${result.stderr.slice(0, 800)}`
  );
  const published = extractLastJson<PublishResult>(result.stdout);
  state.publish = published;
  state.releaseCid = published.releaseCid;
}

When('he publishes it on that channel through his own peer', { timeout: 300_000 }, function () {
  if (!fleetWriteAllowed()) return PENDING;
  publishRelease();
  return undefined;
});

Then(
  "the channel's head is the release he minted, declared staging by that act alone",
  function () {
    const published = state.publish;
    if (!published) return PENDING;
    assert.equal(published.tier, 'staging', 'the publish declared something other than staging');
    assert.ok(published.releaseCid, 'the publish declared no release cid');
    return undefined;
  }
);

Then(
  'the peer that declared the head is his own workstation peer, signing with its own key — nothing was built, pushed, or deployed to move it',
  function () {
    const published = state.publish;
    if (!published) return PENDING;
    assert.equal(
      published.actingPeer,
      WORKSPACE_PEER,
      "the head was declared by a peer other than the developer's own workstation"
    );
    return undefined;
  }
);

// ---------------------------------------------------------------------------
// Station 3 — the crossing, read back through the developer's own conductor
// ---------------------------------------------------------------------------

Given(
  'matthew has published the release on channel {string}',
  { timeout: 400_000 },
  function (channelId: string) {
    assert.equal(channelId, CHANNEL_ID);
    if (!fleetWriteAllowed()) return PENDING;
    publishRelease();
    return undefined;
  }
);

function resolveElection(): StatusReport {
  const result = runDriver('scripts/release-ceremony.ts', [
    'status',
    CHANNEL_ID,
    '--conductors',
    conductorCsv(),
  ]);
  assert.notEqual(
    result.status,
    1,
    `release-ceremony status errored: ${result.stderr.slice(0, 800)}`
  );
  const report = extractLastJson<StatusReport>(result.stdout);
  state.status = report;
  return report;
}

When(
  "the channel's election is resolved through the workspace peer's own conductor on the fleet's network",
  { timeout: 120_000 },
  function () {
    if (!state.releaseCid) return PENDING;
    resolveElection();
    return undefined;
  }
);

Then('the election resolves to exactly the release minted on the workstation', function () {
  const report = state.status;
  if (!report) return PENDING;
  const row = report.peers.find(peer => peer.peer === WORKSPACE_PEER);
  assert.ok(row, 'the status report carries no row for the workspace peer');
  assert.equal(
    row.reachable,
    true,
    'the workspace peer answered nothing — unreachable is not absent'
  );
  const winner = row.headActionHash ?? null;
  assert.equal(
    winner,
    state.releaseCid,
    `the election resolved to ${String(winner)}, not the release minted here (${String(state.releaseCid)})`
  );
});

Then(
  "the resolve is a local read through that peer's own conductor, never a report handed to it by someone else",
  function () {
    const report = state.status;
    if (!report) return PENDING;
    // One configured conductor, and it is ours: the driver's `status` verb reads
    // the election locally (GetStrategy::Local) on each configured peer, so a row
    // for our peer and no other IS the claim that nothing was relayed to us.
    assert.deepEqual(
      report.peers.map(peer => peer.peer),
      [WORKSPACE_PEER],
      "the election was read through a peer other than the developer's own"
    );
    return undefined;
  }
);

// ---------------------------------------------------------------------------
// Station 4 — admissible, and applied by nobody
// ---------------------------------------------------------------------------

Given(
  "the release has crossed to the fleet's network on channel {string}",
  { timeout: 400_000 },
  function (channelId: string) {
    assert.equal(channelId, CHANNEL_ID);
    if (!fleetWriteAllowed()) return PENDING;
    publishRelease();
    resolveElection();
    return undefined;
  }
);

/** Polls OUR OWN peer until its controller has judged this head, or the window ends. */
async function observeWindow(): Promise<AdoptionChannelRow | undefined> {
  const deadline = Date.now() + OBSERVATION_WINDOW_MS;
  let row = await workspaceAdoptionRow();
  while (Date.now() < deadline) {
    if (row?.resolvedHead?.cid === state.releaseCid && row?.verdict) break;
    await sleep(POLL_INTERVAL_MS);
    row = await workspaceAdoptionRow();
  }
  state.adoptionRow = row;
  return row;
}

When(
  'every peer following that channel has been given its observation window',
  { timeout: OBSERVATION_WINDOW_MS + 60_000 },
  async function () {
    if (!state.releaseCid) return PENDING;
    await observeWindow();
    return undefined;
  }
);

Then(
  "the workspace peer's own runtime reports the release verified and admissible, with nothing applied, because observe is the only thing it was asked for",
  function () {
    const row = state.adoptionRow;
    if (!row) return PENDING;
    assert.equal(row.mode, OBSERVE_MODE, 'the workspace peer was not in observe mode');
    assert.equal(row.resolvedHead?.cid, state.releaseCid, 'the peer resolved a different head');
    assert.equal(
      row.verdict?.state,
      'ok',
      `the controller's verdict was "${String(row.verdict?.state)}", not the admissible verdict "ok"`
    );
    assert.equal(
      row.appliedRelease,
      null,
      'something was applied — observe mode applies nothing, ever'
    );
    return undefined;
  }
);

Then(
  "no soak attestation for that release exists anywhere on the network, read through the workspace peer's own conductor — the network's own record that no peer applied it",
  { timeout: 120_000 },
  function () {
    if (!state.releaseCid) return PENDING;
    const result = runDriver('scripts/release-ceremony.ts', [
      'attestations',
      state.releaseCid,
      '--as',
      WORKSPACE_PEER,
      '--conductors',
      conductorCsv(),
    ]);
    assert.equal(
      result.status,
      0,
      `release-ceremony attestations failed (${result.status}): ${result.stderr.slice(0, 800)}`
    );
    const evidence = extractLastJson<AttestationEvidence>(result.stdout);
    state.attestations = evidence;
    assert.equal(
      evidence.linkedCount,
      0,
      `${evidence.linkedCount} attestation(s) are anchored on this release — a peer applied it`
    );
    assert.equal(evidence.qualifying, 0, 'a qualifying soak attestation exists for this release');
    return undefined;
  }
);

Then(
  "the channel's head is still staging, because no promotion ceremony was ever run",
  function () {
    const report = state.status ?? (state.releaseCid ? resolveElection() : undefined);
    if (!report) return PENDING;
    const row = report.peers.find(peer => peer.peer === WORKSPACE_PEER);
    assert.ok(row, 'the status report carries no row for the workspace peer');
    assert.equal(row.tier, 'staging', `the head is "${String(row.tier)}" — something promoted it`);
    return undefined;
  }
);

// ---------------------------------------------------------------------------
// Station 5 — the developer's recipe (needs no substrate at all)
// ---------------------------------------------------------------------------

/**
 * The recipe marks the commands a developer types with a leading `$ ` inside its
 * fenced blocks — the one convention this assertion depends on, stated in the
 * recipe itself so the two cannot drift silently.
 */
function recipeCommands(text: string): string[] {
  return text
    .split('\n')
    .map(line => line.trim())
    .filter(line => line.startsWith('$ '))
    .map(line => line.slice(2).trim());
}

Given(
  'a developer who has never run this ceremony reads the workspace release recipe',
  function () {
    assert.ok(existsSync(RECIPE_PATH), `the workspace release recipe is missing: ${RECIPE_PATH}`);
    state.recipe = readFileSync(RECIPE_PATH, 'utf8');
  }
);

When('the commands it asks them to run are counted', function () {
  assert.ok(state.recipe, 'the recipe was never read');
  state.recipeCommands = recipeCommands(state.recipe);
});

Then('there are five or fewer, covering mint, publish, and observe', function () {
  const commands = state.recipeCommands ?? [];
  assert.ok(commands.length > 0, 'the recipe names no commands at all');
  assert.ok(
    commands.length <= 5,
    `the recipe asks for ${commands.length} commands; the mandate's measure is five or fewer:\n${commands.join('\n')}`
  );
  const text = state.recipe ?? '';
  for (const phase of ['Mint', 'Publish', 'Observe']) {
    assert.ok(text.includes(phase), `the recipe has no ${phase} phase`);
  }
});

Then(
  'the recipe names the one step that is still manual — enrolling a peer in the channel, which is deployment data and a render on the fleet, and one API call on a household mesh',
  function () {
    const text = state.recipe ?? '';
    assert.ok(
      text.includes('/admin/runtime-config/follow'),
      'the recipe does not name the mesh-side enrolment call'
    );
    assert.ok(
      text.includes('deployments.json'),
      'the recipe does not name where fleet enrolment is declared'
    );
  }
);
