/**
 * Household receipt for organic late-join discovery on an already-warm mesh.
 *
 * The proof is conjunctive: each incumbent starts with a non-empty iroh book,
 * one new peer is staged without touching incumbents, the joiner's exact signed
 * NodeId reaches the doorway board, and every unchanged incumbent process then
 * exposes that NodeId after its recurring watch counter advances.
 *
 * Run from genesis/a2o:
 *   pnpm exec tsx scripts/late-joiner-receipt.ts
 */

import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { once } from 'node:events';
import { readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { discoverStoragePeerPid } from '../src/framework/fixtures/process-control.js';

const REPO_ROOT = fileURLToPath(new URL('../../..', import.meta.url));
const DEFAULT_PEERS =
  'matthew=http://localhost:8090,jessica=http://localhost:8091,james=http://localhost:8092';
const DEFAULT_DOORWAY = 'http://localhost:8888';
const DEFAULT_DEADLINE_MS = 95_000;
const DEFAULT_POLL_MS = 2_000;
const DEFAULT_REQUEST_TIMEOUT_MS = 5_000;
const DEFAULT_MESH_DIR = process.env['MESH_DIR'] ?? path.join(tmpdir(), 'elohim-local-mesh');
const DEFAULT_MESH_SCRIPT = path.join(REPO_ROOT, 'app/elohim-app/scripts/hc-mesh.sh');
const BASH_BIN = '/bin/bash';
const FLOCK_BIN = '/usr/bin/flock';

const USAGE = `Usage: late-joiner-receipt.ts [options]

Options:
  --joiner <name>          fresh peer name (default: generated per run)
  --peers <csv>            incumbents as name=http://localhost:port
  --doorway <url>          manifest-board base (default: ${DEFAULT_DOORWAY})
  --deadline-secs <n>      discovery budget (default: ${DEFAULT_DEADLINE_MS / 1_000})
  --poll-ms <n>            poll interval (default: ${DEFAULT_POLL_MS})
  --request-timeout <ms>   per-request timeout (default: ${DEFAULT_REQUEST_TIMEOUT_MS})
  --mesh-script <path>     hc-mesh.sh path (default: repository script)
  -h, --help               show this help

The mesh must already be running and warm. This command stages the joiner by
calling hc-mesh.sh join-peer directly; the root justfile intentionally remains
unchanged because its mesh action map is an explicit, separately-owned surface.`;

interface PeerTarget {
  name: string;
  url: string;
  port: number;
}

interface Options {
  joiner: string;
  peers: PeerTarget[];
  doorway: string;
  deadlineMs: number;
  pollMs: number;
  requestTimeoutMs: number;
  meshScript: string;
  help: boolean;
}

interface IrohPeerObservation {
  nodeId: string;
  userAgent?: string | null;
}

interface PeerObservation {
  peerId: string;
  irohNodeId: string;
  irohPeers: IrohPeerObservation[];
  peersKnown: number;
  watchReads: number;
  watchSeeded: number;
}

interface Baseline {
  target: PeerTarget;
  pid: number;
  processStartTicks: string;
  observation: PeerObservation;
}

interface JoinedPeer {
  name: string;
  index: number;
  url: string;
  nodeId: string;
}

interface Discovery {
  elapsedMs: number;
  observation: PeerObservation;
}

interface PollResult {
  boardSeenMs?: number;
  discoveries: Map<string, Discovery>;
  lastErrors: Map<string, string>;
}

class UsageError extends Error {}

class ReceiptIncomplete extends Error {
  constructor(
    message: string,
    readonly missingPeers: string[] = []
  ) {
    super(message);
  }
}

function positiveInteger(value: string, flag: string): number {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < 1) {
    throw new UsageError(`${flag} expects a positive integer, got: ${value}`);
  }
  return parsed;
}

function requiredValue(args: string[], index: number, flag: string): string {
  const value = args[index + 1];
  if (!value || value.startsWith('--')) throw new UsageError(`${flag} requires a value`);
  return value;
}

