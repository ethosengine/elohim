/**
 * `substrate-verify.selftest` — self-checking parity harness for the
 * TypeScript port of `genesis/scripts/ci/substrate-verify.sh`.
 *
 * genesis/a2o has no vitest; its unit tier is `tsx --test`. This file follows
 * the repo's `scripts/ci/*.test.sh` shape instead: a standalone script that
 * spins REAL `node:http` fixture servers, drives each subcommand's logic
 * against them, prints PASS/FAIL per check, and exits non-zero on any failure.
 *
 *   pnpm --filter @elohim/a2o substrate-verify:selftest
 *   tsx scripts/substrate-verify.selftest.ts
 *
 * What this covers: the RUNNER's parity logic — assertion names, detail
 * strings, report shape, exit codes, state-file handoff, poll budgets,
 * tri-state stream classification, silent-skip branches.
 *
 * What it deliberately does NOT cover: the `@elohim/storage-client` facade
 * itself. The facade is injected here as a fixture implementation of the same
 * frozen interface (real HTTP under it, so GET-not-HEAD and status semantics
 * are exercised); the facade's own correctness is its package's test surface.
 */

import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  runCommand,
  type Assertion,
  type Command,
  type DataplaneApiLike,
  type DoorwayLike,
  type Facade,
  type FleetLike,
  type Report,
  type SeedBlobResult,
  type StorageClientLike,
  type StreamName,
  type StreamState,
} from './substrate-verify.js';

// ---------------------------------------------------------------------------
// tiny assertion harness
// ---------------------------------------------------------------------------

let checks = 0;
let failures = 0;
let currentCase = '';

function testCase(name: string): void {
  currentCase = name;
  console.log(`\n── ${name}`);
}

function check(ok: boolean, label: string, extra?: string): void {
  checks++;
  if (ok) {
    console.log(`   PASS  ${label}`);
  } else {
    failures++;
    const context = extra === undefined ? '' : `\n         ${extra}`;
    console.log(`   FAIL  ${label}${context}`);
  }
}

function eqDeep(actual: unknown, expected: unknown, label: string): void {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  check(a === e, label, a === e ? undefined : `expected ${e}\n         actual   ${a}`);
}

/** `status:name` for every assertion, in order — the parity fingerprint. */
function inventory(report: Report): string[] {
  return report.assertions.map(a => `${a.status}:${a.name}`);
}

function detailOf(report: Report, name: string): string {
  return report.assertions.find(a => a.name === name)?.detail ?? '<absent>';
}

// ---------------------------------------------------------------------------
// fixture HTTP server
// ---------------------------------------------------------------------------

interface Canned {
  status?: number;
  json?: unknown;
  /** Raw body; wins over `json`. */
  text?: string;
}

type Route = Canned | ((req: IncomingMessage, hit: number) => Canned);

interface Fixture {
  baseUrl: string;
  /** Every request path (with query) the server received, in order. */
  seen: string[];
  close(): Promise<void>;
}

async function startFixture(routes: Record<string, Route>): Promise<Fixture> {
  const seen: string[] = [];
  const hits = new Map<string, number>();

  const server: Server = createServer((req: IncomingMessage, res: ServerResponse) => {
    const url = req.url ?? '/';
    seen.push(`${req.method} ${url}`);
    const path = url.split('?')[0] ?? '/';
    // Longest-prefix match so `/blob/<hash>` can be a single route key.
    const key =
      Object.keys(routes)
        .filter(k => path === k || path.startsWith(k.endsWith('/') ? k : `${k}/`))
        .sort((a, b) => b.length - a.length)[0] ?? '';
    const route = routes[key];
    // Drain the request body (the seed PUT sends one).
    req.resume();
    req.on('end', () => {
      if (!route) {
        res.writeHead(404, { 'content-type': 'application/json' });
        res.end('{"error":"no fixture route"}');
        return;
      }
      const n = (hits.get(key) ?? 0) + 1;
      hits.set(key, n);
      const canned = typeof route === 'function' ? route(req, n) : route;
      const body = canned.text ?? JSON.stringify(canned.json ?? null);
      res.writeHead(canned.status ?? 200, { 'content-type': 'application/json' });
      res.end(body);
    });
  });

  await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve));
  const addr = server.address();
  const port = typeof addr === 'object' && addr ? addr.port : 0;
  return {
    baseUrl: `127.0.0.1:${port}`,
    seen,
    close: async () => new Promise<void>(resolve => server.close(() => resolve())),
  };
}

/** A host:port that always refuses — the "unreachable peer" case. */
const DEAD_PEER = '127.0.0.1:1';

// Shared fixture literals (sonarjs/no-duplicate-string, and they double as the
// single place to change a persona / window when the fixtures evolve).
const PID_MATTHEW = 'pid-matthew';
const PID_JESSICA = 'pid-jessica';
const PROBE_HASH = 'sha256-probe';
const AFTER_ISO = '2026-08-15T00:00:00Z';
const N_MESH_VERSION_PARITY = 'mesh.version-parity';
const N_DELIVERY_EVENTS = 'delivery.serve-blob-events';
const USAGE_LINE =
  'usage: substrate-verify.ts {mesh|upload|propagation|delivery|projection|federation|resilience}';

// ---------------------------------------------------------------------------
// fixture facade — the frozen @elohim/storage-client interface over real HTTP
// ---------------------------------------------------------------------------

class HttpError extends Error {
  constructor(readonly status: number) {
    super(`HTTP ${status}`);
  }
}

async function getJson(base: string, path: string): Promise<unknown> {
  const res = await fetch(`${base}${path}`);
  if (!res.ok) throw new HttpError(res.status);
  const text = await res.text();
  return text === '' ? null : JSON.parse(text);
}

class FixtureDataplane implements DataplaneApiLike {
  constructor(private readonly base: string) {}

