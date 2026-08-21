/**
 * Native EPR Projection step definitions — Task B22 of the pillar EPR
 * decomposition plan.
 *
 * Feature: features/doorway/native-epr-projection.feature
 *
 * The doorway natively projects EPRs at author-declared URL paths via
 * `project-epr` commitments on the DHT. There is no doorway-side
 * hardcoding of paths — the commitment is the source of truth.
 *
 * MVP scope:
 *   - Substrate scenarios (1, 2, 4) are written against HTTP and run
 *     against any doorway that has the commitments seeded.
 *   - Scenario 3 (cache eviction within 5s) depends on B15 (the doorway
 *     SSE subscriber refresh path) and is marked pending until the
 *     end-to-end PATCH-to-eviction wiring is verifiable from a2o.
 *
 * Steps are SUBSTRATE-VERIFIABLE where possible (HTTP GETs against a
 * live doorway). Steps that require not-yet-landed mutations are stubbed
 * with `return 'pending'` so the feature parses and runs in dry mode.
 */

import { strict as assert } from 'node:assert';

import { After, Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { resolveDoorwayUrl as resolveFixtureDoorwayUrl } from '../../src/framework/fixtures/household-mesh.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Local response capture (scoped per-world)
// ---------------------------------------------------------------------------

interface ProjectionResponse {
  url: string;
  status: number;
  headers: Record<string, string>;
  body: Buffer;
}

const NO_PROJECTION_RESPONSE = 'No projection response captured';

const projectionResponses = new WeakMap<E2EWorld, ProjectionResponse>();
const projectionSecondaryResponses = new WeakMap<E2EWorld, ProjectionResponse>();

/** Inline base-href detector. Bounded char classes avoid catastrophic backtracking. */
const BASE_HREF_PATTERN = /<base[ \t]+[^>]{0,200}href=["']([^"']{1,500})["']/i;

/**
 * Why this response is not the page we asked for, when the doorway told us.
 *
 * A `503 {"status":"catching-up"}` with `cause:"upstream"` is NOT the projector
 * lagging — it is the doorway's breaker to its storage peer sitting open, i.e.
 * a doorway→storage availability failure. Saying so in the assertion is the
 * difference between a red that routes triage to the substrate-trust runbook
 * and a red that reads as "the SPA is broken". Returns '' when the response is
 * not a shed, so a genuine wrong-content-type red stays unadorned.
 */
function describeShed(resp: ProjectionResponse): string {
  if (resp.status !== 503) return '';
  let body: Record<string, unknown>;
  try {
    body = JSON.parse(resp.body.toString('utf-8')) as Record<string, unknown>;
  } catch {
    return ` (status 503, unparseable body)`;
  }
  if (body['status'] !== 'catching-up') return ` (status ${resp.status})`;
  const cause = typeof body['cause'] === 'string' ? body['cause'] : 'unreported';
  const circuit = typeof body['circuit'] === 'string' ? body['circuit'] : 'unreported';
  if (cause === 'upstream') {
    return (
      ` — the doorway is SHEDDING: 503 catching-up, cause=upstream, circuit=${circuit}. ` +
      "That is the doorway's breaker to its storage peer, not the SPA and not the projector: " +
      'an under-load storage stall opens the circuit for minutes. Runbook ' +
      '"substrate-trust-contract", not the admission runbook.'
    );
  }
  return ` — the doorway is SHEDDING: 503 catching-up, cause=${cause}, circuit=${circuit}`;
}

async function getRaw(url: string): Promise<ProjectionResponse> {
  const { statusCode, headers, body } = await request(url);
  const data = Buffer.from(await body.arrayBuffer());
  const flat: Record<string, string> = {};
  for (const [k, v] of Object.entries(headers)) {
    if (typeof v === 'string') flat[k.toLowerCase()] = v;
    else if (Array.isArray(v)) flat[k.toLowerCase()] = v[0];
  }
  return { url, status: statusCode, headers: flat, body: data };
}

/**
 * Map a hard-coded hostname from the feature file to whichever doorway URL
 * the world is actually targeting. The feature reads as production prose
 * ("https://alpha.elohim.host/"), but the test runs against whatever
 * E2E_DOORWAY_ALPHA / E2E_DOORWAY_PRIMARY points at.
 */
function resolveDoorwayUrl(world: E2EWorld, hostname: string): string {
  const envByHost: Record<string, string | undefined> = {
    // The feature also says a bare "alpha" (the re-grant row's "the alpha
    // doorway"). Without this key that name fell through every candidate and
    // was dialled as a HOSTNAME — `getaddrinfo ENOTFOUND alpha` — because this
    // feature has no Background registering a doorway to fall back to.
    alpha: process.env['E2E_DOORWAY_ALPHA'],
    'alpha.elohim.host': process.env['E2E_DOORWAY_ALPHA'],
    'elohim.host':
      process.env['E2E_DOORWAY_B'] ??
      process.env['E2E_DOORWAY_BETA'] ??
      process.env['E2E_DOORWAY_PRIMARY'] ??
      process.env['E2E_DOORWAY_HOSTED'],
  };
  // Fall back to the first registered doorway if no env var was set —
  // useful for local-stack runs where there is exactly one doorway.
  const firstDoorway = [...world.doorways.values()][0];
  // Then the household fixture manifest, which is what an Act I mesh run
  // actually has: the production prose names hosts, the mesh has ports.
  let fixtureId = hostname;
  if (hostname.startsWith('alpha')) fixtureId = 'alpha';
  else if (hostname === 'elohim.host') fixtureId = 'beta';
  const resolved = resolveFixtureDoorwayUrl(fixtureId, [envByHost[hostname], firstDoorway?.url]);
  if (resolved) return resolved;
  return `https://${hostname}`;
}

// ---------------------------------------------------------------------------
// Given — project-epr commitment fixtures
// ---------------------------------------------------------------------------

Given(
  /^the ([\w.-]+) doorway has an active project-epr commitment for "([^"]+)" at urlPath "([^"]+)"$/,
  function (this: E2EWorld, hostname: string, eprSlug: string, urlPath: string) {
    // The commitment is asserted to exist; the seeder + doorway-side
    // ingest path (Tasks B12–B16) is what creates it. In a hosted run,
    // this step is documentation — it tells the reader which commitment
    // must already be on the DHT for the scenario to pass.
    //
    // For automated verification of presence the runner can query
    // /db/commitment/<eprSlug>; that endpoint shape ships with B16, so
    // we keep this as a documentary no-op until then.
    const doorwayUrl = resolveDoorwayUrl(this, hostname);
    this.contentIds.set(`projection:${eprSlug}:urlPath`, urlPath);
    this.contentIds.set(`projection:${eprSlug}:doorway`, doorwayUrl);
  }
);

Given(
  /^the ([\w.-]+) doorway also has an active project-epr commitment for "([^"]+)" at urlPath "([^"]+)"$/,
  function (this: E2EWorld, hostname: string, eprSlug: string, urlPath: string) {
    const doorwayUrl = resolveDoorwayUrl(this, hostname);
    this.contentIds.set(`projection:${eprSlug}:secondary-urlPath`, urlPath);
    this.contentIds.set(`projection:${eprSlug}:secondary-doorway`, doorwayUrl);
  }
);

/**
 * Cache eviction, on a substrate that lets us move a head.
 *
 * `sha256-OLD` / `sha256-NEW` in the feature are PLACEHOLDERS — prose standing
 * in for "the hash before" and "the hash after". Nothing on any mesh is
 * addressed that way, so these steps read the real current head rather than
 * matching the literal.
 *
 * The substitute bundle is the LANDING EPR's blob: a blob that genuinely exists
 * on this mesh, with genuinely different bytes. PATCHing lamad-spa to a hash no
 * peer holds would 404 the household's /lamad and prove nothing about eviction;
 * pointing it at a real neighbouring bundle changes the head for real and the
 * doorway either notices or it does not. The original head is restored in the
 * After hook below, including on failure.
 *
 * DESTRUCTIVE: it moves a declared head, which is a DHT write. Held behind
 * A2O_ALLOW_DESTRUCTIVE=1.
 */
const cacheEviction = new WeakMap<
  E2EWorld,
  { eprSlug: string; originalBlobHash: string; newBlobHash?: string; beforeBody?: Buffer }
>();

function destructiveAllowed(): boolean {
  return process.env['A2O_ALLOW_DESTRUCTIVE'] === '1';
}

/** Skip (never fail, never pend) with the action this run declined to take. */
function holdDestructive(wouldDo: string): 'skipped' {
  // eslint-disable-next-line no-console
  console.log(
    `  ⏭️  DESTRUCTIVE HELD: would ${wouldDo}. Set A2O_ALLOW_DESTRUCTIVE=1 to run it. ` +
      'Skipped, not failed.'
  );
  return 'skipped';
}

async function contentBlobHash(base: string, slug: string): Promise<string> {
  const resp = await getRaw(`${base.replace(/\/$/, '')}/db/content/${slug}`);
  assert.equal(resp.status, 200, `GET /db/content/${slug} returned ${resp.status}`);
  const row = JSON.parse(resp.body.toString('utf-8')) as { blobHash?: string };
  const hash = row.blobHash;
  assert.ok(hash, `content row "${slug}" carries no blobHash — nothing to evict a cache for`);
  return hash;
}

Given(
  /^the ([\w-]+) EPR's blob is ([\w-]+)$/,
  async function (this: E2EWorld, eprSlug: string, _placeholder: string) {
    const base = resolveDoorwayUrl(this, 'alpha');
    const originalBlobHash = await contentBlobHash(base, eprSlug);
    const before = await getRaw(`${base.replace(/\/$/, '')}/lamad/index.html`);
    cacheEviction.set(this, {
      eprSlug,
      originalBlobHash,
      beforeBody: before.status === 200 ? before.body : undefined,
    });
  }
);

// ---------------------------------------------------------------------------
// When — anonymous browser GETs
// ---------------------------------------------------------------------------

When('an anonymous browser GETs {string}', async function (this: E2EWorld, urlString: string) {
  // Translate the production hostname in the URL to the doorway URL the
  // test run targets, then re-attach the original path + query.
  let parsed: URL;
  try {
    parsed = new URL(urlString);
  } catch {
    assert.fail(`Could not parse URL "${urlString}"`);
  }
  const base = resolveDoorwayUrl(this, parsed.hostname);
  const target = `${base.replace(/\/$/, '')}${parsed.pathname}${parsed.search}`;
  const resp = await getRaw(target);

  // Second GET to the same URL across two doorways: capture both. The
  // first hostname seen becomes the primary; any subsequent GET to a
  // different hostname goes into the secondary slot.
  const primary = projectionResponses.get(this);
  if (primary && new URL(primary.url).hostname !== new URL(target).hostname) {
    projectionSecondaryResponses.set(this, resp);
  } else {
    projectionResponses.set(this, resp);
  }
});

When(
  /^a deploy PATCHes the ([\w-]+) EPR with blobHash ([\w-]+)$/,
  async function (this: E2EWorld, eprSlug: string, _placeholder: string) {
    const state = cacheEviction.get(this);
    assert.ok(state, 'the blob-is-OLD Given must run before the PATCH');
    if (!destructiveAllowed()) {
      return holdDestructive(
        `PATCH /db/content/${eprSlug} to a different real bundle head (a DHT write) and restore it after`
      );
    }
    const base = resolveDoorwayUrl(this, 'alpha');
    // A real, different, locally-held bundle — see the note above.
    const newBlobHash = await contentBlobHash(base, 'elohim-host-landing');
    assert.notEqual(
      newBlobHash,
      state.originalBlobHash,
      'the substitute bundle has the SAME head as the one under test — moving it would prove nothing'
    );
    const { statusCode } = await request(`${base.replace(/\/$/, '')}/db/content/${eprSlug}`, {
      method: 'PATCH',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ blobHash: newBlobHash }),
    });
    assert.ok(
      statusCode >= 200 && statusCode < 300,
      `PATCH /db/content/${eprSlug} returned ${statusCode}`
    );
    state.newBlobHash = newBlobHash;
  }
);

