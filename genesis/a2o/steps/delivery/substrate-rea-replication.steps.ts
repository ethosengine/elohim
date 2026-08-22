/**
 * Substrate-REA replication regression steps — guards Tasks 1–10 of
 * genesis/docs/superpowers/plans/2026-05-26-substrate-rea-replication-fix.md.
 *
 * Scope: the two scenarios in features/delivery/spa-bundle-delivery.feature
 * tagged @substrate-rea-replication-fix — both assert that project-epr
 * commitments for lamad-spa propagated to every alpha storage peer (proof
 * of the substrate-correct write path through ContentStore::create_commitment)
 * and that the doorway can serve /lamad regardless of which storage peer the
 * EprRouter happens to read from.
 *
 * Substrate verification (scenario 1) is done via HTTP probes against the
 * configured alpha doorway: GET /api/v1/commitments?action=project-epr filters
 * to lamad-spa rows and confirms a non-null dhtAnchorHash on every row.
 *
 * Pod-restart verification (scenario 2) requires kubectl access (which a2o
 * does not have); those steps `return 'pending'` so the scenario is a
 * recorded contract rather than a false-positive pass.
 *
 * Response capture is shared with steps/delivery.steps.ts via the exported
 * responseStore WeakMap so the existing `Then('the response status is {int}')`
 * assertion resolves against captures written here.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import {
  loadHouseholdMeshFixture,
  requireFixtureDoorwayUrl,
  requireFixturePoolStorageUrls,
} from '../../src/framework/fixtures/household-mesh.js';
import { doorwayPidForPort, reexecProcess } from '../../src/framework/fixtures/process-control.js';
import {
  destructiveAllowed,
  DESTRUCTIVE_HELD_HINT,
} from '../../src/framework/fixtures/substrate-scope.js';
import { E2EWorld } from '../../src/framework/world.js';
import { fetchApp, responseStore } from '../delivery.steps.js';

const NO_DOORWAY = 'No doorway registered — run the doorway background step first';
const NO_RESPONSE = 'No response captured — run a When-request step first';

function firstDoorwayUrl(world: E2EWorld): string {
  const doorway = [...world.doorways.values()][0];
  assert.ok(doorway, NO_DOORWAY);
  return doorway.url.replace(/\/$/, '');
}

async function fetchJson(
  baseUrl: string,
  path: string
): Promise<{ status: number; body: unknown }> {
  const { statusCode, body } = await request(`${baseUrl}${path}`);
  const text = await body.text();
  let parsed: unknown = null;
  try {
    parsed = JSON.parse(text);
  } catch {
    parsed = text;
  }
  return { status: statusCode, body: parsed };
}

// ---------------------------------------------------------------------------
// Given — SPA-bundle and commitment fixtures
// ---------------------------------------------------------------------------

/**
 * Confirm the doorway has the SPA bundle extracted and cached.
 *
 * Substrate check: GET /apps/{slug}/index.html returns 200 with HTML.
 * That's the canonical signal that the doorway has resolved the spa-bundle
 * content node and unpacked the zip blob into its projection cache.
 */
Given(
  'the SPA bundle for {string} is extracted and cached in doorway {string}',
  async function (this: E2EWorld, contentSlug: string, _doorwayId: string) {
    const base = firstDoorwayUrl(this);
    const resp = await fetchApp(base, `/apps/${contentSlug}/index.html`);
    assert.equal(
      resp.status,
      200,
      `SPA bundle "${contentSlug}" not extracted on ${base} (status ${resp.status})`
    );
    const ct = resp.headers['content-type'] ?? '';
    assert.ok(
      ct.includes('text/html'),
      `SPA bundle "${contentSlug}" served as "${ct}" — expected text/html`
    );
  }
);

/**
 * Substrate proof that the substrate-correct write path landed the
 * project-epr commitment on the DHT — every alpha storage peer is expected
 * to see at least one project-epr commitment whose epr_slug matches.
 *
 * a2o only has one doorway endpoint visible; we treat the doorway's view as
 * the authoritative read because the doorway forwards to a single storage
 * peer and that peer is selected non-deterministically across boots. If the
 * commitment is missing from any peer, the doorway-mediated query would
 * return [] from at least one boot — exactly the failure mode the
 * substrate-rea fix closes.
 */