  async getP2pStatus(): Promise<unknown> {
    return getJson(this.base, '/p2p/status');
  }
  async getVersion(): Promise<{ commit?: string; version?: string }> {
    return (await getJson(this.base, '/version')) as { commit?: string; version?: string };
  }
  async listCommitments(q: { action?: string; limit?: number } = {}): Promise<unknown[]> {
    const qs = new URLSearchParams();
    if (q.action) qs.set('action', q.action);
    if (q.limit !== undefined) qs.set('limit', String(q.limit));
    return (await getJson(this.base, `/api/v1/commitments?${qs}`)) as unknown[];
  }
  async listEconomicEvents(
    q: { action?: string; after?: string; limit?: number } = {}
  ): Promise<unknown[]> {
    const qs = new URLSearchParams();
    if (q.action) qs.set('action', q.action);
    if (q.after) qs.set('after', q.after);
    if (q.limit !== undefined) qs.set('limit', String(q.limit));
    return (await getJson(this.base, `/api/v1/economic-events?${qs}`)) as unknown[];
  }
  async getProjectorStatus(): Promise<unknown> {
    return getJson(this.base, '/api/v1/status/projector');
  }
  async getInventoryParity(): Promise<{ filesystemCount: number | null }> {
    const raw = (await getJson(this.base, '/api/v1/diagnostics/inventory-parity')) as Record<
      string,
      unknown
    > | null;
    const v = raw?.['filesystem_count'] ?? raw?.['filesystemCount'];
    return { filesystemCount: typeof v === 'number' ? v : null };
  }
  /** Real GET, body read and discarded — never HEAD (heal-on-read is load-bearing). */
  async getBlobStatus(hash: string): Promise<number> {
    const res = await fetch(`${this.base}/blob/${hash}`);
    await res.arrayBuffer();
    return res.status;
  }
  async listConnectedPeers(): Promise<unknown> {
    return getJson(this.base, '/p2p/peers');
  }
  async listPeerStatuses(): Promise<unknown> {
    return getJson(this.base, '/api/v1/peer-statuses');
  }
  async getNetworkPosture(): Promise<unknown> {
    return getJson(this.base, '/api/v1/network/posture');
  }
}

class FixtureDoorway implements DoorwayLike {
  constructor(private readonly base: string) {}
  async getHealth(): Promise<unknown> {
    return getJson(this.base, '/health');
  }
  async seedBlob(args: {
    hash: string;
    data: Uint8Array;
    token?: string;
  }): Promise<SeedBlobResult> {
    const headers: Record<string, string> = {
      'X-Blob-Hash': args.hash,
      'Content-Type': 'application/octet-stream',
    };
    // No token ⇒ NO Authorization header at all (dev-mode doorway accepts).
    if (args.token) headers['Authorization'] = `Bearer ${args.token}`;
    const res = await fetch(`${this.base}/admin/seed/blob`, {
      method: 'PUT',
      headers,
      body: Buffer.from(args.data),
    });
    const errorBody = res.ok || res.status === 409 ? undefined : await res.text();
    return { ok: res.ok, status: res.status, alreadyPresent: res.status === 409, errorBody };
  }
  async listFederationDoorways(): Promise<unknown> {
    return getJson(this.base, '/api/v1/federation/doorways');
  }
  async listFederationP2pPeers(): Promise<unknown> {
    return getJson(this.base, '/api/v1/federation/p2p-peers');
  }
}

/**
 * Byte-for-byte mirror of the REAL facade helper
 * (`elohim/sdk/storage-client-ts/src/api/dataplane.ts` `streamSyncState`), so
 * the selftest exercises the semantics CI will actually see — including the
 * one place it diverges from bash's jq: a stream object present but missing
 * `caughtUp`/`caught_up` is 'not-computable' here (bash rendered `false`).
 */
function fixtureStreamSyncState(raw: unknown, stream: StreamName): StreamState {
  const rec = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : null;
  const keys =
    stream === 'projectionReconcile' ? ['projectionReconcile', 'projection_reconcile'] : [stream];
  let node: unknown = null;
  for (const k of keys) {
    const v = rec?.[k];
    if (v !== null && v !== undefined) {
      node = v;
      break;
    }
  }
  if (node === null || typeof node !== 'object') return 'not-computable';
  const o = node as Record<string, unknown>;
  // 'idle' is claimed only for `pull`, and only when the wire carries `total`.
  if (stream === 'pull' && typeof o['total'] === 'number' && o['total'] === 0) return 'idle';
  const cu = o['caughtUp'] ?? o['caught_up'];
  if (typeof cu !== 'boolean') return 'not-computable';
  return cu ? 'caught-up' : 'behind';
}

function fixtureFacade(): Facade {
  return {
    fleetFromCsv(csv: string): FleetLike {
      const entries = csv
        .split(',')
        .map(s => s.trim())
        .filter(s => s !== '' && s.includes('='))
        .map(s => {
          const eq = s.indexOf('=');
          const name = s.slice(0, eq);
          const client: StorageClientLike = {
            dataplane: new FixtureDataplane(`http://${s.slice(eq + 1)}`),
          };
          return { name, client };
        });
      return {
        peers: () => entries,
        peer: (name: string) => entries.find(e => e.name === name)?.client,
        names: () => entries.map(e => e.name),
      };
    },
    makeDoorway: (baseUrl: string) => new FixtureDoorway(baseUrl),
    streamSyncState: fixtureStreamSyncState,
  };
}

// ---------------------------------------------------------------------------
// run harness
// ---------------------------------------------------------------------------

interface Harness {
  exitCode: number;
  report: Report;
  reportPath: string;
  /** Every injected sleep, in seconds, in order. */
  sleeps: number[];
  lines: string[];
  stateDir: string;
  reportDir: string;
}

async function run(command: Command, env: Record<string, string>): Promise<Harness> {
  const dir = mkdtempSync(join(tmpdir(), 'sv-selftest-'));
  const stateDir = env['STATE_DIR'] ?? join(dir, 'state');
  const reportDir = env['REPORT_DIR'] ?? join(dir, 'reports');
  const sleeps: number[] = [];
  const lines: string[] = [];
  const res = await runCommand(command, {
    env: { ...env, STATE_DIR: stateDir, REPORT_DIR: reportDir },
    facade: fixtureFacade(),
    sleep: async (s: number) => {
      sleeps.push(s);
      return Promise.resolve();
    },
    log: (l: string) => lines.push(l),
    cwd: process.cwd(),
  });
  return { ...res, sleeps, lines, stateDir, reportDir };
}

function readReportFile(h: Harness): Report {
  return JSON.parse(readFileSync(h.reportPath, 'utf8')) as Report;
}

function assertEnvelope(h: Harness, command: string): void {
  const onDisk = readReportFile(h);
  check(existsSync(h.reportPath), `report written to ${command}`);
  check(onDisk.schemaVersion === '1', 'schemaVersion == "1"');
  check(onDisk.runner === 'facade', 'runner == "facade" (CI measure field)');
  check(onDisk.command === command, `command == "${command}"`);
  check(
    /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/.test(onDisk.generatedAt),
    'generatedAt is second-precision ISO8601 UTC',
    onDisk.generatedAt
  );
  check(
    onDisk.failed === onDisk.assertions.filter((a: Assertion) => a.status === 'fail').length,
    'failed == count of fail-status assertions'
  );
  check(
    h.exitCode === (onDisk.failed === 0 ? 0 : 1),
    `exit code ${h.exitCode} matches failed=${onDisk.failed}`
  );
}

