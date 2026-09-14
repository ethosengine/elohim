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
import { createHash, randomBytes } from 'node:crypto';

import { Given, When, Then, After, Before } from '@cucumber/cucumber';

import {
  connectConductor,
  declareEarnedCanonicalHead,
  meshConductorPorts,
} from '../../src/framework/dataplane/carried-election.js';
import { cancelOwnedCommitmentWithReadback } from '../../src/framework/dataplane/owned-commitment-cleanup.js';
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
  buildServerFixture,
  doorwayIncarnations,
  doorwayRestartLog,
  meshControl,
  restartStoryDoorway,
  startOwnedDoorwayPair,
  CONVERGENCE_BOUND_MS,
  pollUntil,
  postFixtureCommitment,
  removeFixtureBundle,
  sameOrigin,
  stageBundle,
  stageInvalidFixture,
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
  mountPath: string;
  rootCommitments?: string[];
  publicHostname?: string;
  candidateHostname?: string;
  unrelatedHostname?: string;
  earnedBundle?: FixtureBundle;
  earnedBlobHash?: string;
  /** Exact Content action Matthew promoted as artifact A's earned head. */
  earnedHeadActionHash?: string;
  /** Exact Content action Matthew authored and declared as artifact B's candidate. */
  candidateActionHash?: string;
  /** Exact currently declared browser action, read back after the write. */
  latestHeadActionHash?: string;
  bundle?: FixtureBundle;
  /** The bundle deliberately built without its entry script (station 4). */
  brokenBundle?: FixtureBundle;
  /** `sha256-…` of the coherent bundle, once staged. */
  blobHash?: string;
  serverBlobHash?: string;
  serverDeclarations?: number;
  browserDeclaredAt?: number;
  serverDeclaredAt?: number;
  rendererAdoptionAt?: number;
  recoveredAt?: number;
  forcedDeclaredAt?: number;
  upgradeIncarnations?: string[];
  restarts?: number;
  heldPeers?: Set<string>;
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

const DOORWAYS = ['alpha-A', 'elohim.host'];
const ROOT_AUTHOR = 'human-matthew-manager';
const COMMITMENT_PAGE_SIZE = 100;
const MAX_COMMITMENT_PAGES = 10;

const householdPeers = new WeakMap<E2EWorld, HouseholdTopology>();
const publishedApps = new WeakMap<E2EWorld, PublishedApp>();
const browserVisits = new WeakMap<E2EWorld, Map<string, BrowserVisit>>();

function isDeliverabilityFeature(uri?: string): boolean {
  const normalized = uri?.replaceAll('\\', '/');
  return [
    'features/dataplane/epr-app-deliverability.feature',
    'features/dataplane/epr-app-channel-isolation.feature',
  ].some(feature => normalized?.endsWith(feature));
}

Before(async function (this: E2EWorld, scenario) {
  if (!isDeliverabilityFeature(scenario.gherkinDocument.uri)) return;
  const fixture = loadHouseholdMeshFixture();
  assert.equal(fixture.processControl, true, 'owned doorway pair requires processControl=true');
  const previous = Object.fromEntries(
    ['E2E_DOORWAY_ALPHA', 'E2E_DOORWAY_B', 'E2E_DOORWAY_BETA'].map(key => [key, process.env[key]])
  );
  const pair = await startOwnedDoorwayPair(this, fixture);
  process.env['E2E_DOORWAY_ALPHA'] = pair.url('a');
  process.env['E2E_DOORWAY_B'] = pair.url('b');
  process.env['E2E_DOORWAY_BETA'] = pair.url('b');
  // Registered first, therefore run last: bundle cleanup and the explicit root-cancellation
  // hook may still restart/read these doorways before their owned processes disappear.
  this.onCleanup(
    async () => {
      let cleanupError: unknown;
      try {
        await pair.close();
      } catch (error) {
        cleanupError = error;
      }
      for (const [key, value] of Object.entries(previous)) {
        if (value === undefined) delete process.env[key];
        else process.env[key] = value;
      }
      this.attach(
        JSON.stringify({ kind: 'owned-doorway-pair', ...pair.receipt() }),
        'application/json'
      );
      if (cleanupError) throw cleanupError;
    },
    { required: true }
  );
});

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

export interface ChaosPublishedAuthority {
  slug: string;
  mountPath: string;
  actionHash: string;
  blobHash: string;
  version: string;
  entryScript: string;
  styleSheet: string;
}

export interface ChaosAuthorReceipt {
  schema: 'doorway-chaos-author-receipt/v1';
  source: 'fixture-author-operation';
  receiptId: string;
  authorId: string;
  authoredAt: string;
  contentId: string;
  authority: ChaosPublishedAuthority;
}

/** Exact run-owned authority tuple used by the doorway withdrawal story. */
export function chaosPublishedAuthority(world: E2EWorld): ChaosPublishedAuthority {
  const record = app(world);
  const bundle = requireBundle(world);
  assert.ok(record.latestHeadActionHash, 'the run-owned app has no observed declared action');
  assert.ok(record.blobHash, 'the run-owned app has no declared browser blob');
  return {
    slug: record.slug,
    mountPath: record.mountPath,
    actionHash: record.latestHeadActionHash,
    blobHash: record.blobHash,
    version: bundle.stamp,
    entryScript: bundle.entryScript,
    styleSheet: bundle.styleSheet,
  };
}

/** Receipt emitted from the fixture author operation, before any serving-path observation. */
export function chaosAuthorReceipt(world: E2EWorld): ChaosAuthorReceipt {
  const record = app(world);
  const authority = chaosPublishedAuthority(world);
  assert.ok(record.browserDeclaredAt, 'the fixture author operation has no declaration timestamp');
  return {
    schema: 'doorway-chaos-author-receipt/v1',
    source: 'fixture-author-operation',
    receiptId: authority.actionHash,
    authorId: ROOT_AUTHOR,
    authoredAt: new Date(record.browserDeclaredAt).toISOString(),
    contentId: authority.slug,
    authority,
  };
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
  return `${resolvePeerUrl(peerName)}${app(world).mountPath}`;
}

