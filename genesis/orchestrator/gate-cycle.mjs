// Gate cycle time — the observation, the verdict against the DECLARED ceiling,
// and the finding + dispatch directive when a gate has become too slow to live
// with. One producer (the gate runner), one declaration
// (.claude/epr-meta/measures.yaml), one ledger that already has a reader
// (.claude/data/architecture-findings.jsonl → habits-status).
//
// No threshold, project name, agent name or prompt lives in this file. Every
// entry point is fail-open: a gate's verdict never depends on this module.

import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { createHash } from 'crypto';
import { dirname, join } from 'path';
import { parse as parseYaml } from 'yaml';

export const GATE_CYCLE_MEASURE = 'gate-cycle-seconds@1';

/** Consecutive under-ceiling runs that close a finding — one may be a warm cache. */
const CLOSE_STREAK = 2;

/** The caller declares what kind of run this is; a hook's side effects are not a declaration. */
export function runKind(env) {
  const declared = (env.GATE_RUN_KIND || '').trim();
  return declared || 'full-gate';
}

/**
 * The sidecar addresses a subject by its bytes, so it must be a FILE (or `.`).
 * The gate recipe that was timed is the honest one; fall back to the package
 * manifest, then to the repository itself.
 */
export function subjectFor(project, root) {
  if (!project.dir || project.dir === '.') return '.';
  for (const anchor of ['justfile', 'Cargo.toml', 'package.json']) {
    if (existsSync(join(root, project.dir, anchor))) return `${project.dir}/${anchor}`;
  }
  return '.';
}

export function observationArgs(subject, seconds, run) {
  const args = [
    'flow', 'note', '--kind', 'observation',
    '--measure', GATE_CYCLE_MEASURE,
    '--subject', subject,
    '--value', String(Math.round(seconds)),
    '--unit', 'seconds',
    '--env', `run-kind=${run.kind}`,
    '--env', `exit=${run.exit}`,
  ];
  if (run.jobs) args.push('--env', `jobs=${run.jobs}`);
  return args;
}

/**
 * The ceiling declared for this project and run kind: a lens consuming the
 * measure whose `env.run-kind` matches, preferring one scoped to this subject.
 * `null` when nothing is declared — then this module has no opinion.
 */
export function readCeiling(measuresPath, subject, kind) {
  const doc = parseYaml(readFileSync(measuresPath, 'utf8')) || {};
  const candidates = (doc.lenses || []).filter(lens =>
    (lens.consumes || []).includes(GATE_CYCLE_MEASURE) &&
    (lens.status ?? 'active') === 'active' &&
    ((lens.env || {})['run-kind'] ?? 'full-gate') === kind &&
    (lens.subject === undefined || lens.subject === subject));
  const chosen = candidates.find(lens => lens.subject === subject) || candidates.find(lens => lens.subject === undefined);
  if (!chosen) return null;
  return {
    id: `${chosen.id}@${chosen.version}`,
    soft: chosen.soft ?? null,
    hard: chosen.hard ?? null,
    concern: chosen.concern ?? null,
    agent: chosen['dispatch-agent'] ?? null,
    prompt: chosen['dispatch-prompt'] ?? null,
  };
}

/** 'hard' | 'soft' | 'ok' — at the watermark counts (the registry compares at-or-above). */
export function judgeCycle(seconds, ceiling) {
  if (!ceiling) return 'ok';
  if (ceiling.hard !== null && seconds >= ceiling.hard) return 'hard';
  if (ceiling.soft !== null && seconds >= ceiling.soft) return 'soft';
  return 'ok';
}

export function fingerprint(boundId, subject, kind) {
  return createHash('sha256').update(`${boundId}|${subject}|${kind}`).digest('hex').slice(0, 12);
}

function readLedger(path) {
  if (!existsSync(path)) return [];
  return readFileSync(path, 'utf8').split('\n').filter(Boolean).map(line => {
    try { return JSON.parse(line); } catch { return { raw: line }; }
  });
}