// ---------------------------------------------------------------------------
// canned route sets
// ---------------------------------------------------------------------------

const P2P_STATUS_HEALTHY = (
  peerId: string,
  connected: number,
  extra: Record<string, unknown> = {}
) => ({
  json: {
    peerId,
    connectedPeers: connected,
    natStatus: 'public',
    relayMode: 'off',
    replication: { caughtUp: true },
    pull: { total: 5, fetched: 5, caughtUp: true },
    projectionReconcile: { caughtUp: true },
    ...extra,
  },
});

const CUSTODY_ROWS = (hash: string) => ({
  json: [
    { action: 'custody-blob', resourceClassifiedAs: [hash] },
    { action: 'something-else', resourceClassifiedAs: ['sha256-other'] },
  ],
});

// ===========================================================================
// cases
// ===========================================================================

async function meshHappyPath(): Promise<void> {
  testCase('mesh — happy path (2 reachable peers, bidirectional adjacency, version parity)');
  const matthew = await startFixture({
    '/p2p/status': P2P_STATUS_HEALTHY(PID_MATTHEW, 3),
    '/p2p/peers': { json: { peers: [{ peerId: PID_JESSICA }, { peer_id: 'pid-other' }] } },
    '/version': { json: { commit: 'abc123' } },
  });
  const jessica = await startFixture({
    '/p2p/status': P2P_STATUS_HEALTHY(PID_JESSICA, 2),
    '/p2p/peers': { json: { peers: [{ peer_id: PID_MATTHEW }] } },
    '/version': { json: { commit: 'abc123' } },
  });
  const doorway = await startFixture({
    '/health': { json: { p2p: { peerCount: 7 }, conductor: { pools_healthy: 2, pools_total: 2 } } },
  });
  try {
    const h = await run('mesh', {
      PEER_STORAGE_URLS: `matthew=${matthew.baseUrl},jessica=${jessica.baseUrl}`,
      INTERNAL_DOORWAY_URL: doorway.baseUrl,
    });
    assertEnvelope(h, 'mesh');
    eqDeep(
      inventory(h.report),
      [
        'pass:mesh.matthew.connected',
        'pass:mesh.jessica.connected',
        'pass:mesh.reachable',
        'pass:mesh.adjacency',
        'pass:mesh.adjacency-reverse',
        'pass:mesh.version-parity',
      ],
      'mesh happy-path assertion inventory'
    );
    check(h.exitCode === 0, 'exit 0');
    check(
      detailOf(h.report, 'mesh.matthew.connected') === 'connectedPeers=3 nat=public',
      'connected detail matches bash format',
      detailOf(h.report, 'mesh.matthew.connected')
    );
    check(
      detailOf(h.report, N_MESH_VERSION_PARITY) ===
        'all reachable pods on one binary version (matthew=abc123 jessica=abc123)',
      'version-parity strips exactly one leading space',
      detailOf(h.report, N_MESH_VERSION_PARITY)
    );
    const peers = h.report.context['peers'] as Record<string, unknown>[];
    check(peers.length === 2, 'context.peers has both pods');
    eqDeep(
      peers[0]?.['connectedPeerIds'],
      [PID_JESSICA, 'pid-other'],
      'connectedPeerIds records WHO, with .peerId // .peer_id fallback'
    );
    check(
      h.lines.some(l => l.includes('doorway: p2p.peerCount=7 pools=2/2')),
      'doorway health block is informational only (a note, not an assertion)'
    );
  } finally {
    await Promise.all([matthew.close(), jessica.close(), doorway.close()]);
  }
}

async function meshUnreachablePeer(): Promise<void> {
  testCase('mesh — unreachable peer counts against MESH_MIN_REACHABLE');
  const matthew = await startFixture({
    '/p2p/status': P2P_STATUS_HEALTHY(PID_MATTHEW, 3),
    '/p2p/peers': { json: { peers: [] } },
    '/version': { json: { commit: 'abc123' } },
  });
  try {
    const h = await run('mesh', {
      PEER_STORAGE_URLS: `matthew=${matthew.baseUrl},jessica=${DEAD_PEER}`,
    });
    assertEnvelope(h, 'mesh');
    eqDeep(
      inventory(h.report),
      [
        'pass:mesh.matthew.connected',
        'warn:mesh.jessica.reachable',
        'fail:mesh.reachable',
        'warn:mesh.adjacency',
        // parity asymmetry, straight from bash: forward adjacency WARNs (the
        // target's peer-id is unknown, so the guard short-circuits), but
        // adjacency-reverse FAILS — `peer_url_for jessica` still resolves for
        // a declared-but-down peer, so bash enters the branch and the failed
        // GET lands in the else (bash:198-203).
        'fail:mesh.adjacency-reverse',
        'pass:mesh.version-parity',
      ],
      'mesh unreachable-peer assertion inventory'
    );
    check(h.exitCode === 1, 'exit 1 on a failed assertion');
    check(
      detailOf(h.report, 'mesh.reachable') === 'only 1 peer pod(s) reachable; floor is 2',
      'mesh.reachable failure detail matches bash',
      detailOf(h.report, 'mesh.reachable')
    );
    check(
      detailOf(h.report, 'mesh.jessica.reachable').startsWith(
        `storage HTTP unreachable (http://${DEAD_PEER}, status `
      ),
      'unreachable detail quotes the peer URL + last status',
      detailOf(h.report, 'mesh.jessica.reachable')
    );
    check(
      detailOf(h.report, 'mesh.adjacency') ===
        'skipped — matthew or jessica unreachable/unidentified',
      'adjacency warns (not fails) when the target id is unknown'
    );
  } finally {
    await matthew.close();
  }
}

async function meshVersionSkew(): Promise<void> {
  testCase('mesh — version skew is WARN-only (partial rollout is a real deploy state)');
  const matthew = await startFixture({
    '/p2p/status': P2P_STATUS_HEALTHY(PID_MATTHEW, 3),
    '/p2p/peers': { json: { peers: [{ peerId: PID_JESSICA }] } },
    '/version': { json: { commit: 'abc123' } },
  });
  const jessica = await startFixture({
    '/p2p/status': P2P_STATUS_HEALTHY(PID_JESSICA, 3),
    '/p2p/peers': { json: { peers: [{ peerId: PID_MATTHEW }] } },
    '/version': { json: { version: 'def456' } },
  });
  try {
    const h = await run('mesh', {
      PEER_STORAGE_URLS: `matthew=${matthew.baseUrl},jessica=${jessica.baseUrl}`,
    });
    const skew = h.report.assertions.find(a => a.name === N_MESH_VERSION_PARITY);
    check(skew?.status === 'warn', 'version skew is warn, never fail');
    check(
      skew?.detail ===
        'version skew across pods: matthew=abc123 jessica=def456 — partial rollout? (DHT-partition risk class)',
      'skew detail keeps the unstripped leading space after the colon',
      skew?.detail
    );
    check(h.exitCode === 0, 'warns do not change the exit code');
  } finally {
    await Promise.all([matthew.close(), jessica.close()]);
  }
}