Given(
  'the project-epr commitment for {string} has propagated across all alpha storage peers via DHT gossip',
  async function (this: E2EWorld, eprSlug: string) {
    const base = firstDoorwayUrl(this);
    const { status, body } = await fetchJson(base, '/api/v1/commitments?action=project-epr');
    assert.equal(
      status,
      200,
      `GET /api/v1/commitments?action=project-epr returned ${status} (expected 200)`
    );
    assert.ok(
      Array.isArray(body),
      `Expected array body from /api/v1/commitments, got ${typeof body}`
    );
    const rows = body as Record<string, unknown>[];
    const matching = rows.filter(row => {
      const slug = String(row['eprSlug'] ?? row['epr_slug'] ?? '');
      const inScope = JSON.stringify(row['inScopeOf'] ?? row['in_scope_of'] ?? '');
      return slug === eprSlug || inScope.includes(eprSlug);
    });
    assert.ok(
      matching.length > 0,
      `No project-epr commitment found for "${eprSlug}" — substrate-correct write may not have propagated. Sample row: ${JSON.stringify(rows[0] ?? null).slice(0, 200)}`
    );
    // Stash for the next step's dhtAnchorHash check.
    (this as unknown as Record<string, unknown>)[`__substrateRea:${eprSlug}`] = matching;
  }
);

/**
 * Verify every persisted commitment row carries a real dhtAnchorHash —
 * the marker that ContentStore::create_commitment notarized it on the DHT
 * before diesel inserted the projection row. A null dhtAnchorHash means
 * the row was written diesel-direct (the pre-fix failure mode).
 */
Given(
  /^the commitment row has a non-null dhtAnchorHash on every peer \(proof of substrate-correct write\)$/,
  function (this: E2EWorld) {
    const stashed = this as unknown as Record<string, unknown>;
    const key = Object.keys(stashed).find(k => k.startsWith('__substrateRea:'));
    assert.ok(key, 'No prior commitment rows captured — run the "has propagated" step first');
    const rows = stashed[key] as Record<string, unknown>[];
    for (const row of rows) {
      const anchor = row['dhtAnchorHash'] ?? row['dht_anchor_hash'];
      assert.ok(
        anchor && typeof anchor === 'string' && anchor.length > 0,
        `Commitment ${JSON.stringify(row['id'] ?? '?')} has null/empty dhtAnchorHash — diesel-direct write, substrate-rea fix not applied. Row: ${JSON.stringify(row).slice(0, 300)}`
      );
    }
  }
);

// ---------------------------------------------------------------------------
// When — anonymous HTTP GETs
// ---------------------------------------------------------------------------

/**
 * Issue an unauthenticated GET against a non-root path on the configured
 * doorway. The bare-`/` variant is owned by the bootstrap-page browser
 * scenario in steps/ui/delivery.steps.ts — this regex requires at least one
 * character after the leading slash, so there is no overlap.
 *
 * Writes to the shared responseStore so the existing
 * `Then('the response status is {int}')` step in delivery.steps.ts resolves.
 */
When(
  /^an unauthenticated request is made for (\/[\w-][\w/-]*)$/,
  async function (this: E2EWorld, path: string) {
    const base = firstDoorwayUrl(this);
    const resp = await fetchApp(base, path);
    responseStore.set(this, resp);
  }
);

// ---------------------------------------------------------------------------
// Then — response shape assertions
// ---------------------------------------------------------------------------

