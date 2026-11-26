#!/usr/bin/env node
// Detached, decision-free sampling of one reused gate verdict.

import {
  closeSync, existsSync, mkdirSync, openSync, readFileSync, readdirSync, rmSync, statSync, writeSync,
} from 'node:fs';
import { spawnSync } from 'node:child_process';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const SAMPLE_PREFIX = 'enrich-sample-';
const DEFAULT_DEADLINE_SECONDS = 3600;

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

function deadlineMilliseconds(env) {
  const raw = String(env.GATE_ENRICH_SAMPLE_TIMEOUT_SECONDS ?? '').trim();
  const seconds = raw ? Number(raw) : DEFAULT_DEADLINE_SECONDS;
  return Number.isFinite(seconds) && seconds > 0 ? Math.ceil(seconds * 1000) : DEFAULT_DEADLINE_SECONDS * 1000;
}

function timedOut(result) {
  return result?.timedOut === true || result?.error?.code === 'ETIMEDOUT';
}

class SampleDeadline extends Error {
  constructor() {
    super('timeout');
    this.name = 'SampleDeadline';
  }
}

class SampleNotComparable extends Error {
  constructor(message) {
    super(message);
    this.name = 'SampleNotComparable';
  }
}

function errorKindAndLine(error) {
  const kind = String(error?.name || error?.constructor?.name || typeof error)
    .replace(/[\r\n]+/g, ' ').trim() || 'unknown';
  const message = String(error?.message ?? error).replace(/[\r\n]+/g, ' ').trim() || 'no message';
  return `${kind}: ${message}`;
}

