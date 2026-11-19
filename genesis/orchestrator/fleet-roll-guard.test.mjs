// fleet-roll-guard.mjs: a push to dev or main that CI would answer by rebuilding elohim-edge and
// rolling its fleet refuses in seconds unless a pushed commit declares `[edge:roll]`. Table tests
// use an injected walker; the last block runs the real walker over this tree's manifests.
// Habit `careless-reds-cost-seconds`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  checkFleetRoll, evaluateFleetRoll, fleetRollMode, parseRanges, readMessages, treeWalker,
} from './fleet-roll-guard.mjs';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));

const EDGE_BUILD = {
  name: 'elohim-edge',
  reasons: ['source: elohim/sdk/schemas/v1/gate-live-event.schema.json', 'source: elohim/Cargo.lock', 'source: elohim/Cargo.lock',
    'source: crates/seam-contracts/src/lib.rs', 'source: genesis/a2o/steps/devflow/honest-draw.steps.ts'],
  steps: [{ name: 'cargo-build-storage', class: 'build' }, { name: 'cargo-build-doorway', class: 'build' }, { name: 'dataplane-validation', class: 'build' }],
  runClass: 'build',
};
const EDGE_VALIDATE = {
  name: 'elohim-edge', reasons: ['source: genesis/a2o/steps/devflow/honest-draw.steps.ts'],
  steps: [{ name: 'dataplane-validation', class: 'build' }], runClass: 'build',
};
const OTHER = { name: 'elohim', reasons: ['source: app/elohim-app/src/main.ts'], steps: [{ name: 'build-app', class: 'build' }], runClass: 'build' };

function walkerOf(...pipelines) {
  const calls = [];
  const walk = changed => { calls.push(changed); return { projects: [], pipelines }; };
  walk.calls = calls;
  return walk;
}
const CHANGED = ['elohim/sdk/schemas/v1/gate-live-event.schema.json'];
const registryWithEdge = () => new Map([
  ['elohim-edge', { pipeline: 'elohim-edge', manualOnly: false }],
  ['elohim', { pipeline: 'elohim', manualOnly: false }],
]);
const evaluate = (over = {}) => evaluateFleetRoll({
  changed: CHANGED, messages: ['feat(x): a change'], target: 'dev', mode: 'refuse', walk: walkerOf(EDGE_BUILD), ...over,
});

// ── Outcomes ────────────────────────────────────────────────────────────────

test('an undeclared roll on a push to dev refuses, naming each build step, three paths and the limit', () => {
  const { row, prediction } = evaluate();
  assert.equal(prediction.roll, true);
  assert.equal(row.check, 'fleet-roll');
  assert.equal(row.level, 'REFUSED');
  assert.match(row.why, /^this push to dev would rebuild and roll the Edge fleet: elohim-edge builds cargo-build-storage, cargo-build-doorway, selected by /);
  assert.doesNotMatch(row.why, /builds [^:]*dataplane-validation/, 'the validation step is not a build step');
  assert.match(row.why, /elohim\/sdk\/schemas\/v1\/gate-live-event\.schema\.json, elohim\/Cargo\.lock, crates\/seam-contracts\/src\/lib\.rs and 1 more/);
  assert.match(row.why, /no pushed commit declares \[edge:roll\]/);
  assert.match(row.why, /an Edge step left stale by an earlier landing is not seen/);
  assert.match(row.cure, /take the change out of the Edge build inputs/);
  assert.match(row.cure, /land it with the next deliberate roll/);
  assert.match(row.cure, /add \[edge:roll\] and the reason to a pushed commit message/);
  assert.match(row.cure, /\[edge:validate-only\] on the tip also stops the roll, but it drops every other pipeline/);
});

test('a push to main refuses the same way', () => {
  assert.equal(evaluate({ target: 'main' }).row.level, 'REFUSED');
});

test('a roll declared by [edge:roll] in any pushed commit is ok and names the building steps', () => {
  const tip = evaluate({ messages: ['fix: tip', 'feat(sdk): schema field\n\nRolls alpha on purpose. [edge:roll]', 'chore: base'] }).row;
  assert.equal(tip.level, 'ok');
  assert.match(tip.why, /^roll declared by \[edge:roll\] in a pushed commit: elohim-edge builds cargo-build-storage, cargo-build-doorway/);
  assert.match(evaluate({ messages: ['feat: [EDGE:ROLL] storage'] }).row.why, /in the tip commit/);
});

test('an Edge selection of dataplane-validation alone is ok: it validates only', () => {
  const { row, prediction } = evaluate({ walk: walkerOf(EDGE_VALIDATE) });
  assert.equal(prediction.roll, false);
  assert.equal(row.level, 'ok');
  assert.equal(row.why, 'elohim-edge validates only (dataplane-validation); no build, no fleet roll');
});

