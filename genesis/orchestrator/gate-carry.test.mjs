// gate-carry: the one algorithm (planCarry), the switch, the steps the manifests give, and the
// Carrier contract, run against the second-seat carrier and against an in-memory carrier so the
// contract is not shaped around one implementor.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { carryMode, groupMembers, makeSecondSeat, planCarry, stepsFor, termsRefusal } from './gate-carry.mjs';

const OWNERS = new Set(['self', 'commitment', 'operator', 'charter']);
const SEAT_TERMS = makeSecondSeat().terms;
const seat = [{ name: 'second-seat', terms: SEAT_TERMS }];

const cargo = (project, p90, { jobs = 1, slot = `workspace:${project}:dev` } = {}) => ({
  project, run: { kind: 'just', recipe: 'gate', cargo: { workspace: project } }, key: null, p90,
  demand: { buildJobs: jobs, slot },
});
const plain = (project, p90 = 30) => ({ project, run: { kind: 'just', recipe: 'gate' }, key: null, p90,
  demand: { buildJobs: null, slot: null } });
const attested = project => ({ project, run: { kind: 'attested' }, key: null, p90: 0, demand: { buildJobs: null, slot: null } });
const reasonOf = (plan, project) => plan.notOffered.find(n => n.project === project)?.reason;

// ---- planCarry: every condition ----------------------------------------------------------------

test('a single step is never carried', function plan_one_step_stays_inline() {
  const plan = planCarry([cargo('storage', 3535)], seat);
  assert.deepEqual(plan.offers, []);
  assert.deepEqual(plan.inline, ['storage']);
  assert.match(reasonOf(plan, 'storage'), /only step/);
  assert.equal(plan.notOffered[0].owner, 'operator');
});

test('the second seat takes the long one-job cargo gate and leaves the rest inline', function plan_carries_long_one_job_cargo() {
  const plan = planCarry([plain('app'), cargo('storage', 3535), plain('docs')], seat);
  assert.deepEqual(plan.offers, [{ project: 'storage', carrier: 'second-seat' }]);
  assert.deepEqual(plan.inline, ['app', 'docs']);
  assert.equal(plan.notOffered.length, 2, 'every step not offered has a reason');
  assert.match(reasonOf(plan, 'app'), /not a cargo gate/);
});

test('each term refuses with its own reason', function plan_refuses_each_term() {
  const steps = [plain('app'), attested('brit'), cargo('jobs6', 4000, { jobs: 6 }), cargo('unset', 4000, { jobs: null }),
    cargo('fresh', null), cargo('short', 300), cargo('a', 3000, { slot: 'workspace:shared:dev' }),
    cargo('b', 100, { slot: 'workspace:shared:dev' })];
  const plan = planCarry(steps, seat);
  assert.deepEqual(plan.offers, []);
  assert.match(reasonOf(plan, 'brit'), /attested/);
  assert.match(reasonOf(plan, 'app'), /not a cargo gate/);
  assert.match(reasonOf(plan, 'jobs6'), /6 jobs, not 1/);
  assert.match(reasonOf(plan, 'unset'), /undeclared number of jobs/);
  assert.match(reasonOf(plan, 'fresh'), /no recorded run/);
  assert.match(reasonOf(plan, 'short'), /p90 5 min is under 10 min/);
  assert.match(reasonOf(plan, 'a'), /shares its cargo target with b/);
  for (const row of plan.notOffered) assert.ok(OWNERS.has(row.owner), row.owner);
});

test('the largest p90 wins and a tie keeps the given order', function plan_largest_p90_then_order() {
  const longest = planCarry([cargo('x', 900), cargo('y', 2000), plain('app')], seat);
  assert.deepEqual(longest.offers.map(o => o.project), ['y']);
  assert.match(reasonOf(longest, 'x'), /went to a longer one/);
  const tie = planCarry([cargo('x', 2000), cargo('y', 2000), plain('app')], seat);
  assert.deepEqual(tie.offers.map(o => o.project), ['x']);
});