export async function runEnrichSample(options, {
  env = process.env,
  spawn = spawnSync,
  clock = () => Date.now(),
  sleep = milliseconds => new Promise(resolveSleep => setTimeout(resolveSleep, milliseconds)),
} = {}) {
  const root = resolve(options.root);
  const project = oneLine(options.project, '--project');
  const commit = oneLine(options.commit, '--commit');
  const closure = oneLine(options.closure, '--closure');
  const fromDevice = options.from_device ? oneLine(options.from_device, '--from-device') : null;
  const commit12 = safePart(commit.slice(0, 12));
  const projectPart = safePart(project);
  const timeoutMs = deadlineMilliseconds(env);
  const startedAt = clock();
  const deadline = startedAt + timeoutMs;
  // Keep one slice of the single deadline for the not-comparable record and cleanup after a kill.
  const reserveMs = Math.min(10_000, Math.max(100, Math.floor(timeoutMs / 10)));
  const workDeadline = deadline - reserveMs;
  const sampleId = `${SAMPLE_PREFIX}${commit12}-${projectPart}-${process.pid}`;
  const samplePath = join(root, '.worktrees', sampleId);
  const scratchRoot = join(root, '.eprfs', 'status', 'enrich-sample');
  const scratchPath = join(scratchRoot, sampleId);
  const logPath = join(scratchPath, 'sample.log');
  mkdirSync(scratchPath, { recursive: true });
  const log = openSync(logPath, 'a', 0o600);
  const line = text => writeSync(log, `${new Date(clock()).toISOString()} ${text}\n`);
  const run = (command, args, runOptions = {}) => {
    const {
      deadline: commandDeadline = workDeadline,
      timeout: requestedTimeout = Infinity,
      ...spawnOptions
    } = runOptions;
    const left = commandDeadline - clock();
    if (left <= 0) {
      const result = { status: null, error: { code: 'ETIMEDOUT', message: 'sample deadline expired' }, timedOut: true };
      line(`$ ${command} ${args.join(' ')}`);
      line('exit timeout');
      return result;
    }
    line(`$ ${command} ${args.join(' ')}`);
    const result = spawn(command, args, {
      cwd: spawnOptions.cwd || root,
      env: spawnOptions.env || env,
      stdio: spawnOptions.stdio || ['ignore', log, log],
      ...spawnOptions,
      timeout: Math.max(1, Math.floor(Math.min(requestedTimeout, left))),
      killSignal: 'SIGKILL',
    });
    if (result?.error?.code === 'ETIMEDOUT') result.timedOut = true;
    line(`exit ${timedOut(result) ? 'timeout' : statusOf(result)}`);
    return result;
  };
  const requireSuccess = (result, label) => {
    if (timedOut(result)) throw new SampleDeadline();
    if (result?.status !== 0) throw new SampleNotComparable(`${label} failed (${statusOf(result)})`);
  };
  const removeOwned = path => {
    if (!path.startsWith(`${join(root, '.worktrees', SAMPLE_PREFIX)}`)
        && !path.startsWith(`${join(scratchRoot, SAMPLE_PREFIX)}`)) {
      throw new Error(`refusing to remove non-sampler path ${path}`);
    }
    rmSync(path, { recursive: true, force: true });
  };

  let worktreeAdded = false;
  let named = null;
  let method = `enrich-sample:${commit12}:method-unavailable`;
  let outcome = 'not-comparable';
  let reason = 'setup failed before execution';
  let gateSeconds = null;
  let enriched = false;
  try {
    line(`sample project=${project} commit=${commit} closure=${closure}${fromDevice ? ` from-device=${fromDevice}` : ''}`);

    // A crashed sampler owns only these two prefixes. Age is judged against this sample's one
    // declared deadline, so a still-live sample is never swept early.
    const staleBefore = startedAt - timeoutMs;
    const worktreesRoot = join(root, '.worktrees');
    if (existsSync(worktreesRoot)) {
      for (const entry of readdirSync(worktreesRoot, { withFileTypes: true })) {
        if (!entry.isDirectory() || !entry.name.startsWith(SAMPLE_PREFIX)) continue;
        const path = join(worktreesRoot, entry.name);
        if (path === samplePath || statSync(path).mtimeMs >= staleBefore) continue;
        const removed = run('git', ['-C', root, 'worktree', 'remove', '--force', path]);
        if (timedOut(removed)) throw new SampleDeadline();
        removeOwned(path);
        line(`swept stale worktree ${entry.name}`);
      }
    }
    for (const entry of readdirSync(scratchRoot, { withFileTypes: true })) {
      if (!entry.isDirectory() || !entry.name.startsWith(SAMPLE_PREFIX)) continue;
      const path = join(scratchRoot, entry.name);
      if (path === scratchPath || statSync(path).mtimeMs >= staleBefore) continue;
      removeOwned(path);
      line(`swept stale scratch ${entry.name}`);
    }

    const resolveArgs = [
      'flow', 'enrich', '--project', project, '--closure', closure,
      ...(fromDevice ? ['--from-device', fromDevice] : []),
      '--resolve-only', '--json', '--root', root,
    ];
    const resolved = run(env.EPR_BIN || 'epr', resolveArgs, {
      cwd: root, stdio: ['ignore', 'pipe', log], encoding: 'utf8',
    });
    requireSuccess(resolved, 'verdict lookup');
    try { named = JSON.parse(String(resolved.stdout || '')); }
    catch { throw new SampleNotComparable('verdict lookup returned invalid JSON'); }
    if (named.project !== project || named.closure !== closure || named.commit !== commit || !named.verdict) {
      throw new SampleNotComparable('verdict lookup did not name the requested project, closure, and commit');
    }
    line(`named verdict=${named.verdict} closure=${named.closure}`);

    const justOid = run('git', ['-C', root, 'rev-parse', `${commit}:justfile`], {
      stdio: ['ignore', 'pipe', log], encoding: 'utf8',
    });
    requireSuccess(justOid, 'justfile method lookup');
    const runnerOid = run('git', ['-C', root, 'rev-parse', `${commit}:genesis/orchestrator/gate-runner.mjs`], {
      stdio: ['ignore', 'pipe', log], encoding: 'utf8',
    });
    requireSuccess(runnerOid, 'gate-runner method lookup');
    method = `justfile:${String(justOid.stdout).trim()},gate-runner:${String(runnerOid.stdout).trim()}`;

    while (true) {
      const cargo = run('pgrep', ['-x', 'cargo'], { stdio: 'ignore' });
      const rustc = run('pgrep', ['-x', 'rustc'], { stdio: 'ignore' });
      if (timedOut(cargo) || timedOut(rustc)) throw new SampleDeadline();
      if (cargo.status !== 0 && rustc.status !== 0) break;
      const wait = Math.min(1000, workDeadline - clock());
      if (wait <= 0) throw new SampleDeadline();
      line(`build processes still active; waiting ${wait} ms`);
      await sleep(wait);
    }

    const added = run('git', ['-C', root, 'worktree', 'add', '--detach', samplePath, commit]);
    requireSuccess(added, 'worktree add');
    worktreeAdded = true;

    const submodules = run('git', [
      '-C', samplePath, 'submodule', 'update', '--init', '--recursive', '--',
      'elohim/brit', 'elohim/rakia', 'sophia',
    ]);
    requireSuccess(submodules, 'submodule prep');
    const installed = run('pnpm', ['install', '--frozen-lockfile'], { cwd: samplePath });
    requireSuccess(installed, 'pnpm install');

    const resultPath = join(scratchPath, 'gate-result.json');
    const gateStarted = clock();
    const gate = run('just', ['gate', project], {
      cwd: samplePath,
      env: {
        ...env,
        GATE_RECORD: 'off',
        GATE_EVIDENCE: 'off',
        GATE_ENRICH_SAMPLE: 'off',
        GATE_DEFER: 'off',
        GATE_READING: 'off',
        GATE_TASK: 'off',
        GATE_PREFLIGHT: 'off',
        GATE_SAMPLE_RESULT: resultPath,
      },
    });
    gateSeconds = secondsText(clock() - gateStarted);
    if (timedOut(gate)) throw new SampleDeadline();
    if (gate?.signal) throw new SampleNotComparable(`killed: ${gate.signal}`);
    if (gate?.status === null || gate?.status === undefined) {
      throw new SampleNotComparable(`gate setup failed (${statusOf(gate)})`);
    }

    let actual;
    try { actual = JSON.parse(readFileSync(resultPath, 'utf8')); }
    catch (error) {
      throw new Error(`gate did not report a completed execution (${errorKindAndLine(error)})`);
    }
    if (actual.executed !== true) throw new SampleNotComparable(actual.reason || 'gate was skipped');
    if (actual.project !== project || actual.commit !== commit) {
      throw new SampleNotComparable('gate executed a different project or commit');
    }
    if (actual.closure !== closure) {
      throw new SampleNotComparable(`closure differs: verdict ${closure}, execution ${actual.closure || 'unavailable'}`);
    }
    if (actual.signal) throw new SampleNotComparable(`killed: ${actual.signal}`);
    if (actual.recordable !== true) throw new SampleNotComparable(actual.reason || 'execution was not recordable');

    const dirty = run('git', ['-C', samplePath, 'status', '--porcelain', '--untracked-files=normal'], {
      stdio: ['ignore', 'pipe', log], encoding: 'utf8',
    });
    requireSuccess(dirty, 'post-run worktree check');
    if (String(dirty.stdout || '').trim()) throw new SampleNotComparable('dirty tree after execution');

    outcome = actual.status === 0 && gate.status === 0 ? 'green' : 'red';
    reason = null;
  } catch (error) {
    outcome = 'not-comparable';
    reason = error instanceof SampleDeadline
      ? 'timeout'
      : error instanceof SampleNotComparable
        ? oneLine(error.message, 'sample failure')
        : `unexpected ${errorKindAndLine(error)}`;
    line(`sample stopped: ${reason}`);
  }

  if (named) {
    const enrichArgs = [
      'flow', 'enrich', '--verdict', named.verdict,
      '--kind', 're-executed', '--outcome', outcome, '--method', method,
      ...(reason ? ['--reason', reason] : []),
      ...(gateSeconds === null ? [] : ['--seconds', gateSeconds]),
      '--json', '--root', root,
    ];
    const appended = run(env.EPR_BIN || 'epr', enrichArgs, { cwd: root, deadline });
    enriched = appended.status === 0;
    if (!enriched) line(`enrichment refused (${statusOf(appended)}); the gate decision remains unchanged`);
    else line(`enrichment appended outcome=${outcome}${reason ? ` reason=${reason}` : ''}`);
  }

  if (worktreeAdded) {
    try {
      const removed = run('git', ['-C', root, 'worktree', 'remove', '--force', samplePath], { deadline });
      if (removed.status !== 0) {
        line(`worktree removal failed (${statusOf(removed)}); removing owned directory only`);
        removeOwned(samplePath);
      }
    } catch (error) {
      line(`worktree removal failed (${String(error?.message || error).replace(/[\r\n]+/g, ' ')})`);
    }
  }
  closeSync(log);
  return { outcome, reason, seconds: gateSeconds, enriched, logPath, samplePath };
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
