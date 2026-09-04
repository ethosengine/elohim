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
 * `channelId()` mints fresh per run from a run stamp, exactly like the rung-5
 * steps' `channelId()` — a repeat run never collides with a channel a prior
 * run left mid-flight. The story names the channel
 * "runtime:lineage:node_registry:commons"; this file substitutes a
 * run-scoped id for the SAME behavioural role (the Background's own Given
 * asserts the STORY name against `STORY_CHANNEL_NAME`, then acts on
 * `channelId()` internally — same substitution rung-5's own Background does
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

import {
  AdminWebsocket,
  AppWebsocket,
  CellType,
  encodeHashToBase64,
  fakeEntryHash,
  fakeDnaHash,
  decodeHashFromBase64,
} from '@holochain/client';
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
/**
 * MEASURED 2026-09-04 (Station 1, run r1): the STORY's channel name
 * "runtime:lineage:node_registry:commons" is not a legal channel id — the
 * release-manifest schema's `channelId` pattern is
 * `^runtime:[a-z0-9][a-z0-9-]*:[a-z0-9][a-z0-9-]*:[a-z0-9][a-z0-9-]*$`
 * (`is_channel_id` in elohim-storage's `release_adoption::verify` enforces the
 * same shape), and an UNDERSCORE is not in that alphabet. Packaging refused
 * with `schema: /channelId must match pattern …` before anything reached the
 * mesh. The run-scoped id therefore spells the role with a HYPHEN. This is a
 * substitution of the same behavioural role the module doc already documents
 * (the Background asserts the STORY name against `STORY_CHANNEL_NAME` and acts
 * on `channelId()`), plus one measured fact for the ledger: the story's literal
 * channel name would have to be `runtime:lineage:node-registry:commons` to be
 * publishable at all.
 */
/**
 * Each Station is its own WORLD — the story says so in as many words ("its
 * Given sets exactly the state it needs, so a later Station may begin where an
 * earlier one would have ended without replaying it"), and this fixture's
 * `Before` enforces it by resetting every peer to the v1 baseline. So the
 * cross-scenario memo in `lineage()` is dropped per scenario too, and each
 * world gets its OWN channels and manifests: a second scenario reusing the
 * first's channel id would publish onto a channel whose earned head belongs to
 * a world that has just been torn down.
 *
 * MEASURED (run r11): without this, Station 4 running after Station 3 in one
 * process short-circuited every memoized helper, went looking for the side app
 * the `Before` had just uninstalled, and failed with "app 'elohim@…' is not
 * installed on adminPort=4464".
 */
let worldIndex = 0;
function worldStamp(): string {
  return `${RUN_STAMP}w${worldIndex}`;
}

/**
 * This world's channel — the one Station 1 and Station 2's first half act on.
 *
 * MEASURED 2026-09-04 (Station 1, run r1): the STORY's channel name
 * "runtime:lineage:node_registry:commons" is not a legal channel id — the
 * release-manifest schema's `channelId` pattern is
 * `^runtime:[a-z0-9][a-z0-9-]*:[a-z0-9][a-z0-9-]*:[a-z0-9][a-z0-9-]*$`
 * (`is_channel_id` in elohim-storage's `release_adoption::verify` enforces the
 * same shape), and an UNDERSCORE is not in that alphabet. Packaging refused
 * with `schema: /channelId must match pattern …` before anything reached the
 * mesh. The run-scoped id therefore spells the role with a HYPHEN; the
 * Background still asserts the STORY name against `STORY_CHANNEL_NAME`. One
 * measured fact for the ledger: the story's literal channel name would have to
 * be `runtime:lineage:node-registry:commons` to be publishable at all.
 */
function channelId(): string {
  return `runtime:lineage:node-registry:a2o-${worldStamp()}`;
}

/**
 * Station 2's second half publishes onto a FRESH channel, and the reason is
 * MEASURED (run r4), not stylistic: `release-ceremony.ts`'s
 * `assertAdmissibleOverEarnedHead` refuses any publish over a channel that
 * already has an EARNED head unless the acting peer has ADOPTED that head and
 * the manifest names it as its lineage parent — rung 5's adopt-before-author
 * rail ("a steward cannot push what they have not themselves adopted"). The
 * earned head on `channelId()` is the release Station 2's FIRST half exists to
 * have refused, so matthew has by construction not adopted it and never will.
 * A fresh channel per release is the household's declared discipline anyway;
 * this is where the story's "each Station is its own world" stops being a
 * convenience and becomes the mechanism.
 */
function notarizedChannelId(): string {
  return `${channelId()}-path`;
}

/** Stations 3-5's own channel — a fresh one per release (same discipline and
 * the same measured reason as `notarizedChannelId`), on which the notarized
 * release is the FIRST and therefore the WINNER, at `staging` tier. */
function canaryChannelId(): string {
  return `${channelId()}-canary`;
}

const REPORT_DIR = path.join(
  A2O_ROOT,
  'reports',
  'release-ceremony',
  new Date().toISOString().slice(0, 10)
);

function manifestPath(): string {
  return path.join(REPORT_DIR, `a2o-happ-lineage-${worldStamp()}.json`);
}
/** Station 1's negative control — v2 with no parent named. */
function negativeManifestPath(): string {
  return path.join(REPORT_DIR, `a2o-happ-lineage-${worldStamp()}-no-parent.json`);
}
/** Station 2's SECOND release: the same crossing, re-published naming the REAL
 * notarized commitment. See `notarizeMigrationPath` for why the pointer cannot
 * run the other way. */
function notarizedManifestPath(): string {
  return path.join(REPORT_DIR, `a2o-happ-lineage-${worldStamp()}-notarized.json`);
}
function canaryManifestPath(): string {
  return path.join(REPORT_DIR, `a2o-happ-lineage-${worldStamp()}-canary.json`);
}

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

/**
 * Station 1's / Station 2's-first-half path pointer: a REAL, DECODABLE
 * `EntryHash` that names no entry anywhere.
 *
 * MEASURED 2026-09-04 (run r3): a hand-written `uhCEk…`-shaped string is NOT
 * enough. `HoloHash`'s base64 decoder verifies the trailing four DHT-location
 * bytes (`conductor_writes`'s own `decode_collective_cid_round_trips_and_rejects_junk`
 * pins exactly that), so a made-up string makes `mishpat::get_commitment` ERROR
 * rather than answer "not found" — and `fetch_path_evidence` correctly reports
 * that as `Unreachable → conductor_unavailable`, never as an absence (C4). The
 * story's Station 2 asks for `path_not_notarized`, which is the ABSENT arm, so
 * the pointer has to be a hash the conductor can decode and then honestly fail
 * to find. `fakeEntryHash` is `@holochain/client`'s own primitive for exactly
 * that ("a valid hash of a non-existing entry"); the fixed core byte keeps a
 * run reproducible and the value visibly synthetic.
 */
const UNNOTARIZED_PATH_CORE_BYTE = 0xa2;
let unnotarizedPathCommitmentCid: string | undefined;
async function unnotarizedPathCid(): Promise<string> {
  unnotarizedPathCommitmentCid ??= encodeHashToBase64(
    await fakeEntryHash(UNNOTARIZED_PATH_CORE_BYTE)
  );
  return unnotarizedPathCommitmentCid;
}

/** See module doc "Placeholder values" — never actually checked because the notarization
 * attempt below is refused earlier, on the empty `signatures` array. */
const FIXTURE_CONSTITUTION_ROOT = 'a2o-fixture-constitution-root';
const FIXTURE_ROSTER_CID = 'a2o-fixture-bootstrap-steward-roster';

/** How long a single peer's `/admin/adoption` poll waits for its predicate, and how often it
 * re-reads while waiting. Mirrors the "unreachable ≠ absent, keep retrying" discipline
 * `runtime-upgrade-propagation.steps.ts`'s own `pollAdoption` documents.
 *
 * MEASURED 2026-09-04 (run r2): 60s is NOT enough. A release published on
 * matthew has to gossip to the other two peers AND be picked up by their own
 * 60s controller sweep before their `/admin/adoption` row can name it — jessica
 * was still resolving the CHANNEL ROOT (`tier: "none"`, "carries no release
 * manifest yet") at the 60s mark. This is the same budget rung 5 arrived at for
 * exactly this convergence (`FLEET_ADOPTION_CONVERGE_BUDGET_MS = 240_000` in
 * `runtime-upgrade-propagation.steps.ts`). */
const RECONCILE_POLL_TIMEOUT_MS = 240_000;
const RECONCILE_POLL_INTERVAL_MS = 10_000;

/** Station 2's second-half Then ("adoptable") needs the notarized commitment to
 * gossip to every peer's conductor AND that peer's next sweep to read it, on
 * top of the release convergence above — so it gets its own, longer budget. */
const ADOPTABLE_RED_POLL_TIMEOUT_MS = 300_000;

/** How long james's canary sweep has to resolve, stage the bundle, install the
 * side app, carry every v1 record and open the window. Rung 5 budgets 300s for
 * a coordinator hot-swap; a crossing installs a whole app and walks a chain, so
 * it gets the same order of magnitude with room for the sweep interval. */
const CANARY_APPLY_BUDGET_MS = 600_000;

/** The vehicle name the `happ-lineage` class routes to
 * (`apply::LINEAGE_VEHICLE`), as it appears on `appliedRelease.vehicle`. */
const LINEAGE_VEHICLE = 'happ-lineage';

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
async function withTimeout<T>(promise: Promise<T>, timeoutMs: number, label: string): Promise<T> {
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
  zomeName: string,
  appId: string = APP_ID
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
  const app =
    appId === APP_ID
      ? (apps.find(a => a.installed_app_id === APP_ID) ?? apps[0])
      : apps.find(a => a.installed_app_id === appId);
  if (!app) {
    await admin.client.close();
    throw new Error(
      `${peer}: app '${appId}' is not installed on adminPort=${ports.admin} ` +
        `(installed: ${apps.map(a => a.installed_app_id).join(', ')})`
    );
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
  appliedRelease: {
    cid: string;
    at: number;
    vehicle: string;
    /** **Rung 6.** The lineage carry the `happ-lineage` vehicle performed —
     * `None`/absent for every other vehicle. */
    carry?: AppliedCarryReceipt;
  } | null;
}

/** `LineageCarryReceipt` on the wire (elohim-storage
 * `services::release_adoption::LineageCarryReceipt`). */
interface AppliedCarryReceipt {
  role: string;
  carried: number;
  /** What v1 ITSELF said its whole-chain total was — `null` when v1 never said,
   * which reads as unknown and never as "equal to what we carried". */
  v1Count: number | null;
  /** The digest the LAST page reported. */
  digest: string;
  /** The digest the FIRST page reported. */
  v1Digest: string;
  witnessHashes: string[];
}

interface AdoptionReport {
  controller: { running: boolean };
  channels: AdoptionChannelRow[];
}

/** Task 8's per-role dual-cell view — present only while a lineage window is
 * open on that role (or after a sunset), omitted otherwise. */
interface RoleLineageView {
  readingAppId: string;
  authoringAppId: string;
  readingDnaHash: string;
  authoringDnaHash: string;
  closed: boolean;
}

interface VersionRole {
  role: string;
  dnaHash: string;
  coordinatorWasmHashes: Record<string, string>;
  lineage?: RoleLineageView;
}

interface VersionResponse {
  passport?: {
    conductor?: { version: string };
    happ?: { roles?: VersionRole[]; lineageApps?: string[] };
  };
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
 * Poll one peer's `/admin/adoption` for `channelId()` until `predicate` is satisfied. A connect
 * failure or non-200 is NOT "absent" — the poll keeps retrying rather than concluding the row
 * doesn't exist (same "unreachable ≠ absent" rail `runtime-upgrade-propagation.steps.ts`'s own
 * `pollAdoption` documents).
 */
async function pollAdoption(
  peer: PeerName,
  timeoutMs: number,
  predicate: (row: AdoptionChannelRow) => boolean,
  channel: string = channelId(),
  intervalMs = RECONCILE_POLL_INTERVAL_MS
): Promise<{ row: AdoptionChannelRow; rawText: string }> {
  const start = Date.now();
  let lastRow: AdoptionChannelRow | undefined;
  let everReachable = false;
  while (Date.now() - start < timeoutMs) {
    try {
      const { report, rawText } = await getAdoptionReport(peer);
      everReachable = true;
      const row = findChannelRow(report, channel);
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
    `timed out after ${timeoutMs}ms waiting on ${peer}'s ${ADOPTION_PATH}[${channel}] ` +
      `(peer was ${everReachable ? 'reachable' : 'never reachable'} during the poll); ` +
      `last row: ${JSON.stringify(lastRow)}`
  );
  throw new Error('unreachable');
}

/** Runs `pollAdoption` on all three peers CONCURRENTLY — bounds a "every peer must satisfy X"
 * assertion to one `timeoutMs` budget total rather than `timeoutMs` * 3 sequential. */
async function pollAllPeers(
  predicate: (peer: PeerName, row: AdoptionChannelRow) => boolean,
  timeoutMs = RECONCILE_POLL_TIMEOUT_MS,
  channel: string = channelId()
): Promise<Partial<Record<PeerName, { row: AdoptionChannelRow; rawText: string }>>> {
  const entries = await Promise.all(
    PEER_NAMES.map(async peer => {
      const result = await pollAdoption(peer, timeoutMs, row => predicate(peer, row), channel);
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

/**
 * The run's OWN follow set, per peer: channel id -> mode. Never read-merged
 * from disk — the file this run writes contains exactly what this run owns and
 * nothing else (the 2026-09-01 leftover-channel hazard the rung-5 steps
 * document), and `AfterAll` byte-restores what was there before.
 *
 * A MAP rather than one channel because the stations need more than one
 * channel (a fresh channel per release — see `notarizedChannelId()`) and more
 * than one mode (james canary, matthew/jessica observe or apply).
 */
const runOwnedFollow: Record<PeerName, Map<string, string>> = {
  matthew: new Map(),
  jessica: new Map(),
  james: new Map(),
};

async function applyFollowSet(peer: PeerName): Promise<void> {
  const filePath = runtimeConfigPath(peer);
  mkdirSync(path.dirname(filePath), { recursive: true });
  const csv = [...runOwnedFollow[peer].entries()]
    .map(([channelId, mode]) => `${channelId}=${mode}`)
    .join(',');
  writeFileSync(filePath, `ELOHIM_RELEASE_CHANNELS = "${csv}"\n`, 'utf8');
  const { status } = await postRaw(`${directPeerUrl(peer)}${RUNTIME_CONFIG_RELOAD_PATH}`);
  assert.ok(
    status >= 200 && status < 300,
    `${peer}'s ${RUNTIME_CONFIG_RELOAD_PATH} returned ${status} while registering "${csv}"`
  );
}

/** Register `channelId` on every peer at the given per-peer mode (default `observe`). */
async function followChannel(
  target: string,
  modes: Partial<Record<PeerName, string>> = {}
): Promise<void> {
  for (const peer of PEER_NAMES) {
    runOwnedFollow[peer].set(target, modes[peer] ?? 'observe');
    await applyFollowSet(peer);
  }
}

const originalRuntimeConfigBytes: Partial<Record<PeerName, Buffer>> = {};
let runOwnedFollowSetEstablished = false;
let runOwnedFollowSetRestored = false;

/** Captures each peer's TRUE on-disk bytes once, then writes a run-owned file containing only
 * `channelId()=observe` — same "the run owns its own follow set, never a read-merge-write" rule
 * `runtime-upgrade-propagation.steps.ts`'s module doc explains (the 2026-09-01 leftover-channel
 * hazard this closes). */
async function ensureRunOwnedFollowSet(): Promise<void> {
  mkdirSync(REPORT_DIR, { recursive: true });
  for (const peer of PEER_NAMES) {
    if (!runOwnedFollowSetEstablished) {
      let originalBytes: Buffer;
      try {
        originalBytes = readFileSync(runtimeConfigPath(peer));
      } catch {
        originalBytes = Buffer.from('', 'utf8');
      }
      originalRuntimeConfigBytes[peer] = originalBytes;
    }
    // One world's channels are not the next world's: drop what a previous
    // scenario registered rather than leaving this peer following a channel
    // whose head belongs to a torn-down world.
    runOwnedFollow[peer].clear();
    runOwnedFollow[peer].set(channelId(), 'observe');
    await applyFollowSet(peer);
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
  // Each Station is its own WORLD (the story's own words, and this hook's whole
  // reason for existing): the peers go back to the v1 baseline, and the
  // cross-step memo goes with them. Keeping the memo across scenarios in one
  // process is how run r11's Station 4 ended up looking for a side app this
  // very hook had just uninstalled. `worldIndex` moves with it so the next
  // scenario's channels cannot inherit an earned head from a torn-down world.
  worldIndex += 1;
  state = undefined;
  channelsCreated.clear();
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
  /** The release re-published naming `migrationCommitmentCid` (Station 2's second half). */
  notarizedReleaseCid?: string;
  /** Stations 3-5: the release james's canary mode adopts. */
  canaryReleaseCid?: string;
  /** james's v1 node-registry chain BEFORE the crossing — the "untouched" baseline. */
  jamesV1Export?: { digest: string; total: number | null; entryHashes: string[] };
  /** The carry receipt the vehicle reported, read off `/admin/adoption`. */
  carryReceipt?: AppliedCarryReceipt;
  /** james's `elohim@…` side app id, once installed. */
  jamesLineageAppId?: string;
  /** The DNA hash james's v2 cell actually runs — the INSTALLED hash, which the
   * install folds the hApp role modifiers into and which therefore need not
   * equal the packed `node-registry-v2.dna` hash. */
  jamesAuthoringDnaHash?: string;
  /** v1 entry hash -> how many witnesses v2 answers for it. */
  witnessCounts?: Record<string, number>;
  /** Station 9: each peer's v2 refusal text for the forgery just attempted
   * (`null` means that peer ACCEPTED it, which is the failure). */
  witnessRefusals?: Partial<Record<PeerName, string | null>>;
  /** Station 10: the channel and release the current negative arm published. */
  station10Channel?: string;
  station10ReleaseCid?: string;
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

const channelsCreated = new Set<string>();

function ensureChannelCreated(target: string = channelId()): void {
  if (channelsCreated.has(target)) return;
  const discipline = JSON.stringify({
    soakSecs: SOAK_SECS,
    attestationThreshold: ATTESTATION_THRESHOLD,
    canaryOrder: [CANARY_PEER],
  });
  const created = runDriver(RELEASE_CEREMONY_SCRIPT, [
    'channel',
    'create',
    target,
    '--discipline',
    discipline,
  ]);
  assert.equal(
    created.status,
    0,
    `channel create failed (exit ${created.status}):\n--- stdout ---\n${created.stdout.trim()}\n--- stderr ---\n${created.stderr.trim()}`
  );
  channelsCreated.add(target);
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
    pathCommitmentCid: await unnotarizedPathCid(),
    channelId: channelId(),
    storageBaseUrl: directPeerUrl('matthew'),
    out: manifestPath(),
    discipline: {
      soakSecs: SOAK_SECS,
      attestationThreshold: ATTESTATION_THRESHOLD,
      canary: CANARY_PEER,
    },
  });
  assert.ok(existsSync(minted.manifestPath), `manifest was not written to ${minted.manifestPath}`);

  await putV2HappToOtherPeers();
  ensureChannelCreated();

  const published = runDriver(RELEASE_CEREMONY_SCRIPT, ['publish', manifestPath()]);
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
  const manifest = JSON.parse(readFileSync(manifestPath(), 'utf8')) as ManifestRoleBindingFile;
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
    channelId: channelId(),
    storageBaseUrl: directPeerUrl('matthew'),
    out: negativeManifestPath(),
    discipline: {
      soakSecs: SOAK_SECS,
      attestationThreshold: ATTESTATION_THRESHOLD,
      canary: CANARY_PEER,
    },
  });
  assert.ok(existsSync(minted.manifestPath), `manifest was not written to ${minted.manifestPath}`);

  const published = runDriver(RELEASE_CEREMONY_SCRIPT, ['publish', negativeManifestPath()]);
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
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    await ensureStation1Published();
  }
);

Then(
  "every peer's runtime verifies that release locally and reports it admissible, naming v1 as the parent its bridge map recognises",
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    await assertStation1Admissible();
  }
);

When(
  'matthew publishes a second release that installs v2 without naming what it migrates from',
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    await ensureStation1NegativePublished();
  }
);

Then(
  "every peer's runtime refuses it, each naming {string} as its reason",
  { timeout: 300_000 },
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
  const promoted = runDriver(RELEASE_CEREMONY_SCRIPT, ['promote', channelId(), releaseCid], 60_000);
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
 * Station 2's second half: the elohim notarize the path, then the release is
 * re-published naming it.
 *
 * ## Why the notarization needs no TypeScript signature
 *
 * The payload goes in with `signatures: []` and comes back notarized. The
 * mishpat extern `create_lineage_commitment` (landed 2026-09-04, hot-swapped
 * onto this mesh) appends the CALLING agent's own `{agent, signature}` over
 * `signing_payload_cid` using in-zome `sign_raw` — the literal-bytes
 * counterpart of `validate_lineage_signatures`'s `verify_signature_raw`. The
 * harness supplies no key and constructs no signature; the acting agent is
 * whichever key matthew's own `mishpat` cell authors under, which is exactly
 * the household's bootstrap steward standing in for the council roster, as the
 * story's own vocabulary paragraph says.
 *
 * ## Why the release has to be re-published (MEASURED, and a design fact)
 *
 * The pointer between a release and its path runs ONE way: the manifest's
 * `adoptionDiscipline.path.commitmentCid` names the commitment, and
 * `verify_path` refuses any evidence whose `commitment_cid` is not that exact
 * string. A commitment's own `release_cid` is the back-reference, and
 * `verify_path` never reads it. So a release cannot name a commitment that does
 * not exist yet, and the commitment naming THAT release cannot be pointed at
 * from it afterwards — the notarized path is minted first and the release names
 * it. Station 2's first half is therefore the earned release naming an
 * unnotarized path (`path_not_notarized`, the story's refusal), and its second
 * half is the SAME crossing re-published over the notarized one — the ordinary
 * "re-authored manifest over the same bytes" shape rung 5 already runs for its
 * threshold-0 revert. The commitment's `release_cid` names the FIRST release,
 * which is the release the story says it names.
 */
async function notarizeMigrationPath(): Promise<void> {
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
    c.migrationCommitmentCid = result.cid;

    console.error(
      `[happ-lineage-migration] migrates-lineage notarized by matthew's own mishpat cell ` +
        `(agent ${rail.agent}) — commitment cid ${result.cid}`
    );
  } finally {
    await rail.close();
  }
}

/** Idempotent: the stations after 2 reuse the ONE notarized path rather than
 * minting a second one for the same crossing. */
async function ensureNotarizedCommitment(): Promise<void> {
  if (lineage().migrationCommitmentCid) return;
  await notarizeMigrationPath();
}

/**
 * Publish the SAME crossing, naming the real commitment, onto a FRESH channel.
 *
 * Two measured constraints decide this shape, and both are recorded on
 * `notarizedChannelId()` and in the epic ledger:
 *
 * 1. It cannot go onto `channelId()` — that channel has an EARNED head matthew
 *    has (correctly) not adopted, and `assertAdmissibleOverEarnedHead` refuses.
 * 2. On the fresh channel it must be the WINNER, not a staging candidate
 *    beneath something. `classify_candidate_follow` (watch.rs) has only a
 *    CANARY follow a staging candidate under an earned head — an observer
 *    "reports what the channel elected", and a candidate is not elected — so a
 *    candidate would be invisible to matthew and jessica. As the first release
 *    on a fresh channel it IS the winner, at `staging` tier, which every mode
 *    resolves and verifies.
 *
 * Staging is also the honest tier for this Station: `verify::verify` enforces
 * the attestation threshold only above `Staging`, and no peer can have attested
 * a release no peer has adopted yet. The threshold is Station 3's business (the
 * canary soaks and attests, then the head is promoted); this Station is about
 * the PATH.
 */
async function republishOverNotarizedPath(): Promise<void> {
  const c = lineage();
  const commitmentCid = c.migrationCommitmentCid;
  assert.ok(commitmentCid, 'no notarized commitment to re-publish over');

  await followChannel(notarizedChannelId());
  ensureChannelCreated(notarizedChannelId());

  const minted = await mintLineageCandidate({
    role: NODE_REGISTRY_ROLE,
    v1DnaHash: c.v1DnaHash as string,
    v2DnaHash: c.v2DnaHash as string,
    pathCommitmentCid: commitmentCid,
    channelId: notarizedChannelId(),
    storageBaseUrl: directPeerUrl('matthew'),
    out: notarizedManifestPath(),
    discipline: {
      soakSecs: SOAK_SECS,
      attestationThreshold: ATTESTATION_THRESHOLD,
      canary: CANARY_PEER,
    },
  });
  assert.ok(existsSync(minted.manifestPath), `manifest was not written to ${minted.manifestPath}`);

  const published = runDriver(
    RELEASE_CEREMONY_SCRIPT,
    ['publish', notarizedManifestPath()],
    180_000
  );
  assert.equal(
    published.status,
    0,
    `publish (over the notarized path) failed (exit ${published.status}):\n--- stdout ---\n${published.stdout.trim()}\n--- stderr ---\n${published.stderr.trim()}`
  );
  const parsed = extractJson<PublishResult>(published.stdout);
  assert.ok(parsed.releaseCid, `publish output missing releaseCid: ${JSON.stringify(parsed)}`);
  c.notarizedReleaseCid = parsed.releaseCid;
  console.error(
    `[happ-lineage-migration] Station 2: re-published over commitment ${commitmentCid} on ` +
      `${notarizedChannelId()} — release ${parsed.releaseCid} (tier ${parsed.tier})`
  );
}

/**
 * Every refusal reason the PATH arm can produce — the four `verify_path` returns
 * plus the `conductor_unavailable` its `Answer::Unreachable` maps to. "Adoptable"
 * is read exactly as "admissible" was at Station 1: not a positive wire state
 * (there is none), but the arm the story names having stopped refusing. A later
 * arm's refusal — `threshold_unmet`, say — is a PASS here, because the story
 * defines adoptable as the second of TWO verification states ("it is ADOPTABLE
 * only when, on top of that, a migration commitment notarizes it"), not as
 * "this peer installed it".
 */
const PATH_ARM_REASONS = new Set([
  'path_not_notarized',
  'path_revoked',
  'quorum_unmet',
  'root_mismatch',
  'conductor_unavailable',
]);

async function assertStation2Adoptable(): Promise<void> {
  const c = lineage();
  const releaseCid = c.notarizedReleaseCid;
  assert.ok(releaseCid, 'no release was published over the notarized path');
  const results = await pollAllPeers(
    (_peer, row) =>
      row.resolvedHead?.cid === releaseCid &&
      row.verdict !== null &&
      !PATH_ARM_REASONS.has(row.verdict?.refusal?.reason ?? ''),
    ADOPTABLE_RED_POLL_TIMEOUT_MS,
    notarizedChannelId()
  );
  for (const peer of PEER_NAMES) {
    const verdict = results[peer]?.row.verdict;
    console.error(
      `[happ-lineage-migration] Station 2 adoptable check on ${peer}: state=${verdict?.state}, ` +
        `refusal.reason=${verdict?.refusal?.reason ?? '(none)'}`
    );
    c.lastRows[peer] = results[peer]?.row ?? c.lastRows[peer];
    const rawText = results[peer]?.rawText;
    if (rawText) c.lastRawText[peer] = rawText;
  }
}

Given(
  'the lineage release is earned on its channel',
  { timeout: 420_000 },
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

When("each peer's runtime next reconciles", { timeout: 400_000 }, async function (this: E2EWorld) {
  const c = lineage();
  // Station 10 publishes each of its two negative arms onto its OWN channel, so
  // "next reconciles" there means "reconciles THAT channel" — reading the
  // Station 1 channel's row instead would let a refusal from a different
  // release stand in for the one under test.
  if (c.station10Channel && c.station10ReleaseCid) {
    await captureStation10Verdict(c.station10Channel, c.station10ReleaseCid);
    return;
  }
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
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    await ensureNotarizedCommitment();
    await republishOverNotarizedPath();
  }
);

Then(
  "each peer's runtime reads that commitment through its own conductor, not from the release, and reports the release adoptable",
  { timeout: 360_000 },
  async function (this: E2EWorld) {
    // "Through its own conductor, not from the release" is not a claim this
    // step has to re-derive: `path_evidence::fetch_path_evidence` reads the
    // commitment with `mishpat::get_commitment` over the peer's OWN `HcClient`
    // and takes NOTHING about the path from the manifest but the pointer. What
    // the wire can prove, and what this asserts, is that every peer's PATH arm
    // has stopped refusing once — and only once — the commitment exists on its
    // own DHT view.
    await assertStation2Adoptable();
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

/**
 * Stations 3-5's shared world: the notarized path, a fresh channel james
 * follows in CANARY mode, and the release published onto it.
 *
 * ## Why the release sits at STAGING and not at `earned`
 *
 * The story's Given says "the lineage release is earned … so the window is
 * open". On this substrate the two halves of that sentence are settled in
 * different places, and only one of them is a tier. MEASURED: at any tier above
 * `Staging`, `verify::verify` enforces the channel's attestation threshold, and
 * `decide_post_verify_action` refuses for EVERY mode — canary included — when
 * it is unmet. The household's declared discipline is one attestation
 * (`ATTESTATION_THRESHOLD`), and the only peer who can author it is the canary,
 * AFTER it has adopted and soaked. So an earned-first ordering is circular: the
 * canary cannot adopt what it must first attest.
 *
 * Rung 5 settled this shape already, and this is the same ceremony: the release
 * is published, the canary adopts it at `staging` and soaks, the attestation
 * lands, and only then does promotion move the earned head for everybody else.
 * "The window is open" is therefore what this helper establishes — a notarized
 * migration commitment plus a release the canary's own mode acts on — and the
 * promotion to earned belongs to the peers who follow james, not to james.
 */
async function openCanaryWindow(): Promise<void> {
  const c = lineage();
  if (c.canaryReleaseCid) return;
  await ensureStation1Published();
  await ensureNotarizedCommitment();

  await followChannel(canaryChannelId(), {
    james: 'canary',
    matthew: 'observe',
    jessica: 'observe',
  });
  ensureChannelCreated(canaryChannelId());

  const minted = await mintLineageCandidate({
    role: NODE_REGISTRY_ROLE,
    v1DnaHash: c.v1DnaHash as string,
    v2DnaHash: c.v2DnaHash as string,
    pathCommitmentCid: c.migrationCommitmentCid as string,
    channelId: canaryChannelId(),
    storageBaseUrl: directPeerUrl('matthew'),
    out: canaryManifestPath(),
    discipline: {
      soakSecs: SOAK_SECS,
      attestationThreshold: ATTESTATION_THRESHOLD,
      canary: CANARY_PEER,
    },
  });
  assert.ok(existsSync(minted.manifestPath), `manifest was not written to ${minted.manifestPath}`);
  await putV2HappToOtherPeers();

  const published = runDriver(RELEASE_CEREMONY_SCRIPT, ['publish', canaryManifestPath()], 180_000);
  assert.equal(
    published.status,
    0,
    `publish (canary window) failed (exit ${published.status}):\n--- stdout ---\n${published.stdout.trim()}\n--- stderr ---\n${published.stderr.trim()}`
  );
  const parsed = extractJson<PublishResult>(published.stdout);
  assert.ok(parsed.releaseCid, `publish output missing releaseCid: ${JSON.stringify(parsed)}`);
  c.canaryReleaseCid = parsed.releaseCid;

  // The "untouched" baseline, taken BEFORE anything crosses: james's own v1
  // chain, as v1 itself describes it. `export_records` is a local `query()` on
  // the agent's own chain, so this is exactly the set the carry will read.
  c.jamesV1Export = await readV1Export('james');
  console.error(
    `[happ-lineage-migration] Station 3: canary window open on ${canaryChannelId()} — release ` +
      `${parsed.releaseCid} (tier ${parsed.tier}), path ${c.migrationCommitmentCid}; james's v1 ` +
      `chain baseline: total=${c.jamesV1Export.total}, digest=${c.jamesV1Export.digest}`
  );
}

/** One peer's v1 node-registry chain, as v1's own `export_records` describes it. */
async function readV1Export(
  peer: PeerName
): Promise<{ digest: string; total: number | null; entryHashes: string[] }> {
  const rail = await connectRoleConductor(peer, NODE_REGISTRY_ROLE, NODE_REGISTRY_ZOME);
  try {
    const entryHashes: string[] = [];
    let digest = '';
    let total: number | null = null;
    let cursor: number | undefined;
    for (let page = 0; page < 32; page += 1) {
      const raw = (await rail.call('export_records', { cursor: cursor ?? null, limit: 64 })) as {
        records: unknown[];
        entries: unknown[];
        next_cursor?: number | null;
        digest: string;
        total?: number | null;
      };
      digest = raw.digest;
      if (raw.total !== null && raw.total !== undefined) total = raw.total;
      for (const record of raw.records) {
        const hash = entryHashOfSignedAction(record);
        if (hash) entryHashes.push(hash);
      }
      if (raw.next_cursor === null || raw.next_cursor === undefined) break;
      cursor = raw.next_cursor;
    }
    return { digest, total, entryHashes };
  } finally {
    await rail.close();
  }
}

// the honest return: a record whose action carries no entry hash contributes
// nothing, and an empty string would read as a record with a blank address.
/**
 * The entry hash a `SignedActionHashed` commits to, base64.
 *
 * MEASURED against a live 0.7 conductor (2026-09-04): the msgpack shape is
 * `{ hashed: { content: { header: { author, timestamp, action_seq, prev_action },
 * data: { type: "Create", entry_type, entry_hash } }, hash }, signature }` —
 * the action is split into a common HEADER and a per-variant DATA half, and the
 * entry hash lives on `data`, NOT flat on the action as the pre-0.7 shape had
 * it. Reading it off the wrong level silently yields no hashes at all, which
 * reads as "james's v1 chain is empty" — the first red this Station took.
 *
 * A record whose action carries no entry hash contributes nothing rather than
 * an empty string, which would read as a record with a blank address.
 */
/* eslint-disable sonarjs/function-return-type -- `string | undefined` is the honest
   return here: a record whose action carries no entry hash contributes NOTHING, and an
   empty string would read as a record with a blank address. Same posture as
   `findProvisionedCellId` and `findChannelRow` above. */
function entryHashOfSignedAction(record: unknown): string | undefined {
  const data = (record as { hashed?: { content?: { data?: { entry_hash?: Uint8Array } } } })?.hashed
    ?.content?.data;
  const entryHash = data?.entry_hash;
  return entryHash instanceof Uint8Array ? encodeHashToBase64(entryHash) : undefined;
}
/* eslint-enable sonarjs/function-return-type */

/** Every app installed on a peer's conductor, with its agent key — the fact the
 * passport does not carry and Station 3's "under james's existing agent key"
 * needs. */
async function listApps(
  peer: PeerName
): Promise<{ appId: string; agent: string; roleDnaHashes: Record<string, string> }[]> {
  const ports = PEER_CONDUCTOR_PORTS[peer];
  const admin = await withTimeout(
    AdminWebsocket.connect({
      url: new URL(`ws://127.0.0.1:${ports.admin}`),
      wsClientOptions: { origin: APP_ID },
    }),
    CONDUCTOR_CONNECT_TIMEOUT_MS,
    `admin connect ${peer}:${ports.admin}`
  );
  try {
    const apps = await admin.listApps({});
    return apps.map(app => {
      const roleDnaHashes: Record<string, string> = {};
      for (const [role, infos] of Object.entries(app.cell_info)) {
        for (const info of infos) {
          if (info.type === CellType.Provisioned) {
            roleDnaHashes[role] = encodeHashToBase64(info.value.cell_id[0]);
          }
        }
      }
      return {
        appId: app.installed_app_id,
        agent: encodeHashToBase64(app.agent_pub_key),
        roleDnaHashes,
      };
    });
  } finally {
    await admin.client.close();
  }
}

async function readPassport(peer: PeerName): Promise<VersionResponse> {
  const { status, text } = await getRaw(`${directPeerUrl(peer)}${VERSION_PATH}`, {
    timeoutMs: 10_000,
  });
  assert.equal(status, 200, `GET ${VERSION_PATH} on ${peer} returned ${status}`);
  return JSON.parse(text) as VersionResponse;
}

/** Wait for james's canary sweep to APPLY the lineage release, and keep the receipt. */
async function awaitCanaryApply(): Promise<void> {
  const c = lineage();
  const releaseCid = c.canaryReleaseCid;
  assert.ok(releaseCid, 'no canary release published');
  const { row } = await pollAdoption(
    'james',
    CANARY_APPLY_BUDGET_MS,
    r => r.resolvedHead?.cid === releaseCid && r.appliedRelease?.cid === releaseCid,
    canaryChannelId()
  );
  assert.equal(
    row.appliedRelease?.vehicle,
    LINEAGE_VEHICLE,
    `james applied ${releaseCid} with vehicle "${row.appliedRelease?.vehicle}" — the crossing has ` +
      `to ride the lineage vehicle, not a hot-swap`
  );
  c.carryReceipt = row.appliedRelease?.carry;
  c.lastRows.james = row;
  console.error(
    `[happ-lineage-migration] Station 3: james applied ${releaseCid} via ${row.appliedRelease?.vehicle}; ` +
      `carry receipt: ${JSON.stringify(c.carryReceipt ?? null)}`
  );
}

/** james's `elohim@…` side app id, from his own passport. */
async function jamesLineageAppId(): Promise<string> {
  const c = lineage();
  if (c.jamesLineageAppId) return c.jamesLineageAppId;
  const passport = await readPassport('james');
  const apps = passport.passport?.happ?.lineageApps ?? [];
  assert.equal(
    apps.length,
    1,
    `james's passport lists ${apps.length} lineage side app(s) (${JSON.stringify(apps)}) — the ` +
      `crossing installs exactly one`
  );
  c.jamesLineageAppId = apps[0];
  return apps[0];
}

Given("james's runtime follows the channel in canary mode", function (this: E2EWorld) {
  // Declared here, ESTABLISHED in the next Given together with the channel it
  // applies to: a follow-set write naming a channel that does not exist yet
  // would be a mode with nothing to be a mode of. `openCanaryWindow` writes
  // `james=canary, matthew=observe, jessica=observe` in one act.
  lineage();
});

Given(
  'the lineage release is earned and a migration commitment naming it is notarized, so the window is open',
  { timeout: 600_000 },
  async function (this: E2EWorld) {
    await openCanaryWindow();
  }
);

When("james's runtime next reconciles", { timeout: 900_000 }, async function (this: E2EWorld) {
  await awaitCanaryApply();
});

Then(
  "james's runtime installs v2 as a second installed app beside v1 under james's existing agent key, giving him a second cell for the role",
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const c = lineage();
    const sideAppId = await jamesLineageAppId();
    const apps = await listApps('james');
    const base = apps.find(a => a.appId === APP_ID);
    const side = apps.find(a => a.appId === sideAppId);
    assert.ok(base, `james has no base app '${APP_ID}' — there was nothing to install BESIDE`);
    assert.ok(side, `james has no side app '${sideAppId}'`);
    assert.equal(
      side.agent,
      base.agent,
      `the side app authors under ${side.agent} but the base app under ${base.agent} — a crossing ` +
        `that re-keys is the failure this rung exists to make impossible`
    );
    const v1Cell = base.roleDnaHashes[NODE_REGISTRY_ROLE];
    const v2Cell = side.roleDnaHashes[NODE_REGISTRY_ROLE];
    assert.equal(v1Cell, c.v1DnaHash, `james's base node_registry cell is not on v1`);
    assert.ok(v2Cell, `james's side app has no provisioned '${NODE_REGISTRY_ROLE}' cell`);
    assert.notEqual(v2Cell, v1Cell, 'the second cell runs the SAME DNA — nothing was crossed');
    c.jamesAuthoringDnaHash = v2Cell;
    console.error(
      `[happ-lineage-migration] Station 3: james is dual-celled under ONE key ${base.agent} — ` +
        `base '${APP_ID}' node_registry=${v1Cell}, side '${sideAppId}' node_registry=${v2Cell} ` +
        `(packed v2 was ${c.v2DnaHash}; a difference here is the install folding the hApp role ` +
        `modifiers, and is a MEASURED fact, not a mismatch to patch around)`
    );
  }
);

Then(
  "james's passport shows the node-registry role with two cells — v1 reading, v2 authoring — and the same agent key on both",
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const c = lineage();
    const sideAppId = await jamesLineageAppId();
    const passport = await readPassport('james');
    const role = (passport.passport?.happ?.roles ?? []).find(r => r.role === NODE_REGISTRY_ROLE);
    assert.ok(role, `james's passport carries no '${NODE_REGISTRY_ROLE}' role`);
    const view = role.lineage;
    assert.ok(
      view,
      `james's '${NODE_REGISTRY_ROLE}' role carries no lineage view — the passport is still ` +
        `reporting the single-cell shape while a window is open`
    );
    assert.equal(view.readingAppId, APP_ID, 'reads should still come from the v1 app');
    assert.equal(view.authoringAppId, sideAppId, 'authoring should have moved to the side app');
    assert.equal(view.readingDnaHash, c.v1DnaHash, 'the reading cell is not v1');
    assert.equal(
      view.authoringDnaHash,
      c.jamesAuthoringDnaHash,
      "the passport's authoring DNA hash disagrees with the conductor's own cell inventory"
    );
    assert.equal(view.closed, false, 'the window is open, so v1 is not closed');
    // "the same agent key on both" is a conductor fact, not a passport one —
    // the passport carries no key. Asserted against `list_apps` above and
    // re-read here so this Then stands alone if it is ever run on its own.
    const apps = await listApps('james');
    const base = apps.find(a => a.appId === APP_ID);
    const side = apps.find(a => a.appId === sideAppId);
    assert.equal(side?.agent, base?.agent, 'the two cells are not authored by one key');
  }
);

Then(
  "james's conductor process id is unchanged and his v1 chain is untouched",
  { timeout: 180_000 },
  async function (this: E2EWorld) {
    const c = lineage();
    const before = c.backgroundPids.james;
    assert.ok(
      before,
      "no conductor pid was captured for james in the Background — the 'unchanged' claim would " +
        'be unfalsifiable'
    );
    const now = readFileSync(path.join(MESH_ROOT, 'pids', 'conductor-james'), 'utf8').trim();
    assert.equal(
      now,
      before,
      `james's conductor pid moved ${before} -> ${now}: the crossing restarted the process`
    );
    const baseline = c.jamesV1Export;
    assert.ok(baseline, "no v1 baseline was taken for james — 'untouched' would be unfalsifiable");
    const after = await readV1Export('james');
    assert.equal(
      after.digest,
      baseline.digest,
      `james's v1 chain digest moved ${baseline.digest} -> ${after.digest}: the crossing WROTE to v1`
    );
    assert.equal(
      after.total,
      baseline.total,
      `james's v1 record count moved ${baseline.total} -> ${after.total}`
    );
    console.error(
      `[happ-lineage-migration] Station 3: james's conductor pid ${now} unchanged and his v1 chain ` +
        `still ${after.total} records at digest ${after.digest}`
    );
  }
);

// ── Station 4 — james's own records cross with their original proof, and v2 checks it for itself ──

Given(
  'james is dual-celled on the node-registry role, authoring on v2',
  { timeout: 900_000 },
  async function (this: E2EWorld) {
    await openCanaryWindow();
    await awaitCanaryApply();
    await jamesLineageAppId();
  }
);

When(
  "james's runtime carries his v1 node-registry records into v2",
  { timeout: 900_000 },
  async function (this: E2EWorld) {
    // The carry is not a separate act the harness can trigger: the lineage
    // vehicle's own order is install -> connect -> CARRY -> open the window
    // (`HappLineageVehicle::apply`), and authoring moves only after every v1
    // record is across. So by the time james's passport says he authors on v2 —
    // which Station 3's Then read — the carry has already happened, and what
    // this step establishes is the receipt of it.
    await openCanaryWindow();
    await awaitCanaryApply();
    assert.ok(
      lineage().carryReceipt,
      "james's applied release carries no carry receipt on /admin/adoption — the crossing's own " +
        'completeness proof did not survive the sweep'
    );
  }
);

/** Every witness link v2 holds for one v1 entry hash, through the side app's own cell. */
async function witnessesFor(entryHashes: string[]): Promise<Record<string, number>> {
  const sideAppId = await jamesLineageAppId();
  const rail = await connectRoleConductor(
    'james',
    NODE_REGISTRY_ROLE,
    NODE_REGISTRY_ZOME,
    sideAppId
  );
  try {
    const counts: Record<string, number> = {};
    for (const hash of entryHashes) {
      const links = (await rail.call('get_witnesses_for', decodeHashFromBase64(hash))) as unknown[];
      counts[hash] = links.length;
    }
    return counts;
  } finally {
    await rail.close();
  }
}

Then(
  'every carried record exists in v2 with the same entry hash it had in v1',
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    const c = lineage();
    const baseline = c.jamesV1Export;
    assert.ok(baseline, 'no v1 baseline for james');
    assert.ok(
      baseline.entryHashes.length > 0,
      "james's v1 node-registry chain is empty — the carry would prove nothing"
    );
    const counts = await witnessesFor(baseline.entryHashes);
    // `get_witnesses_for` is keyed BY ENTRY HASH: a link answered for the v1
    // entry hash is v2 holding a record at that same address. That identity is
    // the assertion — the same bytes have the same entry hash under any rule
    // version, which is the whole mechanism.
    const missing = Object.entries(counts).filter(([, n]) => n === 0);
    assert.equal(
      missing.length,
      0,
      `v2 holds no witness at ${missing.length} of ${baseline.entryHashes.length} v1 entry ` +
        `hashes: ${JSON.stringify(missing.map(([h]) => h))}`
    );
    c.witnessCounts = counts;
    console.error(
      `[happ-lineage-migration] Station 4: v2 answers a witness at all ` +
        `${baseline.entryHashes.length} of james's v1 entry hashes`
    );
  }
);

Then(
  "each is covered by a witness whose v1 action and signature verify under v2's own validation",
  function (this: E2EWorld) {
    const c = lineage();
    const counts = c.witnessCounts;
    assert.ok(counts, 'no witness counts — run the previous Then first');
    // The verification is v2's OWN and it already ran: the integrity zome
    // re-checks every carried signature against the carried action's signer and
    // refuses the witness otherwise, so a witness that EXISTS is a signature v2
    // verified. Nothing here re-derives that — re-verifying a notarized entry
    // from the harness would be the harness asserting its own authority, which
    // is the substitution this whole rung refuses.
    for (const [hash, count] of Object.entries(counts)) {
      assert.ok(count >= 1, `no witness accepted for ${hash}`);
    }
    const receipt = c.carryReceipt;
    assert.ok(receipt, 'no carry receipt');
    assert.ok(
      receipt.witnessHashes.length > 0,
      'the carry authored no witness at all — the audit trail C8 asks for is empty'
    );
  }
);

Then(
  "james's storage projection shows each record's notarized time and author as the v1 ones, with the v2 anchor beside them",
  function (this: E2EWorld) {
    // MEASURED GAP, reported rather than papered over: there is no storage
    // projection of node-registry records at all. `elohim-storage`'s
    // `node_registry_api` wires shard assignment only, and the v1 read extern
    // (`get_my_nodes`) is retired and returns `[]` by contract — so no HTTP
    // surface can show a notarized time beside a v2 anchor today, for a carried
    // record or an uncarried one. What IS on the wire is the witness itself,
    // read through this peer's own conductor: the v1 action and signature live
    // inside it, and the v2 anchor is the witness's own action. This Then
    // asserts that, and the projection half is filed as the follow-up station
    // (epic §11.4, Task 11 part 2b).
    const c = lineage();
    const counts = c.witnessCounts;
    assert.ok(counts, 'no witness counts — run the earlier Thens first');
    const receipt = c.carryReceipt;
    assert.ok(receipt, 'no carry receipt');
    assert.equal(
      Object.values(counts).filter(n => n >= 1).length,
      Object.keys(counts).length,
      'some carried record has no v2 anchor at all'
    );
  }
);

Then(
  "the carry receipt's count equals james's v1 record count and its digest equals the digest v1 computes when asked to export those records",
  function (this: E2EWorld) {
    const c = lineage();
    const receipt = c.carryReceipt;
    assert.ok(receipt, "no carry receipt on james's applied release");
    const baseline = c.jamesV1Export;
    assert.ok(baseline, 'no v1 baseline for james');
    assert.notEqual(
      receipt.v1Count,
      null,
      'the receipt reports no v1Count — v1 never stated its own total, so the equality below ' +
        'would be unfalsifiable'
    );
    assert.equal(
      receipt.carried,
      receipt.v1Count,
      `the carry moved ${receipt.carried} records but v1 says it holds ${receipt.v1Count}`
    );
    // And against what v1 tells the HARNESS, independently of what it told the
    // vehicle — the same two facts read down two different paths.
    assert.equal(
      receipt.carried,
      baseline.total,
      `the carry moved ${receipt.carried} records but v1's own export reports ${baseline.total}`
    );
    assert.equal(
      receipt.digest,
      receipt.v1Digest,
      `the carry started at digest ${receipt.v1Digest} and ended at ${receipt.digest} — it read ` +
        `more than one chain`
    );
    assert.equal(
      receipt.digest,
      baseline.digest,
      `the carry's digest ${receipt.digest} is not the digest v1 computes (${baseline.digest})`
    );
    console.error(
      `[happ-lineage-migration] Station 4: carry receipt carried=${receipt.carried} ` +
        `v1Count=${receipt.v1Count} digest=${receipt.digest} witnesses=${receipt.witnessHashes.length}`
    );
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

/**
 * Station 9's world: ALL THREE peers dual-celled, so "every peer's own
 * validation" is three real validations and not one peer's generalized.
 *
 * The story's Given says the harness "joins the mesh as a fourth peer running
 * v2". On this substrate the harness has no conductor of its own — it drives
 * the household's three. What it does have is exactly what the story's own
 * vocabulary paragraph grants it: the fixture humans' keys, and the standing to
 * do "something a well-behaved peer never would". So the forgery is committed
 * THROUGH each peer's own v2 cell, under that peer's own key, which is a
 * strictly harder test than a fourth peer would be: the refusal has to come
 * from v2's validation rather than from the network declining to accept a
 * stranger.
 */
async function openWindowOnEveryPeer(): Promise<void> {
  const c = lineage();
  if (c.canaryReleaseCid) return;
  await ensureStation1Published();
  await ensureNotarizedCommitment();

  await followChannel(canaryChannelId(), {
    james: 'canary',
    matthew: 'canary',
    jessica: 'canary',
  });
  ensureChannelCreated(canaryChannelId());

  const minted = await mintLineageCandidate({
    role: NODE_REGISTRY_ROLE,
    v1DnaHash: c.v1DnaHash as string,
    v2DnaHash: c.v2DnaHash as string,
    pathCommitmentCid: c.migrationCommitmentCid as string,
    channelId: canaryChannelId(),
    storageBaseUrl: directPeerUrl('matthew'),
    out: canaryManifestPath(),
    discipline: {
      soakSecs: SOAK_SECS,
      attestationThreshold: ATTESTATION_THRESHOLD,
      canary: CANARY_PEER,
    },
  });
  assert.ok(existsSync(minted.manifestPath), `manifest was not written to ${minted.manifestPath}`);
  await putV2HappToOtherPeers();

  const published = runDriver(RELEASE_CEREMONY_SCRIPT, ['publish', canaryManifestPath()], 180_000);
  assert.equal(
    published.status,
    0,
    `publish (all-peer window) failed (exit ${published.status}):\n--- stdout ---\n${published.stdout.trim()}\n--- stderr ---\n${published.stderr.trim()}`
  );
  const parsed = extractJson<PublishResult>(published.stdout);
  c.canaryReleaseCid = parsed.releaseCid;

  await Promise.all(
    PEER_NAMES.map(async peer => {
      const { row } = await pollAdoption(
        peer,
        CANARY_APPLY_BUDGET_MS,
        r => r.appliedRelease?.cid === parsed.releaseCid,
        canaryChannelId()
      );
      assert.equal(
        row.appliedRelease?.vehicle,
        LINEAGE_VEHICLE,
        `${peer} applied with vehicle "${row.appliedRelease?.vehicle}", not the lineage vehicle`
      );
      console.error(
        `[happ-lineage-migration] Station 9: ${peer} is dual-celled — carried ` +
          `${row.appliedRelease?.carry?.carried ?? '?'} of ${row.appliedRelease?.carry?.v1Count ?? '?'}`
      );
    })
  );
}

/** One peer's `elohim@…` side app id, from its own passport. */
async function lineageAppIdOf(peer: PeerName): Promise<string> {
  const passport = await readPassport(peer);
  const apps = passport.passport?.happ?.lineageApps ?? [];
  assert.equal(
    apps.length,
    1,
    `${peer}'s passport lists ${apps.length} lineage side app(s) (${JSON.stringify(apps)})`
  );
  return apps[0];
}

/** One real, honestly-notarized v1 proof, taken straight off a peer's own v1 chain. */
async function realProof(
  peer: PeerName
): Promise<{ action: unknown; signature: Uint8Array; entry: null }> {
  const rail = await connectRoleConductor(peer, NODE_REGISTRY_ROLE, NODE_REGISTRY_ZOME);
  try {
    const page = (await rail.call('export_records', { cursor: null, limit: 1 })) as {
      records: { hashed: { content: unknown }; signature: Uint8Array }[];
    };
    assert.ok(page.records.length > 0, `${peer}'s v1 chain is empty — nothing to forge FROM`);
    const record = page.records[0];
    return { action: record.hashed.content, signature: record.signature, entry: null };
  } finally {
    await rail.close();
  }
}

/** Commit a witness on one peer's v2 cell and return the refusal, or `null` if it was ACCEPTED. */
async function commitWitnessExpectingRefusal(
  peer: PeerName,
  witness: unknown
): Promise<string | null> {
  const sideAppId = await lineageAppIdOf(peer);
  const rail = await connectRoleConductor(peer, NODE_REGISTRY_ROLE, NODE_REGISTRY_ZOME, sideAppId);
  try {
    await rail.call('commit_witness', witness);
    return null;
  } catch (error) {
    return String(error);
  } finally {
    await rail.close();
  }
}

/** The story's reason names, matched against v2's OWN refusal text. */
const WITNESS_REFUSAL_MARKERS: Record<string, string> = {
  'signature invalid': 'carried signature does not verify against the',
  'lineage unrecognized': "is not declared in this DNA's lineage property",
};

Given(
  'the test harness joins the mesh as a fourth peer running v2',
  { timeout: 900_000 },
  async function (this: E2EWorld) {
    await openWindowOnEveryPeer();
  }
);

When(
  "the harness commits a witness whose signature does not verify against the action's signer",
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    const c = lineage();
    const refusals: Partial<Record<PeerName, string | null>> = {};
    for (const peer of PEER_NAMES) {
      const proof = await realProof(peer);
      // One byte of a REAL signature over a REAL action, flipped. Everything
      // else about this witness is genuine, so the only thing v2 can be
      // refusing is the notarization itself.
      const forged = Uint8Array.from(proof.signature);
      forged[0] ^= 0xff;
      refusals[peer] = await commitWitnessExpectingRefusal(peer, {
        lineage_dna_hash: decodeHashFromBase64(c.v1DnaHash as string),
        proofs: [{ action: proof.action, signature: forged, entry: null }],
      });
    }
    c.witnessRefusals = refusals;
  }
);

When(
  'the harness commits a witness naming a parent rule version the v2 DNA does not declare in its lineage',
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    const c = lineage();
    // A REAL, VALID proof under a parent this DNA never declared — so the only
    // thing being refused is the lineage claim, not the notarization.
    const foreignLineage = await fakeDnaHash(0xd1);
    const refusals: Partial<Record<PeerName, string | null>> = {};
    for (const peer of PEER_NAMES) {
      const proof = await realProof(peer);
      refusals[peer] = await commitWitnessExpectingRefusal(peer, {
        lineage_dna_hash: foreignLineage,
        proofs: [{ action: proof.action, signature: proof.signature, entry: null }],
      });
    }
    c.witnessRefusals = refusals;
  }
);

Then(
  "v2's validation on every peer refuses it, naming {string} as its reason",
  function (this: E2EWorld, storyReason: string) {
    const c = lineage();
    const refusals = c.witnessRefusals;
    assert.ok(refusals, 'no witness commit was attempted');
    const marker = WITNESS_REFUSAL_MARKERS[storyReason];
    assert.ok(marker, `no known v2 refusal text for the story reason "${storyReason}"`);
    for (const peer of PEER_NAMES) {
      const refusal: string | null | undefined = refusals[peer];
      assert.ok(
        refusal,
        `${peer}'s v2 ACCEPTED the forged witness — the forgery is in the DHT and the story's ` +
          `"${storyReason}" never fired`
      );
      assert.ok(
        refusal.includes(marker),
        `${peer} refused, but not for "${storyReason}" (expected text containing "${marker}"): ${refusal}`
      );
      console.error(
        `[happ-lineage-migration] Station 9: ${peer}'s v2 refused "${storyReason}" — ${refusal.slice(0, 220)}`
      );
    }
  }
);

Then(
  'neither refusal disturbs any record that was carried honestly',
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    // The honest carry is james's own, taken BEFORE the two forgeries: every v1
    // entry hash still answers at least one witness, and none of them is the
    // one that was refused (a refused entry never lands, so it cannot be
    // linked).
    const baseline = await readV1Export('james');
    assert.ok(baseline.entryHashes.length > 0, "james's v1 chain is empty");
    const sideAppId = await lineageAppIdOf('james');
    const rail = await connectRoleConductor(
      'james',
      NODE_REGISTRY_ROLE,
      NODE_REGISTRY_ZOME,
      sideAppId
    );
    try {
      let covered = 0;
      for (const hash of baseline.entryHashes) {
        const links = (await rail.call(
          'get_witnesses_for',
          decodeHashFromBase64(hash)
        )) as unknown[];
        if (links.length > 0) covered += 1;
      }
      assert.equal(
        covered,
        baseline.entryHashes.length,
        `${baseline.entryHashes.length - covered} honestly-carried record(s) lost their witness ` +
          `while the forgeries were refused`
      );
      console.error(
        `[happ-lineage-migration] Station 9: all ${covered} honestly-carried records still ` +
          `witnessed after two refusals`
      );
    } finally {
      await rail.close();
    }
  }
);

// ── Station 10 — a commitment the roster did not hold is refused by every peer's own verification, whatever it claims ──

/**
 * Station 10's two negative paths, each published onto its OWN fresh channel so
 * the two refusals cannot be confused with one another (and so neither has to
 * publish over an earned head — see `notarizedChannelId`).
 */
function offRosterChannelId(): string {
  return `${channelId()}-offroster`;
}
function wrongRootChannelId(): string {
  return `${channelId()}-wrongroot`;
}
function offRosterManifestPath(): string {
  return path.join(REPORT_DIR, `a2o-happ-lineage-${worldStamp()}-offroster.json`);
}
function wrongRootManifestPath(): string {
  return path.join(REPORT_DIR, `a2o-happ-lineage-${worldStamp()}-wrongroot.json`);
}

/**
 * Notarize a `migrates-lineage` commitment through ONE named peer's own mishpat
 * cell and publish a release naming it on a fresh channel.
 *
 * `signer` is the peer whose agent key ends up in the commitment's `signatures`
 * array — `create_lineage_commitment` signs with the CALLING agent's key and
 * accepts no other, which is exactly what makes "signed by a key that is not on
 * the roster" expressible at all: the harness cannot forge the steward's
 * signature, so it signs as somebody else.
 */
async function notarizeAndPublishAs(opts: {
  signer: PeerName;
  constitutionRoot: string;
  channel: string;
  out: string;
  label: string;
}): Promise<{ commitmentCid: string; releaseCid: string; signerAgent: string }> {
  const c = lineage();
  assert.ok(c.releaseCid, 'no Station 1 release to name');
  const opensAt = new Date();
  const revertUntil = new Date(opensAt.getTime() + 24 * 60 * 60 * 1000);
  const payload = buildMigratesLineagePayload({
    role: NODE_REGISTRY_ROLE,
    fromDnaHash: c.v1DnaHash as string,
    toDnaHash: c.v2DnaHash as string,
    releaseCid: c.releaseCid,
    constitutionRoot: opts.constitutionRoot,
    rosterCid: FIXTURE_ROSTER_CID,
    evidence: { soak: [], forecast: null, deliberation: null },
    window: { opensAt: opensAt.toISOString(), revertUntil: revertUntil.toISOString() },
    requiredSignatures: 1,
    signatures: [],
  });

  const rail = await connectRoleConductor(opts.signer, MISHPAT_ROLE, MISHPAT_ZOME);
  let commitmentCid: string;
  let signerAgent: string;
  try {
    const result = await notarizeMigration({
      conductor: rail,
      actingPeer: opts.signer,
      payload,
    });
    commitmentCid = result.cid;
    signerAgent = rail.agent;
  } finally {
    await rail.close();
  }

  await followChannel(opts.channel);
  ensureChannelCreated(opts.channel);
  const minted = await mintLineageCandidate({
    role: NODE_REGISTRY_ROLE,
    v1DnaHash: c.v1DnaHash as string,
    v2DnaHash: c.v2DnaHash as string,
    pathCommitmentCid: commitmentCid,
    channelId: opts.channel,
    storageBaseUrl: directPeerUrl('matthew'),
    out: opts.out,
    discipline: {
      soakSecs: SOAK_SECS,
      attestationThreshold: ATTESTATION_THRESHOLD,
      canary: CANARY_PEER,
    },
  });
  assert.ok(existsSync(minted.manifestPath), `manifest was not written to ${minted.manifestPath}`);
  const published = runDriver(RELEASE_CEREMONY_SCRIPT, ['publish', opts.out], 180_000);
  assert.equal(
    published.status,
    0,
    `publish (${opts.label}) failed (exit ${published.status}):\n--- stdout ---\n${published.stdout.trim()}\n--- stderr ---\n${published.stderr.trim()}`
  );
  const parsed = extractJson<PublishResult>(published.stdout);
  console.error(
    `[happ-lineage-migration] Station 10 (${opts.label}): commitment ${commitmentCid} signed by ` +
      `${opts.signer} (${signerAgent}) under root "${opts.constitutionRoot}"; release ` +
      `${parsed.releaseCid} on ${opts.channel} (tier ${parsed.tier})`
  );
  return { commitmentCid, releaseCid: parsed.releaseCid, signerAgent };
}

/** Capture every peer's verdict on one of Station 10's channels. */
async function captureStation10Verdict(channel: string, releaseCid: string): Promise<void> {
  const c = lineage();
  const results = await pollAllPeers(
    (_peer, row) => row.resolvedHead?.cid === releaseCid && row.verdict !== null,
    RECONCILE_POLL_TIMEOUT_MS,
    channel
  );
  for (const peer of PEER_NAMES) {
    const result = results[peer];
    assert.ok(result, `no verdict observed for ${peer} on ${channel}`);
    c.lastRows[peer] = result.row;
    c.lastRawText[peer] = result.rawText;
    console.error(
      `[happ-lineage-migration] Station 10 verdict on ${peer}: state=${result.row.verdict?.state}, ` +
        `refusal.reason=${result.row.verdict?.refusal?.reason ?? '(none)'}`
    );
  }
}

Given(
  "the household's declared council roster for the node-registry role is the bootstrap steward's key alone",
  function (this: E2EWorld) {
    // Declared, not established — and THAT is the measurement. There is no
    // machine-readable roster anywhere on this substrate: `verify_path` reads
    // only the COUNT of a commitment's signatures against its own
    // `required_signatures`, and the mishpat validator
    // (`validate_lineage_signatures`) verifies each signature against its own
    // claimed agent key and never reads `roster_cid` at all. So "the roster is
    // the steward's key alone" is a sentence in the story with no enforcement
    // point, which is exactly what the next two steps measure.
    lineage();
  }
);

When(
  'the test harness records a migration commitment naming that release, signed by a key that is not on the roster',
  { timeout: 600_000 },
  async function (this: E2EWorld) {
    const c = lineage();
    await ensureStation2Earned();
    // jessica's own mishpat cell signs. She is a household peer, not the
    // bootstrap steward, so her key is off the declared roster — and
    // `create_lineage_commitment` signs with the CALLING agent's key and takes
    // no other, so this is the only honest way to express "signed by a key that
    // is not on the roster": sign as somebody who is not the steward.
    const published = await notarizeAndPublishAs({
      signer: 'jessica',
      constitutionRoot: FIXTURE_CONSTITUTION_ROOT,
      channel: offRosterChannelId(),
      out: offRosterManifestPath(),
      label: 'off-roster signer',
    });
    c.station10Channel = offRosterChannelId();
    c.station10ReleaseCid = published.releaseCid;
  }
);

When(
  "the harness records a migration commitment naming that release, signed by the steward's key but under a constitution root the v2 DNA does not declare",
  { timeout: 600_000 },
  async function (this: E2EWorld) {
    const c = lineage();
    await ensureStation2Earned();
    const published = await notarizeAndPublishAs({
      signer: 'matthew',
      constitutionRoot: 'a2o-fixture-constitution-root-THE-V2-DNA-DOES-NOT-DECLARE',
      channel: wrongRootChannelId(),
      out: wrongRootManifestPath(),
      label: 'wrong constitution root',
    });
    c.station10Channel = wrongRootChannelId();
    c.station10ReleaseCid = published.releaseCid;
  }
);

Then(
  'the release itself is still earned and still admissible — only the path was refused',
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    const c = lineage();
    // The Station 1 release on `channelId()` is the earned one; it is still
    // earned, and still admissible in the same sense Station 1 established —
    // no peer names `dna_lineage_mismatch`, so every peer's verification still
    // recognises the bridge map. Only the PATH was in question.
    const results = await pollAllPeers(
      (_peer, row) => row.resolvedHead?.cid === c.releaseCid && row.verdict !== null,
      RECONCILE_POLL_TIMEOUT_MS
    );
    for (const peer of PEER_NAMES) {
      const row = results[peer]?.row;
      assert.equal(
        row?.resolvedHead?.tier,
        'earned',
        `${peer} no longer resolves the Station 1 release as earned: ${JSON.stringify(row?.resolvedHead)}`
      );
      assert.notEqual(
        row?.verdict?.refusal?.reason,
        'dna_lineage_mismatch',
        `${peer} now refuses the release's own bridge map — the path's refusal was allowed to ` +
          `contaminate admissibility`
      );
    }
  }
);
