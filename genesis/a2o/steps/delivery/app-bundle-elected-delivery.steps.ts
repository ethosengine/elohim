/**
 * App bundle as elected content — step definitions for
 * `features/delivery/app-bundle-elected-delivery.feature`
 * (elected-content spec §12.8 slice 2; native-delivery sprint Lane N6).
 *
 * The story drives the REAL release path end to end on the household mesh:
 *
 *   epr-release-package.ts --artifact-class app-bundle --put-via doorway   (four blob PUTs, ONE doorway)
 *   release-ceremony.ts publish --transport doorway                         (one staging version)
 *   each peer's release-adoption controller (canary)                        (verify: shells boot → AppBundleVehicle)
 *   release-ceremony.ts attestations / promote / revert                     (the existing verbs)
 *
 * and asserts only what each peer REPORTS about itself (`GET /db/content/<app>`,
 * `GET /admin/adoption`) and what a doorway SERVES (`/apps/<app>/index.html`).
 *
 * ## Run-owned, never the household's real apps
 *
 * Two apps and one channel are minted per run (`a2o-app-<stamp>-a|b`,
 * `runtime:app-bundle:household:a2o-<stamp>`), each app bound to the channel
 * in its own record at creation. Pointing the household's real landing page at
 * fixture bytes would deface it, and the earned-head guard would refuse a
 * foreign head on it anyway. The follow entries this run adds are removed in
 * `AfterAll`.
 *
 * ## Preconditions the run cannot manufacture
 *
 * - The storage binary on every peer carries the `app-bundle` class (Lane N;
 *   an older controller refuses the release `manifest_schema_invalid`).
 * - The release-manifest schema the packager validates against carries the
 *   class. Until the rakia pin moves, set `ELOHIM_RAKIA_ROOT` to a rakia tree
 *   on `feat/app-bundle-class` (the packager refuses to emit what its schema
 *   cannot validate, which is the honest failure).
 *
 * ## Cross-scenario state
 *
 * Stations are one causal chain (station 2 adopts what station 1 published),
 * held in module state and walked by idempotent `ensure*` functions, the same
 * shape `runtime-upgrade-propagation.steps.ts` uses.
 */

/* eslint-disable sonarjs/no-os-command-from-path --
   this file deliberately shells out to `zip` and to the two release drivers through the
   workspace's tsx — the composition the story exists to prove. */

import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

import { AfterAll, Given, Then, When } from '@cucumber/cucumber';

import { getRaw, postRaw } from '../../src/framework/dataplane/surfaces.js';
import { householdMeshDir } from '../../src/framework/fixtures/household-mesh.js';
import {
  buildFixtureBundle,
  buildServerFixture,
  pollUntil,
  removeFixtureBundle,
  stageBundleThroughStorage,
  visitInBrowser,
  type FixtureBundle,
} from '../dataplane/epr-app-deliverability.helpers.js';