function localPeer(name: string, rawUrl: string): PeerTarget {
  const url = new URL(rawUrl);
  if (url.protocol !== 'http:' || !['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)) {
    throw new UsageError(`peer ${name} must address the owned loopback HTTP mesh: ${rawUrl}`);
  }
  const port = Number(url.port);
  if (!Number.isInteger(port) || port < 1 || port > 65_535) {
    throw new UsageError(`peer ${name} needs an explicit TCP port: ${rawUrl}`);
  }
  return { name, url: url.toString().replace(/\/$/, ''), port };
}

function parsePeers(csv: string): PeerTarget[] {
  const peers = csv
    .split(',')
    .map(value => value.trim())
    .filter(Boolean)
    .map(value => {
      const separator = value.indexOf('=');
      if (separator < 1 || separator === value.length - 1) {
        throw new UsageError(`peer entry must be name=http://localhost:port, got: ${value}`);
      }
      return localPeer(value.slice(0, separator), value.slice(separator + 1));
    });
  if (peers.length === 0) throw new UsageError('the incumbent peer list is empty');
  const names = new Set<string>();
  const ports = new Set<number>();
  for (const peer of peers) {
    if (names.has(peer.name)) throw new UsageError(`duplicate peer name: ${peer.name}`);
    if (ports.has(peer.port)) throw new UsageError(`duplicate peer port: ${peer.port}`);
    names.add(peer.name);
    ports.add(peer.port);
  }
  return peers;
}

function parseHttpUrl(rawUrl: string, flag: string): string {
  const url = new URL(rawUrl);
  if (url.protocol !== 'http:' && url.protocol !== 'https:') {
    throw new UsageError(`${flag} expects an HTTP(S) URL, got: ${rawUrl}`);
  }
  return url.toString().replace(/\/$/, '');
}

function parseArgs(argv: string[]): Options {
  let joiner = `latejoin-${Date.now().toString(36)}`;
  let peers = parsePeers(process.env['PEER_STORAGE_URLS'] ?? DEFAULT_PEERS);
  let doorway = parseHttpUrl(process.env['DOORWAY_URL'] ?? DEFAULT_DOORWAY, '--doorway');
  let deadlineMs = DEFAULT_DEADLINE_MS;
  let pollMs = DEFAULT_POLL_MS;
  let requestTimeoutMs = DEFAULT_REQUEST_TIMEOUT_MS;
  let meshScript = DEFAULT_MESH_SCRIPT;
  let help = false;

  for (let index = 0; index < argv.length; index++) {
    const arg = argv[index];
    switch (arg) {
      case '--joiner':
        joiner = requiredValue(argv, index, arg);
        index++;
        break;
      case '--peers':
        peers = parsePeers(requiredValue(argv, index, arg));
        index++;
        break;
      case '--doorway':
        doorway = parseHttpUrl(requiredValue(argv, index, arg), arg);
        index++;
        break;
      case '--deadline-secs':
        deadlineMs = positiveInteger(requiredValue(argv, index, arg), arg) * 1_000;
        index++;
        break;
      case '--poll-ms':
        pollMs = positiveInteger(requiredValue(argv, index, arg), arg);
        index++;
        break;
      case '--request-timeout':
        requestTimeoutMs = positiveInteger(requiredValue(argv, index, arg), arg);
        index++;
        break;
      case '--mesh-script':
        meshScript = path.resolve(requiredValue(argv, index, arg));
        index++;
        break;
      case '-h':
      case '--help':
        help = true;
        break;
      default:
        throw new UsageError(`unknown option: ${arg}`);
    }
  }

  if (!/^[A-Za-z0-9][A-Za-z0-9_-]{0,47}$/.test(joiner)) {
    throw new UsageError(`invalid --joiner name: ${joiner}`);
  }
  return { joiner, peers, doorway, deadlineMs, pollMs, requestTimeoutMs, meshScript, help };
}

async function sleep(ms: number): Promise<void> {
  await new Promise(resolve => setTimeout(resolve, ms));
}

async function fetchText(url: string, timeoutMs: number): Promise<string> {
  const response = await fetch(url, { signal: AbortSignal.timeout(timeoutMs) });
  if (!response.ok) throw new ReceiptIncomplete(`${url} answered HTTP ${response.status}`);
  return response.text();
}

async function fetchJson(url: string, timeoutMs: number): Promise<Record<string, unknown>> {
  const response = await fetch(url, {
    headers: { accept: 'application/json' },
    signal: AbortSignal.timeout(timeoutMs),
  });
  if (!response.ok) throw new ReceiptIncomplete(`${url} answered HTTP ${response.status}`);
  const value: unknown = await response.json();
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new ReceiptIncomplete(`${url} did not return a JSON object`);
  }
  return value as Record<string, unknown>;
}

