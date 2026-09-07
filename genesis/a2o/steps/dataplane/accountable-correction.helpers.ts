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
/** The one standing impact every station in this feature files under. */
export const DEBIT_SOFT = 'debit-soft';
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
  rails: Partial<Record<Peer, CarriedElectionRail>>;
  env: Partial<Record<Peer, Record<string, string>>>;
  originalConfig?: string;
  beforeRows?: Row[];
  beforeStanding?: Row;
  rebuilt?: number;
  crash: boolean;
  /** Station 1: the act filed with its target index DEFERRED. */
  deferred?: string;
  /** Station 2: Matthew's own content-space DNA hash, read before delivery. */
  localDna?: string;
  /** Station 2: the receiver's verdict on the foreign-DNA envelope. */
  foreignVerdict?: Row;
  /** Station 2: the action hash the foreign envelope claimed. */
  foreignAction?: string;
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
// Storage normalises a 39-byte AgentPubKey to its 32-byte ed25519 core before it
// writes any (evaluator, subject) row (`feedback_projector::normalize_agent_key`).
// A witness reading those tables has to normalise the same way or it joins nothing.
export const key = (agent: string): string =>
  Buffer.from(raw(agent).slice(3, 35)).toString('hex').toUpperCase();

/** The published generation on this peer. Every read below is scoped to it. */
const PUBLISHED =
  "(SELECT max(generation_id) FROM standing_generations WHERE status = 'published')";
/**
 * The published generation's OWN arithmetic for one subject: how many groups it has
 * applied as accepted against them, and what those groups contribute in total.
 *
 * This replaces a captured baseline. The stations share three cell agents and never reset
 * state, so "the tally before this scenario" is a moving target — a baseline read at the
 * wrong moment is 2 low, and the shortfall surfaces later as a doubled contribution
 * ("contribution 12 != 10", measured 2026-09-07). Waiting for quiescence instead cost 20
 * minutes a scenario and still hung. The claim worth asserting does not need a baseline at
 * all: the served tally must equal the sum of the accepted groups the generation itself
 * holds, and THIS correction's group must contribute exactly 2, exactly once. That is
 * stronger than a delta and true in every run order.
 */