Then('the response body is the SPA index.html', function (this: E2EWorld) {
  const resp = responseStore.get(this);
  assert.ok(resp, NO_RESPONSE);
  const ct = resp.headers['content-type'] ?? '';
  assert.ok(ct.includes('text/html'), `Expected text/html, got "${ct}" — not an SPA index.html`);
  const html = resp.body.toString('utf-8');
  const hasAngularShell =
    html.includes('<app-root') || html.includes('ng-version') || html.includes('<base ');
  assert.ok(
    hasAngularShell,
    `Response does not look like an Angular SPA shell. First 200 chars: ${html.slice(0, 200)}`
  );
});

Then(
  /^Angular's router handles the (\/\S+) route on the client$/,
  function (this: E2EWorld, _path: string) {
    // Substrate check: 200 + an Angular shell HTML body is sufficient
    // evidence that the doorway returned the SPA's entry point and that
    // Angular's router will hydrate on the client side. Browser-only
    // assertion (actual route resolution) lives in steps/ui/delivery.steps.ts.
    const resp = responseStore.get(this);
    assert.ok(resp, NO_RESPONSE);
    assert.equal(
      resp.status,
      200,
      'SPA fallback must return 200 for Angular client-side routing to take over'
    );
  }
);

// ---------------------------------------------------------------------------
// Pod-restart scenario — real, on a mesh that owns its processes
// ---------------------------------------------------------------------------

/** How long a restarted doorway gets to answer /health before the step reds. */
const DOORWAY_RESTART_GRACE_MS = 20_000;
const DOORWAY_RESTART_HEALTH_BUDGET_MS = 90_000;
/** Kill + boot + health wait, with margin — cucumber's 30s default kills this. */
const DOORWAY_RESTART_STEP_TIMEOUT_MS = 150_000;

/**
 * Documentary precondition: a prior probe confirmed /lamad serves the SPA
 * index.html on the configured doorway. We re-verify here so the scenario
 * is self-contained.
 */
Given(
  /^\/lamad has been confirmed serving the SPA index\.html on doorway "([^"]+)"$/,
  async function (this: E2EWorld, _doorwayId: string) {
    const base = firstDoorwayUrl(this);
    const resp = await fetchApp(base, '/lamad');
    assert.equal(resp.status, 200, `/lamad on ${base} returned ${resp.status}, expected 200`);
    const ct = resp.headers['content-type'] ?? '';
    assert.ok(ct.includes('text/html'), `/lamad served as "${ct}", expected text/html`);
    responseStore.set(this, resp);
  }
);

/**
 * Restarting the doorway, for real, because this mesh owns its processes.
 *
 * These steps used to `return 'pending'` with the note "kubectl access is
 * operator-driven" — true of the deployed fleet, where a doorway is a remote
 * pod with no PID on this host. The household mesh is the other case: the
 * doorway is a process on this machine, and the fixture manifest says so
 * (`processControl: true`). So the scenario stops being a contract nobody can
 * run and becomes an ordinary drill.
 *
 * The restart is a re-exec of the SAME process: argv, environment and cwd are
 * read back out of /proc before the kill and replayed after it, so the new
 * doorway is the one the mesh was configured with rather than one this test
 * invented. The pid is discovered by an EXACT argv match on the listen port —
 * never a `pkill -f` pattern, which would match this runner's own shell and
 * kill the run (the self-kill trap, 2026-08-16).
 *
 * DESTRUCTIVE: takes doorway "alpha" down for the length of one boot. Hold it
 * behind an explicit operator go on a shared mesh.
 */

interface RestartRecord {
  pid: number;
  port: string;
  /** Corrective actions a step had to take to get the doorway serving again. */
  interventions: string[];
}

const restarts = new WeakMap<E2EWorld, RestartRecord>();

async function waitForDoorwayHealthy(base: string, budgetMs: number): Promise<void> {
  const deadline = Date.now() + budgetMs;
  let last = 'never answered';
  while (Date.now() < deadline) {
    try {
      const resp = await fetchApp(base, '/health');
      if (resp.status === 200) return;
      last = `status ${resp.status}`;
    } catch (error) {
      last = String(error);
    }
    await new Promise<void>(resolve => setTimeout(resolve, 1_000));
  }
  assert.fail(`doorway at ${base} did not come back within ${budgetMs}ms (${last})`);
}

