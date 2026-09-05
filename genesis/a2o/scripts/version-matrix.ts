/**
 * Render the runtime-version matrix for a local mesh or a named fleet.
 *
 * The /version response is intentionally treated as unknown JSON. Today it is
 * a flat BuildInfo object; the runtime-passport work adds nested blocks while
 * retaining those legacy fields. Flattening the response lets this probe gain
 * rows as that additive contract lands without coupling to an anticipated
 * passport interface.
 *
 * Usage:
 *   tsx scripts/version-matrix.ts [name=url,...]
 *     [--peers name=url,...] [--conductors name=admin:app,...]
 *     [--timeout <ms>] [--observed] [--json] [--out <path>]
 *
 * Peer precedence: --peers / positional CSV, PEER_STORAGE_URLS, then the
 * matthew/jessica/james localhost mesh. Optional conductor ports can also come
 * from PEER_CONDUCTOR_PORTS. A failed conductor probe is omitted; only a failed
 * storage /version request makes the command exit 2.
 */

import { writeFile } from 'node:fs/promises';

// This operator script uses the same dev-only client as the existing mesh probes.
// The new-file-only task contract intentionally forbids reclassifying package deps.
// eslint-disable-next-line import/no-extraneous-dependencies
import { AdminWebsocket, AppWebsocket } from '@holochain/client';

const APP_ID = 'elohim';
const DEFAULT_PEERS =
  'matthew=http://localhost:8090,jessica=http://localhost:8091,james=http://localhost:8092';
const DEFAULT_TIMEOUT_MS = 5_000;
const MISSING = '—';

const USAGE = `Usage: version-matrix.ts [name=url,...] [options]

Options:
  --peers <csv>         storage peers as name=http://host:port
  --conductors <csv>    optional mesh conductor ports as name=admin:app
  --timeout <ms>        per-request timeout (default: ${DEFAULT_TIMEOUT_MS})
  --observed            render who each peer observes on the iroh plane
  --json                emit source evidence plus derived rows as JSON
  --out <path>          also write the selected rendering to this path
  -h, --help            show this help

Environment:
  PEER_STORAGE_URLS     peer CSV used when argv does not provide one
  PEER_CONDUCTOR_PORTS  optional conductor-port CSV`;

interface Options {
  peerCsv: string;
  conductorCsv?: string;
  timeoutMs: number;
  observed: boolean;
  json: boolean;
  out?: string;
  help: boolean;
}

interface Peer {
  name: string;
  url: string;
}

interface ConductorPorts {
  admin: number;
  app: number;
}

interface VersionResult {
  passport?: unknown;
  error?: string;
}

interface StatusResult {
  status?: unknown;
  error?: string;
}

interface PeerResult extends Peer {
  reachable: boolean;
  passport?: unknown;
  zomeBuildInfo?: unknown;
  p2pStatus?: unknown;
  error?: string;
}

interface MatrixRow {
  field: string;
  values: Record<string, string>;
  divergent: boolean;
}

interface JsonReport {
  peers: PeerResult[];
  rows: MatrixRow[];
  observed?: ObservedMatrix;
}

interface ObservedRow {
  observer: string;
  observerNodeId?: string;
  values: Record<string, string>;
}

interface ObservedMatrix {
  nodeIds: string[];
  labels: Record<string, string>;
  rows: ObservedRow[];
  divergentNodeIds: string[];
}

function requiredValue(args: string[], index: number, flag: string): string {
  const value = args[index + 1];
  if (!value || value.startsWith('--')) throw new Error(`${flag} requires a value`);
  return value;
}

function parseTimeout(value: string): number {
  const timeout = Number(value);
  if (!Number.isInteger(timeout) || timeout < 1) {
    throw new Error(`--timeout expects a positive integer, got: ${value}`);
  }
  return timeout;
}