function metricValue(metrics: string, name: string, labels: Record<string, string> = {}): number {
  for (const line of metrics.split('\n')) {
    if (!line.startsWith(name)) continue;
    if (!Object.entries(labels).every(([key, value]) => line.includes(`${key}="${value}"`))) {
      continue;
    }
    const parsed = Number(line.trim().split(/\s+/).at(-1));
    if (Number.isFinite(parsed)) return parsed;
  }
  throw new ReceiptIncomplete(
    `metrics omitted ${name}${Object.keys(labels).length > 0 ? JSON.stringify(labels) : ''}`
  );
}

function watchReadTotal(metrics: string): number {
  let found = false;
  let total = 0;
  for (const line of metrics.split('\n')) {
    if (!line.startsWith('elohim_iroh_doorway_bootstrap_reads_total{')) continue;
    if (!line.includes('phase="watch"')) continue;
    const parsed = Number(line.trim().split(/\s+/).at(-1));
    if (!Number.isFinite(parsed)) continue;
    found = true;
    total += parsed;
  }
  if (!found) {
    throw new ReceiptIncomplete('metrics omitted every phase="watch" doorway-bootstrap counter');
  }
  return total;
}

function irohPeers(status: Record<string, unknown>, peerName: string): IrohPeerObservation[] {
  const value = status['irohPeers'];
  if (!Array.isArray(value)) {
    throw new ReceiptIncomplete(
      `${peerName} /p2p/status has no irohPeers array; land the additive observed-peer surface first`,
      [peerName]
    );
  }
  return value.map((entry, index) => {
    if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
      throw new ReceiptIncomplete(`${peerName} irohPeers[${index}] is not an object`, [peerName]);
    }
    const row = entry as Record<string, unknown>;
    if (typeof row['nodeId'] !== 'string' || !/^[0-9a-f]{64}$/.test(row['nodeId'])) {
      throw new ReceiptIncomplete(`${peerName} irohPeers[${index}] has no valid nodeId`, [
        peerName,
      ]);
    }
    return {
      nodeId: row['nodeId'],
      userAgent: typeof row['userAgent'] === 'string' ? row['userAgent'] : null,
    };
  });
}

async function observePeer(target: PeerTarget, timeoutMs: number): Promise<PeerObservation> {
  const [status, metrics] = await Promise.all([
    fetchJson(`${target.url}/p2p/status`, timeoutMs),
    fetchText(`${target.url}/metrics`, timeoutMs),
  ]);
  const peerId = status['peerId'];
  const irohNodeId = status['irohNodeId'];
  if (typeof peerId !== 'string' || peerId.length === 0) {
    throw new ReceiptIncomplete(`${target.name} exposes no peerId`, [target.name]);
  }
  if (typeof irohNodeId !== 'string' || !/^[0-9a-f]{64}$/.test(irohNodeId)) {
    throw new ReceiptIncomplete(`${target.name} is not a live dual/iroh observer`, [target.name]);
  }
  return {
    peerId,
    irohNodeId,
    irohPeers: irohPeers(status, target.name),
    peersKnown: metricValue(metrics, 'elohim_iroh_peers_known'),
    watchReads: watchReadTotal(metrics),
    watchSeeded: metricValue(metrics, 'elohim_iroh_doorway_bootstrap_reads_total', {
      phase: 'watch',
      result: 'seeded',
    }),
  };
}

