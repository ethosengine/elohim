/**
 * A steward sponsors a runner, and the runner works only inside the promises made to it —
 * live against the household (features/compute/ci-runner-stewarded.feature).
 *
 * The runner is the hosted CI runner's real client (scripts/lib/compute-runner.ts): it
 * registers as a service agent through its sponsor's doorway, speaks only through its own
 * hosted cells, and stocks payloads in its sponsor's custody. Every verdict here is the
 * substrate's own: the sponsor's pool peer notarizes the hosting grant, a provider's storage
 * notarizes the capacity grant and decides admission against it (api/compute_tasks.rs
 * `authorize`), the sponsor's custody refuses a withdrawn promise or a reach narrower than
 * its grant (api/hosted_custody.rs), and the runner's conductor refuses any call its
 * function-listed capability does not name.
 *
 * Two things this file does that a deployed provider's worker would do instead:
 * - it asks a provider's storage to ACCEPT a task (`POST …/tasks/{ref}/accept`, the same
 *   call genesis/agentic/compute/worker.mjs makes before it runs anything) and, on a
 *   refusal, records the provider's decline. The decision is the provider's storage's.
 * - it names stages with genesis/agentic/compute/cid-executor.mjs, because the rakia
 *   compute-executor is unavailable; the substrate re-derives every CID it is handed.
 * Nothing here RUNS a stage. The scenario that reads a stage's report back carries
 * @requires:compute-executor and is held until the executor returns.
 *
 * CI_RUNNER_A2O_CONFIG names the fixture genesis/agentic/compute/runner-fixture.sh writes
 * from the running household; absent ⇒ every step is `pending`, never a pass.
 */
import { strict as assert } from 'node:assert';
import { createHash, randomBytes, randomUUID } from 'node:crypto';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

import { After, Given, Then, When } from '@cucumber/cucumber';

import { decodeHashFromBase64 } from '@holochain/client';

import { materialize } from '../../../agentic/compute/payloads.mjs';
import { buildStageTask } from '../../../agentic/compute/stage/build-stage-task.mjs';
import { publishSubmission, submitRequest } from '../../../agentic/compute/submission.mjs';
import {
  CONTENT_ROLE,
  CONTENT_ZOME,
  doorwayPayload,
  jwtClaims,
  payloadHeaders,
  RunnerSession,
} from '../../scripts/lib/compute-runner.js';

interface Sponsor {
  name: string;
  doorway: string;
  poolApi: string;
  performer: string;
}
interface Provider {
  name: string;
  api: string;
  key: string;
}
interface Config {
  adminKey: string;
  localToken: string;
  stageFeature: string;
  sponsors: Sponsor[];
  providers: Provider[];
  timeoutSeconds?: number;
}
interface GrantView {
  grantCid: string;
  scope: string;
  provider: string;
  recipient: string;
  state: string;
  validFrom: string;
  validUntil: string;
  bounds: { reach_ceiling?: string } | null;
}
interface Runner {
  sponsor: Sponsor;
  identifier: string;
  serviceAgentId: string;
  jwt: string;
  agentKey: string;
  account: Record<string, unknown>;
  session: RunnerSession;
  closed?: boolean;
}
interface Grant {
  provider: Provider;
  recipient: string;
  grantCid: string;
  grantActionHash: string;
  withdrawn?: boolean;
}
interface Submission {
  runner: Runner;
  provider: Provider;
  grant: Grant;
  requestActionHash: string;
  taskCid: string;
  envelope: Record<string, unknown>;
  admission?: { status: number; reason?: string };
}
interface State {
  config: Config;
  runners: Record<'first' | 'second', Runner | undefined>;
  grants: Grant[];
  submissions: Submission[];
  scratch: string[];
  deviceRefusal?: unknown;
  publication?: { status: number; body: Record<string, unknown> };
}

const states = new WeakMap<object, State>();
/** Every runner key this run has sponsored, across scenarios (Background step 3 reads it). */
const sponsoredThisRun = new Set<string>();
const repoRoot = resolve('../..');
// Loaded lazily: cid-executor.mjs imports ESM-only codecs (@ipld/dag-cbor) that cucumber's
// CommonJS step loader cannot require.
const cidExecute = async (program: string, args: string[]): Promise<string> =>
  (await import('../../../agentic/compute/cid-executor.mjs')).execute(program, args);
const pause = async (ms: number) => new Promise(r => setTimeout(r, ms));