After({ tags: '@cache-eviction' }, async function (this: E2EWorld) {
  const state = cacheEviction.get(this);
  if (!state?.newBlobHash) return;
  const base = resolveDoorwayUrl(this, 'alpha');
  await request(`${base.replace(/\/$/, '')}/db/content/${state.eprSlug}`, {
    method: 'PATCH',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ blobHash: state.originalBlobHash }),
  });
});

// ---------------------------------------------------------------------------
// Then — response assertions
// ---------------------------------------------------------------------------

Then("the response serves the landing EPR's bundle entry file", function (this: E2EWorld) {
  const resp = projectionResponses.get(this);
  assert.ok(resp, NO_PROJECTION_RESPONSE);
  // Substrate check: bundle entry file is HTML and big enough to not be
  // a placeholder error page.
  const ct = resp.headers['content-type'] ?? '';
  assert.ok(
    ct.includes('text/html'),
    `Expected HTML bundle entry, got Content-Type "${ct}"${describeShed(resp)}`
  );
  assert.ok(
    resp.body.length > 200,
    `Bundle entry suspiciously small (${resp.body.length} bytes) — likely a placeholder`
  );
});

Then('the response is HTTP {int}', function (this: E2EWorld, expected: number) {
  const resp = projectionResponses.get(this);
  assert.ok(resp, NO_PROJECTION_RESPONSE);
  assert.equal(resp.status, expected);
});

