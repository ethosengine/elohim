#!/usr/bin/env node
// Detached, decision-free sampling of one reused gate verdict.

import { closeSync, mkdirSync, openSync, writeSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

function oneLine(value, flag) {
  const text = String(value ?? '').trim();
  if (!text || /[\r\n]/.test(text)) throw new Error(`${flag} needs nonempty one-line text`);
  return text;
}

export function parseArgs(argv) {
  const values = {};
  for (let index = 0; index < argv.length; index += 2) {
    const flag = argv[index];
    if (!['--project', '--commit', '--closure', '--from-device', '--root'].includes(flag)) {
      throw new Error(`unknown argument ${flag || '(empty)'}`);
    }
    if (index + 1 >= argv.length) throw new Error(`${flag} needs a value`);
    values[flag.slice(2).replace('-', '_')] = oneLine(argv[index + 1], flag);
  }
  for (const flag of ['project', 'commit', 'closure', 'root']) {
    if (!values[flag]) throw new Error(`--${flag} is required`);
  }
  return values;
}

function safePart(value) {
  return value.replace(/[^A-Za-z0-9._-]+/g, '-').replace(/^-+|-+$/g, '') || 'sample';
}

function secondsText(milliseconds) {
  const seconds = Math.max(0, milliseconds / 1000);
  return Number.isInteger(seconds) ? String(seconds) : String(Number(seconds.toFixed(3)));
}

function statusOf(result) {
  return result?.status ?? result?.signal ?? result?.error?.message ?? 'not-started';
}

export async function runEnrichSample(options, {
  env = process.env,
  spawn = spawnSync,
  now = () => Date.now(),
  sleep = milliseconds => new Promise(resolveSleep => setTimeout(resolveSleep, milliseconds)),
} = {}) {
  const root = resolve(options.root);
  const project = oneLine(options.project, '--project');
  const commit = oneLine(options.commit, '--commit');
  const closure = oneLine(options.closure, '--closure');
  const fromDevice = options.from_device ? oneLine(options.from_device, '--from-device') : null;
  const commit12 = safePart(commit.slice(0, 12));
  const projectPart = safePart(project);
  const samplePath = join(root, '.worktrees', `enrich-sample-${commit12}-${projectPart}`);
  const logPath = join(root, '.eprfs', 'status', 'enrich-sample', `${commit12}-${projectPart}.log`);
  mkdirSync(dirname(logPath), { recursive: true });
  const log = openSync(logPath, 'a', 0o600);
  const line = text => writeSync(log, `${new Date().toISOString()} ${text}\n`);
  const run = (command, args, runOptions = {}) => {
    line(`$ ${command} ${args.join(' ')}`);
    const result = spawn(command, args, {
      cwd: runOptions.cwd || root,
      env: runOptions.env || env,
      stdio: runOptions.stdio || ['ignore', log, log],
      ...runOptions,
    });
    line(`exit ${statusOf(result)}`);
    return result;
  };
  let worktreeAdded = false;
  try {
    line(`sample project=${project} commit=${commit} closure=${closure}${fromDevice ? ` from-device=${fromDevice}` : ''}`);
    while (true) {
      const cargo = run('pgrep', ['-x', 'cargo'], { stdio: 'ignore' });
      const rustc = run('pgrep', ['-x', 'rustc'], { stdio: 'ignore' });
      if (cargo.status !== 0 && rustc.status !== 0) break;
      line('build processes still active; waiting 1000 ms');
      await sleep(1000);
    }

    const added = run('git', ['-C', root, 'worktree', 'add', '--detach', samplePath, commit]);
    if (added.status !== 0) throw new Error(`worktree add failed (${statusOf(added)})`);
    worktreeAdded = true;

    const submodules = run('git', [
      '-C', samplePath, 'submodule', 'update', '--init', '--recursive', '--',
      'elohim/brit', 'elohim/rakia', 'sophia',
    ]);
    if (submodules.status !== 0) throw new Error(`submodule prep failed (${statusOf(submodules)})`);
    const installed = run('pnpm', ['install', '--frozen-lockfile'], { cwd: samplePath });
    if (installed.status !== 0) throw new Error(`pnpm install failed (${statusOf(installed)})`);

    const gateStarted = now();
    const gate = run('just', ['gate', project], {
      cwd: samplePath,
      env: {
        ...env,
        GATE_EVIDENCE: 'off',
        GATE_ENRICH_SAMPLE: 'off',
        GATE_READING: 'off',
        GATE_TASK: 'off',
      },
    });
    const gateSeconds = secondsText(now() - gateStarted);
    const outcome = gate.status === 0 ? 'green' : 'red';

    const justOid = run('git', ['-C', root, 'rev-parse', `${commit}:justfile`], {
      stdio: ['ignore', 'pipe', log], encoding: 'utf8',
    });
    if (justOid.status !== 0) throw new Error(`justfile method lookup failed (${statusOf(justOid)})`);
    const runnerOid = run('git', ['-C', root, 'rev-parse', `${commit}:genesis/orchestrator/gate-runner.mjs`], {
      stdio: ['ignore', 'pipe', log], encoding: 'utf8',
    });
    if (runnerOid.status !== 0) throw new Error(`gate-runner method lookup failed (${statusOf(runnerOid)})`);
    const method = `justfile:${String(justOid.stdout).trim()},gate-runner:${String(runnerOid.stdout).trim()}`;
    const enrichArgs = [
      'flow', 'enrich', '--project', project, '--closure', closure,
      ...(fromDevice ? ['--from-device', fromDevice] : []),
      '--kind', 're-executed', '--outcome', outcome, '--method', method,
      '--seconds', gateSeconds, '--json', '--root', root,
    ];
    const enriched = run(env.EPR_BIN || 'epr', enrichArgs, { cwd: root });
    if (enriched.status !== 0) line(`enrichment refused (${statusOf(enriched)}); the gate decision remains unchanged`);
    else line(`enrichment appended outcome=${outcome} seconds=${gateSeconds}`);
    return { outcome, seconds: gateSeconds, enriched: enriched.status === 0, logPath, samplePath };
  } catch (error) {
    line(`sample stopped: ${String(error?.message || error).replace(/[\r\n]+/g, ' ')}`);
    return { outcome: null, enriched: false, logPath, samplePath };
  } finally {
    if (worktreeAdded) {
      try {
        const removed = run('git', ['-C', root, 'worktree', 'remove', '--force', samplePath]);
        if (removed.status !== 0) line(`worktree removal failed (${statusOf(removed)})`);
      } catch (error) {
        line(`worktree removal failed (${String(error?.message || error).replace(/[\r\n]+/g, ' ')})`);
      }
    }
    closeSync(log);
  }
}

export async function main(argv = process.argv.slice(2), dependencies = {}) {
  try {
    const options = parseArgs(argv);
    await runEnrichSample(options, dependencies);
  } catch (error) {
    // A malformed detached request has no trustworthy root for a log. It still cannot affect a gate.
    process.stderr.write(`enrich-sample: ${String(error?.message || error).replace(/[\r\n]+/g, ' ')}\n`);
  }
  return 0;
}

const isMain = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  await main();
  process.exitCode = 0;
}