async function uploadCases(): Promise<void> {
  const dir = mkdtempSync(join(tmpdir(), 'sv-content-'));
  const contentPath = join(dir, 'manifesto.md');
  writeFileSync(contentPath, '# manifesto\n');

  for (const [status, label] of [
    [201, 'upload — HTTP 201 passes and writes the STATE_DIR handoff'],
    [409, 'upload — HTTP 409 passes as idempotent (distinct detail from 200/201)'],
    [500, 'upload — HTTP 500 fails with the truncated error body'],
  ] as [number, string][]) {
    testCase(label);
    const seenAuth: (string | undefined)[] = [];
    const doorway = await startFixture({
      '/admin/seed/blob': (req: IncomingMessage) => {
        seenAuth.push(req.headers['authorization']);
        return status === 500
          ? { status, text: 'boom: storage refused the blob' }
          : { status, json: { ok: status !== 500 } };
      },
    });
    try {
      const h = await run('upload', {
        INTERNAL_DOORWAY_URL: doorway.baseUrl,
        CONTENT_PATH: contentPath,
        BUILD_TAG: 'selftest-42',
        GIT_COMMIT: 'deadbeef',
      });
      assertEnvelope(h, 'upload');
      const expected = status === 500 ? 'fail' : 'pass';
      eqDeep(
        inventory(h.report),
        [`${expected}:upload.content`, `${expected}:upload.probe`],
        `upload ${status} assertion inventory`
      );
      check(h.exitCode === (status === 500 ? 1 : 0), `exit ${status === 500 ? 1 : 0}`);
      check(
        seenAuth.every(a => a === undefined),
        'no Authorization header without SEED_DOORWAY_TOKEN'
      );

      const detail = detailOf(h.report, 'upload.content');
      if (status === 409) {
        check(
          detail.endsWith('→ HTTP 409 (already present, idempotent)'),
          '409 carries the idempotent wording',
          detail
        );
      } else if (status === 500) {
        check(
          detail === 'HTTP 500 — boom: storage refused the blob',
          '500 carries the response body (head -c 300)',
          detail
        );
      } else {
        check(detail.endsWith('(12 bytes) → HTTP 201'), '201 carries size + status', detail);
        check(
          detail.startsWith('sha256-'),
          'blob hash is sha256-prefixed hex from node:crypto',
          detail
        );
        // STATE_DIR handoff — Jenkins `cat`s these files between stages.
        for (const f of [
          'content_blob_hash',
          'content_blob_size',
          'probe_blob_hash',
          'probe_blob_size',
          'build_start_iso',
        ]) {
          check(existsSync(join(h.stateDir, f)), `state file ${f} written`);
        }
        check(
          readFileSync(join(h.stateDir, 'content_blob_size'), 'utf8').trim() === '12',
          'content_blob_size is the byte length'
        );
        const probe = readFileSync(join(h.stateDir, 'propagation-probe.txt'), 'utf8');
        check(
          probe.includes('build: selftest-42') && probe.includes('commit: deadbeef'),
          'probe blob is build-unique (BUILD_TAG + GIT_COMMIT + nonce)'
        );
        check(/nonce: [0-9a-f]{32}\n$/.test(probe), 'probe carries a 16-byte random nonce');
        eqDeep(
          Object.keys(h.report.context),
          ['contentBlobHash', 'probeBlobHash', 'buildStartIso'],
          'upload context payload keys match bash'
        );
      }
    } finally {
      await doorway.close();
    }
  }

  testCase('upload — bearer header present only with SEED_DOORWAY_TOKEN');
  const seenAuth: (string | undefined)[] = [];
  const doorway = await startFixture({
    '/admin/seed/blob': (req: IncomingMessage) => {
      seenAuth.push(req.headers['authorization']);
      return { status: 200, json: {} };
    },
  });
  try {
    await run('upload', {
      INTERNAL_DOORWAY_URL: doorway.baseUrl,
      CONTENT_PATH: contentPath,
      SEED_DOORWAY_TOKEN: 'tok-123',
    });
    check(
      seenAuth.every(a => a === 'Bearer tok-123'),
      'Authorization: Bearer sent when the token is exported'
    );
  } finally {
    await doorway.close();
  }
}

async function preflightAlwaysWritesReport(): Promise<void> {
  testCase('preflight failures still write the report (bash `finish` always writes first)');

  const upload = await run('upload', { CONTENT_PATH: '/nonexistent' });
  assertEnvelope(upload, 'upload');
  eqDeep(inventory(upload.report), ['fail:upload.preflight'], 'upload preflight inventory');
  eqDeep(upload.report.context, {}, 'upload preflight context is {}');
  check(upload.exitCode === 1, 'upload preflight exit 1');

  const prop = await run('propagation', { PEER_STORAGE_URLS: 'matthew=127.0.0.1:1' });
  assertEnvelope(prop, 'propagation');
  eqDeep(inventory(prop.report), ['fail:propagation.preflight'], 'propagation preflight inventory');
  check(
    detailOf(prop.report, 'propagation.preflight') === 'CONTENT_BLOB_HASH unset',
    'propagation preflight fails on the content hash first'
  );

  const delivery = await run('delivery', { PEER_STORAGE_URLS: 'matthew=127.0.0.1:1' });
  assertEnvelope(delivery, 'delivery');
  eqDeep(inventory(delivery.report), ['fail:delivery.preflight'], 'delivery preflight inventory');
  check(
    detailOf(delivery.report, 'delivery.preflight') ===
      'no build-start timestamp (upload stage skipped?)',
    'delivery preflight detail matches bash'
  );

  const fed = await run('federation', {});
  assertEnvelope(fed, 'federation');
  eqDeep(inventory(fed.report), ['fail:federation.preflight'], 'federation preflight inventory');

  // Missing content file — after build_start_iso is already written.
  const missing = await run('upload', {
    INTERNAL_DOORWAY_URL: '127.0.0.1:1',
    CONTENT_PATH: '/nope/manifesto.md',
  });
  eqDeep(inventory(missing.report), ['fail:upload.content'], 'missing content file inventory');
  check(
    existsSync(join(missing.stateDir, 'build_start_iso')),
    'build_start_iso is written BEFORE the content-file check (bash ordering)'
  );
}

