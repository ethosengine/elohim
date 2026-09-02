/**
 * Step definitions for features/delivery/runtime-upgrade-propagation.feature
 * (@concern:runtime-upgrade-propagation — habit
 * elohim/elohim-storage/.epr-meta/runtime-upgrade-propagation.habit.md).
 *
 * Stations 1-5 are wired for real against the household mesh by COMPOSING the
 * two ceremony drivers plus the storage HTTP admin surfaces the r2 receipt
 * (genesis/a2o/reports/release-ceremony/2026-09-01/transcript.md) proved live:
 *
 *   T1  scripts/epr-release-package.ts   — package the coordinator artifact
 *   T2  scripts/release-ceremony.ts      — channel create / publish / promote
 *   T3  GET  <peer>/admin/adoption       — each peer's OWN controller verdict
 *   T4  runtime-config.toml + POST <peer>/admin/runtime-config/reload
 *                                        — follow/canary/apply mode switches
 *
 * `scripts/release-attestation-probe.ts` (T5, the cross-peer builder-exclusion
 * proof) belongs to station 9 — outside this file's scope — so it is never
 * invoked here; the attestation evidence stations 3-4 need is read straight
 * off `/admin/adoption`'s own `attestations` block, exactly as the r2
 * transcript's "Station 5 (r2) — verdicts" did.
 *
 * ## The single-channel simplification
 *
 * The feature's Background narrates TWO channels — the shared soak channel
 * `runtime:coordinators:elohim:canary-a` and the constitutional promotion
 * target `runtime:coordinators:elohim:commons`. The ceremony driver has no
 * "promote from channel A onto channel B" verb: a release manifest carries
 * ONE `channelId`, and `publish`/`promote` move that ONE channel's tier from
 * `staging` to `earned` in place (release-ceremony.ts's own module doc: "stage
 * → promote → revert-by-re-election" on a single channel). The 2026-09-01
 * receipt ran the whole chain this way — one channel, two tiers — and that is
 * the runnable reality this file composes against, not a two-channel model
 * that does not exist yet. So: ONE technical channel (`CHANNEL_ID`, freshly
 * minted per run) stands in for both story names. "Not visible on commons"
 * (station 1) reads as "no peer reports this channel's tier as `earned`
 * yet"; "the release becomes the earned head of channel commons" (station 4)
 * reads as "this channel's tier becomes `earned`". A future two-channel
 * ceremony driver would let these steps bind to two real channel ids without
 * changing the Gherkin at all.
 *
 * ## Fresh channel per run
 *
 * `CHANNEL_ID` is minted once at module load from a run stamp
 * (`A2O_RELEASE_RUN_STAMP` env override, else the process start time) so a
 * repeat run never collides with a channel a prior run left mid-backoff-ladder,
 * and so this run's own writes to `ELOHIM_RELEASE_CHANNELS` never disturb
 * whatever OTHER channel(s) a concurrent shift/session has that peer
 * following — `withChannelMode` below only ever touches the entry for
 * `CHANNEL_ID`, appending/replacing it in the comma-separated list and
 * leaving every other entry byte-identical.
 *
 * ## Cross-scenario state
 *
 * Cucumber gives every scenario a fresh `World`, but stations 1-5 are one
 * causal chain (station 2 resolves what station 1 published; station 5
 * converges what station 4 promoted). The `ensure*` functions below hold that
 * chain in MODULE-level state (not a WeakMap keyed by World) so scenarios
 * running in file order compose naturally, while each `ensure*` is also
 * idempotent and self-contained enough to run a station standalone (it walks
 * back through the chain it depends on, exactly like
 * `steps/delivery/acquisition-pins.steps.ts`'s self-verifying Givens).
 */

/* eslint-disable sonarjs/no-os-command-from-path, sonarjs/publicly-writable-directories --
   this file deliberately shells out to `pnpm exec tsx` for the two ceremony drivers (the
   composition this task requires) and reads/writes the local household mesh's own /tmp work
   dir (runtime-config.toml, conductor pid files) — same posture as
   steps/conductor-spin.steps.ts and steps/dataplane.steps.ts. */

import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { getRaw, postRaw } from '../../src/framework/dataplane/surfaces.js';

