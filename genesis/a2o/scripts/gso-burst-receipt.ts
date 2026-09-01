/**
 * Household T2 receipt for the vendored iroh-quinn GSO fix.
 *
 * The measured fleet failure was an iroh-only peer sending a first-contact
 * view-federation burst to an iroh-capable storage and disappearing. The old
 * iroh-quinn tail-loss probe could then panic in a destructor and abort the
 * whole receiver. This probe recreates that process shape on the owned mesh:
 * restart an iroh-only requester, count its transport-neutral reconcile RPCs,
 * SIGKILL it at the burst threshold, and prove the receiver still serves.
 *
 * Run from genesis/a2o:
 *   pnpm exec tsx scripts/gso-burst-receipt.ts
 *
 * Start the required mixed mesh with, for example:
 *   MESH_PEER_TRANSPORTS=matthew=dual,jessica=iroh,james=dual just mesh start
 */

import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { once } from 'node:events';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  processAlive,
  readProcEnvironment,
  requireSinglePidByArgv,
  storagePeerArgvMatch,
  writeRestartCapture,
} from '../src/framework/fixtures/process-control.js';
import {
  DESTRUCTIVE_HELD_HINT,
  destructiveAllowed,
} from '../src/framework/fixtures/substrate-scope.js';

const REPO_ROOT = fileURLToPath(new URL('../../..', import.meta.url));
const DEFAULT_MESH_DIR = process.env['MESH_DIR'] ?? path.join(tmpdir(), 'elohim-local-mesh');
const DEFAULT_MINIMUM_EXCHANGES = 200;
const DEFAULT_BURST_TIMEOUT_MS = 6 * 60_000;
const DEFAULT_REQUEST_TIMEOUT_MS = 5_000;
const POLL_MS = 500;
const BASH_BIN = '/bin/bash';
const FLOCK_BIN = '/usr/bin/flock';
const LEG_PRECONDITION = 'precondition';
const LEG_REMOTE_RESTART = 'remote-restart';
const MESH_STORAGE_RESTART = 'storage-restart';
const REQUIRED_REMOTE_TRANSPORT = 'iroh';

const USAGE = `Usage: gso-burst-receipt.ts [options]

Options:
  --survivor <name=url>       receiving peer (default: matthew=http://localhost:8090)
  --remote <name=url>         restarted/SIGKILLed requester (default: jessica=http://localhost:8091)
  --minimum <count>           required view-federation exchanges (default: ${DEFAULT_MINIMUM_EXCHANGES})
  --burst-timeout <ms>        wait for the burst threshold (default: ${DEFAULT_BURST_TIMEOUT_MS})
  --request-timeout <ms>      per-HTTP-request timeout (default: ${DEFAULT_REQUEST_TIMEOUT_MS})
  --mesh-dir <path>           mesh state/log directory (default: ${DEFAULT_MESH_DIR})
  --json                      emit the final receipt as JSON (progress stays on stderr)
  -h, --help                  show this help

The remote must be iroh-only and the survivor must be iroh or dual. This is
intentional: an all-dual pair normally satisfies reconcile over libp2p first
and therefore does not exercise the iroh-quinn GSO path.`;

interface PeerTarget {
  name: string;
  url: string;
  port: number;
}

interface Options {
  survivor: PeerTarget;
  remote: PeerTarget;
  minimum: number;
  burstTimeoutMs: number;
  requestTimeoutMs: number;
  meshDir: string;
  json: boolean;
  help: boolean;
}

interface ExchangeCounters {
  inventoryPages: number;
  headRecordVerifies: number;
  headRecordPresent: number;
  headRecordAbsent: number;
  headRecordUnreachable: number;
  total: number;
}

interface Receipt {
  runId: string;
  startedAt: string;
  finishedAt: string;
  survivor: {
    name: string;
    url: string;
    transport: string;
    pid: number;
    pidUnchanged: boolean;
    postDisconnectStatus: number;
    panicLines: string[];
  };
  remote: {
    name: string;
    url: string;
    transport: string;
    pid: number;
    signal: 'SIGKILL';
    pidGone: boolean;
    restored: boolean;
  };
  burst: ExchangeCounters & {
    minimum: number;
    elapsedMs: number;
  };
  outcome: 'passed';
}