function state(world: object): State {
  const found = states.get(world);
  assert(found, 'the household cast was not loaded (Background)');
  return found;
}

function localHeaders(key: string, token: string): Record<string, string> {
  return {
    'content-type': 'application/json',
    'x-elohim-verified-performer': key,
    'x-elohim-compute-token': token,
  };
}

async function json(response: Response): Promise<Record<string, unknown>> {
  const text = await response.text();
  try {
    return JSON.parse(text) as Record<string, unknown>;
  } catch {
    return { raw: text.slice(0, 300) };
  }
}

/** A provider's (or sponsor pool peer's) own read of the grants it issued. */
async function grantsIssued(
  api: string,
  key: string,
  token: string,
  filter: { recipient?: string; scope?: string }
): Promise<GrantView[]> {
  const url = new URL('/api/v1/compute/grants', api);
  if (filter.recipient) url.searchParams.set('recipient', filter.recipient);
  if (filter.scope) url.searchParams.set('scope', filter.scope);
  const response = await fetch(url, {
    headers: localHeaders(key, token),
    signal: AbortSignal.timeout(30000),
  });
  assert.equal(response.status, 200, `grant read on ${api} returned ${response.status}`);
  return ((await response.json()) as { grants: GrantView[] }).grants;
}

/** One sponsorship, retried ONCE when the household substrate faltered under load — the
 * doorway's MongoDB write was interrupted (DB_ERROR) or the conductor refused the install
 * (PROVISIONING_FAILED, e.g. its SQLite `database is locked`). The doorway unwinds that
 * attempt, so the retry is a fresh registration under a fresh id (backlog
 * household-doorway-mongo-insert-interrupted-under-load). Any other refusal fails the step. */
async function sponsorRunner(config: Config, sponsor: Sponsor): Promise<Runner> {
  try {
    return await sponsorOnce(config, sponsor);
  } catch (error) {
    if (!(error instanceof DoorwayDbError)) throw error;
    console.warn(
      `[ci-runner] ${sponsor.name}'s doorway lost a write (${error.message}); retrying once`
    );
    await pause(3000);
    return sponsorOnce(config, sponsor);
  }
}

class DoorwayDbError extends Error {}

async function sponsorOnce(config: Config, sponsor: Sponsor): Promise<Runner> {
  const slug = `ci-runner-story-${randomBytes(4).toString('hex')}`;
  const identifier = `${slug}@household.local`;
  const serviceAgentId = `agent:${slug}`;
  const password = randomBytes(24).toString('hex');
  const register = await fetch(new URL('/auth/register', sponsor.doorway), {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      identifier,
      identifierType: 'email',
      password,
      displayName: `Story runner sponsored by ${sponsor.name}`,
      agencyPhase: 'service-agent',
      serviceAgentId,
      adminBootstrapKey: config.adminKey,
    }),
    signal: AbortSignal.timeout(170000),
  });
  const registered = await json(register);
  if (
    (register.status === 500 && registered.code === 'DB_ERROR') ||
    (register.status === 503 && registered.code === 'PROVISIONING_FAILED')
  )
    throw new DoorwayDbError(String(registered.error ?? registered.code));
  assert(
    [200, 201].includes(register.status),
    `sponsoring through ${sponsor.name}'s doorway returned ${register.status}: ${JSON.stringify(registered)}`
  );
  // Login is idempotent: a 5xx (the doorway's MongoDB read interrupted, same backlog as
  // above) is retried once, logged; anything else fails the step.
  const logIn = async () =>
    fetch(new URL('/auth/login', sponsor.doorway), {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ identifier, password }),
      signal: AbortSignal.timeout(30000),
    });
  let login = await logIn();
  if (login.status >= 500) {
    console.warn(
      `[ci-runner] ${sponsor.name}'s doorway answered login ${login.status}; retrying once`
    );
    await pause(3000);
    login = await logIn();
  }
  assert.equal(login.status, 200, `runner login returned ${login.status}`);
  const jwt = String((await json(login)).token ?? '');
  assert(jwt, 'runner login returned no token');
  const account = await readAccount(sponsor, jwt);
  const agentKey = String(jwtClaims(jwt).agentPubKey ?? account.agentPubKey ?? '');
  assert(agentKey.startsWith('uhCAk'), `the runner session names no agent key (${agentKey})`);
  sponsoredThisRun.add(agentKey);
  const session = new RunnerSession({
    env: {
      doorway: new URL(sponsor.doorway).origin,
      jwt,
      adminKey: config.adminKey,
      root: 'genesis/a2o/reports/compute-runner',
    },
  });
  return { sponsor, identifier, serviceAgentId, jwt, agentKey, account, session };
}