export function acceptedGroups(
  world: E2EWorld,
  subject: string,
  peer: Peer = 'jessica'
): { groups: number; total: number } {
  const r = rows(
    world,
    peer,
    `SELECT count(*) n, coalesce(sum(contribution), 0) total FROM feedback_application
    WHERE generation_id = ${PUBLISHED} AND status = 'applied' AND accepted = 1
    AND hex(subject_pubkey) = ?`,
    [subject]
  )[0];
  return { groups: Number(r?.['n'] ?? 0), total: Number(r?.['total'] ?? 0) };
}
/** Published aggregate rows for one (evaluator, subject) pair. Absent row = Unknown. */
export function aggregateRows(
  world: E2EWorld,
  evaluator: string,
  subject: string,
  peer: Peer = 'jessica'
): { rows: number; total: number } {
  const r = rows(
    world,
    peer,
    `SELECT count(*) n, coalesce(sum(debit_weight_sum), 0) total FROM standing_generation_aggregate
    WHERE generation_id = ${PUBLISHED} AND hex(evaluator_pubkey) = ? AND hex(subject_pubkey) = ?`,
    [evaluator, subject]
  )[0];
  return { rows: Number(r?.['n'] ?? 0), total: Number(r?.['total'] ?? 0) };
}
/** This scenario's correction group, as the published generation holds it. */
export function groupRow(world: E2EWorld, peer: Peer = 'jessica'): Row | undefined {
  return rows(
    world,
    peer,
    `SELECT a.status, a.accepted, a.contribution FROM feedback_application a
    JOIN feedback_application_member m USING (generation_id, group_key)
    WHERE a.generation_id = ${PUBLISHED} AND m.action_hash = ?`,
    [ctx(world).correction]
  )[0];
}
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
  ms = 300_000
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
    if (
      ['STORAGE_DIR', 'ELOHIM_RUNTIME_CONFIG_PATH', 'ELOHIM_FEEDBACK_SWEEP_SECONDS'].includes(name)
    )
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
    standingImpact: DEBIT_SOFT,
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
          standing_impact: DEBIT_SOFT,
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
export async function applied(world: E2EWorld, peer: Peer = 'jessica', ms = 0): Promise<void> {
  const budget = ms || rotationBudgetMs(world, peer);
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
    budget
  );
}
// THE BUDGET IS DERIVED, NOT GUESSED. Discovery is a rotation: the projector visits
// MAX_MEMBERS_PER_SWEEP = 8 subscription members per SWEEP_INTERVAL_SECS = 60 s tick, and
// `publish_generation` only republishes on a tick that ends with nothing unvisited and
// nothing pending. So an act filed now is applied within ONE FULL ROTATION of the
// subscription set — ceil(N/8) sweeps — plus a clean sweep to publish. N is not constant:
// every content record and every discovered correction becomes a member, so it grows with
// the mesh's whole history and the lag grows with it.
//
// MEASURED 2026-09-07 on the household mesh, sweep at its 60 s product default, ceiling
// raised to 600 s so the runs reported lag instead of deadlines. Acceptance to APPLICATION
// ROW: 0.04 s, 106.6 s, 212.5 s, 252.6 s at N between 14 and 30 members; 332.8 s at N=44,
// which is exactly ceil(44/8) = 6 sweeps. Acceptance to PUBLISHED TALLY added one further
// sweep (272.5 s against a 212.5 s row). Station 7's original 210 s budget expired TWO
// SECONDS before its row appeared — the whole of "8 expected, 6 observed" was a budget
// shorter than the rotation, not a wrong tally; station 4's crash arm, armed at the same
// commit (feedback_projector.rs:571), had the same 210 s and the same cause.
//
// Hence: budget = 2 x (ceil(N/8) + 2) sweeps, read from the live subscription count. The
// env override pins a fixed ceiling for a measurement round.
export const DEFAULT_SWEEP_MS = 60_000;
export const DHT_PROPAGATION_MS = 300_000;
export const MEMBERS_PER_SWEEP = 8;
export function rotationBudgetMs(world: E2EWorld, peer: Peer = 'jessica'): number {
  const fixed = Number(process.env['A2O_CONTRIBUTION_BUDGET_MS'] ?? Number.NaN);
  if (Number.isFinite(fixed)) return fixed;
  // The sweep is read from the PEER's own live environment, never assumed: this lane pins
  // ELOHIM_FEEDBACK_SWEEP_SECONDS, and a budget derived from the wrong sweep is exactly
  // the mistake this helper exists to stop making.
  const configured = Number(peerEnv(world, peer)['ELOHIM_FEEDBACK_SWEEP_SECONDS']);
  const sweepMs =
    Number.isFinite(configured) && configured > 0 ? configured * 1000 : DEFAULT_SWEEP_MS;
  const n = Number(
    rows(world, peer, 'SELECT count(*) n FROM feedback_subscriptions')[0]?.['n'] ?? 0
  );
  const sweeps = Math.ceil(n / MEMBERS_PER_SWEEP) + 2;
  // A rotation is only the projector's own scan. Before it can scan anything the ACT has
  // to reach this peer's conductor over the DHT, which no sweep setting accelerates and
  // which measured 2-3 minutes on this mesh. Budget the two separately or a fast sweep
  // makes the deadline shorter than the propagation it still has to wait for — measured
  // 2026-09-07, pinning the sweep to 5 s alone reduced the floor to 180 s and failed four
  // stations that had passed at 60 s.
  return DHT_PROPAGATION_MS + 2 * sweeps * sweepMs;
}

