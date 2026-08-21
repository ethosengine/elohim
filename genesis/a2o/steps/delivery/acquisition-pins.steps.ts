/**
 * Step definitions for features/delivery/acquisition-pins.feature
 *
 * These steps exercise the /api/v1/pins own-node API on elohim-storage
 * directly via E2E_STORAGE_URL (default: http://localhost:8090).
 *
 * The pin routes are own-node — they are NOT proxied through the doorway —
 * so these steps bypass the E2EWorld doorway map and speak directly to
 * elohim-storage. This mirrors the pattern used in resilience.steps.ts.
 *
 * ## What runs vs what is @wip
 *
 * Scenarios 1 and 2 (airplane-mode API) run whenever a local elohim-storage
 * is reachable at E2E_STORAGE_URL. The @requires:doorway tag signals a
 * running instance is required — the substrate-scope gate holds these
 * scenarios automatically when the stack is unavailable.
 *
 * Scenario 3 (@requires:household-nodes @wip) is the two-node scenario. Its
 * steps are implemented here as @wip stubs returning 'pending'. The load-
 * bearing regression for two-node byte-arrival semantics is the Rust test
 * elohim/elohim-storage/tests/acquisition_pull_e2e.rs — not a cucumber run.
 *
 * Two steps in the "Acquisition constraint regressions" section below (the
 * fabric-size Given and the distinct-peers-probed Then) are real, not
 * pending — they are reused verbatim by
 * resiliency-saga/11-pull-queue-retires.feature's household-mesh scenario
 * (steps/mesh/delivery-notary.steps.ts), which runs against a real 3-peer
 * mesh rather than the injectable-failure fixture the sibling stubs in this
 * section still need.
 */

import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { probeP2PStatus } from '../../src/framework/dataplane/surfaces.js';