async function readAccount(sponsor: Sponsor, jwt: string): Promise<Record<string, unknown>> {
  const response = await fetch(new URL('/auth/account', sponsor.doorway), {
    headers: { authorization: `Bearer ${jwt}` },
    signal: AbortSignal.timeout(30000),
  });
  assert.equal(response.status, 200, `GET /auth/account returned ${response.status}`);
  return json(response);
}

async function grantCapacity(config: Config, provider: Provider, runner: Runner): Promise<Grant> {
  const now = new Date();
  const until = new Date(now.getTime() + 86_400_000);
  const stamp = (d: Date) => `${d.toISOString().slice(0, 19)}Z`;
  const response = await fetch(new URL('/api/v1/compute/grants', provider.api), {
    method: 'POST',
    headers: localHeaders(provider.key, config.localToken),
    body: JSON.stringify({
      recipient: runner.agentKey,
      scope: 'measure-stage',
      issuedAt: stamp(now),
      validFrom: stamp(now),
      validUntil: stamp(until),
      bounds: {
        epr_scope: ['*'],
        reach_ceiling: 'commons',
        rate_per_hour: 8,
        rotation_ttl_days: 1,
      },
    }),
    signal: AbortSignal.timeout(120000),
  });
  const body = await json(response);
  assert.equal(
    response.status,
    200,
    `${provider.name}'s grant returned ${response.status}: ${JSON.stringify(body)}`
  );
  return {
    provider,
    recipient: runner.agentKey,
    grantCid: String(body.grantCid),
    grantActionHash: String(body.grantActionHash),
  };
}

async function withdrawGrant(api: string, key: string, token: string, grantCid: string) {
  const response = await fetch(new URL('/api/v1/compute/grants', api), {
    method: 'POST',
    headers: localHeaders(key, token),
    body: JSON.stringify({ grantCid, revoke: true }),
    signal: AbortSignal.timeout(120000),
  });
  const body = await json(response);
  assert.equal(
    response.status,
    200,
    `withdrawal returned ${response.status}: ${JSON.stringify(body)}`
  );
  return body;
}

/** The runner names a pinned stage for `provider`, stocks its inputs in its sponsor's
 * custody, and submits it on its own cell under `grant`. */
async function submitStage(st: State, runner: Runner, provider: Provider, grant: Grant) {
  const out = await mkdtemp(join(tmpdir(), 'ci-runner-stage-'));
  st.scratch.push(out);
  const executor = resolve(repoRoot, 'genesis/agentic/compute/cid-executor.mjs');
  const runtimeImage = `sha256:${createHash('sha256')
    .update(await readFile(executor))
    .digest('hex')}`;
  const stage = await buildStageTask({
    feature: st.config.stageFeature,
    requester: runner.agentKey,
    provider: provider.key,
    out: join(out, 'stage'),
    runtimeImage,
    executor,
    execute: cidExecute,
    probeRun: false,
    repoRoot,
  });
  const { envelope, taskCid } = await publishSubmission({
    envelope: stage.envelope,
    inputPaths: [stage.runnerPath, stage.dnaPath],
    file: stage.taskPath,
    root: out,
    executor,
    payload: doorwayPayload({ doorway: runner.sponsor.doorway, jwt: runner.jwt }),
    execute: cidExecute,
    newRun: true,
  });
  const status = await runner.session.call<Record<string, unknown>>(
    CONTENT_ROLE,
    CONTENT_ZOME,
    'compute_task',
    {
      ...submitRequest({ envelope, taskCid, grantActionHash: grant.grantActionHash }),
      operation: 'submit',
    }
  );
  const requestActionHash = String(status.requestActionHash ?? '');
  assert(requestActionHash, `the runner's submit returned no request: ${JSON.stringify(status)}`);
  const submission: Submission = {
    runner,
    provider,
    grant,
    requestActionHash,
    taskCid,
    envelope: envelope,
  };
  st.submissions.push(submission);
  return submission;
}

/** The provider's storage decides admission for one task, as its worker would ask it to;
 * a refusal is recorded as the provider's decline so the runner can read why. */
