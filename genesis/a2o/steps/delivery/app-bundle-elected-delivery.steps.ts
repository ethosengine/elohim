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
import { mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

import { AfterAll, Given, Then, When } from '@cucumber/cucumber';

import { getRaw, postRaw } from '../../src/framework/dataplane/surfaces.js';
import {
  buildFixtureBundle,
  buildServerFixture,
  pollUntil,
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
} = {
  workDir: mkdtempSync(path.join(tmpdir(), `a2o-app-bundle-${RUN_STAMP}-`)),
  appsAuthored: false,
  following: new Set<PeerName>(),
  samples: { matthew: [], jessica: [], james: [] },
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
  const { manifestPath, packagerLog } = packageRelease(
    world,
    'earlier',
    builds,
    await currentHeadCid(),
    0
  );
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
  'both apps moved together on each peer, in the same step, so no peer ever showed one app new and the other old',
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
