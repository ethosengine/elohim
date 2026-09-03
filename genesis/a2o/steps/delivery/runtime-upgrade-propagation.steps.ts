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
 * ## One channel, two tiers — plus a genuine second channel
 *
 * The feature's Background narrates what this file actually mints:
 * `runtime:coordinators:elohim:commons` (`CHANNEL_ID` below, freshly minted
 * per run) is ONE technical channel whose single head carries two tiers —
 * `staging` while it is a resolving-but-unproven candidate, `earned` once
 * the promotion ceremony declares it so. The ceremony driver has no
 * "promote from channel A onto channel B" verb: a release manifest carries
 * ONE `channelId`, and `publish`/`promote` move that ONE channel's tier from
 * `staging` to `earned` in place (release-ceremony.ts's own module doc: "stage
 * → promote → revert-by-re-election" on a single channel). The household's
 * SOAK is that head's staging tier, not a separate channel to reconcile
 * against commons: every peer resolves the staged candidate through its own
 * conductor, but only the canary (james) is expected to actually apply and
 * attest it before the household trusts it enough to promote. "Not visible
 * as the earned head" (station 1) reads as "no peer reports this channel's
 * tier as `earned` yet"; "the release becomes the earned head of channel
 * commons" (station 4) reads as "this channel's tier becomes `earned`".
 *
 * `runtime:coordinators:elohim:canary-james` (`PERSONAL_CHANNEL_ID` below,
 * likewise minted per run with a `-personal` suffix, per the same collision
 * rationale as `CHANNEL_ID`) IS a real second channel — james's own personal
 * experiment channel, a separate content identity that carries a compatible
 * variant release (station 7) and is never expected to converge with commons
 * at all; nobody promotes it and nobody forces james off it.
 *
 * ## Fresh channel per run — and the run OWNS the follow set
 *
 * `CHANNEL_ID` (and `PERSONAL_CHANNEL_ID`) are minted once at module load
 * from a run stamp (`A2O_RELEASE_RUN_STAMP` env override, else the process
 * start time) so a repeat run never collides with a channel a prior run left
 * mid-backoff-ladder.
 *
 * A 2026-09-01 05:0xZ run measured that fresh-channel-ids alone are not
 * enough: Station 6 (revert) refused `coordinator_lineage_mismatch` because
 * a LEFTOVER channel line from an EARLIER ceremony was still on matthew's
 * disk, competing with this run's own channel in the controller's follow
 * set — a hazard the previous read-merge-write helper (`withChannelMode`)
 * could not close, because it deliberately preserved every pre-existing
 * entry. The fix: this run keeps its OWN in-memory channel->mode map per
 * peer (`runOwnedChannels`, in the "runtime-config.toml follow/mode
 * switching" section below) and every write derives the file's entire
 * content from that map alone — never a merge with whatever an earlier
 * ceremony left on disk. The true pre-run bytes are captured once and
 * restored byte-for-byte in `AfterAll`, so a leftover from THIS run can
 * never become the next run's hazard either.
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

import { Given, When, Then, Before, AfterAll } from '@cucumber/cucumber';

import { request } from 'undici';

import { getRaw, postRaw } from '../../src/framework/dataplane/surfaces.js';

import { mintCoordinatorCandidate } from './coordinator-candidate.js';

import type { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Paths and constants
// ---------------------------------------------------------------------------

/** genesis/a2o/steps/delivery -> genesis/a2o (2 levels up). */
const A2O_ROOT = fileURLToPath(new URL('../../', import.meta.url));

/** genesis/a2o/steps/delivery -> repo root (4 levels up). */
const REPO_ROOT = fileURLToPath(new URL('../../../../', import.meta.url));

/**
 * Station 6's revert target: the DNA workdir bundle — pre-fix bytes distinct
 * from this run's freshly-minted candidate (see `candidateHapp` below),
 * already on disk, never written by this ceremony (coordinator hot-swap
 * patches a running conductor; it never touches this file). Same role the
 * r2 receipt's "N" bundle played (`elohim/holochain/local-dev/…` was that
 * receipt's "O" — a THIRD distinct bundle used there for an unrelated
 * already-installed check; this file only needs ONE bundle that differs
 * from the fix). It also doubles as `candidateHapp`'s own source of
 * currently-installed integrity+coordinator bytes — see
 * `coordinator-candidate.ts`'s module doc for why that beats reading the
 * cargo target dir directly.
 */
// The "installed baseline" is the bundle the mesh conductors were INSTALLED from, which is
// not necessarily the workdir's current pack: `just pack` in dna/elohim rewrites
// workdir/elohim.happ whenever a zome changes (2026-09-02, mid-ceremony). Resolution order:
// an explicit pin, a preserved copy of the installed bytes, then the workdir as last resort.
const BASELINE_HAPP = (() => {
  const pinned = process.env['E2E_BASELINE_HAPP'];
  if (pinned) return path.resolve(pinned);
  const preserved = path.join(
    REPO_ROOT,
    'genesis/a2o/reports/release-ceremony/2026-09-03/baseline-N/elohim.happ'
  );
  if (existsSync(preserved)) return preserved;
  return path.join(REPO_ROOT, 'elohim/holochain/dna/elohim/workdir/elohim.happ');
})();

const RUN_STAMP =
  process.env['A2O_RELEASE_RUN_STAMP'] ?? new Date().toISOString().replace(/\D/g, '').slice(0, 14);
const CHANNEL_ID = `runtime:coordinators:elohim:a2o-${RUN_STAMP}`;
/** James's personal experiment channel — a genuine SECOND channel (station 7). */
const PERSONAL_CHANNEL_ID = `runtime:coordinators:elohim:a2o-${RUN_STAMP}-personal`;

const REPORT_DIR = path.join(
  A2O_ROOT,
  'reports',
  'release-ceremony',
  new Date().toISOString().slice(0, 10)
);
const MANIFEST_PATH = path.join(REPORT_DIR, `a2o-runtime-upgrade-${RUN_STAMP}.json`);
const REVERT_MANIFEST_PATH = path.join(REPORT_DIR, `a2o-runtime-upgrade-${RUN_STAMP}-revert.json`);
const PERSONAL_MANIFEST_PATH = path.join(
  REPORT_DIR,
  `a2o-runtime-upgrade-${RUN_STAMP}-personal.json`
);

/**
 * This run's freshly-minted coordinator-only candidate — replaces the
 * PREVIOUS fixed-bundle fixture (`reports/release-ceremony/2026-09-01/elohim-P.happ`),
 * which was whatever the household had already converged on by the time a
 * later run started, making Station 3's apply `already_current` (no
 * observable coordinator effect) and cascading `threshold_unmet` refusals
 * through Stations 6-8. Minted once per process and reused by every station
 * (commons channel AND james's personal channel both publish this same
 * candidate — see `coordinator-candidate.ts`).
 */
function candidateHapp(): string {
  const minted = mintCoordinatorCandidate(BASELINE_HAPP, REPORT_DIR, RUN_STAMP, CHANNEL_ID);
  // Recorded once, for station 3/5/6's ground-truth comparisons below — see
  // `candidateWasmSha256` on `CeremonyState`.
  ceremony().candidateWasmSha256 ??= minted.coordinatorWasmSha256;
  return minted.happPath;
}

const SOAK_SECS = 30;
const ATTESTATION_THRESHOLD = 1;

/**
 * 2026-09-02 diagnosis of station 6's `coordinator_lineage_mismatch`: the
 * elohim-storage release-adoption controller caches "installed reality"
 * (its own snapshot of what a peer's runtime would report) for
 * `INSTALLED_REALITY_TTL_SECS` seconds with NO apply-triggered invalidation
 * (`elohim/elohim-storage/src/services/release_adoption/watch.rs`,
 * `installed_reality()` — reads through only when
 * `now - read_at >= INSTALLED_REALITY_TTL_SECS`). A head verified against
 * this peer within that window of the cache's last refresh is checked
 * against WHATEVER installed reality happened to be cached then — which can
 * predate the fleet's own forward apply. Live evidence: the r4 run's
 * failure declared the release "supersedes coordinator wasm [[candidate]],
 * which this peer does not run (running: [pre-apply hash])" under a minute
 * after this same peer's `appliedRelease.cid` had already flipped to that
 * candidate — the peer's own live `/version` already reported the
 * candidate; only the controller's cached view, reused for the revert's
 * verify check, was stale. This file cannot change the controller (Rust,
 * out of scope here), so `ensureReverted` waits out the TTL (plus a buffer)
 * before packaging the revert, forcing a fresh read on account of age alone.
 */
const INSTALLED_REALITY_TTL_SECS = 300;
const INSTALLED_REALITY_TTL_BUFFER_SECS = 15;

type PeerName = 'matthew' | 'jessica' | 'james';
const PEER_NAMES: readonly PeerName[] = ['matthew', 'jessica', 'james'];
const CANARY_PEER: PeerName = 'james';

const STORY_COMMONS_CHANNEL_NAME = 'runtime:coordinators:elohim:commons';
const STORY_PERSONAL_CHANNEL_NAME = 'runtime:coordinators:elohim:canary-james';

/** T2 ceremony driver, relative to `A2O_ROOT` (`runDriver`'s `cwd`). */
const RELEASE_CEREMONY_SCRIPT = 'scripts/release-ceremony.ts';

/** T1 packaging driver, relative to `A2O_ROOT` — every packaging call site in this file
 * (station 1, station 6's revert target, station 7's Background publish AND its two
 * `republishPersonalVariant` rebases, plus the pre/post-run teardown packager) shares this
 * script and the same `epr-release-package.ts` CLI flag literals below, hence these constants:
 * five call sites made the raw string literals cross this file's `sonarjs/no-duplicate-string`
 * (threshold 5) gate. */
const EPR_RELEASE_PACKAGE_SCRIPT = 'scripts/epr-release-package.ts';
const FLAG_ARTIFACT = '--artifact';
const FLAG_ARTIFACT_CLASS = '--artifact-class';
const ARTIFACT_CLASS_COORDINATOR_BUNDLE = 'coordinator-bundle';
const FLAG_CHANNEL_ID = '--channel-id';
const FLAG_APPLIES_TO_FROM = '--applies-to-from';
const FLAG_SOAK_SECS = '--soak-secs';
const FLAG_ATTESTATION_THRESHOLD = '--attestation-threshold';

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

/**
 * `release-ceremony.ts revert <channel> <manifest.json>` prints TWO
 * pretty-printed JSON objects (the freshly-authored version, then the
 * declare-earned result) — this is the shape of the LAST one, read via
 * `extractLastJson`.
 */
interface RevertResult {
  tier: string;
  releaseCid: string;
  canonical?: boolean;
  secondPeerVerification?: unknown;
}

/** One cell of the station 8 observed matrix — always a RECORDED read, never a fresh one. */
interface MatrixRow {
  peer: PeerName;
  station: 'staging' | 'earned' | 'reverted' | 'personal';
  releaseCid: string | undefined;
  tier: string | undefined;
  readAt: number | null;
  route: string;
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

type PersonalStation = 'staging' | 'promotion' | 'revert';

interface JamesChannelRows {
  commons?: AdoptionChannelRow;
  personal?: AdoptionChannelRow;
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
  /** Coordinator wasm sha256 (post-marker) from the mint itself — see
   * `coordinator-candidate.ts`'s `MintedCandidate.coordinatorWasmSha256`. */
  candidateWasmSha256?: string;
  /**
   * The candidate's REAL installed lamad coordinatorWasmHashes, read back
   * from james's `/version` passport right after station 3's canary apply —
   * the ground truth every later station's passport reads are compared
   * against (never `appliedRelease.cid` alone, which names the RELEASE
   * object, not the coordinator wasm the conductor actually swapped in).
   */
  candidateWasmHashes?: Record<string, string>;
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
  /** Snapshot of `setPeerModeCallCount` right after station 5's own mode
   * switches — station 6's "nothing outside the ceremony" Then compares
   * against this to prove revert touched no runtime-config lever at all. */
  setPeerModeCallsAfterPromotion?: number;

  // Station 6 — revert by re-election.
  preRevertRows: Partial<Record<PeerName, AdoptionChannelRow>>;
  priorManifestPath?: string;
  priorReleaseCid?: string;
  revertResult?: RevertResult;
  revertedAt?: number;
  postRevertRows: Partial<Record<PeerName, FleetPeerResult>>;
  revertConvergeMs?: number;

  // Station 7 — james's personal channel.
  personalChannelCreated: boolean;
  personalChannelFollowed: boolean;
  personalReleaseCid?: string;
  /**
   * The personal channel's expected cid PER STATION — a personal channel
   * rebases when commons moves: its appliesTo is what it supersedes, and a
   * node runs one coordinator per role. The Background's own publish (bound
   * to matthew's pre-ceremony baseline reality) covers "staging"; the
   * "promotion" and "revert" entries are set by `republishPersonalVariant`
   * once james's live reality has moved past that envelope. Compared against
   * in `assertPersonalDiverging`, falling back to `personalReleaseCid` (the
   * latest) when a given station was never republished.
   */
  personalReleaseCidByStation: Partial<Record<PersonalStation, string>>;
  personalRowsByStation: Partial<Record<PersonalStation, JamesChannelRows>>;
  lastPersonalStation?: PersonalStation;

  // Station 8 — the observed matrix, assembled only from recorded reads.
  observedMatrix?: MatrixRow[];
}

let state: CeremonyState | undefined;

function ceremony(): CeremonyState {
  state ??= {
    baselineEstablished: false,
    channelCreated: false,
    candidateSha256: '',
    stagingConvergedRows: {},
    fleetRows: {},
    preRevertRows: {},
    postRevertRows: {},
    personalChannelCreated: false,
    personalChannelFollowed: false,
    personalReleaseCidByStation: {},
    personalRowsByStation: {},
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
  channelId: string = CHANNEL_ID,
  intervalMs = 10_000
): Promise<{ row: AdoptionChannelRow; elapsedMs: number }> {
  const start = Date.now();
  let lastRow: AdoptionChannelRow | undefined;
  let everReachable = false;
  while (Date.now() - start < timeoutMs) {
    try {
      const report = await getAdoptionReport(world, peer);
      everReachable = true;
      const row = findChannelRow(report, channelId);
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
    `timed out after ${timeoutMs}ms waiting on ${peer}'s ${ADOPTION_PATH}[${channelId}] ` +
      `(peer was ${everReachable ? 'reachable' : 'never reachable'} during the poll); ` +
      `last row: ${JSON.stringify(lastRow)}`
  );
  throw new Error('unreachable');
}

/**
 * Reads one peer's `/version` passport roles from an already-resolved URL —
 * the primitive `readRoles` wraps for `World`-based callers, and the one the
 * pre/post-run convergence-to-baseline routines use directly (no `World` is
 * available in `AfterAll`; see `directPeerUrl`).
 */
async function readRolesAt(url: string, peerLabel: string): Promise<VersionRole[]> {
  const { status, text } = await getRaw(`${url}${VERSION_PATH}`);
  assert.equal(status, 200, `GET ${VERSION_PATH} on ${peerLabel} returned ${status}`);
  const body = JSON.parse(text) as VersionResponse;
  return body.passport?.happ?.roles ?? [];
}

async function readRoles(world: E2EWorld, peer: PeerName): Promise<VersionRole[]> {
  return readRolesAt(peerUrl(world, peer), peer);
}

function lamadRole(roles: VersionRole[]): VersionRole | undefined {
  return roles.find(role => role.role === LAMAD_ROLE);
}

/**
 * Sorted, deduped hash VALUES out of a `coordinatorWasmHashes` record — the
 * same normalization `epr-release-package.ts`'s `roleBindingFrom` applies
 * before writing a manifest's `appliesTo.roles.<role>.coordinatorWasmHashes`
 * array, so a passport record and a manifest array compare apples-to-apples
 * regardless of which zome names are present on each side.
 */
function wasmHashValues(record: Record<string, string> | undefined): string[] {
  return [...new Set(Object.values(record ?? {}))].sort((a, b) => a.localeCompare(b));
}

/**
 * Poll one peer's `GET /version` passport until its lamad
 * `coordinatorWasmHashes` match `expected` (compared as sorted, deduped
 * VALUES via `wasmHashValues` — see that helper's own doc).
 *
 * 2026-09-02 diagnosis: `/admin/adoption`'s `appliedRelease.cid` can flip to
 * the new release's cid BEFORE the conductor's own coordinator hot-swap has
 * actually landed — an async propagation lag between "the controller decided
 * to apply" and "the wasm is really running." A single snapshot read taken
 * right after an `appliedRelease.cid` match can race that lag and observe
 * the OLD hash, which is exactly what produced station 6's confusing
 * `coordinator_lineage_mismatch` on matthew: station 5 declared convergence
 * off `appliedRelease.cid` alone, station 6 packaged (and matthew's own live
 * read still showed the candidate) but the conductor's REAL installed wasm
 * had not caught up yet by the time the revert's own verify-time check ran.
 * Polling — the same "unreachable ≠ absent, keep retrying" discipline as
 * `pollAdoption` — gives the real swap the time it needs while still failing
 * loudly (with the last observed value) if it never lands.
 */
async function pollLamadWasmHashMatchAt(
  url: string,
  peerLabel: string,
  expected: Record<string, string> | undefined,
  timeoutMs: number,
  intervalMs = 5_000
): Promise<{ observed: Record<string, string> | undefined; elapsedMs: number }> {
  const start = Date.now();
  const expectedValues = wasmHashValues(expected);
  let lastObserved: Record<string, string> | undefined;
  while (Date.now() - start < timeoutMs) {
    try {
      const roles = await readRolesAt(url, peerLabel);
      lastObserved = lamadRole(roles)?.coordinatorWasmHashes;
      if (JSON.stringify(wasmHashValues(lastObserved)) === JSON.stringify(expectedValues)) {
        return { observed: lastObserved, elapsedMs: Date.now() - start };
      }
    } catch {
      // Transient unreachability — retry, never conclude mismatch from it.
    }
    await new Promise<void>(resolve => setTimeout(resolve, intervalMs));
  }
  assert.fail(
    `${peerLabel}'s /version passport never converged on the expected lamad coordinatorWasmHashes within ` +
      `${timeoutMs}ms (expected ${JSON.stringify(expected)}, last observed ${JSON.stringify(lastObserved)})`
  );
  throw new Error('unreachable');
}

async function pollLamadWasmHashMatch(
  world: E2EWorld,
  peer: PeerName,
  expected: Record<string, string> | undefined,
  timeoutMs: number,
  intervalMs = 5_000
): Promise<{ observed: Record<string, string> | undefined; elapsedMs: number }> {
  return pollLamadWasmHashMatchAt(peerUrl(world, peer), peer, expected, timeoutMs, intervalMs);
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
//
// ## The run OWNS the follow set — measured hazard, 2026-09-01 05:0xZ run
//
// Stations 1-5 passed with the per-run candidate; Station 6 (revert) refused
// `coordinator_lineage_mismatch` because a LEFTOVER channel line from an
// EARLIER ceremony (`receipt-20260901-r2=apply`) was still present in
// matthew's on-disk `runtime-config.toml`, competing with this run's own
// channel in the controller's follow set. Live inspection of the household
// mesh on 2026-09-02 confirmed the shape of the hazard beyond that one line:
// every peer's file had accreted FOUR to SIX comma-joined entries from
// earlier a2o runs and manual ceremonies, all still `=observe` today only
// because an operator hand-flipped them after the fact — hygiene this run
// must never depend on.
//
// The fix: for the run's own channel(s), this file never reads the
// pre-existing `ELOHIM_RELEASE_CHANNELS` value and merges into it. It keeps
// an in-memory map of channel -> mode PER PEER (`runOwnedChannels`) and
// every write derives the file's entire content from that map alone —
// `applyRunOwnedChannels` is the ONE helper every follow/mode-switch in this
// file goes through; nothing else ever touches the file. The true
// pre-existing bytes are captured once (`ensureRunOwnedFollowSet`, driven by
// the `Before` hook below) and restored byte-for-byte in the `AfterAll`
// below — a leftover from THIS run can never become the next run's hazard.
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

/**
 * This run's OWN follow set, per peer — the only source of truth for what
 * gets written to disk. Never populated from a disk read; only ever
 * mutated by this run's own stations via `applyRunOwnedChannels`.
 */
const runOwnedChannels: Record<PeerName, Map<string, string>> = {
  matthew: new Map<string, string>(),
  jessica: new Map<string, string>(),
  james: new Map<string, string>(),
};

/** Renders `runOwnedChannels[peer]` as the `channelId=mode,channelId=mode` value the conductor parses. */
function ownedChannelsValue(peer: PeerName): string {
  return [...runOwnedChannels[peer].entries()].map(([id, mode]) => `${id}=${mode}`).join(',');
}

/**
 * Writes the run-owned value as the file's ENTIRE content — never a
 * read-merge-write against whatever else is on disk. Every peer's real
 * runtime-config.toml carries only this one key (verified live on the
 * household mesh 2026-09-02), so this is the byte-for-byte "a file that
 * contains ONLY this run's channel lines" the fix calls for, not a
 * simplification that happens to work today.
 */
function writeOwnedChannelsFile(peer: PeerName): void {
  const filePath = runtimeConfigPath(peer);
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, `ELOHIM_RELEASE_CHANNELS = "${ownedChannelsValue(peer)}"\n`, 'utf8');
}

/**
 * Counts every mode-switch call this process makes. Station 6's "nothing
 * outside the ceremony itself was needed" Then reads this to prove revert
 * converged through the controller sweep alone — no operator runtime-config
 * lever pulled after promotion.
 */
let setPeerModeCallCount = 0;

/**
 * THE one helper every follow/mode-switch in this file goes through.
 * Rewrites the WHOLE `ELOHIM_RELEASE_CHANNELS` value from `runOwnedChannels`
 * — never appends to, or preserves, whatever a prior ceremony left on disk.
 *
 * Resolves the peer's URL directly from `E2E_STORAGE_<PEER>` (`directPeerUrl`,
 * defined below) rather than through `world.getDoorway`/`peerUrl` — this is
 * called from the `Before` hook (see `ensureRunOwnedFollowSet`), which runs
 * BEFORE the Background's own `Given peer "<peer>" at "E2E_STORAGE_<PEER>"`
 * step has registered any doorway on `world`, so `peerUrl(world, peer)`
 * would throw `Unknown doorway` at exactly that call.
 */
async function applyRunOwnedChannels(peer: PeerName): Promise<void> {
  writeOwnedChannelsFile(peer);
  const { status } = await postRaw(`${directPeerUrl(peer)}${RUNTIME_CONFIG_RELOAD_PATH}`);
  assert.ok(
    status >= 200 && status < 300,
    `${peer}'s ${RUNTIME_CONFIG_RELOAD_PATH} returned ${status} while reloading its run-owned channel set`
  );
}

async function setPeerMode(_world: E2EWorld, peer: PeerName, mode: string): Promise<void> {
  setPeerModeCallCount += 1;
  runOwnedChannels[peer].set(CHANNEL_ID, mode);
  await applyRunOwnedChannels(peer);
}

// ---------------------------------------------------------------------------
// Run-owned follow set: established before Station 1 even runs (`Before`,
// tag-scoped to this feature's own `@runtime-upgrade`) and restored
// byte-for-byte no matter how the run ends (`AfterAll`). See the module doc
// above this section for the hazard this closes.
// ---------------------------------------------------------------------------

/** True once the true on-disk bytes have been captured and the run-owned file written for every peer. */
let runOwnedFollowSetEstablished = false;
/** True once `AfterAll` has restored every peer — guards against a double-restore. */
let runOwnedFollowSetRestored = false;
/** The TRUE pre-run bytes, per peer — restored byte-for-byte in `AfterAll`, never mutated after capture. */
const originalRuntimeConfigBytes: Partial<Record<PeerName, Buffer>> = {};

/**
 * `E2E_STORAGE_<PEER>` — the exact env var the Background's own
 * `Given peer "<peer>" at "E2E_STORAGE_<PEER>"` step resolves
 * `peerUrl`/`world.getDoorway(peer).url` from (`steps/dataplane.steps.ts`'s
 * `resolvePeerUrl`). Read directly here because `AfterAll` runs with no
 * `World` (no scenario, no doorway registry) to read `peerUrl` from.
 */
function directPeerUrl(peer: PeerName): string {
  const envVar = `E2E_STORAGE_${peer.toUpperCase()}`;
  const url = process.env[envVar];
  assert.ok(url, `${envVar} is not set — cannot reach ${peer} to restore its runtime-config`);
  return url.replace(/\/$/, '');
}

/**
 * Logs (never fails on) any leftover non-run channel already sitting in
 * `apply`/`canary` in a peer's TRUE original bytes — a receipt for the next
 * operator, not a gate for this run.
 */
function logLeftoverChannels(peer: PeerName, originalBytes: Buffer): void {
  const originalValue = extractChannelsValueFromToml(originalBytes.toString('utf8'));
  for (const entry of originalValue
    .split(/[,;\n]/)
    .map(e => e.trim())
    .filter(Boolean)) {
    const [entryId, entryMode] = entry.split('=').map(part => part.trim());
    if (
      entryId &&
      entryId !== CHANNEL_ID &&
      entryId !== PERSONAL_CHANNEL_ID &&
      (entryMode === 'apply' || entryMode === 'canary')
    ) {
      // eslint-disable-next-line no-console
      console.log(
        `  ⚠️  leftover channel on ${peer} in mode "${String(entryMode)}": ${entryId} — ` +
          "an operator-hygiene leftover from an earlier ceremony, not this run's concern; " +
          'this run owns its own follow set and does not depend on it being cleaned up.'
      );
    }
  }
}

/**
 * Captures one peer's TRUE on-disk bytes exactly ONCE — idempotent across
 * retries. This matters because `ensureRunOwnedFollowSet`'s loop can be
 * re-entered by a LATER scenario's `Before` hook after an EARLIER scenario's
 * attempt already ran (and possibly already overwrote peer N's own file)
 * but then threw on peer N+1, leaving `runOwnedFollowSetEstablished` false.
 * Without this per-peer guard, that retry would snapshot peer N's ALREADY
 * run-owned bytes as if they were the true original — the exact
 * self-referential corruption a 2026-09-02 dry run of this file measured.
 */
function captureOriginalBytesOnce(peer: PeerName): void {
  if (peer in originalRuntimeConfigBytes) return;
  let originalBytes: Buffer;
  try {
    originalBytes = readFileSync(runtimeConfigPath(peer));
  } catch {
    originalBytes = Buffer.from('', 'utf8');
  }
  originalRuntimeConfigBytes[peer] = originalBytes;
  writeFileSync(path.join(REPORT_DIR, `runtime-config.${peer}.orig.toml`), originalBytes);
  logLeftoverChannels(peer, originalBytes);
}

/**
 * Captures each peer's TRUE on-disk bytes (once — see `captureOriginalBytesOnce`),
 * then overwrites the file with a run-owned set containing only
 * `CHANNEL_ID=observe` — the run's baseline before any station acts.
 */
async function ensureRunOwnedFollowSet(): Promise<void> {
  if (runOwnedFollowSetEstablished) return;
  mkdirSync(REPORT_DIR, { recursive: true });

  for (const peer of PEER_NAMES) {
    captureOriginalBytesOnce(peer);
    runOwnedChannels[peer] = new Map([[CHANNEL_ID, 'observe']]);
    await applyRunOwnedChannels(peer);
  }
  runOwnedFollowSetEstablished = true;
}

/** Same extraction `readChannelsValue` does, but from bytes already in hand (no re-read from disk). */
function extractChannelsValueFromToml(content: string): string {
  const match = /ELOHIM_RELEASE_CHANNELS\s*=\s*"([^"]*)"/.exec(content);
  return match ? match[1] : '';
}

/** Byte-restores one peer's runtime-config.toml and reloads it — best-effort on the reload. */
async function restoreOnePeerRuntimeConfig(peer: PeerName, originalBytes: Buffer): Promise<void> {
  writeFileSync(runtimeConfigPath(peer), originalBytes);
  try {
    const { status } = await postRaw(`${directPeerUrl(peer)}${RUNTIME_CONFIG_RELOAD_PATH}`);
    if (status < 200 || status >= 300) {
      // eslint-disable-next-line no-console
      console.log(`  ⚠️  restore reload on ${peer} returned ${status}`);
    }
  } catch (error) {
    // eslint-disable-next-line no-console
    console.log(`  ⚠️  restore reload on ${peer} failed: ${String(error)}`);
  }
}

/**
 * Receipt-only confirmation that no run-created channel remains in
 * apply/canary on one peer after the byte-restore above — the byte-restore
 * is the authority; this never retries or fails the run, it only names what
 * it sees.
 */
async function confirmOnePeerRestored(peer: PeerName): Promise<void> {
  try {
    const { status, text } = await getRaw(`${directPeerUrl(peer)}${ADOPTION_PATH}`, {
      timeoutMs: 10_000,
    });
    if (status !== 200) return;
    const report = JSON.parse(text) as AdoptionReport;
    for (const channelId of [CHANNEL_ID, PERSONAL_CHANNEL_ID]) {
      const row = report.channels.find(r => r.channelId === channelId);
      if (row && (row.mode === 'apply' || row.mode === 'canary')) {
        // eslint-disable-next-line no-console
        console.log(
          `  ⚠️  ${peer} still reports ${channelId} in mode "${row.mode}" after byte-restore`
        );
      }
    }
    // eslint-disable-next-line no-console
    console.log(`  ✅  ${peer}'s runtime-config.toml byte-restored to its pre-run bytes.`);
  } catch {
    // Best-effort confirmation only — the byte-restore already happened above.
  }
}

AfterAll({ timeout: 900_000 }, async function () {
  // Restore whenever ANY peer was snapshotted — not gated on full
  // establishment completing, so a mid-loop throw (a station/env failure
  // partway through `ensureRunOwnedFollowSet`) still restores whatever
  // peers it already touched, rather than leaving them run-owned forever.
  if (Object.keys(originalRuntimeConfigBytes).length === 0 || runOwnedFollowSetRestored) return;
  runOwnedFollowSetRestored = true;

  // Converge every peer back onto baseline N BEFORE the byte-restore below
  // — see the "Pre/post-run convergence to baseline N" section for the r5
  // diagnosis this closes — so the mesh is left homogeneous on N for
  // whichever run starts next, not on whatever hash this run's own stations
  // (station 7's never-reverted personal channel, or a station 6 that never
  // converged) happened to leave behind. Best-effort: a convergence failure
  // here must never block the byte-restore that follows, or a run-owned
  // file becomes permanent.
  try {
    await convergeAllToBaseline(directPeerUrl, 'post');
  } catch (error) {
    // eslint-disable-next-line no-console
    console.log(
      `  ⚠️  post-run convergence-to-baseline failed before byte-restore: ${String(error)} — ` +
        'proceeding to byte-restore anyway so the next run does not inherit a run-owned file.'
    );
  }

  for (const peer of PEER_NAMES) {
    const originalBytes = originalRuntimeConfigBytes[peer];
    if (originalBytes === undefined) continue;
    await restoreOnePeerRuntimeConfig(peer, originalBytes);
  }

  for (const peer of PEER_NAMES) {
    await confirmOnePeerRestored(peer);
  }
});

/**
 * Tag-scoped to this feature's own `@runtime-upgrade` so no other suite's
 * runtime-config is ever touched. Fires before Station 1's own Given/When
 * steps, establishing the run-owned follow set the whole ceremony depends
 * on — see the module doc above `runOwnedChannels`.
 */
Before({ tags: '@runtime-upgrade', timeout: 60_000 }, async function (this: E2EWorld) {
  await ensureRunOwnedFollowSet();
});

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

/**
 * `revert <channel> <manifest.json>` is the one verb that prints TWO
 * pretty-printed JSON objects (`authorVersionFromManifest`'s "publish"-shaped
 * echo, then `declareEarned`'s own result) — `extractJson` would slice from
 * the FIRST `{` and hand `JSON.parse` two concatenated objects, which throws.
 * Each `JSON.stringify(obj, null, 2)` block starts with a bare `{` at column
 * 0, so the LAST such line is the start of the real (declare-earned) result.
 */
function extractLastJson<T>(stdout: string): T {
  const starts = [...stdout.matchAll(/^\{$/gm)].map(match => match.index ?? -1).filter(i => i >= 0);
  assert.ok(starts.length > 0, `no JSON object in driver output: ${stdout.slice(0, 400)}`);
  const start = starts.at(-1) ?? 0;
  return JSON.parse(stdout.slice(start)) as T;
}

// ---------------------------------------------------------------------------
// Pre/post-run convergence to baseline N — the fix for the flaw the
// 2026-09-02 06:16Z (r5) run measured: this ceremony's own steady state
// after a green run is HETEROGENEOUS, not converged. Station 6 restores the
// commons channel (`CHANNEL_ID`) to `BASELINE_HAPP`, but station 7 leaves
// james on his own personal channel's candidate forever — nobody reverts a
// personal channel, and nobody is meant to — so a green run's own receipt
// ends with james on one coordinator hash and matthew/jessica on another.
// If a PRIOR run's own station 6 revert never converged either (as r4's did
// not), matthew/jessica are left on THAT run's candidate too. The next run's
// station 1/3 packages its release with `--applies-to-from <matthew>`,
// baking matthew's LIVE hash into the manifest's "supersedes" list — a hash
// james, running a different leftover, does not run — so his canary apply
// is refused `coordinator_lineage_mismatch`. The controller is correct to
// refuse; the fixture lacked a teardown.
//
// The fix: before this run's own station 1 ever packages a release
// (`ensurePreRunConvergedToBaseline`, called from `ensureCeremonyBaseline`
// below), and again after this run's own stations finish but BEFORE its
// runtime-config byte-restore (`AfterAll`, so whichever run starts next
// inherits a homogeneous fleet), read every peer's live lamad
// `coordinatorWasmHashes` and, for every DISTINCT hash present that is not
// `BASELINE_COORDINATOR_WASM_HASH`, package `BASELINE_HAPP`'s own bytes as a
// throwaway "teardown" release bound to that exact hash, publish + promote
// it on a fresh teardown channel followed (in `apply` mode) by exactly the
// peer(s) reporting it, wait for their passports to converge on the
// baseline, then drop the teardown channel from the run-owned follow set
// again. Bounded by peer count: at most `PEER_NAMES.length` distinct
// non-baseline groups can ever exist.
// ---------------------------------------------------------------------------

/**
 * The coordinator `WasmHash` (Holochain's own blake2b + holo-hash encoding —
 * NOT the bare sha256 `coordinator-candidate.ts` computes over different
 * bytes; see `MintedCandidate.coordinatorWasmSha256`'s own doc) the fleet
 * reports for lamad's `content_store` coordinator when running
 * `BASELINE_HAPP` (bundle "N") unmodified. The bundle pin and this hash pin
 * are ONE pair: a new baseline N (a DNA rebuild, a conductor-line change)
 * moves both, so `E2E_BASELINE_HAPP` and `E2E_BASELINE_COORDINATOR_WASM_HASH`
 * override them together. The default is the holochain 0.7.0 baseline N
 * preserved under `reports/release-ceremony/2026-09-03/baseline-N/` (the 0.6
 * pair, 2026-09-02, was `uhCok88IBM4RXPCWLyz_y00a8Ksyyys03p8Fq7dzOWj1gyqRx7H0B`).
 * Re-derive it without a conductor: `hc dna unpack <workdir>/lamad.dna -o d`
 * then WasmHash = 'u' + base64url( 0x84 0x2a 0x24 ‖ blake2b-256(wasm) ‖ loc )
 * where loc = the 16-byte blake2b of that 32-byte core xor-folded to 4 bytes
 * (reproduced the conductor's value byte-exact on 2026-09-03). It is asserted
 * again, for real, every time `runOneTeardown` polls a peer back onto it: a
 * wrong pin fails that poll loudly rather than silently declaring the wrong
 * hash "baseline."
 */
const BASELINE_COORDINATOR_WASM_HASH =
  process.env['E2E_BASELINE_COORDINATOR_WASM_HASH'] ??
  'uhCokta9pl3fv4sMfPbWWh_Q4fXh0UA1LsCdTmyuJQ4AZnquZs7fX';

/** At most this many distinct non-baseline hash groups can exist across `PEER_NAMES`. */
const MAX_TEARDOWN_GROUPS = PEER_NAMES.length;
const TEARDOWN_POLL_TIMEOUT_MS = 4 * 60_000;

/**
 * Resolves a peer's base URL without a `World` — `(peer) => peerUrl(world,
 * peer)` for the Background-driven pre-run pass, `directPeerUrl` itself for
 * `AfterAll`'s post-run pass (no `World` exists there — see `directPeerUrl`'s
 * own doc).
 */
type PeerUrlResolver = (peer: PeerName) => string;

interface PeerHashGroup {
  /** Sorted, deduped hash values shared by every peer in `peers` (see `wasmHashValues`). */
  hashValues: string[];
  peers: PeerName[];
}

async function readAllPeerLamadHashValues(
  resolveUrl: PeerUrlResolver
): Promise<Partial<Record<PeerName, string[]>>> {
  const out: Partial<Record<PeerName, string[]>> = {};
  for (const peer of PEER_NAMES) {
    const roles = await readRolesAt(resolveUrl(peer), peer);
    out[peer] = wasmHashValues(lamadRole(roles)?.coordinatorWasmHashes);
  }
  return out;
}

/** Groups peers NOT already on the baseline hash by their shared, distinct hash-set. */
function groupNonBaselinePeers(readings: Partial<Record<PeerName, string[]>>): PeerHashGroup[] {
  const groups = new Map<string, PeerHashGroup>();
  for (const peer of PEER_NAMES) {
    const values = readings[peer] ?? [];
    if (values.length === 0) {
      // eslint-disable-next-line no-console
      console.log(
        `  ⚠️  ${peer} reports no lamad coordinatorWasmHashes at all — nothing to bind a ` +
          'teardown supersedes to; leaving it untouched.'
      );
      continue;
    }
    if (values.length === 1 && values[0] === BASELINE_COORDINATOR_WASM_HASH) continue;
    const hashKey = values.join(',');
    const existing = groups.get(hashKey);
    if (existing) existing.peers.push(peer);
    else groups.set(hashKey, { hashValues: values, peers: [peer] });
  }
  return [...groups.values()];
}

/**
 * Tears down ONE distinct non-baseline hash group: packages `BASELINE_HAPP`
 * bound to that exact hash on a fresh teardown channel, has exactly the
 * affected peers follow it in `apply` mode, declares it earned, waits for
 * their passports to converge, then drops the channel from the run-owned
 * follow set again.
 */
async function runOneTeardown(
  resolveUrl: PeerUrlResolver,
  phase: 'pre' | 'post',
  group: PeerHashGroup,
  index: number
): Promise<void> {
  const teardownChannelId = `runtime:coordinators:elohim:a2o-teardown-${RUN_STAMP}-${phase}-${index}`;
  const teardownManifestPath = path.join(
    REPORT_DIR,
    `a2o-teardown-${RUN_STAMP}-${phase}-${index}.json`
  );
  mkdirSync(REPORT_DIR, { recursive: true });
  const start = Date.now();
  // eslint-disable-next-line no-console
  console.log(
    `[${phase}-run-convergence] teardown ${index}: ${group.peers.join('+')} report ` +
      `${JSON.stringify(group.hashValues)} — packaging baseline N bound to that hash on ` +
      `channel ${teardownChannelId}`
  );

  const created = runDriver(RELEASE_CEREMONY_SCRIPT, [
    'channel',
    'create',
    teardownChannelId,
    '--discipline',
    JSON.stringify({ soakSecs: 0, attestationThreshold: 0, canaryOrder: [] }),
  ]);
  assert.equal(
    created.status,
    0,
    `teardown channel create failed (exit ${created.status}):\n--- stdout ---\n${created.stdout.trim()}\n--- stderr ---\n${created.stderr.trim()}`
  );

  const repPeer = group.peers[0];
  const repPeerUrl = resolveUrl(repPeer);
  const packaged = runDriver(
    EPR_RELEASE_PACKAGE_SCRIPT,
    [
      FLAG_ARTIFACT,
      BASELINE_HAPP,
      FLAG_ARTIFACT_CLASS,
      ARTIFACT_CLASS_COORDINATOR_BUNDLE,
      FLAG_CHANNEL_ID,
      teardownChannelId,
      FLAG_APPLIES_TO_FROM,
      repPeerUrl,
      FLAG_SOAK_SECS,
      '0',
      FLAG_ATTESTATION_THRESHOLD,
      '0',
      '--peer',
      repPeerUrl,
      '--notes',
      `${phase}-run convergence: teardown to baseline N before run ${RUN_STAMP} ` +
        `(was ${group.hashValues.join(',')})`,
      '--out',
      teardownManifestPath,
    ],
    60_000
  );
  assert.equal(
    packaged.status,
    0,
    `teardown package (peers ${group.peers.join('+')}) failed (exit ${packaged.status}):\n--- stdout ---\n${packaged.stdout.trim()}\n--- stderr ---\n${packaged.stderr.trim()}`
  );
  assert.ok(
    existsSync(teardownManifestPath),
    `teardown manifest was not written to ${teardownManifestPath}`
  );

  const baselineBytes = readFileSync(BASELINE_HAPP);
  const baselineSha256 = createHash('sha256').update(baselineBytes).digest('hex');
  for (const peer of group.peers.filter(name => name !== repPeer)) {
    const putStatus = await putBlobRaw(resolveUrl(peer), baselineSha256, baselineBytes);
    assert.ok(
      putStatus === 200 || putStatus === 201,
      `PUT teardown-target blob to ${peer} returned ${putStatus}`
    );
  }

  const published = runDriver(RELEASE_CEREMONY_SCRIPT, ['publish', teardownManifestPath]);
  assert.equal(
    published.status,
    0,
    `teardown publish failed (exit ${published.status}):\n--- stdout ---\n${published.stdout.trim()}\n--- stderr ---\n${published.stderr.trim()}`
  );
  const publishedParsed = extractJson<PublishResult>(published.stdout);
  assert.ok(
    publishedParsed.releaseCid,
    `teardown publish missing releaseCid: ${JSON.stringify(publishedParsed)}`
  );

  for (const peer of group.peers) {
    runOwnedChannels[peer].set(teardownChannelId, 'apply');
    await applyRunOwnedChannels(peer);
  }

  const promoted = runDriver(RELEASE_CEREMONY_SCRIPT, [
    'promote',
    teardownChannelId,
    publishedParsed.releaseCid,
  ]);
  assert.equal(
    promoted.status,
    0,
    `teardown promote failed (exit ${promoted.status}):\n--- stdout ---\n${promoted.stdout.trim()}\n--- stderr ---\n${promoted.stderr.trim()}`
  );
  const promotedParsed = extractJson<PromoteResult>(promoted.stdout);
  assert.equal(
    promotedParsed.tier,
    'earned',
    `teardown promote did not declare earned: ${JSON.stringify(promotedParsed)}`
  );

  for (const peer of group.peers) {
    const { observed, elapsedMs } = await pollLamadWasmHashMatchAt(
      resolveUrl(peer),
      peer,
      { baseline: BASELINE_COORDINATOR_WASM_HASH },
      TEARDOWN_POLL_TIMEOUT_MS
    );
    // eslint-disable-next-line no-console
    console.log(
      `[${phase}-run-convergence] teardown ${index}: ${peer} converged on baseline ` +
        `${JSON.stringify(observed)} after ${elapsedMs}ms`
    );
  }

  for (const peer of group.peers) {
    runOwnedChannels[peer].delete(teardownChannelId);
    await applyRunOwnedChannels(peer);
  }

  // eslint-disable-next-line no-console
  console.log(
    `[${phase}-run-convergence] teardown ${index} done in ${Date.now() - start}ms — ` +
      `${group.peers.join('+')} dropped from ${teardownChannelId}'s follow set`
  );
}

/** Reads every peer's live lamad hash and tears down every distinct non-baseline group, in order. */
async function convergeAllToBaseline(
  resolveUrl: PeerUrlResolver,
  phase: 'pre' | 'post'
): Promise<void> {
  const readings = await readAllPeerLamadHashValues(resolveUrl);
  const groups = groupNonBaselinePeers(readings);
  // eslint-disable-next-line no-console
  console.log(
    `[${phase}-run-convergence] observed lamad coordinatorWasmHashes: ${JSON.stringify(readings)} — ` +
      `${groups.length} distinct non-baseline group(s) need teardown`
  );
  assert.ok(
    groups.length <= MAX_TEARDOWN_GROUPS,
    `${phase}-run convergence found ${groups.length} distinct non-baseline coordinator hashes across ` +
      `only ${PEER_NAMES.length} peers — impossible unless the grouping logic double-counted`
  );
  for (const [index, group] of groups.entries()) {
    await runOneTeardown(resolveUrl, phase, group, index);
  }

  if (groups.length > 0) {
    // 2026-09-02 r6 diagnosis: a teardown's own poll confirms the peer's REAL
    // `/version` passport landed on baseline, but the release-adoption
    // controller's `installed_reality()` cache (the SAME
    // `INSTALLED_REALITY_TTL_SECS` cache `ensureReverted`/station 6 already
    // waits out — see that constant's own doc) is a SEPARATE, unrelated read
    // that does not invalidate just because the wasm hot-swapped. r6 measured
    // this directly: teardown 1 confirmed james's live passport on baseline
    // at T+126s, then station 3 — only ~45s later, nowhere near the 300s TTL
    // — was refused `coordinator_lineage_mismatch` citing james still
    // "running" his PRE-teardown hash, a stale controller-cache read, not a
    // real one. Wait out the same TTL (once, after every teardown this pass
    // ran) before returning, so whichever station verifies against any
    // torn-down peer next reads a cache that has actually refreshed.
    const ttlWaitMs = (INSTALLED_REALITY_TTL_SECS + INSTALLED_REALITY_TTL_BUFFER_SECS) * 1000;
    // eslint-disable-next-line no-console
    console.log(
      `[${phase}-run-convergence] waiting ${ttlWaitMs}ms for the release-adoption controller's ` +
        `installed-reality cache (TTL ${INSTALLED_REALITY_TTL_SECS}s) to age out on every torn-down ` +
        'peer before any station verifies a release against it'
    );
    await new Promise<void>(resolve => setTimeout(resolve, ttlWaitMs));
  }
}

let preRunConvergenceDone = false;

/** Called once, from `ensureCeremonyBaseline`, before this run's own station 1 packages anything. */
async function ensurePreRunConvergedToBaseline(world: E2EWorld): Promise<void> {
  if (preRunConvergenceDone) return;
  await convergeAllToBaseline(peer => peerUrl(world, peer), 'pre');
  preRunConvergenceDone = true;
}

// ---------------------------------------------------------------------------
// Ensure-chain: each function performs its station's REAL action exactly
// once per process (memoized on `ceremony()` state) and walks back through
// its own dependencies first, so any station can also run standalone.
// ---------------------------------------------------------------------------

async function ensureCeremonyBaseline(world: E2EWorld): Promise<void> {
  const c = ceremony();
  if (c.baselineEstablished) return;
  // Self-contained even if a station runs standalone outside the normal
  // cucumber `Before` hook — establishes the run-owned follow set first,
  // idempotently, exactly like every other `ensure*` in this chain.
  await ensureRunOwnedFollowSet();
  // Converge every peer onto the SAME baseline hash before this run's own
  // station 1 packages anything — see the "Pre/post-run convergence to
  // baseline N" section above for the r5 diagnosis this closes.
  await ensurePreRunConvergedToBaseline(world);
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

  const candidatePath = candidateHapp();
  assert.ok(existsSync(candidatePath), `candidate bundle missing: ${candidatePath}`);
  c.candidateBytes = readFileSync(candidatePath);
  c.candidateSha256 = createHash('sha256').update(c.candidateBytes).digest('hex');
  mkdirSync(REPORT_DIR, { recursive: true });

  const matthewUrl = peerUrl(world, 'matthew');
  const packaged = runDriver(
    EPR_RELEASE_PACKAGE_SCRIPT,
    [
      FLAG_ARTIFACT,
      candidatePath,
      FLAG_ARTIFACT_CLASS,
      ARTIFACT_CLASS_COORDINATOR_BUNDLE,
      FLAG_CHANNEL_ID,
      CHANNEL_ID,
      FLAG_APPLIES_TO_FROM,
      matthewUrl,
      FLAG_SOAK_SECS,
      String(SOAK_SECS),
      FLAG_ATTESTATION_THRESHOLD,
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
    `epr-release-package.ts failed (exit ${packaged.status}):\n--- stdout ---\n${packaged.stdout.trim()}\n--- stderr ---\n${packaged.stderr.trim()}`
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
    const created = runDriver(RELEASE_CEREMONY_SCRIPT, [
      'channel',
      'create',
      CHANNEL_ID,
      '--discipline',
      discipline,
    ]);
    assert.equal(
      created.status,
      0,
      `channel create failed (exit ${created.status}):\n--- stdout ---\n${created.stdout.trim()}\n--- stderr ---\n${created.stderr.trim()}`
    );
    c.channelCreated = true;
  }

  const published = runDriver(RELEASE_CEREMONY_SCRIPT, ['publish', MANIFEST_PATH]);
  assert.equal(
    published.status,
    0,
    `publish failed (exit ${published.status}):\n--- stdout ---\n${published.stdout.trim()}\n--- stderr ---\n${published.stderr.trim()}`
  );
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

  // Station 7's "as they happened" recording: james's personal-channel row
  // AT THIS MOMENT — never re-read later, once the ceremony has moved past
  // staging (a later station's live read would show reverted/earned, not
  // staging).
  if (c.personalReleaseCid && !c.personalRowsByStation.staging) {
    c.personalRowsByStation.staging = await readJamesChannelRows(world);
  }
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

  // Ground truth for every later station's convergence check: james's own
  // `/version` passport, read immediately after his controller reported
  // `appliedRelease.cid === c.releaseCid` — never `appliedRelease.cid` alone,
  // which names the RELEASE object, not the coordinator wasm the conductor
  // actually swapped in.
  c.candidateWasmHashes = lamadRole(c.jamesAfter.roles)?.coordinatorWasmHashes;
  // eslint-disable-next-line no-console
  console.log(
    `[station3] james applied release ${c.releaseCid}: lamad coordinatorWasmHashes=` +
      `${JSON.stringify(c.candidateWasmHashes)} (mint sha256: happ=${c.candidateSha256}, ` +
      `coordinator-wasm=${c.candidateWasmSha256})`
  );
  assert.ok(
    c.candidateWasmHashes && Object.keys(c.candidateWasmHashes).length > 0,
    `james's /version passport reports no lamad coordinatorWasmHashes after the canary apply — ` +
      `nothing to compare later stations' convergence against`
  );

  // james's own reality just moved onto the commons candidate — his personal
  // channel's envelope (packaged in the Background against matthew's
  // pre-ceremony baseline) no longer names what he runs. Rebase it now,
  // before any later station reads his personal-channel row. See
  // `republishPersonalVariant`'s own doc for the rule this is an instance of.
  await republishPersonalVariant(world, 'promotion');
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
    RELEASE_CEREMONY_SCRIPT,
    ['promote', CHANNEL_ID, c.releaseCid],
    60_000
  );
  assert.equal(
    promoted.status,
    0,
    `promote failed (exit ${promoted.status}):\n--- stdout ---\n${promoted.stdout.trim()}\n--- stderr ---\n${promoted.stderr.trim()}`
  );
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
  // The last runtime-config lever this ceremony ever pulls for CHANNEL_ID —
  // station 6's "nothing outside the ceremony" Then proves revert pulls none.
  c.setPeerModeCallsAfterPromotion = setPeerModeCallCount;
  for (const peer of PEER_NAMES) {
    const { row } = await pollAdoption(
      world,
      peer,
      240_000,
      r => r.appliedRelease?.cid === c.releaseCid
    );
    const snapshot = before.get(peer);
    assert.ok(snapshot, `no before-snapshot captured for ${peer}`);

    // STRENGTHENED convergence check (2026-09-02 diagnosis): `appliedRelease.cid`
    // names the RELEASE object this peer's controller believes it applied —
    // it does NOT, by itself, prove the conductor's coordinator wasm actually
    // hot-swapped, and the two can be observed apart (an async propagation
    // lag — see `pollLamadWasmHashMatch`'s own doc). POLL the peer's own
    // `/version` passport (bounded, not a single racy snapshot) until it
    // matches the ground truth station 3 established from james's
    // post-apply passport — never compare against `c.releaseCid`, a
    // different namespace (a release object's cid) entirely.
    const { observed: observedHashes, elapsedMs: wasmConvergeMs } = await pollLamadWasmHashMatch(
      world,
      peer,
      c.candidateWasmHashes,
      120_000
    );
    // eslint-disable-next-line no-console
    console.log(
      `[station5] ${peer}: appliedRelease.cid=${row.appliedRelease?.cid} lamad ` +
        `coordinatorWasmHashes=${JSON.stringify(observedHashes)} (candidate: ` +
        `${JSON.stringify(c.candidateWasmHashes)}) — matched after ${wasmConvergeMs}ms of ` +
        `passport polling past the appliedRelease.cid match`
    );

    c.fleetRows[peer] = {
      row,
      pidBefore: snapshot.pid,
      pidAfter: readPid(peer),
      healthBefore: snapshot.health,
      healthAfter: await readBlobTotal(world, peer),
    };
  }
  c.fleetConvergeMs = Date.now() - start;

  // Station 7's "as they happened" recording — see the identical comment in
  // `ensureStagingConvergedAll`.
  if (c.personalReleaseCid && !c.personalRowsByStation.promotion) {
    c.personalRowsByStation.promotion = await readJamesChannelRows(world);
  }
}

// ---------------------------------------------------------------------------
// Station 6 — revert by re-election. The prior release is packaged from
// BASELINE_HAPP (pre-fix bytes, already on disk — see the constant's doc)
// bound to the CURRENT fleet reality (a peer already running the fix), per
// the r2 receipt's Station 8 finding: "a revert target is a NEW manifest for
// OLD bytes… `revert <priorReleaseCid>` must take a manifest bound to the
// current fleet reality," not the reality that held before the fix shipped.
// ---------------------------------------------------------------------------

/** The one field of a release manifest this file reads back for verification. */
interface RevertManifestRoleBinding {
  coordinatorWasmHashes?: string[];
}
interface RevertManifestFile {
  appliesTo?: { roles?: Record<string, RevertManifestRoleBinding> };
}

async function ensureReverted(world: E2EWorld): Promise<void> {
  const c = ceremony();
  // Depend on the FULL station-5 chain (fleet convergence), not just
  // promotion (station 4) — 2026-09-02 diagnosis: this used to call only
  // `ensurePromoted`, so a standalone run of this station (or any future
  // reordering) could package the revert target BEFORE the fleet's
  // controllers had actually converged their conductors onto the candidate,
  // authoring a revert manifest whose "supersedes" hash no peer runs yet.
  await ensureFleetConverged(world);
  if (c.revertConvergeMs !== undefined) return;

  // Honest before-state, read the same way every other station reads it —
  // never asserted from the ceremony's own intent.
  for (const peer of PEER_NAMES) {
    const report = await getAdoptionReport(world, peer);
    const row = findChannelRow(report, CHANNEL_ID);
    if (row) c.preRevertRows[peer] = row;
  }

  if (!c.priorReleaseCid) {
    // Wait out the release-adoption controller's installed-reality cache TTL
    // (see the `INSTALLED_REALITY_TTL_SECS` doc above) BEFORE packaging or
    // publishing the revert — every peer's fleet-convergence apply just
    // completed, and that same event is what the controller's cache may
    // still be stale about for up to `INSTALLED_REALITY_TTL_SECS` more
    // seconds. This is the actual fix for the 2026-09-02 diagnosis: without
    // it, the revert's verify check can run inside that window and see
    // pre-apply "installed reality" even though every peer's live /version
    // (confirmed by the polls above and below) already shows the candidate.
    const ttlWaitMs = (INSTALLED_REALITY_TTL_SECS + INSTALLED_REALITY_TTL_BUFFER_SECS) * 1000;
    // eslint-disable-next-line no-console
    console.log(
      `[station6] waiting ${ttlWaitMs}ms for the release-adoption controller's installed-reality ` +
        `cache (TTL ${INSTALLED_REALITY_TTL_SECS}s) to age out before packaging/publishing the revert`
    );
    await new Promise<void>(resolve => setTimeout(resolve, ttlWaitMs));

    assert.ok(
      existsSync(BASELINE_HAPP),
      `revert-target (pre-fix) bundle missing: ${BASELINE_HAPP}`
    );
    const matthewUrl = peerUrl(world, 'matthew');

    // Confirm (poll, not a single racy snapshot — station 5's own
    // convergence check can pass while the real hot-swap is still landing;
    // see `pollLamadWasmHashMatch`'s doc) matthew's LIVE lamad
    // coordinatorWasmHashes immediately before packaging — `--applies-to-from`
    // below reads the identical `GET /version` endpoint inside
    // epr-release-package.ts, so this (a) proves what ground truth was
    // available AT THAT MOMENT and (b) fails fast, with a clear message,
    // instead of a confusing multi-minute `coordinator_lineage_mismatch`
    // timeout later if matthew never actually lands on the candidate.
    const { observed: matthewLamadBeforePackaging, elapsedMs: matthewWasmConvergeMs } =
      await pollLamadWasmHashMatch(world, 'matthew', c.candidateWasmHashes, 60_000);
    // eslint-disable-next-line no-console
    console.log(
      `[station6] matthew's live lamad coordinatorWasmHashes read immediately before packaging the ` +
        `revert manifest: ${JSON.stringify(matthewLamadBeforePackaging)} (candidate established in ` +
        `station 3/5: ${JSON.stringify(c.candidateWasmHashes)}; matched after an additional ` +
        `${matthewWasmConvergeMs}ms of polling)`
    );

    const packaged = runDriver(
      'scripts/epr-release-package.ts',
      [
        '--artifact',
        BASELINE_HAPP,
        '--artifact-class',
        'coordinator-bundle',
        '--channel-id',
        CHANNEL_ID,
        '--applies-to-from',
        matthewUrl,
        '--soak-secs',
        String(SOAK_SECS),
        // A revert needs nothing but the ceremony saying so (this station's
        // title): the manifest's own adoption discipline declares threshold 0,
        // because nobody attests a release the fleet is being asked to LEAVE —
        // r7 (2026-09-02) packaged it at the forward threshold (1) and every
        // peer refused `threshold_unmet` (0 of 1) forever. The lineage
        // envelope (appliesTo bound to the live passport) is the safety here.
        '--attestation-threshold',
        '0',
        '--canary',
        CANARY_PEER,
        '--peer',
        matthewUrl,
        '--notes',
        'a2o station 6: revert target — pre-fix baseline coordinator bundle',
        '--out',
        REVERT_MANIFEST_PATH,
      ],
      60_000
    );
    assert.equal(
      packaged.status,
      0,
      `epr-release-package.ts (revert target) failed (exit ${packaged.status}):\n--- stdout ---\n${packaged.stdout.trim()}\n--- stderr ---\n${packaged.stderr.trim()}`
    );
    assert.ok(
      existsSync(REVERT_MANIFEST_PATH),
      `revert manifest was not written to ${REVERT_MANIFEST_PATH}`
    );

    // Read the manifest back and compare against both reads above — proves
    // (or disproves) that `--applies-to-from` captured the SAME ground truth
    // this function just observed matthew running, one call earlier.
    const revertManifest = JSON.parse(
      readFileSync(REVERT_MANIFEST_PATH, 'utf8')
    ) as RevertManifestFile;
    const manifestLamadHashes =
      revertManifest.appliesTo?.roles?.[LAMAD_ROLE]?.coordinatorWasmHashes;
    // eslint-disable-next-line no-console
    console.log(
      `[station6] revert manifest ${REVERT_MANIFEST_PATH} appliesTo.roles.${LAMAD_ROLE}.` +
        `coordinatorWasmHashes=${JSON.stringify(manifestLamadHashes)}`
    );
    assert.deepEqual(
      manifestLamadHashes ? [...manifestLamadHashes].sort((a, b) => a.localeCompare(b)) : [],
      wasmHashValues(c.candidateWasmHashes),
      `revert manifest declares supersedes=${JSON.stringify(manifestLamadHashes)}, which does not match ` +
        `the candidate hash established in station 3/5 (${JSON.stringify(c.candidateWasmHashes)}) — ` +
        `packaging did not capture the same ground truth this function just read from matthew`
    );

    const baselineBytes = readFileSync(BASELINE_HAPP);
    const baselineSha256 = createHash('sha256').update(baselineBytes).digest('hex');
    for (const peer of PEER_NAMES.filter(name => name !== 'matthew')) {
      const putStatus = await putBlobRaw(peerUrl(world, peer), baselineSha256, baselineBytes);
      assert.ok(
        putStatus === 200 || putStatus === 201,
        `PUT revert-target blob to ${peer} returned ${putStatus}`
      );
    }
    c.priorManifestPath = REVERT_MANIFEST_PATH;
  }

  assert.ok(c.priorManifestPath, 'no revert-target manifest path recorded');
  const reverted = runDriver(
    RELEASE_CEREMONY_SCRIPT,
    ['revert', CHANNEL_ID, c.priorManifestPath],
    90_000
  );
  assert.equal(reverted.status, 0, `revert failed: ${reverted.stderr || reverted.stdout}`);
  const parsed = extractLastJson<RevertResult>(reverted.stdout);
  assert.equal(parsed.tier, 'earned', `revert did not declare earned: ${JSON.stringify(parsed)}`);
  assert.ok(parsed.releaseCid, `revert result missing releaseCid: ${JSON.stringify(parsed)}`);
  c.revertResult = parsed;
  c.priorReleaseCid = parsed.releaseCid;

  const start = Date.now();
  for (const peer of PEER_NAMES) {
    const beforeSnap = c.fleetRows[peer];
    assert.ok(beforeSnap, `no fleet-convergence snapshot for ${peer} — run station 5 first`);
    const { row } = await pollAdoption(
      world,
      peer,
      240_000,
      r => r.appliedRelease?.cid === c.priorReleaseCid
    );
    c.postRevertRows[peer] = {
      row,
      pidBefore: beforeSnap.pidAfter,
      pidAfter: readPid(peer),
      healthBefore: beforeSnap.healthAfter,
      healthAfter: await readBlobTotal(world, peer),
    };
  }
  c.revertConvergeMs = Date.now() - start;
  c.revertedAt = Date.now();

  // james's own reality just moved back onto baseline — the personal
  // channel's envelope from the "promotion" republish (bound to james
  // running the commons candidate) no longer names what he runs either.
  // Rebase it again, before recording the "revert" moment's row below. See
  // `republishPersonalVariant`'s own doc for the rule this is an instance of.
  await republishPersonalVariant(world, 'revert');

  // Station 7/8's "as they happened" recording — see the identical comment
  // in `ensureStagingConvergedAll`. Also the row station 8 builds its
  // "personal channel ran alongside the entire time" matrix cell from.
  if (c.personalReleaseCid && !c.personalRowsByStation.revert) {
    c.personalRowsByStation.revert = await readJamesChannelRows(world);
  }
}

// ---------------------------------------------------------------------------
// Station 7 — james's own personal channel: a genuine second channel he
// follows alongside commons, carrying a compatible variant nobody promotes
// and nobody forces him off.
// ---------------------------------------------------------------------------

async function ensurePersonalChannelFollowed(world: E2EWorld): Promise<void> {
  const c = ceremony();
  await ensureCeremonyBaseline(world);
  if (c.personalChannelFollowed) return;

  if (!c.personalChannelCreated) {
    const discipline = JSON.stringify({
      soakSecs: SOAK_SECS,
      attestationThreshold: ATTESTATION_THRESHOLD,
      canaryOrder: [],
    });
    const created = runDriver(RELEASE_CEREMONY_SCRIPT, [
      'channel',
      'create',
      PERSONAL_CHANNEL_ID,
      '--discipline',
      discipline,
    ]);
    assert.equal(
      created.status,
      0,
      `personal channel create failed (exit ${created.status}):\n--- stdout ---\n${created.stdout.trim()}\n--- stderr ---\n${created.stderr.trim()}`
    );
    c.personalChannelCreated = true;
  }

  // james's run-owned commons mode is whatever it already is in the map
  // (Background fires after the `Before` hook establishes it, so this is
  // `observe` at this point) — `runOwnedChannels['james']` already carries
  // that entry, so this only ADDS the personal channel to the same map;
  // `applyRunOwnedChannels` renders both in the
  // `channelId=mode,channelId=mode` format `state.rs::parse_followed_channels`
  // reads.
  // `apply`, NOT `canary`: the personal variant is only ever published to the
  // staging tier and never promoted, and james's runtime must stay converged
  // on commons at every station (the story's own assertion). In `apply` mode a
  // staging head resolves (diverging, heard) and verdicts `waiting` — never
  // applied, never refused. r8 (2026-09-02) had `canary` here: james applied
  // the personal variant the moment it was published, so his coordinator slot
  // no longer matched the commons candidate's envelope and Station 3 refused
  // `coordinator_lineage_mismatch` — a race r7 happened to win the other way.
  // A node runs ONE coordinator per role; two channels whose releases both
  // supersede the same base cannot both be applied on it.
  runOwnedChannels.james.set(PERSONAL_CHANNEL_ID, 'apply');
  await applyRunOwnedChannels('james');
  c.personalChannelFollowed = true;
}

async function ensurePersonalVariantPublished(world: E2EWorld): Promise<void> {
  const c = ceremony();
  await ensurePersonalChannelFollowed(world);
  if (c.personalReleaseCid) return;

  // Independent of CHANNEL_ID's own fix — the personal channel carries its
  // own release, published from Background so it is already diverging
  // before Station 1's own publish ever runs (never a dependency on
  // `ensureStaged`, which is CHANNEL_ID's act, not this one's).
  const candidatePath = candidateHapp();
  assert.ok(existsSync(candidatePath), `personal-channel artifact missing: ${candidatePath}`);
  const matthewUrl = peerUrl(world, 'matthew');
  const packaged = runDriver(
    'scripts/epr-release-package.ts',
    [
      '--artifact',
      candidatePath,
      '--artifact-class',
      'coordinator-bundle',
      '--channel-id',
      PERSONAL_CHANNEL_ID,
      '--applies-to-from',
      matthewUrl,
      '--wire-epoch',
      '0',
      '--additive-only',
      'true',
      '--soak-secs',
      String(SOAK_SECS),
      '--attestation-threshold',
      String(ATTESTATION_THRESHOLD),
      '--peer',
      matthewUrl,
      '--notes',
      "a2o station 7: james's personal channel — compatible variant, never promoted",
      '--out',
      PERSONAL_MANIFEST_PATH,
    ],
    60_000
  );
  assert.equal(
    packaged.status,
    0,
    `personal-channel package failed (exit ${packaged.status}):\n--- stdout ---\n${packaged.stdout.trim()}\n--- stderr ---\n${packaged.stderr.trim()}`
  );
  assert.ok(existsSync(PERSONAL_MANIFEST_PATH), `personal-channel manifest was not written`);

  const published = runDriver(RELEASE_CEREMONY_SCRIPT, ['publish', PERSONAL_MANIFEST_PATH]);
  assert.equal(
    published.status,
    0,
    `personal-channel publish failed (exit ${published.status}):\n--- stdout ---\n${published.stdout.trim()}\n--- stderr ---\n${published.stderr.trim()}`
  );
  const parsed = extractJson<PublishResult>(published.stdout);
  assert.ok(
    parsed.releaseCid,
    `personal-channel publish missing releaseCid: ${JSON.stringify(parsed)}`
  );
  c.personalReleaseCid = parsed.releaseCid;
  // The Background publish IS the staging-moment head: james is on the
  // baseline until Station 3, so this cid (bound to the baseline) is what his
  // personal row must resolve when the after-staging report is read back.
  // r11 (2026-09-02) left this unset and the assertion fell back to the LATEST
  // cid (the post-revert rebase) — a bookkeeping miss, not a controller one.
  c.personalReleaseCidByStation.staging = parsed.releaseCid;
}

/**
 * A personal channel rebases when commons moves: its appliesTo is what it
 * supersedes, and a node runs one coordinator per role. `ensurePersonalVariantPublished`
 * packages the personal variant ONCE in the Background, bound to matthew's
 * pre-ceremony baseline reality (`--applies-to-from matthew`) — that envelope
 * is only honest as long as james's own runtime is still on baseline. Once
 * james applies the commons candidate (station 3) his live reality moves,
 * and the Background's envelope no longer names what he runs; the same
 * happens again in reverse once the commons revert (station 6) lands him
 * back on baseline. This republishes the SAME artifact on `PERSONAL_CHANNEL_ID`
 * bound to james's OWN live passport at the moment it is called — never
 * matthew's — so the personal channel's envelope always matches the reality
 * it is layered on top of, for exactly the two stations where that reality
 * changed. Station "staging" needs no republish: the Background's own
 * baseline-bound publish already matches, because nothing has moved james
 * yet.
 *
 * Idempotent per station (guarded on `personalReleaseCidByStation[station]`)
 * and a no-op if the personal channel was never established — every real
 * scenario in this feature establishes it from the Background before this
 * can be reached, but the guard keeps this function safe to call from any
 * ensure-chain entry point standalone, matching the file's own convention.
 *
 * Blocks until james's own `/admin/adoption` row for `PERSONAL_CHANNEL_ID`
 * resolves the new cid before returning, so callers can capture
 * `personalRowsByStation[station]` immediately afterward with no separate
 * poll of their own.
 */
async function republishPersonalVariant(
  world: E2EWorld,
  station: Exclude<PersonalStation, 'staging'>
): Promise<void> {
  const c = ceremony();
  if (c.personalReleaseCidByStation[station]) return;
  if (!c.personalChannelFollowed || !c.personalReleaseCid) return;

  const candidatePath = candidateHapp();
  assert.ok(existsSync(candidatePath), `personal-channel artifact missing: ${candidatePath}`);
  const jamesUrl = peerUrl(world, 'james');
  const matthewUrl = peerUrl(world, 'matthew');
  const republishManifestPath = path.join(
    REPORT_DIR,
    `a2o-runtime-upgrade-${RUN_STAMP}-personal-${station}.json`
  );
  const packaged = runDriver(
    'scripts/epr-release-package.ts',
    [
      '--artifact',
      candidatePath,
      '--artifact-class',
      'coordinator-bundle',
      '--channel-id',
      PERSONAL_CHANNEL_ID,
      '--applies-to-from',
      jamesUrl,
      '--wire-epoch',
      '0',
      '--additive-only',
      'true',
      '--soak-secs',
      String(SOAK_SECS),
      '--attestation-threshold',
      String(ATTESTATION_THRESHOLD),
      '--peer',
      matthewUrl,
      '--notes',
      `a2o station 7: james's personal channel rebased to his live reality after ${station} — ` +
        'a personal channel rebases when commons moves: its appliesTo is what it supersedes, ' +
        'and a node runs one coordinator per role',
      '--out',
      republishManifestPath,
    ],
    60_000
  );
  assert.equal(
    packaged.status,
    0,
    `personal-channel republish package (${station}) failed (exit ${packaged.status}):\n--- stdout ---\n${packaged.stdout.trim()}\n--- stderr ---\n${packaged.stderr.trim()}`
  );
  assert.ok(
    existsSync(republishManifestPath),
    `personal-channel republish manifest (${station}) was not written to ${republishManifestPath}`
  );

  const published = runDriver(RELEASE_CEREMONY_SCRIPT, ['publish', republishManifestPath]);
  assert.equal(
    published.status,
    0,
    `personal-channel republish publish (${station}) failed (exit ${published.status}):\n--- stdout ---\n${published.stdout.trim()}\n--- stderr ---\n${published.stderr.trim()}`
  );
  const parsed = extractJson<PublishResult>(published.stdout);
  assert.ok(
    parsed.releaseCid,
    `personal-channel republish (${station}) missing releaseCid: ${JSON.stringify(parsed)}`
  );
  c.personalReleaseCidByStation[station] = parsed.releaseCid;
  c.personalReleaseCid = parsed.releaseCid;

  const { elapsedMs } = await pollAdoption(
    world,
    'james',
    120_000,
    r => r.resolvedHead?.cid === parsed.releaseCid,
    PERSONAL_CHANNEL_ID
  );
  // eslint-disable-next-line no-console
  console.log(
    `[station7] personal channel republished after ${station}: james resolved ${parsed.releaseCid} ` +
      `after ${elapsedMs}ms (rebased to his own live reality via --applies-to-from ${jamesUrl})`
  );
}

async function readJamesChannelRows(world: E2EWorld): Promise<JamesChannelRows> {
  const report = await getAdoptionReport(world, 'james');
  return {
    commons: findChannelRow(report, CHANNEL_ID),
    personal: findChannelRow(report, PERSONAL_CHANNEL_ID),
  };
}

// ---------------------------------------------------------------------------
// Station 8 — the observed version matrix, assembled ONLY from the RECORDED
// `/admin/adoption` reads earlier stations already took (never a fresh read
// taken here, and never asserted from the ceremony's own intent).
// ---------------------------------------------------------------------------

function buildObservedMatrix(): MatrixRow[] {
  const c = ceremony();
  const rows: MatrixRow[] = [];
  for (const peer of PEER_NAMES) {
    const staging = c.stagingConvergedRows[peer];
    if (staging) {
      rows.push({
        peer,
        station: 'staging',
        releaseCid: staging.resolvedHead?.cid,
        tier: staging.resolvedHead?.tier,
        readAt: staging.lastCheckedAt,
        route: ADOPTION_PATH,
      });
    }
    const earned = c.fleetRows[peer];
    if (earned) {
      rows.push({
        peer,
        station: 'earned',
        releaseCid: earned.row.appliedRelease?.cid,
        tier: earned.row.resolvedHead?.tier,
        readAt: earned.row.lastCheckedAt,
        route: ADOPTION_PATH,
      });
    }
    const reverted = c.postRevertRows[peer];
    if (reverted) {
      rows.push({
        peer,
        station: 'reverted',
        releaseCid: reverted.row.appliedRelease?.cid,
        tier: reverted.row.resolvedHead?.tier,
        readAt: reverted.row.lastCheckedAt,
        route: ADOPTION_PATH,
      });
    }
  }
  const jamesPersonal = c.personalRowsByStation.revert?.personal;
  if (jamesPersonal) {
    rows.push({
      peer: 'james',
      station: 'personal',
      releaseCid: jamesPersonal.resolvedHead?.cid,
      tier: jamesPersonal.resolvedHead?.tier,
      readAt: jamesPersonal.lastCheckedAt,
      route: ADOPTION_PATH,
    });
  }
  return rows;
}

// ---------------------------------------------------------------------------
// Background — documentary declarations plus the one real precondition
// (peers following CHANNEL_ID in `observe` mode) they collectively describe.
// ---------------------------------------------------------------------------

Given(
  "the household's runtime follows release channel {string}",
  // Bumped from 120_000: `ensureCeremonyBaseline` now runs the pre-run
  // convergence-to-baseline pass first (up to `MAX_TEARDOWN_GROUPS` teardown
  // ceremonies, each with a `TEARDOWN_POLL_TIMEOUT_MS` poll) before it even
  // gets to setting every peer's mode to `observe` — see the "Pre/post-run
  // convergence to baseline N" section.
  { timeout: 900_000 },
  async function (this: E2EWorld, channelName: string) {
    assert.equal(
      channelName,
      STORY_COMMONS_CHANNEL_NAME,
      `unexpected commons channel name: ${channelName}`
    );
    await ensureCeremonyBaseline(this);
  }
);

Given("james is designated the canary on that channel's staging tier", function () {
  // Documentary: the packaged manifest's canaryOrder (set in ensureStaged) is
  // where this becomes real, exactly like `--canary james` in the receipt.
  assert.equal(CANARY_PEER, 'james');
});

Given(
  "james's runtime additionally follows his own personal channel {string}",
  // Bumped from 200_000 for the same reason as the previous Given — if this
  // station ever runs standalone (its own `ensureCeremonyBaseline` call is
  // the first one in the process), it inherits the full pre-run
  // convergence-to-baseline budget too.
  { timeout: 900_000 },
  async function (this: E2EWorld, channelName: string) {
    assert.equal(
      channelName,
      STORY_PERSONAL_CHANNEL_NAME,
      `unexpected personal channel name: ${channelName}`
    );
    await ensurePersonalChannelFollowed(this);
    // Published from the Background — before Station 1 even runs — so the
    // compatible variant is already diverging "all story long" and every
    // later station's own convergence (staging, promotion, revert) can
    // record james's personal-channel row AS IT HAPPENED, never a re-read
    // after the ceremony has already moved past that moment.
    await ensurePersonalVariantPublished(this);
  }
);

// ---------------------------------------------------------------------------
// Station 1 — publish as a staging candidate, never straight to earned.
// ---------------------------------------------------------------------------

Given(
  "matthew has built a coordinator fix — new behavior, no change to the household's DNA lineage",
  { timeout: 60_000 },
  function () {
    const candidatePath = candidateHapp();
    assert.ok(existsSync(candidatePath), `candidate coordinator bundle missing: ${candidatePath}`);
  }
);

When(
  'matthew publishes it as a release manifest on the channel {string}',
  { timeout: 90_000 },
  async function (this: E2EWorld, channelName: string) {
    assert.equal(
      channelName,
      STORY_COMMONS_CHANNEL_NAME,
      `unexpected commons channel name: ${channelName}`
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
  'the release is not visible as the earned head of the commons channel',
  { timeout: 30_000 },
  function () {
    const result = runDriver(RELEASE_CEREMONY_SCRIPT, ['status', CHANNEL_ID], 20_000);
    const report = extractJson<StatusReport>(result.stdout);
    const earnedRows = report.peers.filter(row => row.tier === 'earned');
    assert.equal(
      earnedRows.length,
      0,
      `release already reads as earned (commons) on: ${JSON.stringify(earnedRows)} — a staging publish ` +
        'must never be visible as the earned head before promotion.'
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
      STORY_COMMONS_CHANNEL_NAME,
      `unexpected commons channel name: ${channelName}`
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
      STORY_COMMONS_CHANNEL_NAME,
      `unexpected commons channel name: ${channelName}`
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
  "james's staging attestation for the release on channel {string} is recorded",
  { timeout: 400_000 },
  async function (this: E2EWorld, channelName: string) {
    assert.equal(
      channelName,
      STORY_COMMONS_CHANNEL_NAME,
      `unexpected commons channel name: ${channelName}`
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
  // 400_000 (was 260_000): the 2026-09-02 strengthened check adds a bounded
  // `pollLamadWasmHashMatch` per peer on top of the existing `pollAdoption`
  // poll — more real work per peer, so more ceiling, even though the
  // success-path duration is typically far below either ceiling.
  { timeout: 400_000 },
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
// Station 6 — revert by re-election: the household finds it wanting.
// ---------------------------------------------------------------------------

Given(
  'the household has converged on the promoted release and now judges it a regression',
  { timeout: 400_000 },
  async function (this: E2EWorld) {
    // The judgment ("wanting") is the ceremony's own act — the revert declare
    // itself, in the When step below. This Given's only real precondition is
    // the convergence revert acts on.
    await ensureFleetConverged(this);
  }
);

When(
  'matthew runs the revert ceremony, re-declaring the prior release the earned head of channel {string}',
  // 600_000 (was 400_000): `ensureReverted` now deliberately waits out the
  // release-adoption controller's installed-reality cache TTL
  // (`INSTALLED_REALITY_TTL_SECS` = 300s + a 15s buffer) before packaging
  // the revert — see that constant's doc — on top of the packaging, the
  // revert driver call, and the post-revert convergence poll this step
  // already budgeted for.
  { timeout: 600_000 },
  async function (this: E2EWorld, channelName: string) {
    assert.equal(
      channelName,
      STORY_COMMONS_CHANNEL_NAME,
      `unexpected commons channel name: ${channelName}`
    );
    await ensureReverted(this);
  }
);

Then(
  "matthew's, jessica's, and james's runtimes all return to the prior coordinator behavior, converging backward through the identical loop they used to converge forward",
  function () {
    const c = ceremony();
    for (const peer of PEER_NAMES) {
      const result = c.postRevertRows[peer];
      assert.ok(result, `no post-revert row captured for ${peer}`);
      assert.equal(
        result.row.appliedRelease?.cid,
        c.priorReleaseCid,
        `${peer} has not converged back to the prior release: ${JSON.stringify(result.row)}`
      );
      assert.ok(
        result.pidAfter.length > 0,
        `${peer}'s conductor PID could not be read after the revert`
      );
      assert.equal(
        result.pidAfter,
        result.pidBefore,
        `${peer}'s conductor PID changed on revert (${result.pidBefore} -> ${result.pidAfter}) — ` +
          'revert must converge through the SAME hot-swap loop as promotion, never a restart'
      );
    }
  }
);

Then(
  'nothing outside the ceremony itself was needed to get there — no operator flag, no re-key, no DHT reset',
  { timeout: 60_000 },
  async function (this: E2EWorld) {
    const c = ceremony();
    assert.ok(c.revertResult, 'no revert result captured — run the revert step first');
    assert.ok(
      c.revertResult.secondPeerVerification,
      'the revert result carries no secondPeerVerification — the earned declaration was never checked from a second peer'
    );
    assert.ok(
      c.setPeerModeCallsAfterPromotion !== undefined,
      'no post-promotion runtime-config baseline recorded — run station 5 first'
    );
    assert.equal(
      setPeerModeCallCount,
      c.setPeerModeCallsAfterPromotion,
      'a runtime-config mode switch happened during/after the revert — revert must converge through ' +
        'the controller sweep alone, no operator lever'
    );
    // No restart (already proven above) means no re-key. The DNA-lineage
    // validation identity is the other half of "nothing outside the
    // ceremony" — a coordinator-only round trip must never move it, on any
    // peer, checked against the pre-fix baseline station 3 captured.
    assert.ok(c.jamesBefore, 'no pre-fix passport baseline captured — run station 3 first');
    for (const peer of PEER_NAMES) {
      const roles = await readRoles(this, peer);
      for (const baselineRole of c.jamesBefore.roles) {
        const role = roles.find(r => r.role === baselineRole.role);
        assert.ok(
          role,
          `role "${baselineRole.role}" missing from ${peer}'s passport after the revert`
        );
        assert.equal(
          role.dnaHash,
          baselineRole.dnaHash,
          `${peer}'s ${baselineRole.role} dnaHash changed across the round trip — a coordinator-only ` +
            'revert must never move the DNA lineage'
        );
      }
    }
  }
);

// ---------------------------------------------------------------------------
// Station 7 — throughout: james's personal channel rides alongside,
// compatible and never forced to converge.
// ---------------------------------------------------------------------------

Given(
  'matthew has run the ceremony that staged, promoted, and reverted a release on the commons channel',
  // 600_000: matches station 6's own "When" — this is `ensureReverted`'s
  // first call if station 6's scenario did not already run it.
  { timeout: 600_000 },
  async function (this: E2EWorld) {
    await ensureReverted(this);
  }
);

Given(
  "james's runtime reported on both of its channels at each of those three moments, as they happened",
  { timeout: 600_000 },
  async function (this: E2EWorld) {
    // Every real recording already happened INSIDE `ensureStagingConvergedAll`
    // / `ensureFleetConverged` / `ensureReverted`, at the moment each station
    // actually converged — never re-derived here after the fact, because by
    // now the household has already moved past staging/earned into the
    // reverted state and a fresh live read could no longer see them. This
    // step asserts the three recordings exist; it takes none itself.
    await ensureReverted(this);
    const c = ceremony();
    for (const station of ['staging', 'promotion', 'revert'] as const) {
      assert.ok(
        c.personalRowsByStation[station],
        `james's personal-channel report for station "${station}" was never recorded — ` +
          'the household ceremony must run stations 1, 5, and 6 (in order) first'
      );
    }
  }
);

When(
  /^the report james's runtime gave after (staging|promotion|the revert) is read back$/,
  function (this: E2EWorld, when: string) {
    const c = ceremony();
    const station: PersonalStation =
      when === 'the revert' ? 'revert' : (when as 'staging' | 'promotion');
    assert.ok(
      c.personalRowsByStation[station],
      `no recorded report for station "${station}" — run the household ceremony first`
    );
    c.lastPersonalStation = station;
  }
);

function assertPersonalDiverging(): void {
  const c = ceremony();
  const station = c.lastPersonalStation;
  assert.ok(
    station,
    'no station recorded for the personal-channel question — run the When step first'
  );
  const rows = c.personalRowsByStation[station];
  assert.ok(rows?.personal, `no personal-channel row captured for james at station "${station}"`);
  // The expected cid is PER STATION: "promotion" and "revert" are rebased by
  // `republishPersonalVariant` once james's own reality moved past the
  // Background's baseline-bound envelope — see that function's doc. Falls
  // back to the latest `personalReleaseCid` for "staging" (never rebased)
  // or if a station's republish somehow never ran.
  const expectedCid = c.personalReleaseCidByStation[station] ?? c.personalReleaseCid;
  assert.ok(expectedCid, `no personal-channel release cid recorded for station "${station}" yet`);
  assert.equal(
    rows.personal.resolvedHead?.cid,
    expectedCid,
    `james's personal-channel resolved head at station "${station}" is ` +
      `${JSON.stringify(rows.personal.resolvedHead)}, not the expected compatible variant ` +
      `${expectedCid} (personalReleaseCidByStation=${JSON.stringify(c.personalReleaseCidByStation)}, ` +
      `latest personalReleaseCid=${String(c.personalReleaseCid)})`
  );
  assert.notEqual(
    rows.personal.resolvedHead?.cid,
    rows.commons?.resolvedHead?.cid,
    "james's personal channel resolved the SAME head as commons — it should diverge"
  );
  assert.notEqual(
    rows.personal.verdict?.state,
    'refused',
    `james's personal-channel verdict is a refusal — outside the compatibility envelope: ${JSON.stringify(rows.personal.verdict)}`
  );
}

Then(
  'his personal channel was diverging from commons, inside the same compatibility envelope',
  assertPersonalDiverging
);

Then(
  'his personal channel was still diverging from commons, inside the same compatibility envelope',
  assertPersonalDiverging
);

Then(
  "james's runtime was converged on commons at that moment, exactly like matthew's and jessica's",
  function () {
    const c = ceremony();
    const station = c.lastPersonalStation;
    assert.ok(station, 'no station recorded — run the When step first');
    const jamesCommons = c.personalRowsByStation[station]?.commons;
    assert.ok(jamesCommons, `no commons-channel row captured for james at station "${station}"`);

    if (station === 'staging') {
      for (const peer of PEER_NAMES) {
        const row = c.stagingConvergedRows[peer];
        assert.ok(row, `no staging row for ${peer}`);
        assert.equal(
          row.resolvedHead?.cid,
          jamesCommons.resolvedHead?.cid,
          `${peer} and james diverge on the commons staged head`
        );
      }
    } else if (station === 'promotion') {
      for (const peer of PEER_NAMES) {
        const result = c.fleetRows[peer];
        assert.ok(result, `no fleet row for ${peer}`);
        assert.equal(
          result.row.appliedRelease?.cid,
          jamesCommons.appliedRelease?.cid,
          `${peer} and james diverge on the applied commons release`
        );
      }
    } else {
      for (const peer of PEER_NAMES) {
        const result = c.postRevertRows[peer];
        assert.ok(result, `no post-revert row for ${peer}`);
        assert.equal(
          result.row.appliedRelease?.cid,
          jamesCommons.appliedRelease?.cid,
          `${peer} and james diverge on the reverted commons release`
        );
      }
    }
  }
);

Then(
  "nobody promotes james's channel to commons and nobody forces james off it",
  { timeout: 30_000 },
  function () {
    const c = ceremony();
    assert.ok(c.personalChannelFollowed, 'personal channel was never established');
    const result = runDriver(RELEASE_CEREMONY_SCRIPT, ['status', PERSONAL_CHANNEL_ID], 20_000);
    const report = extractJson<StatusReport>(result.stdout);
    const earnedRows = report.peers.filter(row => row.tier === 'earned');
    assert.equal(
      earnedRows.length,
      0,
      `personal channel already reads earned on: ${JSON.stringify(earnedRows)}`
    );
    const channelsValue = readChannelsValue('james');
    assert.ok(
      channelsValue
        .split(',')
        .some(entry => entry.trim().split('=')[0].trim() === PERSONAL_CHANNEL_ID),
      `james's runtime-config no longer lists the personal channel: ${channelsValue}`
    );
  }
);

// ---------------------------------------------------------------------------
// Station 8 — the observed proof, read back honestly, not asserted from intent.
// ---------------------------------------------------------------------------

Given(
  "james's personal channel ran alongside that ceremony the entire time",
  { timeout: 600_000 },
  async function (this: E2EWorld) {
    // Recorded already, inside `ensureStagingConvergedAll` / `ensureFleetConverged`
    // / `ensureReverted` — the personal channel was published from Background,
    // before Station 1 even ran, so it rode alongside every station.
    await ensurePersonalVariantPublished(this);
    await ensureReverted(this);
    const c = ceremony();
    assert.ok(
      c.personalRowsByStation.staging &&
        c.personalRowsByStation.promotion &&
        c.personalRowsByStation.revert,
      "james's personal channel was not recorded at every station — the ceremony must run in order"
    );
  }
);

When("an operator reads the household's observed version matrix", function () {
  const c = ceremony();
  c.observedMatrix = buildObservedMatrix();
  // A receipt of the matrix, in the cucumber report — not the assertion itself.
  // eslint-disable-next-line no-console
  console.log(`\nObserved version matrix for channel ${CHANNEL_ID}:`);
  for (const row of c.observedMatrix) {
    // eslint-disable-next-line no-console
    console.log(
      `  ${row.peer.padEnd(9)} ${row.station.padEnd(9)} tier=${String(row.tier).padEnd(8)} ` +
        `cid=${String(row.releaseCid).slice(0, 24)} readAt=${String(row.readAt)} route=${row.route}`
    );
  }
});

Then(
  "the matrix shows matthew's, jessica's, and james's runtimes moving staging, then earned, then back, in that order",
  function () {
    const c = ceremony();
    assert.ok(c.observedMatrix, 'no observed matrix — run the When step first');
    const matrix: MatrixRow[] = c.observedMatrix;
    const order: MatrixRow['station'][] = ['staging', 'earned', 'reverted'];
    for (const peer of PEER_NAMES) {
      const peerRows: MatrixRow[] = matrix.filter(
        row => row.peer === peer && row.station !== 'personal'
      );
      const stationsSeen: MatrixRow['station'][] = order.filter(station =>
        peerRows.some(row => row.station === station)
      );
      assert.deepEqual(
        stationsSeen,
        order,
        `${peer}'s matrix rows do not show staging -> earned -> reverted, in order: ${JSON.stringify(peerRows)}`
      );
      const staging = peerRows.find(row => row.station === 'staging');
      const earned = peerRows.find(row => row.station === 'earned');
      const reverted = peerRows.find(row => row.station === 'reverted');
      assert.equal(
        staging?.releaseCid,
        c.releaseCid,
        `${peer}'s staging cell is not the fix release`
      );
      assert.equal(
        earned?.releaseCid,
        c.releaseCid,
        `${peer}'s earned cell is not the fix release`
      );
      assert.equal(
        reverted?.releaseCid,
        c.priorReleaseCid,
        `${peer}'s reverted cell is not the prior release`
      );
      assert.ok(
        staging &&
          earned &&
          reverted &&
          (staging.readAt ?? 0) <= (earned.readAt ?? 0) &&
          (earned.readAt ?? 0) <= (reverted.readAt ?? 0),
        `${peer}'s matrix timestamps are not monotonically ordered staging -> earned -> reverted`
      );
    }
  }
);

Then("the matrix shows james's personal channel diverging compatibly the whole time", function () {
  const c = ceremony();
  assert.ok(c.observedMatrix, 'no observed matrix — run the When step first');
  const personalRow = c.observedMatrix.find(
    row => row.peer === 'james' && row.station === 'personal'
  );
  assert.ok(personalRow, 'no personal-channel row in the matrix for james');
  assert.equal(
    personalRow.releaseCid,
    c.personalReleaseCid,
    "james's personal-channel matrix cell is not the compatible variant"
  );
  const jamesReverted = c.observedMatrix.find(
    row => row.peer === 'james' && row.station === 'reverted'
  );
  assert.ok(jamesReverted, 'no reverted commons row for james in the matrix');
  assert.notEqual(
    personalRow.releaseCid,
    jamesReverted.releaseCid,
    "james's personal channel matches commons in the matrix — it should still diverge"
  );
});

Then(
  "every row in the matrix is read from what each runtime itself reports, never asserted from the ceremony's own intent",
  function () {
    const c = ceremony();
    assert.ok(
      c.observedMatrix && c.observedMatrix.length > 0,
      'no observed matrix — run the When step first'
    );
    for (const row of c.observedMatrix) {
      assert.equal(
        row.route,
        ADOPTION_PATH,
        `matrix row for ${row.peer}/${row.station} carries no source route`
      );
      assert.ok(
        typeof row.readAt === 'number' && row.readAt > 0,
        `matrix row for ${row.peer}/${row.station} carries no readAt — cannot have been read from the runtime`
      );
    }
  }
);

// ---------------------------------------------------------------------------
// The two constitutional scenarios (jessica's no-opt-out; DNA-lineage
// refusal) — pending. Untouched by stations 1-8's setup; never faked.
// ---------------------------------------------------------------------------

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
  "jessica can still read the release's own explanation of what changed and why, and can raise it to the steward if her stored content, her recorded agreements, or her identity were mishandled by it — and the revert ceremony is the household's remedy",
  function () {
    return 'pending';
  }
);

Given(
  'a release manifest was built against a different validation-rule identity than the one the household actually runs',
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