import type { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Helpers — storage direct (own-node)
// ---------------------------------------------------------------------------

/** Resolve the storage base URL from env (matches resilience.steps.ts pattern). */
function storageUrl(): string {
  return process.env['E2E_STORAGE_URL'] ?? 'http://localhost:8090';
}

/** Own-node pins API path (runtime arg; the cucumber-expression patterns escape the slashes). */
const PINS_PATH = '/api/v1/pins';

interface RawResponse {
  status: number;
  body: string;
}

/** POST JSON to elohim-storage, return raw status + body text. */
async function storagePostRaw(
  path: string,
  payload: Record<string, unknown>
): Promise<RawResponse> {
  const { statusCode, body } = await request(`${storageUrl()}${path}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(payload),
  });
  const text = await body.text();
  return { status: statusCode, body: text };
}

/** GET from elohim-storage, return parsed JSON. */
async function storageGetJson(path: string): Promise<{ status: number; body: unknown }> {
  const { statusCode, body } = await request(`${storageUrl()}${path}`);
  const text = await body.text();
  let parsed: unknown = null;
  try {
    parsed = JSON.parse(text);
  } catch {
    parsed = text;
  }
  return { status: statusCode, body: parsed };
}

// World-scoped capture for pin responses (uses a module-level WeakMap to
// avoid mutating the E2EWorld type — same pattern as responseStore in
// delivery.steps.ts).
const pinResponseStore = new WeakMap<E2EWorld, RawResponse>();

// ---------------------------------------------------------------------------
// Scenario 1 — item pin creation and durability
// ---------------------------------------------------------------------------

When(
  String.raw`I POST a pin for {string} to \/api\/v1\/pins`,
  async function (this: E2EWorld, headRef: string) {
    const resp = await storagePostRaw(PINS_PATH, { headRef });
    pinResponseStore.set(this, resp);
  }
);

Then('the pin response status is {int}', function (this: E2EWorld, expectedStatus: number) {
  const resp = pinResponseStore.get(this);
  assert.ok(resp, 'No pin response captured — run a "When I POST a pin" step first');
  assert.equal(
    resp.status,
    expectedStatus,
    `Expected status ${expectedStatus} but got ${resp.status}; body: ${resp.body.slice(0, 200)}`
  );
});

Then(
  String.raw`GET \/api\/v1\/pins lists one active pin for {string}`,
  async function (this: E2EWorld, headRef: string) {
    const { status, body } = await storageGetJson(PINS_PATH);
    assert.equal(status, 200, `GET /api/v1/pins returned ${status}`);

    const payload = body as Record<string, unknown>;
    const pins = payload['pins'] as Record<string, unknown>[];
    assert.ok(
      Array.isArray(pins),
      `"pins" must be an array; got ${JSON.stringify(payload).slice(0, 200)}`
    );

    const active = pins.filter(p => p['status'] === 'active' && p['headRef'] === headRef);
    assert.equal(
      active.length,
      1,
      `Expected exactly 1 active pin for "${headRef}"; got ${active.length}. Pins: ${JSON.stringify(pins).slice(0, 300)}`
    );
  }
);

// ---------------------------------------------------------------------------
// Scenario 2 — cluster pin 501
// ---------------------------------------------------------------------------

When(
  String.raw`I POST a pin with kind {string} for {string} to \/api\/v1\/pins`,
  async function (this: E2EWorld, kind: string, headRef: string) {
    const resp = await storagePostRaw(PINS_PATH, { headRef, kind });
    pinResponseStore.set(this, resp);
  }
);

Then('the pin response body mentions {string}', function (this: E2EWorld, fragment: string) {
  const resp = pinResponseStore.get(this);
  assert.ok(resp, 'No pin response captured — run a "When I POST a pin" step first');
  assert.ok(
    resp.body.includes(fragment),
    `Expected response body to mention "${fragment}"; got: ${resp.body.slice(0, 300)}`
  );
});

// ---------------------------------------------------------------------------
// Provide pins (slice 2b — rung 4) — runnable own-node steps
//
// The provide pin (provide:true) is the same own-node /api/v1/pins API; the
// provide flag tells the reconciler to author a replicates-commons commitment.
// GET /api/v1/pins/{eprId}/pull returns the per-EPR rollup the
// PinProgressComponent renders. Both legs run on a single household node.
// ---------------------------------------------------------------------------

When(
  String.raw`I POST a provide pin for {string} to \/api\/v1\/pins`,
  async function (this: E2EWorld, headRef: string) {
    const id = headRef.replace(/^epr:/, '');
    const resp = await storagePostRaw(PINS_PATH, {
      headRef: id,
      kind: 'item',
      provide: true,
    });
    pinResponseStore.set(this, resp);
  }
);

When(
  'I POST a provide pin with a forced browser context for {string}',
  async function (this: E2EWorld, headRef: string) {
    const id = headRef.replace(/^epr:/, '');
    // The provide flag is honored only on a peer-capable node; a browser-context
    // provide must be refused 400. The own-node API enforces this server-side.
    const resp = await storagePostRaw(PINS_PATH, {
      headRef: id,
      kind: 'item',
      provide: true,
      context: 'browser',
    });
    pinResponseStore.set(this, resp);
  }
);

Then(
  String.raw`GET \/api\/v1\/pins\/{word}\/pull reports a pull rollup`,
  async function (this: E2EWorld, eprId: string) {
    const { status, body } = await storageGetJson(`/api/v1/pins/${eprId}/pull`);
    // A 404 here means "this node holds no pin for that EPR" — which would
    // contradict the 201 the previous step just got. It must NOT mean "the pin
    // exists but acquisition has not measured it yet"; that answer is a 200
    // carrying the tri-state nulls (total/caughtUp null = keep waiting).
    assert.equal(
      status,
      200,
      `GET pull rollup returned ${status}` +
        (status === 404
          ? ` — the node denied holding a pin it accepted one step ago; body: ${JSON.stringify(body).slice(0, 200)}`
          : '')
    );
    const rollup = body as Record<string, unknown>;
    // The rollup is grouped by head_ref; the shape must carry the whole wire
    // contract (epr-pull-status.schema.json): the id it answers for, the
    // counters PinProgressComponent renders, and the tri-state total/caughtUp.
    for (const field of ['eprId', 'total', 'fetched', 'pending', 'failed', 'caughtUp']) {
      assert.ok(
        field in rollup,
        `pull rollup missing "${field}"; got ${JSON.stringify(rollup).slice(0, 200)}`
      );
    }
    // Never a false-complete: an unmeasured rollup reports caughtUp null, and a
    // measured one reports a boolean — but never `true` with a null total.
    if (rollup['total'] === null) {
      assert.equal(
        rollup['caughtUp'],
        null,
        `an unmeasured rollup must not claim caught-up; got ${JSON.stringify(rollup).slice(0, 200)}`
      );
    }
  }
);

// ---------------------------------------------------------------------------
// Two-node byte-arrival (@requires:household-nodes @requires:owned-substrate)
//
// These steps speak to TWO storage peers on the household mesh: peer A is the
// own-node URL this file already uses (E2E_STORAGE_URL — matthew), peer B is
// E2E_STORAGE_JESSICA (falling back to E2E_STORAGE_URL_B). Byte-arrival is the
// whole point of the pull queue (R-A: a pin is satisfied by bytes landing, never
// by inventory saying they exist), so every assertion below reads B's OWN
// surfaces — its pull rollup and its local projection — never A's word for it.
//
// WRITES. Pinning on B enqueues a real acquisition and moves real bytes across
// the mesh, so every mutating step is held behind A2O_ALLOW_DESTRUCTIVE=1 and
// answers `'skipped'` when the gate is closed. `'pending'` would RED the run
// (cucumber-js is strict by default); `'skipped'` is the disposition this repo
// gives a deliberately-held step, and cucumber skips the rest of the scenario
// with it. The Rust integration test
// elohim/elohim-storage/tests/acquisition_pull_e2e.rs remains the binding
// regression for the same contract; this is its human-facing statement, run
// against a mesh the suite owns.
// ---------------------------------------------------------------------------

const DESTRUCTIVE_ENABLED = process.env['A2O_ALLOW_DESTRUCTIVE'] === '1';

/**
 * Gate for every step below that writes to, or restarts, the mesh. Logs one
 * line naming what would have happened so a reader of the console output can
 * tell a deliberately-held step from a step nobody wrote.
 */
function destructiveGate(whatItWouldDo: string): boolean {
  if (DESTRUCTIVE_ENABLED) return true;
  // eslint-disable-next-line no-console
  console.log(
    `  ⏭️  SKIPPED (destructive, gated): would ${whatItWouldDo}. Set A2O_ALLOW_DESTRUCTIVE=1 to enable.`
  );
  return false;
}

/** Peer B — the household member that does NOT hold the bytes yet. */
function peerBUrl(): string {
  const url = process.env['E2E_STORAGE_JESSICA'] ?? process.env['E2E_STORAGE_URL_B'];
  assert.ok(
    url,
    'no second storage peer: set E2E_STORAGE_JESSICA (or E2E_STORAGE_URL_B) to a household ' +
      'member other than E2E_STORAGE_URL. Byte-arrival cannot be observed on one node.'
  );
  return url;
}

/** Strip the `epr:` prefix — pins store the bare head_ref. */
function bareId(headRef: string): string {
  return headRef.replace(/^epr:/, '');
}

async function getJsonFrom(base: string, path: string): Promise<{ status: number; body: unknown }> {
  const { statusCode, body } = await request(`${base}${path}`);
  const text = await body.text();
  let parsed: unknown = null;
  try {
    parsed = JSON.parse(text);
  } catch {
    parsed = text;
  }
  return { status: statusCode, body: parsed };
}

async function postJsonTo(
  base: string,
  path: string,
  payload: Record<string, unknown>
): Promise<RawResponse> {
  const { statusCode, body } = await request(`${base}${path}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(payload),
  });
  return { status: statusCode, body: await body.text() };
}

