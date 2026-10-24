/**
 * The devices-share-one-berth fixture: two devices of one person in one container (see the steps
 * file's header for what is real and what is a test clock). Helpers only — no step or hook here.
 */
import assert from 'node:assert/strict';
import { type ChildProcess, spawn, spawnSync } from 'node:child_process';
import { createSocket } from 'node:dgram';
import {
  chmodSync,
  closeSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readdirSync,
  readFileSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, relative, resolve } from 'node:path';

import { eprBinary, resolvedEpr } from './epr-binary.js';

export const TAG = '@concern:devices-share-one-berth';
export const repository = resolve('../..');
export const HANDLE = 'matthew';
// The `epr` the @requires:epr-cli guard resolves ($EPR_BIN, else `epr` on PATH): one resolver, so
// a run the guard holds is a run this fixture never starts. Only read (its mtime and the binary).
export const EPR = resolvedEpr ?? eprBinary;
export const POLICY = join(repository, 'genesis/agentic/pool-policy.json');
export const ROSTER = `.eprfs/status/participants/${HANDLE}.jsonl`;
export const DEV = 'dev-integration';
export const STEWARD = new Set([DEV, 'alpha-fleet', 'jenkins-dispatch']);

export type Label = 'A' | 'B';
export interface Outcome {
  status: number | null;
  stdout: string;
  stderr: string;
  ms: number;
}
export interface Device {
  label: Label;
  dir: string;
  config: string;
  berthDir: string;
  repo: string;
  log: string;
  did: string;
  node: string;
  port: number;
  daemon?: ChildProcess;
  daemonStarts: number;
}
export interface Fixture {
  base: string;
  origin: string;
  baseSha: string;
  devices: Record<Label, Device>;
  last?: Outcome;
  push?: Outcome;
  clockBase?: number;
  pending: Partial<Record<Label, { started: number; outcome: Promise<Outcome> }>>;
  outcomes: Partial<Record<Label, Outcome>>;
  linkBackAt?: number;
}
export const other = (label: Label): Label => (label === 'A' ? 'B' : 'A');
export const sleep = (ms: number) => new Promise(done => setTimeout(done, ms));

/** The environment one device's processes see: its own config home, berth store and checkout. */
export function env(d: Device, extra: Record<string, string> = {}): NodeJS.ProcessEnv {
  const keep = ['PATH', 'LANG', 'GIT_CONFIG_COUNT', 'GIT_CONFIG_KEY_0', 'GIT_CONFIG_VALUE_0'];
  const out: NodeJS.ProcessEnv = {};
  for (const key of keep) if (process.env[key] !== undefined) out[key] = process.env[key];
  return {
    ...out,
    HOME: join(d.dir, 'home'),
    XDG_CONFIG_HOME: d.config,
    BERTH_DIR: d.berthDir,
    CLAUDE_CONFIG_DIR: join(d.dir, 'claude-config'),
    CLAUDE_PROJECT_DIR: repository,
    EPR_BIN: EPR,
    ...extra,
  };
}

export function runSync(
  argv: string[],
  options: { cwd?: string; env?: NodeJS.ProcessEnv } = {}
): Outcome {
  const started = Date.now();
  const r = spawnSync(argv[0], argv.slice(1), {
    cwd: options.cwd,
    env: options.env ?? process.env,
    encoding: 'utf8',
    timeout: 60_000,
    maxBuffer: 8 * 1024 * 1024,
  });
  return {
    status: r.status,
    stdout: r.stdout ?? '',
    stderr: r.stderr ?? '',
    ms: Date.now() - started,
  };
}

export function run(
  argv: string[],
  options: { cwd?: string; env?: NodeJS.ProcessEnv } = {}
): Promise<Outcome> {
  const started = Date.now();
  return new Promise(done => {
    const child = spawn(argv[0], argv.slice(1), { cwd: options.cwd, env: options.env });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', chunk => (stdout += chunk));
    child.stderr.on('data', chunk => (stderr += chunk));
    child.on('close', status => done({ status, stdout, stderr, ms: Date.now() - started }));
  });
}