import type { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const A2O_ROOT = fileURLToPath(new URL('../../', import.meta.url));
const REPO_ROOT = fileURLToPath(new URL('../../../../', import.meta.url));
const TSX = path.join(REPO_ROOT, 'node_modules/.bin/tsx');

const PEERS = ['matthew', 'jessica', 'james'] as const;
type PeerName = (typeof PEERS)[number];

const RUN_STAMP = `${Date.now().toString(36)}${process.pid.toString(36)}`;
const CHANNEL_ID = `runtime:app-bundle:household:a2o-${RUN_STAMP}`;
const APPS = [`a2o-app-${RUN_STAMP}-a`, `a2o-app-${RUN_STAMP}-b`] as const;

/** One soak, short enough for a story, long enough to be a soak. */
const SOAK_SECS = 20;
/** The household is three devices of one kind: one attester (james) earns a release. */
const ATTESTATION_THRESHOLD = 1;
/** A peer's sweep, an artifact pull and the vehicle — bounded like the runtime story's apply. */
const ADOPT_BUDGET_MS = 300_000;
/** The doorway's bundle-heads tick is 30 s; twice that plus slack. */
const SERVE_BUDGET_MS = 75_000;
const ATTEST_BUDGET_MS = Math.max((SOAK_SECS + 65) * 1000, 90_000);

const FOLLOW_PATH = '/admin/runtime-config/follow';
const RELEASE_CEREMONY_SCRIPT = 'release-ceremony.ts';
const ADOPTION_PATH = '/admin/adoption';

// ---------------------------------------------------------------------------
// Run state
// ---------------------------------------------------------------------------

/** One app's two halves in one build. */
interface AppBuild {
  browser: FixtureBundle;
  server: FixtureBundle;
  browserZip: string;
  serverZip: string;
  /** `sha256-<hex>` of each zip — the spelling every app record carries. */
  browserHash: string;
  serverHash: string;
}

/** One release this run published: its manifest, its cid, and the bytes per app. */
interface Release {
  label: string;
  manifestPath: string;
  cid: string;
  builds: Record<string, AppBuild>;
  /** The packager's stderr — the blob round-trip lines are the upload receipt. */
  packagerLog: string;
  ceremonyOutput: Record<string, unknown>;
}

const run: {
  workDir: string;
  appsAuthored: boolean;
  following: Set<PeerName>;
  earlier?: Release;
  current?: Release;
  reverted?: Release;
  broken?: Release;
  /** Per peer, every sampled (appA-new, appB-new) pair while station 2 waited. */
  samples: Record<PeerName, [boolean, boolean][]>;
  /** The retention story's three releases, in the order they were published. */
  kept: Release[];
  /** Peers whose retention depth this run set, to be put back afterwards. */
  retentionSet: Set<PeerName>;
} = {
  workDir: mkdtempSync(path.join(tmpdir(), `a2o-app-bundle-${RUN_STAMP}-`)),
  appsAuthored: false,
  following: new Set<PeerName>(),
  samples: { matthew: [], jessica: [], james: [] },
  kept: [],
  retentionSet: new Set<PeerName>(),
};

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

function storageUrl(peer: PeerName): string {
  const envVar = `E2E_STORAGE_${peer.toUpperCase()}`;
  const url = process.env[envVar];
  assert.ok(url, `${envVar} is not set — the household lane exports it (just test mesh)`);
  return url.replace(/\/$/, '');
}

function doorwayUrl(world: E2EWorld, name: string): string {
  return world.getDoorway(name).url.replace(/\/$/, '');
}

function adminKey(): string {
  return process.env['STORAGE_API_KEY_ADMIN'] ?? 'mesh-admin-dev-key';
}

function sha256File(file: string): string {
  return createHash('sha256').update(readFileSync(file)).digest('hex');
}

function zipDir(dir: string, out: string): string {
  mkdirSync(path.dirname(out), { recursive: true });
  const zipped = spawnSync('zip', ['-X', '-qr', out, '.'], { cwd: dir, encoding: 'utf8' });
  assert.equal(zipped.status, 0, `zip ${dir}: ${zipped.stderr}`);
  return out;
}

interface DriverResult {
  status: number | null;
  stdout: string;
  stderr: string;
}

function runDriver(script: string, args: string[], timeoutMs = 180_000): DriverResult {
  const result = spawnSync(TSX, [path.join(A2O_ROOT, 'scripts', script), ...args], {
    cwd: A2O_ROOT,
    encoding: 'utf8',
    timeout: timeoutMs,
    env: { ...process.env, STORAGE_API_KEY_ADMIN: adminKey() },
  });
  return { status: result.status, stdout: result.stdout ?? '', stderr: result.stderr ?? '' };
}

/** The last top-level JSON object a driver printed (drivers log a line before it). */
function lastJson(stdout: string): Record<string, unknown> {
  const start = stdout.lastIndexOf('\n{');
  const text = start >= 0 ? stdout.slice(start + 1) : stdout.slice(stdout.indexOf('{'));
  return JSON.parse(text) as Record<string, unknown>;
}

async function contentRow(peer: PeerName, id: string): Promise<Record<string, unknown> | null> {
  const { status, text } = await getRaw(`${storageUrl(peer)}/db/content/${id}`, {
    timeoutMs: 10_000,
  });
  if (status !== 200) return null;
  return JSON.parse(text) as Record<string, unknown>;
}

interface AdoptionRow {
  channelId: string;
  appliedRelease: { cid: string; vehicle?: string } | null;
  verdict?: {
    state?: string;
    releaseCid?: string;
    refusal?: { reason: string; detail: string; transient: boolean };
  } | null;
}

async function adoptionRow(peer: PeerName): Promise<AdoptionRow | undefined> {
  const { status, text } = await getRaw(`${storageUrl(peer)}${ADOPTION_PATH}`, {
    timeoutMs: 10_000,
  });
  if (status !== 200) return undefined;
  const body = JSON.parse(text) as { channels?: AdoptionRow[] };
  return (body.channels ?? []).find(row => row.channelId === CHANNEL_ID);
}

/** Does this peer's record for `app` name exactly `build`'s two halves? */
async function recordNames(peer: PeerName, app: string, build: AppBuild): Promise<boolean> {
  const row = await contentRow(peer, app);
  return row?.['blobHash'] === build.browserHash && row?.['serverBlobHash'] === build.serverHash;
}

async function channelHead(peer: PeerName): Promise<Record<string, unknown> | null> {
  const { status, text } = await getRaw(`${storageUrl(peer)}/db/content/${CHANNEL_ID}/head`, {
    timeoutMs: 10_000,
  });
  return status === 200 ? (JSON.parse(text) as Record<string, unknown>) : null;
}

// ---------------------------------------------------------------------------
// Building and packaging releases
// ---------------------------------------------------------------------------

function buildApps(label: string, brokenApp?: string): Record<string, AppBuild> {
  const builds: Record<string, AppBuild> = {};
  for (const app of APPS) {
    const browser = buildFixtureBundle({ coherent: app !== brokenApp, baseHref: `/apps/${app}/` });
    const server = buildServerFixture(browser);
    const browserZip = zipDir(browser.dir, path.join(run.workDir, label, app, 'browser.zip'));
    const serverZip = zipDir(server.dir, path.join(run.workDir, label, app, 'server.zip'));
    builds[app] = {
      browser,
      server,
      browserZip,
      serverZip,
      browserHash: `sha256-${sha256File(browserZip)}`,
      serverHash: `sha256-${sha256File(serverZip)}`,
    };
  }
  return builds;
}

/**
 * Package one app-bundle release through ONE doorway. `threshold` is the
 * release's own adoption discipline: 0 only for the steward's direct earned
 * declarations (the baseline and the revert), where no attestation is asked.
 */
function packageRelease(
  world: E2EWorld,
  label: string,
  builds: Record<string, AppBuild>,
  lineageParent: string | null,
  threshold: number
): { manifestPath: string; packagerLog: string } {
  const manifestPath = path.join(run.workDir, `${label}.manifest.json`);
  const appArgs = APPS.flatMap(app => [
    '--app-artifact',
    `${app}:browser:${builds[app].browserZip}`,
    '--app-artifact',
    `${app}:server:${builds[app].serverZip}`,
    '--app-mount',
    `${app}=/apps/${app}/`,
  ]);
  const packaged = runDriver('epr-release-package.ts', [
    '--artifact-class',
    'app-bundle',
    '--channel-id',
    CHANNEL_ID,
    ...appArgs,
    '--soak-secs',
    String(SOAK_SECS),
    '--attestation-threshold',
    String(threshold),
    '--canary',
    'james',
    '--builder-agent',
    'a2o-app-bundle-story',
    '--peer',
    doorwayUrl(world, 'alpha'),
    '--put-via',
    'doorway',
    ...(lineageParent ? ['--lineage-parent', lineageParent] : ['--first-release']),
    '--strict',
    '--out',
    manifestPath,
  ]);
  assert.equal(
    packaged.status,
    0,
    `packaging ${label} failed (exit ${packaged.status}) — if the schema refused the class, set ` +
      `ELOHIM_RAKIA_ROOT to a rakia tree carrying app-bundle:\n${packaged.stderr.slice(-2000)}`
  );
  return { manifestPath, packagerLog: packaged.stderr };
}

function publishStaging(world: E2EWorld, manifestPath: string): Record<string, unknown> {
  const published = runDriver(RELEASE_CEREMONY_SCRIPT, [
    'publish',
    manifestPath,
    '--transport',
    'doorway',
    '--doorway',
    doorwayUrl(world, 'alpha'),
  ]);
  assert.equal(published.status, 0, `publish failed:\n${published.stdout}\n${published.stderr}`);
  return lastJson(published.stdout);
}

/** The steward's direct EARNED declaration of a manifest (the baseline, and the revert). */
function declareEarned(manifestPath: string): Record<string, unknown> {
  const declared = runDriver(RELEASE_CEREMONY_SCRIPT, ['revert', CHANNEL_ID, manifestPath]);
  assert.equal(
    declared.status,
    0,
    `earned declaration failed:\n${declared.stdout}\n${declared.stderr}`
  );
  return lastJson(declared.stdout);
}

async function currentHeadCid(): Promise<string | null> {
  const head = await channelHead('matthew');
  return typeof head?.['headActionHash'] === 'string' ? head['headActionHash'] : null;
}

async function waitForRecords(release: Release, sample = false): Promise<void> {
  const elapsed = await pollUntil(
    async () => {
      let all = true;
      for (const peer of PEERS) {
        const states = await Promise.all(
          APPS.map(async app => recordNames(peer, app, release.builds[app]))
        );
        if (sample) run.samples[peer].push([states[0], states[1]]);
        if (!states.every(Boolean)) all = false;
      }
      return all;
    },
    ADOPT_BUDGET_MS,
    1_000
  );
  if (elapsed === null) {
    const detail: string[] = [];
    for (const peer of PEERS) {
      const row = await adoptionRow(peer);
      detail.push(`${peer}: ${JSON.stringify(row?.verdict ?? row ?? 'no row')}`);
    }
    assert.fail(
      `within ${ADOPT_BUDGET_MS / 1000}s not every peer's records named release ${release.label} ` +
        `(${release.cid}):\n  ${detail.join('\n  ')}`
    );
  }
}

// ---------------------------------------------------------------------------
// The causal chain
// ---------------------------------------------------------------------------

async function ensureAppsAuthored(): Promise<void> {
  if (run.appsAuthored) return;
  const created = runDriver(RELEASE_CEREMONY_SCRIPT, [
    'channel',
    'create',
    CHANNEL_ID,
    '--reach',
    'commons',
    '--discipline',
    JSON.stringify({
      soakSecs: SOAK_SECS,
      attestationThreshold: ATTESTATION_THRESHOLD,
      canaryOrder: ['james'],
    }),
  ]);
  assert.equal(created.status, 0, `channel create failed:\n${created.stdout}\n${created.stderr}`);

  // Each app is BORN bound: its own record names the channel.
  const response = await fetch(`${storageUrl('matthew')}/db/content/bulk`, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      'x-schema-version': '1',
      'x-api-key': adminKey(),
    },
    body: JSON.stringify(
      APPS.map(id => ({
        id,
        title: `App bundle story fixture (${id})`,
        description: 'an app this test run owns, elected by its release channel',
        contentType: 'collective',
        contentFormat: 'html5-app',
        content: { slug: id, entryPoint: 'index.html' },
        reach: 'commons',
        metadata: { releaseChannel: CHANNEL_ID },
      }))
    ),
  });
  assert.ok(response.ok, `authoring the run's apps: ${response.status} ${await response.text()}`);

  // Every peer must hold both records before a release can move them.
  const everywhere = await pollUntil(
    async () => {
      for (const peer of PEERS) {
        for (const app of APPS) {
          const row = await contentRow(peer, app);
          const metadata = (row?.['metadata'] ?? {}) as Record<string, unknown>;
          if (metadata['releaseChannel'] !== CHANNEL_ID) return false;
        }
      }
      return true;
    },
    ADOPT_BUDGET_MS,
    2_000
  );
  assert.notEqual(everywhere, null, 'the run-owned app records did not reach every household peer');
  run.appsAuthored = true;
}