test('a manifest-only change capped below deploy validates only, as CI dispatches it', () => {
  const capped = { ...EDGE_BUILD, steps: [{ name: 'cargo-build-storage', class: 'build' }], runClass: 'verify' };
  assert.equal(evaluate({ walk: walkerOf(capped) }).row.level, 'ok');
});

test('no Edge selection stays silent, like every unaffected preflight check', () => {
  assert.equal(evaluate({ walk: walkerOf(OTHER) }).row, null);
  assert.equal(evaluate({ walk: walkerOf() }).row, null);
});

test('mode warn gives the refusal text as WARN; mode off gives no row and walks nothing', () => {
  const refused = evaluate().row;
  const warned = evaluate({ mode: 'warn' }).row;
  assert.equal(warned.level, 'WARN');
  assert.equal(warned.why, refused.why);
  assert.equal(warned.cure, refused.cure);
  const walk = walkerOf(EDGE_BUILD);
  assert.deepEqual(evaluate({ mode: 'off', walk }), { prediction: null, row: null });
  assert.equal(walk.calls.length, 0);
});

test('off a deploying branch, or outside a push, a would-be roll warns and never refuses', () => {
  const feature = evaluate({ target: 'self/shem/x' }).row;
  assert.equal(feature.level, 'WARN');
  assert.match(feature.why, /^this change would roll the Edge fleet if pushed to dev \(target self\/shem\/x does not deploy\)/);
  const gate = evaluate({ target: null, messages: [] }).row;
  assert.equal(gate.level, 'WARN');
  assert.match(gate.why, /if pushed to dev \(not a push\)/);
  assert.equal(evaluate({ target: 'claude/feature', messages: new Error('x') }).row.level, 'WARN');
});