export function must(o: Outcome, what: string): Outcome {
  assert.equal(o.status, 0, `${what} exited ${o.status}\n${o.stdout}\n${o.stderr}`);
  return o;
}

export function epr(d: Device, args: string[]): Outcome {
  return runSync([EPR, ...args], { cwd: d.repo, env: env(d) });
}

export function eprJson(d: Device, args: string[]): Record<string, unknown> {
  return JSON.parse(must(epr(d, args), `epr ${args.join(' ')} on ${d.label}`).stdout);
}

export function berthArgv(d: Device): string[] {
  return ['python3', join(d.repo, 'genesis/agentic/bin/berth')];
}

export function berth(
  d: Device,
  args: string[],
  extra: Record<string, string> = {}
): Promise<Outcome> {
  return run([...berthArgv(d), ...args], { cwd: d.repo, env: env(d, extra) });
}

export function git(cwd: string, args: string[], extra: NodeJS.ProcessEnv = {}): Outcome {
  return runSync(
    ['git', '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', ...args],
    { cwd, env: { ...process.env, HOME: cwd, ...extra } }
  );
}

/** `epr peer leases --json` on device `d`: its standing, its channel store and its lease view. */
export function leases(d: Device): Record<string, unknown> {
  return eprJson(d, ['peer', 'leases', '--json', '--root', d.repo, '--policy', POLICY]);
}

export function verdict(d: Device, resource: string): Record<string, unknown> | undefined {
  const v = leases(d)[resource];
  return v && typeof v === 'object' ? (v as Record<string, unknown>) : undefined;
}

export function peerStatus(d: Device): {
  ownSeq: number | null;
  devices: { device: string; seq: number | null; ageS: number | null }[];
} {
  return eprJson(d, ['peer', 'status', '--json', '--root', d.repo]) as never;
}

export async function waitFor(
  what: string,
  timeoutMs: number,
  check: () => boolean
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    let ok = false;
    try {
      ok = check();
    } catch {
      ok = false;
    }
    if (ok) return;
    if (Date.now() > deadline) assert.fail(`timed out after ${timeoutMs} ms waiting for: ${what}`);
    await sleep(200);
  }
}

export function freePort(): Promise<number> {
  return new Promise((done, fail) => {
    const socket = createSocket('udp4');
    socket.on('error', fail);
    socket.bind(0, '127.0.0.1', () => {
      const { port } = socket.address();
      socket.close(() => done(port));
    });
  });
}

export function startDaemon(f: Fixture, d: Device): void {
  const peer = f.devices[other(d.label)];
  const log = openSync(d.log, 'a');
  d.daemon = spawn(
    EPR,
    [
      'peer',
      'serve',
      '--root',
      d.repo,
      '--listen',
      `127.0.0.1:${d.port}`,
      '--peer',
      `${peer.node}@127.0.0.1:${peer.port}`,
      '--policy',
      POLICY,
    ],
    { cwd: d.repo, env: env(d), stdio: ['ignore', log, log] }
  );
  closeSync(log);
  d.daemonStarts += 1;
}

export async function stopDaemon(d: Device): Promise<void> {
  const daemon = d.daemon;
  if (daemon?.exitCode !== null || daemon.signalCode !== null) return;
  const exited = new Promise(done => daemon.once('exit', done));
  daemon.kill('SIGTERM');
  const killed = await Promise.race([exited.then(() => true), sleep(5000).then(() => false)]);
  if (!killed) {
    daemon.kill('SIGKILL');
    await exited;
  }
}

