/**
 * Peer-executed a2o stage — live probe against the household mesh's local compute
 * surface. Mirrors the phrasing of steps/dataplane/delegated-sweettest.steps.ts (the
 * substrate under measure is the same one) but owns its own state, per
 * genesis/docs/superpowers/specs/2026-09-08-peer-executed-stage-design.md §9 task 3:
 * "Own your state ... its phrasings cannot be reused without exporting that map."
 *
 * PEER_STAGE_A2O_CONFIG names a local JSON fixture (never committed):
 * { stageDir, requesterApi, providerApi, requesterKey, providerKey, env, timeoutSeconds }
 * `stageDir` is the output directory of build-stage-task.mjs (task.json, stage-runner.sh,
 * the copied <feature>.feature). `env` carries the workspace wrapper's variables
 * (COMPUTE_API_URL, COMPUTE_PERFORMER, ELOHIM_COMPUTE_LOCAL_TOKEN, COMPUTE_GRANT_ACTION,
 * COMPUTE_EXECUTOR, COMPUTE_INBOX_ROOT). Absent fixture ⇒ `return 'pending'` (never a
 * passing live result), the same discipline as delegated-sweettest.steps.ts:96-97.
 */
import { strict as assert } from 'node:assert';
import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readdir, readFile } from 'node:fs/promises';
import { basename, join, resolve } from 'node:path';
import { promisify } from 'node:util';

import { Given, Then, When } from '@cucumber/cucumber';

const execute = promisify(execFile);
const workspaceScript = resolve('../agentic/compute/workspace.mjs');

// Verbatim slug expression, design §3 D6 — duplicated deliberately rather than imported:
// the requester and the generated guest script must each compute it independently
// without communicating, so the two sides staying byte-identical IS the property proved.
const slugify = (s: string): string =>
  s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 96)
    .replace(/-+$/, '');

interface Artifact {
  cid: string;
  sha256: string;
  bytes: number;
  chunks?: Artifact[];
}
interface Envelope {
  project: string;
  binary: Artifact;
  dna: Artifact;
  expectedTests: string[];
  requester: string;
  provider: string;
  invocation?: string;
}
interface Receipt {
  binary: Artifact;
  dna: Artifact;
  provider: string;
  status: string;
  logs: Array<{ name: string; chunks?: Artifact[] }>;
}
interface Status {
  requestActionHash: string;
  taskCid: string;
  provider: string;
  envelope: Envelope;
  acceptance?: { actionHash: string };
  completion?: { actionHash: string; receipt: Receipt };
  refusal?: { reason: string };
}
interface Fixture {
  stageDir: string;
  requesterApi: string;
  providerApi: string;
  requesterKey: string;
  providerKey: string;
  env: NodeJS.ProcessEnv;
  timeoutSeconds?: number;
}
interface CucumberScenarioElement {
  type: string;
  name: string;
  steps?: Array<{ result?: { status?: string } }>;
}
interface CucumberFeatureDoc {
  uri?: string;
  elements?: CucumberScenarioElement[];
}
interface State {
  config: Fixture;
  env: NodeJS.ProcessEnv;
  taskJson: Envelope;
  featureFileName: string;
  featureAbsPath: string;
  featureRel: string;
  scenarioSlugs: string[];
  reference: string;
  taskCid: string;
  invocation: string;
  submitted: Status;
  secondSubmitted: Status;
  status: Status;
  report: CucumberFeatureDoc[];
  reportFeature: CucumberFeatureDoc;
}
const states = new WeakMap<object, State>();
const pause = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

function assertLoopback(url: string, label: string) {
  const parsed = new URL(url);
  assert(
    ['localhost', '127.0.0.1', '[::1]'].includes(parsed.hostname),
    `${label} must be a loopback compute adapter, got ${url}`
  );
}

async function localRead(base: string, performer: string, token: string, reference: string): Promise<Status> {
  const response = await fetch(
    new URL(`/api/v1/compute/tasks/${encodeURIComponent(reference)}`, base),
    {
      headers: {
        'x-elohim-verified-performer': performer,
        'x-elohim-compute-token': token,
      },
      redirect: 'error',
      signal: AbortSignal.timeout(30000),
    }
  );
  assert.equal(response.status, 200, `native task read returned ${response.status}`);
  return response.json() as Promise<Status>;
}