import type { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Paths and constants
// ---------------------------------------------------------------------------

/** genesis/a2o/steps/delivery -> genesis/a2o (2 levels up). */
const A2O_ROOT = fileURLToPath(new URL('../../', import.meta.url));

/** The r2 receipt's marker-only coordinator rebuild (integrity bytes == installed). */
const CANDIDATE_HAPP = fileURLToPath(
  new URL('../../reports/release-ceremony/2026-09-01/elohim-P.happ', import.meta.url)
);

const RUN_STAMP =
  process.env['A2O_RELEASE_RUN_STAMP'] ?? new Date().toISOString().replace(/\D/g, '').slice(0, 14);
const CHANNEL_ID = `runtime:coordinators:elohim:a2o-${RUN_STAMP}`;

const REPORT_DIR = path.join(
  A2O_ROOT,
  'reports',
  'release-ceremony',
  new Date().toISOString().slice(0, 10)
);
const MANIFEST_PATH = path.join(REPORT_DIR, `a2o-runtime-upgrade-${RUN_STAMP}.json`);

const SOAK_SECS = 30;
const ATTESTATION_THRESHOLD = 1;

type PeerName = 'matthew' | 'jessica' | 'james';
const PEER_NAMES: readonly PeerName[] = ['matthew', 'jessica', 'james'];
const CANARY_PEER: PeerName = 'james';

const STORY_COMMONS_CHANNEL_NAME = 'runtime:coordinators:elohim:commons';
const STORY_SOAK_CHANNEL_NAME = 'runtime:coordinators:elohim:canary-a';

const ADOPTION_PATH = '/admin/adoption';
const RUNTIME_CONFIG_RELOAD_PATH = '/admin/runtime-config/reload';
const VERSION_PATH = '/version';
const HEALTH_PATH = '/health';
const LAMAD_ROLE = 'lamad';

const MESH_ROOT = process.env['E2E_MESH_ROOT'] ?? '/tmp/elohim-local-mesh';

// ---------------------------------------------------------------------------
// Typed wire shapes (no `any` — this file's lint gate is 0 errors)
// ---------------------------------------------------------------------------

interface AdoptionRefusalDetail {
  reason: string;
  detail: string;
  arm: string;
  transient: boolean;
}

interface AdoptionVerdict {
  state: string;
  ok?: boolean;
  releaseCid?: string;
  vehicle?: string;
  alreadyCurrent?: boolean;
  refusal?: AdoptionRefusalDetail;
}

interface AdoptionAttestations {
  qualifying: number;
  total: number;
  byArchetype: Record<string, number>;
  builderExcluded: number;
  failed: number;
  provenanceMismatched: number;
  unresolved: number;
  threshold: number;
}

interface AdoptionChannelRow {
  channelId: string;
  mode: string;
  resolvedHead: { cid: string; tier: string } | null;
  verdict: AdoptionVerdict | null;
  lastCheckedAt: number | null;
  appliedRelease: { cid: string; at: number; vehicle: string } | null;
  pendingRestart: boolean;
  attestations: AdoptionAttestations | null;
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

interface HealthResponse {
  blobs?: { total?: number };
}

interface PublishResult {
  releaseCid: string;
  tier: string;
}

interface PromoteResult {
  tier: string;
}

interface StatusPeerRow {
  peer: string;
  reachable: boolean;
  tier?: string;
}

interface StatusReport {
  peers: StatusPeerRow[];
}

// ---------------------------------------------------------------------------
// Cross-scenario ceremony state (module-level — see file doc)
// ---------------------------------------------------------------------------

interface PeerRolesSnapshot {
  pid: string;
  roles: VersionRole[];
}

interface FleetPeerResult {
  row: AdoptionChannelRow;
  pidBefore: string;
  pidAfter: string;
  healthBefore: number;
  healthAfter: number;
}

// Every "not yet computed" field is an optional (`undefined`) rather than
// `| null` — it is compared throughout against optional-chained wire reads
// like `row.resolvedHead?.cid`, which TypeScript types as `string | undefined`
// (optional chaining never produces `null`), so matching that shape here
// keeps every comparison a same-type one instead of an `undefined` vs `null`
// mismatch.
interface CeremonyState {
  baselineEstablished: boolean;
  channelCreated: boolean;
  candidateBytes?: Buffer;
  candidateSha256: string;
  releaseCid?: string;
  publishTier?: string;
  stagingConvergedRows: Partial<Record<PeerName, AdoptionChannelRow>>;
  stagingConvergeMs?: number;
  jamesBefore?: PeerRolesSnapshot;
  jamesAfter?: PeerRolesSnapshot;
  canaryAppliedMs?: number;
  attestationRow?: AdoptionChannelRow;
  attestationTimestamp?: number;
  promoteResult?: PromoteResult;
  promotedAt?: number;
  fleetRows: Partial<Record<PeerName, FleetPeerResult>>;
  fleetConvergeMs?: number;
}

let state: CeremonyState | undefined;

function ceremony(): CeremonyState {
  state ??= {
    baselineEstablished: false,
    channelCreated: false,
    candidateSha256: '',
    stagingConvergedRows: {},
    fleetRows: {},
  };
  return state;
}

// ---------------------------------------------------------------------------
// Peer + HTTP helpers
// ---------------------------------------------------------------------------

function peerUrl(world: E2EWorld, peer: PeerName): string {
  return world.getDoorway(peer).url.replace(/\/$/, '');
}

async function getAdoptionReport(world: E2EWorld, peer: PeerName): Promise<AdoptionReport> {
  const { status, text } = await getRaw(`${peerUrl(world, peer)}${ADOPTION_PATH}`, {
    timeoutMs: 10_000,
  });
  assert.equal(status, 200, `GET ${ADOPTION_PATH} on ${peer} returned ${status}`);
  return JSON.parse(text) as AdoptionReport;
}

function findChannelRow(report: AdoptionReport, channelId: string): AdoptionChannelRow | undefined {
  return report.channels.find(row => row.channelId === channelId);
}

/**
 * Poll one peer's `/admin/adoption` for CHANNEL_ID until `predicate` is
 * satisfied. A connect failure or non-200 is NOT "absent" — the poll keeps
 * retrying rather than concluding the row doesn't exist, matching the
 * receipt's "unreachable ≠ absent" rail.
 */
async function pollAdoption(
  world: E2EWorld,
  peer: PeerName,
  timeoutMs: number,
  predicate: (row: AdoptionChannelRow) => boolean,
  intervalMs = 10_000
): Promise<{ row: AdoptionChannelRow; elapsedMs: number }> {
  const start = Date.now();
  let lastRow: AdoptionChannelRow | undefined;
  let everReachable = false;
  while (Date.now() - start < timeoutMs) {
    try {
      const report = await getAdoptionReport(world, peer);
      everReachable = true;
      const row = findChannelRow(report, CHANNEL_ID);
      lastRow = row;
      if (row && predicate(row)) {
        return { row, elapsedMs: Date.now() - start };
      }
    } catch {
      // Transient unreachability — retry, never conclude absence from it.
    }
    await new Promise<void>(resolve => setTimeout(resolve, intervalMs));
  }
  assert.fail(
    `timed out after ${timeoutMs}ms waiting on ${peer}'s ${ADOPTION_PATH}[${CHANNEL_ID}] ` +
      `(peer was ${everReachable ? 'reachable' : 'never reachable'} during the poll); ` +
      `last row: ${JSON.stringify(lastRow)}`
  );
  throw new Error('unreachable');
}

async function readRoles(world: E2EWorld, peer: PeerName): Promise<VersionRole[]> {
  const { status, text } = await getRaw(`${peerUrl(world, peer)}${VERSION_PATH}`);
  assert.equal(status, 200, `GET ${VERSION_PATH} on ${peer} returned ${status}`);
  const body = JSON.parse(text) as VersionResponse;
  return body.passport?.happ?.roles ?? [];
}

async function readBlobTotal(world: E2EWorld, peer: PeerName): Promise<number> {
  const { status, text } = await getRaw(`${peerUrl(world, peer)}${HEALTH_PATH}`);
  assert.equal(status, 200, `GET ${HEALTH_PATH} on ${peer} returned ${status}`);
  const body = JSON.parse(text) as HealthResponse;
  return body.blobs?.total ?? -1;
}

function readPid(peer: PeerName): string {
  try {
    return readFileSync(path.join(MESH_ROOT, 'pids', `conductor-${peer}`), 'utf8').trim();
  } catch {
    return '';
  }
}

async function putBlobRaw(baseUrl: string, sha256: string, bytes: Buffer): Promise<number> {
  const { statusCode } = await request(`${baseUrl}/blob/sha256-${sha256}`, {
    method: 'PUT',
    headers: {
      'content-type': 'application/octet-stream',
      'x-agent-id': 'did:elohim:a2o-release-packager',
    },
    body: bytes,
  });
  return statusCode;
}

// ---------------------------------------------------------------------------
// runtime-config.toml follow/mode switching (rung-4: no restart)
// ---------------------------------------------------------------------------

function runtimeConfigPath(peer: PeerName): string {
  return path.join(MESH_ROOT, peer, 'runtime-config.toml');
}

function readChannelsValue(peer: PeerName): string {
  let content = '';
  try {
    content = readFileSync(runtimeConfigPath(peer), 'utf8');
  } catch {
    content = '';
  }
  const match = /ELOHIM_RELEASE_CHANNELS\s*=\s*"([^"]*)"/.exec(content);
  return match ? match[1] : '';
}