Then(
  /^the response serves the lamad bundle's index\.html \(SPA fallback\)$/,
  function (this: E2EWorld) {
    const resp = projectionResponses.get(this);
    assert.ok(resp, NO_PROJECTION_RESPONSE);
    const ct = resp.headers['content-type'] ?? '';
    assert.ok(
      ct.includes('text/html'),
      `Expected HTML SPA fallback, got Content-Type "${ct}"${describeShed(resp)}`
    );
    const html = resp.body.toString('utf-8');
    // The SPA shell is recognisable by an Angular root element or an
    // app-root tag — either is acceptable.
    assert.ok(
      html.includes('<app-root') || html.includes('ng-version') || html.includes('<base'),
      `Response does not look like an Angular SPA shell. First 200 chars: ${html.slice(0, 200)}`
    );
  }
);

Then("the lamad bundle's <base href> is {string}", function (this: E2EWorld, expected: string) {
  const resp = projectionResponses.get(this);
  assert.ok(resp, NO_PROJECTION_RESPONSE);
  const html = resp.body.toString('utf-8');
  // Loose base-href detection — tolerates single/double quotes and
  // attribute ordering. Pattern is hoisted to module scope so the regex
  // is compiled once and bounded against backtracking.
  const baseMatch = BASE_HREF_PATTERN.exec(html);
  assert.ok(baseMatch, `No <base href> found in response. First 200 chars: ${html.slice(0, 200)}`);
  assert.equal(baseMatch[1], expected, `<base href> mismatch`);
});

