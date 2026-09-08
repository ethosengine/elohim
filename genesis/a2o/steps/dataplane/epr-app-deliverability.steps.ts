/**
 * EPR-app deliverability — step definitions.
 *
 * Two features share this file:
 *   · features/dataplane/epr-app-deliverability.feature (@act:i, household mesh
 *     this run owns and may write to) — the whole deploy, staged and watched.
 *   · features/dataplane/served-shell-boots.feature (@act:ii, deployed fleet,
 *     read-only) — its dynamic clause, which needs the same browser steps.
 *
 * The browser steps live here rather than in steps/ui/ because they assert on a
 * page a STRANGER is handed: no login, no fixture human, no page object. What
 * they measure is whether the bytes a doorway served can start an application.
 *
 * The static-clause assertions ("every script and stylesheet the page from peer
 * X names is one that peer serves") are NOT re-implemented here — the Act I
 * story reuses the implementations in steps/dataplane.steps.ts by handing them
 * the page it already fetched, via `recordServedPage`.
 *
 * Spec: genesis/docs/superpowers/specs/2026-09-08-epr-app-deliverability-through-doorway.md
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import {
  getRaw,
  getRawWithHeaders,
  resolvePeerUrl,
  resolveStorageUrl,
} from '../../src/framework/dataplane/surfaces.js';
import { loadHouseholdMeshFixture } from '../../src/framework/fixtures/household-mesh.js';
import { E2EWorld } from '../../src/framework/world.js';
import { recordServedPage } from '../dataplane.steps.js';

import {
  buildFixtureBundle,
  CONVERGENCE_BOUND_MS,
  pollUntil,
  removeFixtureBundle,
  sameOrigin,
  stageBundle,
  visitInBrowser,
  type BrowserVisit,
  type FixtureBundle,
} from './epr-app-deliverability.helpers.js';

// ---------------------------------------------------------------------------
// Scenario-local state
// ---------------------------------------------------------------------------

interface PublishedApp {
  /** The EPR id this run authored — fresh per scenario, never a seeded page. */
  slug: string;
  bundle?: FixtureBundle;
  /** The bundle deliberately built without its entry script (station 4). */
  brokenBundle?: FixtureBundle;
  /** `sha256-…` of the coherent bundle, once staged. */
  blobHash?: string;
  /** `sha256-…` of the incoherent bundle, once its bytes were uploaded. */
  brokenBlobHash?: string;
  /** Every doorway URL this run PATCHed a head onto, in order. */
  declaredThrough: string[];
  /** stage-spa-blob.sh's last refusal output (station 4). */
  refusal?: { code: number; output: string };
}

interface HouseholdTopology {
  /** Every storage peer this household holds, doorway-backed or not. */
  all: string[];
  /** The peer with no front door — the sharpest witness of peer-to-peer travel. */
  doorless: string;
  /** doorway name → the storage peer it reads from. */
  behind: Record<string, string>;
}

const householdPeers = new WeakMap<E2EWorld, HouseholdTopology>();
const publishedApps = new WeakMap<E2EWorld, PublishedApp>();
const browserVisits = new WeakMap<E2EWorld, Map<string, BrowserVisit>>();

function app(world: E2EWorld): PublishedApp {
  const record = publishedApps.get(world);
  assert.ok(
    record,
    'no app has been published in this scenario — the step "a coherent EPR app bundle this run just built" must run first'
  );
  return record;
}

function requireBundle(world: E2EWorld): FixtureBundle {
  const bundle = app(world).bundle;
  assert.ok(bundle, 'this scenario built no coherent bundle');
  return bundle;
}

function visitMap(world: E2EWorld): Map<string, BrowserVisit> {
  let map = browserVisits.get(world);
  if (!map) {
    map = new Map<string, BrowserVisit>();
    browserVisits.set(world, map);
  }
  return map;
}

function requireVisit(world: E2EWorld, peerName: string): BrowserVisit {
  const visit = visitMap(world).get(peerName);
  assert.ok(
    visit,
    `no browser opened a page from ${peerName} in this scenario — a "visitor opens … in a browser" step must run first`
  );
  return visit;
}