/** Resolve a symbolic doorway to the storage peer declared by this scenario. */
function storageUrlBehindDoorway(world: E2EWorld, doorway: string): string {
  const topology = householdPeers.get(world);
  assert.ok(topology, 'the household storage topology has not been declared');
  const peer = topology.behind[doorway];
  assert.ok(peer, `the household topology names no storage peer behind doorway "${doorway}"`);
  const storageUrl = loadHouseholdMeshFixture().storagePeers?.[peer]?.url;
  assert.ok(storageUrl, `the household fixture has no storage URL for peer "${peer}"`);
  return storageUrl;
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

interface RootCommitmentRow {
  state: string;
  inScopeOf: string[];
  metadata?: { urlPath?: string };
}

/**
 * Refuse to borrow `/` unless every matching commitment has been inspected.
 * The API returns a plain array with no total, so a full last page is
 * ambiguous. Bound the walk and fail closed rather than treating a truncated
 * result as proof that no root claim exists.
 */
async function assertRootCommitmentIsUnclaimed(
  storageUrl: string,
  peerName: string,
  doorwayId: string
): Promise<void> {
  for (let page = 0; page < MAX_COMMITMENT_PAGES; page += 1) {
    const offset = page * COMMITMENT_PAGE_SIZE;
    const response = await fetch(
      `${storageUrl}/api/v1/commitments?action=project-epr&limit=${COMMITMENT_PAGE_SIZE}&offset=${offset}`
    );
    if (!response.ok) {
      assert.fail(
        `${peerName}: could not inspect project-epr commitments: ${response.status} ${await response.text()}`
      );
    }
    const rows = (await response.json()) as RootCommitmentRow[];
    assert.ok(Array.isArray(rows), `${peerName}: commitments response is not a list`);
    assert.ok(
      !rows.some(
        row =>
          !['cancelled', 'superseded'].includes(row.state) &&
          row.metadata?.urlPath === '/' &&
          row.inScopeOf.some(scope => scope.startsWith(`doorway:${doorwayId}|`))
      ),
      `${peerName}: a root commitment already exists; fixture refuses to replace it`
    );
    if (rows.length < COMMITMENT_PAGE_SIZE) return;
  }
  assert.fail(
    `${peerName}: project-epr commitment scan reached ${MAX_COMMITMENT_PAGES * COMMITMENT_PAGE_SIZE} rows; ` +
      'cannot prove the root is unclaimed within the bounded precondition'
  );
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

function buildNextFixture(this: E2EWorld, root = false): void {
  const existing = publishedApps.get(this);
  const slug =
    existing?.slug ??
    `epr-app-deliverability-${Date.now().toString(36)}-${randomBytes(4).toString('hex')}`;
  const mountPath = existing?.mountPath ?? (root ? '/' : `/lamad/concept/${slug}`);
  const bundle = buildFixtureBundle({
    coherent: true,
    baseHref: `${mountPath.replace(/\/$/, '')}/`,
  });
  this.onCleanup(async () => {
    removeFixtureBundle(bundle);
    return Promise.resolve();
  });
  publishedApps.set(this, {
    slug,
    mountPath,
    ...existing,
    bundle,
    declaredThrough: existing?.declaredThrough ?? [],
  });
}

Given('a coherent EPR app bundle this run just built', function (this: E2EWorld) {
  buildNextFixture.call(this);
});
Given(
  'a coherent EPR app bundle this run just built for the root address',
  function (this: E2EWorld) {
    buildNextFixture.call(this, true);
  }
);
Given('Matthew has built coherent root app artifact A', function (this: E2EWorld) {
  buildNextFixture.call(this, true);
});
When('this run builds a next coherent browser and server version', function (this: E2EWorld) {
  buildNextFixture.call(this);
  app(this).upgradeIncarnations = doorwayIncarnations(this);
});
When('this run builds a next coherent browser version', function (this: E2EWorld) {
  buildNextFixture.call(this);
  app(this).upgradeIncarnations = doorwayIncarnations(this);
});
When('Matthew builds coherent successor artifact B', function (this: E2EWorld) {
  buildNextFixture.call(this);
  app(this).upgradeIncarnations = doorwayIncarnations(this);
});

/**
 * Plant the EPR record the head will hang from.
 *
 * A record this run AUTHORS, never a seeded one: declaring a fixture bundle as
 * the head of a page the household is reading would deface it, and the earned
 * declaration guard would refuse the write anyway (measured 2026-09-05 —
 * declare_earned_canonical_head is restricted to a page's root author).
 */
async function authorOwnedEprRecord(world: E2EWorld): Promise<void> {
  const record = app(world);
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
  // Mount the run-owned site through the same EPR router as the public landing.
  // These are hosting commitments, not extra writes of the app's head.
  for (const peerName of DOORWAYS) {
    const base = resolvePeerUrl(peerName);
    const coherenceResponse = await fetch(`${base}/api/v1/federation/coherence`);
    const coherence = (await coherenceResponse.json()) as {
      doorwayId: string;
    };
    assert.ok(coherence.doorwayId, `${peerName} exposes no doorway identity`);
    if (record.mountPath === '/') {
      const routes = await fetch(`${base}/api/v1/federation/coherence`);
      const projected = (await routes.json()) as { heads: { urlPath: string }[] };
      assert.ok(
        !projected.heads.some(head => head.urlPath === '/'),
        `${peerName}: root is already claimed; cannot borrow it for this fixture`
      );
      await assertRootCommitmentIsUnclaimed(storageUrl, peerName, coherence.doorwayId);
      record.rootCommitments ??= [];
      record.rootCommitments.push(`${record.slug}-${coherence.doorwayId}`);
    }
    const metadata = {
      urlPath: record.mountPath,
      baseHref: `${record.mountPath.replace(/\/$/, '')}/`,
      mode: 'cached',
      reach: 'commons',
      entryFile: 'index.html',
      ...(record.publicHostname && {
        hostnames: [record.publicHostname],
        channel: 'converged',
      }),
    };
    const mounted = await postFixtureCommitment(`${storageUrl}/api/v1/commitments`, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        'X-API-Key': process.env['STORAGE_API_KEY_ADMIN'] ?? 'mesh-admin-dev-key',
      },
      body: JSON.stringify({
        id: `${record.slug}-${coherence.doorwayId}`,
        action: 'project-epr',
        provider: ROOT_AUTHOR,
        receiver: ROOT_AUTHOR,
        inScopeOf: `doorway:${coherence.doorwayId}|epr:${record.slug}`,
        metadataJson: JSON.stringify(metadata),
        metadata,
      }),
    });
    const mountResult = mounted.text;
    if (!mounted.ok && mountResult.includes('UNIQUE constraint failed: rea_commitments.id')) {
      // The canonical signal can project this very creation before the eager
      // HTTP projection inserts it. Only exact anchored readback satisfies setup.
      const id = `${record.slug}-${coherence.doorwayId}`;
      const readback: Response = await fetch(`${storageUrl}/api/v1/commitments/${id}`);
      assert.equal(readback.status, 200, `mount race readback ${id}`);
      const existing = (await readback.json()) as {
        id: string;
        action: string;
        provider: string;
        receiver: string;
        inScopeOf: string[];
        metadata: unknown;
        dhtAnchorHash?: string;
      };
      assert.deepEqual(
        {
          id: existing.id,
          action: existing.action,
          provider: existing.provider,
          receiver: existing.receiver,
          inScopeOf: existing.inScopeOf,
          metadata: existing.metadata,
        },
        {
          id,
          action: 'project-epr',
          provider: ROOT_AUTHOR,
          receiver: ROOT_AUTHOR,
          inScopeOf: [`doorway:${coherence.doorwayId}|epr:${record.slug}`],
          metadata,
        }
      );
      assert.ok(existing.dhtAnchorHash, 'mount race readback is not anchored');
    } else {
      assert.ok(mounted.ok, `mount ${peerName}: ${mounted.status} ${mountResult}`);
    }
    if (record.candidateHostname) {
      const candidateMetadata = {
        ...metadata,
        hostnames: [record.candidateHostname],
        channel: 'candidate',
      };
      const candidateId = `${record.slug}-${coherence.doorwayId}-candidate`;
      record.rootCommitments?.push(candidateId);
      const candidate = await postFixtureCommitment(`${storageUrl}/api/v1/commitments`, {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'X-API-Key': process.env['STORAGE_API_KEY_ADMIN'] ?? 'mesh-admin-dev-key',
        },
        body: JSON.stringify({
          id: candidateId,
          action: 'project-epr',
          provider: ROOT_AUTHOR,
          receiver: ROOT_AUTHOR,
          inScopeOf: `doorway:${coherence.doorwayId}|epr:${record.slug}`,
          metadataJson: JSON.stringify(candidateMetadata),
          metadata: candidateMetadata,
        }),
      });
      assert.ok(candidate.ok, `candidate mount ${peerName}: ${candidate.status} ${candidate.text}`);
    }
  }
  // Head reconciliation targets configured apps, including CSR fallbacks.
  for (const doorway of ['a', 'b']) await restartStoryDoorway(world, doorway, record.slug);
  for (const doorway of DOORWAYS) {
    const ready = await pollUntil(
      async () =>
        (await getRaw(`${resolvePeerUrl(doorway)}/health`, { timeoutMs: 2000 })).status === 200,
      60_000
    );
    assert.notEqual(ready, null, `${doorway}: configured fixture doorway did not become ready`);
  }
}