test('an overlap-only carrier always leaves one step inline', function plan_keeps_one_inline() {
  const wide = [{ name: 'wide', terms: { ...SEAT_TERMS, steps: 5 } }];
  const plan = planCarry([cargo('x', 900), cargo('y', 2000)], wide);
  assert.equal(plan.offers.length, 1);
  assert.deepEqual(plan.inline, ['x']);
});

test('carriers are tried in order and a step goes to at most one', function plan_carriers_in_order() {
  const plan = planCarry([cargo('x', 900), cargo('y', 2000), plain('app')],
    [{ name: 'first', terms: SEAT_TERMS }, { name: 'second', terms: SEAT_TERMS }]);
  assert.deepEqual(plan.offers, [{ project: 'y', carrier: 'first' }, { project: 'x', carrier: 'second' }]);
  assert.deepEqual(plan.inline, ['app']);
});

test('termsRefusal admits a step that meets every term', function terms_admit_a_fit_step() {
  assert.equal(termsRefusal(cargo('s', 3535), SEAT_TERMS, [cargo('s', 3535), plain('app')]), null);
});

test('the switch defaults to shadow and reads anything else as shadow, saying so', function carry_mode_switch() {
  assert.deepEqual(carryMode({}), { mode: 'shadow' });
  assert.deepEqual(carryMode({ GATE_CARRY: 'on' }), { mode: 'on' });
  assert.deepEqual(carryMode({ GATE_CARRY: 'off' }), { mode: 'off' });
  const odd = carryMode({ GATE_CARRY: 'yes' });
  assert.equal(odd.mode, 'shadow');
  assert.match(odd.note, /not off\|shadow\|on/);
});

// ---- steps from manifests: the resolved cargo env, not a re-implementation ----------------------

