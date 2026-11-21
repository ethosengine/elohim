#!/usr/bin/env node
// gate-carry: a gate step carried beside the push loop instead of inside it.
//
// The loop that runs a push's gates one after another depends on two things here and nothing else:
// `planCarry`, the one algorithm that says which steps stay inline and which step is offered to which
// carrier, and the `Carrier` contract every carrier implements. The first carrier, `secondSeat`, runs
// the step on this device's own second cargo seat. A later carrier (`ark task run` in the device's
// own envelope, a sibling device's offer) implements the same four verbs, and the loop does not
// change. Names follow ark-core (`TaskSpec`, `TaskOutcome`, `ExitClass`), placement-core (`Need`,
// `Demand`, `OfferTerms`, `NotCarried`), elohim-compute (`LimitOwner`) and eprfs-core
// (`DerivationBasis`), so the contract can later be backed by them without changing its callers.
//
// Capacity is one answer: a carrier asks this device's own lines (`berth room cargo --json`); it never
// re-derives room here. A refusal names whose line it honoured. Nothing waits on anything but this
// device's own processes, and a step that is not carried runs inline, as it always has: the requester
// can always run the work alone.
//
// Switch: GATE_CARRY=off|shadow|on (default shadow). shadow prints what would be carried and why and
// changes nothing; off prints nothing; on carries.
//
// CLI (the pre-push hook is a thin caller of these five verbs):
//   gate-carry.mjs offer <project…>     stdout: one "<project>\t<handle>" line per carried step
//   gate-carry.mjs peek <handle>        stdout: "running" or the step's exit status (never waits)
//   gate-carry.mjs collect <handle>     stdout: the step's output block, then "carry-verdict <project> <status> <seconds>"
//   gate-carry.mjs withdraw <handle…>   stops each carried step and everything it started
//   gate-carry.mjs plan <project…>      the plan as JSON (read-only)
// Everything said to a person goes to stderr; stdout is the hook's contract.

import { spawn, spawnSync } from 'child_process';
import {
  closeSync, existsSync, mkdirSync, mkdtempSync, openSync, readFileSync, readdirSync, renameSync, writeFileSync,
} from 'fs';
import { constants as osConstants, tmpdir } from 'os';
import { join, resolve } from 'path';
import { fileURLToPath } from 'url';
import { costOf, readCycleUnion } from './gate-cost.mjs';
import { gateChildEnv, worktreeTargetDir } from './gate-runner.mjs';
import { loadGateRegistry } from './pipeline-registry.mjs';

const SELF = fileURLToPath(import.meta.url);

// ---- the contract ------------------------------------------------------------------------------

/**
 * Whose line a refusal honoured: elohim-compute `actuation.rs` `LimitOwner`, in its serde words.
 * `self` is this device's own limit (its RAM and I/O guards); `operator` the runtime default the
 * steward declared (pool-policy, the carrier's terms, the GATE_CARRY switch); `commitment` a bounded
 * grant; `charter` a space's own rules. `faith` is not a lever and no carry refusal uses it.
 * @typedef {'self'|'commitment'|'operator'|'charter'} LimitOwner
 */

/**
 * What a step asks of a carrier (placement-core `Demand`/`Requirements`, narrowed to a gate).
 * @typedef {object} StepDemand
 * @property {number|null} buildJobs  the resolved CARGO_BUILD_JOBS; null when the gate declares none
 * @property {string|null} slot       the cargo target it builds in; null for a gate with no cargo
 */

/**
 * What is offered: one gate project (ark-core `TaskSpec`, placement-core `Need`, as a gate sees them).
 * @typedef {object} Step
 * @property {string} project        the manifest's gate project name
 * @property {object} run            the manifest's `run` block, as declared
 * @property {string|null} key       the closure (derivation) key when known; null = not computed yet
 * @property {number|null} p90       expected seconds: this workspace's recorded p90 (gate-cost `costOf`)
 * @property {StepDemand} demand
 */

/**
 * A carrier's declared posture (placement-core `OfferTerms`): what it will take, read by `planCarry`.
 * @typedef {object} CarrierTerms
 * @property {number} steps          at most this many steps per push
 * @property {boolean} cargo         only cargo gates (an attested or cargo-less gate is never taken)
 * @property {number|null} buildJobs only steps whose resolved CARGO_BUILD_JOBS equals this; null: any
 * @property {number} minP90         only steps whose recorded p90 is at least this many seconds
 * @property {boolean} ownSlot       only a step whose cargo target no other selected step builds in
 * @property {boolean} overlapOnly   only while at least one step stays inline (it saves time by overlap)
 */