Given('an EPR record this run owns for it', { timeout: 180_000 }, async function (this: E2EWorld) {
  await authorOwnedEprRecord(this);
});

Given(
  'Matthew has registered PUBLIC_NAME for published releases and CANDIDATE_NAME for staged releases of a fresh empty app',
  { timeout: 180_000 },
  async function (this: E2EWorld) {
    const record = app(this);
    record.publicHostname = `public-${record.slug}.elohim.local`;
    record.candidateHostname = `candidate-${record.slug}.elohim.local`;
    record.unrelatedHostname = `unrelated-${record.slug}.elohim.local`;
    await authorOwnedEprRecord(this);
    for (const doorway of DOORWAYS) {
      const storageUrl = storageUrlBehindDoorway(this, doorway);
      const head = await fetch(`${storageUrl}/db/content/${record.slug}/head`);
      if (head.status === 404) continue;

      assert.equal(head.status, 200, `${doorway}: fresh head read returned ${head.status}`);
      const projected = (await head.json()) as {
        headActionHash?: string;
        declared?: boolean;
        dhtAnchorHash?: string | null;
        stagingCandidate?: string | null;
        stagingCandidateBlobHash?: string | null;
        stagingCandidateState?: string;
      };
      const contentResponse = await fetch(`${storageUrl}/db/content/${record.slug}`);
      assert.equal(
        contentResponse.status,
        200,
        `${doorway}: fresh content read returned ${contentResponse.status}`
      );
      const content = (await contentResponse.json()) as { dhtAnchorHash?: string | null };
      assert.ok(content.dhtAnchorHash, `${doorway}: fresh content has no notarized anchor`);
      assert.equal(
        projected.headActionHash,
        content.dhtAnchorHash,
        `${doorway}: fresh head is neither absent nor the content's undeclared anchor fallback`
      );
      assert.equal(projected.declared, false, `${doorway}: fresh record has a declared release`);
      assert.equal(
        projected.dhtAnchorHash,
        content.dhtAnchorHash,
        `${doorway}: fresh head reports a different notarized anchor`
      );
      assert.equal(
        projected.stagingCandidateState,
        'none',
        `${doorway}: fresh record does not have authoritative candidate absence`
      );
      assert.equal(projected.stagingCandidate, null, `${doorway}: fresh record has a candidate`);
      assert.equal(
        projected.stagingCandidateBlobHash,
        null,
        `${doorway}: fresh record has candidate bytes`
      );
    }
  }
);

async function handCurrentBundleBytesToEachDoorway(world: E2EWorld): Promise<void> {
  const record = app(world);
  const bundle = requireBundle(world);
  for (const peerName of DOORWAYS) {
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

When(
  "each doorway is handed the bundle's bytes",
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    await handCurrentBundleBytesToEachDoorway(this);
  }
);
When(
  /^each doorway receives artifact [AB]'s exact bytes$/,
  { timeout: 300_000 },
  async function (this: E2EWorld) {
    await handCurrentBundleBytesToEachDoorway(this);
  }
);

async function declareCurrentBrowserBundle(this: E2EWorld, peerName: string): Promise<void> {
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
  assert.ok(outcome.authoredActionHash, `authoring through ${peerName} returned no exact action`);
  record.latestHeadActionHash = outcome.authoredActionHash;
  record.browserDeclaredAt = Date.now();
  record.declaredThrough.push(doorwayUrl);
}

async function declareCurrentBrowserBundleThroughAuthorStorage(world: E2EWorld): Promise<void> {
  const record = app(world);
  const bundle = requireBundle(world);
  const storageUrl = resolveStorageUrl('alpha-A');
  assert.ok(storageUrl, 'canonical source-author storage URL is absent');
  const outcome = await stageBundle({
    bundle,
    slug: record.slug,
    doorwayUrl: storageUrl,
    declare: true,
  });
  assert.strictEqual(
    outcome.code,
    0,
    `declaring the head through the source author's storage peer failed (exit ${outcome.code}):\n${outcome.output}`
  );
  record.blobHash = outcome.blobHash;
  assert.ok(
    outcome.authoredActionHash,
    'source-author storage publication returned no exact action'
  );
  record.latestHeadActionHash = outcome.authoredActionHash;
  record.browserDeclaredAt = Date.now();
  record.declaredThrough.push(storageUrl);
}

When(
  "Matthew submits artifact A's byte-binding declaration through doorway {string}",
  {
    timeout: 300_000,
  },
  declareCurrentBrowserBundle
);

When(
  'only doorway {string} is told this bundle is the new version',
  {
    timeout: 300_000,
  },
  declareCurrentBrowserBundle
);

When(
  'the canonical source author publishes authority B through its storage peer',
  { timeout: 300_000 },
  async function (this: E2EWorld): Promise<void> {
    await declareCurrentBrowserBundleThroughAuthorStorage(this);
  }
);

When(
  'Matthew submits his authorized publication claim for artifact A',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const record = app(this);
    const storageUrl = resolveStorageUrl('alpha-A');
    assert.ok(storageUrl, 'root-author storage URL is absent');
    const head = await fetch(`${storageUrl}/db/content/${record.slug}/head`);
    assert.equal(head.status, 200, `root-author head read: ${head.status}`);
    const { headActionHash } = (await head.json()) as { headActionHash?: string };
    assert.ok(headActionHash, 'root-author head read names no action hash');
    const { adminPort, appPort } = meshConductorPorts(0);
    const rail = await connectConductor(adminPort, appPort);
    try {
      const earned = await declareEarnedCanonicalHead(rail, record.slug, headActionHash);
      assert.equal(earned?.canonical, true, 'root-author promotion did not mint an earned head');
    } finally {
      await rail.close();
    }
    record.earnedBundle = requireBundle(this);
    record.earnedBlobHash = record.blobHash;
    record.earnedHeadActionHash = headActionHash;
  }
);