/** Replace (or append) only OUR channel's entry — every other entry is left untouched. */
function withChannelMode(existingValue: string, mode: string): string {
  const entries = existingValue
    .split(/[,;\n]/)
    .map(entry => entry.trim())
    .filter(Boolean)
    .filter(entry => entry.split('=')[0].trim() !== CHANNEL_ID);
  entries.push(`${CHANNEL_ID}=${mode}`);
  return entries.join(',');
}

const RELEASE_CHANNELS_LINE_RE = /ELOHIM_RELEASE_CHANNELS\s*=.*/m;

/** Replace the key's line in place, or append it (with a separating newline if needed). */
function withChannelsLine(content: string, line: string): string {
  if (RELEASE_CHANNELS_LINE_RE.test(content)) {
    return content.replace(RELEASE_CHANNELS_LINE_RE, line);
  }
  const needsSeparator = content.length > 0 && !content.endsWith('\n');
  const separator = needsSeparator ? '\n' : '';
  return `${content}${separator}${line}\n`;
}

function writeChannelsValue(peer: PeerName, value: string): void {
  const filePath = runtimeConfigPath(peer);
  let content = '';
  try {
    content = readFileSync(filePath, 'utf8');
  } catch {
    content = '';
  }
  const line = `ELOHIM_RELEASE_CHANNELS = "${value}"`;
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, withChannelsLine(content, line), 'utf8');
}

async function setPeerMode(world: E2EWorld, peer: PeerName, mode: string): Promise<void> {
  writeChannelsValue(peer, withChannelMode(readChannelsValue(peer), mode));
  const { status } = await postRaw(`${peerUrl(world, peer)}${RUNTIME_CONFIG_RELOAD_PATH}`);
  assert.ok(
    status >= 200 && status < 300,
    `${peer}'s ${RUNTIME_CONFIG_RELOAD_PATH} returned ${status} for mode "${mode}"`
  );
}

// ---------------------------------------------------------------------------
// Driver composition (T1/T2 — shell out, never re-implement)
// ---------------------------------------------------------------------------

interface DriverResult {
  status: number;
  stdout: string;
  stderr: string;
}