/**
 * @typedef {{ admitted: true } | { admitted: false, reason: string, owner: LimitOwner }} Admission
 */

/**
 * A carried step, serializable so a later process can collect or withdraw it.
 * @typedef {object} Handle
 * @property {string} carrier        the carrier's name
 * @property {string} project
 * @property {string} dir            where the carrier keeps the step's record and output
 * @property {string} logPath
 */

/**
 * @typedef {{ carried: true, handle: Handle } | { carried: false, reason: string, owner: LimitOwner }} OfferAnswer
 */

/**
 * How the step ended: ark-core `ExitClass`. `unknown` is an honest absence (the carrier lost the
 * step's end), never a pass.
 * @typedef {{ kind: 'exited', code: number } | { kind: 'signaled', signal: string } | { kind: 'unknown', why: string }} ExitClass
 */

/**
 * The answer to a carried step (ark-core `TaskOutcome`).
 * @typedef {object} Collected
 * @property {string} project
 * @property {ExitClass} exit
 * @property {number} status         the gate's own exit status, as the loop would have seen it
 * @property {number} seconds        the step's own run time
 * @property {string} logPath
 * @property {'computed'|'under-grant'|'recomputed'} basis
 *   eprfs-core `DerivationBasis` in its serde words (`basis` tag, kebab-case). `second-seat` only ever
 *   answers `computed` (work done on this device); a carrier on another device answers `under-grant`
 *   (carried for its requester under a grant). `recomputed` is a check of another record's output.
 */

/**
 * The contract. `admit` is read-only and starts nothing; `offer` admits the same way, then starts.
 * A carried step can always be collected or withdrawn; withdraw leaves no process of it behind.
 * `collect(handle, { wait: false })` answers null while the step still runs.
 * @typedef {object} Carrier
 * @property {string} name
 * @property {CarrierTerms} terms
 * @property {(step: Step, ctx: object) => Promise<Admission>} admit
 * @property {(step: Step, ctx: object) => Promise<OfferAnswer>} offer
 * @property {(handle: Handle, opts?: { wait?: boolean }) => Promise<Collected|null>} collect
 * @property {(handle: Handle) => Promise<{ left: number[] }>} withdraw
 */

/**
 * @typedef {object} CarryPlan
 * @property {string[]} inline                                   in the order given
 * @property {{ project: string, carrier: string }[]} offers     what to offer to which carrier
 * @property {{ project: string, reason: string, owner: LimitOwner }[]} notOffered
 */

// ---- the one algorithm -------------------------------------------------------------------------

const minutes = seconds => `${Math.round(seconds / 60)} min`;

/** The first term `step` fails for a carrier, or null when the terms admit it. Pure. */
export function termsRefusal(step, terms, steps) {
  if (terms.cargo && step.run?.kind === 'attested') return 'an attested gate is a forge read; nothing to carry';
  if (terms.cargo && !step.run?.cargo) return 'not a cargo gate';
  if (terms.buildJobs != null && step.demand.buildJobs !== terms.buildJobs) {
    return `builds with ${step.demand.buildJobs ?? 'an undeclared number of'} jobs, not ${terms.buildJobs}`;
  }
  if (step.p90 == null) return 'no recorded run to expect a cost from';
  if (step.p90 < terms.minP90) return `p90 ${minutes(step.p90)} is under ${minutes(terms.minP90)}`;
  if (terms.ownSlot && step.demand.slot) {
    const sharer = steps.find(other => other.project !== step.project && other.demand.slot === step.demand.slot);
    if (sharer) return `shares its cargo target with ${sharer.project}`;
  }
  return null;
}

/**
 * Which steps run inline and which step is offered to which carrier. Pure: it reads only the steps
 * and each carrier's declared terms; capacity is the carrier's own answer at offer time. Each carrier
 * in order takes its eligible steps with the largest p90 (a tie keeps the given order); every step
 * not offered carries the reason the first carrier gave it.
 * @param {Step[]} steps
 * @param {Pick<Carrier, 'name'|'terms'>[]} carriers
 * @returns {CarryPlan}
 */