/** Full-fixture peer used by propagation/delivery cases. */
async function propagationPeer(opts: {
  blobs: Set<string>;
  custodyFor?: string;
  parity?: number[];
  events?: number;
}): Promise<Fixture> {
  const parity = opts.parity ?? [];
  return startFixture({
    '/blob': (req: IncomingMessage) => {
      const hash = decodeURIComponent((req.url ?? '').replace('/blob/', ''));
      return opts.blobs.has(hash) ? { status: 200, text: 'bytes' } : { status: 404, text: '' };
    },
    '/api/v1/commitments': {
      json: opts.custodyFor ? CUSTODY_ROWS(opts.custodyFor).json : [],
    },
    '/api/v1/diagnostics/inventory-parity': (_req: IncomingMessage, hit: number) =>
      parity.length === 0
        ? { status: 500, text: 'no such route' }
        : { json: { filesystem_count: parity[Math.min(hit - 1, parity.length - 1)] } },
    '/api/v1/economic-events': {
      json: Array.from({ length: opts.events ?? 0 }, (_, i) => ({ id: i, action: 'serve-blob' })),
    },
  });
}

async function propagationHappyPath(): Promise<void> {
  testCase('propagation — happy path (probe served immediately, manifest converged)');
  const CONTENT = 'sha256-content';
  const PROBE = PROBE_HASH;
  const src = await propagationPeer({
    blobs: new Set([CONTENT, PROBE]),
    custodyFor: CONTENT,
    parity: [5],
  });
  const tgt = await propagationPeer({
    blobs: new Set([CONTENT, PROBE]),
    custodyFor: CONTENT,
    parity: [5, 6],
  });
  try {
    const h = await run('propagation', {
      PEER_STORAGE_URLS: `matthew=${src.baseUrl},jessica=${tgt.baseUrl}`,
      CONTENT_BLOB_HASH: CONTENT,
      PROBE_BLOB_HASH: PROBE,
    });
    assertEnvelope(h, 'propagation');
    eqDeep(
      inventory(h.report),
      [
        'pass:propagation.custody-manifest',
        'pass:propagation.source-content',
        'pass:propagation.source-probe',
        'pass:propagation.probe-fetch',
        'pass:propagation.content-fetch',
        'pass:propagation.bytes-persisted',
        'pass:propagation.custody-convergence',
      ],
      'propagation happy-path assertion inventory'
    );
    check(h.exitCode === 0, 'exit 0');
    check(h.sleeps.length === 0, 'no polling when the probe is served on attempt 1');
    eqDeep(
      h.report.context,
      {
        contentBlobHash: CONTENT,
        probeBlobHash: PROBE,
        waitedSecs: 0,
        attempts: 1,
        custodyManifestMissingOn: [],
      },
      'propagation context payload matches bash'
    );
    check(
      detailOf(h.report, 'propagation.bytes-persisted') ===
        'jessica filesystem_count 5 → 6 (replica on disk)',
      'bytes-persisted reads the inventory-parity delta',
      detailOf(h.report, 'propagation.bytes-persisted')
    );
    check(
      src.seen.some(s => s.startsWith(`GET /blob/${PROBE}`)),
      'blob checks are real GETs (heal-on-read), never HEAD'
    );
    check(!src.seen.some(s => s.startsWith('HEAD ')), 'no HEAD request was ever issued');
  } finally {
    await Promise.all([src.close(), tgt.close()]);
  }
}

async function propagationSharedBudget(): Promise<void> {
  testCase('propagation — both polls share ONE timeout budget (custody gets the REMAINDER)');
  const CONTENT = 'sha256-content';
  const PROBE = PROBE_HASH;
  const src = await propagationPeer({ blobs: new Set([CONTENT, PROBE]), custodyFor: CONTENT });
  // Target never serves the probe and never sees the custody row.
  const tgt = await propagationPeer({ blobs: new Set([CONTENT]) });
  try {
    const h = await run('propagation', {
      PEER_STORAGE_URLS: `matthew=${src.baseUrl},jessica=${tgt.baseUrl}`,
      CONTENT_BLOB_HASH: CONTENT,
      PROBE_BLOB_HASH: PROBE,
      PROPAGATION_TIMEOUT_SECS: '30',
      PROPAGATION_POLL_SECS: '15',
    });
    assertEnvelope(h, 'propagation');
    eqDeep(
      inventory(h.report),
      [
        'pass:propagation.custody-manifest',
        'pass:propagation.source-content',
        'pass:propagation.source-probe',
        'warn:propagation.parity-baseline',
        'fail:propagation.probe-fetch',
        'pass:propagation.content-fetch',
        'fail:propagation.custody-convergence',
      ],
      'propagation exhausted-budget assertion inventory'
    );
    check(h.exitCode === 1, 'exit 1');
    // 0 ≤ 30, 15 ≤ 30, 30 ≤ 30 → 3 attempts, then waited=45 (bash sleeps on
    // the last iteration too). The custody poll then sees waited(45) >= 30 and
    // breaks WITHOUT a single extra sleep — proof of the shared budget.
    check(
      h.report.context['attempts'] === 3,
      `probe poll ran 3 attempts (got ${h.report.context['attempts']})`
    );
    check(
      h.report.context['waitedSecs'] === 45,
      `waitedSecs=45 — one poll past the budget, as bash reports (got ${h.report.context['waitedSecs']})`
    );
    eqDeep(
      h.sleeps,
      [15, 15, 15],
      'exactly 3 sleeps total — the custody poll got ZERO fresh budget'
    );
    eqDeep(
      h.report.context['custodyManifestMissingOn'],
      ['jessica'],
      'custodyManifestMissingOn names the lagging pod'
    );
    check(
      detailOf(h.report, 'propagation.custody-convergence').includes(
        'missing on: jessica after 45s'
      ),
      'custody-convergence detail keeps bash\'s "missing on: <space-joined>" rendering',
      detailOf(h.report, 'propagation.custody-convergence')
    );
    check(
      detailOf(h.report, 'propagation.probe-fetch').includes('→ 404 after 45s / 3 attempts'),
      'probe-fetch failure quotes the last status, waited, attempts',
      detailOf(h.report, 'propagation.probe-fetch')
    );
  } finally {
    await Promise.all([src.close(), tgt.close()]);
  }
}

