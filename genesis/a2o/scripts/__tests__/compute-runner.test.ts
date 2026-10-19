/* eslint-disable @typescript-eslint/no-floating-promises -- node:test owns test promises. */
/* eslint-disable @typescript-eslint/require-await -- the fakes model async seams synchronously. */
import { strict as assert } from 'node:assert';
import { createHash, randomBytes } from 'node:crypto';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { it } from 'node:test';

import { encodeHashToBase64, hashFrom32AndType, HoloHashType } from '@holochain/client';
import { encode } from '@msgpack/msgpack';

import { publishSubmission, submitRequest } from '../../../agentic/compute/submission.mjs';
import { main, parseArgs } from '../compute-runner.js';
import {
  deriveRunnerCredentials,
  grantStatus,
  payloadHeaders,
  payloadRefusal,
  readRunnerEnv,
  RunnerRefusal,
} from '../lib/compute-runner.js';

import type { Conductor, ConductorOptions } from '../lib/steward-conductor.js';

// ---------------------------------------------------------------------------
// Fixtures: one runner, one sponsor doorway, one hosted cell per role.
// ---------------------------------------------------------------------------

const ADMIN_KEY = 'admin-bootstrap-key-for-tests-only-0123456789';
const raw = (type: HoloHashType) => hashFrom32AndType(new Uint8Array(randomBytes(32)), type);
const RUNNER_RAW = raw(HoloHashType.Agent);
const RUNNER = encodeHashToBase64(RUNNER_RAW);
const PROVIDER = encodeHashToBase64(raw(HoloHashType.Agent));
const CONTENT_DNA_RAW = raw(HoloHashType.Dna);
const GOVERNANCE_DNA_RAW = raw(HoloHashType.Dna);
const GRANT_ACTION = encodeHashToBase64(raw(HoloHashType.Action));
const GRANT_ENTRY_RAW = raw(HoloHashType.Entry);
const NOW = Date.parse('2026-10-20T12:00:00Z');
const STDOUT_LOG = 'stdout.log';

function jwt(exp = NOW / 1000 + 86400, agent = RUNNER): string {
  const part = (o: unknown) => Buffer.from(JSON.stringify(o)).toString('base64url');
  return `${part({ alg: 'HS256' })}.${part({ exp, agent_pub_key: agent })}.sig`;
}
const JWT = jwt();

function env(extra: Record<string, string> = {}): NodeJS.ProcessEnv {
  return {
    DOORWAY_URL: 'https://doorway.test',
    RUNNER_JWT: JWT,
    ADMIN_KEY,
    COMPUTE_EXECUTOR: '/pinned/compute-executor',
    RUNNER_ROOT: mkdtempSync(join(tmpdir(), 'runner-root-')),
    ...extra,
  };
}

/** The pinned executor's two verbs, reproduced over sha256 so a test needs no binary. */
const executorCalls: string[][] = [];
async function execute(program: string, args: string[]): Promise<string> {
  executorCalls.push([program, ...args]);
  const bytes = readFileSync(args[1]);
  const sha = createHash('sha256').update(bytes).digest('hex');
  if (args[0] === 'artifact')
    return JSON.stringify({ cid: `bafkrei${sha.slice(0, 20)}`, sha256: sha, bytes: bytes.length });
  if (args[0] === 'cid') return `bafyreitask${sha.slice(0, 20)}\n`;
  throw new Error(`unknown executor verb ${args[0]}`);
}

interface Recorded {
  connects: { reuseCredentials: boolean; signingKey: string; capSecret: string }[];
  grants: Set<string>;
  payloads: { method: string; url: string; headers: Record<string, string>; body?: Buffer }[];
  zome: { role: string; zome: string; fn: string; payload: unknown }[];
  stored: Map<string, Buffer>;
}

/** A doorway: /hc/connect (granting only when reuseCredentials is false) and the
 * sponsor's payload custody. */