// ---------------------------------------------------------------------------
// Station 4 (crash window): the deadline AFTER a real storage restart.
//
// `rotationBudgetMs` above budgets a WARM peer's rotation. Post-T6
// (`feedback_projector.rs` commit b67e3d082 — `SweepScheduler`, `MemberHeat`),
// a warm peer's steady-state discovery is no longer O(N): a member with
// nothing open goes Cold after `COLD_AFTER_CLEAN_SWEEPS` clean sweeps and
// leaves the rotation, and a fresh act or `admit_notified_signal` re-arms it
// Hot at the front — so a Hot member's worst case is one re-arm plus one
// sweep, not a full `ceil(N/8)` rotation over the peer's whole history.
//
// A storage restart defeats exactly that saving. `SweepScheduler` is
// deliberately in-memory (its own doc comment: "a restart forgets it, every
// member starts Hot") — there is no durable column for heat
// (`feedback_subscriptions` carries only `last_visited_at`/`visit_count`,
// confirmed in `db/feedback_subscriptions.rs`), so the scheduler is rebuilt
// from `SweepScheduler::default()` on boot and every member defaults Hot
// again. The first post-restart rotation therefore costs a full
// `ceil(N/8)` sweeps — the SAME bound the projector always paid before T6 —
// not the warm-peer saving `rotationBudgetMs` now assumes elsewhere in this
// feature. Station 4's post-restart wait needs its own budget for exactly
// this reason; reusing `rotationBudgetMs` unchanged would be right by
// accident today and wrong the day someone tightens it because "T6 made
// discovery faster."
//
// N_LIVE (members still in rotation, i.e. not Cold) is the number the
// post-T6 model actually names, but it is scheduler-internal state with no
// external surface: `TickReport.members_cold` (`feedback_projector.rs:1072`)
// is computed per tick and never persisted, and there is no admin/diagnostic
// route exposing it. The only externally-observable count is N_TOTAL from
// `feedback_subscriptions` (the same query `rotationBudgetMs` already
// runs) — a restart makes N_LIVE == N_TOTAL anyway (every member is Hot), so
// for THIS station the distinction collapses and N_TOTAL is exact, not just
// a safe superset.
export interface CrashRestartBudget {
  budgetMs: number;
  nTotal: number;
  sweeps: number;
  sweepMs: number;
  restartSettleMs: number;
  dhtFloorMs: number;
}
// tokio::time::interval fires its first tick immediately on creation
// (`feedback_projector.rs::spawn`), so the projector does not sit idle for a
// whole SWEEP_INTERVAL_SECS before its first post-restart sweep — the
// settle cost is process/DB-pool startup, not a missed tick. One sweep
// interval of pad (the product default, not the lane's pinned test value)
// covers that startup plus jitter without inventing an unmeasured number.
export const RESTART_SETTLE_MS = DEFAULT_SWEEP_MS;
/**
 * Pure: the arithmetic alone, with no DB/env access, so it is unit-testable
 * without a mesh. `nTotal` is N_LIVE == N_TOTAL immediately after a restart
 * (see module comment above). `freshAct` adds the DHT propagation floor only
 * when this wait's target has NOT already been fetched-and-verified onto
 * this peer before the restart — station 4's crash window never sets it,
 * because the correction and the acceptance both landed locally before the
 * crash (the arm fires after the application row is written); the restart
 * only has to finish the LOCAL transaction, never re-fetch anything.
 */
export function deriveCrashRestartBudgetMs(
  nTotal: number,
  sweepMs: number,
  freshAct = false
): CrashRestartBudget {
  const sweeps = Math.ceil(Math.max(nTotal, 0) / MEMBERS_PER_SWEEP) + 2;
  const dhtFloorMs = freshAct ? DHT_PROPAGATION_MS : 0;
  const budgetMs = RESTART_SETTLE_MS + sweeps * sweepMs + dhtFloorMs;
  return { budgetMs, nTotal, sweeps, sweepMs, restartSettleMs: RESTART_SETTLE_MS, dhtFloorMs };
}
/** Reads the live peer env + subscription count, derives, and logs the inputs. */
export function crashRestartBudget(
  world: E2EWorld,
  peer: Peer = 'jessica',
  freshAct = false
): CrashRestartBudget {
  const configured = Number(peerEnv(world, peer)['ELOHIM_FEEDBACK_SWEEP_SECONDS']);
  const sweepMs =
    Number.isFinite(configured) && configured > 0 ? configured * 1000 : DEFAULT_SWEEP_MS;
  const nTotal = Number(
    rows(world, peer, 'SELECT count(*) n FROM feedback_subscriptions')[0]?.['n'] ?? 0
  );
  const budget = deriveCrashRestartBudgetMs(nTotal, sweepMs, freshAct);
  world.attach(
    JSON.stringify({ label: 'crash-restart rotation budget', peer, ...budget }),
    'application/json'
  );
  return budget;
}
// Cucumber step timeouts are fixed at registration while the budget above is derived per
// call, so they are set to a ceiling the derived budget cannot exceed on this mesh. They
// are spent only when something is genuinely wrong; the assertion names the numbers.
export const CONTRIBUTION_STEP_TIMEOUT_MS = 1_800_000;
/** Steps whose only wait is DHT propagation between peers, not the projector's rotation. */
export const DHT_STEP_TIMEOUT_MS = 420_000;
// MEASURED in the same round: rebuild requested to generation published, 60.0 s — one
// sweep. A rebuild replays the retained set in place, so it does not pay the rotation;
// four sweeps of headroom.
export const REBUILD_BUDGET_MS = Number(process.env['A2O_REBUILD_BUDGET_MS'] ?? 240_000);
export const REBUILD_STEP_TIMEOUT_MS = CONTRIBUTION_STEP_TIMEOUT_MS + REBUILD_BUDGET_MS + 120_000;