async function providerAdmits(st: State, submission: Submission) {
  const { provider } = submission;
  const headers = localHeaders(provider.key, st.config.localToken);
  const ref = encodeURIComponent(submission.requestActionHash);
  const attemptId = randomUUID();
  const deadline = Date.now() + (st.config.timeoutSeconds ?? 120) * 1000;
  // The provider sees the task once it has gossiped from the runner's cell.
  for (;;) {
    const seen = await fetch(new URL(`/api/v1/compute/tasks/${ref}`, provider.api), {
      headers,
      signal: AbortSignal.timeout(30000),
    });
    if (seen.status === 200) break;
    assert(Date.now() < deadline, `${provider.name} never saw the task (last ${seen.status})`);
    await pause(2000);
  }
  const accept = await fetch(new URL(`/api/v1/compute/tasks/${ref}/accept`, provider.api), {
    method: 'POST',
    headers,
    body: JSON.stringify({ attemptId }),
    signal: AbortSignal.timeout(120000),
  });
  const body = await json(accept);
  submission.admission = { status: accept.status, reason: body.reason as string | undefined };
  if (accept.status === 403) {
    const decline = await fetch(new URL(`/api/v1/compute/tasks/${ref}/decline`, provider.api), {
      method: 'POST',
      headers,
      body: JSON.stringify({ attemptId, reason: String(body.reason ?? 'compute-grant-refused') }),
      signal: AbortSignal.timeout(120000),
    });
    assert.equal(decline.status, 200, `${provider.name}'s decline returned ${decline.status}`);
  }
  return submission.admission;
}

/** The runner's own reading of its task, through its own cell. */
async function runnerReads(
  submission: Submission,
  until: (task: Record<string, unknown>) => boolean
) {
  const deadline = Date.now() + 120_000;
  for (;;) {
    const task = await submission.runner.session.call<Record<string, unknown>>(
      CONTENT_ROLE,
      CONTENT_ZOME,
      'compute_task',
      { operation: 'get', requestActionHash: submission.requestActionHash }
    );
    if (until(task)) return task;
    assert(
      Date.now() < deadline,
      `the runner's task never showed the expected state: ${JSON.stringify(task).slice(0, 400)}`
    );
    await pause(2000);
  }
}

function runner(st: State, which: 'first' | 'second'): Runner {
  const found = st.runners[which];
  assert(found, `the ${which} steward has not sponsored a runner in this scenario`);
  return found;
}

function grantFor(st: State, provider: Provider, recipient: string): Grant {
  const found = st.grants.find(g => g.provider === provider && g.recipient === recipient);
  assert(found, `${provider.name} has granted no capacity to ${recipient}`);
  return found;
}

function assertStatesReachAndEnd(grant: GrantView, label: string) {
  assert.equal(grant.bounds?.reach_ceiling, 'commons', `${label} must state its reach`);
  const ends = Date.parse(grant.validUntil);
  assert(
    Number.isFinite(ends),
    `${label} must state when it ends (validUntil ${grant.validUntil})`
  );
  assert(ends > Date.now(), `${label} has already ended (${grant.validUntil})`);
}

// ---------------------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------------------

Given('a household with two stewards, each running their own peer', async function () {
  const file = process.env.CI_RUNNER_A2O_CONFIG;
  if (!file) return 'pending';
  const config = JSON.parse(await readFile(file, 'utf8')) as Config;
  assert.equal(config.sponsors?.length, 2, 'the fixture must name two sponsoring stewards');
  const [a, b] = config.sponsors;
  assert.notEqual(a.performer, b.performer, 'the two stewards must run two different peers');
  for (const sponsor of config.sponsors) {
    const health = await fetch(new URL('/health', sponsor.doorway), {
      signal: AbortSignal.timeout(10000),
    });
    assert.equal(health.status, 200, `${sponsor.name}'s doorway is not up`);
    await grantsIssued(sponsor.poolApi, sponsor.performer, config.localToken, {
      scope: 'hosted-cell',
    });
  }
  states.set(this, {
    config,
    runners: { first: undefined, second: undefined },
    grants: [],
    submissions: [],
    scratch: [],
  });
});