When(
  "Matthew submits artifact B's byte-binding staging declaration through doorway {string}",
  { timeout: 300_000 },
  async function (this: E2EWorld, peerName: string) {
    const record = app(this);
    assert.ok(record.blobHash, 'artifact B bytes have no content address to declare');
    assert.ok(
      record.earnedHeadActionHash,
      'artifact A has no exact earned action for artifact B to name as its lineage parent'
    );
    const doorwayUrl = resolvePeerUrl(peerName);
    const patch = await postFixtureCommitment(`${doorwayUrl}/db/content/${record.slug}`, {
      method: 'PATCH',
      headers: {
        'content-type': 'application/json',
        'X-API-Key': process.env['STORAGE_API_KEY_ADMIN'] ?? '',
      },
      body: JSON.stringify({
        blobHash: record.blobHash,
        metadata: {
          kind: 'release-manifest',
          publishedAt: new Date().toISOString(),
          manifest: { envelope: { lineageParentCid: record.earnedHeadActionHash } },
        },
      }),
    });
    assert.ok(
      patch.ok,
      `artifact B authoring PATCH through ${peerName} failed: HTTP ${patch.status}: ${patch.text}`
    );
    let patched: { dhtAnchorHash?: string } = {};
    try {
      patched = JSON.parse(patch.text) as { dhtAnchorHash?: string };
    } catch {
      // The assertion below reports the exact response body.
    }
    assert.ok(
      patched.dhtAnchorHash,
      `artifact B authoring PATCH through ${peerName} returned no dhtAnchorHash: ` +
        `HTTP ${patch.status}: ${patch.text}`
    );
    record.candidateActionHash = patched.dhtAnchorHash;

    const canonical = await postFixtureCommitment(
      `${doorwayUrl}/db/content/${record.slug}/canonical-head`,
      {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'X-API-Key': process.env['STORAGE_API_KEY_ADMIN'] ?? '',
        },
        body: JSON.stringify({ headActionHash: record.candidateActionHash }),
      }
    );
    assert.ok(
      canonical.ok,
      `artifact B canonical declaration through ${peerName} failed: ` +
        `HTTP ${canonical.status}: ${canonical.text}`
    );

    record.browserDeclaredAt = Date.now();
  }
);

interface BundleServeObservation {
  doorway: string;
  expected: { entryScript: string; commit: string };
  root?: {
    status: number;
    entryScriptPresent: boolean;
    bundle?: string;
    reason?: string;
    bodyPrefix?: string;
  };
  asset?: { status: number };
  version?: { status: number; commit?: string; parseError?: string };
  requestError?: string;
}

async function hostnameServesBundle(
  doorwayUrl: string,
  hostname: string,
  bundle: FixtureBundle,
  observe: (observation: BundleServeObservation) => void
): Promise<boolean> {
  const headers = { Host: hostname };
  const observation: BundleServeObservation = {
    doorway: doorwayUrl,
    expected: { entryScript: bundle.entryScript, commit: bundle.stamp },
  };
  try {
    const root = await getRawWithHeaders(`${doorwayUrl}/`, { headers });
    observation.root = {
      status: root.status,
      entryScriptPresent: root.text.includes(bundle.entryScript),
      bundle: root.headers['x-elohim-bundle'],
      reason: root.headers['x-deliverability-reason'],
      ...(root.status < 200 || root.status >= 300 ? { bodyPrefix: root.text.slice(0, 160) } : {}),
    };
    observe(observation);
    if (root.status < 200 || root.status >= 300 || !observation.root.entryScriptPresent) {
      return false;
    }

    const asset = await getRawWithHeaders(`${doorwayUrl}/${bundle.entryScript}`, { headers });
    observation.asset = { status: asset.status };
    observe(observation);
    if (asset.status < 200 || asset.status >= 300) return false;

    const version = await getRawWithHeaders(`${doorwayUrl}/version.json`, { headers });
    observation.version = { status: version.status };
    observe(observation);
    if (version.status < 200 || version.status >= 300) return false;
    try {
      observation.version.commit = (JSON.parse(version.text) as { commit?: string }).commit;
    } catch (error) {
      observation.version.parseError = error instanceof Error ? error.message : String(error);
      observe(observation);
      throw error;
    }
    observe(observation);
    return observation.version.commit === bundle.stamp;
  } catch (error) {
    if (!observation.version?.parseError) {
      observation.requestError = error instanceof Error ? error.message : String(error);
      observe(observation);
    }
    throw error;
  }
}

async function bothDoorwaysServe(
  hostname: string,
  bundle: FixtureBundle,
  timeoutMs = CONVERGENCE_BOUND_MS
): Promise<{ ok: boolean; last: BundleServeObservation[] }> {
  const deadline = Date.now() + timeoutMs;
  const last: BundleServeObservation[] = [];
  const results = await Promise.all(
    DOORWAYS.map(async (doorway, index) =>
      pollUntil(
        async () =>
          hostnameServesBundle(resolvePeerUrl(doorway), hostname, bundle, observation => {
            last[index] = { ...observation };
          }),
        Math.max(0, deadline - Date.now())
      )
    )
  );
  return { ok: results.every(Boolean), last };
}

function bundleServeFailure(message: string, result: { last: BundleServeObservation[] }): string {
  return `${message}; last observations: ${JSON.stringify(result.last)}`;
}

Then(
  'within 75 seconds both doorways serve PUBLIC_NAME with artifact A',
  { timeout: 90_000 },
  async function (this: E2EWorld) {
    const record = app(this);
    assert.ok(record.publicHostname, 'public hostname absent');
    const result = await bothDoorwaysServe(record.publicHostname, requireBundle(this));
    assert.equal(
      result.ok,
      true,
      bundleServeFailure(
        'public hostname did not serve the exact earned HTML, asset, and version stamp',
        result
      )
    );
  }
);

Then(
  'within 75 seconds both doorways serve CANDIDATE_NAME with artifact B',
  { timeout: 90_000 },
  async function (this: E2EWorld) {
    const record = app(this);
    assert.ok(record.candidateHostname, 'candidate hostname absent');
    const result = await bothDoorwaysServe(record.candidateHostname, requireBundle(this));
    assert.equal(
      result.ok,
      true,
      bundleServeFailure(
        'candidate hostname did not serve the exact staged HTML, asset, and version stamp',
        result
      )
    );
  }
);