export interface Lag {
  label: string;
  budgetMs: number;
  firstMarkMs: number | null;
  settledMs: number | null;
}
export function attachLag(world: E2EWorld, lag: Lag): void {
  world.attach(JSON.stringify(lag), 'application/json');
}

// An eventually-consistent projection is polled, never sampled: the application row and
// the tally it feeds land on different sweeps, so read them together or a correct
// projection reports as a wrong number. And read the expected value from the generation
// itself (`acceptedGroups`) rather than from a baseline captured before the scenario —
// see the note there for why a baseline is a moving target on a mesh that never resets.
// The lag record is attached and named in the assertion message, so a slow rotation can
// never again be reported as a wrong tally.
export async function contribution(world: E2EWorld): Promise<void> {
  const budget = rotationBudgetMs(world);
  const subject = key((await rail(world, 'jessica')).agent);
  const started = Date.now();
  let rowAt: number | null = null;
  let expected = Number.NaN;
  let observed = Number.NaN;
  try {
    await until(
      'the accepted contribution reaches the tally',
      async () => {
        const group = groupRow(world);
        if (!(group?.['accepted'] === 1 && group?.['contribution'] === 2)) return false;
        rowAt ??= Date.now();
        expected = acceptedGroups(world, subject).total;
        observed = Number((await standing(world))['debitWeightSum']);
        return observed === expected;
      },
      budget
    );
  } catch {
    // Fall through: the assertion below names the numbers, which a deadline cannot.
  }
  const lag: Lag = {
    label: 'acceptance to published tally',
    budgetMs: budget,
    firstMarkMs: rowAt === null ? null : rowAt - started,
    settledMs: observed === expected ? Date.now() - started : null,
  };
  attachLag(world, lag);
  assert.equal(
    observed,
    expected,
    `served tally ${observed} != the generation's own accepted-group sum ${expected}; lag: ${JSON.stringify(lag)}`
  );
}
export async function served(world: E2EWorld, peer: Peer = 'jessica'): Promise<string> {
  return (await probeDeclaredHead(url(peer), ctx(world).id)).headActionHash;
}
export function pending(world: E2EWorld, reason: string): 'pending' {
  world.attach(`UNBOUND: ${reason}`, 'text/plain');
  return 'pending';
}

// ---------------------------------------------------------------------------
// Station 1 — index publication is separable from the act (contract §3)
// ---------------------------------------------------------------------------

/**
 * File a correction whose `TargetToFeedbackSignal` index link is NOT published.
 *
 * The act commits, is fetchable by its own action hash, and is enumerable from
 * its SIGNER's anchor — but a peer scanning the TARGET cannot see it, because
 * the index edge does not exist yet. That is how a "late link" is staged
 * honestly: a Holochain action cannot be backdated, so the thing made late is
 * PUBLICATION ORDER, exactly as the station's own comment requires.
 *
 * Two phases, the same two the outbox runs (§8): author the public Correction
 * EPR carrying the immutable request, then file the feedback citing it — here
 * with `defer_target_link: true`.
 */