export function planCarry(steps, carriers) {
  const offers = [];
  const taken = new Set();
  const why = new Map();
  for (const carrier of carriers) {
    const { terms } = carrier;
    const ranked = [];
    for (const [index, step] of steps.entries()) {
      if (taken.has(step.project)) continue;
      const refusal = terms.overlapOnly && steps.length < 2
        ? 'the only step: carrying it overlaps nothing'
        : termsRefusal(step, terms, steps);
      if (refusal) {
        if (!why.has(step.project)) why.set(step.project, `${carrier.name}: ${refusal}`);
      } else {
        ranked.push({ step, index });
      }
    }
    ranked.sort((a, b) => b.step.p90 - a.step.p90 || a.index - b.index);
    for (const { step } of ranked) {
      const room = terms.steps - offers.filter(o => o.carrier === carrier.name).length;
      const keepsOneInline = !terms.overlapOnly || steps.length - taken.size > 1;
      if (room > 0 && keepsOneInline) {
        offers.push({ project: step.project, carrier: carrier.name });
        taken.add(step.project);
      } else if (!why.has(step.project)) {
        why.set(step.project, `${carrier.name}: its ${terms.steps} step(s) went to a longer one`);
      }
    }
  }
  const notOffered = steps.filter(step => !taken.has(step.project)).map(step => ({
    project: step.project, reason: why.get(step.project) || 'no carrier declared', owner: 'operator',
  }));
  return { inline: steps.filter(step => !taken.has(step.project)).map(step => step.project), offers, notOffered };
}

/** The switch, read once: off | shadow | on; anything else reads as shadow and says so. */
export function carryMode(env = process.env) {
  const raw = env.GATE_CARRY;
  if (raw === undefined || raw === '') return { mode: 'shadow' };
  if (['off', 'shadow', 'on'].includes(raw)) return { mode: raw };
  return { mode: 'shadow', note: `GATE_CARRY=${raw} is not off|shadow|on; reading it as shadow` };
}

/** Steps for the named projects, from the manifests and the recorded cost fold. */
export function stepsFor(names, { root, rows, registry = loadGateRegistry(root) }) {
  let ledger = rows;
  if (ledger === undefined) {
    try {
      const union = readCycleUnion(root);
      ledger = union.readableLedgers ? union.rows : [];
    } catch { ledger = []; }
  }
  return names.map(name => {
    const project = registry.get(name);
    if (!project) return { project: name, run: {}, key: null, p90: null, demand: { buildJobs: null, slot: null } };
    const cargo = project.run?.cargo;
    let buildJobs = null;
    if (cargo) {
      const resolved = gateChildEnv(project, {}, root, { warn: () => {} }).GATE_CARGO_ENV;
      const jobs = resolved ? JSON.parse(resolved).CARGO_BUILD_JOBS : undefined;
      buildJobs = jobs === undefined ? null : Number(jobs);
    }
    // The cargo slot is the pool's (workspace, profile) or a fixed target dir: two gates naming the
    // same one build in one target directory and queue on cargo's own lock.
    const slot = !cargo ? null : cargo.workspace
      ? `workspace:${cargo.workspace}:${cargo.profile || 'dev'}`
      : cargo.targetDir ? `target:${worktreeTargetDir(cargo.targetDir, root)}` : null;
    const p90 = project.run?.kind === 'attested' ? 0 : costOf(ledger, { project, subjectFallback: null }).p90;
    return { project: name, run: project.run, key: null, p90, demand: { buildJobs, slot } };
  });
}

// ---- the first carrier: this device's second cargo seat ------------------------------------------

function readJson(path) {
  try { return JSON.parse(readFileSync(path, 'utf8')); } catch { return null; }
}

function writeJsonAtomic(path, value) {
  writeFileSync(`${path}.tmp`, JSON.stringify(value));
  renameSync(`${path}.tmp`, path);
}

/** /proc/<pid>/stat as { state, pgrp, start }; null when the process is gone. */
function procStat(pid, procDir = '/proc') {
  try {
    const text = readFileSync(join(procDir, String(pid), 'stat'), 'utf8');
    const fields = text.slice(text.lastIndexOf(')') + 2).split(' ');
    return { state: fields[0], pgrp: Number(fields[2]), start: fields[19] };
  } catch { return null; }
}

