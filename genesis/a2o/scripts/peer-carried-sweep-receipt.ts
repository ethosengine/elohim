/**
 * Household-mesh receipt for the sweep-driven peer-carried election arm.
 *
 * The receipt deliberately makes Jessica's conductor unable to answer while
 * Jessica's storage process remains alive and continues its conductor-free
 * discovery sweep. No election function is called on Jessica.
 *
 * Exit codes:
 *   0 — peer_carried moved and Jessica serves Matthew's elected head;
 *   1 — fixture/precondition/control failure;
 *   2 — the sweep stalled at a named station (the story-graph residue).
 *
 * Run from genesis/a2o:
 *   pnpm exec tsx scripts/peer-carried-sweep-receipt.ts
 */
import { execFileSync } from 'node:child_process';
import { readFileSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// eslint-disable-next-line import/no-extraneous-dependencies -- local mesh probe dependency
import { AdminWebsocket, AppWebsocket, encodeHashToBase64, type CellId } from '@holochain/client';

const APP_ID = 'elohim';
const SUPPLIER = process.env.PEER_CARRIED_SUPPLIER ?? 'http://localhost:8090';
const ADOPTER = process.env.PEER_CARRIED_ADOPTER ?? 'http://localhost:8091';
const SUPPLIER_ADMIN = Number(process.env.PEER_CARRIED_SUPPLIER_ADMIN ?? '4444');
const SUPPLIER_APP = Number(process.env.PEER_CARRIED_SUPPLIER_APP ?? '4445');
const ADOPTER_ADMIN = Number(process.env.PEER_CARRIED_ADOPTER_ADMIN ?? '4454');
const ADOPTER_APP = Number(process.env.PEER_CARRIED_ADOPTER_APP ?? '4455');
const ADOPTER_CONFIG =
  process.env.PEER_CARRIED_ADOPTER_CONFIG ??
  '/projects/elohim/elohim/holochain/local-dev/jessica/conductor-config.yaml';
const ADOPTER_LOG =
  process.env.PEER_CARRIED_ADOPTER_LOG ?? join(tmpdir(), 'elohim-local-mesh/logs/jessica.log');
const CONTROL_WAIT_MS = Number(process.env.PEER_CARRIED_CONTROL_WAIT_MS ?? 90_000);
const RECEIPT_WAIT_MS = Number(process.env.PEER_CARRIED_RECEIPT_WAIT_MS ?? 110_000);
const POLL_MS = Number(process.env.PEER_CARRIED_POLL_MS ?? 5_000);

interface ConductorClient {
  call: (fnName: string, payload: unknown) => Promise<unknown>;
  agent: string;
}

interface ElectionMetrics {
  attempted: number;
  noElection: number;
  resolveError: number;
  fetchFailed: number;
  validateFailed: number;
  carried: number;
  peerCarried: number;
}

interface ReceiptObservation {
  served: string | null;
  metrics: ElectionMetrics;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

async function sleep(ms: number): Promise<void> {
  await new Promise(resolve => setTimeout(resolve, ms));
}

async function conductor(adminPort: number, appPort: number): Promise<ConductorClient> {
  const wsClientOptions = { origin: APP_ID };
  const admin = await AdminWebsocket.connect({
    url: new URL(`ws://127.0.0.1:${adminPort}`),
    wsClientOptions,
  });
  const apps = await admin.listApps({});
  const app = apps.find(candidate => candidate.installed_app_id === APP_ID) ?? apps[0];
  if (!app) throw new Error(`no installed app on conductor admin port ${adminPort}`);

  const cells = new Map<string, CellId>();
  for (const [role, infos] of Object.entries(app.cell_info)) {
    for (const info of infos) {
      if (info.type === 'provisioned') {
        cells.set(role, info.value.cell_id);
      }
    }
  }
  const learningCell = cells.get('lamad');
  if (!learningCell) throw new Error(`no learning cell on conductor admin port ${adminPort}`);
  await admin.authorizeSigningCredentials(learningCell);
  const token = await admin.issueAppAuthenticationToken({ installed_app_id: app.installed_app_id });
  const appWs = await AppWebsocket.connect({
    url: new URL(`ws://127.0.0.1:${appPort}`),
    token: token.token,
    wsClientOptions,
  });
  return {
    agent: encodeHashToBase64(learningCell[1]),
    call: async (fnName, payload) =>
      await appWs.callZome({
        cell_id: learningCell,
        zome_name: 'content_store',
        fn_name: fnName,
        payload,
      }),
  };
}

async function requireHealthy(base: string): Promise<void> {
  const response = await fetch(`${base}/health`, { signal: AbortSignal.timeout(5_000) });
  if (!response.ok) throw new Error(`${base}/health returned ${response.status}`);
  const health: unknown = await response.json();
  if (!isRecord(health) || health.status !== 'ok')
    throw new Error(`${base}/health is not ok: ${JSON.stringify(health)}`);
}

async function authorDeclare(
  storage: string,
  id: string,
  body: string,
  agent: string
): Promise<string> {
  const bulk = await fetch(`${storage}/db/content/bulk`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify([
      {
        id,
        title: `Peer-carried sweep receipt (${storage})`,
        description: 'household mesh receipt fixture',
        contentType: 'concept',
        contentFormat: 'markdown',
        content: body,
        reach: 'commons',
      },
    ]),
  });
  if (!bulk.ok) throw new Error(`bulk create on ${storage}: ${bulk.status} ${await bulk.text()}`);

  const patch = await fetch(`${storage}/db/content/${id}`, {
    method: 'PATCH',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ contentBody: body, reach: 'commons' }),
  });
  const patched: unknown = await patch.json().catch(() => ({}));
  if (!patch.ok) throw new Error(`PATCH on ${storage}: ${patch.status} ${JSON.stringify(patched)}`);
  const head = isRecord(patched) ? patched.dhtAnchorHash : undefined;
  if (!head) throw new Error(`PATCH on ${storage} returned no dhtAnchorHash`);

  const declare = await fetch(`${storage}/db/content/${id}/head`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'X-Agent-Cid': agent },
    body: JSON.stringify({ headActionHash: head }),
  });
  if (!declare.ok) {
    throw new Error(`declare on ${storage}: ${declare.status} ${await declare.text()}`);
  }
  console.log(`  ${storage}: authored + declared ${head}`);
  return String(head);
}