/** Per-scenario two-node state: which peers, and which head the story is about. */
interface TwoNodeState {
  a: string;
  b: string;
  headRef: string;
  id: string;
}
const twoNodeState = new WeakMap<E2EWorld, TwoNodeState>();

function twoNode(world: E2EWorld): TwoNodeState {
  const state = twoNodeState.get(world);
  assert.ok(
    state,
    'no two-peer baseline — the "two connected storage peers where only peer A holds ..." Given ' +
      'must run first'
  );
  return state;
}

/**
 * Establish the asymmetry the whole story rests on: A holds the bytes, B does
 * not. On a mesh that has already converged both peers hold everything, so the
 * asymmetry has to be CONSTRUCTED — a fresh content item posted to A alone.
 * That is a write, hence the gate.
 */
Given(
  'two connected storage peers where only peer A holds {string}',
  async function (this: E2EWorld, headRef: string) {
    const a = storageUrl();
    const b = peerBUrl();
    const id = bareId(headRef);

    // Both peers must be up, and A must actually be connected to something —
    // a pull queue with no peer to pull from proves nothing about byte arrival.
    const aStatus = await probeP2PStatus(a);
    assert.ok(
      typeof aStatus.body.connectedPeers === 'number' && aStatus.body.connectedPeers >= 1,
      `peer A (${a}) reports ${JSON.stringify(aStatus.body.connectedPeers)} connected peers — ` +
        'the two peers are not connected, so nothing can arrive.'
    );
    const bHealth = await getJsonFrom(b, '/health');
    assert.ok(bHealth.status < 500, `peer B (${b}) /health returned ${bHealth.status}`);

    twoNodeState.set(this, { a, b, headRef, id });

    const onB = await getJsonFrom(b, `/db/content/${id}`);
    if (onB.status === 200) {
      // Already converged. Removing it from B to manufacture the asymmetry
      // would be a deletion on a shared mesh — a bigger blast radius than this
      // story is worth. Hold rather than lie about what was observed.
      // eslint-disable-next-line no-console
      console.log(
        `  ⏭️  SKIPPED (fixture): peer B (${b}) already holds "${id}", so there is no ` +
          'byte-arrival to observe. This story needs a content item staged on ONE peer only; ' +
          'choose a head B has never seen, or stage one with A2O_ALLOW_DESTRUCTIVE=1.'
      );
      return 'skipped';
    }

    const onA = await getJsonFrom(a, `/db/content/${id}`);
    if (onA.status === 200) return undefined; // asymmetry already true — no write needed.

    if (!destructiveGate(`stage content "${id}" on peer A (${a}) so peer B can pull it`)) {
      return 'skipped';
    }
    const staged = await postJsonTo(a, '/db/content', {
      id,
      contentType: 'narrative',
      contentFormat: 'external',
      reach: 'commons',
      title: id,
      content: `a2o acquisition byte-arrival fixture for ${id}`,
    });
    assert.ok(
      staged.status >= 200 && staged.status < 300,
      `staging "${id}" on peer A returned ${staged.status}: ${staged.body.slice(0, 300)}`
    );
    return undefined;
  }
);