/** Live (non-zombie) members of process group `pgid`. */
export function groupMembers(pgid, procDir = '/proc') {
  const members = [];
  let names = [];
  try { names = readdirSync(procDir); } catch { return members; }
  for (const name of names) {
    if (!/^\d+$/.test(name)) continue;
    const stat = procStat(name, procDir);
    if (stat && stat.pgrp === pgid && stat.state !== 'Z') members.push(Number(name));
  }
  return members;
}

const sleep = ms => new Promise(done => setTimeout(done, ms));

function exitStatus(exit) {
  if (exit.kind === 'exited') return exit.code;
  if (exit.kind === 'signaled') return 128 + (osConstants.signals[exit.signal] || 0);
  return 1;
}

/** Ask this device's berth whether `want` more builds may start; never throws. */
function askRoom(want, { root, env, spawnSync: run = spawnSync }) {
  const berth = env.GATE_CARRY_BERTH || resolve(root, 'genesis/agentic/bin/berth');
  try {
    const r = run(berth, ['room', 'cargo', '--json', '--want', String(want)], {
      cwd: root, env, encoding: 'utf8', timeout: 15_000,
    });
    const room = JSON.parse(r.stdout || 'null');
    if (room && (room.verdict === 'admit' || room.verdict === 'refuse')) return room;
    return { verdict: 'refuse', axis: 'host-floor', owner: 'self',
      reason: `berth room gave no verdict (exit ${r.status ?? r.signal ?? r.error?.code})` };
  } catch (error) {
    return { verdict: 'refuse', axis: 'host-floor', owner: 'self', reason: `berth room unreadable: ${error.message}` };
  }
}

/**
 * The second-seat carrier. `deps` are its far side: how it asks for room, what command a step runs,
 * where /proc is, and how long withdraw waits. The defaults are this device.
 */
export function makeSecondSeat(deps = {}) {
  const procDir = deps.procDir || '/proc';
  const pollMs = deps.pollMs ?? 500;
  const graceMs = deps.graceMs ?? 5000;
  // The loop keeps its own seat while this one runs, so the question is room for two.
  const room = deps.room || ((want, ctx) => askRoom(want, ctx));
  const command = deps.command || ((step, ctx) => [process.execPath, resolve(ctx.root, 'genesis/orchestrator/gate-runner.mjs'),
    '--target', step.project]);
  const terms = { steps: 1, cargo: true, buildJobs: 1, minP90: 600, ownSlot: true, overlapOnly: true };

  const alive = handle => {
    const stat = procStat(handle.pid, procDir);
    return Boolean(stat && stat.state !== 'Z' && stat.start === handle.pidStart);
  };

  /** @type {Carrier} */
  const carrier = {
    name: 'second-seat',
    terms,
    async admit(step, ctx) {
      const refusal = termsRefusal(step, terms, ctx.steps || [step]);
      if (refusal) return { admitted: false, reason: refusal, owner: 'operator' };
      const answer = await room(2, ctx);
      if (answer.verdict !== 'admit') {
        return { admitted: false, reason: `${answer.axis}: ${answer.reason}`, owner: answer.owner || 'self' };
      }
      return { admitted: true };
    },
    async offer(step, ctx) {
      const admission = await carrier.admit(step, ctx);
      if (!admission.admitted) return { carried: false, reason: admission.reason, owner: admission.owner };
      const dir = ctx.dirFor(step.project);
      mkdirSync(dir, { recursive: true });
      const logPath = join(dir, 'output.log');
      writeJsonAtomic(join(dir, 'spec.json'), {
        argv: command(step, ctx), cwd: ctx.root, env: { GATE_RUN_KIND: 'pre-push', GATE_REMAINING: ctx.remaining ?? '' },
      });
      const log = openSync(logPath, 'a');
      let child;
      try {
        // detached: the supervisor leads its own session and process group, so withdraw can end the
        // step and everything it started, and a terminal's Ctrl-C reaches the hook, never the step.
        child = spawn(process.execPath, [SELF, 'supervise', dir], {
          cwd: ctx.root, detached: true, stdio: ['ignore', log, log], env: ctx.env,
        });
      } finally { closeSync(log); }
      child.unref();
      const pidStart = procStat(child.pid, procDir)?.start ?? null;
      const handle = { carrier: carrier.name, project: step.project, dir, logPath, pid: child.pid, pidStart,
        startedAt: Date.now() };
      try {
        writeJsonAtomic(join(dir, 'handle.json'), handle);
      } catch (error) {
        // Unrecorded, the step could never be collected or withdrawn: end it, and let the loop run it inline.
        await carrier.withdraw(handle);
        return { carried: false, reason: `could not record the carried step: ${error.message}`, owner: 'self' };
      }
      return { carried: true, handle };
    },
    async collect(handle, { wait = true } = {}) {
      for (;;) {
        let ended = readJson(join(handle.dir, 'exit.json'));
        if (!ended && !alive(handle)) {
          ended = readJson(join(handle.dir, 'exit.json'))
            || { exit: { kind: 'unknown', why: 'the step ended without recording how' },
              seconds: Math.round((Date.now() - handle.startedAt) / 1000) };
        }
        if (ended) {
          return { project: handle.project, exit: ended.exit, status: exitStatus(ended.exit),
            seconds: ended.seconds, logPath: handle.logPath, basis: 'computed' };
        }
        if (!wait) return null;
        await sleep(pollMs);
      }
    },
    async withdraw(handle) {
      const pgid = handle.pid;
      for (const [signal, waitMs] of [['SIGTERM', graceMs], ['SIGKILL', 2000]]) {
        if (groupMembers(pgid, procDir).length === 0) break;
        try { process.kill(-pgid, signal); } catch { /* the group ended between the read and the signal */ }
        const deadline = Date.now() + waitMs;
        while (groupMembers(pgid, procDir).length && Date.now() < deadline) await sleep(50);
      }
      const left = groupMembers(pgid, procDir);
      if (!left.length && existsSync(handle.dir)) {
        try { writeJsonAtomic(join(handle.dir, 'withdrawn.json'), { at: Date.now() }); } catch { /* the marker is a courtesy */ }
      }
      return { left };
    },
  };
  return carrier;
}