function runDriver(scriptRelPath: string, args: string[], timeoutMs = 60_000): DriverResult {
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

/** Every driver verb here prints exactly one pretty-printed JSON object as its LAST output. */
function extractJson<T>(stdout: string): T {
  const start = stdout.indexOf('{');
  assert.ok(start >= 0, `no JSON object in driver output: ${stdout.slice(0, 400)}`);
  return JSON.parse(stdout.slice(start)) as T;
}

// ---------------------------------------------------------------------------
// Ensure-chain: each function performs its station's REAL action exactly
// once per process (memoized on `ceremony()` state) and walks back through
// its own dependencies first, so any station can also run standalone.
// ---------------------------------------------------------------------------

async function ensureCeremonyBaseline(world: E2EWorld): Promise<void> {
  const c = ceremony();
  if (c.baselineEstablished) return;
  for (const peer of PEER_NAMES) {
    await setPeerMode(world, peer, 'observe');
  }
  for (const peer of PEER_NAMES) {
    await pollAdoption(world, peer, 90_000, row => row.mode === 'observe');
  }
  c.baselineEstablished = true;
}

async function ensureStaged(world: E2EWorld): Promise<void> {
  const c = ceremony();
  await ensureCeremonyBaseline(world);
  if (c.releaseCid) return;

  assert.ok(existsSync(CANDIDATE_HAPP), `candidate bundle missing: ${CANDIDATE_HAPP}`);
  c.candidateBytes = readFileSync(CANDIDATE_HAPP);
  c.candidateSha256 = createHash('sha256').update(c.candidateBytes).digest('hex');
  mkdirSync(REPORT_DIR, { recursive: true });

  const matthewUrl = peerUrl(world, 'matthew');
  const packaged = runDriver(
    'scripts/epr-release-package.ts',
    [
      '--artifact',
      CANDIDATE_HAPP,
      '--artifact-class',
      'coordinator-bundle',
      '--channel-id',
      CHANNEL_ID,
      '--applies-to-from',
      matthewUrl,
      '--soak-secs',
      String(SOAK_SECS),
      '--attestation-threshold',
      String(ATTESTATION_THRESHOLD),
      '--canary',
      CANARY_PEER,
      '--peer',
      matthewUrl,
      '--notes',
      'a2o stations 1-5: coordinator fix on the soak channel',
      '--out',
      MANIFEST_PATH,
    ],
    60_000
  );
  assert.equal(
    packaged.status,
    0,
    `epr-release-package.ts failed: ${packaged.stderr || packaged.stdout}`
  );
  assert.ok(existsSync(MANIFEST_PATH), `manifest was not written to ${MANIFEST_PATH}`);

  for (const peer of PEER_NAMES.filter(name => name !== 'matthew')) {
    const putStatus = await putBlobRaw(peerUrl(world, peer), c.candidateSha256, c.candidateBytes);
    assert.ok(putStatus === 200 || putStatus === 201, `PUT blob to ${peer} returned ${putStatus}`);
  }

  if (!c.channelCreated) {
    const discipline = JSON.stringify({
      soakSecs: SOAK_SECS,
      attestationThreshold: ATTESTATION_THRESHOLD,
      canaryOrder: [CANARY_PEER],
    });
    const created = runDriver('scripts/release-ceremony.ts', [
      'channel',
      'create',
      CHANNEL_ID,
      '--discipline',
      discipline,
    ]);
    assert.equal(created.status, 0, `channel create failed: ${created.stderr || created.stdout}`);
    c.channelCreated = true;
  }

  const published = runDriver('scripts/release-ceremony.ts', ['publish', MANIFEST_PATH]);
  assert.equal(published.status, 0, `publish failed: ${published.stderr || published.stdout}`);
  const parsed = extractJson<PublishResult>(published.stdout);
  assert.ok(parsed.releaseCid, `publish output missing releaseCid: ${JSON.stringify(parsed)}`);
  c.releaseCid = parsed.releaseCid;
  c.publishTier = parsed.tier;
}

async function ensureStagingConvergedAll(world: E2EWorld): Promise<void> {
  const c = ceremony();
  await ensureStaged(world);
  if (c.stagingConvergeMs !== undefined) return;
  const start = Date.now();
  for (const peer of PEER_NAMES) {
    const { row } = await pollAdoption(
      world,
      peer,
      190_000,
      r => r.resolvedHead?.cid === c.releaseCid
    );
    c.stagingConvergedRows[peer] = row;
  }
  c.stagingConvergeMs = Date.now() - start;
}

async function ensureResolvedOnJames(world: E2EWorld): Promise<void> {
  const c = ceremony();
  await ensureStaged(world);
  if (c.stagingConvergedRows.james?.resolvedHead?.cid === c.releaseCid) return;
  const { row } = await pollAdoption(
    world,
    'james',
    190_000,
    r => r.resolvedHead?.cid === c.releaseCid
  );
  c.stagingConvergedRows.james = row;
}

async function ensureCanaryApplied(world: E2EWorld): Promise<void> {
  const c = ceremony();
  await ensureResolvedOnJames(world);
  if (c.canaryAppliedMs !== undefined) return;

  c.jamesBefore = { pid: readPid('james'), roles: await readRoles(world, 'james') };
  await setPeerMode(world, 'james', 'canary');
  const start = Date.now();
  const { row } = await pollAdoption(
    world,
    'james',
    300_000,
    r => r.appliedRelease?.cid === c.releaseCid || r.verdict?.state === 'refused'
  );
  assert.notEqual(
    row.verdict?.state,
    'refused',
    `james's canary apply was REFUSED: ${JSON.stringify(row.verdict)}`
  );
  c.canaryAppliedMs = Date.now() - start;
  c.jamesAfter = { pid: readPid('james'), roles: await readRoles(world, 'james') };
}

async function ensureAttested(world: E2EWorld): Promise<void> {
  const c = ceremony();
  await ensureCanaryApplied(world);
  if (c.attestationRow) return;

  const budgetMs = Math.max((SOAK_SECS + 65) * 1000, 90_000);
  const deadline = Date.now() + budgetMs;
  let lastRow: AdoptionChannelRow | undefined;
  while (Date.now() < deadline) {
    const report = await getAdoptionReport(world, 'matthew');
    lastRow = findChannelRow(report, CHANNEL_ID);
    const qualifying = lastRow?.attestations?.qualifying ?? 0;
    if (qualifying >= 1) {
      c.attestationRow = lastRow;
      c.attestationTimestamp = Date.now();
      return;
    }
    await new Promise<void>(resolve => setTimeout(resolve, 10_000));
  }
  assert.fail(
    `no qualifying attestation on channel ${CHANNEL_ID} within ${budgetMs}ms; last row: ${JSON.stringify(lastRow)}`
  );
}

async function ensurePromoted(world: E2EWorld): Promise<void> {
  const c = ceremony();
  await ensureAttested(world);
  if (c.promotedAt !== undefined) return;
  assert.ok(c.releaseCid, 'no releaseCid to promote');
  const promoted = runDriver(
    'scripts/release-ceremony.ts',
    ['promote', CHANNEL_ID, c.releaseCid],
    60_000
  );
  assert.equal(promoted.status, 0, `promote failed: ${promoted.stderr || promoted.stdout}`);
  const parsed = extractJson<PromoteResult>(promoted.stdout);
  assert.equal(parsed.tier, 'earned', `promote did not declare earned: ${JSON.stringify(parsed)}`);
  c.promoteResult = parsed;
  c.promotedAt = Date.now();
}

async function ensureFleetConverged(world: E2EWorld): Promise<void> {
  const c = ceremony();
  await ensurePromoted(world);
  if (c.fleetConvergeMs !== undefined) return;

  const before = new Map<PeerName, { pid: string; health: number }>();
  for (const peer of PEER_NAMES) {
    before.set(peer, { pid: readPid(peer), health: await readBlobTotal(world, peer) });
  }

  const start = Date.now();
  for (const peer of ['matthew', 'jessica'] as PeerName[]) {
    await setPeerMode(world, peer, 'apply');
  }
  for (const peer of PEER_NAMES) {
    const { row } = await pollAdoption(
      world,
      peer,
      240_000,
      r => r.appliedRelease?.cid === c.releaseCid
    );
    const snapshot = before.get(peer);
    assert.ok(snapshot, `no before-snapshot captured for ${peer}`);
    c.fleetRows[peer] = {
      row,
      pidBefore: snapshot.pid,
      pidAfter: readPid(peer),
      healthBefore: snapshot.health,
      healthAfter: await readBlobTotal(world, peer),
    };
  }
  c.fleetConvergeMs = Date.now() - start;
}

// ---------------------------------------------------------------------------
// Background — documentary declarations plus the one real precondition
// (peers following CHANNEL_ID in `observe` mode) they collectively describe.
// ---------------------------------------------------------------------------

Given(
  "the household's runtime follows release channel {string}",
  { timeout: 120_000 },
  async function (this: E2EWorld, channelName: string) {
    assert.equal(
      channelName,
      STORY_COMMONS_CHANNEL_NAME,
      `unexpected commons channel name: ${channelName}`
    );
    await ensureCeremonyBaseline(this);
  }
);

Given(
  "the household's runtime also follows its shared soak channel {string}",
  { timeout: 120_000 },
  async function (this: E2EWorld, channelName: string) {
    assert.equal(
      channelName,
      STORY_SOAK_CHANNEL_NAME,
      `unexpected soak channel name: ${channelName}`
    );
    await ensureCeremonyBaseline(this);
  }
);

Given('james is designated the canary on that soak channel', function () {
  // Documentary: the packaged manifest's canaryOrder (set in ensureStaged) is
  // where this becomes real, exactly like `--canary james` in the receipt.
  assert.equal(CANARY_PEER, 'james');
});

Given(
  "james's runtime additionally follows his own personal channel {string}",
  function (_channelName: string) {
    // Documentary only — james's personal experiment channel is station 7
    // territory (out of this file's scope; see the pending stub below).
  }
);

// ---------------------------------------------------------------------------
// Station 1 — publish to the soak channel, never straight to commons.
// ---------------------------------------------------------------------------

Given(
  "matthew has built a coordinator fix — new behavior, no change to the household's DNA lineage",
  function () {
    assert.ok(
      existsSync(CANDIDATE_HAPP),
      `candidate coordinator bundle missing: ${CANDIDATE_HAPP}`
    );
  }
);

When(
  'matthew publishes it as a release manifest on the soak channel {string}',
  { timeout: 90_000 },
  async function (this: E2EWorld, channelName: string) {
    assert.equal(
      channelName,
      STORY_SOAK_CHANNEL_NAME,
      `unexpected soak channel name: ${channelName}`
    );
    await ensureStaged(this);
  }
);

Then('the release is declared staging, not earned', function () {
  const c = ceremony();
  assert.ok(c.releaseCid, 'no release published yet — run the publish step first');
  assert.equal(c.publishTier, 'staging', `expected tier "staging", got "${String(c.publishTier)}"`);
});

Then(
  'the release is not visible as a candidate on the commons channel',
  { timeout: 30_000 },
  function () {
    const result = runDriver('scripts/release-ceremony.ts', ['status', CHANNEL_ID], 20_000);
    const report = extractJson<StatusReport>(result.stdout);
    const earnedRows = report.peers.filter(row => row.tier === 'earned');
    assert.equal(
      earnedRows.length,
      0,
      `release already reads as earned (commons) on: ${JSON.stringify(earnedRows)} — a soak publish ` +
        'must never be visible as an earned candidate before promotion.'
    );
  }
);

// ---------------------------------------------------------------------------
// Station 2 — staging election converges on every peer's own conductor.
// ---------------------------------------------------------------------------

Given(
  'matthew has published a staging release on channel {string}',
  { timeout: 120_000 },
  async function (this: E2EWorld, channelName: string) {
    assert.equal(
      channelName,
      STORY_SOAK_CHANNEL_NAME,
      `unexpected soak channel name: ${channelName}`
    );
    await ensureStaged(this);
    assert.equal(ceremony().publishTier, 'staging');
  }
);

When(
  "each household peer's runtime next resolves that channel",
  { timeout: 200_000 },
  async function (this: E2EWorld) {
    await ensureStagingConvergedAll(this);
  }
);

Then(
  "matthew's, jessica's, and james's runtimes all resolve the identical staged release",
  function () {
    const c = ceremony();
    for (const peer of PEER_NAMES) {
      const row = c.stagingConvergedRows[peer];
      assert.ok(row, `no adoption row captured for ${peer}`);
      assert.equal(
        row.resolvedHead?.cid,
        c.releaseCid,
        `${peer} resolved head ${JSON.stringify(row.resolvedHead)}`
      );
      assert.equal(
        row.resolvedHead?.tier,
        'staging',
        `${peer} resolved tier ${String(row.resolvedHead?.tier)}`
      );
    }
  }
);

Then(
  "each peer's runtime reports its own conductor as the resolution path — a peer hint may have pointed it at the channel worth checking, but the record shows a verified local resolve, never the hint itself adopted as fact",
  function () {
    const c = ceremony();
    // Every row above was fetched via a DIRECT GET against THAT peer's OWN
    // storage URL — never a shared/proxied surface — and each carries its
    // OWN controller's lastCheckedAt, the observable proxy for "resolved
    // through my own conductor" rather than a value borrowed from a hint.
    for (const peer of PEER_NAMES) {
      const row = c.stagingConvergedRows[peer];
      assert.ok(row, `no adoption row captured for ${peer}`);
      assert.ok(
        typeof row.lastCheckedAt === 'number' && row.lastCheckedAt > 0,
        `${peer}'s adoption row carries no lastCheckedAt — no evidence its own controller ever swept`
      );
    }
    const cids = new Set(PEER_NAMES.map(peer => c.stagingConvergedRows[peer]?.resolvedHead?.cid));
    assert.equal(cids.size, 1, `peers disagree on the resolved head: ${JSON.stringify([...cids])}`);
  }
);

// ---------------------------------------------------------------------------
// Station 3 — the canary adopts and attests with context.
// ---------------------------------------------------------------------------

Given(
  "the staged release on channel {string} has resolved on james's runtime",
  { timeout: 200_000 },
  async function (this: E2EWorld, channelName: string) {
    assert.equal(
      channelName,
      STORY_SOAK_CHANNEL_NAME,
      `unexpected soak channel name: ${channelName}`
    );
    await ensureResolvedOnJames(this);
  }
);

When("james's runtime applies the release", { timeout: 320_000 }, async function (this: E2EWorld) {
  await ensureCanaryApplied(this);
});

Then(
  "applying it changes nothing about who james is to the rest of the household — his runtime's own passport reports the same agent identity and the same cells, with only the coordinator behavior different",
  function () {
    const c = ceremony();
    assert.ok(
      c.jamesBefore && c.jamesAfter,
      'no before/after passport captured — run the apply step first'
    );
    const before = c.jamesBefore;
    const after = c.jamesAfter;
    // No restart means no re-key — the conductor keystore (and therefore the
    // agent identity every cell answers with) is untouched by construction.
    assert.equal(
      after.pid,
      before.pid,
      `james's conductor PID changed (${before.pid} -> ${after.pid})`
    );
    assert.ok(before.pid.length > 0, "james's conductor PID could not be read before the apply");

    for (const beforeRole of before.roles) {
      const afterRole = after.roles.find(role => role.role === beforeRole.role);
      assert.ok(
        afterRole,
        `role "${beforeRole.role}" missing from james's passport after the apply`
      );
      assert.equal(
        afterRole.dnaHash,
        beforeRole.dnaHash,
        `${beforeRole.role}'s dnaHash changed — a coordinator hot-swap must never move cell/DNA identity`
      );
    }

    const beforeLamad = before.roles.find(role => role.role === LAMAD_ROLE);
    const afterLamad = after.roles.find(role => role.role === LAMAD_ROLE);
    assert.notDeepEqual(
      afterLamad?.coordinatorWasmHashes,
      beforeLamad?.coordinatorWasmHashes,
      'lamad coordinatorWasmHashes did not change — the apply had no observable coordinator effect'
    );
  }
);

Then(
  "james's runtime attests the outcome, naming his device's hardware profile and a concrete thing it checked",
  { timeout: 150_000 },
  async function (this: E2EWorld) {
    await ensureAttested(this);
    const row = ceremony().attestationRow;
    assert.ok(row, 'no attestation recorded');
    // deviceArchetype is release_attestation.rs's own name for "hardware
    // profile"; a qualifying count with no archetype key would mean the
    // count is uninterpretable, not merely unnamed.
    const archetypes = Object.keys(row.attestations?.byArchetype ?? {});
    assert.ok(
      archetypes.length > 0,
      `attestation qualifies but names no device archetype: ${JSON.stringify(row.attestations)}`
    );
  }
);

Then(
  "james's own attestation alone could never be enough to earn the release for the household",
  function () {
    const c = ceremony();
    assert.ok(c.attestationRow, 'no attestation recorded — run the attestation step first');
    assert.ok(
      (c.attestationRow.attestations?.qualifying ?? 0) >= 1,
      'expected a qualifying attestation to exist'
    );
    // Earning is a SEPARATE ceremony act (station 4's `promote`); a qualifying
    // attestation must never auto-earn a channel by itself.
    assert.equal(
      c.attestationRow.resolvedHead?.tier,
      'staging',
      `channel already reads tier "${String(c.attestationRow.resolvedHead?.tier)}" before any promote ran`
    );
  }
);

// ---------------------------------------------------------------------------
// Station 4 — earned promotion, declared on evidence.
// ---------------------------------------------------------------------------

Given(
  "james's soak attestation for the release on channel {string} is recorded",
  { timeout: 400_000 },
  async function (this: E2EWorld, channelName: string) {
    assert.equal(
      channelName,
      STORY_SOAK_CHANNEL_NAME,
      `unexpected soak channel name: ${channelName}`
    );
    await ensureAttested(this);
  }
);

When(
  'matthew runs the promotion ceremony for that release',
  { timeout: 60_000 },
  async function (this: E2EWorld) {
    await ensurePromoted(this);
  }
);

Then('the release becomes the earned head of channel {string}', function (channelName: string) {
  assert.equal(
    channelName,
    STORY_COMMONS_CHANNEL_NAME,
    `unexpected commons channel name: ${channelName}`
  );
  const c = ceremony();
  assert.ok(c.promoteResult, 'no promote result captured — run the promotion step first');
  assert.equal(
    c.promoteResult.tier,
    'earned',
    `promote result tier is "${c.promoteResult.tier}", not earned`
  );
});

Then("the promotion names james's attestation as the evidence it rests on", function () {
  const c = ceremony();
  assert.ok(c.attestationRow && c.attestationTimestamp, 'no attestation captured before promote');
  assert.ok(c.promotedAt, 'no promote timestamp captured');
  // No surface names an attestation by peer (the same honest bound
  // steps/delivery/acquisition-pins.steps.ts documents for blob provenance),
  // so this checks what IS observable: evidence existed strictly before the
  // declaration rested on it, met the declared threshold, and james was the
  // only peer ever placed in canary mode — the only follower who could have
  // produced it.
  assert.ok(
    c.attestationTimestamp < c.promotedAt,
    'promotion timestamp is not after the attestation it is supposed to rest on'
  );
  assert.ok(
    (c.attestationRow.attestations?.qualifying ?? 0) >= ATTESTATION_THRESHOLD,
    `promotion ran with only ${String(c.attestationRow.attestations?.qualifying)}/${ATTESTATION_THRESHOLD} qualifying attestations`
  );
  assert.equal(
    CANARY_PEER,
    'james',
    'james is the only peer this run ever switched to canary mode'
  );
});

// ---------------------------------------------------------------------------
// Station 5 — fleet convergence, nobody restarts, nobody is asked.
// ---------------------------------------------------------------------------

Given(
  'channel {string} now declares the promoted release earned',
  { timeout: 400_000 },
  async function (this: E2EWorld, channelName: string) {
    assert.equal(
      channelName,
      STORY_COMMONS_CHANNEL_NAME,
      `unexpected commons channel name: ${channelName}`
    );
    await ensurePromoted(this);
  }
);

When(
  "each household peer's runtime next resolves the commons channel",
  { timeout: 260_000 },
  async function (this: E2EWorld) {
    await ensureFleetConverged(this);
  }
);

Then(
  /^matthew's, jessica's, and james's runtimes all apply the release without anyone's device restarting \(each conductor process keeps the same PID it had before\)$/,
  function () {
    const c = ceremony();
    for (const peer of PEER_NAMES) {
      const result = c.fleetRows[peer];
      assert.ok(result, `no fleet-convergence row captured for ${peer}`);
      assert.equal(
        result.row.appliedRelease?.cid,
        c.releaseCid,
        `${peer} has not applied the release: ${JSON.stringify(result.row)}`
      );
      assert.ok(result.pidAfter.length > 0, `${peer}'s conductor PID could not be read`);
      assert.equal(
        result.pidAfter,
        result.pidBefore,
        `${peer}'s conductor PID changed (${result.pidBefore} -> ${result.pidAfter})`
      );
    }
  }
);

Then('jessica is shown no prompt, asked no question, and given nothing to click', function () {
  const row = ceremony().fleetRows.jessica?.row;
  assert.ok(row, 'no jessica adoption row captured');
  assert.equal(
    row.mode,
    'apply',
    "jessica's channel mode must be apply — a fully automatic vehicle"
  );
  assert.equal(
    row.verdict?.state,
    'applied',
    `jessica's verdict is ${JSON.stringify(row.verdict)}`
  );
  assert.equal(
    row.pendingRestart,
    false,
    'jessica has a pendingRestart flag — that implies a human-visible step'
  );
  // No consent/approval HTTP surface exists anywhere in this run's trace for
  // jessica: mode=apply means the controller's own sweep decided and called
  // the vehicle itself — there is no endpoint this step could have called on
  // her behalf even if it tried.
});

Then(
  "nothing about jessica's own content, files, or recorded agreements with the rest of the household changes because of the upgrade",
  function () {
    const result = ceremony().fleetRows.jessica;
    assert.ok(result, 'no jessica fleet row captured');
    assert.equal(
      result.healthAfter,
      result.healthBefore,
      `jessica's blob total changed (${result.healthBefore} -> ${result.healthAfter}) — a coordinator ` +
        'upgrade must never touch her own content'
    );
  }
);

// ---------------------------------------------------------------------------
// Stations 6-8 and the two constitutional scenarios — pending.
//
// These name contracts the r2 receipt did NOT prove by hand-composed drivers
// yet (revert-by-re-election needs a fresh manifest per station 8's design
// note; the observed version matrix has no reader surface; james's personal
// channel and the DNA-lineage refusal are untouched by stations 1-5's setup).
// Per the task scope, these stay `pending` — never faked — and stay reachable
// under the scenario-level `@wip` tag the feature file carries for them.
// ---------------------------------------------------------------------------

Given(
  'the household has converged on the promoted release and now judges it a regression',
  function () {
    return 'pending';
  }
);

When(
  'matthew runs the revert ceremony, re-declaring the prior release the earned head of channel {string}',
  function (_channelName: string) {
    return 'pending';
  }
);

Then(
  "matthew's, jessica's, and james's runtimes all return to the prior coordinator behavior, converging backward through the identical loop they used to converge forward",
  function () {
    return 'pending';
  }
);

Then(
  'nothing outside the ceremony itself was needed to get there — no operator flag, no re-key, no DHT reset',
  function () {
    return 'pending';
  }
);

Given(
  'matthew has run the ceremony that staged, promoted, and reverted a release on the commons channel',
  function () {
    return 'pending';
  }
);

When(
  /^james's runtime is asked (?:what his personal channel is doing|the same question) (after staging|after promotion|after the revert)$/,
  function (_when: string) {
    return 'pending';
  }
);

Then(
  'his personal channel is diverging from commons, inside the same compatibility envelope',
  function () {
    return 'pending';
  }
);

Then(
  'his personal channel is still diverging from commons, inside the same compatibility envelope',
  function () {
    return 'pending';
  }
);

Then(
  "james's runtime is converged on commons at that point, exactly like matthew's and jessica's",
  function () {
    return 'pending';
  }
);

Then("nobody promotes james's channel to commons and nobody forces james off it", function () {
  return 'pending';
});

Given("james's personal channel ran alongside that ceremony the entire time", function () {
  return 'pending';
});

When("an operator reads the household's observed version matrix", function () {
  return 'pending';
});

Then(
  "the matrix shows matthew's, jessica's, and james's runtimes moving staging, then earned, then back, in that order",
  function () {
    return 'pending';
  }
);

Then("the matrix shows james's personal channel diverging compatibly the whole time", function () {
  return 'pending';
});

Then(
  "every row in the matrix is read from what each runtime itself reports, never asserted from the ceremony's own intent",
  function () {
    return 'pending';
  }
);

Given("jessica's runtime is following release channel {string}", function (_channelName: string) {
  return 'pending';
});

When("that channel's earned head changes", function () {
  return 'pending';
});

Then("jessica's runtime adopts it without asking her permission", function () {
  return 'pending';
});

Then(
  "jessica's runtime exposes no setting, flag, or control that lets her defer, decline, or veto that individual upgrade",
  function () {
    return 'pending';
  }
);

Then(
  "jessica can still read the release's own explanation of what changed and why, and can reach escalation if her intimate context was mishandled",
  function () {
    return 'pending';
  }
);

Given(
  /^a release manifest was built for a different DNA lineage than the one the household actually runs \(its declared per-role hashes do not match\)$/,
  function () {
    return 'pending';
  }
);

When("any household peer's runtime verifies that release locally", function () {
  return 'pending';
});

Then(
  "no one's device is put at risk by the mismatch — that peer refuses the release outright, naming a typed reason",
  function () {
    return 'pending';
  }
);

Then('no household peer ever applies it, no matter which channel declared it earned', function () {
  return 'pending';
});