function fakeDoorway(rec: Recorded, payloadStatus?: { status: number; body: string }) {
  return (async (input: URL | string, init?: RequestInit) => {
    const url = new URL(String(input));
    const headers = Object.fromEntries(
      Object.entries((init?.headers ?? {}) as Record<string, string>).map(([k, v]) => [
        k.toLowerCase(),
        v,
      ])
    );
    if (url.pathname === '/hc/connect') {
      const body = JSON.parse(String(init?.body)) as Recorded['connects'][number];
      rec.connects.push(body);
      if (!body.reuseCredentials) rec.grants.add(body.signingKey);
      return new Response(
        JSON.stringify({
          appToken: Buffer.from('app-token').toString('base64'),
          appPort: 4445,
          agentPubKey: Buffer.from(RUNNER_RAW).toString('base64'),
          installedAppId: 'elohim',
          conductorId: 'conductor-0',
          cellIds: {
            lamad: [
              Buffer.from(CONTENT_DNA_RAW).toString('base64'),
              Buffer.from(RUNNER_RAW).toString('base64'),
            ],
            mishpat: [
              Buffer.from(GOVERNANCE_DNA_RAW).toString('base64'),
              Buffer.from(RUNNER_RAW).toString('base64'),
            ],
          },
          token: jwt(),
        }),
        { status: 200 }
      );
    }
    if (url.pathname.startsWith('/api/v1/compute/payloads/')) {
      const method = String(init?.method ?? 'GET');
      const body = init?.body ? Buffer.from(init.body as Buffer) : undefined;
      rec.payloads.push({ method, url: url.href, headers, body });
      if (payloadStatus) return new Response(payloadStatus.body, { status: payloadStatus.status });
      const cid = decodeURIComponent(url.pathname.split('/').pop()!);
      if (method === 'PUT') {
        rec.stored.set(cid, body!);
        return new Response(body, { status: 200 });
      }
      const stored = rec.stored.get(cid);
      return stored ? new Response(stored, { status: 200 }) : new Response('', { status: 503 });
    }
    return new Response('not found', { status: 404 });
  }) as typeof fetch;
}

/** The runner's hosted cells. A call is refused unless the doorway granted the runner's
 * signing key; the receipt the session wrote must be unexpired. */
function fakeConnect(
  rec: Recorded,
  cells: {
    task?: Record<string, unknown>;
    record?: unknown;
    links?: unknown[];
  } = {}
) {
  return async (o: ConductorOptions): Promise<Conductor> => {
    const receipt = JSON.parse(readFileSync(o.hosted!.receipt, 'utf8')) as {
      expiresAt: number;
      agentPubKey: string;
    };
    assert.equal(o.expectedAgent, RUNNER);
    assert.equal(receipt.agentPubKey, RUNNER);
    assert.ok(existsSync(o.signingCredentialsDir!), 'signing profile written for the cell');
    const signingKey = Buffer.from(deriveRunnerCredentials(ADMIN_KEY).signingAgentKey).toString(
      'base64'
    );
    return {
      agent: RUNNER,
      dna: '',
      async call<T>(fn: string, payload: unknown): Promise<T> {
        if (!rec.grants.has(signingKey)) throw new Error('Unauthorized zome call');
        rec.zome.push({ role: o.role, zome: o.zome!, fn, payload });
        if (fn === 'compute_task') {
          const op = (payload as { operation: string }).operation;
          if (op === 'list') return { tasks: [], nextOffset: null } as T;
          if (op === 'submit')
            return { requestActionHash: 'uhCkkRequest', state: 'requested', ...cells.task } as T;
          if (op === 'get') return (cells.task ?? { state: 'requested' }) as T;
        }
        if (fn === 'get_commitment_record') return (cells.record ?? null) as T;
        if (fn === 'get_commitment_authority_links') return (cells.links ?? []) as T;
        throw new Error(`unexpected ${fn}`);
      },
      async close() {},
    };
  };
}