test('unreadable commit messages or changed paths refuse on a deploying push, never read as no tags', () => {
  const r = evaluate({ messages: new Error('git log a..b exited 128: bad revision') }).row;
  assert.equal(r.level, 'REFUSED');
  assert.match(r.why, /^push to dev: the pushed commits' messages could not be read \(git log a\.\.b exited 128: bad revision\)/);
  assert.match(r.why, /never reads as "nothing changed"/);
  assert.match(r.cure, /git fetch origin/);
  assert.equal(evaluate({ changed: null }).row.level, 'REFUSED');
  assert.equal(evaluate({ messages: new Error('x'), mode: 'warn' }).row.level, 'WARN');
});

// ── Tags, read the way CI reads them: the tip commit's message ──────────────

test('forcing tags on the tip predict a roll even with no Edge path: [build:edge], a list, [build:all], [deploy-only]', () => {
  const none = walkerOf(OTHER);
  for (const tip of ['ci: measure [build:edge]', 'ci: [build:dna,edge]', 'ci: [BUILD:Edge]', 'ci: [build:all]', 'ops: [deploy-only]']) {
    const { row, prediction } = evaluate({ walk: none, messages: [tip], registry: registryWithEdge });
    assert.equal(row?.level, 'REFUSED', tip);
    assert.equal(prediction.roll, true, tip);
    assert.match(row.why, /the tip's \[.*\] forces a full elohim-edge run/, tip);
    assert.match(row.cure, /drop \[.*\] from the tip commit/, tip);
  }
  assert.equal(evaluate({ walk: none, messages: ['ci: [build:edge] [edge:roll]'] }).row.level, 'ok', 'a forced roll can be declared');
});

test('a forcing tag below the tip is inert, as in CI', () => {
  assert.equal(evaluate({ walk: walkerOf(OTHER), messages: ['feat: tip', 'ci: [build:edge]'] }).row, null);
});

test('suppressing tags on the tip: [skip ci], [edge:validate-only] (over [build:edge]) and [run:verify] keep Edge from rolling', () => {
  for (const [tip, tag] of [
    ['docs: x [skip ci]', '[skip ci]'], ['docs: x [ci skip]', '[skip ci]'], ['docs: x [no ci]', '[skip ci]'],
    ['measure [build:edge] [edge:validate-only]', '[edge:validate-only]'],
    ['measure [edge:validate-only] [deploy-only]', '[edge:validate-only]'],
    ['measure [run:verify]', '[run:verify]'],
  ]) {
    const { row, prediction } = evaluate({ messages: [tip], registry: registryWithEdge });
    assert.equal(prediction.roll, false, tip);
    assert.equal(row.level, 'ok', tip);
    assert.equal(row.why, `the tip's ${tag} keeps elohim-edge from building or deploying for this push to dev`, tip);
  }
});

test('[run:verify] does not cap a forced or deploy-only Edge run; a suppressing tag below the tip is inert', () => {
  assert.equal(evaluate({ messages: ['[run:verify] [build:edge]'], registry: registryWithEdge }).row.level, 'REFUSED');
  assert.equal(evaluate({ messages: ['[run:verify] [deploy-only]'] }).row.level, 'REFUSED');
  assert.equal(evaluate({ messages: ['feat: tip', 'docs: [skip ci]', 'x [edge:validate-only]'] }).row.level, 'REFUSED');
});

// ── Reading the push ────────────────────────────────────────────────────────

test('GATE_PUSH_RANGES lines parse into ref, range, pushed sha and branch', () => {
  assert.deepEqual(parseRanges('refs/heads/dev abc..def\nrefs/tags/v1 origin/dev..123\n\n'), [
    { ref: 'refs/heads/dev', range: 'abc..def', sha: 'def', branch: 'dev' },
    { ref: 'refs/tags/v1', range: 'origin/dev..123', sha: '123', branch: null },
  ]);
});

test('a failed git call is an error, never an empty message list', () => {
  const failing = () => ({ status: 128, stdout: '', stderr: 'fatal: bad revision\n' });
  const result = readMessages({ root: '/x', range: 'a..b', sha: 'b', spawn: failing });
  assert.ok(result instanceof Error);
  assert.match(result.message, /exited 128: fatal: bad revision/);
  const spawned = () => ({ error: Object.assign(new Error('spawn git ENOENT'), { code: 'ENOENT' }), status: null, stdout: '' });
  assert.ok(readMessages({ root: '/x', range: 'a..b', sha: 'b', spawn: spawned }) instanceof Error);
  const rows = checkFleetRoll({ root: '/x', changed: CHANGED, walk: walkerOf(EDGE_BUILD), spawn: failing, mode: 'refuse',
    env: { GATE_PUSH_TARGETS: 'refs/heads/dev', GATE_PUSH_RANGES: 'refs/heads/dev a..b' } });
  assert.equal(rows.length, 1);
  assert.equal(rows[0].level, 'REFUSED');
  assert.match(rows[0].why, /could not be read/);
});

test('a push whose hook hands over no range refuses; outside a push the check needs a changed list', () => {
  const rows = checkFleetRoll({ root: '/x', changed: CHANGED, walk: walkerOf(EDGE_BUILD), mode: 'refuse', env: { GATE_PUSH_TARGETS: 'refs/heads/dev' } });
  assert.equal([].concat(rows)[0].level, 'REFUSED');
  assert.match([].concat(rows)[0].why, /GATE_PUSH_RANGES is not set/);
  assert.equal(checkFleetRoll({ root: '/x', changed: null, walk: walkerOf(EDGE_BUILD), mode: 'refuse', env: {} }), null);
  assert.equal(checkFleetRoll({ root: '/x', changed: CHANGED, walk: walkerOf(EDGE_BUILD), mode: 'refuse', env: {} }).level, 'WARN');
  assert.equal(checkFleetRoll({ root: '/x', changed: CHANGED, walk: walkerOf(EDGE_BUILD), mode: 'off', env: { GATE_PUSH_TARGETS: 'refs/heads/dev' } }), null);
});

test('a real pushed range: the declaration in a middle commit is read through git log', (t) => {
  const dir = mkdtempSync(join(tmpdir(), 'fleet-roll-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const env = Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith('GIT_') || k.startsWith('GIT_CONFIG_')));
  const git = (...args) => {
    const r = spawnSync('git', ['-C', dir, '-c', 'user.name=t', '-c', 'user.email=t@t', '-c', 'commit.gpgsign=false', ...args], { encoding: 'utf8', env });
    assert.equal(r.status, 0, r.stderr);
    return r.stdout.trim();
  };
  git('init', '--quiet');
  git('commit', '--quiet', '--allow-empty', '-m', 'base');
  const base = git('rev-parse', 'HEAD');
  git('commit', '--quiet', '--allow-empty', '-m', 'feat(sdk): a field\n\nThe roll is deliberate. [edge:roll]');
  git('commit', '--quiet', '--allow-empty', '-m', 'fix: tip');
  const tip = git('rev-parse', 'HEAD');
  const messages = readMessages({ root: dir, range: `${base}..${tip}`, sha: tip });
  assert.equal(messages[0], 'fix: tip');
  assert.equal(messages.length, 3, 'the tip, then the two pushed commits');
  const rows = checkFleetRoll({ root: dir, changed: CHANGED, walk: walkerOf(EDGE_BUILD), mode: 'refuse',
    env: { GATE_PUSH_TARGETS: 'refs/heads/dev', GATE_PUSH_RANGES: `refs/heads/dev ${base}..${tip}` } });
  assert.equal(rows[0].level, 'ok');
  assert.match(rows[0].why, /^roll declared by \[edge:roll\] in a pushed commit/);
  const undeclared = checkFleetRoll({ root: dir, changed: CHANGED, walk: walkerOf(EDGE_BUILD), mode: 'refuse',
    env: { GATE_PUSH_TARGETS: 'refs/heads/dev', GATE_PUSH_RANGES: `refs/heads/dev ${tip}..${tip}` } });
  assert.equal(undeclared[0].level, 'REFUSED', 'a range already upstream still reads its tip, and no declaration');
});

test('only deploying refs are judged when a push carries several', () => {
  const rows = checkFleetRoll({ root: '/x', changed: CHANGED, walk: walkerOf(EDGE_BUILD), mode: 'refuse',
    spawn: () => ({ status: 0, stdout: 'feat: x\x1e\n' }),
    env: { GATE_PUSH_TARGETS: 'refs/heads/self/a refs/heads/dev', GATE_PUSH_RANGES: 'refs/heads/self/a b..c\nrefs/heads/dev b..c' } });
  assert.equal(rows.length, 1);
  assert.match(rows[0].why, /push to dev/);
});

test('the policy mode reads pool-policy.json fleet_roll_guard.mode; absent or unknown reads as refuse', (t) => {
  const dir = mkdtempSync(join(tmpdir(), 'fleet-roll-policy-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  assert.equal(fleetRollMode(dir), 'refuse');
  mkdirSync(join(dir, 'genesis/agentic'), { recursive: true });
  for (const [mode, expected] of [['warn', 'warn'], ['off', 'off'], ['refuse', 'refuse'], ['sometimes', 'refuse']]) {
    writeFileSync(join(dir, 'genesis/agentic/pool-policy.json'), JSON.stringify({ fleet_roll_guard: { mode } }));
    assert.equal(fleetRollMode(dir), expected, mode);
  }
  assert.equal(fleetRollMode(ROOT), 'refuse', 'this tree declares refuse');
});

// ── The real walker over this tree's manifests ──────────────────────────────

const walk = treeWalker(ROOT);
const onDev = changed => evaluateFleetRoll({ changed, messages: ['feat: x'], target: 'dev', mode: 'refuse', walk, registry: () => new Map() });

test('real manifests: the steward\'s named paths each predict a roll', () => {
  for (const path of ['elohim/eprfs/device-runtime/src/lib.rs', 'elohim/Cargo.lock']) {
    const { row, prediction } = onDev([path]);
    assert.equal(prediction.roll, true, path);
    assert.equal(row.level, 'REFUSED', path);
  }
});

test('real manifests: the 2026-11-19 schema edit predicts a roll naming cargo-build-storage', () => {
  const { row } = onDev(['elohim/sdk/schemas/v1/gate-live-event.schema.json']);
  assert.equal(row.level, 'REFUSED');
  assert.match(row.why, /elohim-edge builds cargo-build-storage, selected by elohim\/sdk\/schemas\/v1\/gate-live-event\.schema\.json/);
});

test('real manifests: an a2o step file alone validates only', () => {
  const { row, prediction } = onDev(['genesis/a2o/steps/devflow/honest-draw.steps.ts']);
  assert.equal(prediction.roll, false);
  assert.equal(row.level, 'ok');
  assert.match(row.why, /validates only/);
});

// The seventeen paths of a real stack about to land on 2026-11-19
// (.worktrees/steward-arrival/genesis/local-dev/native-throughput/l2b-paths.txt).
const L2B_PATHS = [
  'elohim/eprfs/Cargo.lock',
  'elohim/eprfs/bin-build-view.json',
  'elohim/eprfs/epr-cli/Cargo.toml',
  'elohim/eprfs/epr-cli/build.rs',
  'elohim/eprfs/epr-cli/src/bin.rs',
  'elohim/eprfs/epr-cli/src/exercise.rs',
  'elohim/eprfs/epr-cli/src/lib.rs',
  'elohim/eprfs/epr-cli/src/main.rs',
  'elohim/eprfs/epr-cli/src/peer.rs',
  'elohim/eprfs/epr-cli/src/peer/custody_slot_tests.rs',
  'elohim/eprfs/epr-cli/src/peer/serve_check_seam_tests.rs',
  'elohim/eprfs/epr-cli/tests/artifact.rs',
  'elohim/eprfs/epr-cli/tests/bin_build_view.rs',
  'elohim/eprfs/epr-cli/tests/peer.rs',
  'genesis/a2o/features/devflow/honest-draw.feature',
  'genesis/a2o/steps/devflow/honest-draw.steps.ts',
  'justfile',
];

test('real manifests: the seventeen-path L2b stack validates only', () => {
  assert.equal(L2B_PATHS.length, 17);
  const { row, prediction } = onDev(L2B_PATHS);
  assert.equal(prediction.roll, false, JSON.stringify(prediction));
  assert.equal(row.level, 'ok');
  assert.match(row.why, /validates only/);
});
