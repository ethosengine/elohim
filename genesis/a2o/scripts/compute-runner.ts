/**
 * compute-runner — the hosted CI runner's command line (plan "Jenkins as a stewarded
 * runner", slice B2). Every verb prints ONE JSON document on stdout; refusals go to stderr
 * as `REFUSED <name>: <why> — <cure>` with exit 1.
 *
 *   compute-runner submit TASK.json BINARY DNA --grant <provider grant action hash>
 *                         [--provider <agent key>] [--new-run]
 *   compute-runner get <request action hash>
 *   compute-runner poll <request action hash> [--timeout-seconds N] [--interval-seconds N]
 *                         [--out DIR]
 *   compute-runner grant-status <grant action hash>
 *
 * Configuration and every secret come from the environment ONLY (see lib/compute-runner.ts
 * readRunnerEnv): DOORWAY_URL, RUNNER_JWT (from genesis/scripts/ci/runner-ensure.sh),
 * ADMIN_KEY (derives the stable signing key and cap secret), COMPUTE_EXECUTOR (the pinned
 * rakia executor, for chunk and task CIDs), RUNNER_ROOT, RUNNER_APP_ID. No verb accepts a
 * secret on its command line.
 *
 * Why an a2o script and not genesis/agentic/compute/runner.mjs: the hosted websocket client
 * it reuses (lib/steward-conductor.ts connectConductor, lib/steward-credential.ts) is
 * TypeScript in this package, and @holochain/client is this package's dependency. The task
 * shape and chunking are still the compute code's own (submission.mjs, payloads.mjs).
 */