function parseArgs(argv: string[], env: NodeJS.ProcessEnv): Options {
  const positionals: string[] = [];
  let peersFlag: string | undefined;
  let conductorCsv: string | undefined;
  let timeoutMs = DEFAULT_TIMEOUT_MS;
  let observed = false;
  let json = false;
  let out: string | undefined;
  let help = false;

  for (let index = 0; index < argv.length; index++) {
    const arg = argv[index];
    switch (arg) {
      case '--peers':
        peersFlag = requiredValue(argv, index, arg);
        index++;
        break;
      case '--conductors':
        conductorCsv = requiredValue(argv, index, arg);
        index++;
        break;
      case '--timeout':
        timeoutMs = parseTimeout(requiredValue(argv, index, arg));
        index++;
        break;
      case '--json':
        json = true;
        break;
      case '--observed':
        observed = true;
        break;
      case '--out':
        out = requiredValue(argv, index, arg);
        index++;
        break;
      case '-h':
      case '--help':
        help = true;
        break;
      default:
        if (arg.startsWith('-')) throw new Error(`unknown option: ${arg}`);
        positionals.push(arg);
    }
  }

  if (positionals.length > 2) {
    throw new Error(`unexpected positional argument: ${positionals[2]}`);
  }
  const [positionalPeers, positionalConductors] = positionals;
  if (peersFlag && positionalPeers) {
    throw new Error('provide peers either positionally or with --peers, not both');
  }
  if (conductorCsv && positionalConductors) {
    throw new Error('provide conductor ports either positionally or with --conductors, not both');
  }

  return {
    peerCsv: peersFlag ?? positionalPeers ?? env['PEER_STORAGE_URLS'] ?? DEFAULT_PEERS,
    conductorCsv: conductorCsv ?? positionalConductors ?? env['PEER_CONDUCTOR_PORTS'] ?? undefined,
    timeoutMs,
    observed,
    json,
    out,
    help,
  };
}

function parseHttpUrl(input: string): string {
  const withScheme = /^https?:\/\//i.test(input) ? input : `http://${input}`;
  const url = new URL(withScheme);
  if (url.protocol !== 'http:' && url.protocol !== 'https:') {
    throw new Error(`unsupported peer URL protocol: ${url.protocol}`);
  }
  return url.toString().replace(/\/$/, '');
}

function derivePeerName(url: string): string {
  return new URL(url).host;
}

function parsePeers(csv: string): Peer[] {
  const peers = csv
    .split(',')
    .map(entry => entry.trim())
    .filter(Boolean)
    .map(entry => {
      const separator = entry.indexOf('=');
      const rawName = separator >= 0 ? entry.slice(0, separator).trim() : '';
      const rawUrl = separator >= 0 ? entry.slice(separator + 1).trim() : entry;
      if (!rawUrl) throw new Error(`empty URL in peer entry: ${entry}`);
      const url = parseHttpUrl(rawUrl);
      return { name: rawName || derivePeerName(url), url };
    });

  if (peers.length === 0) throw new Error('peer list is empty');
  const names = new Set<string>();
  for (const peer of peers) {
    if (names.has(peer.name)) throw new Error(`duplicate peer name: ${peer.name}`);
    names.add(peer.name);
  }
  return peers;
}

function parsePort(value: string, entry: string): number {
  const port = Number(value);
  if (!Number.isInteger(port) || port < 1 || port > 65_535) {
    throw new Error(`invalid conductor port in entry: ${entry}`);
  }
  return port;
}