When(
  "doorway {string}'s pod is restarted",
  { timeout: DOORWAY_RESTART_STEP_TIMEOUT_MS },
  async function (this: E2EWorld, doorwayId: string) {
    // The real gate: the lane's declared owned-substrate cap, or the operator's
    // explicit A2O_ALLOW_DESTRUCTIVE switch (substrate-scope.ts destructiveAllowed —
    // never fail-open, so this file is safe to run anywhere).
    if (!destructiveAllowed()) {
      // eslint-disable-next-line no-console
      console.log(
        `  ⏭️  DESTRUCTIVE HELD: would SIGTERM doorway "${doorwayId}" and re-exec it from its own ` +
          `/proc argv+environ+cwd. ${DESTRUCTIVE_HELD_HINT} Skipped, not failed.`
      );
      return 'skipped';
    }
    const fixture = loadHouseholdMeshFixture();
    const base = requireFixtureDoorwayUrl(fixture, doorwayId);
    const port = new URL(base).port || '80';
    const pid = doorwayPidForPort(port);

    // The replacement is the same doorway re-exec'd from its own /proc
    // argv+environ+cwd — never a differently-configured one.
    const childPid = await reexecProcess(pid, {
      graceMs: DOORWAY_RESTART_GRACE_MS,
      logPath: fixture.doorways?.[doorwayId]?.logPath,
    });

    await waitForDoorwayHealthy(base, DOORWAY_RESTART_HEALTH_BUDGET_MS);
    restarts.set(this, { pid: childPid, port, interventions: [] });
  }
);

/**
 * The scenario's premise: the replacement doorway may read a DIFFERENT storage
 * peer than the one that died, and that must not matter. This step proves the
 * premise is live rather than assuming it — if the doorway had exactly one
 * storage peer to choose from, the next assertion would pass for a reason that
 * has nothing to do with substrate-correct writes.
 */
When(
  "the new pod's storage peer is selected non-deterministically from the alpha cluster",
  function (this: E2EWorld) {
    const fixture = loadHouseholdMeshFixture();
    const pool = requireFixturePoolStorageUrls(fixture, 'alpha');
    assert.ok(
      pool.length > 1,
      `the doorway has only ${pool.length} storage peer to read from, so "whichever peer the ` +
        'new pod picks" is not a choice — this scenario cannot fail for its own reason here'
    );
  }
);

Then(
  /^a subsequent request for (\/\S+) returns the SPA index\.html with status (\d+)$/,
  { timeout: DOORWAY_RESTART_STEP_TIMEOUT_MS },
  async function (this: E2EWorld, path: string, status: string) {
    const record = restarts.get(this);
    assert.ok(record, 'no doorway restart was recorded — this Then must follow the restart When');
    const fixture = loadHouseholdMeshFixture();
    const base = requireFixtureDoorwayUrl(fixture, 'alpha');
    const resp = await fetchApp(base, path);
    assert.equal(
      resp.status,
      Number(status),
      `${path} returned ${resp.status} after the restart: ${resp.body.toString('utf-8').slice(0, 200)}`
    );
    const ct = resp.headers['content-type'] ?? '';
    assert.ok(ct.includes('text/html'), `${path} served as "${ct}" after the restart`);
    responseStore.set(this, resp);
  }
);

/**
 * "No operator intervention" is a claim about what the RUN had to do, so it is
 * asserted against the run's own record: the restart brought the doorway back
 * and the SPA served again without any step reaching for a corrective lever
 * (an admin refresh, a re-seed, a second restart). Any step that ever needs
 * one must push its name onto `interventions`, and this assertion turns red —
 * which is the only way the claim stays falsifiable as the file grows.
 */
Then('no operator intervention is required', function (this: E2EWorld) {
  const record = restarts.get(this);
  assert.ok(record, 'no doorway restart was recorded — this Then must follow the restart When');
  assert.deepEqual(
    record.interventions,
    [],
    `the doorway only served again after: ${record.interventions.join(', ')}`
  );
});