async function ensureFollowing(): Promise<void> {
  for (const peer of PEERS) {
    if (run.following.has(peer)) continue;
    const { status, text } = await postRaw(`${storageUrl(peer)}${FOLLOW_PATH}`, {
      channel: CHANNEL_ID,
      mode: 'canary',
    });
    assert.ok(status >= 200 && status < 300, `${peer} follow: ${status} ${text.slice(0, 300)}`);
    run.following.add(peer);
  }
}

async function ensureEarlierEarned(world: E2EWorld): Promise<void> {
  if (run.earlier) return;
  await ensureAppsAuthored();
  await ensureFollowing();
  const builds = buildApps('earlier');
  // The first release on a channel this run just created names no parent. The
  // channel's content head at this point is its root entry, not a release, and
  // every peer refuses a declared parent that is not a release on the channel
  // (`lineage_parent_mismatch`, household run 2026-09-26).
  const { manifestPath, packagerLog } = packageRelease(world, 'earlier', builds, null, 0);
  const declared = declareEarned(manifestPath);
  run.earlier = {
    label: 'earlier',
    manifestPath,
    cid: String(declared['releaseCid']),
    builds,
    packagerLog,
    ceremonyOutput: declared,
  };
  await waitForRecords(run.earlier);
}

async function ensureStaged(world: E2EWorld): Promise<void> {
  if (run.current) return;
  await ensureEarlierEarned(world);
  const builds = buildApps('current');
  const { manifestPath, packagerLog } = packageRelease(
    world,
    'current',
    builds,
    run.earlier?.cid ?? null,
    ATTESTATION_THRESHOLD
  );
  const published = publishStaging(world, manifestPath);
  run.current = {
    label: 'current',
    manifestPath,
    cid: String(published['releaseCid']),
    builds,
    packagerLog,
    ceremonyOutput: published,
  };
}

async function ensureAdopted(world: E2EWorld): Promise<void> {
  await ensureStaged(world);
  const current = run.current as Release;
  await waitForRecords(current, true);
}

/** Qualifying attestations for a release, read BY CID off james's conductor (the builder excluded). */
function qualifyingAttestations(releaseCid: string): number {
  const read = runDriver(RELEASE_CEREMONY_SCRIPT, [
    'attestations',
    releaseCid,
    '--as',
    'james',
    '--builder',
    'matthew',
  ]);
  if (read.status !== 0) return 0;
  const report = lastJson(read.stdout) as { qualifying?: number };
  return report.qualifying ?? 0;
}