Given(
  'two more stewards, each running a peer, who will provide capacity and sponsor no runner',
  async function () {
    const st = state(this);
    const { providers, sponsors } = st.config;
    assert.equal(providers?.length, 2, 'the fixture must name two providing stewards');
    const keys = new Set([...sponsors.map(s => s.performer), ...providers.map(p => p.key)]);
    assert.equal(keys.size, 4, 'four stewards, four peers: no one may play two roles here');
    for (const provider of providers)
      await grantsIssued(provider.api, provider.key, st.config.localToken, {
        scope: 'measure-stage',
      });
  }
);

Given('neither of those two providers has granted any capacity yet', async function () {
  const st = state(this);
  for (const provider of st.config.providers) {
    const live = (
      await grantsIssued(provider.api, provider.key, st.config.localToken, {
        scope: 'measure-stage',
      })
    ).filter(g => g.state === 'active' && sponsoredThisRun.has(g.recipient));
    assert.deepEqual(
      live.map(g => g.grantCid),
      [],
      `${provider.name} still lends capacity to a runner from an earlier scenario`
    );
  }
});

Given(
  "a stage taken from the household's own checks and pinned by the fingerprints of its bytes",
  async function () {
    const st = state(this);
    const bytes = await readFile(resolve(repoRoot, 'genesis/a2o', st.config.stageFeature));
    const pinned = JSON.parse(
      await cidExecute('cid-executor', [
        'artifact',
        resolve(repoRoot, 'genesis/a2o', st.config.stageFeature),
      ])
    ) as {
      sha256: string;
    };
    assert.equal(pinned.sha256, createHash('sha256').update(bytes).digest('hex'));
  }
);

// ---------------------------------------------------------------------------------------
// Sponsorship
// ---------------------------------------------------------------------------------------

When('the first steward sponsors a runner', { timeout: 240000 }, async function () {
  const st = state(this);
  st.runners.first = await sponsorRunner(st.config, st.config.sponsors[0]);
});

Given('the first steward has sponsored a runner', { timeout: 240000 }, async function () {
  const st = state(this);
  st.runners.first = await sponsorRunner(st.config, st.config.sponsors[0]);
});

Given(
  'the second steward has sponsored a runner of their own',
  { timeout: 240000 },
  async function () {
    const st = state(this);
    st.runners.second = await sponsorRunner(st.config, st.config.sponsors[1]);
    assert.notEqual(st.runners.second.agentKey, runner(st, 'first').agentKey);
  }
);

Then(
  "the runner has its own agent key, distinct from the steward's",
  { timeout: 120000 },
  async function () {
    const st = state(this);
    const r = runner(st, 'first');
    assert.notEqual(r.agentKey, r.sponsor.performer, "the runner must not hold its sponsor's key");
    // The key is a live cell: the runner reaches it through its own session.
    assert.equal(await r.session.whoami(), r.agentKey);
  }
);

Then("no person's identity record is associated with the runner's key", async function () {
  const st = state(this);
  const r = runner(st, 'first');
  assert.equal(r.account.agencyPhase, 'service-agent');
  assert.notEqual(r.account.permissionLevel, 'ADMIN', 'a runner is never its doorway’s admin');
  const core = Buffer.from(decodeHashFromBase64(r.agentKey).slice(3, 35)).toString('base64');
  const response = await fetch(new URL('/db/humans', r.sponsor.poolApi), {
    signal: AbortSignal.timeout(30000),
  });
  assert.equal(response.status, 200);
  const humans = ((await response.json()) as { items: { id: string; agentPubKey: string }[] })
    .items;
  const named = humans.filter(h => h.agentPubKey === core || h.agentPubKey === r.agentKey);
  assert.deepEqual(
    named.map(h => h.id),
    [],
    'a human record names the runner’s key'
  );
});

Then(
  "exactly one hosting grant names the first steward's peer as provider and the runner as recipient",
  async function () {
    const st = state(this);
    const r = runner(st, 'first');
    const grants = await grantsIssued(
      r.sponsor.poolApi,
      r.sponsor.performer,
      st.config.localToken,
      {
        recipient: r.agentKey,
        scope: 'hosted-cell',
      }
    );
    assert.equal(grants.length, 1, `expected one hosting grant, saw ${grants.length}`);
    const [grant] = grants;
    assert.equal(grant.provider, r.sponsor.performer);
    assert.equal(grant.recipient, r.agentKey);
    assert.equal(grant.state, 'active');
    assert.equal(grant.grantCid, r.account.hostedCellGrantCid);
  }
);