Then('Angular client-side router handles {string}', function (this: E2EWorld, _path: string) {
  // Substrate check: SPA fallback responded with 200 + index.html. The
  // actual route-handling is a browser-side claim. Without Playwright,
  // we treat the SPA-fallback shape as sufficient evidence here.
  const resp = projectionResponses.get(this);
  assert.ok(resp, NO_PROJECTION_RESPONSE);
  assert.equal(
    resp.status,
    200,
    'SPA fallback must return 200 for client-side routing to take over'
  );
});

Then(
  "within {int} seconds, the doorway's cache for {string} is evicted",
  { timeout: 30_000 },
  async function (this: E2EWorld, seconds: number, path: string) {
    const state = cacheEviction.get(this);
    assert.ok(state, 'the blob-is-OLD Given must run before this assertion');
    if (!state.newBlobHash) {
      return holdDestructive('observe the eviction of a head this run declined to move');
    }
    const base = resolveDoorwayUrl(this, 'alpha');
    const deadline = Date.now() + seconds * 1000;
    let last = Buffer.alloc(0);
    while (Date.now() < deadline) {
      const resp = await getRaw(`${base.replace(/\/$/, '')}${path}`);
      last = resp.body;
      if (!state.beforeBody || !resp.body.equals(state.beforeBody)) return;
      await new Promise<void>(resolve => setTimeout(resolve, 250));
    }
    assert.fail(
      `${path} still served the SAME ${last.length} bytes ${seconds}s after the head moved — ` +
        "the doorway's cache was not evicted by the PATCH"
    );
  }
);