async function ensurePromoted(world: E2EWorld): Promise<void> {
  await ensureAdopted(world);
  const current = run.current as Release;
  const head = await channelHead('matthew');
  if (head?.['headActionHash'] === current.cid && (head?.['stagingCandidate'] ?? null) === null)
    return;
  const attested = await pollUntil(
    async () => Promise.resolve(qualifyingAttestations(current.cid) >= ATTESTATION_THRESHOLD),
    ATTEST_BUDGET_MS,
    5_000
  );
  assert.notEqual(attested, null, `james's attestation for ${current.cid} never counted`);
  const promoted = runDriver(RELEASE_CEREMONY_SCRIPT, ['promote', CHANNEL_ID, current.cid]);
  assert.equal(promoted.status, 0, `promote failed:\n${promoted.stdout}\n${promoted.stderr}`);
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

Given(
  "two apps this run owns, each bound in its own record to this run's release channel",
  { timeout: 360_000 },
  async function () {
    await ensureAppsAuthored();
  }
);

Given(
  'every household peer follows that channel as a canary',
  { timeout: 60_000 },
  async function () {
    await ensureFollowing();
  }
);

Given(
  "the channel's earned head is an earlier release of both apps",
  { timeout: 600_000 },
  async function (this: E2EWorld) {
    await ensureEarlierEarned(this);
  }
);

// ---------------------------------------------------------------------------
// Station 1 — publish once, at staging
// ---------------------------------------------------------------------------

Given('this run has built a new browser bundle and a new server bundle for each app', function () {
  // Built inside `ensureStaged`, with the publish, so a standalone station
  // never publishes bytes it did not build in the same breath.
});

When(
  'matthew publishes them as one release through doorway {string}',
  { timeout: 360_000 },
  async function (this: E2EWorld, doorway: string) {
    assert.equal(doorway, 'alpha', 'this story publishes through doorway "alpha"');
    await ensureStaged(this);
  }
);

Then(
  'the release is staged on the channel, beneath the earned head, not earned itself',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const current = run.current as Release;
    const earlier = run.earlier as Release;
    assert.equal(current.ceremonyOutput['tier'], 'staging');
    const ok = await pollUntil(
      async () => {
        const head = await channelHead('matthew');
        return (
          head?.['headActionHash'] === earlier.cid && head?.['stagingCandidate'] === current.cid
        );
      },
      60_000,
      2_000
    );
    assert.notEqual(
      ok,
      null,
      `matthew's channel head should stay ${earlier.cid} with ${current.cid} staged beneath it: ` +
        JSON.stringify(await channelHead('matthew'))
    );
  }
);

Then(
  "the files reached the household through doorway {string} alone, and nothing was written onto any peer's app record",
  function (this: E2EWorld, doorway: string) {
    const current = run.current as Release;
    const origin = doorwayUrl(this, doorway);
    const roundTrips = current.packagerLog
      .split('\n')
      .filter(line => line.startsWith('round-trip '));
    assert.equal(
      roundTrips.length,
      APPS.length * 2,
      `one upload per (app, kind):\n${current.packagerLog}`
    );
    assert.equal(current.ceremonyOutput['transport'], 'doorway');
    assert.equal(current.ceremonyOutput['doorway'], origin);
    // The only content this run's publish touched is the CHANNEL: the packager
    // uploads blobs, and the ceremony writes one version on the channel id.
    // No app id appears in either driver's writes.
    for (const app of APPS) {
      assert.ok(
        !JSON.stringify(current.ceremonyOutput).includes(app),
        `the publish named app ${app} — it may only write the channel`
      );
    }
  }
);

// ---------------------------------------------------------------------------
// Station 2 — every peer takes the release up by itself
// ---------------------------------------------------------------------------

Given(
  "matthew's new release is staged on the channel",
  { timeout: 600_000 },
  async function (this: E2EWorld) {
    await ensureStaged(this);
  }
);

When(
  "each household peer's runtime next looks at the channel",
  { timeout: 600_000 },
  async function (this: E2EWorld) {
    await ensureAdopted(this);
  }
);

Then(
  "on matthew's, jessica's, and james's peers each app's record names the release's browser bundle and its server bundle",
  { timeout: 120_000 },
  async function () {
    const current = run.current as Release;
    for (const peer of PEERS) {
      for (const app of APPS) {
        assert.ok(
          await recordNames(peer, app, current.builds[app]),
          `${peer}'s record for ${app}: ${JSON.stringify(await contentRow(peer, app))}`
        );
      }
      const row = await adoptionRow(peer);
      assert.equal(
        row?.appliedRelease?.cid,
        current.cid,
        `${peer} applied: ${JSON.stringify(row)}`
      );
    }
  }
);

Then(
  "throughout the take-up, no peer was ever seen naming one app's new build beside the other app's old one",
  function () {
    for (const peer of PEERS) {
      const mixed = run.samples[peer].filter(([a, b]) => a !== b);
      assert.equal(
        mixed.length,
        0,
        `${peer} was sampled ${mixed.length} time(s) with one app new and the other old ` +
          `(of ${run.samples[peer].length} samples)`
      );
      assert.ok(run.samples[peer].length > 0, `${peer} was never sampled`);
    }
  }
);

// ---------------------------------------------------------------------------
// Station 3 — a visitor is served the new build
// ---------------------------------------------------------------------------

Given(
  "every household peer has taken up matthew's new release",
  { timeout: 600_000 },
  async function (this: E2EWorld) {
    await ensureAdopted(this);
  }
);

When('a visitor asks doorway {string} for each app', function (this: E2EWorld, doorway: string) {
  doorwayUrl(this, doorway);
});

Then(
  "within 75 seconds the page each app is served names the new browser bundle's entry script",
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const current = run.current as Release;
    const base = doorwayUrl(this, 'alpha');
    for (const app of APPS) {
      const entry = current.builds[app].browser.entryScript;
      const served = await pollUntil(
        async () => {
          const { status, text } = await getRaw(`${base}/apps/${app}/index.html`, {
            timeoutMs: 10_000,
          });
          return status === 200 && text.includes(entry);
        },
        SERVE_BUDGET_MS,
        3_000
      );
      assert.notEqual(served, null, `doorway alpha never served ${app} naming ${entry}`);
    }
  }
);

Then(
  'a browser opening the first app on doorway {string} starts it without an error',
  { timeout: 120_000 },
  async function (this: E2EWorld, doorway: string) {
    const visit = await visitInBrowser(`${doorwayUrl(this, doorway)}/apps/${APPS[0]}/`);
    assert.deepEqual(visit.pageErrors, [], `uncaught errors: ${visit.pageErrors.join('; ')}`);
    assert.ok(visit.rootText.length > 0, 'the app root stayed empty — the app did not start');
  }
);

// ---------------------------------------------------------------------------
// Station 4 — attest and promote
// ---------------------------------------------------------------------------

Given(
  "james's peer has run matthew's new release",
  { timeout: 600_000 },
  async function (this: E2EWorld) {
    await ensureAdopted(this);
  }
);