Then('the hosting grant states its reach and when it ends', async function () {
  const st = state(this);
  const r = runner(st, 'first');
  const [grant] = await grantsIssued(r.sponsor.poolApi, r.sponsor.performer, st.config.localToken, {
    recipient: r.agentKey,
    scope: 'hosted-cell',
  });
  assertStatesReachAndEnd(grant, 'the hosting grant');
});

// ---------------------------------------------------------------------------------------
// Capacity
// ---------------------------------------------------------------------------------------

When(
  'the first provider grants the runner capacity to run stages',
  { timeout: 180000 },
  async function () {
    const st = state(this);
    st.grants.push(await grantCapacity(st.config, st.config.providers[0], runner(st, 'first')));
  }
);

Given(
  "the first provider has granted capacity to the first steward's runner only",
  { timeout: 180000 },
  async function () {
    const st = state(this);
    const provider = st.config.providers[0];
    st.grants.push(await grantCapacity(st.config, provider, runner(st, 'first')));
    const toSecond = await grantsIssued(provider.api, provider.key, st.config.localToken, {
      recipient: runner(st, 'second').agentKey,
    });
    assert.deepEqual(
      toSecond,
      [],
      "the second steward's runner must hold no grant from this provider"
    );
  }
);

Given(
  'both providers have each granted the runner capacity',
  { timeout: 300000 },
  async function () {
    const st = state(this);
    for (const provider of st.config.providers)
      st.grants.push(await grantCapacity(st.config, provider, runner(st, 'first')));
  }
);

When(
  'the runner submits a pinned stage to the first provider',
  { timeout: 300000 },
  async function () {
    const st = state(this);
    const r = runner(st, 'first');
    const provider = st.config.providers[0];
    await submitStage(st, r, provider, grantFor(st, provider, r.agentKey));
  }
);

Then('the capacity grant states its reach and when it ends', async function () {
  const st = state(this);
  const provider = st.config.providers[0];
  const [grant] = await grantsIssued(provider.api, provider.key, st.config.localToken, {
    recipient: runner(st, 'first').agentKey,
    scope: 'measure-stage',
  });
  assert(grant, 'the capacity grant is not on the provider’s record');
  assertStatesReachAndEnd(grant, 'the capacity grant');
});

Then(
  'the first provider accepts the stage under that capacity grant',
  { timeout: 300000 },
  async function () {
    const st = state(this);
    const submission = st.submissions.at(-1)!;
    const admission = await providerAdmits(st, submission);
    assert.equal(admission.status, 200, `the provider refused: ${admission.reason ?? '?'}`);
    const task = await runnerReads(submission, t => Boolean(t.acceptance));
    assert(task.acceptance, 'the runner must see the acceptance on its own task');
  }
);

Then(
  "the runner reads back from the first provider a report that names the pinned stage and each scenario's outcome",
  { timeout: 2600000 },
  async function () {
    const st = state(this);
    const submission = st.submissions.at(-1)!;
    const deadline = Date.now() + 2_500_000;
    let task = await runnerReads(submission, () => true);
    while (!task.completion && !task.refusal) {
      assert(Date.now() < deadline, 'the stage never completed');
      await pause(5000);
      task = await runnerReads(submission, () => true);
    }
    assert(
      task.completion,
      `the stage ended without a completion: ${JSON.stringify(task.refusal)}`
    );
    const receipt = (task.completion as { receipt: { logs: { name: string }[] } }).receipt;
    const stdoutLog = receipt.logs.find(log => log.name === 'stdout.log');
    assert(stdoutLog, 'the receipt names no stdout.log');
    const dir = await mkdtemp(join(tmpdir(), 'ci-runner-report-'));
    st.scratch.push(dir);
    const path = join(dir, 'stdout.log');
    await materialize(
      stdoutLog as Parameters<typeof materialize>[0],
      path,
      doorwayPayload({
        doorway: submission.runner.sponsor.doorway,
        jwt: submission.runner.jwt,
        request: submission.requestActionHash,
      }),
      submission.taskCid,
      0
    );
    const stdout = await readFile(path, 'utf8');
    const begin = '-----BEGIN ELOHIM STAGE REPORT-----';
    const end = '-----END ELOHIM STAGE REPORT-----';
    const start = stdout.indexOf(begin);
    const stop = stdout.indexOf(end);
    assert(start >= 0 && stop > start, 'the report frame is missing');
    const doc = JSON.parse(
      Buffer.from(stdout.slice(start + begin.length, stop).trim(), 'base64').toString('utf8')
    ) as {
      uri?: string;
      elements?: { type: string; name: string; steps?: { result?: { status?: string } }[] }[];
    }[];
    const feature = doc.find(f => f.uri === st.config.stageFeature);
    assert(feature, `the report does not name ${st.config.stageFeature}`);
    const scenarios = (feature.elements ?? []).filter(e => e.type === 'scenario');
    assert(scenarios.length > 0, 'the report names no scenario');
    for (const scenario of scenarios)
      assert(
        (scenario.steps ?? []).every(s => s.result?.status),
        `scenario "${scenario.name}" carries no outcome`
      );
  }
);