async function propagationStatePrecedence(): Promise<void> {
  testCase('propagation — env var overrides the STATE_DIR file; state file is the fallback');
  const ENV_HASH = 'sha256-from-env';
  const FILE_HASH = 'sha256-from-state-file';
  const PROBE = PROBE_HASH;
  const src = await propagationPeer({
    blobs: new Set([ENV_HASH, FILE_HASH, PROBE]),
    custodyFor: ENV_HASH,
  });
  const tgt = await propagationPeer({ blobs: new Set([ENV_HASH, FILE_HASH, PROBE]) });
  const stateDir = mkdtempSync(join(tmpdir(), 'sv-state-'));
  writeFileSync(join(stateDir, 'content_blob_hash'), `${FILE_HASH}\n`);
  writeFileSync(join(stateDir, 'probe_blob_hash'), `${PROBE}\n`);
  const base = {
    PEER_STORAGE_URLS: `matthew=${src.baseUrl},jessica=${tgt.baseUrl}`,
    STATE_DIR: stateDir,
    PROPAGATION_TIMEOUT_SECS: '0',
    PROPAGATION_POLL_SECS: '1',
  };
  try {
    const withEnv = await run('propagation', { ...base, CONTENT_BLOB_HASH: ENV_HASH });
    check(
      withEnv.report.context['contentBlobHash'] === ENV_HASH,
      'CONTENT_BLOB_HASH env wins over the state file',
      String(withEnv.report.context['contentBlobHash'])
    );
    check(
      withEnv.report.context['probeBlobHash'] === PROBE,
      'PROBE_BLOB_HASH falls back to the state file when unset'
    );

    const fromFile = await run('propagation', base);
    check(
      fromFile.report.context['contentBlobHash'] === FILE_HASH,
      'state file is used when the env var is unset',
      String(fromFile.report.context['contentBlobHash'])
    );

    const emptyEnv = await run('propagation', { ...base, CONTENT_BLOB_HASH: '' });
    check(
      emptyEnv.report.context['contentBlobHash'] === FILE_HASH,
      'an EMPTY env var also falls back (bash ${VAR:-...} semantics)'
    );
  } finally {
    await Promise.all([src.close(), tgt.close()]);
  }
}

async function deliveryCases(): Promise<void> {
  testCase('delivery — synthetic heal forces serve-blob emission, then counts events');
  const PROBE = PROBE_HASH;
  const matthew = await propagationPeer({ blobs: new Set([PROBE]), events: 2 });
  const jessica = await propagationPeer({ blobs: new Set([PROBE]), events: 1 });
  try {
    const h = await run('delivery', {
      PEER_STORAGE_URLS: `matthew=${matthew.baseUrl},jessica=${jessica.baseUrl}`,
      PROBE_BLOB_HASH: PROBE,
      SUBSTRATE_BUILD_START_ISO: AFTER_ISO,
    });
    assertEnvelope(h, 'delivery');
    eqDeep(inventory(h.report), [`pass:${N_DELIVERY_EVENTS}`], 'delivery happy-path inventory');
    check(h.exitCode === 0, 'exit 0');
    eqDeep(h.sleeps, [12], 'DELIVERY_HEAL_SETTLE_SECS=12 settle sleep after the forced heal');
    check(
      matthew.seen.filter(s => s.startsWith(`GET /blob/${PROBE}`)).length === 1 &&
        jessica.seen.filter(s => s.startsWith(`GET /blob/${PROBE}`)).length === 1,
      'the heal GET is issued against EVERY peer'
    );
    eqDeep(h.report.context['perPeer'], { matthew: 2, jessica: 1 }, 'perPeer counts');
    check(h.report.context['after'] === AFTER_ISO, 'context.after echoes the window');
    check(
      detailOf(h.report, N_DELIVERY_EVENTS) ===
        '3 serve-blob event(s) across peers — transfers leave an REA delivery trail',
      'delivery pass detail matches bash'
    );
  } finally {
    await Promise.all([matthew.close(), jessica.close()]);
  }

  testCase('delivery — 0 events fails with the "despite a forced synthetic heal" wording');
  const empty = await propagationPeer({ blobs: new Set([PROBE]), events: 0 });
  try {
    const h = await run('delivery', {
      PEER_STORAGE_URLS: `matthew=${empty.baseUrl}`,
      PROBE_BLOB_HASH: PROBE,
      SUBSTRATE_BUILD_START_ISO: AFTER_ISO,
    });
    eqDeep(inventory(h.report), [`fail:${N_DELIVERY_EVENTS}`], 'delivery zero-event inventory');
    check(h.exitCode === 1, 'exit 1');
    check(
      detailOf(h.report, N_DELIVERY_EVENTS) ===
        `0 serve-blob events since 2026-08-15T00:00:00Z despite a forced synthetic heal of ${PROBE} — ` +
          "the event-emission leg (blob_fetch atomic pair) regressed (a real fetch was forced, so 'no transfer happened' is ruled out)",
      'zero-event detail matches bash conditional interpolation',
      detailOf(h.report, N_DELIVERY_EVENTS)
    );
  } finally {
    await empty.close();
  }

  testCase('delivery — PROBE_BLOB_HASH is read from the ENV ONLY (bash:425), never the state file');
  const noProbe = await propagationPeer({ blobs: new Set(), events: 1 });
  const stateDir = mkdtempSync(join(tmpdir(), 'sv-state-'));
  writeFileSync(join(stateDir, 'probe_blob_hash'), `${PROBE}\n`);
  writeFileSync(join(stateDir, 'build_start_iso'), '2026-08-15T00:00:00Z\n');
  try {
    const h = await run('delivery', {
      PEER_STORAGE_URLS: `matthew=${noProbe.baseUrl}`,
      STATE_DIR: stateDir,
    });
    eqDeep(
      inventory(h.report),
      ['warn:delivery.probe', `pass:${N_DELIVERY_EVENTS}`],
      'no-probe inventory warns rather than healing'
    );
    check(h.sleeps.length === 0, 'no settle sleep without a probe');
    check(
      !noProbe.seen.some(s => s.startsWith('GET /blob/')),
      'no heal GET issued — the state-file probe hash is deliberately NOT consulted'
    );
    check(
      h.report.context['after'] === AFTER_ISO,
      'build_start_iso IS read from the state file (only the probe hash is env-only)'
    );
  } finally {
    await noProbe.close();
  }

  testCase('delivery — unreachable economic-events query warns per peer and counts 0');
  const alive = await propagationPeer({ blobs: new Set(), events: 4 });
  try {
    const h = await run('delivery', {
      PEER_STORAGE_URLS: `matthew=${alive.baseUrl},jessica=${DEAD_PEER}`,
      SUBSTRATE_BUILD_START_ISO: AFTER_ISO,
    });
    eqDeep(
      inventory(h.report),
      ['warn:delivery.probe', 'warn:delivery.jessica', `pass:${N_DELIVERY_EVENTS}`],
      'per-peer unreachable warn uses the bare `delivery.<name>` name'
    );
    eqDeep(h.report.context['perPeer'], { matthew: 4, jessica: 0 }, 'unreachable peer counts 0');
  } finally {
    await alive.close();
  }
}