function parseConductors(csv?: string): Map<string, ConductorPorts> {
  const conductors = new Map<string, ConductorPorts>();
  if (!csv) return conductors;

  for (const rawEntry of csv.split(',')) {
    const entry = rawEntry.trim();
    if (!entry) continue;
    const [name, ports, extra] = entry.split('=');
    const [admin, app, extraPort] = (ports ?? '').split(':');
    if (!name || !admin || !app || extra || extraPort) {
      throw new Error(`conductor entry must be name=admin:app, got: ${entry}`);
    }
    if (conductors.has(name)) throw new Error(`duplicate conductor name: ${name}`);
    conductors.set(name, {
      admin: parsePort(admin, entry),
      app: parsePort(app, entry),
    });
  }
  return conductors;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function probeVersion(peer: Peer, timeoutMs: number): Promise<VersionResult> {
  try {
    const response = await fetch(`${peer.url}/version`, {
      headers: { accept: 'application/json' },
      signal: AbortSignal.timeout(timeoutMs),
    });
    if (!response.ok) return { error: `HTTP ${response.status} ${response.statusText}`.trim() };
    return { passport: (await response.json()) as unknown };
  } catch (error) {
    return { error: errorMessage(error) };
  }
}

async function probeP2pStatus(peer: Peer, timeoutMs: number): Promise<StatusResult> {
  try {
    const response = await fetch(`${peer.url}/p2p/status`, {
      headers: { accept: 'application/json' },
      signal: AbortSignal.timeout(timeoutMs),
    });
    if (!response.ok) return { error: `HTTP ${response.status} ${response.statusText}`.trim() };
    return { status: (await response.json()) as unknown };
  } catch (error) {
    return { error: errorMessage(error) };
  }
}

async function probeZomeBuildInfo(ports: ConductorPorts): Promise<unknown> {
  const admin = await AdminWebsocket.connect({
    url: new URL(`ws://127.0.0.1:${ports.admin}`),
    wsClientOptions: { origin: APP_ID },
  });
  let appWebsocket: AppWebsocket | undefined;
  try {
    const apps = await admin.listApps({});
    const app = apps.find(candidate => candidate.installed_app_id === APP_ID) ?? apps[0];
    if (!app) throw new Error('conductor has no installed apps');

    const learningCell = app.cell_info['lamad']?.find(cell => cell.type === 'provisioned');
    if (!learningCell) {
      throw new Error(`app ${app.installed_app_id} has no provisioned learning cell`);
    }

    await admin.authorizeSigningCredentials(learningCell.value.cell_id);
    const token = await admin.issueAppAuthenticationToken({
      installed_app_id: app.installed_app_id,
    });
    appWebsocket = await AppWebsocket.connect({
      url: new URL(`ws://127.0.0.1:${ports.app}`),
      token: token.token,
      wsClientOptions: { origin: APP_ID },
    });
    return await appWebsocket.callZome<unknown>({
      cell_id: learningCell.value.cell_id,
      zome_name: 'content_store',
      fn_name: 'zome_build_info',
      payload: null,
    });
  } finally {
    const closes: Promise<unknown>[] = [admin.client.close()];
    if (appWebsocket) {
      closes.push(
        Promise.resolve((appWebsocket.client as unknown as { close(): unknown }).close())
      );
    }
    await Promise.allSettled(closes);
  }
}

async function probePeer(
  peer: Peer,
  ports: ConductorPorts | undefined,
  timeoutMs: number,
  observed: boolean
): Promise<PeerResult> {
  const versionPromise = probeVersion(peer, timeoutMs);
  const statusPromise = observed
    ? probeP2pStatus(peer, timeoutMs)
    : Promise.resolve<StatusResult>({});
  const zomePromise = ports
    ? probeZomeBuildInfo(ports).catch(() => undefined)
    : Promise.resolve(undefined);
  const [version, status, zomeBuildInfo] = await Promise.all([
    versionPromise,
    statusPromise,
    zomePromise,
  ]);
  const error = version.error ?? (status.error ? `p2p/status: ${status.error}` : undefined);
  return {
    ...peer,
    reachable: error === undefined,
    passport: version.passport,
    zomeBuildInfo,
    p2pStatus: status.status,
    error,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function displayValue(value: unknown): string {
  if (typeof value === 'string') return value || '""';
  if (value === undefined) return MISSING;
  if (value === null) return 'null';
  return String(value);
}

function flatten(value: unknown, prefix: string, target: Map<string, string>): void {
  if (isRecord(value)) {
    const entries = Object.entries(value);
    if (entries.length === 0 && prefix) target.set(prefix, '{}');
    for (const [key, child] of entries) {
      flatten(child, prefix ? `${prefix}.${key}` : key, target);
    }
    return;
  }

  if (Array.isArray(value)) {
    if (value.length === 0 && prefix) target.set(prefix, '[]');
    for (const [index, child] of value.entries()) flatten(child, `${prefix}[${index}]`, target);
    return;
  }

  target.set(prefix || 'response', displayValue(value));
}

function peerFields(result: PeerResult): Map<string, string> {
  const fields = new Map<string, string>([['reachable', String(result.reachable)]]);
  if (result.passport !== undefined) flatten(result.passport, '', fields);
  if (result.zomeBuildInfo !== undefined) flatten(result.zomeBuildInfo, 'zomeBuildInfo', fields);
  if (result.error) fields.set('error', result.error);
  return fields;
}

function buildRows(results: PeerResult[]): MatrixRow[] {
  const fieldsByPeer = results.map(peerFields);
  const names = results.map(result => result.name);
  const allFields = new Set<string>();
  for (const fields of fieldsByPeer) {
    for (const field of fields.keys()) allFields.add(field);
  }

  const orderedFields = [...allFields].sort((left, right) => {
    if (left === 'reachable') return -1;
    if (right === 'reachable') return 1;
    return left.localeCompare(right);
  });

  return orderedFields.map(field => {
    const values = Object.fromEntries(
      names.map((name, index) => [name, fieldsByPeer[index].get(field) ?? MISSING])
    );
    return {
      field,
      values,
      divergent: new Set(Object.values(values)).size > 1,
    };
  });
}

function renderGrid(headers: string[], body: string[][]): string {
  const widths = headers.map((header, index) =>
    Math.max(header.length, ...body.map(cells => cells[index].length))
  );
  const line = (cells: string[]): string =>
    cells
      .map((cell, index) => cell.padEnd(widths[index]))
      .join('  ')
      .trimEnd();
  return [line(headers), line(widths.map(width => '-'.repeat(width))), ...body.map(line)].join(
    '\n'
  );
}

function renderTable(results: PeerResult[], rows: MatrixRow[]): string {
  const headers = ['FIELD', ...results.map(result => result.name), 'STATUS'];
  const body = rows.map(row => [
    row.field,
    ...results.map(result => row.values[result.name]),
    row.divergent ? 'DIVERGENT' : '',
  ]);
  return renderGrid(headers, body);
}

function irohNodeId(status: unknown): string | undefined {
  if (!isRecord(status)) return undefined;
  const nodeId = status['irohNodeId'];
  return typeof nodeId === 'string' && nodeId ? nodeId : undefined;
}

function irohObservations(status: unknown): Map<string, string> {
  const observations = new Map<string, string>();
  if (!isRecord(status) || !Array.isArray(status['irohPeers'])) return observations;
  for (const peer of status['irohPeers']) {
    if (!isRecord(peer) || typeof peer['nodeId'] !== 'string' || !peer['nodeId']) continue;
    observations.set(
      peer['nodeId'],
      typeof peer['userAgent'] === 'string' && peer['userAgent'] ? peer['userAgent'] : MISSING
    );
  }
  return observations;
}

function abbreviateNodeIds(nodeIds: string[]): Record<string, string> {
  const labels: Record<string, string> = {};
  for (const nodeId of nodeIds) {
    let width = Math.min(12, nodeId.length);
    while (
      width < nodeId.length &&
      nodeIds.some(
        candidate => candidate !== nodeId && candidate.startsWith(nodeId.slice(0, width))
      )
    ) {
      width++;
    }
    labels[nodeId] = width < nodeId.length ? `${nodeId.slice(0, width)}…` : nodeId;
  }
  return labels;
}

function buildObservedMatrix(results: PeerResult[]): ObservedMatrix {
  const byObserver = results.map(result => ({
    observer: result.name,
    observerNodeId: irohNodeId(result.p2pStatus),
    observations: irohObservations(result.p2pStatus),
  }));
  const rosterNodeIds = new Set<string>();
  for (const row of byObserver) {
    if (row.observerNodeId) rosterNodeIds.add(row.observerNodeId);
  }
  // The command's peer arguments define the version-matrix roster. A peer
  // book can legitimately retain observations for late joiners or peers not
  // selected for this run; letting those entries create columns makes a
  // three-peer receipt grow without bound and turns stale evidence into false
  // divergence. Fall back to observed entries only when every queried peer is
  // too old to expose its own irohNodeId.
  const nodeIds = rosterNodeIds.size > 0 ? rosterNodeIds : new Set<string>();
  if (nodeIds.size === 0) {
    for (const row of byObserver) {
      for (const nodeId of row.observations.keys()) nodeIds.add(nodeId);
    }
  }
  const orderedNodeIds = [...nodeIds].sort((left, right) => left.localeCompare(right));
  const rows = byObserver.map(row => ({
    observer: row.observer,
    observerNodeId: row.observerNodeId,
    values: Object.fromEntries(
      orderedNodeIds.map(nodeId => [nodeId, row.observations.get(nodeId) ?? MISSING])
    ),
  }));
  const divergentNodeIds = orderedNodeIds.filter(nodeId => {
    // Peer books intentionally exclude self. Compare only independent
    // observers so the expected diagonal dash is not false divergence.
    const independentValues = rows
      .filter(row => row.observerNodeId !== nodeId)
      .map(row => row.values[nodeId]);
    return new Set(independentValues).size > 1;
  });
  return {
    nodeIds: orderedNodeIds,
    labels: abbreviateNodeIds(orderedNodeIds),
    rows,
    divergentNodeIds,
  };
}

function renderObservedTable(matrix: ObservedMatrix): string {
  const headers = ['OBSERVER', ...matrix.nodeIds.map(nodeId => matrix.labels[nodeId])];
  const body = matrix.rows.map(row => [
    row.observer,
    ...matrix.nodeIds.map(nodeId => row.values[nodeId]),
  ]);
  body.push([
    'STATUS',
    ...matrix.nodeIds.map(nodeId => (matrix.divergentNodeIds.includes(nodeId) ? 'DIVERGENT' : '')),
  ]);
  return renderGrid(headers, body);
}

async function main(): Promise<number> {
  const options = parseArgs(process.argv.slice(2), process.env);
  if (options.help) {
    console.log(USAGE);
    return 0;
  }

  const peers = parsePeers(options.peerCsv);
  const conductors = parseConductors(options.conductorCsv);
  const results = await Promise.all(
    peers.map(async peer =>
      probePeer(peer, conductors.get(peer.name), options.timeoutMs, options.observed)
    )
  );
  const rows = buildRows(results);
  const observed = options.observed ? buildObservedMatrix(results) : undefined;
  const report: JsonReport = { peers: results, rows, observed };
  const rendered = options.json
    ? JSON.stringify(report, null, 2)
    : [
        renderTable(results, rows),
        observed && `OBSERVED IROH USER-AGENTS\n${renderObservedTable(observed)}`,
      ]
        .filter(Boolean)
        .join('\n\n');

  console.log(rendered);
  if (options.out) await writeFile(options.out, `${rendered}\n`, 'utf8');
  return results.every(result => result.reachable) ? 0 : 2;
}

main()
  .then(exitCode => {
    process.exitCode = exitCode;
  })
  .catch(error => {
    console.error(`version-matrix: ${errorMessage(error)}`);
    console.error(USAGE);
    process.exitCode = 64;
  });