export const secondSeat = makeSecondSeat();

/** The carriers this device offers to, in order. The only place a carrier is named. */
export const CARRIERS = [secondSeat];

// ---- the supervisor: the far side of a second-seat offer --------------------------------------------

/** Runs the step, records how it ended, exits. Never collects for anyone: it writes exit.json. */
async function supervise(dir) {
  const spec = readJson(join(dir, 'spec.json'));
  const started = Date.now();
  const [bin, ...args] = spec.argv;
  const child = spawn(bin, args, { cwd: spec.cwd, stdio: ['ignore', 'inherit', 'inherit'],
    env: { ...process.env, ...spec.env } });
  const exit = await new Promise(done => {
    child.on('error', error => done({ kind: 'unknown', why: `could not start: ${error.message}` }));
    child.on('exit', (code, signal) => done(signal ? { kind: 'signaled', signal } : { kind: 'exited', code }));
  });
  writeJsonAtomic(join(dir, 'exit.json'), { exit, seconds: Math.round((Date.now() - started) / 1000) });
}

// ---- the CLI the hook calls -------------------------------------------------------------------

function loadHandle(dir) {
  const handle = readJson(join(dir, 'handle.json'));
  if (!handle) throw new Error(`no carried step recorded at ${dir}`);
  const carrier = CARRIERS.find(c => c.name === handle.carrier);
  if (!carrier) throw new Error(`no carrier named ${handle.carrier} on this device`);
  return { handle, carrier };
}

function carryDirFor(env) {
  const run = env.GATE_LIVE_RUN;
  let base = null;
  return project => {
    if (run) return `${run.replace(/\.jsonl$/, '')}.carry/${project}`;
    base ||= mkdtempSync(join(tmpdir(), 'gate-carry-'));
    return join(base, project);
  };
}

const say = line => process.stderr.write(`[carry] ${line}\n`);

/** One step's expectation, in the reason line's words. */
function costWords(step) {
  return `p90 ${minutes(step.p90)}, ${step.demand.buildJobs === 1 ? 'one build job' : `${step.demand.buildJobs} build jobs`}`;
}