function processStartTicks(pid: number): string {
  const stat = readFileSync(`/proc/${pid}/stat`, 'utf8');
  const close = stat.lastIndexOf(')');
  if (close < 0) throw new ReceiptIncomplete(`/proc/${pid}/stat has no command terminator`);
  const fieldsAfterComm = stat
    .slice(close + 2)
    .trim()
    .split(/\s+/);
  const ticks = fieldsAfterComm[19];
  if (!ticks) throw new ReceiptIncomplete(`/proc/${pid}/stat has no start-time field`);
  return ticks;
}

function processIdentity(target: PeerTarget): { pid: number; processStartTicks: string } {
  const pid = discoverStoragePeerPid(String(target.port));
  if (!pid) {
    throw new ReceiptIncomplete(
      `no unique local elohim-storage process owns ${target.name} :${target.port}`,
      [target.name]
    );
  }
  return { pid, processStartTicks: processStartTicks(pid) };
}

async function baseline(target: PeerTarget, timeoutMs: number): Promise<Baseline> {
  const identity = processIdentity(target);
  const observation = await observePeer(target, timeoutMs);
  if (observation.irohPeers.length === 0 || observation.peersKnown < 1) {
    throw new ReceiptIncomplete(
      `${target.name} is not warm: irohPeers=${observation.irohPeers.length} gauge=${observation.peersKnown}`,
      [target.name]
    );
  }
  return { target, ...identity, observation };
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
    throw new ReceiptIncomplete(`${lockPath} is held by another mesh-touching run`);
  }
  return child;
}