interface RunningPeers {
  survivorPid: number;
  survivorTransport: string;
  survivorLogOffset: number;
  oldRemotePid: number;
  oldRemoteTransport: string;
}

interface BurstMeasurement {
  remotePid: number;
  remoteTransport: string;
  counters: ExchangeCounters;
  elapsedMs: number;
}

class ProbeFailure extends Error {
  constructor(
    readonly leg: string,
    message: string
  ) {
    super(message);
  }
}

function requiredValue(args: string[], index: number, flag: string): string {
  const value = args[index + 1];
  if (!value || value.startsWith('--'))
    throw new ProbeFailure('arguments', `${flag} requires a value`);
  return value;
}

function positiveInteger(value: string, flag: string): number {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < 1) {
    throw new ProbeFailure('arguments', `${flag} expects a positive integer, got: ${value}`);
  }
  return parsed;
}

function parsePeer(value: string, flag: string): PeerTarget {
  const separator = value.indexOf('=');
  if (separator < 1 || separator === value.length - 1) {
    throw new ProbeFailure(
      'arguments',
      `${flag} expects name=http://localhost:port, got: ${value}`
    );
  }
  const name = value.slice(0, separator).trim();
  const url = new URL(value.slice(separator + 1).trim());
  if (url.protocol !== 'http:') {
    throw new ProbeFailure(LEG_PRECONDITION, `${flag} must use local HTTP, got: ${url.protocol}`);
  }
  if (!['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)) {
    throw new ProbeFailure(
      LEG_PRECONDITION,
      `${flag} must address the owned loopback mesh, got host: ${url.hostname}`
    );
  }
  const port = Number(url.port);
  if (!Number.isInteger(port) || port < 1 || port > 65_535) {
    throw new ProbeFailure(
      'arguments',
      `${flag} URL needs an explicit TCP port, got: ${url.toString()}`
    );
  }
  return { name, url: url.toString().replace(/\/$/, ''), port };
}

function parseArgs(argv: string[]): Options {
  let survivor = parsePeer('matthew=http://localhost:8090', '--survivor');
  let remote = parsePeer('jessica=http://localhost:8091', '--remote');
  let minimum = DEFAULT_MINIMUM_EXCHANGES;
  let burstTimeoutMs = DEFAULT_BURST_TIMEOUT_MS;
  let requestTimeoutMs = DEFAULT_REQUEST_TIMEOUT_MS;
  let meshDir = DEFAULT_MESH_DIR;
  let json = false;
  let help = false;

  for (let index = 0; index < argv.length; index++) {
    const arg = argv[index];
    switch (arg) {
      case '--survivor':
        survivor = parsePeer(requiredValue(argv, index, arg), arg);
        index++;
        break;
      case '--remote':
        remote = parsePeer(requiredValue(argv, index, arg), arg);
        index++;
        break;
      case '--minimum':
        minimum = positiveInteger(requiredValue(argv, index, arg), arg);
        index++;
        break;
      case '--burst-timeout':
        burstTimeoutMs = positiveInteger(requiredValue(argv, index, arg), arg);
        index++;
        break;
      case '--request-timeout':
        requestTimeoutMs = positiveInteger(requiredValue(argv, index, arg), arg);
        index++;
        break;
      case '--mesh-dir':
        meshDir = path.resolve(requiredValue(argv, index, arg));
        index++;
        break;
      case '--json':
        json = true;
        break;
      case '-h':
      case '--help':
        help = true;
        break;
      default:
        throw new ProbeFailure('arguments', `unknown option: ${arg}`);
    }
  }

  if (survivor.name === remote.name || survivor.port === remote.port) {
    throw new ProbeFailure(LEG_PRECONDITION, 'survivor and remote must be different mesh peers');
  }
  return { survivor, remote, minimum, burstTimeoutMs, requestTimeoutMs, meshDir, json, help };
}

function progress(options: Options, message: string): void {
  if (options.json) console.error(message);
  else console.log(message);
}

async function sleep(ms: number): Promise<void> {
  await new Promise(resolve => setTimeout(resolve, ms));
}

async function waitUntil(test: () => boolean, timeoutMs: number): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (test()) return true;
    await sleep(100);
  }
  return test();
}