import { mkdir, readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { materialize, leaseSeconds } from '../../agentic/compute/payloads.mjs';
import { publishSubmission, submitRequest } from '../../agentic/compute/submission.mjs';

import {
  checkEnvelope,
  CONTENT_ROLE,
  CONTENT_ZOME,
  doorwayPayload,
  isTerminal,
  readGrantStatus,
  readRunnerEnv,
  RunnerRefusal,
  RunnerSession,
  type RunnerEnv,
} from './lib/compute-runner.js';

import type { ConductorOptions, Conductor } from './lib/steward-conductor.js';

export interface RunnerDeps {
  env?: NodeJS.ProcessEnv;
  fetcher?: typeof fetch;
  connect?: (o: ConductorOptions) => Promise<Conductor>;
  execute?: (program: string, args: string[]) => Promise<string>;
  now?: () => number;
  sleep?: (ms: number) => Promise<void>;
  stdout?: (text: string) => void;
}

/** Flags a verb accepts. Anything else — including any secret-shaped flag — is refused. */
const FLAGS: Record<string, string[]> = {
  submit: ['--grant', '--provider', '--new-run'],
  get: [],
  poll: ['--timeout-seconds', '--interval-seconds', '--out'],
  'grant-status': [],
};
const BOOLEAN_FLAGS = new Set(['--new-run']);

export function parseArgs(argv: string[]): {
  verb: string;
  positional: string[];
  flags: Map<string, string>;
} {
  const [verb, ...rest] = argv;
  if (!verb || !(verb in FLAGS))
    throw new RunnerRefusal(
      'usage',
      `unknown verb ${verb ?? '(none)'}`,
      'use submit | get | poll | grant-status'
    );
  const positional: string[] = [];
  const flags = new Map<string, string>();
  for (let i = 0; i < rest.length; i++) {
    const arg = rest[i];
    if (!arg.startsWith('--')) {
      positional.push(arg);
      continue;
    }
    if (!FLAGS[verb].includes(arg))
      throw new RunnerRefusal(
        'usage',
        `${verb} does not take ${arg}`,
        'secrets come from the environment only (DOORWAY_URL, RUNNER_JWT, ADMIN_KEY)'
      );
    if (BOOLEAN_FLAGS.has(arg)) flags.set(arg, 'true');
    else {
      if (i + 1 >= rest.length)
        throw new RunnerRefusal('usage', `${arg} needs a value`, `pass ${arg} <value>`);
      flags.set(arg, rest[++i]);
    }
  }
  return { verb, positional, flags };
}

function requireExecutor(env: RunnerEnv): string {
  if (!env.executor)
    throw new RunnerRefusal(
      'executor',
      'COMPUTE_EXECUTOR is unset',
      'export COMPUTE_EXECUTOR=<path to the pinned rakia compute-executor>'
    );
  return env.executor;
}

async function getTask(session: RunnerSession, request: string) {
  return session.call<Record<string, unknown>>(CONTENT_ROLE, CONTENT_ZOME, 'compute_task', {
    operation: 'get',
    requestActionHash: request,
  });
}

export async function main(argv: string[], deps: RunnerDeps = {}): Promise<number> {
  const stdout = deps.stdout ?? ((text: string) => process.stdout.write(text));
  const now = deps.now ?? Date.now;
  const { verb, positional, flags } = parseArgs(argv);
  const env = readRunnerEnv(deps.env ?? process.env);
  const session = new RunnerSession({ env, fetcher: deps.fetcher, connect: deps.connect, now });
  try {
    if (verb === 'grant-status') {
      const [grant] = positional;
      if (!grant)
        throw new RunnerRefusal('usage', 'grant-status needs a grant action hash', 'pass it');
      const status = await readGrantStatus(session, grant, now());
      // Name the runner too, so a reader can see the grant is THIS runner's to spend.
      const runner = await session.whoami();
      stdout(
        `${JSON.stringify({ ...status, runner, forThisRunner: status.recipient === runner })}\n`
      );
      return 0;
    }
    if (verb === 'get') {
      const [request] = positional;
      if (!request) throw new RunnerRefusal('usage', 'get needs a request action hash', 'pass it');
      stdout(`${JSON.stringify(await getTask(session, request))}\n`);
      return 0;
    }
    if (verb === 'poll') {
      const [request] = positional;
      if (!request) throw new RunnerRefusal('usage', 'poll needs a request action hash', 'pass it');
      const timeoutMs = Number(flags.get('--timeout-seconds') ?? 3600) * 1000;
      const intervalMs = Number(flags.get('--interval-seconds') ?? 15) * 1000;
      const sleep = deps.sleep ?? (async (ms: number) => new Promise(r => setTimeout(r, ms)));
      const deadline = now() + timeoutMs;
      let task = await getTask(session, request);
      while (!isTerminal(task)) {
        if (now() >= deadline)
          throw new RunnerRefusal(
            'poll',
            `task ${request} not terminal within ${timeoutMs / 1000}s (state ${String(task.state)})`,
            'raise --timeout-seconds, or check the provider worker is running'
          );
        await sleep(intervalMs);
        task = await getTask(session, request);
      }
      const logPaths: string[] = [];
      const out = flags.get('--out');
      const receipt = (task.completion as { receipt?: { logs?: unknown[] } } | null)?.receipt;
      if (out && receipt?.logs?.length) {
        await mkdir(resolve(out), { recursive: true, mode: 0o700 });
        const payload = doorwayPayload({
          doorway: env.doorway,
          jwt: env.jwt,
          request,
          fetcher: deps.fetcher,
        });
        const retention = leaseSeconds(
          (task.envelope as { retention?: { maxAgeSeconds?: number } } | undefined)?.retention
        );
        for (const log of receipt.logs as { name: string }[]) {
          if (!['stdout.log', 'stderr.log', 'witness.json'].includes(log.name))
            throw new RunnerRefusal('poll', 'the receipt names an unknown log', 'inspect the task');
          const path = join(resolve(out), log.name);
          await materialize(
            log as Parameters<typeof materialize>[0],
            path,
            payload,
            String(task.taskCid),
            retention
          );
          logPaths.push(path);
        }
      }
      stdout(`${JSON.stringify({ task, logPaths })}\n`);
      return 0;
    }
    // submit
    const [file, binary, dna] = positional;
    if (!file || !binary || !dna)
      throw new RunnerRefusal(
        'usage',
        'submit needs TASK.json BINARY DNA',
        'pass the three stage files'
      );
    const grant = flags.get('--grant');
    if (!grant)
      throw new RunnerRefusal(
        'submit',
        'no provider grant named',
        "pass --grant <the provider's measure-stage grant action hash for this runner>"
      );
    const executor = requireExecutor(env);
    const envelope = JSON.parse(await readFile(file, 'utf8')) as Record<string, unknown>;
    // Connect first: the runner learns its own agent key from its hosted cell.
    checkEnvelope(envelope, await session.whoami(), flags.get('--provider'));
    const { taskCid } = await publishSubmission({
      envelope,
      inputPaths: [binary, dna],
      file,
      root: resolve(env.root),
      executor,
      payload: doorwayPayload({ doorway: env.doorway, jwt: env.jwt, fetcher: deps.fetcher }),
      execute: deps.execute,
      newRun: flags.has('--new-run'),
    });
    const status = await session.call<Record<string, unknown>>(
      CONTENT_ROLE,
      CONTENT_ZOME,
      'compute_task',
      { ...submitRequest({ envelope, taskCid, grantActionHash: grant }), operation: 'submit' }
    );
    stdout(`${JSON.stringify(status)}\n`);
    return 0;
  } finally {
    await session.close();
  }
}

/** The one stderr line a failed run prints. A RunnerRefusal already reads
 * `REFUSED <name>: <why> — <cure>` and carries no secret; anything else is reported by
 * its message only, never with the environment. */
export function refusalLine(error: unknown): string {
  if (error instanceof RunnerRefusal) return error.message;
  const message = error instanceof Error ? error.message : String(error);
  return `REFUSED runner: ${message} — inspect the doorway and conductor`;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).then(
    code => {
      process.exitCode = code;
    },
    error => {
      // A RunnerRefusal already reads REFUSED <name>: <why> — <cure> and carries no secret.
      // Anything else is reported by its message only, never with the environment.
      process.stderr.write(`${refusalLine(error)}\n`);
      process.exitCode = 1;
    }
  );
}
