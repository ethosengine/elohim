/** Household-only correction fixtures. SQL reads are test witnesses, never writes. */
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

// Integration-only conductor client is intentionally a devDependency.
// eslint-disable-next-line import/no-extraneous-dependencies
import { decodeHashFromBase64, encodeHashToBase64 } from '@holochain/client';

import {
  connectConductor,
  meshConductorPorts,
  MESH_PEER_ORDER,
  type CarriedElectionRail,
} from '../../src/framework/dataplane/carried-election.js';
import { resolvePeerUrl, probeDeclaredHead } from '../../src/framework/dataplane/surfaces.js';
import { E2EWorld } from '../../src/framework/world.js';

export type Peer = 'matthew' | 'jessica' | 'james';
type Row = Record<string, unknown>;
export interface State {
  id: string;
  root: string;
  correction: string;
  acceptance: string;
  successor: string;
  branches: string[];
  chosen: string;
  sequentialPredecessor: string;
  operationId: string;
  operation?: Row;
  request?: Row;
  refusal?: string;
  baseline: Record<string, Row>;
  rails: Partial<Record<Peer, CarriedElectionRail>>;
  env: Partial<Record<Peer, Record<string, string>>>;
  originalConfig?: string;
  beforeRows?: Row[];
  beforeStanding?: Row;
  rebuilt?: number;
  crash: boolean;
}
const states = new WeakMap<E2EWorld, State>();
export function ctx(world: E2EWorld): State {
  let s = states.get(world);
  if (!s) {
    s = {
      id: `accountable-${randomUUID()}`,
      root: '',
      correction: '',
      acceptance: '',
      successor: '',
      branches: [],
      chosen: '',
      sequentialPredecessor: '',
      operationId: randomUUID(),
      baseline: {},
      rails: {},
      env: {},
      crash: false,
    };
    states.set(world, s);
  }
  return s;
}
export const hash = (value: unknown): string =>
  typeof value === 'string' ? value : encodeHashToBase64(value as Uint8Array);