async function offerCli(names, { env, root }) {
  const { mode, note } = carryMode(env);
  if (mode === 'off') return 0;
  if (note) say(note);
  const steps = stepsFor(names, { root });
  const plan = planCarry(steps, CARRIERS);
  if (!plan.offers.length) {
    const nearest = [...plan.notOffered].sort((a, b) =>
      (steps.find(s => s.project === b.project).p90 ?? -1) - (steps.find(s => s.project === a.project).p90 ?? -1))[0];
    say(`${mode === 'shadow' ? 'shadow: ' : ''}nothing carried${nearest ? `: ${nearest.project}: ${nearest.reason} (${nearest.owner})` : ''}`);
    return 0;
  }
  const started = [];
  const withdrawStarted = async () => {
    for (const { carrier, handle } of started) await carrier.withdraw(handle);
  };
  // A signal between a start and its handle reaching the hook must not leave the step running.
  for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) {
    process.once(signal, () => { withdrawStarted().finally(() => process.exit(128 + osConstants.signals[signal])); });
  }
  const dirFor = carryDirFor(env);
  for (const { project, carrier: name } of plan.offers) {
    const carrier = CARRIERS.find(c => c.name === name);
    const step = steps.find(s => s.project === project);
    const ctx = { root, env, steps, dirFor, remaining: plan.inline.join(' ') };
    if (mode === 'shadow') {
      const admission = await carrier.admit(step, ctx);
      say(admission.admitted
        ? `shadow: would carry ${project} on ${name} (${costWords(step)}); GATE_CARRY=on carries it`
        : `shadow: not carried: ${project}: ${admission.reason} (${admission.owner})`);
      continue;
    }
    const answer = await carrier.offer(step, ctx);
    if (!answer.carried) {
      say(`not carried: ${project}: ${answer.reason} (${answer.owner})`);
      continue;
    }
    started.push({ carrier, handle: answer.handle });
    say(`carried: ${project} on ${name} (${costWords(step)}) beside the loop; output: ${answer.handle.logPath}`);
    process.stdout.write(`${project}\t${answer.handle.dir}\n`);
  }
  return 0;
}

export async function main(argv, { env = process.env } = {}) {
  const root = env.GATE_ROOT ? resolve(env.GATE_ROOT) : resolve(fileURLToPath(new URL('../..', import.meta.url)));
  const [verb, ...rest] = argv;
  try {
    if (verb === 'offer') return await offerCli(rest, { env, root });
    if (verb === 'plan') {
      process.stdout.write(`${JSON.stringify(planCarry(stepsFor(rest, { root }), CARRIERS))}\n`);
      return 0;
    }
    if (verb === 'supervise') { await supervise(rest[0]); return 0; }
    if (verb === 'peek') {
      const { handle, carrier } = loadHandle(rest[0]);
      const done = await carrier.collect(handle, { wait: false });
      process.stdout.write(`${done ? done.status : 'running'}\n`);
      return 0;
    }
    if (verb === 'collect') {
      const { handle, carrier } = loadHandle(rest[0]);
      if (!(await carrier.collect(handle, { wait: false }))) say(`waiting for ${handle.project} on ${carrier.name}`);
      const done = await carrier.collect(handle);
      let output = '';
      try { output = readFileSync(done.logPath, 'utf8'); } catch { output = `(no output recorded at ${done.logPath})\n`; }
      const how = done.exit.kind === 'unknown' ? `, ${done.exit.why}` : '';
      process.stdout.write(`──── carried: ${done.project} on ${carrier.name} (${done.logPath}) ────\n${output}`
        + `${output.endsWith('\n') || !output ? '' : '\n'}──── end of ${done.project}: exit ${done.status}${how} ────\n`
        + `carry-verdict ${done.project} ${done.status} ${done.seconds}\n`);
      return 0;
    }
    if (verb === 'withdraw') {
      let failed = 0;
      for (const dir of rest) {
        const { handle, carrier } = loadHandle(dir);
        const { left } = await carrier.withdraw(handle);
        say(left.length
          ? `withdraw: ${handle.project} on ${carrier.name} left pid(s) ${left.join(' ')} running`
          : `withdrawn: ${handle.project} from ${carrier.name} (output: ${handle.logPath})`);
        failed ||= left.length ? 1 : 0;
      }
      return failed;
    }
  } catch (error) {
    say(`${verb}: ${error.message}`);
    return 1;
  }
  process.stderr.write('usage: gate-carry.mjs (offer|plan <project…> | peek|collect <handle> | withdraw <handle…>)\n'
    + '  env: GATE_CARRY=off|shadow|on (default shadow) · GATE_ROOT · GATE_CARRY_BERTH (a berth binary)\n');
  return 2;
}

if (process.argv[1] && resolve(process.argv[1]) === SELF) {
  main(process.argv.slice(2)).then(code => { process.exitCode = code; });
}