async function stageJoiner(options: Options): Promise<JoinedPeer> {
  const child = spawn(BASH_BIN, [options.meshScript, 'join-peer', options.joiner], {
    cwd: REPO_ROOT,
    env: process.env,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let transcript = '';
  const append = (chunk: unknown, error: boolean): void => {
    const text = String(chunk);
    transcript += text;
    (error ? process.stderr : process.stdout).write(text);
  };
  child.stdout.on('data', chunk => append(chunk, false));
  child.stderr.on('data', chunk => append(chunk, true));
  const timer = setTimeout(() => child.kill('SIGKILL'), 8 * 60_000);
  const [code, signal] = (await once(child, 'exit')) as [number | null, NodeJS.Signals | null];
  clearTimeout(timer);
  if (code !== 0) {
    throw new ReceiptIncomplete(
      `join-peer ${options.joiner} exited ${String(code)} signal=${String(signal)}`
    );
  }
  const match = /^JOINED_PEER name=(\S+) index=(\d+) http=(\S+) irohNodeId=([0-9a-f]{64})$/m.exec(
    transcript
  );
  if (!match) throw new ReceiptIncomplete('join-peer emitted no machine-readable JOINED_PEER line');
  return {
    name: match[1],
    index: Number(match[2]),
    url: match[3],
    nodeId: match[4],
  };
}

function manifestEntries(value: unknown): unknown[] {
  if (Array.isArray(value)) return value;
  if (!value || typeof value !== 'object') return [];
  const row = value as Record<string, unknown>;
  if (Array.isArray(row['manifests'])) return row['manifests'];
  if (Array.isArray(row['items'])) return row['items'];
  return [];
}

async function boardHasJoiner(options: Options, nodeId: string): Promise<boolean> {
  const response = await fetch(`${options.doorway}/p2p/manifests`, {
    headers: { accept: 'application/json' },
    signal: AbortSignal.timeout(options.requestTimeoutMs),
  });
  if (!response.ok) return false;
  const value: unknown = await response.json().catch(() => null);
  return manifestEntries(value).some(entry => {
    if (!entry || typeof entry !== 'object' || Array.isArray(entry)) return false;
    const row = entry as Record<string, unknown>;
    const observedNodeId = row['iroh_node_id'] ?? row['irohNodeId'];
    return observedNodeId === nodeId && typeof row['signature'] === 'string';
  });
}

function assertUnchanged(held: Baseline): void {
  const current = processIdentity(held.target);
  if (current.pid === held.pid && current.processStartTicks === held.processStartTicks) return;
  throw new ReceiptIncomplete(
    `${held.target.name} restarted during the receipt (` +
      `${held.pid}/${held.processStartTicks} -> ${current.pid}/${current.processStartTicks})`,
    [held.target.name]
  );
}

async function sampleDiscovery(
  held: Baseline,
  nodeId: string,
  stagedAt: number,
  requestTimeoutMs: number
): Promise<Discovery | undefined> {
  assertUnchanged(held);
  const observation = await observePeer(held.target, requestTimeoutMs);
  const exact = observation.irohPeers.some(peer => peer.nodeId === nodeId);
  const gaugeMoved = observation.peersKnown >= held.observation.peersKnown + 1;
  const watchMoved = observation.watchReads > held.observation.watchReads;
  if (!exact || !gaugeMoved || !watchMoved) return undefined;
  return { elapsedMs: Date.now() - stagedAt, observation };
}

async function sampleBoard(
  options: Options,
  nodeId: string,
  stagedAt: number,
  prior: number | undefined
): Promise<number | undefined> {
  if (prior !== undefined) return prior;
  try {
    return (await boardHasJoiner(options, nodeId)) ? Date.now() - stagedAt : undefined;
  } catch {
    // An unreadable sample is retried inside the declared deadline; it is
    // never converted to an empty board or a successful observation.
    return undefined;
  }
}

async function pollIncumbent(
  held: Baseline,
  options: Options,
  nodeId: string,
  stagedAt: number,
  discoveries: Map<string, Discovery>,
  errors: Map<string, string>
): Promise<void> {
  if (discoveries.has(held.target.name)) return;
  try {
    const discovery = await sampleDiscovery(held, nodeId, stagedAt, options.requestTimeoutMs);
    if (!discovery) return;
    discoveries.set(held.target.name, discovery);
    console.log(
      `discovered ${held.target.name}: ${discovery.elapsedMs / 1_000}s ` +
        `book=${held.observation.peersKnown}->${discovery.observation.peersKnown} ` +
        `watch=${held.observation.watchReads}->${discovery.observation.watchReads}`
    );
  } catch (error) {
    errors.set(held.target.name, error instanceof Error ? error.message : String(error));
  }
}

async function pollForReceipt(
  options: Options,
  baselines: Baseline[],
  nodeId: string,
  stagedAt: number
): Promise<PollResult> {
  const deadline = stagedAt + options.deadlineMs;
  const discoveries = new Map<string, Discovery>();
  let boardSeenMs: number | undefined;
  let lastErrors = new Map<string, string>();
  while (
    Date.now() <= deadline &&
    (boardSeenMs === undefined || discoveries.size < baselines.length)
  ) {
    boardSeenMs = await sampleBoard(options, nodeId, stagedAt, boardSeenMs);
    const nextErrors = new Map<string, string>();
    await Promise.all(
      baselines.map(async held =>
        pollIncumbent(held, options, nodeId, stagedAt, discoveries, nextErrors)
      )
    );
    lastErrors = nextErrors;
    if (boardSeenMs === undefined || discoveries.size < baselines.length) {
      await sleep(options.pollMs);
    }
  }
  return { boardSeenMs, discoveries, lastErrors };
}

function assertFreshNodeId(baselines: Baseline[], nodeId: string): void {
  const collision = baselines.find(held =>
    held.observation.irohPeers.some(peer => peer.nodeId === nodeId)
  );
  if (!collision) return;
  throw new ReceiptIncomplete(
    `${collision.target.name} already held the supposedly fresh NodeId before staging`,
    [collision.target.name]
  );
}

async function run(options: Options): Promise<void> {
  const lock = await acquireMeshLock(DEFAULT_MESH_DIR);
  try {
    const baselines = await Promise.all(
      options.peers.map(async target => baseline(target, options.requestTimeoutMs))
    );
    console.log('warm incumbents before staging:');
    for (const held of baselines) {
      console.log(
        `  ${held.target.name}: pid=${held.pid} startTicks=${held.processStartTicks} ` +
          `irohPeers=${held.observation.irohPeers.length} gauge=${held.observation.peersKnown} ` +
          `watchReads=${held.observation.watchReads}`
      );
    }

    const joined = await stageJoiner(options);
    const joinedTarget = localPeer(joined.name, joined.url);
    const joinedStatus = await fetchJson(`${joined.url}/p2p/status`, options.requestTimeoutMs);
    if (joinedStatus['irohNodeId'] !== joined.nodeId) {
      throw new ReceiptIncomplete(
        `${joined.name} status NodeId does not match the staged receipt (${String(joinedStatus['irohNodeId'])})`
      );
    }
    assertFreshNodeId(baselines, joined.nodeId);

    const stagedAt = Date.now();
    const { boardSeenMs, discoveries, lastErrors } = await pollForReceipt(
      options,
      baselines,
      joined.nodeId,
      stagedAt
    );

    const missing = baselines.map(held => held.target.name).filter(name => !discoveries.has(name));
    if (boardSeenMs === undefined || missing.length > 0) {
      const reasons = missing.map(name => `${name}: ${lastErrors.get(name) ?? 'NodeId absent'}`);
      if (boardSeenMs === undefined) reasons.unshift('doorway: exact signed NodeId absent');
      throw new ReceiptIncomplete(
        `late joiner ${joined.name} was not learned within ${options.deadlineMs / 1_000}s — ${reasons.join('; ')}`,
        missing
      );
    }

    console.log(`doorway board: exact signed NodeId seen at ${boardSeenMs / 1_000}s`);
    for (const held of baselines) {
      const discovery = discoveries.get(held.target.name)!;
      console.log(
        `PASS ${held.target.name}: exact NodeId ${joined.nodeId.slice(0, 12)}… in ` +
          `${(discovery.elapsedMs / 1_000).toFixed(1)}s; pid/startTicks unchanged; ` +
          `watch seeded ${held.observation.watchSeeded}->${discovery.observation.watchSeeded}`
      );
    }
    console.log(
      `PASS late joiner ${joined.name} (${joinedTarget.url}): ${baselines.length}/${baselines.length} warm incumbents discovered it without restart`
    );
    console.log(
      'attribution: books were non-empty before staging, incumbent process start-times never moved, ' +
        'the exact signed joiner identity reached the board, and every incumbent watch counter advanced. ' +
        'The retired boot-only/empty-book predicate would leave those warm-book watch counters unchanged.'
    );
  } finally {
    lock.stdin.end();
    await once(lock, 'exit').catch(() => undefined);
  }
}

try {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    console.log(USAGE);
    process.exitCode = 0;
  } else {
    await run(options);
    process.exitCode = 0;
  }
} catch (error) {
  if (error instanceof UsageError) {
    console.error(`${error.message}\n\n${USAGE}`);
    process.exitCode = 64;
  } else if (error instanceof ReceiptIncomplete) {
    console.error(`RECEIPT INCOMPLETE: ${error.message}`);
    if (error.missingPeers.length > 0) {
      console.error(`missing peers: ${error.missingPeers.join(', ')}`);
    }
    process.exitCode = 2;
  } else {
    console.error(error instanceof Error ? error.stack : String(error));
    process.exitCode = 1;
  }
}
