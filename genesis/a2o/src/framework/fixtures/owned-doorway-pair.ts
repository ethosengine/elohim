import { strict as assert } from 'node:assert';
import { spawn, type ChildProcess } from 'node:child_process';
import { createHash, randomBytes } from 'node:crypto';
import { createReadStream, readFileSync } from 'node:fs';
import { chmod, copyFile, mkdir, mkdtemp, open, readFile, readlink, rm } from 'node:fs/promises';
import { connect, createServer } from 'node:net';
import { join } from 'node:path';

import { householdMeshDir } from './household-mesh.js';
import {
  assertStillOwnedProcess,
  processStartTicks,
  resolveOwnedMeshProcess,
  stopOwnedProcess,
  type OwnedProcessHandle,
} from './owned-doorway-process.js';

type DoorwayName = 'a' | 'b';

export interface LaunchTemplate {
  executable: string;
  executableHash: string;
  runningExecutable: string;
  cwd: string;
  argv: string[];
  env: NodeJS.ProcessEnv;
}

interface OwnedDoorway {
  name: DoorwayName;
  id: string;
  url: string;
  healthPort: number;
  database: string;
  primaryPeer: string;
  handle: OwnedProcessHandle;
  child: ChildProcess;
  template: LaunchTemplate;
  argv: string[];
  env: NodeJS.ProcessEnv;
  logPath: string;
}

export interface OwnedDoorwayPairReceipt {
  urls: Record<DoorwayName, string>;
  ids: Record<DoorwayName, string>;
  primaryPeers: Record<DoorwayName, string>;
  storageTopology: Record<
    string,
    { url: string; agentPubKey?: string; conductorAdminPort?: number; conductorAppUrl?: string }
  >;
  executableHash: string;
  lifecycle: 'started' | 'cleaned' | 'cleanup-failed';
  logs: Record<DoorwayName | 'mongo', string>;
}

export interface OwnedDoorwayPairOptions {
  storagePeers: Record<
    string,
    { url?: string; agentPubKey?: string; conductorAdminPort?: number; conductorAppUrl?: string }
  >;
}

function decodeNullMap(bytes: Buffer): NodeJS.ProcessEnv {
  return Object.fromEntries(
    bytes
      .toString()
      .split('\0')
      .filter(Boolean)
      .map(item => {
        const at = item.indexOf('=');
        return [item.slice(0, at), item.slice(at + 1)];
      })
  );
}

async function sha256(path: string): Promise<string> {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(path)) hash.update(chunk as Buffer);
  return hash.digest('hex');
}

async function templateFor(name: DoorwayName): Promise<LaunchTemplate> {
  const owned = await resolveOwnedMeshProcess('doorway', name, `canonical doorway ${name}`);
  await assertStillOwnedProcess(owned, `canonical doorway ${name}`);
  const runningExecutable = `/proc/${owned.pid}/exe`;
  const [cmdline, environ, canonicalCwd, executableHash] = await Promise.all([
    readFile(`/proc/${owned.pid}/cmdline`),
    readFile(`/proc/${owned.pid}/environ`),
    readlink(`/proc/${owned.pid}/cwd`),
    sha256(runningExecutable),
  ]);
  await assertStillOwnedProcess(owned, `canonical doorway ${name}`);
  const argv = cmdline.toString().split('\0').filter(Boolean);
  assert.ok(argv.length > 0, `canonical doorway ${name} exposes no argv`);
  return {
    executable: owned.executable,
    executableHash,
    runningExecutable,
    cwd: canonicalCwd,
    argv,
    env: decodeNullMap(environ),
  };
}

async function freePort(): Promise<number> {
  const server = createServer();
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const address = server.address();
  assert.ok(
    address && typeof address === 'object',
    'ephemeral port allocation returned no address'
  );
  await new Promise<void>((resolve, reject) =>
    server.close(error => (error ? reject(error) : resolve()))
  );
  return address.port;
}

function replaceFlag(argv: string[], flag: string, value: string): string[] {
  const next = [...argv];
  const at = next.indexOf(flag);
  assert.ok(at >= 0 && at + 1 < next.length, `doorway launch template has no ${flag}`);
  next[at + 1] = value;
  return next;
}

export function withExtraSsrSlug(env: NodeJS.ProcessEnv, slug: string): NodeJS.ProcessEnv {
  const current = (env['SSR_BUNDLE_SLUGS'] ?? env['SSR_BUNDLE_SLUG'] ?? '')
    .split(',')
    .filter(value => value && !value.startsWith('epr-app-deliverability-'));
  return { ...env, SSR_BUNDLE_SLUGS: [...new Set([...current, slug])].join(',') };
}