When(
  "james's peer attests that the release ran clean on his device",
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    // Attestation and promotion are one ensure: a promotion without the
    // attestation is refused by the ceremony's own floor.
    await ensurePromoted(this);
  }
);

When(
  'matthew promotes the release on that attestation',
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    await ensurePromoted(this);
  }
);

Then('the release is the earned head of the channel', { timeout: 120_000 }, async function () {
  const current = run.current as Release;
  const earned = await pollUntil(
    async () => (await channelHead('matthew'))?.['headActionHash'] === current.cid,
    60_000,
    2_000
  );
  assert.notEqual(
    earned,
    null,
    `the channel head is not ${current.cid}: ${JSON.stringify(await channelHead('matthew'))}`
  );
});

Then(
  'every household peer still names the same bundles for both apps, because promotion moved no files',
  { timeout: 60_000 },
  async function () {
    const current = run.current as Release;
    for (const peer of PEERS) {
      for (const app of APPS) {
        assert.ok(
          await recordNames(peer, app, current.builds[app]),
          `${peer}/${app} moved on promotion`
        );
      }
    }
  }
);

// ---------------------------------------------------------------------------
// Station 5 — revert by re-election
// ---------------------------------------------------------------------------

Given(
  "matthew's new release is the channel's earned head",
  { timeout: 900_000 },
  async function (this: E2EWorld) {
    await ensurePromoted(this);
  }
);

When(
  'matthew reverts the channel to the earlier release',
  { timeout: 600_000 },
  async function (this: E2EWorld) {
    const earlier = run.earlier as Release;
    // A revert re-elects the earlier BYTES as a new version on top of the
    // current head, declared earned by the steward (threshold 0: nobody attests
    // a release the household is being asked to leave).
    const { manifestPath, packagerLog } = packageRelease(
      this,
      'reverted',
      earlier.builds,
      await currentHeadCid(),
      0
    );
    const declared = declareEarned(manifestPath);
    run.reverted = {
      label: 'reverted',
      manifestPath,
      cid: String(declared['releaseCid']),
      builds: earlier.builds,
      packagerLog,
      ceremonyOutput: declared,
    };
  }
);

When(
  "each household peer's runtime next looks at the channel and takes the earlier release up itself",
  { timeout: 600_000 },
  async function () {
    // Nobody writes the revert onto a peer: each runtime's own sweep applies it.
    await waitForRecords(run.reverted as Release);
  }
);

Then(
  "on matthew's, jessica's, and james's peers each app's record names the earlier release's bundles again",
  { timeout: 600_000 },
  async function () {
    await waitForRecords(run.reverted as Release);
  }
);

Then(
  "within 75 seconds doorway {string} serves each app's earlier page",
  { timeout: 120_000 },
  async function (this: E2EWorld, doorway: string) {
    const earlier = run.earlier as Release;
    const base = doorwayUrl(this, doorway);
    for (const app of APPS) {
      const entry = earlier.builds[app].browser.entryScript;
      const served = await pollUntil(
        async () => {
          const { status, text } = await getRaw(`${base}/apps/${app}/index.html`, {
            timeoutMs: 10_000,
          });
          return status === 200 && text.includes(entry);
        },
        SERVE_BUDGET_MS,
        3_000
      );
      assert.notEqual(served, null, `doorway ${doorway} never served ${app}'s earlier page`);
    }
  }
);

// ---------------------------------------------------------------------------
// Negative — a build that cannot boot
// ---------------------------------------------------------------------------

/** What every peer's records named before the broken publish. */
const beforeBroken: Partial<Record<PeerName, Record<string, unknown>>> = {};

Given(
  "every household peer has taken up the channel's earned head",
  { timeout: 900_000 },
  async function (this: E2EWorld) {
    await ensureEarlierEarned(this);
    const settled = run.reverted ?? run.current ?? run.earlier;
    if (settled === run.current) await ensurePromoted(this);
    await waitForRecords(settled as Release);
    for (const peer of PEERS) {
      beforeBroken[peer] = Object.fromEntries(
        await Promise.all(
          APPS.map(async app => {
            const row = await contentRow(peer, app);
            return [app, { blobHash: row?.['blobHash'], serverBlobHash: row?.['serverBlobHash'] }];
          })
        )
      );
    }
  }
);

When(
  "matthew publishes a build whose first app's page names an entry script its browser bundle does not contain",
  { timeout: 360_000 },
  async function (this: E2EWorld) {
    const builds = buildApps('broken', APPS[0]);
    const { manifestPath, packagerLog } = packageRelease(
      this,
      'broken',
      builds,
      await currentHeadCid(),
      ATTESTATION_THRESHOLD
    );
    const published = publishStaging(this, manifestPath);
    run.broken = {
      label: 'broken',
      manifestPath,
      cid: String(published['releaseCid']),
      builds,
      packagerLog,
      ceremonyOutput: published,
    };
  }
);

Then(
  "matthew's, jessica's, and james's peers each refuse the release as a bundle that cannot boot, naming the missing file",
  { timeout: 600_000 },
  async function () {
    const broken = run.broken as Release;
    const missing = broken.builds[APPS[0]].browser.entryScript;
    for (const peer of PEERS) {
      const refused = await pollUntil(
        async () => {
          const row = await adoptionRow(peer);
          return (
            row?.verdict?.state === 'refused' &&
            row.verdict.refusal?.reason === 'app_bundle_cannot_boot' &&
            (row.verdict.refusal?.detail ?? '').includes(missing)
          );
        },
        ADOPT_BUDGET_MS,
        3_000
      );
      assert.notEqual(
        refused,
        null,
        `${peer} did not refuse ${broken.cid} as app_bundle_cannot_boot naming ${missing}: ` +
          JSON.stringify(await adoptionRow(peer))
      );
    }
  }
);

Then("no peer's record for either app moved", { timeout: 60_000 }, async function () {
  for (const peer of PEERS) {
    for (const app of APPS) {
      const row = await contentRow(peer, app);
      assert.deepEqual(
        { blobHash: row?.['blobHash'], serverBlobHash: row?.['serverBlobHash'] },
        beforeBroken[peer]?.[app],
        `${peer}'s record for ${app} moved on a release that cannot boot`
      );
    }
  }
});

// ---------------------------------------------------------------------------
// Teardown — this run's follow entries come off every peer
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Release retention (features/delivery/release-retention.feature)
// ---------------------------------------------------------------------------