/** The address the app this run published answers at on one doorway. */
function appUrl(world: E2EWorld, peerName: string): string {
  return `${resolvePeerUrl(peerName)}/apps/${encodeURIComponent(app(world).slug)}/index.html`;
}

/** The `commit` field of a version.json body, or '' when it carries none. */
function stampOf(text: string): string {
  try {
    const parsed = JSON.parse(text) as { commit?: unknown };
    return typeof parsed.commit === 'string' ? parsed.commit : '';
  } catch {
    return '';
  }
}

// ---------------------------------------------------------------------------
// Publishing (Act I only)
// ---------------------------------------------------------------------------

/**
 * Establish the household's storage peers in the story's OWN chain.
 *
 * The two Background steps above register the two DOORWAYS. The peers behind
 * them — and the third peer behind no door — are what Stations 2 and 3 assert
 * over, so they are named here rather than left to the reader to carry over
 * from the vocabulary. It also fails early and by name when a run's fixture
 * knows fewer than three peers, instead of letting "every household peer"
 * quietly mean "the two I could see".
 */
Given(
  "the household's storage peers are {string} behind doorway {string}, {string} behind doorway {string}, and {string} behind no doorway",
  function (
    this: E2EWorld,
    firstPeer: string,
    firstDoorway: string,
    secondPeer: string,
    secondDoorway: string,
    doorlessPeer: string
  ) {
    const fixture = loadHouseholdMeshFixture();
    const declared = [firstPeer, secondPeer, doorlessPeer];
    const missing = declared.filter(name => !fixture.storagePeers?.[name]?.url);
    const envNames = missing.map(name => `E2E_STORAGE_${name.toUpperCase()}`).join(', ');
    assert.ok(
      missing.length === 0,
      `this run cannot address household peer(s) ${missing.join(', ')} — set ${envNames} or ` +
        'E2E_HOUSEHOLD_FIXTURE_PATH (`just mesh prologue` writes it). "Every household peer" ' +
        'must mean all three, or Station 2 proves less than it claims.'
    );
    householdPeers.set(this, {
      all: declared,
      doorless: doorlessPeer,
      behind: { [firstDoorway]: firstPeer, [secondDoorway]: secondPeer },
    });
  }
);

Given('a coherent EPR app bundle this run just built', function (this: E2EWorld) {
  const existing = publishedApps.get(this);
  const bundle = buildFixtureBundle({ coherent: true });
  this.onCleanup(() => {
    removeFixtureBundle(bundle);
    return Promise.resolve();
  });
  publishedApps.set(this, {
    slug: existing?.slug ?? `epr-app-deliverability-${Date.now().toString(36)}`,
    ...existing,
    bundle,
    declaredThrough: existing?.declaredThrough ?? [],
  });
});

/**
 * Plant the EPR record the head will hang from.
 *
 * A record this run AUTHORS, never a seeded one: declaring a fixture bundle as
 * the head of a page the household is reading would deface it, and the earned
 * declaration guard would refuse the write anyway (measured 2026-09-05 —
 * declare_earned_canonical_head is restricted to a page's root author).
 */
Given('an EPR record this run owns for it', { timeout: 60_000 }, async function (this: E2EWorld) {
  const record = app(this);
  const storageUrl = resolveStorageUrl('alpha-A');
  assert.ok(
    storageUrl,
    'no direct storage URL for peer "alpha-A" — set E2E_STORAGE_URL (the household lane exports it from hc-mesh.sh mesh_seed_env)'
  );
  const response = await fetch(`${storageUrl}/db/content/bulk`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify([
      {
        id: record.slug,
        title: `EPR app deliverability fixture (${record.slug})`,
        description: 'a small site this test run published, so it may declare its own head',
        contentType: 'collective',
        contentFormat: 'html5-app',
        content: { slug: record.slug, entryPoint: 'index.html' },
        reach: 'commons',
      },
    ]),
  });
  assert.ok(
    response.ok,
    `could not author the EPR record "${record.slug}" on ${storageUrl}: ${response.status} ${await response.text()}`
  );
});