function recorder(): Recorded {
  return { connects: [], grants: new Set(), payloads: [], zome: [], stored: new Map() };
}

/** A stage directory: task.json naming the runner as requester, plus its two inputs. */
async function stage(requester = RUNNER) {
  const dir = mkdtempSync(join(tmpdir(), 'runner-stage-'));
  const binary = join(dir, 'stage-runner.sh');
  const dna = join(dir, 'x.feature');
  writeFileSync(binary, '#!/bin/sh\necho stage\n');
  writeFileSync(dna, 'Feature: x\n');
  const descriptor = async (path: string) => JSON.parse(await execute('x', ['artifact', path]));
  const envelope = {
    schemaVersion: 1,
    taskKind: 'feedback_signal',
    project: 'a2o-stage:x',
    requester,
    provider: PROVIDER,
    invocation: 'pinned-invocation',
    binary: await descriptor(binary),
    dna: await descriptor(dna),
    retention: { maxAgeSeconds: 86400, maxRuns: 5, expireWhen: 'either' },
  };
  const task = join(dir, 'task.json');
  writeFileSync(task, JSON.stringify(envelope));
  return { dir, task, binary, dna, envelope };
}

function capture() {
  const out: string[] = [];
  return {
    out,
    stdout: (text: string) => {
      out.push(text);
    },
  };
}

// ---------------------------------------------------------------------------
// Task shape parity with workspace.mjs
// ---------------------------------------------------------------------------

it('submits the exact task shape workspace.mjs submits (shared submission.mjs)', async () => {
  const rec = recorder();
  const s = await stage();
  const { out, stdout } = capture();
  const code = await main(['submit', s.task, s.binary, s.dna, '--grant', GRANT_ACTION], {
    env: env(),
    fetcher: fakeDoorway(rec),
    connect: fakeConnect(rec),
    execute,
    now: () => NOW,
    stdout,
  });
  assert.equal(code, 0);
  const submit = rec.zome.find(z => (z.payload as { operation: string }).operation === 'submit');
  assert.ok(submit, 'the task was submitted to the runner own content cell');
  assert.equal(submit.role, 'lamad');
  assert.equal(submit.zome, 'content_store');
  assert.equal(submit.fn, 'compute_task');

  // The household requester's path, over the same envelope and a recording payload.
  const household = await stage();
  const householdPuts: { cid: string; owner?: string }[] = [];
  const { envelope, taskCid } = await publishSubmission({
    envelope: household.envelope,
    inputPaths: [household.binary, household.dna],
    root: mkdtempSync(join(tmpdir(), 'household-root-')),
    executor: '/pinned/compute-executor',
    payload: async (cid, _method, _body, _retention, owner) => {
      householdPuts.push({ cid, owner });
      return undefined;
    },
    execute,
  });
  const expected = {
    ...submitRequest({ envelope, taskCid, grantActionHash: GRANT_ACTION }),
    operation: 'submit',
  };
  assert.deepEqual(submit.payload, expected, 'identical submit input to the native extern');
  assert.deepEqual(
    rec.payloads.map(p => decodeURIComponent(p.url.split('/').pop()!)),
    householdPuts.map(p => p.cid),
    'the same chunks are stocked'
  );
  assert.ok(out.join('').includes('uhCkkRequest'));
});

// ---------------------------------------------------------------------------
// Headers on each payload call
// ---------------------------------------------------------------------------