async function declareEarned(client: ConductorClient, id: string, head: string): Promise<void> {
  const earned = await client.call('declare_earned_canonical_head', {
    id,
    head_action_hash: head,
    carried_record: null,
    adopt_before_author: false,
    delegation: null,
  });
  if (!isRecord(earned) || !(earned.head_action_hash instanceof Uint8Array)) {
    throw new Error(`earned declaration returned an invalid wire shape: ${JSON.stringify(earned)}`);
  }
  const winner = encodeHashToBase64(earned.head_action_hash);
  if (earned.canonical !== true || winner !== head) {
    throw new Error(`earned declaration chose ${winner}, expected ${head}`);
  }
  console.log(`  supplier declared EARNED canonical ${winner}`);
}

async function servedHead(storage: string, id: string): Promise<string | null> {
  try {
    const response = await fetch(`${storage}/db/content/${id}/head`, {
      signal: AbortSignal.timeout(5_000),
    });
    if (!response.ok) return null;
    const body: unknown = await response.json().catch(() => null);
    return isRecord(body) && typeof body.headActionHash === 'string' ? body.headActionHash : null;
  } catch {
    return null;
  }
}

function prometheusValue(text: string, name: string, labels: Record<string, string>): number {
  const prefix = `${name}{`;
  for (const line of text.split('\n')) {
    if (!line.startsWith(prefix)) continue;
    if (!Object.entries(labels).every(([key, value]) => line.includes(`${key}="${value}"`))) {
      continue;
    }
    const raw = line.trim().split(/\s+/).at(-1);
    const parsed = Number(raw);
    return Number.isFinite(parsed) ? parsed : 0;
  }
  return 0;
}