Then(
  'both doorways still serve PUBLIC_NAME with artifact A',
  { timeout: 90_000 },
  async function (this: E2EWorld) {
    const record = app(this);
    assert.ok(record.publicHostname && record.earnedBundle, 'earned hostname/version absent');
    const result = await bothDoorwaysServe(record.publicHostname, record.earnedBundle);
    assert.equal(
      result.ok,
      true,
      bundleServeFailure('candidate publication disturbed the exact earned public version', result)
    );
  }
);

Then(
  "both doorways answer 404 for UNRELATED_NAME without artifact B's asset or build stamp",
  async function (this: E2EWorld) {
    const record = app(this);
    const candidate = requireBundle(this);
    assert.ok(record.unrelatedHostname, 'UNRELATED_NAME is absent');
    const unrelatedHostname = record.unrelatedHostname;
    for (const doorway of DOORWAYS) {
      const requestHeaders: Record<string, string> = { Host: unrelatedHostname };
      for (const subpath of ['', candidate.entryScript, 'version.json']) {
        const response = await getRawWithHeaders(`${resolvePeerUrl(doorway)}/${subpath}`, {
          headers: requestHeaders,
        });
        const body = response.text;
        assert.equal(response.status, 404, `${doorway}: UNRELATED_NAME /${subpath} was not 404`);
        assert.ok(
          !body.includes(candidate.entryScript),
          `${doorway}: response named version B asset`
        );
        assert.ok(!body.includes(candidate.stamp), `${doorway}: response named version B stamp`);
      }
    }
  }
);

When(
  'Matthew submits his authorized publication claim for artifact B',
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    const record = app(this);
    const storageUrl = resolveStorageUrl('alpha-A');
    assert.ok(storageUrl, 'root-author storage URL is absent');
    const response = await fetch(`${storageUrl}/db/content/${record.slug}/head`);
    const responseText = await response.text();
    let head: { stagingCandidate?: string } = {};
    try {
      head = JSON.parse(responseText) as { stagingCandidate?: string };
    } catch {
      // The assertions below report the exact response body.
    }
    assert.equal(
      response.status,
      200,
      `candidate head read: HTTP ${response.status}: ${responseText}`
    );
    assert.ok(record.candidateActionHash, 'artifact B has no exact authored action to promote');
    assert.equal(
      head.stagingCandidate,
      record.candidateActionHash,
      `the notary election named candidate ${String(head.stagingCandidate)}, not artifact B action ` +
        `${record.candidateActionHash}: HTTP ${response.status}: ${responseText}`
    );
    const { adminPort, appPort } = meshConductorPorts(0);
    const rail = await connectConductor(adminPort, appPort);
    try {
      const promoted = await declareEarnedCanonicalHead(
        rail,
        record.slug,
        record.candidateActionHash
      );
      assert.equal(promoted?.canonical, true, 'candidate promotion did not mint an earned head');
    } finally {
      await rail.close();
    }
  }
);

Then(
  'within 75 seconds CANDIDATE_NAME answers 404 with "no-candidate-staged" through both doorways',
  { timeout: 90_000 },
  async function (this: E2EWorld) {
    const record = app(this);
    assert.ok(record.candidateHostname, 'candidate hostname absent');
    const deadline = Date.now() + CONVERGENCE_BOUND_MS;
    const results = await Promise.all(
      DOORWAYS.map(async doorway =>
        pollUntil(
          async () => {
            const response = await getRawWithHeaders(`${resolvePeerUrl(doorway)}/`, {
              headers: { Host: record.candidateHostname as string },
            });
            return response.status === 404 && response.text.includes('no-candidate-staged');
          },
          Math.max(0, deadline - Date.now())
        )
      )
    );
    assert.ok(results.every(Boolean), 'candidate hostname retained the promoted candidate CID');
  }
);

Then(
  'both doorways serve PUBLIC_NAME with artifact B',
  { timeout: 90_000 },
  async function (this: E2EWorld) {
    const record = app(this);
    assert.ok(record.publicHostname, 'public hostname absent');
    const result = await bothDoorwaysServe(record.publicHostname, requireBundle(this));
    assert.equal(
      result.ok,
      true,
      bundleServeFailure('public hostname did not follow the newly earned version', result)
    );
  }
);