const RETENTION_KEY = 'ELOHIM_RELEASE_RETENTION_DEPTH';
const RETENTION_BUDGET_MS = 15 * 60_000;

/**
 * Set (or, with `null`, clear) one of a peer's own settings: rewrite the key in
 * the runtime-config file the peer watches, then ask the peer to re-read it.
 * Every other line of the file is left as it was.
 */
async function setRuntimeSetting(peer: PeerName, key: string, value: number | null): Promise<void> {
  const file = path.join(householdMeshDir(), peer, 'runtime-config.toml');
  const kept = existsSync(file)
    ? readFileSync(file, 'utf8')
        .split('\n')
        .filter(line => !line.trimStart().startsWith(key))
    : [];
  while (kept.length > 0 && kept.at(-1) === '') kept.pop();
  if (value !== null) kept.push(`${key} = ${value}`);
  writeFileSync(file, `${kept.join('\n')}\n`);
  const reloaded = await postRaw(`${storageUrl(peer)}/admin/runtime-config/reload`);
  assert.equal(reloaded.status, 200, `${peer} did not reload its settings: ${reloaded.text}`);
}

/** What a peer is running one setting at right now, read from the peer itself. */
async function runtimeSetting(peer: PeerName, key: string): Promise<unknown> {
  const { status, text } = await getRaw(`${storageUrl(peer)}/admin/runtime-config`, {
    timeoutMs: 10_000,
  });
  assert.equal(status, 200, `${peer} /admin/runtime-config answered ${status}`);
  const body = JSON.parse(text) as { settings?: { name: string; effectiveValue: unknown }[] };
  return (body.settings ?? []).find(setting => setting.name === key)?.effectiveValue;
}

async function setRetentionDepth(peer: PeerName, depth: number | null): Promise<void> {
  await setRuntimeSetting(peer, RETENTION_KEY, depth);
}

async function retentionDepth(peer: PeerName): Promise<unknown> {
  return runtimeSetting(peer, RETENTION_KEY);
}

interface RetentionChannel {
  held: string[];
  released: string[];
  pastWindowHeld: number;
}

/** What a peer's last retention pass says about this run's channel. */
async function retentionReport(peer: PeerName): Promise<{
  channel?: RetentionChannel;
  blobsHeld?: Record<string, number>;
}> {
  const { status, text } = await getRaw(`${storageUrl(peer)}${ADOPTION_PATH}`, {
    timeoutMs: 10_000,
  });
  if (status !== 200) return {};
  const body = JSON.parse(text) as {
    retention?: {
      channels?: Record<string, RetentionChannel>;
      blobsHeld?: Record<string, number>;
    } | null;
  };
  return {
    channel: body.retention?.channels?.[CHANNEL_ID],
    blobsHeld: body.retention?.blobsHeld,
  };
}

/** Where a peer's blob store keeps one bundle, read straight off its disk. */
function bundleFile(peer: PeerName, hash: string): string {
  const hex = hash.replace(/^sha256-/, '');
  return path.join(householdMeshDir(), peer, 'blobs', 'blobs', hex.slice(0, 4), `sha256-${hex}`);
}

function bundleHashes(release: Release): string[] {
  return APPS.flatMap(app => [release.builds[app].browserHash, release.builds[app].serverHash]);
}

Given(
  "matthew's peer is set to keep the two latest releases of each channel",
  { timeout: 30_000 },
  async function () {
    await setRetentionDepth('matthew', 2);
    run.retentionSet.add('matthew');
    assert.equal(await retentionDepth('matthew'), 2);
  }
);

Given("jessica's peer keeps the default of ten", { timeout: 30_000 }, async function () {
  assert.equal(await retentionDepth('jessica'), 10);
});

When(
  'matthew publishes three releases of both apps, one after another, and every peer takes each up before the next is published',
  { timeout: 1_200_000 },
  async function (this: E2EWorld) {
    await ensureAppsAuthored();
    await ensureFollowing();
    while (run.kept.length < 3) {
      const label = ['first', 'second', 'third'][run.kept.length];
      const builds = buildApps(`kept-${label}`);
      // A release names the one before it on the channel; the first names none.
      const parent = run.kept.at(-1)?.cid ?? null;
      const { manifestPath, packagerLog } = packageRelease(
        this,
        `kept-${label}`,
        builds,
        parent,
        ATTESTATION_THRESHOLD
      );
      const published = publishStaging(this, manifestPath);
      const release: Release = {
        label,
        manifestPath,
        cid: String(published['releaseCid']),
        builds,
        packagerLog,
        ceremonyOutput: published,
      };
      await waitForRecords(release);
      run.kept.push(release);
    }
  }
);

Then(
  "within 15 minutes matthew's peer's own account of the channel lists the second and third releases as kept and no longer lists the first",
  { timeout: RETENTION_BUDGET_MS + 60_000 },
  async function () {
    const [first, second, third] = run.kept;
    const settled = await pollUntil(
      async () => {
        const { channel } = await retentionReport('matthew');
        return channel !== undefined && !channel.held.includes(first.cid);
      },
      RETENTION_BUDGET_MS,
      10_000
    );
    const report = await retentionReport('matthew');
    assert.notEqual(
      settled,
      null,
      `matthew's peer still keeps the first release ${first.cid} after 15 minutes: ` +
        JSON.stringify(report)
    );
    assert.deepEqual(
      report.channel?.held,
      [third.cid, second.cid],
      `matthew's peer should keep exactly the third and second releases, newest first: ` +
        JSON.stringify(report)
    );
  }
);

Then("the first release's bundle files, for both apps, are gone from matthew's peer", function () {
  for (const hash of bundleHashes(run.kept[0])) {
    const file = bundleFile('matthew', hash);
    assert.ok(!existsSync(file), `matthew's peer still holds ${file}`);
  }
});

Then(
  "the second and third releases' bundle files, for both apps, are still on matthew's peer",
  function () {
    for (const release of [run.kept[1], run.kept[2]]) {
      for (const hash of bundleHashes(release)) {
        const file = bundleFile('matthew', hash);
        assert.ok(existsSync(file), `matthew's peer let go of ${release.label}'s ${file}`);
      }
    }
  }
);

Then("the first release's bundle files, for both apps, are still on jessica's peer", function () {
  for (const hash of bundleHashes(run.kept[0])) {
    const file = bundleFile('jessica', hash);
    assert.ok(existsSync(file), `jessica's peer no longer holds ${file}`);
  }
});