interface StopTarget {
  handle: OwnedProcessHandle;
  label: string;
}

export async function stopFixtureProcesses(
  targets: StopTarget[],
  stop: typeof stopOwnedProcess = stopOwnedProcess
): Promise<void> {
  const failures: unknown[] = [];
  for (const target of targets) {
    try {
      await stop(target.handle, target.label);
    } catch (error) {
      failures.push(error);
    }
  }
  if (failures.length) throw new AggregateError(failures, 'owned doorway fixture cleanup failed');
}

export function fixtureDoorwayLaunch(
  template: LaunchTemplate,
  input: {
    id: string;
    listenPort: number;
    healthPort: number;
    mongoPort: number;
    database: string;
    primaryUrl: string;
    extraUrls: string[];
    /** Scenario-owned root this fixture doorway may write scratch into. */
    scenarioDir: string;
  }
): { argv: string[]; env: NodeJS.ProcessEnv } {
  let argv = replaceFlag(template.argv, '--listen', `127.0.0.1:${input.listenPort}`);
  argv = replaceFlag(argv, '--storage-url', input.primaryUrl);
  const extras = input.extraUrls.join(',');
  const extraAt = argv.indexOf('--storage-urls');
  if (extraAt >= 0) argv[extraAt + 1] = extras;
  else argv.push('--storage-urls', extras);
  // `...template.env` below inherits the CANONICAL doorway's whole launch env
  // (read live off /proc/<pid>/environ in templateFor), including its
  // SSR_BUNDLE_PATH (the repo dist) and DOORWAY_NODE_KEY_FILE (its own node
  // identity). Both must be overridden per fixture doorway, same as
  // SSR_STORAGE_URL below: sharing SSR_BUNDLE_PATH means this fixture's
  // materialize/reconcile scratch lands in the tree the packager zips as the
  // next server bundle (the household server-bundle feedback loop); sharing
  // DOORWAY_NODE_KEY_FILE means this fixture doorway signs as the canonical
  // doorway's identity instead of its own.
  const doorwayDir = join(input.scenarioDir, input.id);
  return {
    argv,
    env: {
      ...template.env,
      DOORWAY_ID: input.id,
      DOORWAY_HEALTH_PORT: String(input.healthPort),
      DOORWAY_URL: `http://127.0.0.1:${input.listenPort}`,
      MONGODB_URI: `mongodb://127.0.0.1:${input.mongoPort}`,
      MONGODB_DB: input.database,
      SSR_STORAGE_URL: input.primaryUrl,
      SSR_BUNDLE_PATH: join(doorwayDir, 'ssr', 'main.server.mjs'),
      DOORWAY_NODE_KEY_FILE: join(doorwayDir, 'node.key'),
    },
  };
}

async function waitFor(url: string, timeoutMs: number): Promise<Response> {
  const deadline = Date.now() + timeoutMs;
  let last: unknown;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(2_000) });
      if (response.ok) return response;
      last = `${response.status} ${await response.text()}`;
    } catch (error) {
      last = error;
    }
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error(`${url} did not become ready: ${String(last)}`);
}

async function waitForPort(port: number, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const ready = await new Promise<boolean>(resolve => {
      const socket = connect(port, '127.0.0.1');
      socket.once('connect', () => {
        socket.destroy();
        resolve(true);
      });
      socket.once('error', () => resolve(false));
    });
    if (ready) return;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error(`fixture mongod did not bind 127.0.0.1:${port}`);
}

async function ownedHandle(child: ChildProcess): Promise<OwnedProcessHandle> {
  assert.ok(child.pid, 'spawned process has no pid');
  return {
    pid: child.pid,
    ticks: await processStartTicks(child.pid),
    executable: await readlink(`/proc/${child.pid}/exe`),
  };
}