async function projectionTriState(): Promise<void> {
  testCase('projection — tri-state stream matrix + projector lag branches');
  const OK_PROJECTOR = { json: { lag: [{ cursor: 'a', lagSeconds: 3 }] } };

  const caught = await startFixture({
    '/api/v1/status/projector': OK_PROJECTOR,
    '/p2p/status': {
      json: {
        replication: { caughtUp: true },
        pull: { total: 5, caughtUp: true },
        projectionReconcile: { caughtUp: true },
      },
    },
  });
  const idle = await startFixture({
    '/api/v1/status/projector': OK_PROJECTOR,
    '/p2p/status': {
      json: {
        replication: { caught_up: true },
        pull: { total: 0 },
        projection_reconcile: { caught_up: true },
      },
    },
  });
  const nul = await startFixture({
    '/api/v1/status/projector': OK_PROJECTOR,
    '/p2p/status': {
      json: { replication: { caughtUp: true }, projectionReconcile: { caughtUp: true } },
    },
  });
  const behind = await startFixture({
    '/api/v1/status/projector': {
      json: {
        lag: [
          { cursor: 'x', lagSeconds: 999 },
          { cursor: 'y', lagSeconds: 1 },
        ],
      },
    },
    '/p2p/status': {
      json: {
        replication: { caughtUp: true },
        pull: { total: 9, caughtUp: false },
        projectionReconcile: { caughtUp: true },
      },
    },
  });
  const unparseable = await startFixture({
    // A bare ARRAY body — jq's `.lag[]?` ERRORS on it, bash records NOTHING.
    '/api/v1/status/projector': { json: ['not', 'an', 'object'] },
    '/p2p/status': {
      json: {
        replication: { caughtUp: true },
        pull: { total: 1, caughtUp: true },
        projectionReconcile: { caughtUp: true },
      },
    },
  });
  // A stream object PRESENT but carrying no caughtUp/caught_up — the one
  // documented divergence: bash rendered this `false` (FAIL), the facade
  // helper renders it 'not-computable' → "null" (WARN).
  const renamed = await startFixture({
    '/api/v1/status/projector': OK_PROJECTOR,
    '/p2p/status': {
      json: {
        replication: { caughtUp: true },
        pull: { total: 4, fetchedOK: 4 },
        projectionReconcile: { caughtUp: true },
      },
    },
  });
  const dead = { name: 'dead', url: DEAD_PEER };

  try {
    const h = await run('projection', {
      PEER_STORAGE_URLS: [
        `caught=${caught.baseUrl}`,
        `idle=${idle.baseUrl}`,
        `nul=${nul.baseUrl}`,
        `behind=${behind.baseUrl}`,
        `unparseable=${unparseable.baseUrl}`,
        `renamed=${renamed.baseUrl}`,
        `${dead.name}=${dead.url}`,
      ].join(','),
      PROJECTION_RETRIES: '1',
      PROJECTION_MAX_LAG_SECS: '120',
    });
    assertEnvelope(h, 'projection');
    eqDeep(
      inventory(h.report),
      [
        'pass:projection.caught.lag',
        'pass:projection.caught.streams',
        'pass:projection.idle.lag',
        'warn:projection.idle.streams',
        'pass:projection.nul.lag',
        'warn:projection.nul.streams',
        'fail:projection.behind.lag',
        'fail:projection.behind.streams',
        // unparseable: NO .lag assertion (bash skips silently), streams pass.
        'pass:projection.unparseable.streams',
        'pass:projection.renamed.lag',
        'fail:projection.renamed.streams',
        'warn:projection.dead.projector',
        'fail:projection.dead.streams',
      ],
      'projection tri-state assertion inventory'
    );
    check(
      detailOf(h.report, 'projection.renamed.streams').startsWith(
        'replication=true pull=false projection_reconcile=true'
      ),
      'FAIL-CLOSED PARITY: a PRESENT stream with a renamed caughtUp field renders "false" and FAILS, matching bash ("a schema rename must fail closed via the caughtUp path")',
      detailOf(h.report, 'projection.renamed.streams')
    );
    check(h.exitCode === 1, 'exit 1');
    check(
      detailOf(h.report, 'projection.caught.streams') ===
        'replication=true pull=true projection_reconcile=true',
      'caught-up detail string',
      detailOf(h.report, 'projection.caught.streams')
    );
    check(
      detailOf(h.report, 'projection.idle.streams').startsWith(
        'replication=true pull=idle projection_reconcile=true — pull total==0'
      ),
      'pull.total==0 renders "idle" and warns (never a clean pass)',
      detailOf(h.report, 'projection.idle.streams')
    );
    check(
      detailOf(h.report, 'projection.nul.streams').startsWith(
        'replication=true pull=null projection_reconcile=true — null streams are'
      ),
      'a missing stream renders "null" and warns ("not computable")',
      detailOf(h.report, 'projection.nul.streams')
    );
    check(
      detailOf(h.report, 'projection.behind.streams') ===
        'replication=true pull=false projection_reconcile=true after 1 attempt(s)',
      'pull=false fails after PROJECTION_RETRIES attempts',
      detailOf(h.report, 'projection.behind.streams')
    );
    check(
      detailOf(h.report, 'projection.behind.lag') ===
        '1 cursor(s) lag > 120s: [{"cursor":"x","lagSeconds":999}]',
      'lag failure quotes only the over-threshold cursors as compact JSON',
      detailOf(h.report, 'projection.behind.lag')
    );
    check(
      !h.report.assertions.some(
        a => a.name.startsWith('projection.unparseable.') && a.name.endsWith('.lag')
      ),
      'parity: an unparseable projector body records NO lag assertion (bash skips silently)'
    );
    check(
      detailOf(h.report, 'projection.dead.streams').startsWith('p2p/status unreachable ('),
      'unreachable peer fails streams with the last-status detail',
      detailOf(h.report, 'projection.dead.streams')
    );
    // Only the failing-stream peers sleep (10s per non-breaking attempt):
    // behind, dead, and renamed (fail-closed on a present-but-renamed field).
    eqDeep(h.sleeps, [10, 10, 10], 'a 10s sleep per non-breaking stream attempt');
    eqDeep(h.report.context, {}, 'projection context is {}');
  } finally {
    await Promise.all([
      caught.close(),
      idle.close(),
      nul.close(),
      behind.close(),
      unparseable.close(),
      renamed.close(),
    ]);
  }
}