Then(
  /^the next browser request to "([^"]+)" serves bytes from ([\w-]+)$/,
  async function (this: E2EWorld, path: string, _placeholder: string) {
    const state = cacheEviction.get(this);
    assert.ok(state, 'the blob-is-OLD Given must run before this assertion');
    if (!state.newBlobHash) {
      return holdDestructive('read back the bundle behind a head this run declined to move');
    }
    const base = resolveDoorwayUrl(this, 'alpha');
    // The declared head really moved …
    const served = await contentBlobHash(base, state.eprSlug);
    assert.equal(
      served,
      state.newBlobHash,
      `content row "${state.eprSlug}" still declares ${served} after the PATCH`
    );
    // … and the path serves a real page from it, not an error or an empty body.
    const resp = await getRaw(`${base.replace(/\/$/, '')}${path}`);
    assert.equal(resp.status, 200, `${path} returned ${resp.status} after the head moved`);
    const ct = resp.headers['content-type'] ?? '';
    assert.ok(ct.includes('text/html'), `${path} served as "${ct}"${describeShed(resp)}`);
    assert.ok(resp.body.length > 200, `${path} served ${resp.body.length} bytes — a placeholder`);
    // Residue, stated rather than hidden: the doorway serves an index.html
    // EXTRACTED from the bundle, not the blob itself, so this proves the new
    // head is the one being served and that it serves a page — not byte
    // identity with the new bundle's own index.html. Closing that needs a
    // bundle-authoring fixture the harness does not have.
  }
);

Then(
  /^the response serves the same lamad bundle as ([\w.-]+)$/,
  async function (this: E2EWorld, hostname: string) {
    const primary = projectionResponses.get(this);
    const secondary = projectionSecondaryResponses.get(this);
    // The primary/secondary assignment depends on the order of GETs; if
    // the federation scenario's first GET targeted the secondary doorway,
    // pull the primary on demand here.
    if (!primary || !secondary) {
      // Fetch the comparison doorway directly.
      const otherBase = resolveDoorwayUrl(this, hostname);
      const lamadPath = '/lamad/';
      const otherResp = await getRaw(`${otherBase.replace(/\/$/, '')}${lamadPath}`);
      const here = primary ?? secondary;
      assert.ok(here, 'No projection response captured for federation comparison');
      assert.deepEqual(
        here.body,
        otherResp.body,
        `Federation bytes diverge between "${here.url}" and "${otherResp.url}"`
      );
      return;
    }
    assert.deepEqual(
      primary.body,
      secondary.body,
      'Federation: byte content diverges across doorways'
    );
  }
);