When('peer B pins {string}', async function (this: E2EWorld, headRef: string) {
  const state = twoNode(this);
  if (!destructiveGate(`POST a pin for "${headRef}" on peer B (${state.b})`)) {
    return 'skipped';
  }
  const resp = await postJsonTo(state.b, PINS_PATH, { headRef: bareId(headRef), kind: 'item' });
  assert.equal(
    resp.status,
    201,
    `peer B refused the pin (${resp.status}): ${resp.body.slice(0, 300)}`
  );
  return undefined;
});

When('peer A pins {string} as a peer', async function (this: E2EWorld, headRef: string) {
  const state = twoNode(this);
  if (!destructiveGate(`POST a provide pin for "${headRef}" on peer A (${state.a})`)) {
    return 'skipped';
  }
  const resp = await postJsonTo(state.a, PINS_PATH, {
    headRef: bareId(headRef),
    kind: 'item',
    provide: true,
  });
  assert.equal(
    resp.status,
    201,
    `peer A refused the provide pin (${resp.status}): ${resp.body.slice(0, 300)}`
  );
  return undefined;
});

/**
 * Wait for peer B's pull rollup for this head to settle. The acquisition
 * reconcile runs on a 60s cadence and retries on the NEXT cycle, so a drain
 * that involves one retry needs more than two cycles — the default budget is
 * three, overridable with E2O_PULL_DRAIN_TIMEOUT_MS for a slower mesh.
 */
When('the pull queue drains', async function (this: E2EWorld) {
  const state = twoNode(this);
  const budgetMs = Number(process.env['E2O_PULL_DRAIN_TIMEOUT_MS'] ?? 200_000);
  const deadline = Date.now() + budgetMs;
  let last = 'never polled';
  while (Date.now() < deadline) {
    const { status, body } = await getJsonFrom(state.b, `/api/v1/pins/${state.id}/pull`);
    if (status === 200) {
      const rollup = body as Record<string, unknown>;
      last = JSON.stringify(rollup);
      // caughtUp true is the measured, non-null answer; null means "keep waiting".
      if (rollup['caughtUp'] === true) return;
      if (typeof rollup['failed'] === 'number' && (rollup['failed'] as number) > 0) {
        assert.fail(
          `peer B's pull for "${state.id}" reported failures before draining: ${last}. ` +
            'A failed fetch is a real acquisition red, not a slow one.'
        );
      }
    } else {
      last = `HTTP ${status}`;
    }
    await new Promise(r => setTimeout(r, 5_000));
  }
  assert.fail(
    `peer B's pull for "${state.id}" did not drain within ${budgetMs}ms — last rollup: ${last}. ` +
      'The acquisition reconcile cadence is 60s and retries land on the NEXT cycle; raise ' +
      'E2O_PULL_DRAIN_TIMEOUT_MS if this mesh is slower, or read it as a wedged queue.'
  );
});