function writeLedger(path, rows) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, rows.map(row => (row.raw !== undefined ? row.raw : JSON.stringify(row))).join('\n') + '\n');
}

function minutes(seconds) {
  return `${Math.round(seconds / 60)} min`;
}

function directive(project, seconds, ceiling, kind) {
  return [
    '',
    `[gate-cycle] ${project.name}: ${minutes(seconds)} (${Math.round(seconds)} s) is at/over the ${ceiling.hard} s hard ceiling ${ceiling.id} for a ${kind} run.`,
    `[gate-cycle] DISPATCH a background review now — agent: ${ceiling.agent || 'general-purpose'} · subject: ${project.dir}`,
    `[gate-cycle] charter: ${ceiling.prompt || 'Does this cycle time make sense — can this be modularized?'}`,
    '[gate-cycle] filed in .claude/data/architecture-findings.jsonl; it closes itself after two runs back under the ceiling.',
    '',
  ].join('\n');
}

/**
 * Record one project's gate duration and act on the declared ceiling.
 * `status` is the gate's exit status; a failed gate stopped early, so its
 * duration is recorded (exit=fail) and never judged.
 */
export function recordCycle(project, seconds, status, env, deps) {
  try {
    const kind = runKind(env);
    const exit = status === 0 ? 'ok' : 'fail';
    try {
      const subject = subjectFor(project, deps.root || process.cwd());
      deps.runEpr(observationArgs(subject, seconds, { kind, exit, jobs: env.CARGO_BUILD_JOBS }));
    } catch { /* the observation is best-effort; the verdict below does not need it */ }
    if (exit !== 'ok') return;

    const ceiling = readCeiling(deps.measuresPath, project.dir, kind);
    const verdict = judgeCycle(seconds, ceiling);
    if (!ceiling) return;
    const fp = fingerprint(ceiling.id, project.dir, kind);
    const rows = readLedger(deps.ledgerPath);
    const open = rows.find(row => row.fp === fp && row.status === 'open');

    if (verdict === 'hard') {
      if (open) {
        open.under_streak = 0;
        open.stock = Math.round(seconds);
        open.last_seen = deps.now();
        writeLedger(deps.ledgerPath, rows);
        deps.print(`[gate-cycle] ${project.name}: ${minutes(seconds)} — still over ${ceiling.id}; finding ${fp} is open.`);
        return;
      }
      mkdirSync(dirname(deps.ledgerPath), { recursive: true });
      appendFileSync(deps.ledgerPath, JSON.stringify({
        fp,
        rule: 'gate-cycle-ceiling',
        policy: ceiling.id,
        path: project.dir,
        detail: `\`${project.name}\` ${kind} took ${Math.round(seconds)} s — at/over the ${ceiling.hard} s hard ceiling. ${ceiling.prompt || ''}`.trim(),
        first_seen: deps.now(),
        status: 'open',
        stock: Math.round(seconds),
        limit: ceiling.hard,
        concern: ceiling.concern,
        run_kind: kind,
        under_streak: 0,
      }) + '\n');
      deps.print(directive(project, seconds, ceiling, kind));
      return;
    }

    if (verdict === 'soft') {
      deps.print(`[gate-cycle] ${project.name}: ${minutes(seconds)} is over the ${ceiling.soft} s soft ceiling ${ceiling.id} (hard ${ceiling.hard} s).`);
    }
    if (open) {
      open.under_streak = (open.under_streak || 0) + 1;
      if (open.under_streak >= CLOSE_STREAK) {
        open.status = 'closed';
        open.closed_at = deps.now();
        deps.print(`[gate-cycle] ${project.name}: back under ${ceiling.id} for ${CLOSE_STREAK} runs — finding ${fp} closed.`);
      }
      writeLedger(deps.ledgerPath, rows);
    }
  } catch { /* fail-open by contract */ }
}