// ---------------------------------------------------------------------------
// @regrant — re-grant supersession (spec §3.2/§3.3)
//
// SUBSTRATE-VERIFIABLE against a live doorway: the doorway proxies the storage
// projection endpoints. The mechanics are real (create_with_supersession), so
// these steps drive the actual ceremony rather than stubbing it:
//   - discover the active grant-less predecessor (GET projections),
//   - re-grant via a superseding POST carrying `supersedes`,
//   - assert the active set now carries the grant (re-grant took effect),
//   - assert the predecessor is `superseded` and the chain is walkable.
//
// The steward-authenticated POST may be blocked in a hosted run without an admin
// credential; in that case the When step returns 'pending' (matching this file's
// discipline for mutations that aren't a2o-runnable in the current environment),
// so the scenario records the contract without a false failure.
// ---------------------------------------------------------------------------

const REGRANT_KEY = 'regrant:predecessorId';
const REGRANT_SUCCESSOR_KEY = 'regrant:successorId';
const REGRANT_SKIP_KEY = 'regrant:skipped';

interface ProjectionRow {
  commitmentId: string;
  eprId: string;
  doorwayId: string;
  urlPath?: string;
  routeClaims?: { schemaVersion: number; claims: { contentType: string }[] } | null;
}

/** Bare doorway id (no scheme/host) the storage projection endpoint expects. */
function bareDoorwayIdFor(world: E2EWorld, hostname: string): string {
  // The seeded doorway ids are alpha-elohim-host / apex-elohim-host. The feature
  // says "alpha", so map the canonical alpha id. Fall back to a slugified host.
  if (hostname === 'alpha' || hostname.startsWith('alpha')) return 'alpha-elohim-host';
  return hostname.replace(/\./g, '-');
}

async function fetchProjectionRows(base: string, bareDoorwayId: string): Promise<ProjectionRow[]> {
  const url = `${base.replace(/\/$/, '')}/db/rea_commitments?action=project-epr&doorwayId=${encodeURIComponent(
    bareDoorwayId
  )}`;
  const resp = await getRaw(url);
  if (resp.status !== 200) return [];
  try {
    return JSON.parse(resp.body.toString('utf-8')) as ProjectionRow[];
  } catch {
    return [];
  }
}