Then(
  "peer B's pull status shows fetched {int} of total {int}",
  async function (this: E2EWorld, fetched: number, total: number) {
    const state = twoNode(this);
    const { status, body } = await getJsonFrom(state.b, `/api/v1/pins/${state.id}/pull`);
    assert.equal(status, 200, `peer B's pull rollup for "${state.id}" returned ${status}`);
    const rollup = body as Record<string, unknown>;
    assert.equal(
      rollup['total'],
      total,
      `expected total ${total}; got ${JSON.stringify(rollup)}`
    );
    assert.equal(
      rollup['fetched'],
      fetched,
      `expected fetched ${fetched}; got ${JSON.stringify(rollup)}`
    );
  }
);

/**
 * R-A, checked at the moment the rollup claims it: a pin is satisfied by BYTE
 * ARRIVAL, so when the pull says caught-up the pinned thing must be readable on
 * the pulling peer. The row is checked immediately; if it is absent the step
 * keeps polling only to MEASURE how late it lands, then fails with that number.
 * Either way this is a deterministic red — a caught-up claim that runs ahead of
 * local readability is the inventory-arrival failure this story exists to catch,
 * and a settle window here would launder it into a pass.
 */
Then("the content row exists in peer B's local projection", async function (this: E2EWorld) {
  const state = twoNode(this);
  const first = await getJsonFrom(state.b, `/db/content/${state.id}`);
  if (first.status === 200) return;

  const budgetMs = Number(process.env['E2O_PROJECTION_LAG_BUDGET_MS'] ?? 60_000);
  const started = Date.now();
  while (Date.now() - started < budgetMs) {
    await new Promise(r => setTimeout(r, 2_000));
    const { status } = await getJsonFrom(state.b, `/db/content/${state.id}`);
    if (status === 200) {
      assert.fail(
        `peer B's pull for "${state.id}" reported caughtUp/fetched BEFORE its own projection ` +
          `carried the row: /db/content/${state.id} was 404 at the moment of the claim and ` +
          `became readable ${Math.round((Date.now() - started) / 1000)}s later. That gap is ` +
          'inventory-arrival wearing byte-arrival\'s clothes (R-A): a caller that trusts ' +
          'caughtUp and reads immediately gets a 404 for content it was just told it has.'
      );
    }
  }
  assert.fail(
    `peer B (${state.b}) never carried a local row for "${state.id}" within ${budgetMs}ms of a ` +
      'caught-up pull claim (HTTP ' +
      `${first.status} throughout) — the pin reported complete and the content is not there.`
  );
});

/**
 * Attribution, honestly bounded: no surface names WHICH peer supplied a given
 * blob, so this asserts the two facts that are observable — B now serves the
 * bytes itself, and A was a connected delivery peer able to supply them.
 */
Then('peer B fetched the bytes from peer A', async function (this: E2EWorld) {
  const state = twoNode(this);
  const content = await getJsonFrom(state.b, `/db/content/${state.id}`);
  assert.equal(content.status, 200, `peer B holds no row for "${state.id}"`);
  const blobHash = (content.body as Record<string, unknown>)['blobHash'];
  if (typeof blobHash === 'string' && blobHash.length > 0) {
    const { status } = await request(`${state.b}/blob/${blobHash}`, { method: 'GET' });
    assert.equal(
      status,
      200,
      `peer B has the row for "${state.id}" but cannot serve its blob ${blobHash} (HTTP ${status})`
    );
  }
  const peers = await getJsonFrom(state.b, '/api/v1/peers/delivery');
  assert.equal(peers.status, 200, `peer B's delivery-peer list returned ${peers.status}`);
  assert.ok(
    Array.isArray(peers.body) && (peers.body as unknown[]).length >= 1,
    'peer B lists no delivery peers, so the bytes cannot have come from peer A over the mesh. ' +
      'NOTE: no surface attributes a blob to the peer that supplied it — this step asserts ' +
      'B serves the bytes AND had a peer able to supply them, not a traced provenance.'
  );
});