async function electionMetrics(storage: string): Promise<ElectionMetrics> {
  const response = await fetch(`${storage}/metrics`, { signal: AbortSignal.timeout(5_000) });
  if (!response.ok) throw new Error(`${storage}/metrics returned ${response.status}`);
  const text = await response.text();
  return {
    attempted: prometheusValue(text, 'elohim_content_election_obey_probe_total', {
      outcome: 'attempted',
    }),
    noElection: prometheusValue(text, 'elohim_content_election_obey_probe_total', {
      outcome: 'no_election',
    }),
    resolveError: prometheusValue(text, 'elohim_content_election_obey_probe_total', {
      outcome: 'resolve_error',
    }),
    fetchFailed: prometheusValue(text, 'elohim_content_election_obey_failed_total', {
      class: 'fetch',
    }),
    validateFailed: prometheusValue(text, 'elohim_content_election_obey_failed_total', {
      class: 'validate',
    }),
    carried: prometheusValue(text, 'elohim_content_election_obeyed_total', {
      path: 'carried',
    }),
    peerCarried: prometheusValue(text, 'elohim_content_election_obeyed_total', {
      path: 'peer_carried',
    }),
  };
}

function metricDelta(after: ElectionMetrics, before: ElectionMetrics): ElectionMetrics {
  return Object.fromEntries(
    Object.keys(before).map(key => [
      key,
      after[key as keyof ElectionMetrics] - before[key as keyof ElectionMetrics],
    ])
  ) as unknown as ElectionMetrics;
}

function conductorPid(configPath: string): number {
  const listing = execFileSync('/usr/bin/ps', ['-eo', 'pid=,args='], { encoding: 'utf8' });
  const matches = listing
    .split('\n')
    .map(line => line.trim())
    .filter(
      line =>
        line.includes('holochain') && line.includes('--config-path') && line.includes(configPath)
    )
    .map(line => Number(line.split(/\s+/, 1)[0]))
    .filter(Number.isInteger);
  if (matches.length !== 1) {
    throw new Error(
      `expected exactly one adopter conductor for ${configPath}, found ${matches.length}: ${matches.join(', ')}`
    );
  }
  return matches[0];
}

let pausedPid: number | undefined;

function resumeAdopter(): void {
  if (pausedPid === undefined) return;
  try {
    process.kill(pausedPid, 'SIGCONT');
    console.log(`cleanup: resumed adopter conductor pid ${pausedPid}`);
  } catch (error) {
    console.error(`cleanup: failed to resume adopter conductor pid ${pausedPid}: ${String(error)}`);
  }
  pausedPid = undefined;
}

for (const signal of ['SIGINT', 'SIGTERM'] as const) {
  process.once(signal, () => {
    resumeAdopter();
    process.exit(signal === 'SIGINT' ? 130 : 143);
  });
}

function readLogFrom(offset: number): string {
  const bytes = readFileSync(ADOPTER_LOG);
  return bytes.subarray(Math.min(offset, bytes.length)).toString('utf8');
}

function discoverySawGap(logTail: string): boolean {
  for (const line of logTail.split('\n')) {
    if (!line.includes('projection-reconcile: discovery complete')) continue;
    try {
      const entry: unknown = JSON.parse(line);
      const fields = isRecord(entry) && isRecord(entry.fields) ? entry.fields : undefined;
      if (Number(fields?.content_gaps ?? 0) > 0) return true;
    } catch {
      // A non-JSON log line is not evidence for this station.
    }
  }
  return false;
}

async function waitForHead(
  id: string,
  expected: string,
  waitMs: number,
  fallbackMetrics: ElectionMetrics
): Promise<ReceiptObservation> {
  const deadline = Date.now() + waitMs;
  let observation: ReceiptObservation = {
    served: await servedHead(ADOPTER, id),
    metrics: fallbackMetrics,
  };
  while (Date.now() < deadline) {
    if (observation.served === expected) return observation;
    await sleep(POLL_MS);
    let metrics = observation.metrics;
    try {
      metrics = await electionMetrics(ADOPTER);
    } catch {
      // A stopped conductor may make an adjacent HTTP observation transiently
      // unreachable. Preserve the last measured counters and keep the bound.
    }
    observation = {
      served: await servedHead(ADOPTER, id),
      metrics,
    };
  }
  return observation;
}