it('stocks inputs with bearer + owner and draws outputs naming the request', async () => {
  const rec = recorder();
  const s = await stage();
  await main(['submit', s.task, s.binary, s.dna, '--grant', GRANT_ACTION], {
    env: env(),
    fetcher: fakeDoorway(rec),
    connect: fakeConnect(rec),
    execute,
    now: () => NOW,
    stdout: () => {},
  });
  const puts = rec.payloads.filter(p => p.method === 'PUT');
  assert.ok(puts.length >= 2);
  const taskCid = (
    rec.zome.find(z => (z.payload as { operation: string }).operation === 'submit')!.payload as {
      taskCid: string;
    }
  ).taskCid;
  for (const put of puts) {
    assert.equal(put.headers.authorization, `Bearer ${JWT}`);
    assert.equal(put.headers['x-compute-owner'], taskCid);
    assert.equal(put.headers['x-compute-retain-seconds'], '0');
    assert.equal(put.headers['x-compute-request'], undefined, 'no request header on a stock');
    assert.equal(put.headers['x-elohim-compute-token'], undefined, 'never the node-local token');
  }

  // Poll a completed task and draw its output back.
  const outChunk = Buffer.from('stage stdout\n');
  const sha = createHash('sha256').update(outChunk).digest('hex');
  rec.stored.set('bafkreiout', outChunk);
  const completed = {
    state: 'completed',
    taskCid,
    requester: RUNNER,
    envelope: s.envelope,
    completion: {
      receipt: {
        logs: [
          {
            name: STDOUT_LOG,
            sha256: sha,
            bytes: outChunk.length,
            chunks: [{ cid: 'bafkreiout', sha256: sha, bytes: outChunk.length }],
          },
        ],
      },
    },
  };
  const outDir = mkdtempSync(join(tmpdir(), 'runner-out-'));
  const { out, stdout } = capture();
  await main(['poll', 'uhCkkRequest', '--out', outDir], {
    env: env(),
    fetcher: fakeDoorway(rec),
    connect: fakeConnect(rec, { task: completed }),
    now: () => NOW,
    stdout,
  });
  const get = rec.payloads.find(p => p.method === 'GET')!;
  assert.equal(get.headers['x-compute-request'], 'uhCkkRequest');
  assert.equal(get.headers['x-compute-owner'], taskCid);
  assert.equal(get.headers.authorization, `Bearer ${JWT}`);
  assert.equal(readFileSync(join(outDir, STDOUT_LOG), 'utf8'), 'stage stdout\n');
  assert.ok(out.join('').includes(STDOUT_LOG));
  rmSync(outDir, { recursive: true, force: true });
});

it('payloadHeaders sends exactly the custody headers', () => {
  assert.deepEqual(
    payloadHeaders({ jwt: 'J', owner: 'bafyT', retention: 86400, request: 'uhCkkR' }),
    {
      authorization: 'Bearer J',
      'content-type': 'application/octet-stream',
      'x-compute-owner': 'bafyT',
      'x-compute-retain-seconds': '86400',
      'x-compute-request': 'uhCkkR',
    }
  );
});

// ---------------------------------------------------------------------------
// Grant authored once; token refreshed, not failed
// ---------------------------------------------------------------------------

it('authors the listed grant once, and a later build only refreshes transport', async () => {
  const rec = recorder();
  const deps = {
    env: env(),
    fetcher: fakeDoorway(rec),
    connect: fakeConnect(rec),
    now: () => NOW,
    stdout: () => {},
  };
  await main(['get', 'uhCkkRequest'], deps);
  assert.deepEqual(
    rec.connects.map(c => c.reuseCredentials),
    [true, false],
    'probe refused under reuse, then ONE connect that lets the doorway grant'
  );
  await main(['get', 'uhCkkRequest'], deps);
  assert.deepEqual(
    rec.connects.map(c => c.reuseCredentials),
    [true, false, true],
    'the next build reuses the granted credentials: no second grant'
  );
  const signing = new Set(rec.connects.map(c => c.signingKey));
  const secrets = new Set(rec.connects.map(c => c.capSecret));
  assert.equal(signing.size, 1, 'one stable signing key across builds');
  assert.equal(secrets.size, 1, 'one stable cap secret across builds');
});