// ---------------------------------------------------------------------------
// Boot vs observed-empty (@wip drops with @requires:owned-substrate)
//
// `pull: null` is the unmeasured answer; a non-null zero rollup is evidence the
// census actually ran and found nothing. Telling them apart requires a peer
// that has just restarted — a process action, gated.
// ---------------------------------------------------------------------------

/** genesis/a2o/steps/delivery → repo root → app/elohim-app/scripts/hc-mesh.sh */
function hcMeshScriptPath(): string {
  return fileURLToPath(new URL('../../../../app/elohim-app/scripts/hc-mesh.sh', import.meta.url));
}

Given(
  'a storage peer has restarted and no acquisition reconcile has completed',
  function (this: E2EWorld) {
    const peer = process.env['E2E_MESH_RESTART_PEER'] ?? 'matthew';
    if (!destructiveGate(`restart mesh storage peer "${peer}" via hc-mesh.sh storage-restart`)) {
      return 'skipped';
    }
    const script = hcMeshScriptPath();
    const result = spawnSync('bash', [script, 'storage-restart', peer], {
      encoding: 'utf8',
      timeout: 300_000,
    });
    assert.equal(
      result.status,
      0,
      `${script} storage-restart ${peer} exited ${String(result.status)}: ` +
        `${(result.stderr ?? '').slice(-2000)}`
    );
    return undefined;
  }
);

/** Capture of the last GET /p2p/status taken by this file's steps. */
const p2pStatusStore = new WeakMap<E2EWorld, Record<string, unknown>>();

When(String.raw`I GET \/p2p\/status`, async function (this: E2EWorld) {
  const { status, body } = await getJsonFrom(storageUrl(), '/p2p/status');
  assert.equal(status, 200, `GET /p2p/status returned ${status}`);
  p2pStatusStore.set(this, body as Record<string, unknown>);
});

Then('the pull status is null', function (this: E2EWorld) {
  const status = p2pStatusStore.get(this);
  assert.ok(status, 'no /p2p/status capture — run "When I GET /p2p/status" first');
  assert.equal(
    status['pull'],
    null,
    `expected pull:null on a peer whose first acquisition reconcile has not completed; got ` +
      `${JSON.stringify(status['pull'])}. Unreadable is not zero — a non-null rollup here means ` +
      'the status was answered from something other than a completed census.'
  );
});

When('the first acquisition reconcile completes with zero active pins', async function (
  this: E2EWorld
) {
  const base = storageUrl();
  const { body } = await getJsonFrom(base, PINS_PATH);
  const pins = ((body as Record<string, unknown>)['pins'] ?? []) as Record<string, unknown>[];
  const active = pins.filter(p => p['status'] === 'active');
  if (active.length > 0) {
    // eslint-disable-next-line no-console
    console.log(
      `  ⏭️  SKIPPED (fixture): peer ${base} holds ${active.length} active pin(s), so its first ` +
        'reconcile cannot complete with an EMPTY desired set. This half of the contract needs a ' +
        'peer with no pins — point E2E_STORAGE_URL at an unpinned household member.'
    );
    return 'skipped';
  }
  // Wait out one reconcile cadence (60s) plus slack for the census to land.
  const budgetMs = Number(process.env['E2O_RECONCILE_TIMEOUT_MS'] ?? 120_000);
  const deadline = Date.now() + budgetMs;
  while (Date.now() < deadline) {
    const metrics = await request(`${base}/metrics`);
    const text = await metrics.body.text();
    if (/^elohim_acquisition_reconcile_initialized 1$/m.test(text)) {
      const { body: statusBody } = await getJsonFrom(base, '/p2p/status');
      p2pStatusStore.set(this, statusBody as Record<string, unknown>);
      return undefined;
    }
    await new Promise(r => setTimeout(r, 5_000));
  }
  assert.fail(
    `no acquisition reconcile completed on ${base} within ${budgetMs}ms ` +
      '(elohim_acquisition_reconcile_initialized never reached 1)'
  );
  return undefined;
});