When(
  'only doorway {string} is told this bundle is the new server-rendered version',
  { timeout: 300_000 },
  async function (this: E2EWorld, peerName: string) {
    const record = app(this);
    const bundle = buildServerFixture(requireBundle(this));
    this.onCleanup(async () => {
      removeFixtureBundle(bundle);
      return Promise.resolve();
    });
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
    record.serverBlobHash = outcome.blobHash;
    record.serverDeclaredAt = Date.now();
    record.rendererAdoptionAt = record.serverDeclaredAt;
    record.serverDeclarations = (record.serverDeclarations ?? 0) + 1;
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
    const anchor = record.recoveredAt ?? record.browserDeclaredAt;
    assert.ok(
      anchor,
      'no browser publication or observed peer recovery starts the readiness deadline'
    );
    let last = { status: 0, text: '' };
    const elapsed = await pollUntil(
      async () => {
        last = await getRaw(url, { timeoutMs: 10_000 });
        if (last.status !== 200 || !last.text.includes(bundle.entryScript)) return false;
        // A fresh mount's shell and its slug asset route can converge on separate
        // ticks. Spend the same bounded readiness window on both; the strict
        // static and browser assertions run only after the whole bundle is ready.
        const assets = await Promise.all(
          [bundle.entryScript, bundle.styleSheet].map(async file =>
            getRaw(`${url}/${file}`, { timeoutMs: 5000 })
          )
        );
        return assets.every(asset => asset.status === 200);
      },
      bound * 1000 - (Date.now() - anchor)
    );
    assert.ok(
      elapsed !== null,
      `${peerName} did not serve the published bundle within ${bound}s: GET ${url} answered ${last.status} ` +
        `and the page plus "${bundle.entryScript}" / "${bundle.styleSheet}" never became servable together (declared through ${record.declaredThrough.join(', ') || 'nobody'}). ` +
        'A doorway still behind after two of its own head-reconcile ticks is not slow, it is stuck.'
    );
    // Hand the page to the static-clause assertions in steps/dataplane.steps.ts,
    // so this story asserts through the SAME implementation the fleet story uses.
    recordServedPage(this, peerName, {
      urlPath: `${record.mountPath.replace(/\/$/, '')}/`,
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
    assert.ok(
      record.serverBlobHash,
      'no server bundle was staged, so there is no pointer to converge'
    );
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
    assert.ok(record.serverDeclaredAt, 'no server declaration starts the propagation deadline');
    const seen = new Map<string, string>();
    const elapsed = await pollUntil(
      async () => {
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
          if (pointer !== record.serverBlobHash) all = false;
        }
        return all;
      },
      bound * 1000 - (Date.now() - record.serverDeclaredAt)
    );
    const answered = [...seen].map(([name, value]) => `${name}=${value}`).join(', ');
    assert.ok(
      elapsed !== null,
      `the server pointer declared once on one peer did not reach every peer within ${bound}s. ` +
        `Declared ${record.serverBlobHash}; peers answered ${answered}. ` +
        "A pointer written straight into one peer's own database and told to nobody is the 2026-09-08 shape."
    );
  }
);

Then(
  'no peer other than the one that was told was written to by this run',
  function (this: E2EWorld) {
    const record = app(this);
    assert.equal(record.serverDeclarations, 1, 'the server head must be declared exactly once');
    assert.strictEqual(
      new Set(record.declaredThrough).size,
      1,
      `this run declared a head through ${record.declaredThrough.length} doorways ` +
        `(${record.declaredThrough.join(', ')}); the claim under test is that ONE declaration reaches every peer, ` +
        'so a second declaration would make the convergence above vacuous'
    );
  }
);

// ---------------------------------------------------------------------------
// Restart while a primary peer is down. Cleanup always restores the held peer.
// ---------------------------------------------------------------------------

Given(
  'doorway {string} can be restarted while peer {string} is held down',
  function (this: E2EWorld, doorway: string, peer: string) {
    assert.ok(DOORWAYS.includes(doorway));
    assert.ok(loadHouseholdMeshFixture().storagePeers?.[peer]?.url, `no owned peer ${peer}`);
  }
);

When(
  'doorway {string} restarts while peer {string} is down',
  { timeout: 180_000 },
  async function (this: E2EWorld, doorway: string, peer: string) {
    const record = app(this);
    record.heldPeers ??= new Set();
    record.heldPeers.add(peer);
    this.onCleanup(async () => {
      if (record.heldPeers?.has(peer)) await meshControl('storage-restart', peer);
    });
    const learned = await pollUntil(
      async () => {
        const row = await fetch(
          `${loadHouseholdMeshFixture().storagePeers![peer].url}/db/content/${record.slug}`
        );
        return ((await row.json()) as { blobHash?: string }).blobHash === record.blobHash;
      },
      Math.max(0, CONVERGENCE_BOUND_MS - (Date.now() - record.browserDeclaredAt!))
    );
    assert.notEqual(learned, null, `${peer} never learned the new head before the outage`);
    await meshControl('storage-stop', peer);
    const url = loadHouseholdMeshFixture().storagePeers![peer].url;
    const alive = await fetch(`${url}/health`, { signal: AbortSignal.timeout(2000) }).then(
      () => true,
      () => false
    );
    assert.equal(alive, false, `${peer} must refuse connections before doorway restarts`);
    const name = doorway === 'alpha-A' ? 'a' : 'b';
    const logOffset = doorwayRestartLog(name, this).length;
    await restartStoryDoorway(this, name);
    record.restarts = (record.restarts ?? 0) + 1;
    const ready = await pollUntil(
      async () =>
        (await getRaw(`${resolvePeerUrl(doorway)}/health`, { timeoutMs: 2000 })).status === 200,
      60_000
    );
    assert.notEqual(ready, null, `${doorway} did not boot while ${peer} was down`);
    const failedLookup = await pollUntil(
      async () =>
        await Promise.resolve(
          doorwayRestartLog(name, this)
            .slice(logOffset)
            .split('\n')
            .some(line => line.includes(record.slug) && line.includes('content GET failed'))
        ),
      30_000
    );
    assert.notEqual(
      failedLookup,
      null,
      `${doorway} did not attempt its initial head lookup during the outage`
    );
  }
);

When(
  'peer {string} comes back',
  { timeout: 180_000 },
  async function (this: E2EWorld, peer: string) {
    assert.ok(app(this).heldPeers?.has(peer), `${peer} was not held down`);
    const restart = meshControl('storage-restart', peer).then(
      () => null,
      error => error as Error
    );
    const storageUrl = loadHouseholdMeshFixture().storagePeers?.[peer]?.url;
    assert.ok(storageUrl);
    const restartError = await restart;
    assert.equal(restartError, null, `peer restart failed: ${String(restartError)}`);
    // The maintained arm owns its readiness horizon and returns only after the
    // storage peer is ready. Check that completed state afresh: a concurrent
    // shorter poll can expire while the arm is still making valid progress and
    // leave a stale failure result even though the restart succeeds.
    const health = await getRaw(`${storageUrl}/health`, { timeoutMs: 2000 });
    assert.equal(health.status, 200, `${peer} did not become reachable`);
    app(this).recoveredAt = Date.now();
    app(this).heldPeers!.delete(peer);
  }
);

Then('nobody cleared a cache or restarted anything to make that happen', function (this: E2EWorld) {
  assert.equal(
    app(this).restarts,
    1,
    'recovery must use exactly one doorway restart and no cache-clear request'
  );
  assert.equal(app(this).heldPeers?.size, 0);
});

// ---------------------------------------------------------------------------
// The bundle that cannot boot (Act I only)
// ---------------------------------------------------------------------------

When('this run builds a second bundle with its entry script removed', function (this: E2EWorld) {
  const record = app(this);
  const broken = buildFixtureBundle({
    coherent: false,
    baseHref: `${record.mountPath.replace(/\/$/, '')}/`,
  });
  this.onCleanup(async () => {
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
    // The SDK may reject before upload. The next step explicitly injects an
    // unchecked archive only for the old-publisher defense test.
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
    if (!record.brokenBlobHash) {
      assert.ok(record.brokenBundle, 'no deliberately broken fixture was built');
      record.brokenBlobHash = await stageInvalidFixture(
        record.brokenBundle,
        resolvePeerUrl('alpha-A')
      );
    }
    const storageUrl = resolveStorageUrl('alpha-A');
    assert.ok(storageUrl, 'no direct storage URL for peer "alpha-A" — set E2E_STORAGE_URL');
    const response = await postFixtureCommitment(`${storageUrl}/db/content/${record.slug}`, {
      method: 'PATCH',
      headers: {
        'content-type': 'application/json',
        'X-API-Key': process.env['STORAGE_API_KEY_ADMIN'] ?? '',
      },
      body: JSON.stringify({ blobHash: record.brokenBlobHash }),
    });
    assert.ok(
      response.ok,
      `could not force the incoherent head onto "${record.slug}": ${response.status} ${response.text}`
    );
    // Match the publisher's canonical declaration too. A projection-only PATCH
    // is correctly healed back to the prior canonical head and proves nothing.
    const rowResponse = await fetch(`${storageUrl}/db/content/${record.slug}`);
    const row = (await rowResponse.json()) as { blobHash?: string; dhtAnchorHash?: string };
    assert.equal(
      row.blobHash,
      record.brokenBlobHash,
      `the forced head for "${record.slug}" did not resolve to the staged incoherent blob`
    );
    assert.ok(row.dhtAnchorHash, 'the forced head has no notarized action to declare');
    const canonical = await fetch(`${storageUrl}/db/content/${record.slug}/canonical-head`, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        'X-API-Key': process.env['STORAGE_API_KEY_ADMIN'] ?? '',
      },
      body: JSON.stringify({ headActionHash: row.dhtAnchorHash }),
    });
    assert.ok(canonical.ok, `forced canonical head: ${canonical.status} ${await canonical.text()}`);
    record.forcedDeclaredAt = Date.now();
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
    const visit = await visitInBrowser(url);
    assert.ok(
      visit.bootstrapReady && visit.rootPresent && visit.rootText.trim().length > 0,
      'Previous shell did not complete bootstrap'
    );
    assert.deepEqual(visit.pageErrors, [], 'Previous shell raised browser errors');
    assert.deepEqual(
      visit.httpErrors.filter(entry => sameOrigin(url, entry.url)),
      [],
      'Fallback asset HTTP errors'
    );
    assert.deepEqual(
      visit.failedRequests.filter(entry => sameOrigin(url, entry.url)),
      [],
      'Fallback asset failures'
    );
    assert.ok(
      visit.rootText.includes(bundle.stamp.replace(/^fixture-/, '')),
      'Fallback did not boot the previous build'
    );
  }
);

Then(
  'doorway {string} says on the wire that it is behind and names the missing file',
  { timeout: CONVERGENCE_BOUND_MS + 30_000 },
  async function (this: E2EWorld, peerName: string) {
    const record = app(this);
    const broken = record.brokenBundle;
    assert.ok(broken, 'no incoherent bundle was built');
    const url = appUrl(this, peerName);
    let marker = '';
    assert.ok(record.forcedDeclaredAt);
    const observed = await pollUntil(
      async () => {
        const { headers } = await getRawWithHeaders(url, { timeoutMs: 10_000 });
        marker = headers['x-elohim-bundle'] ?? '';
        return marker.startsWith('behind') && marker.includes(broken.entryScript);
      },
      Math.max(0, CONVERGENCE_BOUND_MS - (Date.now() - record.forcedDeclaredAt))
    );
    assert.notEqual(observed, null, 'Doorway did not judge the forced head within 75 seconds');
    const declared = await fetch(`${resolveStorageUrl('alpha-A')}/db/content/${record.slug}`);
    assert.equal(
      ((await declared.json()) as { blobHash?: string }).blobHash,
      record.brokenBlobHash
    );
    assert.ok(
      marker.startsWith('behind'),
      `${peerName} served this app with x-elohim-bundle "${marker || 'absent'}". A doorway serving an ` +
        'older version because the current one cannot boot must say so on the wire, or the only way to ' +
        'discover it is for a person to notice the page is old.'
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
    assert.equal(
      declared.status,
      200,
      `${peerName}: declared head ${declaredHead} has no version.json; cannot prove head consistency`
    );
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
    const url = `${resolvePeerUrl(peerName)}${record.mountPath.replace(/\/$/, '')}/version.json`;
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

// Each trip is measured independently: pointer, bytes, adopted renderer, HTTP output.
Then(
  'every household peer serves the declared server bundle bytes by their content address',
  { timeout: 180_000 },
  async function (this: E2EWorld) {
    const hash = app(this).serverBlobHash;
    assert.ok(hash);
    const fixture = loadHouseholdMeshFixture();
    for (const peer of householdPeers.get(this)!.all) {
      const response: Response = await fetch(`${fixture.storagePeers![peer].url}/blob/${hash}`, {
        signal: AbortSignal.timeout(60_000),
      });
      assert.equal(response.status, 200, `${peer}: cannot serve server bundle ${hash}`);
      const bytes = Buffer.from(await response.arrayBuffer());
      assert.equal(
        `sha256-${createHash('sha256').update(bytes).digest('hex')}`,
        hash,
        `${peer}: server bundle bytes do not match declared content address`
      );
    }
  }
);

Given(
  'both doorways are configured to render this run-owned site',
  { timeout: 180_000 },
  async function (this: E2EWorld) {
    for (const doorway of ['a', 'b']) await restartStoryDoorway(this, doorway, app(this).slug);
    app(this).rendererAdoptionAt = Date.now();
  }
);

Then(
  'within {int} seconds both doorways attest they materialized that server pointer for this app',
  { timeout: 360_000 },
  async function (this: E2EWorld, seconds: number) {
    const record = app(this);
    assert.ok(record.rendererAdoptionAt, 'no publication/configuration starts renderer adoption');
    const deadline = record.rendererAdoptionAt + seconds * 1000;
    await Promise.all(
      DOORWAYS.map(async peer => {
        let last = '';
        const elapsed = await pollUntil(async () => {
          const response = await getRaw(`${resolvePeerUrl(peer)}/health/startup`, {
            timeoutMs: 5000,
          });
          last = response.text;
          if (response.status !== 200) return false;
          const heads = (
            JSON.parse(last) as {
              servedBundleHeads: { slug: string; serverBlobHash: string; status: string }[];
            }
          ).servedBundleHeads;
          return heads.some(
            head =>
              head.slug === record.slug &&
              head.serverBlobHash === record.serverBlobHash &&
              head.status === 'current'
          );
        }, deadline - Date.now());
        assert.notEqual(
          elapsed,
          null,
          `${peer}: expected renderer ${record.serverBlobHash}; health=${last}`
        );
      })
    );
  }
);

Then(
  'both doorways return that server-rendered build before any browser script runs',
  { timeout: 90_000 },
  async function (this: E2EWorld) {
    const deadline = Date.now() + 85_000;
    for (const peer of DOORWAYS) {
      let lastStatus = 0;
      let lastHeaders: Record<string, string | undefined> = {};
      const rendered = await pollUntil(
        async () => {
          const response = await getRawWithHeaders(appUrl(this, peer), { timeoutMs: 10_000 });
          lastStatus = response.status;
          lastHeaders = response.headers;
          return (
            response.status === 200 &&
            response.text.includes(`data-ssr-stamp="${requireBundle(this).stamp}"`)
          );
        },
        Math.max(0, deadline - Date.now())
      );
      assert.notEqual(
        rendered,
        null,
        `${peer}: SSR output did not adopt this server build within the bound; ` +
          `status=${lastStatus} headers=${JSON.stringify(lastHeaders)}`
      );
    }
  }
);

Then(
  'both warm doorways keep serving that rendered build while all storage peers are down',
  { timeout: 600_000 },
  async function (this: E2EWorld) {
    const held: string[] = [];
    try {
      for (const peer of householdPeers.get(this)!.all) {
        held.push(peer);
        await meshControl('storage-stop', peer);
        const storageUrl = loadHouseholdMeshFixture().storagePeers?.[peer]?.url;
        assert.ok(storageUrl, `no address for ${peer}`);
        const alive = await fetch(`${storageUrl}/health`, {
          signal: AbortSignal.timeout(2000),
        }).then(
          () => true,
          () => false
        );
        assert.equal(alive, false, `${peer} still accepts requests; outage not established`);
      }
      const visitorsStarted = Date.now();
      for (const peer of DOORWAYS) {
        for (let visitor = 0; visitor < 3; visitor++) {
          const response = await getRawWithHeaders(appUrl(this, peer), {
            timeoutMs: Math.max(1, 10_000 - (Date.now() - visitorsStarted)),
          });
          assert.ok(
            Date.now() - visitorsStarted <= 10_000,
            'Six cached requests exceeded ten seconds'
          );
          assert.equal(response.status, 200, `${peer}: warm page stopped serving without peers`);
          assert.equal(
            response.headers['x-render-cache'],
            'HIT',
            `${peer}: repeated visitor was not served from the doorway render cache`
          );
          assert.ok(
            response.text.includes(`data-ssr-stamp="${requireBundle(this).stamp}"`),
            `${peer}: warm visitor did not receive the adopted SSR build`
          );
        }
      }
    } finally {
      // Attempt every restore even if a preceding peer fails to start.
      const restores = await Promise.allSettled(
        held.map(async peer => meshControl('storage-restart', peer))
      );
      assert.ok(
        restores.every(result => result.status === 'fulfilled'),
        `storage restore failures: ${restores
          .filter(result => result.status === 'rejected')
          .map(result => String(result.reason))
          .join('; ')}`
      );
    }
  }
);

Then(
  'the browser on peer {string} completed client bootstrap',
  function (this: E2EWorld, peer: string) {
    assert.equal(
      requireVisit(this, peer).bootstrapReady,
      true,
      `${peer}: client bootstrap marker was absent; SSR text alone cannot pass`
    );
  }
);

Then(
  'neither doorway restarted while adopting the next rendered version',
  function (this: E2EWorld) {
    assert.ok(app(this).upgradeIncarnations, 'no running-renderer baseline was captured');
    assert.deepEqual(
      doorwayIncarnations(this),
      app(this).upgradeIncarnations,
      'renderer upgrade must happen in the same doorway process incarnation'
    );
  }
);

Then(
  'within {int} seconds both doorways serve the new browser bundle by content address',
  { timeout: 105_000 },
  async function (this: E2EWorld, seconds: number) {
    const record = app(this);
    const bundle = requireBundle(this);
    assert.ok(record.browserDeclaredAt && record.blobHash);
    const deadline = record.browserDeclaredAt + seconds * 1000;
    await Promise.all(
      DOORWAYS.map(async peer => {
        const ready = await pollUntil(
          async () => {
            const base = resolvePeerUrl(peer);
            const row = await getRaw(`${base}/db/content/${record.slug}`, { timeoutMs: 5000 });
            if (
              row.status !== 200 ||
              (JSON.parse(row.text) as { blobHash?: string }).blobHash !== record.blobHash
            )
              return false;
            const paths = ['index.html', bundle.entryScript, bundle.styleSheet, 'version.json'];
            const files = await Promise.all(
              paths.map(async path =>
                getRaw(`${base}/apps/${record.blobHash}/${path}`, { timeoutMs: 5000 })
              )
            );
            return (
              files.every(file => file.status === 200) &&
              files[0].text.includes(bundle.entryScript) &&
              files[3].text.includes(bundle.stamp)
            );
          },
          Math.max(0, deadline - Date.now())
        );
        assert.notEqual(
          ready,
          null,
          `${peer}: next browser declaration and immutable bytes did not converge within ${seconds}s`
        );
      })
    );
  }
);

// This After hook must fail the scenario if restoration fails; world's generic
// cleanup callbacks are best-effort and cannot attest ownership restoration.
After({ tags: '@deliverability-browser', timeout: 180_000 }, async function (this: E2EWorld) {
  const record = publishedApps.get(this);
  if (!record?.rootCommitments?.length) return;
  // Every root commitment here was minted through ONE storage endpoint: "an
  // EPR record this run owns for it" (above) always posts via
  // resolveStorageUrl('alpha-A') — matthew's storage — regardless of which
  // doorway's hosting path the mount loop is on. The zome's commitment
  // lifecycle guard ("recover original bindings and observed commitment
  // lifecycle", 2cea494ee) restricts an update to the root's AUTHOR, so
  // cancelling through any other household peer's storage endpoint is
  // correctly refused with "Only the Commitment root author may update its
  // observed lifecycle" — that is expected behavior, not a fault to route
  // around. Cancel through the author peer (alpha-A / matthew) only.
  const authorStorageUrl = resolveStorageUrl('alpha-A');
  assert.ok(
    authorStorageUrl,
    'no direct storage URL for peer "alpha-A" (the root commitment author) — set E2E_STORAGE_URL'
  );
  const restores = await Promise.allSettled(
    record.rootCommitments.map(async id => {
      const response = await cancelOwnedCommitmentWithReadback(
        id,
        async () =>
          await postFixtureCommitment(`${authorStorageUrl}/api/v1/commitments/${id}`, {
            method: 'PATCH',
            headers: {
              'content-type': 'application/json',
              'X-API-Key': process.env['STORAGE_API_KEY_ADMIN'] ?? 'mesh-admin-dev-key',
            },
            body: JSON.stringify({ state: 'cancelled', finished: true }),
          }),
        async () => {
          const readback = await fetch(`${authorStorageUrl}/api/v1/commitments/${id}`);
          return readback.status === 200
            ? ((await readback.json()) as { id?: string; state?: string; finished?: boolean })
            : undefined;
        }
      );
      assert.ok(
        response.ok || response.status === 404,
        `alpha-A/matthew (root author): could not cancel owned root mount ${id}: ${response.status} ${response.text}`
      );
      if (response.ok) {
        const readback = await fetch(`${authorStorageUrl}/api/v1/commitments/${id}`);
        assert.equal(
          ((await readback.json()) as { state: string }).state,
          'cancelled',
          `alpha-A/matthew (root author): owned root commitment remains live`
        );
      }
    })
  );
  assert.ok(
    restores.every(result => result.status === 'fulfilled'),
    `owned root cleanup failed: ${restores
      .filter(result => result.status === 'rejected')
      .map(result => String(result.reason))
      .join('; ')}`
  );
  // A sibling-primary doorway may learn the author's cancellation only after
  // DHT propagation and its next 30s projection refresh. Use the same two-tick
  // convergence contract as head publication so refresh phase cannot expire
  // the cleanup check just before the observation that clears the route.
  const deadline = Date.now() + CONVERGENCE_BOUND_MS;
  await Promise.all(
    DOORWAYS.map(async peer => {
      const restored = await pollUntil(
        async () => {
          const response = await fetch(`${resolvePeerUrl(peer)}/api/v1/federation/coherence`);
          const state = (await response.json()) as { heads: { urlPath: string }[] };
          return !state.heads.some(head => head.urlPath === '/');
        },
        Math.max(0, deadline - Date.now())
      );
      assert.notEqual(
        restored,
        null,
        `${peer}: temporary root mount still projected after cleanup`
      );
    })
  );
});