When(
  "each doorway is handed the bundle's bytes",
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    const record = app(this);
    const bundle = requireBundle(this);
    for (const peerName of ['alpha-A', 'elohim.host']) {
      const outcome = await stageBundle({
        bundle,
        slug: record.slug,
        doorwayUrl: resolvePeerUrl(peerName),
        declare: false,
      });
      assert.strictEqual(
        outcome.code,
        0,
        `handing the bundle's bytes to ${peerName} failed (exit ${outcome.code}):\n${outcome.output}`
      );
      record.blobHash = outcome.blobHash;
    }
    assert.ok(record.blobHash, 'stage-spa-blob.sh reported no blob hash for the staged bundle');
  }
);

When(
  'only doorway {string} is told this bundle is the new version',
  { timeout: 300_000 },
  async function (this: E2EWorld, peerName: string) {
    const record = app(this);
    const bundle = requireBundle(this);
    const doorwayUrl = resolvePeerUrl(peerName);
    const outcome = await stageBundle({ bundle, slug: record.slug, doorwayUrl, declare: true });
    assert.strictEqual(
      outcome.code,
      0,
      `declaring the head through ${peerName} failed (exit ${outcome.code}):\n${outcome.output}`
    );
    record.blobHash = outcome.blobHash;
    record.declaredThrough.push(doorwayUrl);
  }
);

When(
  'only doorway {string} is told this bundle is the new server-rendered version',
  { timeout: 300_000 },
  async function (this: E2EWorld, peerName: string) {
    const record = app(this);
    const bundle = requireBundle(this);
    const doorwayUrl = resolvePeerUrl(peerName);
    const outcome = await stageBundle({
      bundle,
      slug: record.slug,
      doorwayUrl,
      declare: true,
      kind: 'server',
    });
    assert.strictEqual(
      outcome.code,
      0,
      `declaring the server head through ${peerName} failed (exit ${outcome.code}):\n${outcome.output}`
    );
    record.blobHash = outcome.blobHash;
    record.declaredThrough.push(doorwayUrl);
  }
);

// ---------------------------------------------------------------------------
// Convergence (Act I only)
// ---------------------------------------------------------------------------

Then(
  "within {int} seconds doorway {string} serves a page naming that bundle's entry script",
  { timeout: CONVERGENCE_BOUND_MS + 60_000 },
  async function (this: E2EWorld, bound: number, peerName: string) {
    const record = app(this);
    const bundle = requireBundle(this);
    const url = appUrl(this, peerName);
    let last = { status: 0, text: '' };
    const elapsed = await pollUntil(async () => {
      last = await getRaw(url, { timeoutMs: 20_000 });
      return last.status === 200 && last.text.includes(bundle.entryScript);
    }, bound * 1000);
    assert.ok(
      elapsed !== null,
      `${peerName} did not serve the published bundle within ${bound}s: GET ${url} answered ${last.status} ` +
        `and the page never named "${bundle.entryScript}" (declared through ${record.declaredThrough.join(', ') || 'nobody'}). ` +
        'A doorway still behind after two of its own head-reconcile ticks is not slow, it is stuck.'
    );
    // Hand the page to the static-clause assertions in steps/dataplane.steps.ts,
    // so this story asserts through the SAME implementation the fleet story uses.
    recordServedPage(this, peerName, {
      urlPath: `/apps/${record.slug}/index.html`,
      url,
      status: last.status,
      text: last.text,
      ride: '',
    });
  }
);