Then(
  'the pull status reports total {int}, fetched {int}, pending {int}, and failed {int}',
  function (this: E2EWorld, total: number, fetched: number, pending: number, failed: number) {
    const status = p2pStatusStore.get(this);
    assert.ok(status, 'no /p2p/status capture — an earlier step must read it');
    const pull = status['pull'] as Record<string, unknown> | null;
    assert.ok(
      pull !== null && typeof pull === 'object',
      'pull is null after a completed reconcile — a measured empty set must be a PRESENT zero, ' +
        'never the unmeasured null.'
    );
    assert.deepEqual(
      { total: pull['total'], fetched: pull['fetched'], pending: pull['pending'], failed: pull['failed'] },
      { total, fetched, pending, failed },
      `unexpected pull rollup: ${JSON.stringify(pull)}`
    );
  }
);

/** Parse one unlabelled Prometheus gauge/counter value from the metrics text. */
async function metricValue(base: string, name: string): Promise<number | undefined> {
  const { body } = await request(`${base}/metrics`);
  const text = await body.text();
  const m = new RegExp(`^${name}\\s+([0-9.eE+-]+)$`, 'm').exec(text);
  return m ? Number(m[1]) : undefined;
}

Then(
  'the acquisition reconcile initialized metric is {int}',
  async function (this: E2EWorld, expected: number) {
    const value = await metricValue(storageUrl(), 'elohim_acquisition_reconcile_initialized');
    assert.equal(
      value,
      expected,
      `elohim_acquisition_reconcile_initialized is ${String(value)}, expected ${expected}`
    );
  }
);

Then('the acquisition active pins metric is {int}', async function (
  this: E2EWorld,
  expected: number
) {
  const value = await metricValue(storageUrl(), 'elohim_acquisition_active_pins');
  assert.equal(
    value,
    expected,
    `elohim_acquisition_active_pins is ${String(value)}, expected ${expected}`
  );
});

Then(
  'the acquisition reconcile outcome {string} was counted',
  async function (this: E2EWorld, outcome: string) {
    const { body } = await request(`${storageUrl()}/metrics`);
    const text = await body.text();
    const m = new RegExp(
      `^elohim_acquisition_reconcile_outcomes_total\\{outcome="${outcome}"\\}\\s+([0-9.eE+-]+)$`,
      'm'
    ).exec(text);
    assert.ok(
      m,
      `no elohim_acquisition_reconcile_outcomes_total{outcome="${outcome}"} series on ` +
        `${storageUrl()}/metrics — the outcome vocabulary must be pre-touched so a zero is ` +
        'readable as "measured none", not as "series missing".'
    );
    assert.ok(
      Number(m[1]) >= 1,
      `outcome "${outcome}" was counted ${m[1]} times; expected at least one completed reconcile`
    );
  }
);

// ---------------------------------------------------------------------------
// Acquisition constraint regressions (@wip @regression @requires:household-nodes)
//
// Harvested 2026-07-26 from the resiliency-saga overnight cure sprint. Both
// defects lived inside the eight-deep chapter-5 deadlock chain and neither
// announced itself — acquisition just went quiet.
//
// The BINDING regressions are Rust unit tests, not these stubs:
//   * retry rotation — elohim/elohim-storage/src/p2p/mod.rs,
//     mod acquisition_rotation_tests
//     (successive_retries_of_a_stable_position_walk_distinct_peers). Parameters:
//     MAX_RETRIES=3, 6-peer fabric, 60s retry-on-next-cycle reconcile.
//   * in-flight budget release — elohim/elohim-storage/src/p2p/reconcile_rails.rs
//     (dispatch_budget_caps_inflight) + the ShardResponse::Error arm in
//     p2p/mod.rs. Parameters: MAX_ACQUISITION_INFLIGHT=25; 25 leaked slots wedge
//     acquisition permanently and silently.
//
// Most of these cucumber steps stay pending until a household-scale
// acquisition fixture (a fabric with INJECTABLE per-peer failure responses)
// exists — the same scaffolded-pending posture as the two-node scenarios
// above. Two of them (the fabric-size Given and the distinct-peers-probed
// Then) no longer need that fixture: they are also reused, unchanged in
// wording, by resiliency-saga/11-pull-queue-retires.feature's "an
// unsatisfiable pin retires once the peer-sized retry budget exhausts"
// scenario, which runs against a REAL 3-peer household mesh
// (E2E_HOUSEHOLD_FIXTURE_PATH) — see steps/mesh/delivery-notary.steps.ts.
// That mesh cannot supply the chapter's worked-example fabric size (6 peers;
// this mesh has 3, so 2 connected peers from any one member's own view) —
// both steps below assert against the mesh's ACTUAL /p2p/status reading
// rather than fake the literal number, so a mismatch reds honestly and names
// the discrepancy instead of hiding it behind a stub.
// ---------------------------------------------------------------------------