async function fetchText(
  url: string,
  timeoutMs: number
): Promise<{ status: number; body: string }> {
  const response = await fetch(url, { signal: AbortSignal.timeout(timeoutMs) });
  return { status: response.status, body: await response.text() };
}

function histogramCount(metrics: string, atom: string): number {
  let total = 0;
  for (const line of metrics.split('\n')) {
    if (!line.startsWith('elohim_atom_duration_ms_count{')) continue;
    if (!line.includes(`atom="${atom}"`)) continue;
    const value = Number(line.trim().split(/\s+/).at(-1));
    if (Number.isFinite(value)) total += value;
  }
  return total;
}

function labeledCounter(metrics: string, metric: string, label: string, value: string): number {
  let total = 0;
  for (const line of metrics.split('\n')) {
    if (!line.startsWith(`${metric}{`)) continue;
    if (!line.includes(`${label}="${value}"`)) continue;
    const parsed = Number(line.trim().split(/\s+/).at(-1));
    if (Number.isFinite(parsed)) total += parsed;
  }
  return total;
}

function exchangeCounters(metrics: string): ExchangeCounters {
  const inventoryPages = histogramCount(metrics, 'inventory_page');
  const headRecordVerifies = histogramCount(metrics, 'head_record_verify');
  const headRecordPresent = labeledCounter(
    metrics,
    'elohim_content_head_record_fetch_total',
    'state',
    'present'
  );
  const headRecordAbsent = labeledCounter(
    metrics,
    'elohim_content_head_record_fetch_total',
    'state',
    'absent'
  );
  const headRecordUnreachable = labeledCounter(
    metrics,
    'elohim_content_head_record_fetch_total',
    'state',
    'unreachable'
  );
  return {
    inventoryPages,
    headRecordVerifies,
    headRecordPresent,
    headRecordAbsent,
    headRecordUnreachable,
    // Only decoded replies count toward the burst threshold. The duration
    // histograms above include timeouts and stay diagnostic rather than proof.
    total: headRecordPresent + headRecordAbsent,
  };
}

function transportForPid(pid: number): string {
  return readProcEnvironment(pid)['ELOHIM_TRANSPORT_BACKEND'] ?? 'libp2p';
}

function storagePid(port: number, leg: string): number {
  try {
    return requireSinglePidByArgv(storagePeerArgvMatch(String(port)));
  } catch (error) {
    throw new ProbeFailure(
      leg,
      `cannot resolve exactly one storage on port ${port}: ${String(error)}`
    );
  }
}

function survivorLogTail(logPath: string, byteOffset: number): string {
  const bytes = readFileSync(logPath);
  return bytes.subarray(bytes.length >= byteOffset ? byteOffset : 0).toString('utf8');
}

function panicLines(logTail: string): string[] {
  const panicPattern =
    /assertion failed: untracked_bytes <= segment_size|panicked at|panic in a destructor|thread caused non-unwinding panic|fatal runtime error|sigabrt|aborted \(core dumped\)/i;
  return logTail
    .split('\n')
    .filter(line => panicPattern.test(line))
    .slice(0, 20);
}