Then(
  "doorway {string} still serves the third release's build of each app",
  { timeout: 120_000 },
  async function (this: E2EWorld, doorway: string) {
    const third = run.kept[2];
    const base = doorwayUrl(this, doorway);
    for (const app of APPS) {
      const entry = third.builds[app].browser.entryScript;
      const served = await pollUntil(
        async () => {
          const { status, text } = await getRaw(`${base}/apps/${app}/index.html`, {
            timeoutMs: 10_000,
          });
          return status === 200 && text.includes(entry);
        },
        SERVE_BUDGET_MS,
        3_000
      );
      assert.notEqual(served, null, `doorway ${doorway} is not serving ${app} naming ${entry}`);
    }
  }
);

// ---------------------------------------------------------------------------
// An earlier build that nothing names (release-retention.feature, scenario 2)
// ---------------------------------------------------------------------------

const PASS_SECONDS_KEY = 'ELOHIM_RELEASE_RETENTION_PASS_SECONDS';
const UNNAMED_AGE_KEY = 'ELOHIM_UNNAMED_MIN_AGE_SECONDS';
/** The app whose record names its bundle directly: bound to no channel. */
const SOLO_APP = `a2o-app-${RUN_STAMP}-solo`;
const QUICK_PEERS: PeerName[] = ['matthew', 'jessica'];
const FIRST_BUILD_MISSING = 'the first build must be handed over first';

interface SoloBuild {
  bundle: FixtureBundle;
  /** `sha256-<hex>` of the zip the peer was handed. */
  hash: string;
}

const solo: {
  authored: boolean;
  first?: SoloBuild;
  second?: SoloBuild;
  /** A file handed to matthew's peer by a route that records nothing. */
  unrecorded?: string;
  quickened: Set<PeerName>;
} = { authored: false, quickened: new Set<PeerName>() };

interface HoldsAccount {
  blobs?: Record<string, { blobs: number; bytes: number }>;
  ownUnnamed?: { blobHash: string; arrivedVia: string[]; passes: number }[];
  recentlyLetGo?: { blobHash: string }[];
}

/** A peer's own account of why it holds what is in its blob store. */
async function holdsAccount(peer: PeerName): Promise<HoldsAccount | null> {
  const { status, text } = await getRaw(`${storageUrl(peer)}${ADOPTION_PATH}`, {
    timeoutMs: 10_000,
  });
  if (status !== 200) return null;
  return (JSON.parse(text) as { holds?: HoldsAccount | null }).holds ?? null;
}

/** Hand matthew's peer one build of the solo app and point its record at it. */
async function handSoloBuild(stepTimeoutMs: number): Promise<SoloBuild> {
  process.env['STORAGE_API_KEY_ADMIN'] ??= adminKey();
  const bundle = buildFixtureBundle({ coherent: true, baseHref: `/apps/${SOLO_APP}/` });
  const staged = await stageBundleThroughStorage({
    bundle,
    slug: SOLO_APP,
    storageUrl: storageUrl('matthew'),
    stepTimeoutMs,
  });
  assert.equal(staged.code, 0, `matthew's peer did not take the build: ${staged.output}`);
  const build = { bundle, hash: staged.blobHash };
  // jessica's own record of the app must name it before she can be asked for it.
  const reached = await pollUntil(
    async () => (await contentRow('jessica', SOLO_APP))?.['blobHash'] === build.hash,
    ADOPT_BUDGET_MS,
    2_000
  );
  assert.notEqual(reached, null, `jessica's record of ${SOLO_APP} never named ${build.hash}`);
  return build;
}

Given(
  'one more app this run owns, whose record names its bundle directly and which is bound to no channel',
  { timeout: ADOPT_BUDGET_MS + 30_000 },
  async function () {
    if (solo.authored) return;
    const response = await fetch(`${storageUrl('matthew')}/db/content/bulk`, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        'x-schema-version': '1',
        'x-api-key': adminKey(),
      },
      body: JSON.stringify([
        {
          id: SOLO_APP,
          title: `Earlier-build story fixture (${SOLO_APP})`,
          description: 'an app this test run owns, whose record names its bundle directly',
          contentType: 'collective',
          contentFormat: 'html5-app',
          content: { slug: SOLO_APP, entryPoint: 'index.html' },
          reach: 'commons',
        },
      ]),
    });
    assert.ok(response.ok, `authoring ${SOLO_APP}: ${response.status} ${await response.text()}`);
    const everywhere = await pollUntil(
      async () => {
        for (const peer of PEERS) {
          if ((await contentRow(peer, SOLO_APP)) === null) return false;
        }
        return true;
      },
      ADOPT_BUDGET_MS,
      2_000
    );
    assert.notEqual(everywhere, null, `${SOLO_APP} did not reach every household peer`);
    solo.authored = true;
  }
);

Given(
  "matthew's peer and jessica's peer are set to check every 15 seconds and to let an own file go once it is a minute old and nothing names it",
  { timeout: 60_000 },
  async function () {
    for (const peer of QUICK_PEERS) {
      await setRuntimeSetting(peer, PASS_SECONDS_KEY, 15);
      await setRuntimeSetting(peer, UNNAMED_AGE_KEY, 60);
      solo.quickened.add(peer);
      assert.equal(await runtimeSetting(peer, PASS_SECONDS_KEY), 15);
      assert.equal(await runtimeSetting(peer, UNNAMED_AGE_KEY), 60);
    }
  }
);

Given(
  "matthew's peer holds a file with no note of how it arrived",
  { timeout: 30_000 },
  async function () {
    // The direct shard route stores bytes and writes nothing about them.
    const bytes = Buffer.from(`a2o file with no note of how it arrived (${RUN_STAMP})`);
    const hash = `sha256-${createHash('sha256').update(bytes).digest('hex')}`;
    const response = await fetch(`${storageUrl('matthew')}/shard/${hash}`, {
      method: 'PUT',
      headers: { 'content-type': 'application/octet-stream', 'x-api-key': adminKey() },
      body: new Uint8Array(bytes),
      signal: AbortSignal.timeout(20_000),
    });
    assert.ok(response.ok, `handing matthew's peer the file: ${response.status}`);
    assert.ok(
      existsSync(bundleFile('matthew', hash)),
      `matthew's peer does not hold ${bundleFile('matthew', hash)}`
    );
    solo.unrecorded = hash;
  }
);

When(
  "matthew hands his peer a first build of that app and points the app's record at it",
  { timeout: ADOPT_BUDGET_MS + 240_000 },
  async function () {
    solo.first = await handSoloBuild(180_000);
    assert.ok(existsSync(bundleFile('matthew', solo.first.hash)));
  }
);