async function negativeControl(supplier: ConductorClient, adopter: ConductorClient): Promise<void> {
  const id = `peer-carried-control-${Date.now()}`;
  console.log(`negative control (adopter conductor UP): ${id}`);
  const before = await electionMetrics(ADOPTER);
  const elected = await authorDeclare(SUPPLIER, id, '# Control A\n\nElected.', supplier.agent);
  await authorDeclare(ADOPTER, id, '# Control B\n\nStale.', adopter.agent);
  await declareEarned(supplier, id, elected);
  const observed = await waitForHead(id, elected, CONTROL_WAIT_MS, before);
  const delta = metricDelta(observed.metrics, before);
  if (delta.peerCarried !== 0) {
    throw new Error(
      `negative control selected peer_carried with conductor up (delta=${delta.peerCarried}); fixture is not discriminating`
    );
  }
  console.log('  control result:', {
    elected,
    served: observed.served,
    peerCarriedDelta: delta.peerCarried,
    carriedDelta: delta.carried,
    note:
      observed.served === elected
        ? 'converged without peer_carried'
        : 'peer_carried label unaffected; ordinary DHT convergence did not land inside control bound',
  });
}

async function receiptAttempt(
  supplier: ConductorClient,
  adopter: ConductorClient
): Promise<number> {
  const id = `peer-carried-sweep-receipt-${Date.now()}`;
  console.log(`receipt attempt (adopter conductor STOPPED, storage sweeping): ${id}`);
  const elected = await authorDeclare(SUPPLIER, id, '# Receipt A\n\nElected.', supplier.agent);
  const stale = await authorDeclare(ADOPTER, id, '# Receipt B\n\nStale.', adopter.agent);
  if (elected === stale) throw new Error('fixture failed: supplier and adopter heads are equal');

  const before = await electionMetrics(ADOPTER);
  const logOffset = statSync(ADOPTER_LOG).size;
  pausedPid = conductorPid(ADOPTER_CONFIG);
  process.kill(pausedPid, 'SIGSTOP');
  console.log(
    `  adopter conductor pid ${pausedPid} stopped; adopter storage remains on ${ADOPTER}`
  );

  const started = Date.now();
  await declareEarned(supplier, id, elected);
  const observed = await waitForHead(id, elected, RECEIPT_WAIT_MS, before);
  const elapsedMs = Date.now() - started;
  const delta = metricDelta(observed.metrics, before);
  const logTail = readLogFrom(logOffset);
  const hintObserved = discoverySawGap(logTail);

  console.log('  receipt observation:', {
    elected,
    stale,
    served: observed.served,
    elapsedMs,
    hintObserved,
    metricDelta: delta,
  });

  if (delta.peerCarried > 0 && observed.served === elected) {
    console.log('PASS: peer_carried moved organically and the adopter serves the elected head');
    return 0;
  }

  let station: 'no-hint' | 'hint-no-fetch' | 'fetch-no-obey' | 'obey-no-serve';
  if (!hintObserved) station = 'no-hint';
  else if (delta.attempted === 0) station = 'hint-no-fetch';
  else if (delta.peerCarried === 0) station = 'fetch-no-obey';
  else station = 'obey-no-serve';

  console.error(
    `STALLED station=${station}: a stopped conductor cannot both answer observed absence and ` +
      'execute verify_carried_election + validate_carried_head_record; the sweep did not produce the receipt'
  );
  console.error(
    'STORY-GRAPH chain / between sweep-hint→peer_carried-adopt / missing node: ' +
      'fixture an observed-absent election read while the adopter conductor remains callable for wasm ' +
      `verification / probe=${id} counter_delta=${delta.peerCarried} served=${observed.served ?? 'absent'} ` +
      `/ current state=${station}`
  );
  return 2;
}

async function main(): Promise<void> {
  await Promise.all([requireHealthy(SUPPLIER), requireHealthy(ADOPTER)]);
  const [supplier, adopter] = await Promise.all([
    conductor(SUPPLIER_ADMIN, SUPPLIER_APP),
    conductor(ADOPTER_ADMIN, ADOPTER_APP),
  ]);
  console.log('fixture agents:', { supplier: supplier.agent, adopter: adopter.agent });

  await negativeControl(supplier, adopter);
  let exitCode = 1;
  try {
    exitCode = await receiptAttempt(supplier, adopter);
  } finally {
    resumeAdopter();
  }
  process.exit(exitCode);
}

main().catch(error => {
  resumeAdopter();
  console.error(String(error).slice(0, 1_200));
  process.exit(1);
});
