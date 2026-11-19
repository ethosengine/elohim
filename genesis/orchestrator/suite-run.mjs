#!/usr/bin/env node
// Run the orchestrator suite from suite-reads.json (E11).
//   node suite-run.mjs named   the named files, under read-fence.mjs; fails on any fence violation
//   node suite-run.mjs tree    the files keyed on the whole commit tree, unfenced
//   node suite-run.mjs all     both (package.json `test`)
// Groups run in list order, one runner per group, as the old package.json chain did. Every group
// runs even after one fails, so one run reports every red; the exit status is the first failure's.
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { buildPreload, checkLog } from './read-fence.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../..');
const FENCE = join(HERE, 'read-fence.mjs');

export function loadSuite(text = readFileSync(join(HERE, 'suite-reads.json'), 'utf8')) {
  return JSON.parse(text);
}

/** The commands one side runs: [{ runner, cmd, args }] in group order. */
export function plan(suite, side) {
  const steps = [];
  for (const group of suite.groups) {
    const files = group.files.filter(entry => side === 'all' || (side === 'tree') === Boolean(entry.tree)).map(entry => entry.file);
    if (!files.length) continue;
    if (group.runner === 'bash') {
      for (const file of files) steps.push({ runner: 'bash', cmd: 'bash', args: [file] });
    } else {
      const tsx = group.runner === 'node-tsx' ? ['--import', 'tsx'] : [];
      steps.push({ runner: group.runner, cmd: process.execPath, args: [...tsx, '--test', '--test-isolation=process', ...files] });
    }
  }
  return steps;
}

function runSide(suite, side, { env = process.env, spawn = spawnSync } = {}) {
  const fenced = side === 'named';
  const scratch = fenced ? mkdtempSync(join(tmpdir(), 'orchestrator-read-fence-')) : null;
  const log = fenced ? join(scratch, 'violations.jsonl') : null;
  const childEnv = { ...env };
  delete childEnv.NODE_TEST_CONTEXT;
  delete childEnv.GATE_READ_FENCE_FILE;
  if (fenced) {
    childEnv.NODE_OPTIONS = `${env.NODE_OPTIONS ? `${env.NODE_OPTIONS} ` : ''}--import=${FENCE}`;
    childEnv.GATE_READ_FENCE_ROOT = ROOT;
    childEnv.GATE_READ_FENCE_LOG = log;
    const common = spawn('git', ['rev-parse', '--path-format=absolute', '--git-common-dir'], { cwd: ROOT, encoding: 'utf8' });
    if (common.status !== 0 || !common.stdout.trim()) throw new Error('suite-run: cannot name the git common dir the fence guards');
    childEnv.GATE_READ_FENCE_GITDIR = common.stdout.trim();
    const so = buildPreload({ env, spawn });
    childEnv.GATE_READ_FENCE_SO = so;
    childEnv.LD_PRELOAD = env.LD_PRELOAD ? `${so}:${env.LD_PRELOAD}` : so;
  }
  let status = 0;
  try {
    for (const step of plan(suite, side)) {
      const result = spawn(step.cmd, step.args, { cwd: HERE, stdio: 'inherit', env: childEnv });
      const code = result.status ?? 1;
      if (code !== 0) {
        process.stdout.write(`[suite] ${side} group (${step.runner}) failed with ${code}: ${step.args.filter(a => /\.test\./.test(a)).length} file(s)\n`);
        if (status === 0) status = code;
      }
    }
    if (fenced && checkLog(log, process.stdout, suite) !== 0 && status === 0) status = 1;
  } finally {
    if (scratch) rmSync(scratch, { recursive: true, force: true });
  }
  return status;
}

function main(side) {
  if (!['named', 'tree', 'all'].includes(side)) throw new Error('Usage: node suite-run.mjs named|tree|all');
  const suite = loadSuite();
  if (side !== 'all') return runSide(suite, side);
  const named = runSide(suite, 'named');
  const tree = runSide(suite, 'tree');
  return named || tree;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { process.exitCode = main(process.argv[2]); }
  catch (error) { console.error(error.message); process.exitCode = 1; }
}