export const raw = decodeHashFromBase64;
export const url = (peer: Peer): string => resolvePeerUrl(`E2E_STORAGE_${peer.toUpperCase()}`);
export async function rail(world: E2EWorld, peer: Peer): Promise<CarriedElectionRail> {
  const s = ctx(world);
  if (!s.rails[peer]) {
    const ports = meshConductorPorts(MESH_PEER_ORDER.indexOf(peer));
    s.rails[peer] = await connectConductor(ports.adminPort, ports.appPort);
  }
  return s.rails[peer];
}
interface ContentOutput {
  action_hash: Uint8Array;
  content: { metadata_json: string };
}
interface LineageOutput {
  contested: boolean;
  contested_predecessors: Uint8Array[];
  candidates: { action_hash: Uint8Array; predecessor: Uint8Array | null }[];
}
export function call(
  world: E2EWorld,
  peer: Peer,
  fn: 'create_content' | 'amend_content',
  payload: unknown
): Promise<ContentOutput>;
export function call(
  world: E2EWorld,
  peer: Peer,
  fn: 'get_content',
  payload: unknown
): Promise<ContentOutput | null>;
export function call(
  world: E2EWorld,
  peer: Peer,
  fn: 'get_content_lineage',
  payload: unknown
): Promise<LineageOutput>;
export function call(world: E2EWorld, peer: Peer, fn: string, payload: unknown): Promise<unknown>;
export async function call(
  world: E2EWorld,
  peer: Peer,
  fn: string,
  payload: unknown
): Promise<unknown> {
  return (await rail(world, peer)).call(fn, payload);
}
export async function http(peer: Peer, path: string, body?: unknown): Promise<Row> {
  const res = await fetch(`${url(peer)}${path}`, {
    method: body === undefined ? 'GET' : 'POST',
    headers: { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(60_000),
  });
  const text = await res.text();
  assert.ok(res.ok, `${peer} ${path}: HTTP ${res.status}: ${text}`);
  return JSON.parse(text) as Row;
}
// 180s left the poll budget SHORTER than the 240s step timeout wrapping it, so a
// third peer's durable scan (gossip + its own sweep + apply) ran out of poll while
// the step still had a minute in hand: station 1 failed at 180s and the row it was
// waiting for landed moments later (measured 2026-09-07 — matthew's application row
// was present and correct in the projection after the run). 210s spends the step's
// budget while leaving headroom for the assertion that follows.
export async function until(
  label: string,
  check: () => Promise<boolean> | boolean,
  ms = 210_000
): Promise<void> {
  const deadline = Date.now() + ms;
  let last: unknown;
  do {
    try {
      if (await check()) return;
    } catch (e) {
      last = e;
    }
    await delay(1000);
  } while (Date.now() < deadline);
  throw new Error(`${label}: deadline exceeded; last error: ${String(last)}`);
}
export function peerEnv(world: E2EWorld, peer: Peer): Record<string, string> {
  const s = ctx(world);
  if (s.env[peer]) return s.env[peer];
  // The household harness owns this directory; PID and env paths come from its live peer.
  // eslint-disable-next-line sonarjs/publicly-writable-directories
  const mesh = process.env['MESH_DIR'] ?? '/tmp/elohim-local-mesh';
  // hc-mesh.sh's `record_mesh_pid` writes "<pid> <start-ticks>", NOT a bare pid: the
  // second field is /proc/<pid>/stat's starttime, the harness's own PID-REUSE guard.
  // Reading the file as one number matched nothing and failed every scenario in the
  // Background (measured 2026-09-07). Take the pid to reach /proc, and spend the guard
  // it was written for — this helper is about to trust that process's ENVIRONMENT, so
  // "same pid" is not good enough; it has to be the same process the mesh launched.
  const [pid, launchTicks] = readFileSync(resolve(mesh, 'pids', `storage-${peer}`), 'utf8')
    .trim()
    .split(/\s+/);
  assert.match(pid, /^\d+$/);
  if (launchTicks) {
    // Mirror hc-mesh.sh's parse: `comm` may contain spaces, so strip pid+comm through
    // the final ')' first; starttime is field 20 of what remains.
    const stat = readFileSync(`/proc/${pid}/stat`, 'utf8');
    const started = stat
      .slice(stat.lastIndexOf(')') + 1)
      .trim()
      .split(/\s+/)[19];
    assert.equal(started, launchTicks, `${peer}: pid ${pid} is not the process the mesh launched`);
  }
  const entries = readFileSync(`/proc/${pid}/environ`, 'utf8').split('\0');
  const selected: Record<string, string> = {};
  for (const entry of entries) {
    const at = entry.indexOf('=');
    const name = entry.slice(0, at);
    if (['STORAGE_DIR', 'ELOHIM_RUNTIME_CONFIG_PATH'].includes(name))
      selected[name] = entry.slice(at + 1);
  }
  assert.ok(selected['ELOHIM_RUNTIME_CONFIG_PATH'], `${peer}: runtime-config watcher not armed`);
  assert.ok(selected['STORAGE_DIR'], `${peer}: storage directory missing`);
  s.env[peer] = selected;
  return selected;
}
export function rows(world: E2EWorld, peer: Peer, sql: string, params: unknown[] = []): Row[] {
  const db = resolve(peerEnv(world, peer)['STORAGE_DIR'], 'content.db');
  const script =
    'import sqlite3,json,sys\nc=sqlite3.connect("file:"+sys.argv[1]+"?mode=ro",uri=True)\nc.row_factory=sqlite3.Row\nprint(json.dumps([dict(r) for r in c.execute(sys.argv[2],json.loads(sys.argv[3]))]))';
  return JSON.parse(
    execFileSync('/usr/bin/python3', ['-c', script, db, sql, JSON.stringify(params)], {
      encoding: 'utf8',
    })
  ) as Row[];
}
export async function notify(world: E2EWorld, enabled: boolean): Promise<void> {
  const s = ctx(world);
  const path = peerEnv(world, 'james')['ELOHIM_RUNTIME_CONFIG_PATH'];
  s.originalConfig ??= readFileSync(path, 'utf8');
  const text = readFileSync(path, 'utf8').replace(/^ELOHIM_FEEDBACK_NOTIFY\s*=.*\n?/gm, '');
  writeFileSync(path, `${text}\nELOHIM_FEEDBACK_NOTIFY = ${enabled ? 1 : 0}\n`);
  await http('james', '/admin/runtime-config/reload', {});
  const config = await http('james', '/admin/runtime-config');
  world.attach(JSON.stringify(config), 'application/json');
  // The diagnostic shape is owned by runtime_config, recursively locate its named setting.
  const find = (v: unknown): Row | undefined => {
    if (!v || typeof v !== 'object') return undefined;
    const r = v as Row;
    if (r['name'] === 'ELOHIM_FEEDBACK_NOTIFY') return r;
    return Object.values(r).map(find).find(Boolean);
  };
  const setting = find(config);
  assert.ok(setting, 'runtime-config must report FeedbackNotify');
  assert.equal(Number(setting['effectiveValue']), enabled ? 1 : 0);
}
export async function standing(
  world: E2EWorld,
  subject: Peer = 'jessica',
  observer: Peer = 'jessica'
): Promise<Row> {
  const agent = (await rail(world, subject)).agent;
  const evaluator = (await rail(world, observer)).agent;
  const r = await http(observer, `/api/v1/standing/${agent}?evaluator=${evaluator}`);
  delete r['computedAt'];
  return r;
}
export async function file(world: E2EWorld, discard = false): Promise<void> {
  const s = ctx(world);
  s.request ??= {
    operationId: s.operationId,
    targetActionHash: s.root,
    signalKind: 'correction',
    standingImpact: 'debit-soft',
    body: 'The original statement needs correction.',
  };
  const response = await fetch(`${url('james')}/api/v1/feedback/operations`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(s.request),
    signal: AbortSignal.timeout(120_000),
  });
  assert.ok(
    response.ok,
    `file correction: ${response.status} ${response.ok ? '' : await response.text()}`
  );
  if (discard) {
    await response.body?.cancel();
    return;
  }
  s.operation = (await response.json()) as Row;
  assert.equal(s.operation['status'], 'resolved', JSON.stringify(s.operation));
  s.correction = String(s.operation['feedbackActionHash']);
}
export async function accept(world: E2EWorld): Promise<void> {
  const s = ctx(world);
  await until('Jessica can fetch correction', async () =>
    Boolean(await call(world, 'jessica', 'get_feedback_signal_record', raw(s.correction)))
  );
  // `create_vouch`'s coordinator gate is `must_get_valid_record(target)` — a strictly
  // stronger primitive than the `get` above. A record Jessica can GET is not yet one her
  // conductor can resolve as VALID, so acceptance raced DHT convergence and died with
  // Host("Failed to get Record …") in four stations (measured 2026-09-07). Contract §6
  // draws exactly this line: a dependency that cannot be FETCHED is pending and retried,
  // while a positive MISMATCH (wrong author, wrong type) is rejected. So retry the fetch
  // failure only, and let every real refusal out immediately rather than spending the
  // whole deadline turning a rejection into a timeout.
  let refusal: unknown;
  await until('Jessica accepts the correction', async () => {
    try {
      s.acceptance = hash(
        await call(world, 'jessica', 'create_vouch', {
          target_action_hash: raw(s.correction),
          vouch_kind: 'accept-correction',
          standing_impact: 'debit-soft',
        })
      );
      return true;
    } catch (e) {
      if (String(e).includes('Failed to get Record')) return false;
      refusal = e;
      return true;
    }
  });
  if (refusal) throw refusal;
}
export async function lineage(world: E2EWorld, peer: Peer = 'jessica'): Promise<LineageOutput> {
  return call(world, peer, 'get_content_lineage', {
    action_hash: raw(ctx(world).root),
    local: false,
  });
}
export async function amend(world: E2EWorld, predecessor: string, body: string): Promise<string> {
  const result = await call(world, 'jessica', 'amend_content', {
    predecessor_action_hash: raw(predecessor),
    content: { content: body },
  });
  return hash(result.action_hash);
}
// A peer that is NOT the author's own is the slowest path in this feature: the act has
// to gossip to that peer's conductor, its durable scan has to come round to the target,
// and only then does the application row land. Measured 2026-09-07 on the household mesh
// that took ~2-3 minutes, which sat right on the old 210s poll and made station 1 flip
// between pass and "deadline exceeded" run to run. Give the cross-peer assertion a budget
// that reflects the path it is actually waiting on; the wrapping step timeout is raised to
// match, or Cucumber would cut in first with a less informative error.
export async function applied(
  world: E2EWorld,
  peer: Peer = 'jessica',
  ms = 400_000
): Promise<void> {
  await until(
    `${peer} applies operation once`,
    () => {
      const result = rows(
        world,
        peer,
        `SELECT a.status, a.accepted, a.contribution FROM feedback_application a JOIN feedback_application_member m USING (generation_id, group_key) JOIN standing_generations g USING (generation_id) WHERE m.action_hash = ? AND g.status = 'published' ORDER BY g.generation_id DESC LIMIT 1`,
        [ctx(world).correction]
      );
      return result.length === 1 && result[0]['status'] === 'applied';
    },
    ms
  );
}
export async function contribution(world: E2EWorld): Promise<void> {
  await until('one accepted contribution', () => {
    const result = rows(
      world,
      'jessica',
      `SELECT a.accepted, a.contribution FROM feedback_application a JOIN feedback_application_member m USING (generation_id, group_key) JOIN standing_generations g USING (generation_id) WHERE m.action_hash = ? AND g.status = 'published' ORDER BY g.generation_id DESC LIMIT 1`,
      [ctx(world).correction]
    );
    return result[0]?.['accepted'] === 1 && result[0]?.['contribution'] === 2;
  });
  const value = await standing(world);
  assert.equal(
    value['debitWeightSum'],
    Number(ctx(world).baseline['jessica']['debitWeightSum']) + 2
  );
}
export async function served(world: E2EWorld, peer: Peer = 'jessica'): Promise<string> {
  return (await probeDeclaredHead(url(peer), ctx(world).id)).headActionHash;
}
export function pending(world: E2EWorld, reason: string): 'pending' {
  world.attach(`UNBOUND: ${reason}`, 'text/plain');
  return 'pending';
}