function manifestFixture(t, { policy = {}, rows = [] } = {}) {
  const root = mkdtempSync(join(tmpdir(), 'gate-carry-steps-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, 'genesis/agentic'), { recursive: true });
  writeFileSync(join(root, 'genesis/agentic/pool-policy.json'), JSON.stringify(policy));
  writeFileSync(join(root, 'build-manifest.json'), JSON.stringify({ pipeline: 'fx', steps: { build: {} }, gate: { projects: {
    storage: { dir: 'storage', run: { kind: 'just', recipe: 'gate', cargo: { workspace: 'storage', env: { CARGO_BUILD_JOBS: '1' } } } },
    sibling: { dir: 'sibling', run: { kind: 'root-just', recipe: 'gate', cargo: { workspace: 'storage' } } },
    policy: { dir: 'policy', run: { kind: 'just', recipe: 'gate', cargo: { workspace: 'policy' } } },
    app: { dir: 'app', run: { kind: 'just', recipe: 'gate' } },
  } } }));
  mkdirSync(join(root, '.eprfs/status'), { recursive: true });
  writeFileSync(join(root, '.eprfs/status/flows.jsonl'), rows.map((row, i) => JSON.stringify({ cid: `c${i}`, record: {
    occurredAt: '2026-11-20T00:00:00Z',
    classifiedAs: ['run:observation', row.project, 'measure:gate-cycle-seconds@1', `value:${row.value}`,
      'env:run-kind=pre-push', 'env:exit=ok', `env:project=${row.project}`],
  } })).join('\n'));
  return root;
}

test('steps read the jobs gate-runner would send and the slot the pool would give', function steps_resolve_jobs_and_slot(t) {
  const root = manifestFixture(t, {
    policy: { cargo_env_overrides: { policy: { CARGO_BUILD_JOBS: '1' } } },
    rows: [{ project: 'storage', value: 3535 }, { project: 'storage', value: 3400 }, { project: 'app', value: 20 }],
  });
  const steps = stepsFor(['storage', 'sibling', 'policy', 'app', 'gone'], { root });
  const by = name => steps.find(s => s.project === name);
  assert.equal(by('storage').demand.buildJobs, 1);
  assert.equal(by('storage').p90, 3535);
  assert.equal(by('storage').demand.slot, by('sibling').demand.slot, 'two gates on one workspace share a slot');
  assert.equal(by('sibling').demand.buildJobs, null);
  assert.equal(by('policy').demand.buildJobs, 1, 'a pool-policy cargo_env_overrides cap counts too');
  assert.equal(by('app').demand.slot, null);
  assert.equal(by('gone').p90, null, 'an unknown project is a step nothing will carry');
  assert.match(reasonOf(planCarry(steps, seat), 'storage'), /shares its cargo target with sibling/);
});

// ---- the Carrier contract ------------------------------------------------------------------------

/**
 * Any carrier must pass this. `make({ admit, exitCode, holdMs })` returns { carrier, step, ctx,
 * running() }: `running()` lists what the carrier started that is still alive, which is how the
 * contract sees "starts nothing" and "leaves no process".
 */
async function carrierContract(make) {
  // An offer that is not admitted returns a reason and an owner, and starts nothing.
  {
    const { carrier, step, ctx, running } = make({ admit: false, exitCode: 0, holdMs: 50 });
    const answer = await carrier.offer(step, ctx);
    assert.equal(answer.carried, false);
    assert.equal(typeof answer.reason, 'string');
    assert.ok(answer.reason.length > 0);
    assert.ok(OWNERS.has(answer.owner), `owner ${answer.owner}`);
    assert.deepEqual(running(), [], 'a refused offer started nothing');
    const admission = await carrier.admit(step, ctx);
    assert.equal(admission.admitted, false);
  }
  // A carried step is collected with the gate's own exit status.
  for (const exitCode of [0, 3]) {
    const { carrier, step, ctx, running } = make({ admit: true, exitCode, holdMs: 100 });
    assert.deepEqual(await carrier.admit(step, ctx), { admitted: true });
    assert.deepEqual(running(), [], 'admit is read-only');
    const answer = await carrier.offer(step, ctx);
    assert.equal(answer.carried, true);
    const done = await carrier.collect(answer.handle);
    assert.equal(done.project, step.project);
    assert.equal(done.status, exitCode);
    assert.deepEqual(done.exit, { kind: 'exited', code: exitCode });
    assert.ok(['computed', 'under-grant', 'recomputed'].includes(done.basis), `basis ${done.basis} is a DerivationBasis`);
    assert.equal(typeof done.seconds, 'number');
    assert.equal(typeof done.logPath, 'string');
    assert.deepEqual((await carrier.withdraw(answer.handle)).left, [], 'withdraw after collect is a no-op');
  }
  // A running step answers null without waiting, can always be withdrawn, and leaves nothing.
  {
    const { carrier, step, ctx, running } = make({ admit: true, exitCode: 0, holdMs: 60_000 });
    const answer = await carrier.offer(step, ctx);
    assert.equal(answer.carried, true);
    assert.equal(await carrier.collect(answer.handle, { wait: false }), null);
    const { left } = await carrier.withdraw(answer.handle);
    assert.deepEqual(left, []);
    assert.deepEqual(running(), [], 'withdraw left no process');
    const after = await carrier.collect(answer.handle);
    assert.notEqual(after.status, 0, 'a withdrawn step never reads as a pass');
  }
}

function inMemoryCarrier({ admit, exitCode, holdMs }) {
  const live = new Map();
  let n = 0;
  const carrier = {
    name: 'in-memory',
    terms: SEAT_TERMS,
    async admit() { return admit ? { admitted: true } : { admitted: false, reason: 'seats:test: full', owner: 'commitment' }; },
    async offer(step, ctx) {
      const admission = await carrier.admit(step, ctx);
      if (!admission.admitted) return { carried: false, reason: admission.reason, owner: admission.owner };
      const id = ++n;
      const entry = { started: Date.now(), done: null };
      entry.timer = setTimeout(() => { entry.done = { kind: 'exited', code: exitCode }; live.delete(id); }, holdMs);
      live.set(id, entry);
      return { carried: true, handle: { carrier: 'in-memory', project: step.project, dir: '', logPath: '(memory)', id, entry } };
    },
    async collect(handle, { wait = true } = {}) {
      while (!handle.entry.done) {
        if (!wait) return null;
        await new Promise(r => setTimeout(r, 10));
      }
      const exit = handle.entry.done;
      return { project: handle.project, exit, status: exit.kind === 'exited' ? exit.code : 1,
        seconds: Math.round((Date.now() - handle.entry.started) / 1000), logPath: handle.logPath, basis: 'computed' };
    },
    async withdraw(handle) {
      if (live.has(handle.id)) {
        clearTimeout(handle.entry.timer);
        handle.entry.done = { kind: 'unknown', why: 'withdrawn' };
        live.delete(handle.id);
      }
      return { left: [] };
    },
  };
  return { carrier, step: cargo('storage', 3535), ctx: { steps: [cargo('storage', 3535), plain('app')] }, running: () => [...live.keys()] };
}

const alivePid = pid => {
  try {
    const stat = readFileSync(`/proc/${pid}/stat`, 'utf8');
    return stat.slice(stat.lastIndexOf(')') + 2)[0] !== 'Z';
  } catch { return false; }
};

// The gate the second seat runs here: it starts a grandchild (so a withdraw must end the whole group),
// records both pids, prints, holds, and exits with the code it was given.
const FAKE_GATE = `
const { spawn } = require('child_process');
const { writeFileSync } = require('fs');
const [pids, code, ms] = process.argv.slice(2);
const kid = spawn('sleep', ['120'], { stdio: 'ignore' });
writeFileSync(pids, process.pid + ' ' + kid.pid);
console.log('fake gate output');
setTimeout(() => { kid.kill(); process.exit(Number(code)); }, Number(ms));
`;

function secondSeatFixture(t) {
  const dir = mkdtempSync(join(tmpdir(), 'gate-carry-seat-'));
  const groups = [];
  t.after(() => {
    for (const pgid of groups) { try { process.kill(-pgid, 'SIGKILL'); } catch { /* gone */ } }
    rmSync(dir, { recursive: true, force: true });
  });
  writeFileSync(join(dir, 'fake-gate.cjs'), FAKE_GATE);
  let n = 0;
  return ({ admit, exitCode, holdMs }) => {
    const id = ++n;
    const pidsFile = join(dir, `pids-${id}`);
    const roomAnswer = admit
      ? { verdict: 'admit', reason: 'fixture' }
      : { verdict: 'refuse', axis: 'host-floor:memory', owner: 'self', reason: 'ram-guard level high at 82% committed' };
    const carrier = makeSecondSeat({
      pollMs: 50,
      graceMs: 2000,
      room: () => roomAnswer,
      command: () => [process.execPath, join(dir, 'fake-gate.cjs'), pidsFile, String(exitCode), String(holdMs)],
    });
    const wrapped = { ...carrier, async offer(step, ctx) {
      const answer = await carrier.offer(step, ctx);
      if (answer.carried) groups.push(answer.handle.pid);
      return answer;
    } };
    const ctx = { root: dir, env: process.env, steps: [cargo('storage', 3535), plain('app')],
      dirFor: project => join(dir, `carry-${id}`, project) };
    const running = () => {
      const pids = existsSync(pidsFile) ? readFileSync(pidsFile, 'utf8').trim().split(' ').map(Number) : [];
      const handle = existsSync(join(dir, `carry-${id}`, 'storage', 'handle.json'))
        ? JSON.parse(readFileSync(join(dir, `carry-${id}`, 'storage', 'handle.json'), 'utf8')) : null;
      return [...new Set([...pids.filter(alivePid), ...(handle ? groupMembers(handle.pid) : [])])];
    };
    return { carrier: wrapped, step: cargo('storage', 3535), ctx, running };
  };
}

test('the contract holds for an in-memory carrier', async function contract_in_memory_carrier() {
  await carrierContract(inMemoryCarrier);
});

test('the contract holds for the second seat', async function contract_second_seat(t) {
  await carrierContract(secondSeatFixture(t));
  // The second seat computes on this device, and says so.
  const { carrier, step, ctx } = secondSeatFixture(t)({ admit: true, exitCode: 0, holdMs: 50 });
  const answer = await carrier.offer(step, ctx);
  assert.equal((await carrier.collect(answer.handle)).basis, 'computed');
});

test('the second seat refuses in the owner berth names, and a step outside its terms as operator', async function second_seat_refusals(t) {
  const make = secondSeatFixture(t);
  const { carrier, ctx } = make({ admit: false, exitCode: 0, holdMs: 10 });
  const refused = await carrier.offer(cargo('storage', 3535), ctx);
  assert.equal(refused.reason, 'host-floor:memory: ram-guard level high at 82% committed');
  assert.equal(refused.owner, 'self');
  const outside = await carrier.offer(cargo('storage', 120), ctx);
  assert.match(outside.reason, /p90 2 min is under 10 min/);
  assert.equal(outside.owner, 'operator');
  const unreadable = makeSecondSeat({ room: () => ({ verdict: 'refuse', axis: 'host-floor', owner: 'self',
    reason: 'berth room unreadable: spawn ENOENT' }) });
  const blind = await unreadable.offer(cargo('storage', 3535), ctx);
  assert.equal(blind.owner, 'self');
  assert.match(blind.reason, /unreadable/);
});

test('the second seat asks berth room for two builds, the loop seat and its own', async function second_seat_asks_for_two(t) {
  const asked = [];
  const carrier = makeSecondSeat({ room: want => { asked.push(want); return { verdict: 'refuse', axis: 'seats:cargo', owner: 'operator', reason: 'x' }; } });
  await carrier.admit(cargo('storage', 3535), { steps: [cargo('storage', 3535), plain('app')] });
  assert.deepEqual(asked, [2]);
});

test('a step that ends on a signal collects as 128 plus the signal', async function second_seat_signal_status(t) {
  const make = secondSeatFixture(t);
  const { carrier, step, ctx } = make({ admit: true, exitCode: 0, holdMs: 60_000 });
  const answer = await carrier.offer(step, ctx);
  const deadline = Date.now() + 10_000;
  let pids = null;
  while (Date.now() < deadline) {
    try { pids = readFileSync(join(ctx.root, 'pids-1'), 'utf8').trim().split(' ').map(Number); break; } catch { await new Promise(r => setTimeout(r, 20)); }
  }
  assert.ok(pids, 'the fake gate started');
  process.kill(pids[0], 'SIGTERM');
  const done = await carrier.collect(answer.handle);
  assert.deepEqual(done.exit, { kind: 'signaled', signal: 'SIGTERM' });
  assert.equal(done.status, 143);
  await carrier.withdraw(answer.handle);
});

test('a step whose record cannot be written is ended, not left running', async function second_seat_unrecorded_step_withdrawn(t) {
  const make = secondSeatFixture(t);
  const { carrier, step, ctx } = make({ admit: true, exitCode: 0, holdMs: 60_000 });
  const pidsFile = join(ctx.root, 'pids-1');
  const live = () => (existsSync(pidsFile) ? readFileSync(pidsFile, 'utf8').trim().split(' ').map(Number).filter(alivePid) : []);
  // handle.json is a non-empty directory, so the atomic rename of the record fails after the spawn.
  mkdirSync(join(ctx.dirFor(step.project), 'handle.json'), { recursive: true });
  writeFileSync(join(ctx.dirFor(step.project), 'handle.json', 'x'), '');
  const answer = await carrier.offer(step, ctx);
  assert.equal(answer.carried, false);
  assert.match(answer.reason, /^could not record the carried step: /);
  assert.equal(answer.owner, 'self');
  const deadline = Date.now() + 5000;
  await new Promise(r => setTimeout(r, 300)); // a gate that had started has written its pids by now
  while (live().length && Date.now() < deadline) await new Promise(r => setTimeout(r, 50));
  assert.deepEqual(live(), [], 'no gate process is left');
});