When(
  "jessica's peer serves that app once, fetching the first build to do so",
  { timeout: ADOPT_BUDGET_MS + 30_000 },
  async function () {
    assert.ok(solo.first, FIRST_BUILD_MISSING);
    const entry = solo.first.bundle.entryScript;
    const served = await pollUntil(
      async () => {
        const { status, text } = await getRaw(
          `${storageUrl('jessica')}/apps/${SOLO_APP}/index.html`,
          { timeoutMs: 20_000 }
        );
        return status === 200 && text.includes(entry);
      },
      ADOPT_BUDGET_MS,
      3_000
    );
    assert.notEqual(served, null, `jessica's peer never served ${SOLO_APP} naming ${entry}`);
    assert.ok(
      existsSync(bundleFile('jessica', solo.first.hash)),
      `jessica's peer served the first build without holding ${solo.first.hash}`
    );
  }
);

When(
  "matthew hands his peer a second build of that app and points the app's record at that instead",
  { timeout: ADOPT_BUDGET_MS + 240_000 },
  async function () {
    assert.ok(solo.first, FIRST_BUILD_MISSING);
    solo.second = await handSoloBuild(180_000);
    assert.notEqual(solo.second.hash, solo.first.hash, 'the two builds must differ');
  }
);

Then(
  "within 15 minutes matthew's peer's own account has listed the first build as an own file that nothing names, and then as let go",
  { timeout: RETENTION_BUDGET_MS + 60_000 },
  async function () {
    assert.ok(solo.first, FIRST_BUILD_MISSING);
    const first = solo.first.hash;
    let listedAs: string[] | undefined;
    const letGo = await pollUntil(
      async () => {
        const account = await holdsAccount('matthew');
        listedAs ??= account?.ownUnnamed?.find(blob => blob.blobHash === first)?.arrivedVia;
        return (account?.recentlyLetGo ?? []).some(blob => blob.blobHash === first);
      },
      RETENTION_BUDGET_MS,
      3_000
    );
    const account = await holdsAccount('matthew');
    assert.notEqual(
      letGo,
      null,
      `matthew's peer did not let ${first} go: ${JSON.stringify(account).slice(0, 1200)}`
    );
    assert.ok(
      listedAs,
      `matthew's peer let ${first} go without ever listing it as an own file nothing names`
    );
    assert.ok(
      listedAs.includes('self-put'),
      `the first build should read as handed to matthew's peer, not ${JSON.stringify(listedAs)}`
    );
  }
);

Then("the first build's bundle file is gone from matthew's peer", function () {
  assert.ok(solo.first, FIRST_BUILD_MISSING);
  const file = bundleFile('matthew', solo.first.hash);
  assert.ok(!existsSync(file), `matthew's peer still holds ${file}`);
});

Then(
  "within 5 minutes more the first build's bundle file is gone from jessica's peer",
  { timeout: 330_000 },
  async function () {
    assert.ok(solo.first, FIRST_BUILD_MISSING);
    const file = bundleFile('jessica', solo.first.hash);
    const gone = await pollUntil(async () => Promise.resolve(!existsSync(file)), 300_000, 3_000);
    assert.notEqual(
      gone,
      null,
      `jessica's peer still holds ${file}: ${JSON.stringify(await holdsAccount('jessica')).slice(0, 1200)}`
    );
  }
);

Then("the second build's bundle file is still on matthew's peer", function () {
  assert.ok(solo.second, 'the second build must be handed over first');
  const file = bundleFile('matthew', solo.second.hash);
  assert.ok(existsSync(file), `matthew's peer no longer holds ${file}`);
});

Then(
  "the file with no note of how it arrived is still on matthew's peer, and his peer's own account counts it as unrecorded",
  { timeout: 30_000 },
  async function () {
    assert.ok(solo.unrecorded, 'the unrecorded file must be handed over first');
    const file = bundleFile('matthew', solo.unrecorded);
    assert.ok(existsSync(file), `matthew's peer no longer holds ${file}`);
    const account = await holdsAccount('matthew');
    const unrecorded = account?.blobs?.['unrecorded']?.blobs ?? 0;
    assert.ok(unrecorded >= 1, `matthew's account counts ${unrecorded} unrecorded files`);
    assert.ok(
      !(account?.ownUnnamed ?? []).some(blob => blob.blobHash === solo.unrecorded),
      'a file with no note of how it arrived must never be listed as an own file'
    );
  }
);

Then(
  'doorway {string} serves the second build of that app',
  { timeout: 120_000 },
  async function (this: E2EWorld, doorway: string) {
    assert.ok(solo.second, 'the second build must be handed over first');
    const entry = solo.second.bundle.entryScript;
    const base = doorwayUrl(this, doorway);
    const served = await pollUntil(
      async () => {
        const { status, text } = await getRaw(`${base}/apps/${SOLO_APP}/index.html`, {
          timeoutMs: 10_000,
        });
        return status === 200 && text.includes(entry);
      },
      SERVE_BUDGET_MS,
      3_000
    );
    assert.notEqual(served, null, `doorway ${doorway} is not serving ${SOLO_APP} naming ${entry}`);
  }
);

AfterAll({ timeout: 60_000 }, async function () {
  for (const peer of solo.quickened) {
    try {
      await setRuntimeSetting(peer, PASS_SECONDS_KEY, null);
      await setRuntimeSetting(peer, UNNAMED_AGE_KEY, null);
    } catch {
      // Best effort: the keys are this run's own and a later run sets them again.
    }
  }
  for (const build of [solo.first, solo.second]) {
    if (build) removeFixtureBundle(build.bundle);
  }
});

AfterAll({ timeout: 60_000 }, async function () {
  for (const peer of run.retentionSet) {
    try {
      await setRetentionDepth(peer, null);
    } catch {
      // Best effort: the key is this run's own and a later run sets it again.
    }
  }
});

AfterAll({ timeout: 60_000 }, async function () {
  for (const peer of run.following) {
    try {
      await postRaw(`${storageUrl(peer)}${FOLLOW_PATH}`, { channel: CHANNEL_ID, remove: true });
    } catch {
      // A peer that is down keeps a follow entry for a run-owned channel no
      // one publishes on again; it idles. Logged by the absence of cleanup.
    }
  }
  rmSync(run.workDir, { recursive: true, force: true });
});