async function runMeshCommand(options: Options, action: string, peer: string): Promise<void> {
  const script = path.join(REPO_ROOT, 'app/elohim-app/scripts/hc-mesh.sh');
  const child = spawn(BASH_BIN, [script, action, peer], {
    cwd: REPO_ROOT,
    env: {
      ...process.env,
      MESH_DIR: options.meshDir,
      // storage-restart deliberately overlays its caller's peer selection on
      // the captured daemon environment. Preserve the receipt's iroh-only
      // requester on both the pre-burst restart and post-kill restore.
      MESH_PEER_TRANSPORTS: `${peer}=${REQUIRED_REMOTE_TRANSPORT}`,
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let stdout = '';
  let stderr = '';
  child.stdout.on('data', chunk => (stdout += String(chunk)));
  child.stderr.on('data', chunk => (stderr += String(chunk)));
  // hc-mesh's own health window is 180s; leave room for its post-health zome
  // probe so this wrapper reports the harness result instead of racing it.
  const timeout = setTimeout(() => child.kill('SIGKILL'), 4 * 60_000);
  const [code, signal] = (await once(child, 'exit')) as [number | null, NodeJS.Signals | null];
  clearTimeout(timeout);
  for (const line of `${stdout}${stderr}`.trim().split('\n').filter(Boolean)) {
    progress(options, `mesh: ${line}`);
  }
  if (code !== 0) {
    throw new ProbeFailure(
      LEG_REMOTE_RESTART,
      `${path.basename(script)} ${action} ${peer} exited ${String(code)} signal=${String(signal)}`
    );
  }
}

async function acquireMeshLock(meshDir: string): Promise<ChildProcessWithoutNullStreams> {
  const lockPath = path.join(meshDir, 'a2o.lock');
  const child = spawn(
    FLOCK_BIN,
    ['-n', lockPath, BASH_BIN, '-c', String.raw`printf "locked\n"; read -r _`],
    { stdio: 'pipe' }
  );
  const acquired = await Promise.race([
    once(child.stdout, 'data').then(([chunk]) => String(chunk).includes('locked')),
    once(child, 'exit').then(() => false),
    sleep(2_000).then(() => false),
  ]);
  if (!acquired) {
    child.stdin.end();
    throw new ProbeFailure(
      'mesh-lock',
      `${lockPath} is held by another mesh run; refusing to overlap destructive probes`
    );
  }
  return child;
}

function inspectRunningPeers(options: Options, survivorLog: string): RunningPeers {
  const survivorPid = storagePid(options.survivor.port, LEG_PRECONDITION);
  const oldRemotePid = storagePid(options.remote.port, LEG_PRECONDITION);
  const survivorTransport = transportForPid(survivorPid);
  const oldRemoteTransport = transportForPid(oldRemotePid);
  if (!['dual', 'iroh'].includes(survivorTransport)) {
    throw new ProbeFailure(
      LEG_PRECONDITION,
      `survivor ${options.survivor.name} transport=${survivorTransport}; expected dual or iroh`
    );
  }
  if (oldRemoteTransport !== REQUIRED_REMOTE_TRANSPORT) {
    throw new ProbeFailure(
      LEG_PRECONDITION,
      `remote ${options.remote.name} transport=${oldRemoteTransport}; expected iroh-only so the ` +
        'view-federation burst cannot fall through the libp2p-first leg'
    );
  }
  return {
    survivorPid,
    survivorTransport,
    survivorLogOffset: statSync(survivorLog).size,
    oldRemotePid,
    oldRemoteTransport,
  };
}

async function measureBurstAfterRestart(
  options: Options,
  peers: RunningPeers
): Promise<BurstMeasurement> {
  await runMeshCommand(options, MESH_STORAGE_RESTART, options.remote.name);
  const remotePid = storagePid(options.remote.port, LEG_REMOTE_RESTART);
  if (remotePid === peers.oldRemotePid) {
    throw new ProbeFailure(
      LEG_REMOTE_RESTART,
      `remote PID did not change across storage-restart (${remotePid})`
    );
  }
  const remoteTransport = transportForPid(remotePid);
  if (remoteTransport !== REQUIRED_REMOTE_TRANSPORT) {
    throw new ProbeFailure(
      LEG_PRECONDITION,
      `restarted remote transport=${remoteTransport}; expected captured iroh-only posture`
    );
  }

  const burstStarted = Date.now();
  let counters: ExchangeCounters = {
    inventoryPages: 0,
    headRecordVerifies: 0,
    headRecordPresent: 0,
    headRecordAbsent: 0,
    headRecordUnreachable: 0,
    total: 0,
  };
  let lastPrinted = -1;
  while (Date.now() - burstStarted < options.burstTimeoutMs) {
    if (!processAlive(remotePid)) {
      throw new ProbeFailure(
        'no-burst',
        `remote ${options.remote.name} died before the probe sent SIGKILL (count=${counters.total})`
      );
    }
    const metrics = await fetchText(`${options.remote.url}/metrics`, options.requestTimeoutMs);
    if (metrics.status !== 200) {
      throw new ProbeFailure('no-burst', `remote /metrics returned ${metrics.status}`);
    }
    counters = exchangeCounters(metrics.body);
    if (
      counters.total !== lastPrinted &&
      (counters.total >= options.minimum || counters.total % 25 === 0)
    ) {
      progress(
        options,
        `BURST successful=${counters.total} present=${counters.headRecordPresent} ` +
          `absent=${counters.headRecordAbsent} unreachable=${counters.headRecordUnreachable} ` +
          `inventory_attempts=${counters.inventoryPages} ` +
          `head_record_attempts=${counters.headRecordVerifies} target=${options.minimum}`
      );
      lastPrinted = counters.total;
    }
    if (counters.total >= options.minimum) break;
    await sleep(POLL_MS);
  }
  if (counters.total < options.minimum) {
    throw new ProbeFailure(
      'no-burst',
      `view-federation count reached ${counters.total}, below ${options.minimum}, within ` +
        `${options.burstTimeoutMs}ms; missing fixture station: mesh-harness -> burst-regime`
    );
  }
  return {
    remotePid,
    remoteTransport,
    counters,
    elapsedMs: Date.now() - burstStarted,
  };
}

async function assertSurvivorAfterDisconnect(
  options: Options,
  peers: RunningPeers,
  survivorLog: string
): Promise<{ status: number; panics: string[] }> {
  await sleep(1_000);
  const survivorStillPid = storagePid(options.survivor.port, 'survivor-dead');
  if (!processAlive(peers.survivorPid) || survivorStillPid !== peers.survivorPid) {
    throw new ProbeFailure(
      'survivor-dead',
      `survivor PID changed/died: before=${peers.survivorPid} after=${survivorStillPid}`
    );
  }
  const post = await fetchText(`${options.survivor.url}/p2p/status`, options.requestTimeoutMs);
  if (post.status !== 200) {
    throw new ProbeFailure(
      'post-disconnect-dead',
      `survivor /p2p/status returned ${post.status}: ${post.body.slice(0, 240)}`
    );
  }
  await sleep(1_000);
  const panics = panicLines(survivorLogTail(survivorLog, peers.survivorLogOffset));
  if (panics.length > 0) {
    throw new ProbeFailure('survivor-log-panic', panics.join('\n'));
  }
  return { status: post.status, panics };
}

async function run(options: Options): Promise<Receipt> {
  const runId = `gso-burst-${new Date().toISOString().replace(/[:.]/g, '-')}`;
  const startedAt = new Date().toISOString();
  const survivorLog = path.join(options.meshDir, 'logs', `${options.survivor.name}.log`);
  if (!existsSync(survivorLog)) {
    throw new ProbeFailure(LEG_PRECONDITION, `survivor log is absent: ${survivorLog}`);
  }

  // This executable is a household-only probe. Select that declared lane when
  // the operator did not choose another; an explicit A2O_ALLOW_DESTRUCTIVE=0
  // still wins inside destructiveAllowed().
  process.env['ELOHIM_CLUSTER_STATE_PATH_OVERRIDE'] ??= path.join(
    REPO_ROOT,
    'genesis/manifests/cluster-state.act1-household.yaml'
  );
  if (!destructiveAllowed()) {
    throw new ProbeFailure('destructive-gate', DESTRUCTIVE_HELD_HINT);
  }

  const lock = await acquireMeshLock(options.meshDir);
  let remoteKilled = false;
  let remoteRestored = false;
  let receipt: Receipt | undefined;
  let runFailure: unknown;
  try {
    const peers = inspectRunningPeers(options, survivorLog);

    progress(
      options,
      `PRECONDITION survivor=${options.survivor.name} pid=${peers.survivorPid} ` +
        `transport=${peers.survivorTransport} remote=${options.remote.name} ` +
        `pid=${peers.oldRemotePid} transport=${peers.oldRemoteTransport}`
    );
    const burst = await measureBurstAfterRestart(options, peers);

    writeRestartCapture(
      path.join(options.meshDir, 'storage-restart'),
      options.remote.name,
      burst.remotePid
    );
    process.kill(burst.remotePid, 'SIGKILL');
    remoteKilled = true;
    const pidGone = await waitUntil(() => !processAlive(burst.remotePid), 15_000);
    progress(
      options,
      `REMOTE-KILLED name=${options.remote.name} pid=${burst.remotePid} signal=SIGKILL gone=${pidGone}`
    );
    if (!pidGone) {
      throw new ProbeFailure(
        'remote-not-killed',
        `remote PID ${burst.remotePid} still exists after SIGKILL`
      );
    }

    const survivor = await assertSurvivorAfterDisconnect(options, peers, survivorLog);
    progress(
      options,
      `SURVIVOR-LIVE name=${options.survivor.name} pid=${peers.survivorPid} unchanged=true ` +
        `postDisconnectStatus=${survivor.status} panicLines=0`
    );

    receipt = {
      runId,
      startedAt,
      finishedAt: '',
      survivor: {
        name: options.survivor.name,
        url: options.survivor.url,
        transport: peers.survivorTransport,
        pid: peers.survivorPid,
        pidUnchanged: true,
        postDisconnectStatus: survivor.status,
        panicLines: survivor.panics,
      },
      remote: {
        name: options.remote.name,
        url: options.remote.url,
        transport: burst.remoteTransport,
        pid: burst.remotePid,
        signal: 'SIGKILL',
        pidGone: true,
        restored: false,
      },
      burst: {
        ...burst.counters,
        minimum: options.minimum,
        elapsedMs: burst.elapsedMs,
      },
      outcome: 'passed',
    };
  } catch (error) {
    runFailure = error;
  } finally {
    if (remoteKilled) {
      try {
        await runMeshCommand(options, MESH_STORAGE_RESTART, options.remote.name);
        remoteRestored = true;
      } catch (error) {
        runFailure ??= error;
      }
    }
    lock.stdin.end();
    await Promise.race([once(lock, 'exit'), sleep(2_000)]);
  }

  if (runFailure) throw runFailure;
  if (!receipt) throw new ProbeFailure('internal', 'probe completed without a receipt');
  receipt.remote.restored = remoteRestored;
  receipt.finishedAt = new Date().toISOString();
  return receipt;
}

async function main(): Promise<void> {
  let options: Options | undefined;
  try {
    options = parseArgs(process.argv.slice(2));
    if (options.help) {
      console.log(USAGE);
      return;
    }
    const receipt = await run(options);
    if (options.json) console.log(JSON.stringify(receipt, null, 2));
    else {
      console.log(
        `PASS runId=${receipt.runId} exchanges=${receipt.burst.total} ` +
          `remoteRestored=${receipt.remote.restored}`
      );
    }
  } catch (error) {
    const failure =
      error instanceof ProbeFailure ? error : new ProbeFailure('internal', String(error));
    const line = `FAIL leg=${failure.leg}: ${failure.message}`;
    if (options?.json) {
      console.log(
        JSON.stringify({ outcome: 'failed', leg: failure.leg, summary: failure.message })
      );
    } else console.error(line);
    process.exitCode = 2;
  }
}

await main();