// ---------------------------------------------------------------------------------------
// One runner cannot spend another's promise
// ---------------------------------------------------------------------------------------

When(
  "the second steward's runner submits a pinned stage to the first provider",
  { timeout: 300000 },
  async function () {
    const st = state(this);
    const provider = st.config.providers[0];
    // It holds no grant of its own here, so the only promise it could name is the one
    // made to the first steward's runner.
    await submitStage(
      st,
      runner(st, 'second'),
      provider,
      grantFor(st, provider, runner(st, 'first').agentKey)
    );
  }
);

Then(
  'the first provider refuses the stage, naming the missing capacity grant',
  { timeout: 300000 },
  async function () {
    const st = state(this);
    const submission = st.submissions.at(-1)!;
    const admission = await providerAdmits(st, submission);
    assert.equal(
      admission.status,
      403,
      `the provider admitted a stage it should refuse (${admission.status})`
    );
    assert.equal(admission.reason, 'compute-grant-refused');
    const task = await runnerReads(submission, t => Boolean(t.refusal));
    assert.equal((task.refusal as { reason?: string }).reason, 'compute-grant-refused');
  }
);

Then(
  "the first steward's runner can still have a stage accepted by the first provider",
  { timeout: 600000 },
  async function () {
    const st = state(this);
    const r = runner(st, 'first');
    const provider = st.config.providers[0];
    const submission = await submitStage(st, r, provider, grantFor(st, provider, r.agentKey));
    const admission = await providerAdmits(st, submission);
    assert.equal(admission.status, 200, `the provider refused: ${admission.reason ?? '?'}`);
    await runnerReads(submission, t => Boolean(t.acceptance));
  }
);

// ---------------------------------------------------------------------------------------
// Withdrawal
// ---------------------------------------------------------------------------------------

When('the first provider withdraws its capacity grant', { timeout: 180000 }, async function () {
  const st = state(this);
  const provider = st.config.providers[0];
  const grant = grantFor(st, provider, runner(st, 'first').agentKey);
  await withdrawGrant(provider.api, provider.key, st.config.localToken, grant.grantCid);
  grant.withdrawn = true;
});

Then("the first provider refuses the runner's next stage", { timeout: 600000 }, async function () {
  const st = state(this);
  const r = runner(st, 'first');
  const provider = st.config.providers[0];
  const submission = await submitStage(st, r, provider, grantFor(st, provider, r.agentKey));
  const admission = await providerAdmits(st, submission);
  assert.equal(admission.status, 403, `a withdrawn grant admitted a stage (${admission.status})`);
  await runnerReads(submission, t => Boolean(t.refusal));
});

Then('the second provider still accepts it', { timeout: 600000 }, async function () {
  const st = state(this);
  const r = runner(st, 'first');
  const provider = st.config.providers[1];
  const submission = await submitStage(st, r, provider, grantFor(st, provider, r.agentKey));
  const admission = await providerAdmits(st, submission);
  assert.equal(admission.status, 200, `the second provider refused: ${admission.reason ?? '?'}`);
  await runnerReads(submission, t => Boolean(t.acceptance));
});

When('the first steward withdraws the hosting grant', { timeout: 180000 }, async function () {
  const st = state(this);
  const r = runner(st, 'first');
  await withdrawGrant(
    r.sponsor.poolApi,
    r.sponsor.performer,
    st.config.localToken,
    String(r.account.hostedCellGrantCid)
  );
});

