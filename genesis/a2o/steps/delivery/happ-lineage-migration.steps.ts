/**
 * Step definitions for features/delivery/happ-lineage-migration.feature
 * (@concern:happ-lineage-migration — habit
 * elohim/holochain/.epr-meta/happ-lineage-migration.habit.md; design spec
 * genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md
 * §4, §8, §11).
 *
 * Holochain Evolution Epic, Task 11 PART 1: the skeleton for all ten
 * Stations (generated below, `return 'pending'`), plus REAL implementations
 * for the Background and Stations 1-2 only. No live mesh writes happened
 * while authoring this file — every assertion below is written to be correct
 * against a running household mesh, but this dispatch's own verification was
 * limited to `tsc --noEmit`, `eslint`, and a `--dry-run` step-binding check
 * (the storage binary on the mesh predates the `verify_path`/`happ-lineage`
 * changes this station exercises, and another session held the mesh lease).
 *
 * ## Rails composed (mirrors `runtime-upgrade-propagation.steps.ts`'s own
 * composition discipline — shell out to the ceremony drivers, never
 * reimplement them)
 *
 *   T1  `lineage-candidate.ts`      — Task 10's `mintLineageCandidate` /
 *                                     `lineageReleaseWithoutParent`, which
 *                                     shell to `scripts/epr-release-package.ts`
 *   T2  `scripts/release-ceremony.ts` — channel create / publish / promote,
 *                                     via `spawnSync('pnpm', ['exec','tsx',…])`
 *                                     exactly as the rung-5 steps' `runDriver`
 *   T3  `GET  <peer>/admin/adoption`  — each peer's OWN controller verdict
 *   T4  `runtime-config.toml` + `POST <peer>/admin/runtime-config/reload`
 *                                     — follow-set registration (run-owned,
 *                                     byte-restored in `AfterAll`, same
 *                                     discipline as the rung-5 steps)
 *   T5  `lineage-commitments.ts`     — Task 10's `buildMigratesLineagePayload`
 *                                     / `notarizeMigration`, submitted over a
 *                                     conductor rail this file connects
 *                                     directly (see "Conductor rail" below —
 *                                     `release-ceremony.ts` only knows the
 *                                     `elohim`/`lamad`/`content_store` triple,
 *                                     so it cannot be reused for `mishpat` or
 *                                     `node_registry` zome calls without
 *                                     editing that shared driver, which is
 *                                     out of this dispatch's one-file scope).
 *
 * ## The verify_path caveat — READ BEFORE "fixing" Station 2's second half
 *
 * `elohim/elohim-storage/src/services/release_adoption/verify.rs`'s
 * `VerifyInput.path: Answer<PathEvidence>` field carries its own doc: "caller-
 * fetched evidence … `Answer::Absent` for every existing caller today (the
 * fetch site is a later task's)". `verify_path` reads `Answer::Absent` as
 * "not notarized YET" (C4: absence is not refusal) and refuses
 * `path_not_notarized` — which means EVERY `happ-lineage` release refuses
 * with `path_not_notarized` right now, REGARDLESS of whether a real
 * `migrates-lineage` commitment exists. That is exactly what makes Station
 * 1's two Thens and Station 2's FIRST half real, verifiable assertions today
 * (see the per-Then comments below for exactly which wire fact each one
 * reads), and exactly why Station 2's SECOND half (the "adoptable" Then) is
 * expected to stay red until BOTH (a) this fetch is wired (a later task) and
 * (b) mishpat's self-signing extern lands (Task 11 part 2 — see
 * `lineage-commitments.ts`'s own "Signing path" module doc section).
 *
 * ## Conductor rail (mishpat + node_registry zome calls)
 *
 * `release-ceremony.ts`'s `conductor()` helper is hardcoded to the `elohim`
 * app's `lamad` role / `content_store` zome. This file needs the SAME admin
 * connect → resolve provisioned cell → authorize signing → app connect →
 * `callZome` rail, but parameterized by role/zome (`mishpat` for Task 10's
 * `notarizeMigration`, `node_registry` for the Background's fixture
 * seeding) — `connectRoleConductor` below copies that shape rather than
 * editing the shared driver (this dispatch's commit is path-limited to this
 * one file). Ports follow the same local household mesh convention
 * `release-ceremony.ts` documents: admin_port(i) = 4444 + 10i,
 * app_port(i) = 4445 + 10i (i = 0 matthew, 1 jessica, 2 james).
 *
 * ## Placeholder values — documented, never silently trusted
 *
 * - `PLACEHOLDER_PATH_COMMITMENT_CID`: `epr-release-package.ts` REQUIRES
 *   `--path-commitment <cid>` whenever `--artifact-class happ-lineage` is
 *   given (it refuses to package without one), but only checks PRESENCE —
 *   never that the cid resolves to a real notarized commitment (that
 *   resolution is `verify_path`'s job, at read time, on a live peer; see the
 *   caveat above). Station 1 needs a `happ-lineage` release to exist before
 *   Station 2 ever notarizes anything, so this file packages Station 1's
 *   release with a placeholder commitment cid — never checked against
 *   anything on the wire before Station 2 mints a REAL one (which today
 *   never actually lands either — see the caveat above again).
 * - `FIXTURE_CONSTITUTION_ROOT` / `FIXTURE_ROSTER_CID`: Station 2's
 *   `migrates-lineage` payload needs SOME values here, but the notarization
 *   is expected to be refused before either field is ever checked (empty
 *   `signatures` fails `validate_lineage_signatures`'s "non-empty array"
 *   requirement first — see `attemptMigrationNotarization`'s own comment).
 *
 * ## Run-scoped channel, one channel only (Part 1's scope)
 *
 * `CHANNEL_ID` mints fresh per run from a run stamp, exactly like the rung-5
 * steps' `CHANNEL_ID` — a repeat run never collides with a channel a prior
 * run left mid-flight. The story names the channel
 * "runtime:lineage:node_registry:commons"; this file substitutes a
 * run-scoped id for the SAME behavioural role (the Background's own Given
 * asserts the STORY name against `STORY_CHANNEL_NAME`, then acts on
 * `CHANNEL_ID` internally — same substitution rung-5's own Background does
 * for `STORY_COMMONS_CHANNEL_NAME`). Part 1 only ever needs the channel in
 * `observe` mode (no station here applies or promotes past a bare
 * `channel create` + `publish` + `promote`), so the follow-set machinery
 * below is a single channel->mode write, not the multi-channel map
 * `runtime-upgrade-propagation.steps.ts` needs for its canary/apply/personal
 * channel matrix — Stations 3+ (a later part) will likely need to grow this
 * the same way, at which point copy that file's `runOwnedChannels` shape
 * rather than re-deriving it.
 */

/* eslint-disable sonarjs/no-os-command-from-path, sonarjs/publicly-writable-directories --
   this file deliberately shells out to `pnpm exec tsx` for the release-ceremony driver (the
   composition this task requires) and reads/writes the local household mesh's own /tmp work
   dir (runtime-config.toml), same posture as steps/delivery/runtime-upgrade-propagation.steps.ts
   and steps/conductor-spin.steps.ts. */

import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

import { Given, When, Then, Before, AfterAll } from '@cucumber/cucumber';

import { AdminWebsocket, AppWebsocket, CellType, encodeHashToBase64 } from '@holochain/client';
import { request } from 'undici';

import { getRaw, postRaw } from '../../src/framework/dataplane/surfaces.js';