/** Device `d` has folded `from`'s head as it stands now (a head at least as new as `from`'s own). */
export async function foldedCurrentHead(
  d: Device,
  from: Device,
  timeoutMs = 20_000
): Promise<void> {
  let own: number | null = null;
  await waitFor(`${d.label} folds ${from.label}'s current head`, timeoutMs, () => {
    own ??= peerStatus(from).ownSeq;
    const seen = peerStatus(d).devices.find(x => x.device === from.did);
    return own !== null && seen !== undefined && seen.seq !== null && seen.seq >= own;
  });
}

export function ledger(d: Device): Record<string, unknown>[] {
  const path = join(d.berthDir, 'ledger.jsonl');
  if (!existsSync(path)) return [];
  return readFileSync(path, 'utf8')
    .split('\n')
    .filter(line => line.trim())
    .map(line => JSON.parse(line));
}

/** When `session` on `d` claimed `resource`, as its berth stamped it (RFC 3339, whole seconds). */
export function claimedAt(d: Device, session: string, resource: string): string {
  const row = ledger(d)
    .filter(r => r.kind === 'claim' && r.session === session && r.resource === resource)
    .pop();
  assert.ok(row, `no claim of ${resource} by ${session} on ${d.label}`);
  return rfc3339(Number(row.claimed_at));
}

export function rfc3339(seconds: number): string {
  return new Date(Math.floor(seconds) * 1000).toISOString().replace(/\.\d{3}Z$/, 'Z');
}

/** `berth status` on `d`: the block it prints for device `did` among the other devices. */
export function otherDeviceBlock(status: string, did: string): string[] {
  const section = status.split('other devices')[1] ?? '';
  const lines = section.split('\n');
  const start = lines.findIndex(l => l.startsWith(`    device ${did}`));
  assert.ok(start >= 0, `berth status lists no device ${did}:\n${status}`);
  const block = [lines[start]];
  for (const l of lines.slice(start + 1)) {
    if (!l.startsWith('      ')) break;
    block.push(l);
  }
  return block;
}

export function refusalLine(o: Outcome, resource: string): string {
  const line = `${o.stdout}\n${o.stderr}`
    .split('\n')
    .find(l => l.startsWith(`${resource}: REFUSED — `));
  assert.ok(line, `no refusal line for ${resource}:\n${o.stdout}\n${o.stderr}`);
  return line;
}

export function holds(d: Device, resource: string, session: string): boolean {
  const who = JSON.parse(
    must(runSync([...berthArgv(d), 'who', resource], { env: env(d) }), 'berth who').stdout
  );
  let mine: { holder?: string }[] = [];
  if (Array.isArray(who)) mine = who;
  else if (who) mine = [who];
  return mine.some((lease: { holder?: string }) => lease.holder === session);
}

// ---------------------------------------------------------------- the fixture

let eprChecked: string | null = null;

/**
 * Every tree `epr` builds from, derived from the Cargo path dependencies: start at the eprfs
 * workspace's manifests and follow every `path = "…"` that names a directory holding a
 * `Cargo.toml`, transitively (the roster and contest admission in `elohim/epr-rea` among them).
 * Repository-relative.
 */
function eprSources(): string[] {
  const seen = new Set<string>();
  const queue = ['elohim/eprfs'];
  for (const entry of readdirSync(join(repository, 'elohim/eprfs'), { withFileTypes: true })) {
    if (
      entry.isDirectory() &&
      existsSync(join(repository, 'elohim/eprfs', entry.name, 'Cargo.toml'))
    )
      queue.push(`elohim/eprfs/${entry.name}`);
  }
  while (queue.length > 0) {
    const dir = queue.shift() as string;
    if (seen.has(dir)) continue;
    seen.add(dir);
    const manifest = readFileSync(join(repository, dir, 'Cargo.toml'), 'utf8');
    for (const match of manifest.matchAll(/\bpath\s*=\s*"([^"]+)"/g)) {
      const target = relative(repository, resolve(repository, dir, match[1]));
      if (existsSync(join(repository, target, 'Cargo.toml')) && !seen.has(target))
        queue.push(target);
    }
  }
  return [...seen].sort((x, y) => x.localeCompare(y));
}