it('refreshes the hosted app token before it lapses instead of failing', async () => {
  const rec = recorder();
  let now = NOW;
  const { RunnerSession } = await import('../lib/compute-runner.js');
  const session = new RunnerSession({
    env: readRunnerEnv(env()),
    fetcher: fakeDoorway(rec),
    connect: fakeConnect(rec),
    now: () => now,
  });
  try {
    await session.call('lamad', 'content_store', 'compute_task', { operation: 'get' });
    const before = rec.connects.length;
    now += 3_600_000; // the app token's lifetime
    await session.call('lamad', 'content_store', 'compute_task', { operation: 'get' });
    assert.equal(rec.connects.length, before + 1, 'one refresh');
    assert.equal(rec.connects.at(-1)!.reuseCredentials, true, 'refresh never re-grants');
  } finally {
    await session.close();
  }
});

// ---------------------------------------------------------------------------
// Refusals name their cure
// ---------------------------------------------------------------------------

it('refusals name their cure', async () => {
  assert.throws(
    () => readRunnerEnv({ DOORWAY_URL: 'https://d.test', ADMIN_KEY }),
    (e: unknown) => e instanceof RunnerRefusal && e.message.includes('runner-ensure.sh')
  );
  assert.throws(
    () => readRunnerEnv({ DOORWAY_URL: 'https://d.test', RUNNER_JWT: JWT }),
    (e: unknown) => e instanceof RunnerRefusal && e.cure.includes('ADMIN_KEY')
  );
  const wrong = await stage(PROVIDER);
  const rec = recorder();
  await assert.rejects(
    main(['submit', wrong.task, wrong.binary, wrong.dna, '--grant', GRANT_ACTION], {
      env: env(),
      fetcher: fakeDoorway(rec),
      connect: fakeConnect(rec),
      execute,
      now: () => NOW,
      stdout: () => {},
    }),
    (e: unknown) =>
      e instanceof RunnerRefusal &&
      e.message.startsWith('REFUSED submit:') &&
      e.cure.includes(`--requester ${RUNNER}`)
  );
  await assert.rejects(
    main(['submit', wrong.task, wrong.binary, wrong.dna], {
      env: env(),
      fetcher: fakeDoorway(rec),
      connect: fakeConnect(rec),
      execute,
      now: () => NOW,
      stdout: () => {},
    }),
    (e: unknown) => e instanceof RunnerRefusal && e.cure.includes('--grant')
  );
  for (const [status, body, cure] of [
    [403, '{"reason":"no-active-hosted-cell-grant"}', /re-issue the runner hosted-cell grant/],
    [403, '{"reason":"rate-limit-exceeded"}', /hour window/],
    [403, '{"reason":"local-compute-capability-required"}', /session agent key/],
    [401, '', /runner-ensure\.sh/],
    [404, '', /hosted custody/],
  ] as const) {
    const refusal = payloadRefusal(status, body);
    assert.ok(cure.test(refusal.cure), `${status} ${body}: ${refusal.message}`);
    assert.ok(refusal.message.startsWith('REFUSED payload:'));
  }
  const expired = readRunnerEnv(env({ RUNNER_JWT: jwt(NOW / 1000 - 1) }));
  const { RunnerSession } = await import('../lib/compute-runner.js');
  assert.throws(
    () => new RunnerSession({ env: expired, now: () => NOW }),
    (e: unknown) => e instanceof RunnerRefusal && e.cure.includes('runner-ensure.sh')
  );
});

it('a verb never takes a secret on its command line', () => {
  for (const flag of ['--jwt', '--admin-key', '--token', '--cap-secret']) {
    assert.throws(
      () => parseArgs(['get', 'uhCkkR', flag, 'x']),
      (e: unknown) => e instanceof RunnerRefusal && e.cure.includes('environment only')
    );
  }
});

// ---------------------------------------------------------------------------
// No secret in argv or stdout
// ---------------------------------------------------------------------------