import {
  mintLineageCandidate,
  lineageReleaseWithoutParent,
  NODE_REGISTRY_ROLE,
  resolveV2HappPath,
  computeDnaHash,
  NODE_REGISTRY_V2_DNA,
} from './lineage-candidate.js';
import { buildMigratesLineagePayload, notarizeMigration } from './lineage-commitments.js';

import type { MigratesLineagePayload } from './lineage-commitments.js';
import type { E2EWorld } from '../../src/framework/world.js';
import type { AppInfo, CellId } from '@holochain/client';

// ---------------------------------------------------------------------------
// Paths and run-scoped constants
// ---------------------------------------------------------------------------

/** genesis/a2o/steps/delivery -> genesis/a2o (2 levels up) — the driver's own cwd. */
const A2O_ROOT = fileURLToPath(new URL('../../', import.meta.url));

/** Distinct from rung-5's `A2O_RELEASE_RUN_STAMP` — a separate channel family, minted
 * independently so the two features' fixtures never share (or fight over) a run stamp. */
const RUN_STAMP =
  process.env['A2O_LINEAGE_RUN_STAMP'] ?? new Date().toISOString().replace(/\D/g, '').slice(0, 14);

/** The story's own channel name (Background's `{string}` literal) — never acted on directly;
 * see module doc "Run-scoped channel, one channel only". */
const STORY_CHANNEL_NAME = 'runtime:lineage:node_registry:commons';
const CHANNEL_ID = `runtime:lineage:node_registry:a2o-${RUN_STAMP}`;

const REPORT_DIR = path.join(
  A2O_ROOT,
  'reports',
  'release-ceremony',
  new Date().toISOString().slice(0, 10)
);
const MANIFEST_PATH = path.join(REPORT_DIR, `a2o-happ-lineage-${RUN_STAMP}.json`);
const NEGATIVE_MANIFEST_PATH = path.join(
  REPORT_DIR,
  `a2o-happ-lineage-${RUN_STAMP}-no-parent.json`
);

const MESH_ROOT = process.env['E2E_MESH_ROOT'] ?? '/tmp/elohim-local-mesh';

type PeerName = 'matthew' | 'jessica' | 'james';
const PEER_NAMES: readonly PeerName[] = ['matthew', 'jessica', 'james'];
const CANARY_PEER: PeerName = 'james';

const ADOPTION_PATH = '/admin/adoption';
const RUNTIME_CONFIG_RELOAD_PATH = '/admin/runtime-config/reload';
const VERSION_PATH = '/version';
/** Task 10 part 2 — the fixture's baseline-convergence vehicle (see the
 * `Before`/`AfterAll` hooks below). Resets `LineageRoles` to v1 base on the
 * target peer and disables + uninstalls any lineage side app. */
const LINEAGE_RESET_PATH = '/admin/lineage/reset';

/** `epr-release-package.ts`'s own soak/threshold discipline flags — a call site that omits
 * either is refused at exit 64 (see that script's own module doc, quoted in
 * runtime-upgrade-propagation.steps.ts). One attester, a short soak — the same household
 * discipline numbers rung-5 uses (`SOAK_SECS` / `ATTESTATION_THRESHOLD` there). */
const SOAK_SECS = 30;
const ATTESTATION_THRESHOLD = 1;

/** T2 ceremony driver, relative to `A2O_ROOT` (`runDriver`'s `cwd`). */
const RELEASE_CEREMONY_SCRIPT = 'scripts/release-ceremony.ts';

/** See module doc "Placeholder values". Shaped like a mishpat commitment cid
 * (`uhCEk…`, a base64 EntryHash — `epr-release-package.ts`'s own help text: "the migrates-lineage
 * commitment (entry hash, uhCEk…) that notarizes this path") without being one — the packager
 * only checks presence, and `verify_path`'s own evidence-fetch never reads it today (module doc
 * "The verify_path caveat"), so no live check ever compares this string to a real commitment. */
const PLACEHOLDER_PATH_COMMITMENT_CID = 'uhCEka2oHappLineageStation1PlaceholderNotYetNotarized';

/** See module doc "Placeholder values" — never actually checked because the notarization
 * attempt below is refused earlier, on the empty `signatures` array. */
const FIXTURE_CONSTITUTION_ROOT = 'a2o-fixture-constitution-root';
const FIXTURE_ROSTER_CID = 'a2o-fixture-bootstrap-steward-roster';

/** How long a single peer's `/admin/adoption` poll waits for its predicate, and how often it
 * re-reads while waiting. Mirrors the "unreachable ≠ absent, keep retrying" discipline
 * `runtime-upgrade-propagation.steps.ts`'s own `pollAdoption` documents. */
const RECONCILE_POLL_TIMEOUT_MS = 60_000;
const RECONCILE_POLL_INTERVAL_MS = 10_000;

/** Station 2's second-half Then is KNOWN RED today (module doc "The verify_path caveat") — a
 * distinct, shorter budget so a future live run of this file does not burn extra wall-clock on
 * an assertion that cannot pass yet. */
const ADOPTABLE_RED_POLL_TIMEOUT_MS = 120_000;

// ---------------------------------------------------------------------------
// Conductor rail — role/zome-parameterized, see module doc "Conductor rail"
// ---------------------------------------------------------------------------

const APP_ID = 'elohim';
const NODE_REGISTRY_ZOME = 'node_registry_coordinator';
const MISHPAT_ROLE = 'mishpat';
const MISHPAT_ZOME = 'mishpat';

const PEER_CONDUCTOR_PORTS: Record<PeerName, { admin: number; app: number }> = {
  matthew: { admin: 4444, app: 4445 },
  jessica: { admin: 4454, app: 4455 },
  james: { admin: 4464, app: 4465 },
};

/** Same default `release-ceremony.ts` uses for its own `conductor()` connects. */
const CONDUCTOR_CONNECT_TIMEOUT_MS = 5_000;

/**
 * Copied byte-for-byte (behaviourally) from `release-ceremony.ts`'s own module-local
 * `withTimeout` — a silent-but-open port would otherwise hang past the cucumber step timeout and
 * leave a dangling handle, exactly the failure mode that helper exists to close. Every connect on
 * `connectRoleConductor`'s rail goes through this, mirroring which calls `conductor()` wraps
 * (`AdminWebsocket.connect` / `AppWebsocket.connect` only — `listApps`,
 * `authorizeSigningCredentials` and `issueAppAuthenticationToken` are NOT wrapped there either).
 */
function withTimeout<T>(promise: Promise<T>, timeoutMs: number, label: string): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new Error(`timeout after ${timeoutMs}ms: ${label}`)),
      timeoutMs
    );
    promise.then(
      value => {
        clearTimeout(timer);
        resolve(value);
      },
      error => {
        clearTimeout(timer);
        reject(error as Error);
      }
    );
  });
}

interface RoleConductorRail {
  call: (fnName: string, payload: unknown) => Promise<unknown>;
  /** base64 AgentPubKey this rail authors/calls as — the resolved cell's own agent key. */
  agent: string;
  close: () => Promise<void>;
}

// `CellId | undefined` from a loop is the correct, narrow return type; the next line's sonarjs
// check false-positives on it the same way it does on runtime-upgrade-propagation.steps.ts's own
// `findChannelRow` (a plain `.find()`).
// eslint-disable-next-line sonarjs/function-return-type
function findProvisionedCellId(app: AppInfo, role: string): CellId | undefined {
  for (const info of app.cell_info[role] ?? []) {
    if (info.type === CellType.Provisioned) return info.value.cell_id;
  }
  return undefined;
}