/** The `epr` must exist and be newer than every tracked source it builds from: fail loudly otherwise. */
export function checkEpr(): void {
  if (eprChecked === EPR) return;
  const build =
    'CARGO_TARGET_DIR=/tmp/eprfs-gate-target RUSTFLAGS= cargo build --manifest-path ' +
    'elohim/eprfs/Cargo.toml -p elohim-epr-cli --bin epr, then set EPR_BIN=/tmp/eprfs-gate-target/debug/epr ' +
    '(or put a fresh epr first on PATH)';
  assert.ok(existsSync(EPR), `no epr at ${EPR} — build it: ${build}`);
  const files = must(
    runSync(['git', 'ls-files', ...eprSources()], { cwd: repository }),
    'git ls-files'
  ).stdout.split('\n');
  let newest = 0;
  let newestFile = '';
  for (const file of files) {
    if (!/(\.rs|Cargo\.toml|Cargo\.lock)$/.test(file)) continue;
    const mtime = statSync(join(repository, file)).mtimeMs;
    if (mtime > newest) [newest, newestFile] = [mtime, file];
  }
  assert.ok(
    statSync(EPR).mtimeMs >= newest,
    `${EPR} is older than ${newestFile} — a stale epr measures old code; rebuild: ${build}`
  );
  eprChecked = EPR;
}

export function makeDevice(
  base: string,
  name: string,
  origin: string
): Omit<Device, 'label' | 'did' | 'node' | 'port'> {
  const dir = join(base, name);
  const repo = join(dir, 'repo');
  for (const sub of ['config', 'berth', 'home', 'claude-config'])
    mkdirSync(join(dir, sub), { recursive: true });
  must(git(dir, ['clone', '-q', '-b', 'dev', origin, repo]), 'clone');
  // berth folds into the flow store of the checkout it lives in: the one this device's daemon serves.
  mkdirSync(join(repo, '.claude'), { recursive: true });
  symlinkSync(join(repository, '.claude/hooks'), join(repo, '.claude/hooks'));
  symlinkSync(join(repository, '.claude/epr-meta'), join(repo, '.claude/epr-meta'));
  // The real pre-push hook, as this checkout's own hook (no git config is changed).
  copyFileSync(join(repository, '.husky/pre-push.bash'), join(repo, '.git/hooks/pre-push'));
  chmodSync(join(repo, '.git/hooks/pre-push'), 0o755);
  return {
    dir,
    config: join(dir, 'config'),
    berthDir: join(dir, 'berth'),
    repo,
    log: join(dir, 'serve.log'),
    daemonStarts: 0,
  };
}