Then(
  "the runner's key still exists, but its next request is refused, naming the withdrawn hosting grant",
  { timeout: 120000 },
  async function () {
    const st = state(this);
    const r = runner(st, 'first');
    const account = await readAccount(r.sponsor, r.jwt);
    assert.equal(
      account.agentPubKey ?? r.agentKey,
      r.agentKey,
      'the runner’s key must still be its own'
    );
    const bytes = randomBytes(64);
    const dir = await mkdtemp(join(tmpdir(), 'ci-runner-probe-'));
    st.scratch.push(dir);
    const path = join(dir, 'probe');
    await writeFile(path, bytes);
    const { cid } = JSON.parse(await cidExecute('cid-executor', ['artifact', path])) as {
      cid: string;
    };
    const response = await fetch(new URL(`/api/v1/compute/payloads/${cid}`, r.sponsor.doorway), {
      method: 'PUT',
      body: bytes,
      headers: payloadHeaders({ jwt: r.jwt, owner: 'bafyrei-withdrawn-probe', retention: 60 }),
      signal: AbortSignal.timeout(60000),
    });
    const body = await json(response);
    assert.equal(response.status, 403, `the runner was still served: ${response.status}`);
    assert.equal(body.reason, 'hosted-cell-grant-withdrawn');
  }
);

// ---------------------------------------------------------------------------------------
// A runner cannot speak as a person
// ---------------------------------------------------------------------------------------

When(
  "the runner tries to approve a new device in the first steward's name",
  { timeout: 120000 },
  async function () {
    const st = state(this);
    const r = runner(st, 'first');
    await r.session.whoami();
    try {
      await r.session.call('imagodei', 'imagodei', 'register_device_identity', {
        human_id: `human-${r.sponsor.name}`,
        device_agent_key: r.agentKey,
      });
      st.deviceRefusal = undefined;
    } catch (error) {
      st.deviceRefusal = error;
    }
  }
);

Then('the approval is refused', function () {
  const st = state(this);
  assert(st.deviceRefusal, 'the runner approved a device');
  const message = String((st.deviceRefusal as Error).message ?? st.deviceRefusal);
  assert.match(message, /unauthori[sz]ed|capabilit|cap.?grant|not.?granted/i, message);
});

When(
  'the runner tries to publish something more private than the commons',
  { timeout: 120000 },
  async function () {
    const st = state(this);
    const r = runner(st, 'first');
    const bytes = randomBytes(64);
    const dir = await mkdtemp(join(tmpdir(), 'ci-runner-reach-'));
    st.scratch.push(dir);
    const path = join(dir, 'private');
    await writeFile(path, bytes);
    const { cid } = JSON.parse(await cidExecute('cid-executor', ['artifact', path])) as {
      cid: string;
    };
    const response = await fetch(new URL(`/api/v1/compute/payloads/${cid}`, r.sponsor.doorway), {
      method: 'PUT',
      body: bytes,
      headers: payloadHeaders({
        jwt: r.jwt,
        owner: 'bafyrei-reach-probe',
        retention: 60,
        reach: 'intimate',
      }),
      signal: AbortSignal.timeout(60000),
    });
    st.publication = { status: response.status, body: await json(response) };
  }
);

Then("the publication is refused, naming the grant's reach", function () {
  const st = state(this);
  assert(st.publication, 'nothing was published');
  assert.equal(
    st.publication.status,
    403,
    `the publication was not refused: ${JSON.stringify(st.publication)}`
  );
  assert.equal(st.publication.body.reason, 'reach-narrower-than-hosting-grant');
  assert.equal(st.publication.body.grantReach, 'commons');
});

// ---------------------------------------------------------------------------------------
// Leave nothing lent and nobody hosted
// ---------------------------------------------------------------------------------------

After({ timeout: 300000 }, async function () {
  const st = states.get(this);
  if (!st) return;
  for (const grant of st.grants.filter(g => !g.withdrawn))
    await withdrawGrant(
      grant.provider.api,
      grant.provider.key,
      st.config.localToken,
      grant.grantCid
    ).catch(() => undefined);
  for (const r of Object.values(st.runners)) {
    if (!r || r.closed) continue;
    await r.session.close().catch(() => undefined);
    await fetch(new URL('/auth/close-account', r.sponsor.doorway), {
      method: 'POST',
      headers: { 'content-type': 'application/json', authorization: `Bearer ${r.jwt}` },
      body: JSON.stringify({ confirmIdentifier: r.identifier }),
      signal: AbortSignal.timeout(120000),
    }).catch(() => undefined);
    r.closed = true;
  }
  for (const dir of st.scratch) await rm(dir, { recursive: true, force: true });
  states.delete(this);
});