it('no secret ever reaches argv or stdout', async () => {
  executorCalls.length = 0;
  const rec = recorder();
  const s = await stage();
  const { out, stdout } = capture();
  await main(['submit', s.task, s.binary, s.dna, '--grant', GRANT_ACTION], {
    env: env(),
    fetcher: fakeDoorway(rec),
    connect: fakeConnect(rec),
    execute,
    now: () => NOW,
    stdout,
  });
  const credentials = deriveRunnerCredentials(ADMIN_KEY);
  const secrets = [
    ADMIN_KEY,
    JWT,
    Buffer.from(credentials.seed).toString('hex'),
    Buffer.from(credentials.capSecret).toString('hex'),
    Buffer.from(credentials.capSecret).toString('base64'),
  ];
  const printed = out.join('');
  const argv = executorCalls.flat().join(' ');
  for (const secret of secrets) {
    assert.ok(!printed.includes(secret), 'a secret reached stdout');
    assert.ok(!argv.includes(secret), 'a secret reached a child argv');
  }
});

// ---------------------------------------------------------------------------
// Grant status
// ---------------------------------------------------------------------------

function grantRecord(policy: Record<string, unknown>, author = PROVIDER) {
  return {
    signed_action: {
      hashed: {
        content: {
          author: Buffer.from(author.slice(1), 'base64url'),
          entry_hash: GRANT_ENTRY_RAW,
        },
      },
    },
    entry: {
      Present: {
        entry_type: 'App',
        entry: encode({
          action: 'delegates-compute',
          payload_json: JSON.stringify(policy),
          signed_at: '2026-10-01T00:00:00Z',
        }),
      },
    },
  };
}

const POLICY = {
  scope: 'measure-stage',
  provider: PROVIDER,
  recipient: RUNNER,
  valid_from: '2026-10-01T00:00:00+00:00',
  valid_until: '2026-10-31T00:00:00+00:00',
};
const ACTIVE = { author: PROVIDER, state: 'active', signed_at: '2026-10-01T00:00:00Z' };

it('grant-status reads the signed record and links through the runner own cell', async () => {
  const rec = recorder();
  const { out, stdout } = capture();
  await main(['grant-status', GRANT_ACTION], {
    env: env(),
    fetcher: fakeDoorway(rec),
    connect: fakeConnect(rec, { record: grantRecord(POLICY), links: [ACTIVE] }),
    now: () => NOW,
    stdout,
  });
  const status = JSON.parse(out.join(''));
  assert.equal(status.state, 'active');
  assert.equal(status.runner, RUNNER);
  assert.equal(status.forThisRunner, true, 'the grant names this runner as recipient');
  assert.equal(status.provider, PROVIDER);
  assert.equal(status.daysRemaining, 10);
  assert.equal(status.cid, encodeHashToBase64(GRANT_ENTRY_RAW));
  const reads = rec.zome.filter(z => z.role === 'mishpat');
  assert.deepEqual(
    reads.map(z => z.fn),
    ['get_commitment_record', 'get_commitment_authority_links']
  );
});

it('grantStatus reads withdrawn, lapsed, inactive and unverified honestly', () => {
  const base = {
    grantActionHash: GRANT_ACTION,
    cid: 'uhCEk',
    author: PROVIDER,
    policy: POLICY,
    now: NOW,
  };
  assert.equal(grantStatus({ ...base, links: [ACTIVE] }).state, 'active');
  assert.equal(
    grantStatus({ ...base, links: [ACTIVE, { ...ACTIVE, state: 'revoked' }] }).state,
    'withdrawn'
  );
  assert.equal(
    grantStatus({
      ...base,
      links: [ACTIVE, { ...ACTIVE, author: RUNNER, state: 'revoked' }],
    }).state,
    'active',
    'a withdrawal authored by anyone but the provider is not one'
  );
  assert.equal(grantStatus({ ...base, links: [] }).state, 'inactive');
  assert.equal(
    grantStatus({ ...base, links: [ACTIVE], now: Date.parse('2026-11-01T00:00:00Z') }).state,
    'lapsed'
  );
  assert.equal(grantStatus({ ...base, author: RUNNER, links: [ACTIVE] }).state, 'unverified');
});