Then(
  'within {int} seconds every household peer answers with the same server pointer for this app',
  { timeout: CONVERGENCE_BOUND_MS + 60_000 },
  async function (this: E2EWorld, bound: number) {
    const record = app(this);
    assert.ok(record.blobHash, 'no server bundle was staged, so there is no pointer to converge');
    const topology = householdPeers.get(this);
    assert.ok(
      topology,
      "the household's storage peers were never established — the Background step naming matthew, jessica and james must run first"
    );
    const fixture = loadHouseholdMeshFixture();
    const peers = topology.all.map(name => ({
      name,
      url: fixture.storagePeers?.[name]?.url ?? '',
    }));
    const seen = new Map<string, string>();
    const elapsed = await pollUntil(async () => {
      let all = true;
      for (const peer of peers) {
        const { status, text } = await getRaw(`${peer.url}/db/content/${record.slug}`, {
          timeoutMs: 20_000,
        });
        let pointer = '';
        if (status === 200) {
          try {
            pointer = String(
              (JSON.parse(text) as { serverBlobHash?: unknown }).serverBlobHash ?? ''
            );
          } catch {
            pointer = '';
          }
        }
        seen.set(peer.name, pointer || `<${status}>`);
        if (pointer !== record.blobHash) all = false;
      }
      return all;
    }, bound * 1000);
    const answered = [...seen].map(([name, value]) => `${name}=${value}`).join(', ');
    assert.ok(
      elapsed !== null,
      `the server pointer declared once on one peer did not reach every peer within ${bound}s. ` +
        `Declared ${record.blobHash}; peers answered ${answered}. ` +
        "A pointer written straight into one peer's own database and told to nobody is the 2026-09-08 shape."
    );
  }
);

Then(
  'no peer other than the one that was told was written to by this run',
  function (this: E2EWorld) {
    const record = app(this);
    assert.strictEqual(
      record.declaredThrough.length,
      1,
      `this run declared a head through ${record.declaredThrough.length} doorways ` +
        `(${record.declaredThrough.join(', ')}); the claim under test is that ONE declaration reaches every peer, ` +
        'so a second declaration would make the convergence above vacuous'
    );
  }
);

// ---------------------------------------------------------------------------
// Restart-while-peer-is-down (Act I only) — fixture not yet available
// ---------------------------------------------------------------------------

Given(
  'doorway {string} can be restarted while peer {string} is held down',
  function (this: E2EWorld, doorwayName: string, storagePeer: string) {
    // eslint-disable-next-line no-console
    console.log(
      `  PENDING: this run cannot restart doorway "${doorwayName}" while holding peer "${storagePeer}" down. ` +
        'app/elohim-app/scripts/hc-mesh.sh exposes `storage-restart <peer>` and `conductors-restart`, ' +
        'but no arm that (a) stops one storage peer and LEAVES it stopped, or (b) restarts a single ' +
        'doorway. Both doorways are launched inline by start_doorways and are only stopped by ' +
        '`hc-mesh.sh stop`, which takes the whole mesh with them. Needed: `hc-mesh.sh doorway-restart b` ' +
        'plus a hold-down form of storage-restart (or a documented SIGSTOP/SIGCONT pair on the peer, ' +
        'which is NOT the same fault — a paused peer completes the TCP handshake and a dead one refuses ' +
        'the connection, and this station is about the refused connection). Until one exists this ' +
        'scenario measures nothing and says so.'
    );
    return 'pending';
  }
);

When(
  'doorway {string} restarts while peer {string} is down',
  function (this: E2EWorld, doorwayName: string, storagePeer: string) {
    // eslint-disable-next-line no-console
    console.log(
      `  PENDING: no restart arm for doorway "${doorwayName}", and peer "${storagePeer}" was never ` +
        'held down — see the Given above.'
    );
    return 'pending';
  }
);

When('peer {string} comes back', function (this: E2EWorld, storagePeer: string) {
  // eslint-disable-next-line no-console
  console.log(
    `  PENDING: peer "${storagePeer}" was never held down, so nothing can come back — see the Given above.`
  );
  return 'pending';
});

Then('nobody cleared a cache or restarted anything to make that happen', function (this: E2EWorld) {
  // eslint-disable-next-line no-console
  console.log(
    '  PENDING: this run performs no cache clear and no second restart, but with the restart fixture ' +
      'absent there is no convergence to attribute — see the Given above.'
  );
  return 'pending';
});

// ---------------------------------------------------------------------------
// The bundle that cannot boot (Act I only)
// ---------------------------------------------------------------------------

When('this run builds a second bundle with its entry script removed', function (this: E2EWorld) {
  const record = app(this);
  const broken = buildFixtureBundle({ coherent: false });
  this.onCleanup(() => {
    removeFixtureBundle(broken);
    return Promise.resolve();
  });
  record.brokenBundle = broken;
});