export async function startOwnedChild(
  executable: string,
  argv: string[],
  env: NodeJS.ProcessEnv,
  cwd: string,
  logPath: string
): Promise<{ child: ChildProcess; handle: OwnedProcessHandle }> {
  const output = await open(logPath, 'a');
  let child: ChildProcess;
  try {
    child = spawn(executable, argv.slice(1), {
      cwd,
      env,
      detached: true,
      stdio: ['ignore', output.fd, output.fd],
    });
    await new Promise<void>((resolve, reject) => {
      child.once('spawn', resolve);
      child.once('error', reject);
    });
  } finally {
    await output.close();
  }
  try {
    return { child, handle: await ownedHandle(child) };
  } catch (error) {
    let cleanupError: unknown;
    if (child.pid) {
      try {
        const handle = await ownedHandle(child);
        await stopOwnedProcess(handle, `partially acquired child ${executable}`);
      } catch (stopError) {
        if (!['ENOENT', 'ESRCH'].includes((stopError as NodeJS.ErrnoException).code ?? '')) {
          cleanupError = stopError;
        }
      }
    }
    if (cleanupError) {
      throw new AggregateError([error, cleanupError], `child acquisition and cleanup failed`);
    }
    throw error;
  }
}

export class OwnedDoorwayPair {
  private lifecycle: OwnedDoorwayPairReceipt['lifecycle'] = 'started';
  private constructor(
    private readonly root: string,
    private readonly mongo: OwnedProcessHandle,
    private mongoLogPath: string,
    private readonly doorways: Record<DoorwayName, OwnedDoorway>,
    private readonly storageTopology: OwnedDoorwayPairReceipt['storageTopology']
  ) {}

  static async start(options: OwnedDoorwayPairOptions): Promise<OwnedDoorwayPair> {
    const names = ['matthew', 'jessica', 'james'];
    const urls = Object.fromEntries(names.map(name => [name, options.storagePeers[name]?.url]));
    for (const name of names)
      assert.ok(urls[name], `owned doorway pair needs storage peer ${name}`);
    const scenarioRoot = join(householdMeshDir(), 'scenarios');
    await mkdir(scenarioRoot, { recursive: true });
    const root = await mkdtemp(join(scenarioRoot, 'epr-deliverability-doorways-'));
    const acquired: StopTarget[] = [];
    try {
      const mongoDir = join(root, 'mongo');
      await mkdir(mongoDir);
      const [mongoTemplate, aTemplate, bTemplate] = await Promise.all([
        resolveOwnedMeshProcess('mongod', 'mesh', 'canonical mesh mongod'),
        templateFor('a'),
        templateFor('b'),
      ]);
      assert.equal(
        aTemplate.executableHash,
        bTemplate.executableHash,
        'canonical doorway binaries differ'
      );
      const pinnedDoorway = join(root, 'doorway');
      await copyFile(aTemplate.runningExecutable, pinnedDoorway);
      await chmod(pinnedDoorway, 0o700);
      assert.equal(
        await sha256(pinnedDoorway),
        aTemplate.executableHash,
        'pinned doorway bytes changed'
      );
      aTemplate.executable = pinnedDoorway;
      bTemplate.executable = pinnedDoorway;
      const mongoExe = mongoTemplate.executable;
      const [mongoPort, aPort, bPort, aHealth, bHealth] = await Promise.all(
        Array.from({ length: 5 }, async () => await freePort())
      );
      const mongoLog = join(root, 'mongod.log');
      const mongoStarted = await startOwnedChild(
        mongoExe,
        [mongoExe, '--dbpath', mongoDir, '--bind_ip', '127.0.0.1', '--port', String(mongoPort)],
        process.env,
        root,
        mongoLog
      );
      acquired.push({ handle: mongoStarted.handle, label: 'partial fixture mongod' });
      const started: Partial<Record<DoorwayName, OwnedDoorway>> = {};
      await waitForPort(mongoPort, 20_000);
      const nonce = randomBytes(6).toString('hex');
      const specs = [
        ['a', aTemplate, aPort, aHealth, 'matthew', ['jessica', 'james']],
        ['b', bTemplate, bPort, bHealth, 'jessica', ['matthew', 'james']],
      ] as const;
      for (const [name, template, port, healthPort, primary, extras] of specs) {
        const id = `epr-deliverability-${nonce}-${name}`;
        const database = `epr-deliverability-${nonce}-${name}`;
        const launch = fixtureDoorwayLaunch(template, {
          id,
          listenPort: port,
          healthPort,
          mongoPort,
          database,
          primaryUrl: urls[primary] as string,
          extraUrls: extras.map(peer => urls[peer] as string),
          scenarioDir: root,
        });
        const logPath = join(root, `doorway-${name}.log`);
        const process = await startOwnedChild(
          template.executable,
          launch.argv,
          launch.env,
          template.cwd,
          logPath
        );
        const doorway: OwnedDoorway = {
          name,
          id,
          url: `http://127.0.0.1:${port}`,
          healthPort,
          database,
          primaryPeer: primary,
          ...process,
          template,
          argv: launch.argv,
          env: launch.env,
          logPath,
        };
        started[name] = doorway;
        acquired.unshift({ handle: doorway.handle, label: `partial fixture doorway ${name}` });
        await waitFor(`${doorway.url}/health`, 60_000);
        const coherence = (await (
          await waitFor(`${doorway.url}/api/v1/federation/coherence`, 10_000)
        ).json()) as {
          doorwayId?: string;
        };
        assert.equal(coherence.doorwayId, id, `${name}: fixture doorway identity drifted`);
      }
      return new OwnedDoorwayPair(
        root,
        mongoStarted.handle,
        mongoLog,
        started as Record<DoorwayName, OwnedDoorway>,
        Object.fromEntries(
          names.map(name => {
            const peer = options.storagePeers[name] ?? {};
            return [
              name,
              {
                url: peer.url as string,
                ...(peer.agentPubKey && { agentPubKey: peer.agentPubKey }),
                ...(peer.conductorAdminPort && { conductorAdminPort: peer.conductorAdminPort }),
                ...(peer.conductorAppUrl && { conductorAppUrl: peer.conductorAppUrl }),
              },
            ];
          })
        )
      );
    } catch (error) {
      const cleanupFailures: unknown[] = [];
      try {
        await stopFixtureProcesses(acquired);
      } catch (cleanupError) {
        cleanupFailures.push(cleanupError);
      }
      try {
        await rm(root, { recursive: true, force: true });
      } catch (cleanupError) {
        cleanupFailures.push(cleanupError);
      }
      if (cleanupFailures.length)
        throw new AggregateError(
          [error, ...cleanupFailures],
          'doorway fixture startup and cleanup failed'
        );
      throw error;
    }
  }

