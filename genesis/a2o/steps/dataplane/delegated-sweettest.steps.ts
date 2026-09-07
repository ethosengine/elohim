/** Native compute live probe. Only the workspace's loopback facing is used;
 * Adam must already be running his independently supervised native worker.
 * COMPUTE_A2O_CONFIG names a local JSON fixture (never committed credentials):
 * { task, binary, dna, env, timeoutSeconds?, expiryRequestActionHash?,
 *   unauthorizedTask?, unauthorizedEnv? }.
 */
import { strict as assert } from 'node:assert';
import { execFile } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { promisify } from 'node:util';

import { Given, When, Then } from '@cucumber/cucumber';

const execute = promisify(execFile);
const script = resolve('../agentic/compute/workspace.mjs');
interface Artifact {
  cid: string;
  sha256: string;
  bytes: number;
  chunks?: Artifact[];
}
interface Receipt {
  binary: Artifact;
  dna: Artifact;
  provider: string;
  completedAt: number;
  retention: { maxAgeSeconds: number | null; maxRuns: number | null; expireWhen: string };
  logs: Artifact[];
}
interface Status {
  requestActionHash: string;
  taskCid: string;
  provider: string;
  envelope: { binary: Artifact; dna: Artifact };
  acceptance?: { actionHash: string };
  completion?: { actionHash: string; receipt: Receipt };
  refusal?: { reason: string };
}
interface Fixture {
  task: string;
  binary: string;
  dna: string;
  env: NodeJS.ProcessEnv;
  timeoutSeconds?: number;
  expiryRequestActionHash?: string;
  unauthorizedTask?: string;
  unauthorizedEnv?: NodeJS.ProcessEnv;
}
interface State {
  config: Fixture;
  env: NodeJS.ProcessEnv;
  reference: string;
  status: Status;
  submitted: Status;
  receipt: string;
}
const states = new WeakMap<object, State>();
const pause = async (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

async function localRead(state: State, reference: string): Promise<Status> {
  const base = new URL(state.env.COMPUTE_API_URL ?? 'http://127.0.0.1:8090');
  assert(
    ['localhost', '127.0.0.1', '[::1]'].includes(base.hostname),
    'probe must use local storage'
  );
  const response = await fetch(
    new URL(`/api/v1/compute/tasks/${encodeURIComponent(reference)}`, base),
    {
      headers: {
        'x-elohim-verified-performer': state.env.COMPUTE_PERFORMER ?? '',
        'x-elohim-compute-token': state.env.ELOHIM_COMPUTE_LOCAL_TOKEN ?? '',
      },
      redirect: 'error',
      signal: AbortSignal.timeout(30000),
    }
  );
  assert.equal(response.status, 200, `native task read returned ${response.status}`);
  return response.json() as Promise<Status>;
}

async function waitFor(state: State, predicate: (value: Status) => boolean) {
  const until = Date.now() + (state.config.timeoutSeconds ?? 7200) * 1000;
  while (Date.now() < until) {
    const status = await localRead(state, state.reference);
    if (predicate(status)) {
      state.status = status;
      return;
    }
    await pause(2000);
  }
  assert.fail('native compute deadline expired without the required evidence');
}

Given("a configured native compute workspace and Adam's dedicated test worker", async function () {
  const file = process.env.COMPUTE_A2O_CONFIG;
  if (!file) return 'pending';
  const config = JSON.parse(await readFile(file, 'utf8')) as Fixture;
  assert(
    config.task && config.binary && config.dna && config.env,
    'incomplete native compute fixture'
  );
  states.set(this, { config, env: { ...process.env, ...config.env } } as State);
});

When(
  'the developer submits the pinned feedback sweettest to Adam',
  { timeout: 300000 },
  async function () {
    const state = states.get(this)!;
    const { stdout } = await execute(
      process.execPath,
      [script, 'submit', state.config.task, state.config.binary, state.config.dna],
      {
        env: state.env,
        timeout: 300000,
        maxBuffer: 1024 * 1024,
      }
    );
    state.submitted = JSON.parse(stdout) as Status;
    state.reference = state.submitted.requestActionHash;
    assert(state.reference, 'submission must return its native action reference');
  }
);

When(
  'the submitting client exits while Adam runs the task',
  { timeout: 120000 },
  async function () {
    const state = states.get(this)!;
    const previous = state.config.timeoutSeconds;
    state.config.timeoutSeconds = 90;
    try {
      await waitFor(state, value => Boolean(value.acceptance));
    } finally {
      state.config.timeoutSeconds = previous;
    }
    assert(
      !state.status.completion,
      'fixture must leave enough runtime to observe a free workspace lane'
    );
  }
);

Then('another workspace command completes while Adam is still running the test', async function () {
  const { stdout } = await execute(
    process.execPath,
    ['-e', 'process.stdout.write("lane available")'],
    { timeout: 5000 }
  );
  assert.equal(stdout, 'lane available');
  const state = states.get(this)!;
  const status = await localRead(state, state.reference);
  assert(status.acceptance && !status.completion && !status.refusal);
});

When(
  'the workspace reconnects without its original completion notification',
  { timeout: 7500000 },
  async function () {
    const state = states.get(this)!;
    // This fresh process has no notification cursor or connection from submit.
    await waitFor(state, value => Boolean(value.completion));
    await execute(process.execPath, [script, 'poll'], {
      env: state.env,
      timeout: 1250000,
      maxBuffer: 1024 * 1024,
    });
  }
);

Then("it discovers Adam's signed completion for the submitted artifacts", function () {
  const state = states.get(this)!;
  const { completion } = state.status;
  assert(completion && state.status.acceptance);
  assert(completion.actionHash && state.status.acceptance.actionHash);
  assert.equal(state.status.requestActionHash, state.reference);
  assert.equal(state.status.taskCid, state.submitted.taskCid);
  assert.deepEqual(completion.receipt.binary, state.submitted.envelope.binary);
  assert.deepEqual(completion.receipt.dna, state.submitted.envelope.dna);
  assert.equal(completion.receipt.provider, state.submitted.provider);
});

Then('recovering the receipt starts the selected agent and saves its review', async function () {
  const state = states.get(this)!;
  const { createHash } = await import('node:crypto');
  const name = createHash('sha256').update(state.reference).digest('hex');
  const root = state.env.COMPUTE_INBOX_ROOT ?? resolve('genesis/a2o/reports/compute');
  const entry = JSON.parse(await readFile(resolve(root, 'inbox', `${name}.json`), 'utf8')) as {
    reviews: Record<string, { completion: string; report: string }>;
  };
  const selected = (state.env.COMPUTE_REVIEW_ADAPTERS ?? 'codex').split(',');
  for (const adapter of selected) {
    assert.equal(entry.reviews?.[adapter]?.completion, state.status.completion!.actionHash);
    const review = JSON.parse(await readFile(entry.reviews[adapter].report, 'utf8')) as {
      output: string;
    };
    assert(review.output.length > 0);
  }
});

When(
  'a developer without a matching compute grant submits the pinned feedback sweettest',
  { timeout: 300000 },
  async function () {
    const state = states.get(this)!;
    if (!state.config.unauthorizedTask || !state.config.unauthorizedEnv) return 'pending';
    state.env = { ...state.env, ...state.config.unauthorizedEnv };
    const { stdout } = await execute(
      process.execPath,
      [script, 'submit', state.config.unauthorizedTask, state.config.binary, state.config.dna],
      { env: state.env, timeout: 300000 }
    );
    state.reference = (JSON.parse(stdout) as Status).requestActionHash;
  }
);
Then(
  "the developer receives Adam's refusal because no matching compute grant authorizes the request",
  { timeout: 180000 },
  async function () {
    const state = states.get(this)!;
    state.config.timeoutSeconds = 120;
    await waitFor(state, value => Boolean(value.refusal));
    assert.equal(state.status.refusal!.reason, 'compute-grant-refused');
  }
);
Then('no execution acceptance or completion is recorded for that request', function () {
  const status = states.get(this)!.status;
  assert(!status.acceptance && !status.completion);
});

Given('a completed delegated run with a short configurable payload lifetime', async function () {
  const state = states.get(this)!;
  if (!state.config.expiryRequestActionHash) return 'pending';
  state.reference = state.config.expiryRequestActionHash;
  state.status = await localRead(state, state.reference);
  assert(state.status.completion);
  const policy = state.status.completion.receipt.retention;
  assert(
    policy.maxAgeSeconds && policy.maxAgeSeconds <= 60 && policy.expireWhen === 'either',
    'expiry fixture must expire by age within sixty seconds'
  );
  state.receipt = JSON.stringify(state.status.completion);
});
When(
  'that lifetime expires and the worker cleans retained payloads',
  { timeout: 180000 },
  async function () {
    const state = states.get(this)!;
    // An expiry fixture must have a short lease; never sleep days in a live probe.
    await pause(65000);
    await execute(process.execPath, [script, 'poll'], { env: state.env, timeout: 120000 });
  }
);
Then('the developer is told the full logs have expired', async function () {
  const state = states.get(this)!;
  const log = state.status.completion!.receipt.logs.find((item: Artifact) => item.chunks?.length);
  assert(log, 'expiry fixture must name a previously retrievable payload');
  const response = await fetch(
    new URL(
      `/api/v1/compute/payloads/${log.chunks![0].cid}`,
      state.env.COMPUTE_API_URL ?? 'http://127.0.0.1:8090'
    ),
    {
      headers: {
        'x-elohim-compute-token': state.env.ELOHIM_COMPUTE_LOCAL_TOKEN ?? '',
        'x-compute-owner': state.status.taskCid,
      },
      redirect: 'error',
      signal: AbortSignal.timeout(30000),
    }
  );
  assert.equal(response.status, 410);
});
Then(
  'the original compact receipt identifying the artifacts and outcome remains readable',
  async function () {
    const state = states.get(this)!;
    assert.equal(
      JSON.stringify((await localRead(state, state.reference)).completion),
      state.receipt
    );
  }
);

Given('a reviewer is selected for delegated completions', function () {
  const state = states.get(this)!;
  const adapters = (state.env.COMPUTE_REVIEW_ADAPTERS ?? 'codex').split(',');
  assert(adapters.length > 0 && adapters.every(adapter => ['codex', 'claude'].includes(adapter)));
});

Given('the developer has selected exact test artifacts and a compute grant', function () {
  const state = states.get(this)!;
  assert(state.config.binary && state.config.dna && state.env.COMPUTE_GRANT_ACTION);
});