async function waitFor(
  base: string,
  performer: string,
  token: string,
  reference: string,
  timeoutSeconds: number,
  predicate: (value: Status) => boolean
): Promise<Status> {
  const until = Date.now() + timeoutSeconds * 1000;
  while (Date.now() < until) {
    const status = await localRead(base, performer, token, reference);
    if (predicate(status)) return status;
    await pause(2000);
  }
  assert.fail('peer-executed stage deadline expired without the required evidence');
}

// The D7 sentinel framing (design §3 D7): the cucumber report travels base64-encoded
// between two literal marker lines inside the stdout.log payload.
function decodeFramedReport(stdout: string): CucumberFeatureDoc[] {
  const begin = '-----BEGIN ELOHIM STAGE REPORT-----';
  const end = '-----END ELOHIM STAGE REPORT-----';
  const start = stdout.indexOf(begin);
  const stop = stdout.indexOf(end);
  assert(start >= 0 && stop > start, 'stdout.log did not carry a framed ELOHIM STAGE REPORT');
  const encoded = stdout.slice(start + begin.length, stop).trim();
  return JSON.parse(Buffer.from(encoded, 'base64').toString('utf8')) as CucumberFeatureDoc[];
}

Given(
  'a household compute provider with a signed compute grant for this requester',
  async function () {
    const file = process.env.PEER_STAGE_A2O_CONFIG;
    if (!file) return 'pending';
    const config = JSON.parse(await readFile(file, 'utf8')) as Fixture;
    assert(
      config.stageDir && config.requesterApi && config.providerApi && config.requesterKey && config.providerKey,
      'incomplete peer-stage fixture'
    );
    assertLoopback(config.requesterApi, 'requesterApi');
    assertLoopback(config.providerApi, 'providerApi');
    const env = { ...process.env, ...config.env };
    assert(env.COMPUTE_GRANT_ACTION, 'fixture must name the accepted compute grant action hash');
    states.set(this, { config, env } as State);
  }
);

Given(
  'a pinned stage whose runner script and feature file are content-addressed',
  async function () {
    const state = states.get(this)!;
    const taskJson = JSON.parse(
      await readFile(join(state.config.stageDir, 'task.json'), 'utf8')
    ) as Envelope;
    state.taskJson = taskJson;

    const executor = state.env.COMPUTE_EXECUTOR || 'compute-executor';
    const entries = await readdir(state.config.stageDir);
    const featureFileName = entries.find(name => name.endsWith('.feature'));
    assert(featureFileName, `no .feature file staged in ${state.config.stageDir}`);
    state.featureFileName = featureFileName!;

    const runnerDescriptor = JSON.parse(
      (await execute(executor, ['artifact', join(state.config.stageDir, 'stage-runner.sh')])).stdout
    ) as Artifact;
    assert.equal(runnerDescriptor.cid, taskJson.binary.cid, 'binary cid must match the staged runner');
    assert.equal(runnerDescriptor.sha256, taskJson.binary.sha256);
    assert.equal(runnerDescriptor.bytes, taskJson.binary.bytes);

    const dnaDescriptor = JSON.parse(
      (await execute(executor, ['artifact', join(state.config.stageDir, featureFileName!)])).stdout
    ) as Artifact;
    assert.equal(dnaDescriptor.cid, taskJson.dna.cid, 'dna cid must match the staged feature file');
    assert.equal(dnaDescriptor.sha256, taskJson.dna.sha256);
    assert.equal(dnaDescriptor.bytes, taskJson.dna.bytes);

    // D6: feedback_signal::a2o::<feature-dir>::<feature-slug>::<scenario-slug>
    const parts = taskJson.expectedTests[0].split('::');
    assert(parts[0] === 'feedback_signal' && parts[1] === 'a2o', 'unexpected expectedTests namespace');
    const featureDir = parts[2];
    const featureSlug = parts[3];
    state.featureRel = `features/${featureDir}/${featureSlug}.feature`;
    state.featureAbsPath = resolve('../a2o', state.featureRel);
    state.scenarioSlugs = taskJson.expectedTests.map(entry => entry.split('::').slice(4).join('::'));

    const repoFeatureBytes = await readFile(state.featureAbsPath);
    const repoFeatureSha256 = createHash('sha256').update(repoFeatureBytes).digest('hex');
    assert.equal(
      taskJson.dna.sha256,
      repoFeatureSha256,
      'the pinned dna slot must equal the repo copy of the feature file byte-for-byte'
    );
  }
);