  url(name: DoorwayName): string {
    return this.doorways[name].url;
  }

  incarnations(): string[] {
    return [this.doorways.a.handle.ticks, this.doorways.b.handle.ticks];
  }

  log(name: DoorwayName): string {
    try {
      return readFileSync(this.doorways[name].logPath, 'utf8');
    } catch {
      return '';
    }
  }

  async restart(name: DoorwayName, extraSsrSlug?: string): Promise<void> {
    const doorway = this.doorways[name];
    await stopOwnedProcess(doorway.handle, `fixture doorway ${name}`);
    if (extraSsrSlug) {
      doorway.env = withExtraSsrSlug(doorway.env, extraSsrSlug);
    }
    assert.equal(await sha256(doorway.template.executable), doorway.template.executableHash);
    const next = await startOwnedChild(
      doorway.template.executable,
      doorway.argv,
      doorway.env,
      doorway.template.cwd,
      doorway.logPath
    );
    doorway.child = next.child;
    doorway.handle = next.handle;
    await waitFor(`${doorway.url}/health`, 60_000);
  }

  receipt(): OwnedDoorwayPairReceipt {
    return {
      urls: { a: this.doorways.a.url, b: this.doorways.b.url },
      ids: { a: this.doorways.a.id, b: this.doorways.b.id },
      primaryPeers: { a: this.doorways.a.primaryPeer, b: this.doorways.b.primaryPeer },
      storageTopology: this.storageTopology,
      executableHash: this.doorways.a.template.executableHash,
      lifecycle: this.lifecycle,
      logs: {
        a: this.doorways.a.logPath,
        b: this.doorways.b.logPath,
        mongo: this.mongoLogPath,
      },
    };
  }

  async close(): Promise<void> {
    try {
      await stopFixtureProcesses([
        { handle: this.doorways.b.handle, label: 'fixture doorway b' },
        { handle: this.doorways.a.handle, label: 'fixture doorway a' },
        { handle: this.mongo, label: 'fixture mongod' },
      ]);
      this.lifecycle = 'cleaned';
      const archive = process.env['A2O_FIXTURE_DIR'] ?? join(process.cwd(), 'reports', 'fixtures');
      await mkdir(archive, { recursive: true });
      for (const doorway of Object.values(this.doorways)) {
        const target = join(archive, `${doorway.id}.log`);
        await copyFile(doorway.logPath, target);
        doorway.logPath = target;
      }
      const mongoTarget = join(archive, `${this.doorways.a.id}-mongod.log`);
      await copyFile(this.mongoLogPath, mongoTarget);
      this.mongoLogPath = mongoTarget;
      await rm(this.root, { recursive: true, force: true });
    } catch (error) {
      this.lifecycle = 'cleanup-failed';
      throw error;
    }
  }
}