/** Both devices: checkouts, keys, the roster ceremony and running daemons. */
export async function household(): Promise<Fixture> {
  const base = mkdtempSync(join(tmpdir(), 'devices-share-one-berth-'));
  // The repository both devices integrate on: a bare origin whose dev holds berth.
  const seed = join(base, 'seed');
  const origin = join(base, 'origin.git');
  mkdirSync(join(seed, '.epr-meta'), { recursive: true });
  mkdirSync(join(seed, 'genesis/agentic/bin'), { recursive: true });
  writeFileSync(
    join(seed, '.epr-meta/repository.yaml'),
    'version: 1\nagent: repo:ethosengine/elohim\n'
  );
  writeFileSync(join(seed, 'README.md'), '# devices-share-one-berth fixture\n');
  writeFileSync(join(seed, '.gitignore'), '.claude/\n.eprfs/status/flows.jsonl\n');
  // berth, and the `just status device` module it finds `epr` and the device key through.
  for (const name of ['berth', 'device-preflight'])
    copyFileSync(
      join(repository, 'genesis/agentic/bin', name),
      join(seed, 'genesis/agentic/bin', name)
    );
  // The checks the real hook runs after its claims, as this fixture repository has them: it holds
  // no Rust, so its style reach gate has nothing to measure, and a push that changes only the
  // roster selects no gate (the real `.ci-ignore` reader over the fixture's own `.ci-ignore`).
  mkdirSync(join(seed, 'genesis/orchestrator'), { recursive: true });
  writeFileSync(
    join(seed, 'genesis/orchestrator/rust-style.sh'),
    '#!/usr/bin/env bash\n# The fixture repository holds no Rust: nothing to measure.\nexit 0\n'
  );
  copyFileSync(
    join(repository, 'genesis/orchestrator/ci-ignore.mjs'),
    join(seed, 'genesis/orchestrator/ci-ignore.mjs')
  );
  writeFileSync(join(seed, '.ci-ignore'), '.eprfs/status/participants/\n');
  must(git(base, ['init', '-q', '--bare', origin]), 'init origin');
  must(git(seed, ['init', '-q', '-b', 'dev']), 'init seed');
  must(git(seed, ['add', '-A']), 'add');
  must(git(seed, ['commit', '-q', '-m', 'fixture']), 'commit');
  must(git(seed, ['push', '-q', origin, 'dev']), 'seed origin');
  const baseSha = must(git(seed, ['rev-parse', 'HEAD']), 'rev-parse').stdout.trim();

  // Two devices, X first on the roster; each mints its own keys on first use.
  const x = { ...makeDevice(base, 'x', origin), label: 'A' as Label, did: '', node: '', port: 0 };
  const y = { ...makeDevice(base, 'y', origin), label: 'B' as Label, did: '', node: '', port: 0 };
  const carry = (from: Device, to: Device) => {
    mkdirSync(join(to.repo, '.eprfs/status/participants'), { recursive: true });
    copyFileSync(join(from.repo, ROSTER), join(to.repo, ROSTER));
  };
  must(
    epr(x, [
      'actor',
      'witness',
      '--subject',
      `human:${HANDLE}`,
      '--as',
      'agent:fixture@claude-opus-5-5',
      '--session',
      'fixture-x',
      '--basis',
      'the devices-share-one-berth fixture',
      '--root',
      x.repo,
    ]),
    'witness'
  );
  carry(x, y);
  const request = join(base, 'request.json');
  const authorization = join(base, 'authorization.json');
  writeFileSync(
    request,
    must(epr(y, ['actor', 'device', 'enroll', '--handle', HANDLE, '--root', y.repo]), 'enroll')
      .stdout
  );
  writeFileSync(
    authorization,
    must(epr(x, ['actor', 'device', 'authorize', request, '--root', x.repo]), 'authorize').stdout
  );
  must(epr(y, ['actor', 'device', 'bind', authorization, '--root', y.repo]), 'bind');
  carry(y, x);
  for (const d of [x, y]) d.node = must(epr(d, ['peer', 'id']), 'peer id').stdout.trim();
  must(
    epr(x, ['actor', 'device', 'endpoint', '--node-id', x.node, '--root', x.repo]),
    'endpoint x'
  );
  carry(x, y);
  must(
    epr(y, ['actor', 'device', 'endpoint', '--node-id', y.node, '--root', y.repo]),
    'endpoint y'
  );
  carry(y, x);
  for (const d of [x, y]) {
    d.did = String(eprJson(d, ['peer', 'status', '--json', '--root', d.repo]).device);
    d.port = await freePort();
  }
  // "A" is the device whose signing key sorts lower; the story's tie-break examples rely on it.
  const [a, b] = x.did < y.did ? [x, y] : [y, x];
  a.label = 'A';
  b.label = 'B';
  const f: Fixture = {
    base,
    origin,
    baseSha,
    devices: { A: a, B: b },
    pending: {},
    outcomes: {},
  };
  startDaemon(f, a);
  startDaemon(f, b);
  return f;
}