When('the requester submits the stage to the provider', { timeout: 300000 }, async function () {
  const state = states.get(this)!;
  const { stdout } = await execute(
    process.execPath,
    [
      workspaceScript,
      'submit',
      join(state.config.stageDir, 'task.json'),
      join(state.config.stageDir, 'stage-runner.sh'),
      join(state.config.stageDir, state.featureFileName),
    ],
    { env: state.env, timeout: 300000, maxBuffer: 1024 * 1024 }
  );
  state.submitted = JSON.parse(stdout) as Status;
  state.reference = state.submitted.requestActionHash;
  state.taskCid = state.submitted.taskCid;
  assert(state.reference, 'submission must return its native action reference');
  const written = JSON.parse(
    await readFile(join(state.config.stageDir, 'task.json'), 'utf8')
  ) as Envelope;
  assert(written.invocation, 'submission must write back an invocation nonce');
  state.invocation = written.invocation!;
});

Then('the provider accepts the stage and runs it on its own substrate', { timeout: 300000 }, async function () {
  const state = states.get(this)!;
  const status = await waitFor(
    state.config.requesterApi,
    state.config.requesterKey,
    state.env.ELOHIM_COMPUTE_LOCAL_TOKEN ?? '',
    state.reference,
    state.config.timeoutSeconds ?? 300,
    value => Boolean(value.acceptance)
  );
  assert(status.acceptance, 'expected an acceptance before completion');
});

Then(
  "the requester recovers the provider's signed completion for the pinned stage",
  { timeout: 2600000 },
  async function () {
    const state = states.get(this)!;
    const status = await waitFor(
      state.config.requesterApi,
      state.config.requesterKey,
      state.env.ELOHIM_COMPUTE_LOCAL_TOKEN ?? '',
      state.reference,
      state.config.timeoutSeconds ?? 2500,
      value => Boolean(value.completion)
    );
    state.status = status;
    // Materializes the leased payload chunks (§7.1 step 3) into COMPUTE_INBOX_ROOT so the
    // next step can decode the framed report from the local file.
    await execute(process.execPath, [workspaceScript, 'poll'], {
      env: state.env,
      timeout: 1250000,
      maxBuffer: 1024 * 1024,
    });
    const { completion } = status;
    assert(completion, 'completion must be present');
    assert.equal(status.taskCid, state.taskCid);
    assert.deepEqual(completion!.receipt.binary, state.submitted.envelope.binary);
    assert.deepEqual(completion!.receipt.dna, state.submitted.envelope.dna);
    assert.equal(completion!.receipt.provider, state.config.providerKey);
  }
);

Then(
  'the returned report names the pinned feature file and every declared scenario',
  async function () {
    const state = states.get(this)!;
    const actionHash = state.status.completion!.actionHash;
    const root = resolve(state.env.COMPUTE_INBOX_ROOT || 'genesis/a2o/reports/compute');
    const directory = join(root, 'logs', createHash('sha256').update(actionHash).digest('hex'));
    const stdoutPath = join(directory, 'stdout.log');
    const stdout = await readFile(stdoutPath, 'utf8');
    const doc = decodeFramedReport(stdout);
    state.report = doc;

    const matching = doc.filter(feature => feature.uri === state.featureRel);
    assert.equal(
      matching.length,
      1,
      `expected exactly one report entry for ${state.featureRel}, saw ${doc.map(f => f.uri).join(', ')}`
    );
    state.reportFeature = matching[0];

    const seenSlugs = new Set(
      (state.reportFeature.elements || [])
        .filter(element => element.type === 'scenario')
        .map(element => slugify(element.name))
    );
    assert.deepEqual(
      [...seenSlugs].sort(),
      [...state.scenarioSlugs].sort(),
      'the report must name exactly the declared scenario inventory, no more, no fewer'
    );
  }
);