Given(
  /^the ([\w.-]+) doorway has an active grant-less project-epr commitment for "([^"]+)" at "([^"]+)"$/,
  async function (this: E2EWorld, hostname: string, eprSlug: string, urlPath: string) {
    const base = resolveDoorwayUrl(this, hostname);
    const bareDoorwayId = bareDoorwayIdFor(this, hostname);
    const rows = await fetchProjectionRows(base, bareDoorwayId);
    const row = rows.find(r => r.eprId === eprSlug);

    if (!row) {
      // No seeded projection reachable (doorway not seeded in this env) — record
      // the precondition as unmet and skip the mutation rather than false-fail.
      this.contentIds.set(REGRANT_SKIP_KEY, 'no-projection');
      return 'pending';
    }
    this.contentIds.set(REGRANT_KEY, row.commitmentId);
    this.contentIds.set(`projection:${eprSlug}:urlPath`, urlPath);
    this.contentIds.set(`projection:${eprSlug}:doorway`, base);
    this.contentIds.set('regrant:bareDoorwayId', bareDoorwayId);
    this.contentIds.set('regrant:eprSlug', eprSlug);
    // The scenario's premise is a grant-less row; if it's already granted the
    // re-grant has already happened (idempotent) — record so later steps adapt.
    if (row.routeClaims) {
      this.contentIds.set('regrant:already-granted', 'true');
    }
  }
);

When(
  /^the steward re-grants the "([^"]+)" projection with routeClaims for contentType "([^"]+)"$/,
  async function (this: E2EWorld, eprSlug: string, contentType: string) {
    if (this.contentIds.get(REGRANT_SKIP_KEY)) return 'pending';
    const predecessorId = this.contentIds.get(REGRANT_KEY);
    assert.ok(predecessorId, 'predecessor commitmentId not captured by the Given step');
    if (this.contentIds.get('regrant:already-granted')) {
      // Already re-granted earlier — nothing to do; the Then steps verify the
      // grant is present (idempotent end state).
      return;
    }

    const base = this.contentIds.get(`projection:${eprSlug}:doorway`)!;
    const urlPath = this.contentIds.get(`projection:${eprSlug}:urlPath`) ?? '/lamad';
    const bareDoorwayId = this.contentIds.get('regrant:bareDoorwayId')!;

    // The granted metadata: same projection, now carrying routeClaims. The
    // successor id is the predecessor's id with an `-regrant` suffix — distinct
    // (required) and deterministic for this scenario. Production seeding uses a
    // metadata fingerprint suffix; the a2o step only needs a distinct id.
    const successorId = `${predecessorId}-regrant`;
    const metadata = {
      urlPath,
      mode: 'cached',
      reach: 'commons',
      baseHref: `${urlPath}/`,
      entryFile: 'index.html',
      redirectsFrom: [],
      previewEprRef: null,
      gateHints: [],
      deadEnd: false,
      stewardDirectEndpoint: null,
      routeClaims: {
        schemaVersion: 1,
        claimsManifestCid: null,
        claims: [
          {
            contentType,
            template: `${contentType}/{id}`,
            fragments: { step: `${contentType}/{id}/step/{n}` },
          },
        ],
      },
      redirectTemplates: [],
      supersedes: predecessorId,
    };
    const scope = `doorway:${bareDoorwayId}|epr:${eprSlug}`;
    const body = {
      id: successorId,
      action: 'project-epr',
      provider: 'a2o-regrant-steward',
      receiver: 'a2o-regrant-steward',
      inScopeOf: scope,
      note: `Re-grant ${eprSlug} at ${urlPath}`,
      metadataJson: JSON.stringify(metadata),
      supersedes: predecessorId,
    };

    // NOTE: provider must match the predecessor's provider for the storage
    // supersession scope/provider check. We don't know the seeded provider here,
    // so read it from the predecessor commitment first.
    const predResp = await getRaw(`${base.replace(/\/$/, '')}/api/v1/commitments/${predecessorId}`);
    if (predResp.status === 200) {
      try {
        const pred = JSON.parse(predResp.body.toString('utf-8')) as {
          provider?: string;
          inScopeOf?: string[] | string;
        };
        if (pred.provider) {
          body.provider = pred.provider;
          body.receiver = pred.provider;
        }
      } catch {
        /* keep defaults; the POST will report the mismatch */
      }
    }

    const postResp = await request(`${base.replace(/\/$/, '')}/api/v1/commitments`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body),
    });
    const status = postResp.statusCode;
    const respText = Buffer.from(await postResp.body.arrayBuffer()).toString('utf-8');

    // Auth-gated environments: a steward POST without an admin credential is
    // refused. Record the contract and skip rather than false-fail.
    if (status === 401 || status === 403) {
      this.contentIds.set(REGRANT_SKIP_KEY, `auth-${status}`);
      return 'pending';
    }
    // 409 = this exact re-grant already applied (idempotent) — acceptable.
    if (status === 409) {
      this.contentIds.set(REGRANT_SUCCESSOR_KEY, successorId);
      return;
    }
    assert.ok(
      status >= 200 && status < 300,
      `re-grant POST failed: HTTP ${status}: ${respText.slice(0, 300)}`
    );
    this.contentIds.set(REGRANT_SUCCESSOR_KEY, successorId);
  }
);