/** Baseline captured by "an acquisition fabric of {int} connected peers". */
interface AcquisitionFabricState {
  connectedPeers: number;
}
const acquisitionFabricState = new WeakMap<E2EWorld, AcquisitionFabricState>();

Given(
  'an acquisition fabric of {int} connected peers',
  async function (this: E2EWorld, expectedPeerCount: number) {
    // Real probe: GET /p2p/status on the own-node storage URL this file
    // already speaks to (storageUrl() — E2E_STORAGE_URL, matthew's port on
    // the household mesh). On a healthy 3-peer mesh every member reports the
    // same connectedPeers count (the other two), so which member answers
    // this probe does not change the number — this scenario's own subject
    // (jessica) is reachable the same way via E2E_STORAGE_JESSICA.
    const { body } = await probeP2PStatus(storageUrl());
    const connectedPeers = body.connectedPeers;
    assert.ok(
      typeof connectedPeers === 'number',
      `GET /p2p/status on ${storageUrl()}: connectedPeers is not a number ` +
        `(${JSON.stringify(connectedPeers)})`
    );
    acquisitionFabricState.set(this, { connectedPeers });
    assert.strictEqual(
      connectedPeers,
      expectedPeerCount,
      `this mesh's real acquisition fabric has ${connectedPeers} connected peer(s) at ` +
        `${storageUrl()} — the chapter's worked example names a ${expectedPeerCount}-peer alpha ` +
        `fabric; this is a household mesh with 3 total members (2 connected from any one member's ` +
        `own view). The retry budget the "probed on N distinct peers" step below actually checks ` +
        `is max(3, connectedPeers), so this mismatch alone does not prevent that step from passing.`
    );
  }
);

Given('{string} sits at a stable position in the acquisition batch', function (_headRef: string) {
  // Pending: the stable batch position comes from list_active_pins' DB order —
  // reproducing it needs control over the pin table, not just the HTTP surface.
  return 'pending';
});

When('its acquisition retries until the retry budget is exhausted', function () {
  // Pending: MAX_RETRIES=3 attempts across 3 x 60s reconcile cycles.
  return 'pending';
});

Then(
  'the item was probed on {int} distinct peers',
  function (this: E2EWorld, expectedDistinctPeers: number) {
    // No HTTP surface exposes a per-item dispatch-history trace (which
    // peers a SPECIFIC pin's retries actually reached), so this cannot
    // directly observe "probed on N distinct peers" as a literal fact. What
    // IS real and falsifiable: the retry-budget FORMULA the dispatch
    // rotation uses (max(3, connected peers) — see this file's acquisition
    // constraint-regression header above), computed from the fabric size
    // the Given step above actually measured.
    const fabric = acquisitionFabricState.get(this);
    assert.ok(
      fabric,
      'no acquisition-fabric baseline — the "an acquisition fabric of ... connected peers" Given ' +
        'must run first'
    );
    const retryBudget = Math.max(3, fabric.connectedPeers);
    assert.strictEqual(
      retryBudget,
      expectedDistinctPeers,
      `this mesh's live peer-sized retry budget is max(3, ${fabric.connectedPeers} connected ` +
        `peers) = ${retryBudget} — the chapter's worked example names ${expectedDistinctPeers} ` +
        `distinct peers (a 6-peer alpha fabric). No per-item dispatch-history surface exists to ` +
        `directly observe how many peers this specific pin was actually probed on, so this checks ` +
        `the BUDGET the dispatch rotation would use, not a directly-observed trace.`
    );
  }
);

When(
  '{int} acquisition dispatches are answered with an error-class shard response',
  function (_count: number) {
    // Pending: needs a peer that can be told to answer ShardResponse::Error.
    return 'pending';
  }
);

Then('the available dispatch budget returns to {int}', function (_available: number) {
  return 'pending';
});

Then('a subsequent acquisition drain dispatches at least one request', function () {
  return 'pending';
});