/**
 * Copies `release-ceremony.ts`'s own `conductor()` connect shape (admin connect → resolve
 * provisioned cell → authorize signing credentials → app connect → bound `call`), generalized by
 * role/zome instead of that script's hardcoded `lamad`/`content_store`. See module doc
 * "Conductor rail" for why this is not a shared import.
 */
async function connectRoleConductor(
  peer: PeerName,
  role: string,
  zomeName: string
): Promise<RoleConductorRail> {
  const ports = PEER_CONDUCTOR_PORTS[peer];
  const admin = await withTimeout(
    AdminWebsocket.connect({
      url: new URL(`ws://127.0.0.1:${ports.admin}`),
      wsClientOptions: { origin: APP_ID },
    }),
    CONDUCTOR_CONNECT_TIMEOUT_MS,
    `admin connect ${peer}:${ports.admin}`
  );
  const apps = await admin.listApps({});
  const app = apps.find(a => a.installed_app_id === APP_ID) ?? apps[0];
  if (!app) {
    await admin.client.close();
    throw new Error(`${peer}: no installed app on adminPort=${ports.admin}`);
  }
  const cellId = findProvisionedCellId(app, role);
  if (!cellId) {
    await admin.client.close();
    throw new Error(`${peer}: no '${role}' cell provisioned on adminPort=${ports.admin}`);
  }
  await admin.authorizeSigningCredentials(cellId);
  const authToken = await admin.issueAppAuthenticationToken({
    installed_app_id: app.installed_app_id,
  });
  const appWs = await withTimeout(
    AppWebsocket.connect({
      url: new URL(`ws://127.0.0.1:${ports.app}`),
      token: authToken.token,
      wsClientOptions: { origin: APP_ID },
    }),
    CONDUCTOR_CONNECT_TIMEOUT_MS,
    `app connect ${peer}:${ports.app}`
  );
  const call = async (fnName: string, payload: unknown): Promise<unknown> =>
    appWs.callZome({ cell_id: cellId, zome_name: zomeName, fn_name: fnName, payload });
  const close = async (): Promise<void> => {
    await appWs.client.close();
    await admin.client.close();
  };
  return { call, agent: encodeHashToBase64(cellId[1]), close };
}

// ---------------------------------------------------------------------------
// Background fixture: node-registry v1 records
// ---------------------------------------------------------------------------

/** Every field `NodeRegistration` (node_registry_integrity) declares — snake_case, crossing
 * directly into the zome call with no camelCase layer (this is a DNA entry, not an HTTP view).
 * The integrity zome's own `validate_create_entry` does not validate this entry type at all yet
 * (grepped 2026-09-04 — falls through to `_ => Ok(Valid)`), so a fixture `signature` placeholder
 * is accepted exactly as a real one would be. */
interface NodeRegistrationFixture {
  node_id: string;
  agent_pub_key: string;
  display_name: string;
  cpu_cores: number;
  memory_gb: number;
  storage_tb: number;
  bandwidth_mbps: number;
  region: string;
  latitude: number | null;
  longitude: number | null;
  zomes_hosted: string[];
  steward_tier: string;
  custodian_opt_in: boolean;
  max_custody_gb: number | null;
  max_bandwidth_mbps: number | null;
  max_cpu_percent: number | null;
  uptime_percent: number;
  last_heartbeat: string;
  registered_at: string;
  updated_at: string;
  claim_status: string;
  context_epr_id: string | null;
  signature: string;
}

function fixtureNodeRegistration(peer: PeerName, agentPubKeyB64: string): NodeRegistrationFixture {
  const now = new Date().toISOString();
  return {
    node_id: `a2o-lineage-fixture-${peer}-${RUN_STAMP}`,
    agent_pub_key: agentPubKeyB64,
    display_name: `${peer} (a2o happ-lineage fixture)`,
    cpu_cores: 4,
    memory_gb: 8,
    storage_tb: 1,
    bandwidth_mbps: 100,
    region: 'a2o-fixture',
    latitude: null,
    longitude: null,
    zomes_hosted: [NODE_REGISTRY_ROLE],
    steward_tier: 'household',
    custodian_opt_in: true,
    max_custody_gb: null,
    max_bandwidth_mbps: null,
    max_cpu_percent: null,
    uptime_percent: 1,
    last_heartbeat: now,
    registered_at: now,
    updated_at: now,
    claim_status: 'unclaimed',
    context_epr_id: null,
    signature: `a2o-fixture-unsigned-${peer}-${RUN_STAMP}`,
  };
}

async function seedNodeRegistryRecord(peer: PeerName): Promise<void> {
  const rail = await connectRoleConductor(peer, NODE_REGISTRY_ROLE, NODE_REGISTRY_ZOME);
  try {
    await rail.call('register_node', fixtureNodeRegistration(peer, rail.agent));
  } finally {
    await rail.close();
  }
}

/**
 * Best-effort: authors one fresh v1 node-registry record per peer (idempotent enough — each
 * run's `node_id` is unique by `RUN_STAMP`, so re-running never collides with a prior run's
 * fixture). No query extern exists to check "does a record already exist" (`get_my_nodes` is
 * retired and always returns `[]`; no HTTP route projects node-registry records yet — see
 * `elohim/elohim-storage/src/node_registry_api.rs`, which wires shard assignment only), so this
 * always authors rather than first checking. On any failure (conductor unreachable, port not
 * where expected, …) this logs loudly and returns `'pending'` rather than silently treating "at
 * least one record" as already true — the Background skips the whole scenario, which is the
 * loud, honest failure mode for a fixture precondition that could not be established.
 */
async function ensureNodeRegistryRecords(): Promise<'ok' | 'pending'> {
  for (const peer of PEER_NAMES) {
    try {
      await seedNodeRegistryRecord(peer);
    } catch (error) {
      console.error(
        `[happ-lineage-migration] could not author a v1 node-registry record for ${peer}: ` +
          `${String(error)} — skipping loudly rather than assuming a record already exists.`
      );
      return 'pending';
    }
  }
  return 'ok';
}

// ---------------------------------------------------------------------------
// HTTP + wire types
// ---------------------------------------------------------------------------

function directPeerUrl(peer: PeerName): string {
  const envVar = `E2E_STORAGE_${peer.toUpperCase()}`;
  const url = process.env[envVar];
  assert.ok(url, `${envVar} is not set — cannot reach ${peer}`);
  return url.replace(/\/$/, '');
}

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
  refusal?: AdoptionRefusalDetail;
}

interface AdoptionChannelRow {
  channelId: string;
  mode: string;
  resolvedHead: { cid: string; tier: string } | null;
  verdict: AdoptionVerdict | null;
  lastCheckedAt: number | null;
  appliedRelease: { cid: string; at: number; vehicle: string } | null;
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
  passport?: { conductor?: { version: string }; happ?: { roles?: VersionRole[] } };
}

interface PublishResult {
  releaseCid: string;
  tier: string;
}

interface PromoteResult {
  tier: string;
}

async function getAdoptionReport(
  peer: PeerName
): Promise<{ report: AdoptionReport; rawText: string }> {
  const { status, text } = await getRaw(`${directPeerUrl(peer)}${ADOPTION_PATH}`, {
    timeoutMs: 10_000,
  });
  assert.equal(status, 200, `GET ${ADOPTION_PATH} on ${peer} returned ${status}`);
  return { report: JSON.parse(text) as AdoptionReport, rawText: text };
}

// eslint-disable-next-line sonarjs/function-return-type -- see `findProvisionedCellId`'s comment.
function findChannelRow(report: AdoptionReport, channelId: string): AdoptionChannelRow | undefined {
  return report.channels.find(row => row.channelId === channelId);
}