When(
  'the steward tries to publish the second bundle through doorway {string}',
  { timeout: 300_000 },
  async function (this: E2EWorld, peerName: string) {
    const record = app(this);
    assert.ok(record.brokenBundle, 'no incoherent bundle was built');
    const outcome = await stageBundle({
      bundle: record.brokenBundle,
      slug: record.slug,
      doorwayUrl: resolvePeerUrl(peerName),
      declare: true,
    });
    record.refusal = { code: outcome.code, output: outcome.output };
    // The bytes reach the peer BEFORE it judges them, which is what lets the
    // next step force the head past the judgement the way an older peer would.
    record.brokenBlobHash = outcome.blobHash;
  }
);

Then('the deploy path refuses it and names the missing file', function (this: E2EWorld) {
  const record = app(this);
  const refusal = record.refusal;
  const broken = record.brokenBundle;
  assert.ok(refusal && broken, 'no publish of an incoherent bundle was attempted');
  assert.strictEqual(
    refusal.code,
    2,
    `publishing a bundle with no entry script exited ${refusal.code}, not 2 (the peer's BROKEN verdict). ` +
      `A non-2 exit means the head was minted for a bundle that cannot boot:\n${refusal.output}`
  );
  assert.ok(
    refusal.output.includes(broken.entryScript),
    `the refusal never named the missing file "${broken.entryScript}" — an operator reading it cannot ` +
      `tell what to fix:\n${refusal.output}`
  );
});

When(
  'the incoherent bundle is declared the new version anyway',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const record = app(this);
    assert.ok(
      record.brokenBlobHash,
      'the incoherent bundle never reached the peer, so there are no bytes to declare'
    );
    const storageUrl = resolveStorageUrl('alpha-A');
    assert.ok(storageUrl, 'no direct storage URL for peer "alpha-A" — set E2E_STORAGE_URL');
    const response = await fetch(`${storageUrl}/db/content/${record.slug}`, {
      method: 'PATCH',
      headers: {
        'content-type': 'application/json',
        'X-API-Key': process.env['STORAGE_API_KEY_ADMIN'] ?? '',
      },
      body: JSON.stringify({ blobHash: record.brokenBlobHash }),
    });
    assert.ok(
      response.ok,
      `could not force the incoherent head onto "${record.slug}": ${response.status} ${await response.text()}`
    );
  }
);

Then(
  'a visitor asking doorway {string} for this app is never handed a blank page',
  { timeout: CONVERGENCE_BOUND_MS + 60_000 },
  async function (this: E2EWorld, peerName: string) {
    const record = app(this);
    const broken = record.brokenBundle;
    const bundle = requireBundle(this);
    assert.ok(broken, 'no incoherent bundle was built');
    const url = appUrl(this, peerName);
    const { status, text, headers } = await getRawWithHeaders(url, { timeoutMs: 30_000 });

    if (status === 503) {
      assert.ok(
        headers['retry-after'],
        `${peerName} answered 503 for ${url} with no Retry-After. A converging answer that does not ` +
          'say when to come back is a dead end wearing an honest status code.'
      );
      return;
    }
    assert.strictEqual(
      status,
      200,
      `${peerName} answered ${status} for ${url} — neither the last version that worked nor an honest ` +
        'converging page.'
    );
    assert.ok(
      !text.includes(broken.entryScript),
      `${peerName} served the incoherent bundle's own page, which names "${broken.entryScript}" — a file ` +
        'nothing holds. That is the blank page, delivered with a 200.'
    );
    assert.ok(
      text.includes(bundle.entryScript),
      `${peerName} served neither the incoherent page nor the last version that worked (which names ` +
        `"${bundle.entryScript}"). A visitor was handed something, and this run cannot say it boots.`
    );
  }
);

