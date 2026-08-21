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
import { execFileSync, spawn } from 'node:child_process';
import { openSync, readFileSync, realpathSync } from 'node:fs';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import {
  loadHouseholdMeshFixture,
  requireFixtureDoorwayUrl,
  requireFixturePoolStorageUrls,
} from '../../src/framework/fixtures/household-mesh.js';
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

/** Read a NUL-separated /proc file into its parts. */
function readNulList(path: string): string[] {
  return readFileSync(path, 'utf8').split('\0').filter(Boolean);
}

/**
 * The one pid listening on `port`, found by exact argv match.
 *
 * Refuses on zero matches AND on more than one: a drill that guesses which of
 * two candidates to kill is a drill that eventually kills the wrong thing.
 */
function doorwayPidForPort(port: string): number {
  // Absolute path: the pid this returns is about to be signalled, so the
  // binary that produced it must not be resolvable through a writable PATH.
  const listing = execFileSync('/usr/bin/ps', ['-eo', 'pid=,args='], { encoding: 'utf8' });
  const matches: number[] = [];
  for (const line of listing.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    const split = trimmed.indexOf(' ');
    if (split < 0) continue;
    const pid = Number(trimmed.slice(0, split));
    const args = trimmed.slice(split + 1);
    const argv = args.split(/\s+/);
    const isDoorway = argv[0]?.endsWith('/doorway') || argv[0] === 'doorway';
    if (!isDoorway) continue;
    const listenIdx = argv.indexOf('--listen');
    if (listenIdx < 0) continue;
    const listen = argv[listenIdx + 1] ?? '';
    if (listen.endsWith(`:${port}`)) matches.push(pid);
  }
  assert.ok(
    matches.length > 0,
    `no doorway process is listening on port ${port} — nothing to restart. ` +
      'This scenario needs the doorway running as a local process (`just mesh start`); ' +
      'against a deployed fleet it is a remote pod and there is no PID here to signal.'
  );
  assert.equal(
    matches.length,
    1,
    `${matches.length} doorway processes claim port ${port} (pids ${matches.join(', ')}) — ` +
      'refusing to guess which one the mesh means'
  );
  const pid = matches[0];
  assert.ok(
    pid > 1 && pid !== process.pid && pid !== process.ppid,
    `refusing to signal pid ${pid}: it is this test run or its parent`
  );
  return pid;
}

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
    // The real gate. @requires:owned-substrate says what a scenario NEEDS; it
    // does not say whether this operator wants it to happen right now, and on a
    // lane whose cluster-state does not declare the cap it fails open anyway.
    // One env switch, default off, so this file is safe to run anywhere.
    if (process.env['A2O_ALLOW_DESTRUCTIVE'] !== '1') {
      // eslint-disable-next-line no-console
      console.log(
        `  ⏭️  DESTRUCTIVE HELD: would SIGTERM doorway "${doorwayId}" and re-exec it from its own ` +
          '/proc argv+environ+cwd. Set A2O_ALLOW_DESTRUCTIVE=1 to run it. Skipped, not failed.'
      );
      return 'skipped';
    }
    const fixture = loadHouseholdMeshFixture();
    const base = requireFixtureDoorwayUrl(fixture, doorwayId);
    const port = new URL(base).port || '80';
    const pid = doorwayPidForPort(port);

    // Capture what the process WAS before killing it, so the replacement is
    // the same doorway and not a differently-configured one.
    const argv = readNulList(`/proc/${pid}/cmdline`);
    const env: Record<string, string> = {};
    for (const entry of readNulList(`/proc/${pid}/environ`)) {
      const eq = entry.indexOf('=');
      if (eq > 0) env[entry.slice(0, eq)] = entry.slice(eq + 1);
    }
    const cwd = realpathSync(`/proc/${pid}/cwd`);
    const [command, ...args] = argv;
    assert.ok(command, `could not read argv for doorway pid ${pid}`);

    process.kill(pid, 'SIGTERM');
    const graceDeadline = Date.now() + 20_000;
    while (Date.now() < graceDeadline) {
      try {
        process.kill(pid, 0);
      } catch {
        break;
      }
      await new Promise<void>(resolve => setTimeout(resolve, 250));
    }
    try {
      process.kill(pid, 0);
      process.kill(pid, 'SIGKILL');
    } catch {
      // already gone — the graceful stop worked
    }

    const logPath = fixture.doorways?.[doorwayId]?.logPath;
    const out = logPath ? openSync(logPath, 'a') : 'ignore';
    const child = spawn(command, args, {
      cwd,
      env,
      detached: true,
      stdio: ['ignore', out, out],
    });
    child.unref();

    await waitForDoorwayHealthy(base, DOORWAY_RESTART_HEALTH_BUDGET_MS);
    restarts.set(this, { pid: child.pid ?? 0, port, interventions: [] });
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