async function federationCases(): Promise<void> {
  testCase('federation — doorways online + bootstrap surface + shard capability');
  const doorway = await startFixture({
    '/api/v1/federation/doorways': {
      json: { doorways: [{ status: 'online' }, { status: 'offline' }] },
    },
    '/api/v1/federation/p2p-peers': {
      json: { total: 3, peers: [{ capabilities: ['shard', 'blob'] }, { capabilities: [] }] },
    },
  });
  try {
    const h = await run('federation', { INTERNAL_DOORWAY_URL: doorway.baseUrl });
    assertEnvelope(h, 'federation');
    eqDeep(
      inventory(h.report),
      ['pass:federation.doorways', 'pass:federation.p2p-peers', 'pass:federation.shard-capability'],
      'federation happy-path inventory'
    );
    check(
      detailOf(h.report, 'federation.doorways') === '2 doorway(s) known, 1 online',
      'doorways detail counts known + online',
      detailOf(h.report, 'federation.doorways')
    );
    check(h.exitCode === 0, 'exit 0');
  } finally {
    await doorway.close();
  }

  testCase('federation — no online doorway / empty bootstrap surface / no shard capability');
  const degraded = await startFixture({
    '/api/v1/federation/doorways': { json: { doorways: [{ status: 'offline' }] } },
    '/api/v1/federation/p2p-peers': { json: { total: 0, peers: [] } },
  });
  try {
    const h = await run('federation', { INTERNAL_DOORWAY_URL: degraded.baseUrl });
    eqDeep(
      inventory(h.report),
      ['fail:federation.doorways', 'fail:federation.p2p-peers', 'warn:federation.shard-capability'],
      'federation degraded inventory (shard is WARN-class)'
    );
    check(
      detailOf(h.report, 'federation.p2p-peers') === 'bootstrap surface empty: {"total":0}',
      "empty bootstrap detail renders jq -c '{total}'",
      detailOf(h.report, 'federation.p2p-peers')
    );
    check(h.exitCode === 1, 'exit 1');
  } finally {
    await degraded.close();
  }
}

async function resilienceCases(): Promise<void> {
  testCase('resilience — checks ONLY the first reachable peer (fleet view from one pod)');
  const first = await startFixture({
    '/api/v1/peer-statuses': { json: [{ peer: 'a' }, { peer: 'b' }] },
    '/api/v1/network/posture': {
      json: { activePeers: 2, totalPeers: 3, stalePeers: 1, householdsReciprocating: 1 },
    },
  });
  const second = await startFixture({
    '/api/v1/peer-statuses': { json: [{ peer: 'c' }] },
    '/api/v1/network/posture': { json: { activePeers: 1 } },
  });
  try {
    const h = await run('resilience', {
      PEER_STORAGE_URLS: `matthew=${first.baseUrl},jessica=${second.baseUrl}`,
    });
    assertEnvelope(h, 'resilience');
    eqDeep(
      inventory(h.report),
      ['pass:resilience.matthew.peer-statuses', 'pass:resilience.matthew.posture'],
      'resilience stops at the first answering pod — NOT a fan-out aggregate'
    );
    check(second.seen.length === 0, 'the second peer was never queried');
    check(
      detailOf(h.report, 'resilience.matthew.posture') ===
        'activePeers=2 {"totalPeers":3,"stalePeers":1,"householdsReciprocating":1}',
      'posture detail matches jq -c projection',
      detailOf(h.report, 'resilience.matthew.posture')
    );
    check(h.exitCode === 0, 'exit 0');
  } finally {
    await Promise.all([first.close(), second.close()]);
  }

  testCase('resilience — first peer unreachable, second answers; and nobody answers');
  const alive = await startFixture({
    '/api/v1/peer-statuses': { json: [] },
    '/api/v1/network/posture': { json: { activePeers: 0, totalPeers: 0 } },
  });
  try {
    const h = await run('resilience', {
      PEER_STORAGE_URLS: `dead=${DEAD_PEER},jessica=${alive.baseUrl}`,
    });
    eqDeep(
      inventory(h.report),
      ['fail:resilience.jessica.peer-statuses', 'fail:resilience.jessica.posture'],
      'an unreachable peer is skipped silently; the next one answers for the fleet'
    );
    check(
      detailOf(h.report, 'resilience.jessica.posture') ===
        'activePeers=0: {"activePeers":0,"totalPeers":0}',
      'parity: the failure detail hardcodes "activePeers=0:" plus the raw body',
      detailOf(h.report, 'resilience.jessica.posture')
    );
  } finally {
    await alive.close();
  }

  const none = await run('resilience', { PEER_STORAGE_URLS: `dead=${DEAD_PEER}` });
  eqDeep(inventory(none.report), ['fail:resilience.reachable'], 'no pod answered inventory');
  check(none.exitCode === 1, 'exit 1 when no pod answers');
}

function cliUsageExitCode(): void {
  testCase('CLI — unknown/missing subcommand prints usage and exits 2');
  const script = fileURLToPath(new URL('./substrate-verify.ts', import.meta.url));
  // Absolute node + absolute tsx CLI — no PATH lookup, so the harness cannot
  // pick up a shadowing binary.
  const tsxCli = createRequire(import.meta.url).resolve('tsx/cli');
  for (const args of [[], ['bogus']]) {
    const res = spawnSync(process.execPath, [tsxCli, script, ...args], { encoding: 'utf8' });
    const label = args.length === 0 ? 'no args' : 'unknown command';
    check(res.status === 2, `${label} → exit 2 (got ${res.status})`, res.stderr?.slice(0, 400));
    check((res.stdout ?? '').includes(USAGE_LINE), `${label} → usage line on stdout`, res.stdout);
  }
}

// ===========================================================================

async function main(): Promise<void> {
  console.log('substrate-verify selftest — parity harness for the bash → facade port');
  await meshHappyPath();
  await meshUnreachablePeer();
  await meshVersionSkew();
  await uploadCases();
  await preflightAlwaysWritesReport();
  await propagationHappyPath();
  await propagationSharedBudget();
  await propagationStatePrecedence();
  await deliveryCases();
  await projectionTriState();
  await federationCases();
  await resilienceCases();
  cliUsageExitCode();

  console.log(`\n${'─'.repeat(59)}`);
  if (failures === 0) {
    console.log(`PASS — ${checks} checks, 0 failures`);
  } else {
    console.log(`FAIL — ${checks} checks, ${failures} failure(s)`);
  }
  process.exitCode = failures === 0 ? 0 : 1;
}

main().catch(e => {
  const where = currentCase === '' ? '' : ` in: ${currentCase}`;
  console.error(`selftest crashed${where}`);
  console.error(e instanceof Error ? (e.stack ?? e.message) : String(e));
  process.exitCode = 2;
});