Then('every declared scenario passed in the report, not merely in the receipt', function () {
  // D9: the verdict reads the decoded report, never receipt.status — a provider's
  // status: "passed" is a claim; this assertion checks the evidence the run left behind.
  const state = states.get(this)!;
  const scenarios = (state.reportFeature.elements || []).filter(
    element => element.type === 'scenario'
  );
  assert(scenarios.length > 0, 'report must contain at least one scenario element');
  for (const scenario of scenarios) {
    const steps = scenario.steps || [];
    assert(steps.length > 0, `scenario ${scenario.name} has no steps in the report`);
    for (const step of steps) {
      assert.equal(
        step.result?.status,
        'passed',
        `scenario "${scenario.name}" has a non-passed step in the report`
      );
    }
  }
});

Then('the completion is attested by an economic event naming the grant', async function () {
  const state = states.get(this)!;
  assert(state.status.completion!.actionHash, 'completion must carry a signed action hash');

  const admissionId = `compute-admission:${state.reference}`;
  const response = await fetch(
    new URL(`/api/v1/economic-events/${encodeURIComponent(admissionId)}`, state.config.providerApi)
  );
  assert.equal(response.status, 200, `admission event read returned ${response.status}`);
  const event = (await response.json()) as {
    id: string;
    action: string;
    provider: string;
    receiver: string;
  };
  assert.equal(event.id, admissionId);
  assert.equal(event.action, 'sweettest-feedback');
  // DISCREPANCY (recorded, not fixed — boundary forbids a schema/view change here):
  // the design's §7.2/§10 wording says this event "names the grant" via its bounded_by
  // column equal to the grant commitment cid. The HTTP view actually returned here
  // (elohim/elohim-views/src/shefa.rs:18-49, wired at
  // elohim/elohim-storage/src/views_convert/shefa.rs:71-101, schema
  // elohim/sdk/schemas/v1/views/economic-event-view.schema.json) never projects the
  // `bounded_by` DB column (elohim/elohim-storage/src/db/models.rs:479) into the response
  // — so a boundedBy/grant-cid assertion cannot be made from this route without a view
  // change, which is out of scope (boundary: "no envelope field, no HTTP route ... no
  // schema change"). This step asserts what the route actually returns: the admission id,
  // action and the two parties. It does not verify the bounded_by linkage to the grant.
  const parties = new Set([event.provider, event.receiver]);
  assert(
    parties.has(state.config.requesterKey) && parties.has(state.config.providerKey),
    'admission event must name both the requester and the provider'
  );
});

When('the requester submits the identical stage a second time', { timeout: 300000 }, async function () {
  const state = states.get(this)!;
  const { stdout } = await execute(
    process.execPath,
    [
      workspaceScript,
      'submit',
      join(state.config.stageDir, 'task.json'),
      join(state.config.stageDir, 'stage-runner.sh'),
      join(state.config.stageDir, state.featureFileName),
    ],
    { env: state.env, timeout: 300000, maxBuffer: 1024 * 1024 }
  );
  state.secondSubmitted = JSON.parse(stdout) as Status;
});

Then('the same request is recovered and the provider runs nothing new', async function () {
  const state = states.get(this)!;
  assert.equal(state.secondSubmitted.requestActionHash, state.reference);
  assert.equal(state.secondSubmitted.taskCid, state.taskCid);

  const written = JSON.parse(
    await readFile(join(state.config.stageDir, 'task.json'), 'utf8')
  ) as Envelope;
  assert.equal(written.invocation, state.invocation, 'invocation nonce must not change on plain resubmission');

  const status = await localRead(
    state.config.requesterApi,
    state.config.requesterKey,
    state.env.ELOHIM_COMPUTE_LOCAL_TOKEN ?? '',
    state.reference
  );
  assert.equal(status.completion!.actionHash, state.status.completion!.actionHash);
});

Then('exactly one admission event exists for that stage', async function () {
  const state = states.get(this)!;
  const admissionId = `compute-admission:${state.reference}`;
  const single = await fetch(
    new URL(`/api/v1/economic-events/${encodeURIComponent(admissionId)}`, state.config.providerApi)
  );
  assert.equal(single.status, 200);

  const listed = await fetch(
    new URL(
      `/api/v1/economic-events?action=sweettest-feedback&provider=${encodeURIComponent(state.config.requesterKey)}&limit=500`,
      state.config.providerApi
    )
  );
  assert.equal(listed.status, 200);
  const rows = (await listed.json()) as Array<{ id: string }>;
  const matching = rows.filter(row => row.id === admissionId);
  assert.equal(matching.length, 1, `expected exactly one admission row for ${admissionId}`);
});