Then(
  'doorway {string} says on the wire that it is behind and names the missing file',
  { timeout: 60_000 },
  async function (this: E2EWorld, peerName: string) {
    const record = app(this);
    const broken = record.brokenBundle;
    assert.ok(broken, 'no incoherent bundle was built');
    const url = appUrl(this, peerName);
    const { headers } = await getRawWithHeaders(url, { timeoutMs: 30_000 });
    const marker = headers['x-elohim-bundle'] ?? '';
    assert.ok(
      marker.startsWith('behind'),
      `${peerName} served this app with x-elohim-bundle "${marker || 'absent'}". A doorway serving an ` +
        'older version because the current one cannot boot must say so on the wire, or the only way to ' +
        'discover it is for a person to notice the page is old. SCOPE NOTE before triaging this red: ' +
        'the behind-marker is written by the warm-shell path, which serves EPR mounts projected by the ' +
        'EprRouter (a project-epr commitment binding a url_path). A site published only at its own ' +
        '/apps/{slug} address is proxied straight to storage and never reaches that path, so a red here ' +
        'may be a scope decision about the never-blank promise rather than a doorway defect.'
    );
    assert.ok(
      marker.includes(broken.entryScript),
      `${peerName} said "${marker}" — it admits it is behind but does not name the file that is missing ` +
        `("${broken.entryScript}"), which is the one thing an operator needs to fix it.`
    );
  }
);

// ---------------------------------------------------------------------------
// The browser steps — shared by the Act I and Act II stories
// ---------------------------------------------------------------------------

When(
  'a visitor opens the page at {string} on peer {string} in a browser',
  { timeout: 120_000 },
  async function (this: E2EWorld, urlPath: string, peerName: string) {
    const url = new URL(urlPath, `${resolvePeerUrl(peerName)}/`).toString();
    visitMap(this).set(peerName, await visitInBrowser(url));
  }
);

When(
  'a visitor opens the app this run published on peer {string} in a browser',
  { timeout: 120_000 },
  async function (this: E2EWorld, peerName: string) {
    visitMap(this).set(peerName, await visitInBrowser(appUrl(this, peerName)));
  }
);

Then(
  'the browser on peer {string} reported no uncaught error',
  function (this: E2EWorld, peerName: string) {
    const visit = requireVisit(this, peerName);
    assert.deepStrictEqual(
      visit.pageErrors,
      [],
      `${peerName}: the app threw while starting at ${visit.url} — ${visit.pageErrors.join(' | ')}. ` +
        'Every asset can answer 200 and the person still sees nothing.'
    );
  }
);

Then(
  'every asset the browser asked peer {string} for arrived',
  function (this: E2EWorld, peerName: string) {
    const visit = requireVisit(this, peerName);
    // Off-peer references (a CDN font, an analytics beacon) are somebody else's
    // contract; this holds a doorway only to its own, exactly as the static clause does.
    const missing = visit.httpErrors.filter(entry => sameOrigin(visit.url, entry.url));
    const aborted = visit.failedRequests.filter(entry => sameOrigin(visit.url, entry.url));
    assert.ok(
      missing.length === 0 && aborted.length === 0,
      `${peerName}: loading ${visit.url} left ${missing.length + aborted.length} of its own request(s) ` +
        `unanswered — ${[
          ...missing.map(entry => `${entry.url} -> ${entry.status}`),
          ...aborted.map(entry => `${entry.url} -> ${entry.failure}`),
        ].join('; ')}`
    );
  }
);

Then(
  'the app root on the page from peer {string} has content',
  function (this: E2EWorld, peerName: string) {
    const visit = requireVisit(this, peerName);
    assert.ok(
      visit.rootPresent,
      `${peerName}: the page at ${visit.url} has no <app-root> element at all, so there is nowhere for an ` +
        'app to start.'
    );
    assert.ok(
      visit.rootText.length > 0,
      `${peerName}: <app-root> was still empty after the page at ${visit.url} finished loading. That empty ` +
        'element IS the blank page, in the form a person sees it.'
    );
  }
);

/**
 * ACT II form. version.json records the build's COMMIT, not the bundle's
 * content hash (Jenkinsfile ~1206), and it is emitted once per app build rather
 * than once per EPR slug — so it can never be compared against a blobHash
 * directly. The one comparison it CAN carry is against the copy of version.json
 * inside the declared browser head's OWN bundle, fetched BY CONTENT ADDRESS at
 * /apps/{blobHash}/version.json. By address, never by slug: the slug projection
 * can itself be a bundle behind, and comparing two stale copies would agree
 * perfectly while a visitor reads yesterday's page. Same comparison the CI gate
 * scripts/ci/verify-served-shell.sh makes, so a household red and a pipeline red
 * mean the same thing.
 */
