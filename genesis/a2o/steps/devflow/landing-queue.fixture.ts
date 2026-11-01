import assert from 'node:assert/strict';
import {
  accessSync,
  chmodSync,
  constants,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join } from 'node:path';
import { pathToFileURL } from 'node:url';

import {
  type Device,
  env,
  EPR,
  type Fixture,
  git,
  household,
  type Label,
  leases,
  must,
  repository,
  runSync,
  spawnTracked,
  type TrackedOutcome,
} from './devices-share-one-berth.fixture.js';
import { seedGateClosure, stubTools } from './gate-closure.seed.js';

export const TAG = '@concern:sprint-lands-each-closure-once';
const CHANGE_ID_SHIM_MISSING =
  'no .husky/commit-msg on this tree — the Change-Id shim lands in S4 (R-G)';

export interface LandingFixture extends Fixture {
  worktrees: Partial<Record<'wt-a' | 'wt-b', string>>;
  pendingPushes: Set<Promise<TrackedOutcome>>;
  lastPush?: TrackedOutcome;
  lastChild?: ReturnType<typeof spawnTracked>['child'];
  changeId?: string;
  intent?: string;
}

interface FlowRow {
  cid: string;
  record: {
    kind: string;
    classifiedAs?: string[];
    resourceSpec?: { classifiedAs?: string[] };
    resource?: string;
  };
}

interface Marker {
  re: RegExp;
  format: (...args: string[]) => string;
}
type MarkerSet = Record<string, Marker>;

interface IntentSettlement {
  kind?: string;
}

interface LandingIntent {
  change?: string;
  intent?: string;
  state?: string;
  settled?: IntentSettlement[];
}

interface AdvisoryIntent {
  kind?: string;
}

export interface LandingIntentView {
  store?: unknown;
  intents: LandingIntent[];
  advisory: AdvisoryIntent[];
}

export function seedLanding(seed: string): void {
  seedGateClosure(seed, {
    reuse: ['app'],
    channel: { max_skew_s: 5, freshness_s: 20 },
  });
  writeFileSync(join(seed, '.gitignore'), '.claude/\nnode_modules\n/.eprfs/status/*\n');
  writeFileSync(join(seed, '.ci-ignore'), '.eprfs/status/participants/\ndocs/\n');
}

export function installLandingHooks(d: Device): void {
  const hook = join(d.repo, '.husky/commit-msg');
  if (!existsSync(hook)) throw new Error(CHANGE_ID_SHIM_MISSING);
  chmodSync(join(d.repo, '.husky/pre-push'), 0o755);
  chmodSync(join(d.repo, '.husky/pre-push.bash'), 0o755);
  chmodSync(hook, 0o755);
  must(
    git(d.repo, ['config', 'core.hooksPath', join(d.repo, '.husky')]),
    `configure core.hooksPath on device ${d.label}`
  );
}

export async function landingHousehold(): Promise<LandingFixture> {
  if (!existsSync(join(repository, '.husky/commit-msg'))) throw new Error(CHANGE_ID_SHIM_MISSING);
  const f = (await household({
    prefix: 'landing-queue-',
    seed: seedLanding,
    policy: {
      A: 'genesis/agentic/pool-policy.json',
      B: 'genesis/agentic/pool-policy.json',
    },
    deviceEnv: stubTools,
  })) as LandingFixture;
  f.worktrees = {};
  f.pendingPushes = new Set();
  for (const d of Object.values(f.devices)) {
    installLandingHooks(d);
    d.extraEnv = {
      ...d.extraEnv,
      BRIT_STUB_AGENT: `${'a'.repeat(63)}${d.label === 'A' ? 'a' : 'b'}`,
    };
    symlinkSync(join(repository, 'node_modules'), join(d.dir, 'node_modules'));
  }
  return f;
}

export function identity(label: Label): Record<string, string> {
  const name = `fixture-${label}`;
  const email = `${label.toLowerCase()}@fixture.invalid`;
  return {
    GIT_AUTHOR_NAME: name,
    GIT_AUTHOR_EMAIL: email,
    GIT_COMMITTER_NAME: name,
    GIT_COMMITTER_EMAIL: email,
  };
}