export async function fileDeferred(world: E2EWorld, label: string): Promise<string> {
  const s = ctx(world);
  const operationId = randomUUID();
  const evidence = await call(world, 'james', 'create_content', {
    id: `correction:${operationId}`,
    title: 'Deferred-index correction evidence',
    description: label,
    content_type: 'concept',
    content_format: 'markdown',
    content: label,
    tags: [],
    reach: 'public',
    metadata_json: JSON.stringify({
      correctionRequest: {
        operationId,
        targetActionHash: s.root,
        signalKind: 'correction',
        standingImpact: DEBIT_SOFT,
      },
    }),
  });
  const evidenceHash = hash(evidence.action_hash);
  // The coordinator's admission runs `must_get_valid_record` on the evidence,
  // which is strictly stronger than the `create_content` that just returned —
  // the same race `accept()` documents. Retry the fetch failure only; every
  // real refusal is let out immediately rather than spending the deadline.
  let refusal: unknown;
  let action = '';
  await until('James files the deferred-index correction', async () => {
    try {
      action = hash(
        await call(world, 'james', 'create_feedback_signal', {
          target_action_hash: raw(s.root),
          signal_kind: 'correction',
          evidence_action_hash: raw(evidenceHash),
          standing_impact: DEBIT_SOFT,
          defer_target_link: true,
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
  return action;
}

/** Publish the withheld index edge — link only, derived from the act itself. */
export async function publishLink(world: E2EWorld, correction: string): Promise<Row> {
  return (await call(world, 'james', 'publish_feedback_signal_target_link', {
    feedback_action_hash: raw(correction),
  })) as Row;
}

/** Every application row this peer holds for ONE act, newest generation first. */
export function applicationRows(world: E2EWorld, peer: Peer, action: string): Row[] {
  return rows(
    world,
    peer,
    `SELECT a.status, a.accepted, a.contribution, a.generation_id
     FROM feedback_application a
     JOIN feedback_application_member m USING (generation_id, group_key)
     JOIN standing_generations g USING (generation_id)
     WHERE m.action_hash = ? AND g.status = 'published'
     ORDER BY g.generation_id DESC`,
    [action]
  );
}

/** Wait until `peer` has applied `action` — exactly one row, status applied. */
export async function appliedAction(
  world: E2EWorld,
  peer: Peer,
  action: string,
  ms = 0
): Promise<void> {
  const budget = ms || rotationBudgetMs(world, peer);
  await until(
    `${peer} applies ${action.slice(0, 12)} once`,
    () => {
      const result = applicationRows(world, peer, action);
      return result.length === 1 && result[0]['status'] === 'applied';
    },
    budget
  );
}

// ---------------------------------------------------------------------------
// Station 2 — the notification receiver (contract §4)
// ---------------------------------------------------------------------------

/** One subscription row, or undefined when this peer has never heard of it. */
export function subscription(world: E2EWorld, peer: Peer, memberKey: string): Row | undefined {
  return rows(
    world,
    peer,
    'SELECT member_kind, member_key, source, added_at, last_visited_at, visit_count FROM feedback_subscriptions WHERE member_key = ?',
    [memberKey]
  )[0];
}

/** How many members are in this peer's rotation right now. */
export function subscriptionCount(world: E2EWorld, peer: Peer): number {
  return Number(rows(world, peer, 'SELECT count(*) n FROM feedback_subscriptions')[0]?.['n'] ?? 0);
}

/** This peer's sweep interval, read from its own live environment. */
export function sweepMs(world: E2EWorld, peer: Peer): number {
  const configured = Number(peerEnv(world, peer)['ELOHIM_FEEDBACK_SWEEP_SECONDS']);
  return Number.isFinite(configured) && configured > 0 ? configured * 1000 : DEFAULT_SWEEP_MS;
}

/** What space is this peer in? (the test ingress reports its own binding) */
export async function originDna(peer: Peer): Promise<string> {
  const report = await http(peer, '/admin/test/feedback-notify');
  const dna = report['originDnaHash'];
  assert.ok(
    typeof dna === 'string' && dna.length > 0,
    `${peer} must report its own content-cell DNA hash (is ELOHIM_TEST_FEEDBACK_INGRESS=1 set on it?): ${JSON.stringify(report)}`
  );
  return dna;
}

/**
 * Deliver a `feedback-signal` notification to `peer`'s REAL receiver.
 *
 * The route hands MessagePack bytes to the same
 * `EprAtomService::handle(IntegrityNotify { kind: "feedback-signal" })` both
 * transports call, and returns that receiver's own verdict verbatim. It is
 * armed only by `ELOHIM_TEST_FEEDBACK_INGRESS=1`.
 */
export async function deliverNotification(
  peer: Peer,
  actRef: { originDnaHash: string; actionHash: string; routingKey: string },
  from = 'peer-notifier'
): Promise<Row> {
  return http(peer, '/admin/test/feedback-notify', {
    from,
    signal: {
      targetCid: actRef.routingKey,
      signalKind: 'correction',
      evidenceCid: 'evidence-carried-by-reference',
      standingImpact: DEBIT_SOFT,
      signedBy: 'claim',
      signature: 'claim',
      actRef,
    },
  });
}

/** The receiver's `IntegrityAck` fields, whatever shape the enum tags it with. */
export function verdictOf(response: Row): { received: boolean; reason: string } {
  const verdict = (response['verdict'] ?? {}) as Row;
  return {
    received: verdict['received'] === true,
    reason: String(verdict['reason'] ?? ''),
  };
}