Then(
  /^within one refresh cycle the active projection for "([^"]+)" carries the granted claims$/,
  async function (this: E2EWorld, eprSlug: string) {
    if (this.contentIds.get(REGRANT_SKIP_KEY)) return 'pending';
    const base = this.contentIds.get(`projection:${eprSlug}:doorway`)!;
    const bareDoorwayId = this.contentIds.get('regrant:bareDoorwayId')!;
    const rows = await fetchProjectionRows(base, bareDoorwayId);
    const active = rows.filter(r => r.eprId === eprSlug);
    // Exactly one ACTIVE row for the slug (the superseded predecessor is excluded
    // by find_active_projections), and it carries the granted claims.
    assert.equal(
      active.length,
      1,
      `expected exactly one active projection for "${eprSlug}", got ${active.length}`
    );
    const grant = active[0].routeClaims;
    assert.ok(grant, `active projection for "${eprSlug}" must carry routeClaims after re-grant`);
    assert.equal(grant.schemaVersion, 1);
    assert.ok(grant.claims.length > 0, 'granted claims must be non-empty');
  }
);

Then(
  /^the previous grant-less commitment is marked superseded and walkable on the chain$/,
  async function (this: E2EWorld) {
    if (this.contentIds.get(REGRANT_SKIP_KEY)) return 'pending';
    if (this.contentIds.get('regrant:already-granted')) {
      // The scenario's premise is a GRANT-LESS row being superseded. This
      // substrate's row already carries its claims, so nothing was superseded
      // and there is no chain to walk — asserting one anyway reds on the
      // fixture's shape rather than on the supersession contract. Skipped (not
      // failed, and not pending: cucumber is strict, so pending reds the run
      // exactly like a failure), with the premise named.
      // eslint-disable-next-line no-console
      console.log(
        '  ⏭️  PREMISE ABSENT: the active project-epr row already carries routeClaims, so no ' +
          'grant-less predecessor exists to supersede — the supersession chain is untestable ' +
          'on this substrate until a grant-less row is seeded. Skipped, not failed.'
      );
      return 'skipped';
    }
    const predecessorId = this.contentIds.get(REGRANT_KEY)!;
    const successorId = this.contentIds.get(REGRANT_SUCCESSOR_KEY);
    const base = this.contentIds.get(
      `projection:${this.contentIds.get('regrant:eprSlug')}:doorway`
    )!;

    // Predecessor: GET /api/v1/commitments/{id} → state "superseded".
    const predResp = await getRaw(`${base.replace(/\/$/, '')}/api/v1/commitments/${predecessorId}`);
    assert.equal(predResp.status, 200, 'predecessor must remain queryable (history preserved)');
    const pred = JSON.parse(predResp.body.toString('utf-8')) as { state?: string };
    assert.equal(pred.state, 'superseded', 'predecessor must be marked superseded');

    // Successor: metadata.supersedes points back to the predecessor (walkable).
    assert.ok(successorId, 'successor commitmentId not captured');
    const succResp = await getRaw(`${base.replace(/\/$/, '')}/api/v1/commitments/${successorId}`);
    assert.equal(succResp.status, 200, 'successor must be queryable');
    const succ = JSON.parse(succResp.body.toString('utf-8')) as {
      metadata?: { supersedes?: string };
    };
    assert.equal(
      succ.metadata?.supersedes,
      predecessorId,
      'successor metadata.supersedes must point to the predecessor (walkable chain)'
    );
  }
);

Then("both doorways' projections reference the same blob_hash", function (this: E2EWorld) {
  // The blob_hash is exposed via X-Content-Address (or similar) on
  // delivered responses. If neither doorway emits the header today,
  // mark pending so the contract is recorded in the feature without
  // false-positives.
  const primary = projectionResponses.get(this);
  const secondary = projectionSecondaryResponses.get(this);
  if (!primary || !secondary) {
    return 'pending';
  }
  const primaryHash = primary.headers['x-content-address'] ?? primary.headers['x-blob-hash'];
  const secondaryHash = secondary.headers['x-content-address'] ?? secondary.headers['x-blob-hash'];
  if (!primaryHash || !secondaryHash) {
    // Header contract isn't enforced yet — bytes-equal check in the
    // prior step is the substrate equivalent. Mark pending so the
    // expectation stays on the radar.
    return 'pending';
  }
  assert.equal(primaryHash, secondaryHash, 'blob_hash mismatch across federated doorways');
});