/**
 * Poll one peer's `/admin/adoption` for `CHANNEL_ID` until `predicate` is satisfied. A connect
 * failure or non-200 is NOT "absent" — the poll keeps retrying rather than concluding the row
 * doesn't exist (same "unreachable ≠ absent" rail `runtime-upgrade-propagation.steps.ts`'s own
 * `pollAdoption` documents).
 */
async function pollAdoption(
  peer: PeerName,
  timeoutMs: number,
  predicate: (row: AdoptionChannelRow) => boolean,
  intervalMs = RECONCILE_POLL_INTERVAL_MS
): Promise<{ row: AdoptionChannelRow; rawText: string }> {
  const start = Date.now();
  let lastRow: AdoptionChannelRow | undefined;
  let everReachable = false;
  while (Date.now() - start < timeoutMs) {
    try {
      const { report, rawText } = await getAdoptionReport(peer);
      everReachable = true;
      const row = findChannelRow(report, CHANNEL_ID);
      lastRow = row;
      if (row && predicate(row)) {
        return { row, rawText };
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

/** Runs `pollAdoption` on all three peers CONCURRENTLY — bounds a "every peer must satisfy X"
 * assertion to one `timeoutMs` budget total rather than `timeoutMs` * 3 sequential. */
async function pollAllPeers(
  predicate: (peer: PeerName, row: AdoptionChannelRow) => boolean,
  timeoutMs = RECONCILE_POLL_TIMEOUT_MS
): Promise<Partial<Record<PeerName, { row: AdoptionChannelRow; rawText: string }>>> {
  const entries = await Promise.all(
    PEER_NAMES.map(async peer => {
      const result = await pollAdoption(peer, timeoutMs, row => predicate(peer, row));
      return [peer, result] as const;
    })
  );
  return Object.fromEntries(entries);
}

async function readRoles(peer: PeerName): Promise<VersionRole[]> {
  const { status, text } = await getRaw(`${directPeerUrl(peer)}${VERSION_PATH}`, {
    timeoutMs: 10_000,
  });
  assert.equal(status, 200, `GET ${VERSION_PATH} on ${peer} returned ${status}`);
  const body = JSON.parse(text) as VersionResponse;
  return body.passport?.happ?.roles ?? [];
}

// eslint-disable-next-line sonarjs/function-return-type -- see `findProvisionedCellId`'s comment.
function nodeRegistryRole(roles: VersionRole[]): VersionRole | undefined {
  return roles.find(role => role.role === NODE_REGISTRY_ROLE);
}

/**
 * Maps a wire refusal reason (`RefusalReason`'s `snake_case` labels in
 * `elohim/elohim-storage/src/services/release_adoption/mod.rs`) to the story's own prose form —
 * `dna_lineage_mismatch` -> "lineage mismatch", `path_not_notarized` -> "path not notarized",
 * `quorum_unmet` -> "quorum unmet", `root_mismatch` -> "root mismatch". A single mechanical rule
 * (strip a leading `dna_`, then underscores to spaces) covers every reason this Part needs; a
 * later Part may need to extend it if a Station 8/9 reason doesn't fit the same shape.
 */
function storyReasonLabel(machineReason: string): string {
  return machineReason.replace(/^dna_/, '').replace(/_/g, ' ');
}

// ---------------------------------------------------------------------------
// runtime-config.toml follow-set — one channel, `observe` mode (Part 1's scope; see module doc)
// ---------------------------------------------------------------------------

function runtimeConfigPath(peer: PeerName): string {
  return path.join(MESH_ROOT, peer, 'runtime-config.toml');
}

async function applyChannelMode(peer: PeerName, mode: string): Promise<void> {
  const filePath = runtimeConfigPath(peer);
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, `ELOHIM_RELEASE_CHANNELS = "${CHANNEL_ID}=${mode}"\n`, 'utf8');
  const { status } = await postRaw(`${directPeerUrl(peer)}${RUNTIME_CONFIG_RELOAD_PATH}`);
  assert.ok(
    status >= 200 && status < 300,
    `${peer}'s ${RUNTIME_CONFIG_RELOAD_PATH} returned ${status} while registering ${CHANNEL_ID}`
  );
}

const originalRuntimeConfigBytes: Partial<Record<PeerName, Buffer>> = {};
let runOwnedFollowSetEstablished = false;
let runOwnedFollowSetRestored = false;

/** Captures each peer's TRUE on-disk bytes once, then writes a run-owned file containing only
 * `CHANNEL_ID=observe` — same "the run owns its own follow set, never a read-merge-write" rule
 * `runtime-upgrade-propagation.steps.ts`'s module doc explains (the 2026-09-01 leftover-channel
 * hazard this closes). */
async function ensureRunOwnedFollowSet(): Promise<void> {
  if (runOwnedFollowSetEstablished) return;
  mkdirSync(REPORT_DIR, { recursive: true });
  for (const peer of PEER_NAMES) {
    let originalBytes: Buffer;
    try {
      originalBytes = readFileSync(runtimeConfigPath(peer));
    } catch {
      originalBytes = Buffer.from('', 'utf8');
    }
    originalRuntimeConfigBytes[peer] = originalBytes;
    await applyChannelMode(peer, 'observe');
  }
  runOwnedFollowSetEstablished = true;
}

/**
 * Task 10 part 2 (Holochain Evolution Epic MVP) — the fixture's
 * baseline-convergence vehicle. POSTs `LINEAGE_RESET_PATH` with
 * `{"uninstall": true}` on every peer: `LineageRoles` resets to the v1
 * base for every tracked role and any lineage side app (`"<base
 * app id>@…"`) is disabled + uninstalled. A mesh run must start AND end at
 * the same baseline regardless of what a PRIOR run opened (rung 5's
 * lesson, 8181d60a8) — hence a call both in `Before` and in `AfterAll`.
 *
 * Never throws: a failed reset on one peer is logged with the
 * `[happ-lineage-migration]` prefix and the other peers still get their
 * chance, matching this file's other cleanup-leg posture (see the
 * runtime-config restore loop below).
 */
async function resetLineageBaselineOnAllPeers(): Promise<void> {
  for (const peer of PEER_NAMES) {
    try {
      const { status, text } = await postRaw(`${directPeerUrl(peer)}${LINEAGE_RESET_PATH}`, {
        uninstall: true,
      });
      if (status < 200 || status >= 300) {
        console.error(
          `[happ-lineage-migration] ${peer}'s ${LINEAGE_RESET_PATH} returned ${status}: ${text}`
        );
      }
    } catch (error) {
      console.error(
        `[happ-lineage-migration] ${peer}'s ${LINEAGE_RESET_PATH} failed: ${String(error)}`
      );
    }
  }
}

/** Tag-scoped to this feature's own `@happ-lineage` so no other suite's runtime-config is ever
 * touched — fires before Station 1's own Given/When steps. */
Before({ tags: '@happ-lineage', timeout: 60_000 }, async function (this: E2EWorld) {
  await ensureRunOwnedFollowSet();
  await resetLineageBaselineOnAllPeers();
});

AfterAll({ timeout: 180_000 }, async function () {
  if (Object.keys(originalRuntimeConfigBytes).length === 0 || runOwnedFollowSetRestored) return;
  runOwnedFollowSetRestored = true;
  await resetLineageBaselineOnAllPeers();
  for (const peer of PEER_NAMES) {
    const originalBytes = originalRuntimeConfigBytes[peer];
    if (originalBytes === undefined) continue;
    try {
      writeFileSync(runtimeConfigPath(peer), originalBytes);
      await postRaw(`${directPeerUrl(peer)}${RUNTIME_CONFIG_RELOAD_PATH}`);

      console.error(`[happ-lineage-migration] ${peer}'s runtime-config.toml byte-restored.`);
    } catch (error) {
      console.error(
        `[happ-lineage-migration] restore of ${peer}'s runtime-config.toml failed: ${String(error)}`
      );
    }
  }
});

// ---------------------------------------------------------------------------
// Driver composition (T2 — shell out, never re-implement)
// ---------------------------------------------------------------------------

interface DriverResult {
  status: number;
  stdout: string;
  stderr: string;
}

function runDriver(scriptRelPath: string, args: string[], timeoutMs = 90_000): DriverResult {
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

/** Every driver verb here prints exactly one pretty-printed JSON object as its LAST output —
 * same convention `runtime-upgrade-propagation.steps.ts`'s own `extractJson` reads. */
function extractJson<T>(stdout: string): T {
  const start = stdout.indexOf('{');
  assert.ok(start >= 0, `no JSON object in driver output: ${stdout.slice(0, 400)}`);
  return JSON.parse(stdout.slice(start)) as T;
}

async function putBlobRaw(baseUrl: string, sha256: string, bytes: Buffer): Promise<number> {
  const { statusCode } = await request(`${baseUrl}/blob/sha256-${sha256}`, {
    method: 'PUT',
    headers: {
      'content-type': 'application/octet-stream',
      'x-agent-id': 'did:elohim:a2o-happ-lineage-packager',
    },
    body: bytes,
  });
  return statusCode;
}

// ---------------------------------------------------------------------------
// Cross-scenario ceremony state (module-level — Stations 1-2 are one causal
// chain, same convention as runtime-upgrade-propagation.steps.ts's `ceremony()`)
// ---------------------------------------------------------------------------

interface LineageState {
  v1DnaHash?: string;
  v2DnaHash?: string;
  releaseCid?: string;
  publishTier?: string;
  channelCreated: boolean;
  negativeReleaseCid?: string;
  promotedAt?: number;
  migrationPayload?: MigratesLineagePayload;
  migrationCommitmentCid?: string;
  migrationNotarizeError?: string;
  lastRows: Partial<Record<PeerName, AdoptionChannelRow>>;
  lastRawText: Partial<Record<PeerName, string>>;
  backgroundPids: Partial<Record<PeerName, string>>;
}

let state: LineageState | undefined;

function lineage(): LineageState {
  state ??= {
    channelCreated: false,
    lastRows: {},
    lastRawText: {},
    backgroundPids: {},
  };
  return state;
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

Given(
  'the household mesh is running three peers, each on its own conductor, all built from the same conductor software',
  { timeout: 30_000 },
  async function (this: E2EWorld) {
    const versions = await Promise.all(
      PEER_NAMES.map(async peer => {
        const { status, text } = await getRaw(`${directPeerUrl(peer)}${VERSION_PATH}`, {
          timeoutMs: 10_000,
        });
        assert.equal(status, 200, `GET ${VERSION_PATH} on ${peer} returned ${status}`);
        const body = JSON.parse(text) as VersionResponse;
        const version = body.passport?.conductor?.version;
        assert.ok(version, `${peer}'s /version passport carries no passport.conductor.version`);
        return version;
      })
    );
    const distinct = new Set(versions);
    assert.equal(
      distinct.size,
      1,
      `household peers report DIFFERENT conductor versions (${[...distinct].join(', ')}) — the ` +
        'story requires "all built from the same conductor software"'
    );
  }
);

Given(
  "the household's node-registry role runs rule version v1",
  { timeout: 30_000 },
  async function (this: E2EWorld) {
    const entries = await Promise.all(
      PEER_NAMES.map(async peer => {
        const role = nodeRegistryRole(await readRoles(peer));
        assert.ok(role, `${peer}'s /version passport carries no '${NODE_REGISTRY_ROLE}' role`);
        assert.ok(role.dnaHash, `${peer}'s node_registry role reports no dnaHash`);
        return [peer, role.dnaHash] as const;
      })
    );
    const hashes = Object.fromEntries(entries) as Record<PeerName, string>;
    const distinct = new Set(Object.values(hashes));
    assert.equal(
      distinct.size,
      1,
      `peers disagree on the node-registry v1 dnaHash: ${JSON.stringify(hashes)}`
    );
    lineage().v1DnaHash = hashes.matthew;
  }
);

Given(
  "each peer's runtime follows release channel {string}",
  { timeout: 60_000 },
  async function (this: E2EWorld, channelName: string) {
    assert.equal(
      channelName,
      STORY_CHANNEL_NAME,
      `unexpected channel name in story text: ${channelName}`
    );
    await ensureRunOwnedFollowSet();
  }
);

Given(
  'matthew, james and jessica each hold node-registry records they authored on v1',
  { timeout: 90_000 },
  async function (this: E2EWorld) {
    const outcome = await ensureNodeRegistryRecords();
    if (outcome === 'pending') return 'pending';
    return undefined;
  }
);

Given(
  'no peer has been restarted or re-keyed at any point in this story',
  function (this: E2EWorld) {
    // Documentary within Part 1: no step in this dispatch touches a conductor process, so
    // nothing HAS restarted or re-keyed yet by construction. Capturing each peer's pid now is
    // this Given's own contribution to Station 3+ (a later part), which is where "unchanged"
    // becomes a real comparison against these values after a canary adopt/revert/sunset.
    for (const peer of PEER_NAMES) {
      try {
        const pid = readFileSync(path.join(MESH_ROOT, 'pids', `conductor-${peer}`), 'utf8').trim();
        if (pid) lineage().backgroundPids[peer] = pid;
      } catch {
        // No pid file for this peer's launch style — not fatal for Part 1, which asserts
        // nothing further about it; Station 3's "conductor process id is unchanged" Then
        // (still 'pending' here) is where a missing pid would become a real gap.
      }
    }
  }
);

// ---------------------------------------------------------------------------
// Station 1: the crossing is admissible only when the release names what it
// migrates from
// ---------------------------------------------------------------------------

function resolveV2DnaHash(): string {
  const c = lineage();
  if (c.v2DnaHash) return c.v2DnaHash;
  assert.ok(
    existsSync(NODE_REGISTRY_V2_DNA),
    `v2 DNA not found at ${NODE_REGISTRY_V2_DNA} — build it (elohim/holochain/dna/node-registry: ` +
      'just build-witness, needs cargo) before running this station'
  );
  c.v2DnaHash = computeDnaHash(NODE_REGISTRY_V2_DNA);
  return c.v2DnaHash;
}

async function putV2HappToOtherPeers(): Promise<void> {
  const v2HappPath = resolveV2HappPath();
  const bytes = readFileSync(v2HappPath);
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  await Promise.all(
    PEER_NAMES.filter(peer => peer !== 'matthew').map(async peer => {
      const status = await putBlobRaw(directPeerUrl(peer), sha256, bytes);
      assert.ok(status === 200 || status === 201, `PUT v2 happ blob to ${peer} returned ${status}`);
    })
  );
}

function ensureChannelCreated(): void {
  const c = lineage();
  if (c.channelCreated) return;
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

/** Idempotent + self-verifying (same convention as rung-5's `ensure*` helpers): a later Given
 * (e.g. Station 2's "the lineage release is earned on its channel") calls this first so the
 * station can run standalone. */
async function ensureStation1Published(): Promise<void> {
  const c = lineage();
  if (c.releaseCid) return;
  assert.ok(c.v1DnaHash, 'v1 dnaHash not captured — run the Background first');
  const v2DnaHash = resolveV2DnaHash();

  mkdirSync(REPORT_DIR, { recursive: true });
  const minted = await mintLineageCandidate({
    role: NODE_REGISTRY_ROLE,
    v1DnaHash: c.v1DnaHash,
    v2DnaHash,
    pathCommitmentCid: PLACEHOLDER_PATH_COMMITMENT_CID,
    channelId: CHANNEL_ID,
    storageBaseUrl: directPeerUrl('matthew'),
    out: MANIFEST_PATH,
    discipline: {
      soakSecs: SOAK_SECS,
      attestationThreshold: ATTESTATION_THRESHOLD,
      canary: CANARY_PEER,
    },
  });
  assert.ok(existsSync(minted.manifestPath), `manifest was not written to ${minted.manifestPath}`);

  await putV2HappToOtherPeers();
  ensureChannelCreated();

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

interface ManifestRoleBindingFile {
  appliesTo?: { roles?: Record<string, { migrateFrom?: string; lineage?: string[] }> };
}

function assertManifestNamesV1Parent(): void {
  const c = lineage();
  const manifest = JSON.parse(readFileSync(MANIFEST_PATH, 'utf8')) as ManifestRoleBindingFile;
  const binding = manifest.appliesTo?.roles?.[NODE_REGISTRY_ROLE];
  assert.equal(binding?.migrateFrom, c.v1DnaHash, 'manifest does not declare migrateFrom = v1');
  assert.ok(
    binding?.lineage?.includes(c.v1DnaHash ?? ''),
    `manifest lineage chain ${JSON.stringify(binding?.lineage)} does not include v1 (${c.v1DnaHash})`
  );
}

/**
 * "Admissible" has no positive wire state on `/admin/adoption` yet (see module doc "The
 * verify_path caveat") — `verify_path` refuses EVERY happ-lineage release with
 * `path_not_notarized` today, once it gets that far. So "admissible" is read off two facts
 * together: (1) the manifest WE authored declares v1 as `migrateFrom` and a member of its own
 * `lineage` chain (`assertManifestNamesV1Parent` — the "naming v1 as the parent" half), and (2)
 * every peer's live refusal reason is NOT `dna_lineage_mismatch` (the "verifies… and reports it
 * admissible" half — `dna_lineage_mismatch` is `verify_envelope`'s DNA-line refusal, which runs
 * BEFORE `verify_path` in the composed floor; reaching a LATER refusal proves the bridge map
 * passed).
 */
async function assertStation1Admissible(): Promise<void> {
  const c = lineage();
  assert.ok(c.releaseCid, 'Station 1 has not published a release yet');
  assertManifestNamesV1Parent();

  const results = await pollAllPeers(
    (_peer, row) => row.resolvedHead?.cid === c.releaseCid && row.verdict !== null
  );
  for (const peer of PEER_NAMES) {
    const verdict = results[peer]?.row.verdict;
    const reason = verdict?.refusal?.reason;
    assert.notEqual(
      reason,
      'dna_lineage_mismatch',
      `${peer} refused the well-formed lineage release with dna_lineage_mismatch — the bridge ` +
        `map was NOT recognised: ${JSON.stringify(verdict)}`
    );
    // The passing path's own observed state is worth a receipt: today that is
    // `state: "refused", refusal.reason: "path_not_notarized"` — see module doc "The
    // verify_path caveat" for why that is the CORRECT admissible signal right now, not a
    // silent gap. Once the evidence fetch lands this line will start logging `state: "ok"`
    // (or later, `applied`) instead.
    console.error(
      `[happ-lineage-migration] Station 1 admissible check on ${peer}: state=${verdict?.state}, ` +
        `refusal.reason=${reason ?? '(none)'}`
    );
  }
}

async function ensureStation1NegativePublished(): Promise<void> {
  const c = lineage();
  await ensureStation1Published();
  if (c.negativeReleaseCid) return;
  const v2DnaHash = resolveV2DnaHash();
  const minted = await lineageReleaseWithoutParent({
    role: NODE_REGISTRY_ROLE,
    v2DnaHash,
    channelId: CHANNEL_ID,
    storageBaseUrl: directPeerUrl('matthew'),
    out: NEGATIVE_MANIFEST_PATH,
    discipline: {
      soakSecs: SOAK_SECS,
      attestationThreshold: ATTESTATION_THRESHOLD,
      canary: CANARY_PEER,
    },
  });
  assert.ok(existsSync(minted.manifestPath), `manifest was not written to ${minted.manifestPath}`);

  const published = runDriver(RELEASE_CEREMONY_SCRIPT, ['publish', NEGATIVE_MANIFEST_PATH]);
  assert.equal(
    published.status,
    0,
    `publish (negative control) failed (exit ${published.status}):\n--- stdout ---\n${published.stdout.trim()}\n--- stderr ---\n${published.stderr.trim()}`
  );
  const parsed = extractJson<PublishResult>(published.stdout);
  assert.ok(parsed.releaseCid, `publish output missing releaseCid: ${JSON.stringify(parsed)}`);
  c.negativeReleaseCid = parsed.releaseCid;
}

async function assertRefusedWithReason(
  releaseCid: string | undefined,
  expected: string
): Promise<void> {
  assert.ok(releaseCid, 'no release cid to check a refusal for');
  const results = await pollAllPeers(
    (_peer, row) => row.resolvedHead?.cid === releaseCid && row.verdict?.state === 'refused'
  );
  for (const peer of PEER_NAMES) {
    const reason = results[peer]?.row.verdict?.refusal?.reason;
    assert.ok(
      reason,
      `${peer} carries no refusal reason: ${JSON.stringify(results[peer]?.row.verdict)}`
    );
    assert.equal(
      storyReasonLabel(reason),
      expected,
      `${peer} refused with "${reason}" (-> "${storyReasonLabel(reason)}"), story names "${expected}"`
    );
  }
}

When(
  'matthew publishes a lineage release for the node-registry role whose manifest migrates from v1 and installs v2',
  { timeout: 180_000 },
  async function (this: E2EWorld) {
    await ensureStation1Published();
  }
);

Then(
  "every peer's runtime verifies that release locally and reports it admissible, naming v1 as the parent its bridge map recognises",
  { timeout: 90_000 },
  async function (this: E2EWorld) {
    await assertStation1Admissible();
  }
);

When(
  'matthew publishes a second release that installs v2 without naming what it migrates from',
  { timeout: 180_000 },
  async function (this: E2EWorld) {
    await ensureStation1NegativePublished();
  }
);

Then(
  "every peer's runtime refuses it, each naming {string} as its reason",
  { timeout: 90_000 },
  async function (this: E2EWorld, expectedReason: string) {
    await assertRefusedWithReason(lineage().negativeReleaseCid, expectedReason);
  }
);

// ---------------------------------------------------------------------------
// Station 2: the path must be notarized before anyone walks it
// ---------------------------------------------------------------------------

async function ensureStation2Earned(): Promise<void> {
  const c = lineage();
  await ensureStation1Published();
  if (c.promotedAt) return;
  const releaseCid = c.releaseCid as string;
  const promoted = runDriver(RELEASE_CEREMONY_SCRIPT, ['promote', CHANNEL_ID, releaseCid], 60_000);
  assert.equal(
    promoted.status,
    0,
    `promote failed (exit ${promoted.status}):\n--- stdout ---\n${promoted.stdout.trim()}\n--- stderr ---\n${promoted.stderr.trim()}`
  );
  const parsed = extractJson<PromoteResult>(promoted.stdout);
  assert.equal(parsed.tier, 'earned', `promote did not declare earned: ${JSON.stringify(parsed)}`);
  await pollAllPeers(
    (_peer, row) => row.resolvedHead?.cid === releaseCid && row.resolvedHead?.tier === 'earned'
  );
  c.promotedAt = Date.now();
}

async function captureFreshReconcile(): Promise<void> {
  const c = lineage();
  const baselines: Partial<Record<PeerName, number | null>> = {};
  for (const peer of PEER_NAMES) baselines[peer] = c.lastRows[peer]?.lastCheckedAt ?? null;
  const results = await pollAllPeers(
    (peer, row) => row.lastCheckedAt !== null && row.lastCheckedAt !== baselines[peer]
  );
  for (const peer of PEER_NAMES) {
    const result = results[peer];
    assert.ok(result, `no fresh reconcile observed for ${peer}`);
    c.lastRows[peer] = result.row;
    c.lastRawText[peer] = result.rawText;
  }
}

/**
 * Builds Station 2's `migrates-lineage` payload and submits it with an EMPTY `signatures` array
 * — on purpose. `validate_lineage_signatures`
 * (elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs) refuses "signatures must be a
 * non-empty array" before it ever reaches the quorum count, so `create_commitment` is expected
 * to REJECT this call. The self-signing mishpat extern (something like `sign_bytes`, wrapping
 * `hdk::prelude::sign`) that would let this file produce a REAL signature over
 * `signing_payload_cid` is Task 11 part 2's — see `lineage-commitments.ts`'s own "Signing path"
 * module doc section, which names exactly this gap. This function does not throw on the
 * refusal; it records it so the Thens below can assert on the CURRENT true state rather than
 * hiding the failure.
 */
async function attemptMigrationNotarization(): Promise<void> {
  const c = lineage();
  assert.ok(c.releaseCid, 'no Station 1 release to notarize a path for');
  assert.ok(c.v1DnaHash && c.v2DnaHash, 'v1/v2 dna hashes not captured');
  const opensAt = new Date();
  const revertUntil = new Date(opensAt.getTime() + 24 * 60 * 60 * 1000);
  const payload = buildMigratesLineagePayload({
    role: NODE_REGISTRY_ROLE,
    fromDnaHash: c.v1DnaHash,
    toDnaHash: c.v2DnaHash,
    releaseCid: c.releaseCid,
    constitutionRoot: FIXTURE_CONSTITUTION_ROOT,
    rosterCid: FIXTURE_ROSTER_CID,
    evidence: { soak: [], forecast: null, deliberation: null },
    window: { opensAt: opensAt.toISOString(), revertUntil: revertUntil.toISOString() },
    requiredSignatures: 1,
    signatures: [],
  });
  c.migrationPayload = payload;

  const rail = await connectRoleConductor('matthew', MISHPAT_ROLE, MISHPAT_ZOME);
  try {
    const result = await notarizeMigration({ conductor: rail, actingPeer: 'matthew', payload });
    // Reached only once the empty-signatures refusal above no longer applies — a real
    // commitment now exists, and Station 2's second half can start passing.
    c.migrationCommitmentCid = result.cid;
  } catch (error) {
    c.migrationNotarizeError = String(error);

    console.error(
      `[happ-lineage-migration] Station 2's migrates-lineage notarization was refused, as ` +
        `expected today (empty signatures array — the self-signing extern is Task 11 part 2's): ` +
        `${String(error)}`
    );
  } finally {
    await rail.close();
  }
}

async function assertStation2AdoptableRed(): Promise<void> {
  const c = lineage();
  const releaseCid = c.releaseCid as string;
  await pollAllPeers(
    (_peer, row) => row.resolvedHead?.cid === releaseCid && row.verdict?.state !== 'refused',
    ADOPTABLE_RED_POLL_TIMEOUT_MS
  );
}

Given(
  'the lineage release is earned on its channel',
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    await ensureStation2Earned();
  }
);

Given('no migration commitment names it', function (this: E2EWorld) {
  assert.equal(
    lineage().migrationCommitmentCid,
    undefined,
    'a migration commitment was already notarized — this Given expects none yet'
  );
});

When("each peer's runtime next reconciles", { timeout: 90_000 }, async function (this: E2EWorld) {
  await captureFreshReconcile();
});

Then(
  'no peer installs v2, and each names {string} as its reason',
  function (this: E2EWorld, expectedReason: string) {
    const c = lineage();
    for (const peer of PEER_NAMES) {
      const row = c.lastRows[peer];
      assert.ok(
        row,
        `no captured reconcile row for ${peer} — run "each peer's runtime next reconciles" first`
      );
      assert.notEqual(
        row.verdict?.state,
        'applied',
        `${peer} installed v2 (verdict.state=applied)`
      );
      const reason = row.verdict?.refusal?.reason;
      assert.ok(reason, `${peer} carries no refusal reason: ${JSON.stringify(row.verdict)}`);
      assert.equal(
        storyReasonLabel(reason),
        expectedReason,
        `${peer} refused with "${reason}" (-> "${storyReasonLabel(reason)}"), story names "${expectedReason}"`
      );
    }
  }
);

When(
  'the elohim notarize a migration commitment naming that release, v1 as its origin, v2 as its target and a revert horizon',
  { timeout: 30_000 },
  async function (this: E2EWorld) {
    await attemptMigrationNotarization();
  }
);

Then(
  "each peer's runtime reads that commitment through its own conductor, not from the release, and reports the release adoptable",
  { timeout: 150_000 },
  async function (this: E2EWorld) {
    // KNOWN RED as of this dispatch (2026-09-04) — do not "fix" by loosening the assertion.
    // Two independent, named gaps (module doc "The verify_path caveat" and
    // `attemptMigrationNotarization`'s own comment):
    //   1. The notarization attempt above submits `signatures: []` on purpose (the self-signing
    //      mishpat extern is Task 11 part 2's), so `validate_lineage_signatures` refuses the
    //      commitment before it is ever created — `lineage().migrationCommitmentCid` stays
    //      undefined.
    //   2. Even with a REAL commitment notarized, `/admin/adoption`'s sweep caller passes
    //      `Answer::Absent` for `VerifyInput.path` today (that field's own doc: "Absent for
    //      every existing caller today — the fetch site is a later task's"), so `verify_path`
    //      refuses `path_not_notarized` regardless of what mishpat holds.
    // This Then is written faithfully to the story and will start passing once both gaps close;
    // it is expected to fail/time out until then.
    await assertStation2AdoptableRed();
  }
);

Then('no peer asked anyone in the household anything', function (this: E2EWorld) {
  const forbidden = ['prompt', 'confirm(', 'approve this', 'please click', 'y/n', 'yes/no'];
  for (const [peer, rawText] of Object.entries(lineage().lastRawText)) {
    if (!rawText) continue;
    const lower = rawText.toLowerCase();
    for (const word of forbidden) {
      assert.ok(
        !lower.includes(word),
        `${peer}'s ${ADOPTION_PATH} report contains "${word}" — a peer's runtime asked something`
      );
    }
  }
});

// ---------------------------------------------------------------------------
// Stations 3-10: skeleton only (this dispatch's Part 1 scope). Generated from
// `pnpm exec cucumber-js --dry-run --format snippets` against this feature
// (the repo's `generate:skeletons` script globs genesis docs only, not a2o's
// own features — see the task's own dispatch notes), then hand-grouped under
// each Station's own comment header. Every step here is UNIMPLEMENTED by
// design; a later part fills each Station in turn, red before green, per the
// habit atom's own DELTA discipline.
// ---------------------------------------------------------------------------

// ── Station 3 — james, the canary, runs v2 beside v1 under the same key with nothing restarted ──

Given("james's runtime follows the channel in canary mode", function () {
  return 'pending';
});

Given(
  'the lineage release is earned and a migration commitment naming it is notarized, so the window is open',
  function () {
    return 'pending';
  }
);

When("james's runtime next reconciles", function () {
  return 'pending';
});

Then(
  "james's runtime installs v2 as a second installed app beside v1 under james's existing agent key, giving him a second cell for the role",
  function () {
    return 'pending';
  }
);

Then(
  "james's passport shows the node-registry role with two cells — v1 reading, v2 authoring — and the same agent key on both",
  function () {
    return 'pending';
  }
);

Then("james's conductor process id is unchanged and his v1 chain is untouched", function () {
  return 'pending';
});

// ── Station 4 — james's own records cross with their original proof, and v2 checks it for itself ──

Given('james is dual-celled on the node-registry role, authoring on v2', function () {
  return 'pending';
});

When("james's runtime carries his v1 node-registry records into v2", function () {
  return 'pending';
});

Then('every carried record exists in v2 with the same entry hash it had in v1', function () {
  return 'pending';
});

Then(
  "each is covered by a witness whose v1 action and signature verify under v2's own validation",
  function () {
    return 'pending';
  }
);

Then(
  "james's storage projection shows each record's notarized time and author as the v1 ones, with the v2 anchor beside them",
  function () {
    return 'pending';
  }
);

Then(
  "the carry receipt's count equals james's v1 record count and its digest equals the digest v1 computes when asked to export those records",
  function () {
    return 'pending';
  }
);

// ── Station 5 — jessica's record is readable in v2 with jessica's signature intact, though jessica never moved ──

Given('jessica has not adopted the release and keeps running v1 only', function () {
  return 'pending';
});

When("james's bridge sweep runs", function () {
  return 'pending';
});

Then(
  "jessica's v1 node-registry record is readable in v2 through a witness that names james as its courier",
  function () {
    return 'pending';
  }
);

Then(
  "that witness carries jessica's original action and signature, and v2's validation accepts it",
  function () {
    return 'pending';
  }
);

Then("jessica's own chain has not been written to by anyone", function () {
  return 'pending';
});

// ── Station 6 — the window keeps both sides talking, and reports which way it can carry ──

Given('the window is open and jessica keeps authoring on v1', function () {
  return 'pending';
});

When('jessica creates a new node-registry record on v1', function () {
  return 'pending';
});

Then(
  "james's bridge sweep carries it into v2 within one sweep interval, held with jessica's signature",
  function () {
    return 'pending';
  }
);

Then(
  "james's passport reports backward carry as unavailable, because v1 does not carry the witness type",
  function () {
    return 'pending';
  }
);

Then('no peer reports jessica as stale — she is within the window', function () {
  return 'pending';
});

// ── Station 7 — before the sunset, the elohim revoke the path and every peer is back on v1 with nothing lost ──

Given(
  "james has adopted v2 as the canary and self-carried his records, and matthew has followed and done the same after reading james's attestation",
  function () {
    return 'pending';
  }
);

Given('james has authored a new node-registry record on v2 during the window', function () {
  return 'pending';
});

Given('jessica has not adopted v2', function () {
  return 'pending';
});

Given(
  'jessica has raised through her elohim that a record of hers was held-carried with the wrong courier named',
  function () {
    return 'pending';
  }
);

When(
  "the elohim notarize a revocation of the migration commitment, inside its revert horizon, naming jessica's raised concern as its cause",
  function () {
    return 'pending';
  }
);

Then(
  "the release is no longer held, and the channel's earned head returns to the prior release by re-election",
  function () {
    return 'pending';
  }
);

Then(
  'james and matthew mark v1 authoring and v2 reading, disable their v2 cells, and uninstall nothing',
  function () {
    return 'pending';
  }
);

Then(
  'every record any of them authored on v1 before or during the window is still on v1, untouched',
  function () {
    return 'pending';
  }
);

Then(
  "james's record authored on v2 during the window is re-authored by james on v1 with the same entry hash, its v2 proof kept in the disabled cell as evidence",
  function () {
    return 'pending';
  }
);

Then(
  "any v2-authored record not yet re-authored on v1 is reported by its author's passport as pending, never as lost",
  function () {
    return 'pending';
  }
);

Then("jessica's runtime never noticed anything but a head that moved and moved back", function () {
  return 'pending';
});

// ── Station 8 — no sunset without its own commitment; with it the old chains close, stay readable, and no revocation reopens them ──

Given(
  'a fresh migration commitment is notarized and all three peers are dual-celled — v1 reading, v2 authoring — and have attested their carry',
  function () {
    return 'pending';
  }
);

Given('no sunset commitment exists', function () {
  return 'pending';
});

Then('no peer closes its v1 chain', function () {
  return 'pending';
});

When('the elohim notarize a sunset commitment naming the migration', function () {
  return 'pending';
});

Then(
  "each peer's runtime seals the close on its v1 cell naming v2, then the open on its already-running v2 cell naming that close, in that order",
  function () {
    return 'pending';
  }
);

Then('each closed v1 chain is still readable by every peer', function () {
  return 'pending';
});

Then(
  'each peer carries its own close into v2 as a proof, so v2 knows where every old chain ended',
  function () {
    return 'pending';
  }
);

Then(
  "each peer's runtime has disabled its v1 cell, so nothing of its own is written there again",
  function () {
    return 'pending';
  }
);

Then(
  "each peer's passport shows the node-registry role with v2 authoring and v1 closed",
  function () {
    return 'pending';
  }
);

When(
  "the test harness, holding james's key, writes a fact on james's closed v1 cell and offers it to v2 as a carried proof",
  function () {
    return 'pending';
  }
);

Then(
  "the v1 conductor itself accepts that write — the substrate does not fence a closed chain, as the epic's kernel test measured",
  function () {
    return 'pending';
  }
);

Then(
  "v2's validation on every peer refuses the carried proof, naming {string} as its reason",
  function (_reason: string) {
    return 'pending';
  }
);

When('a revocation of the migration commitment is notarized after the sunset', function () {
  return 'pending';
});

Then(
  "nothing changes: the closed chains stay closed, and each peer's passport still shows the node-registry role with v2 authoring and v1 closed",
  function () {
    return 'pending';
  }
);

// ── Station 9 — a forged witness, whoever commits it, is refused by every peer's own validation, naming why ──

Given('the test harness joins the mesh as a fourth peer running v2', function () {
  return 'pending';
});

When(
  "the harness commits a witness whose signature does not verify against the action's signer",
  function () {
    return 'pending';
  }
);

Then(
  "v2's validation on every peer refuses it, naming {string} as its reason",
  function (_reason: string) {
    return 'pending';
  }
);

When(
  'the harness commits a witness naming a parent rule version the v2 DNA does not declare in its lineage',
  function () {
    return 'pending';
  }
);

Then('neither refusal disturbs any record that was carried honestly', function () {
  return 'pending';
});

// ── Station 10 — a commitment the roster did not hold is refused by every peer's own verification, whatever it claims ──

Given(
  "the household's declared council roster for the node-registry role is the bootstrap steward's key alone",
  function () {
    return 'pending';
  }
);

When(
  'the test harness records a migration commitment naming that release, signed by a key that is not on the roster',
  function () {
    return 'pending';
  }
);

When(
  "the harness records a migration commitment naming that release, signed by the steward's key but under a constitution root the v2 DNA does not declare",
  function () {
    return 'pending';
  }
);

Then(
  'the release itself is still earned and still admissible — only the path was refused',
  function () {
    return 'pending';
  }
);