function executable(path: string): boolean {
  try {
    accessSync(path, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

export function pathWithout(tool: string): string {
  const path = (process.env.PATH ?? '')
    .split(':')
    .filter(Boolean)
    .filter(dir => !executable(join(dir, tool)))
    .join(':');
  for (const kept of ['node', 'git', 'just']) {
    const found = path.split(':').some(dir => executable(join(dir, kept)));
    assert.ok(found, `hiding ${tool} also hid required executable ${kept}`);
  }
  assert.ok(!path.split(':').some(dir => executable(join(dir, tool))), `${tool} remains on PATH`);
  return path;
}

export function eprWrapper(base: string, mode: 'no-intents' | 'non-json'): string {
  const path = join(base, 'bin', `epr-${mode}`);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(
    path,
    String.raw`#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
const args = process.argv.slice(2);
if (args[0] === 'peer' && args[1] === 'intents') {
  if (${JSON.stringify(mode)} === 'no-intents') {
    process.stderr.write("unrecognized subcommand 'intents'\n");
    process.exit(2);
  }
  process.stdout.write('not json\n');
  process.exit(0);
}
const child = spawnSync(${JSON.stringify(EPR)}, args, { env: process.env, stdio: 'inherit' });
process.exit(child.status ?? 1);
`
  );
  chmodSync(path, 0o755);
  return path;
}

export function addWorktree(d: Device, name: string): string {
  const path = join(d.dir, `wt-${name}`);
  must(
    git(d.repo, ['worktree', 'add', '-q', path, '-b', name, 'origin/dev']),
    `add worktree ${name}`
  );
  mkdirSync(join(path, '.claude'), { recursive: true });
  symlinkSync(join(repository, '.claude/hooks'), join(path, '.claude/hooks'));
  symlinkSync(join(repository, '.claude/epr-meta'), join(path, '.claude/epr-meta'));
  symlinkSync(join(repository, '.claude/scripts'), join(path, '.claude/scripts'));
  return path;
}

export function commitChange(
  cwd: string,
  options: {
    path: string;
    body: string;
    message: string;
    verify?: boolean;
    label?: Label;
  }
): string {
  mkdirSync(dirname(join(cwd, options.path)), { recursive: true });
  writeFileSync(join(cwd, options.path), options.body);
  must(git(cwd, ['add', options.path], identity(options.label ?? 'A')), `add ${options.path}`);
  const args = ['commit', '-q', '-m', options.message];
  if (options.verify === false) args.push('--no-verify');
  must(git(cwd, args, identity(options.label ?? 'A')), `commit ${options.path}`);
  return must(git(cwd, ['rev-parse', 'HEAD']), 'read committed sha').stdout.trim();
}

export function moveBase(f: LandingFixture, path: string | null): string {
  if (path === null) {
    must(
      git(
        f.seed,
        ['commit', '-q', '--allow-empty', '-m', 'move dev without changing its tree'],
        identity('A')
      ),
      'empty base commit'
    );
  } else {
    commitChange(f.seed, {
      path,
      body: `base move at ${Date.now()}\n`,
      message: `move dev with ${path}`,
    });
  }
  must(git(f.seed, ['push', '--no-verify', 'origin', 'HEAD:dev']), 'move dev');
  return must(git(f.seed, ['rev-parse', 'HEAD']), 'read moved dev').stdout.trim();
}

export async function push(
  d: Device,
  cwd: string,
  session: string,
  extra: Record<string, string> = {}
): Promise<TrackedOutcome> {
  const tracked = spawnTracked(['git', 'push', 'origin', 'HEAD:dev'], {
    cwd,
    env: env(d, {
      ELOHIM_SESSION_ID: session,
      ELOHIM_SESSION_OWNER_PID: String(process.pid),
      ...identity(d.label),
      ...extra,
    }),
  });
  const timeout = new Promise<never>((_, reject) => {
    const timer = setTimeout(
      () => {
        if (tracked.child.pid) process.kill(-tracked.child.pid, 'SIGKILL');
        reject(new Error(`push hung for 30 minutes: ${cwd}`));
      },
      30 * 60 * 1000
    );
    tracked.outcome.finally(() => clearTimeout(timer)).catch(() => undefined);
  });
  const outcome = await Promise.race([tracked.outcome, timeout]);
  assertClean(d);
  return outcome;
}

export function assertClean(d: Device): void {
  const paths = [d.repo];
  for (const name of readdirSync(d.dir)) {
    if (name.startsWith('wt-')) paths.push(join(d.dir, name));
  }
  const dirty = paths.flatMap(path => {
    const status = git(path, ['status', '--porcelain']).stdout.trim();
    return status ? [`${path}:\n${status}`] : [];
  });
  assert.deepEqual(dirty, [], `fixture checkouts are dirty:\n${dirty.join('\n')}`);
}

function commonDir(d: Device): string {
  return must(
    git(d.repo, ['rev-parse', '--path-format=absolute', '--git-common-dir']),
    'resolve common dir'
  ).stdout.trim();
}

export function gateCalls(d: Device): { ns: bigint; tree: string; cwd: string }[] {
  const path = join(commonDir(d), 'gate-calls.log');
  if (!existsSync(path)) return [];
  return readFileSync(path, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map(line => {
      const [ns, tree, ...cwd] = line.split(' ');
      return { ns: BigInt(ns), tree, cwd: cwd.join(' ') };
    });
}

export function intents(d: Device): LandingIntentView {
  return JSON.parse(
    must(
      runSync([EPR, 'peer', 'intents', '--json'], { cwd: d.repo, env: env(d) }),
      `read landing intents on ${d.label}`
    ).stdout
  ) as LandingIntentView;
}

function flowRows(d: Device): FlowRow[] {
  const root = String(leases(d).store ?? d.repo);
  const path = join(root, '.eprfs/status/flows.jsonl');
  if (!existsSync(path)) return [];
  return readFileSync(path, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map(line => JSON.parse(line));
}

export function landingRows(
  d: Device,
  kind: 'atom' | 'raise' | 'intent' | 'landed' | 'withdraw' | 'dismiss'
): FlowRow[] {
  return flowRows(d).filter(row => {
    if (kind === 'atom') {
      return (
        row.record.kind === 'intent' &&
        row.record.resourceSpec?.classifiedAs?.[0] === 'dev-landing:intent'
      );
    }
    const slots = row.record.classifiedAs ?? [];
    return slots.includes('measure:dev-landing@1') && slots.includes(`reason:${kind}`);
  });
}

export async function markers(output: TrackedOutcome): Promise<Record<string, RegExpMatchArray[]>> {
  const { MARKERS } = (await import(
    pathToFileURL(join(repository, 'genesis/orchestrator/gate-landing-markers.mjs')).href
  )) as { MARKERS: MarkerSet };
  const text = `${output.stdout}\n${output.stderr}`;
  return Object.fromEntries(
    Object.entries(MARKERS).map(([key, marker]) => [
      key,
      [...text.matchAll(new RegExp(marker.re.source, 'gm'))],
    ])
  );
}

export async function holdSeconds(output: TrackedOutcome): Promise<number> {
  const found = await markers(output);
  const held = found.held.at(-1);
  assert.ok(held, `push has no held marker:\n${output.stdout}\n${output.stderr}`);
  return Number(held[1]);
}