Then(
  'the build stamp peer {string} serves is the one the declared browser head of EPR {string} carries',
  { timeout: 90_000 },
  async function (this: E2EWorld, peerName: string, eprId: string) {
    const peerUrl = resolvePeerUrl(peerName);
    const row = await getRaw(`${peerUrl}/db/content/${encodeURIComponent(eprId)}`, {
      timeoutMs: 30_000,
    });
    let declaredHead = '';
    if (row.status === 200) {
      try {
        declaredHead = String((JSON.parse(row.text) as { blobHash?: unknown }).blobHash ?? '');
      } catch {
        declaredHead = '';
      }
    }
    assert.ok(
      declaredHead.length > 0,
      `${peerName} declares no browser head for "${eprId}" (GET /db/content/${eprId} answered ${row.status}), ` +
        'so there is nothing for the served build stamp to be measured against'
    );

    const served = await getRaw(`${peerUrl}/version.json`, { timeoutMs: 30_000 });
    assert.strictEqual(
      served.status,
      200,
      `${peerName}: GET /version.json answered ${served.status}. A shell whose own build stamp is absent ` +
        'from the bundle it names is the stale-shell shape one notch down: assets 200, stamp 404.'
    );
    const servedStamp = stampOf(served.text);
    assert.ok(
      servedStamp.length > 0,
      `${peerName}: /version.json carries no "commit" field — ${served.text.slice(0, 200)}`
    );

    const declared = await getRaw(
      `${peerUrl}/apps/${encodeURIComponent(declaredHead)}/version.json`,
      { timeoutMs: 30_000 }
    );
    if (declared.status !== 200) {
      // eslint-disable-next-line no-console
      console.log(
        `  NOTE: ${peerName} — the declared head ${declaredHead} carries no version.json ` +
          `(GET /apps/${declaredHead}/version.json answered ${declared.status}), so the served stamp cannot ` +
          `be tied to the head. Asserted only that the served stamp exists and is well-formed ("${servedStamp}").`
      );
      return;
    }
    assert.strictEqual(
      servedStamp,
      stampOf(declared.text),
      `${peerName}: the page a visitor is served carries build stamp "${servedStamp}", but the declared ` +
        `browser head ${declaredHead} of "${eprId}" carries "${stampOf(declared.text)}". Two builds, one address.`
    );
  }
);

/** ACT I form — the published bundle's own stamp is known to this run directly. */
Then(
  'the build stamp peer {string} serves for this app is the one the published bundle carries',
  { timeout: 90_000 },
  async function (this: E2EWorld, peerName: string) {
    const record = app(this);
    const bundle = requireBundle(this);
    const url = `${resolvePeerUrl(peerName)}/apps/${encodeURIComponent(record.slug)}/version.json`;
    const { status, text } = await getRaw(url, { timeoutMs: 30_000 });
    assert.strictEqual(
      status,
      200,
      `${peerName}: GET ${url} answered ${status} — no build stamp served`
    );
    assert.strictEqual(
      stampOf(text),
      bundle.stamp,
      `${peerName}: serves build stamp "${stampOf(text)}" for an app published at "${bundle.stamp}". ` +
        'The page and its stamp came from different builds.'
    );
  }
);

// STATION 2b — peer-to-doorway trip of the server-rendered version. The renderer adopts
// CONFIGURED slugs only (doorway render/registry.rs: adoption targets = ctx.slugs), so a
// run-owned slug is never materialized and the comparison cannot be made honestly here.
// Pending with the precondition named; the fleet-side twin is served-projected-head.feature.
Then(
  'within {int} seconds both doorways attest they materialized that server pointer for this app',
  async function (this: E2EWorld, _seconds: number) {
    console.log(
      '  PENDING: the doorway renderer materializes configured SSR slugs only — a run-owned slug is not one. ' +
        'Needs a mesh arm that mounts this run\'s slug as a rendered site (project-epr commitment + SSR_BUNDLE_SLUGS), ' +
        'then compare /health/startup servedBundleHeads[slug].serverBlobHash on doorways A and B to the declared pointer.'
    );
    return 'pending';
  }
);
