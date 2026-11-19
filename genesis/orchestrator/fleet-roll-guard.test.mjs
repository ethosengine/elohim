// fleet-roll-guard.mjs: a pushed dev or main ref for which CI would select an Edge build and its
// deploy step is red unless a pushed commit carries `[edge:roll]`. Parity cases run CI's graph
// (graph-walker.mjs walkGraphCi) over fixture manifests and over this tree's real ones; per-ref
// cases use real git histories. Habit `careless-reds-cost-seconds`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ciGlobMatch, walkGraph } from './graph-walker.mjs';
import {
  checkFleetRoll, ciWalker, doneFlagHonoured, evaluateRef, failureRows, fleetRollMode, parseRefs, predict,
} from './fleet-roll-guard.mjs';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));

// A small graph with the shapes that matter: a cross-pipeline dependency, a manual-only producer,
// a dependency on a pipeline with no manifest, a validation step, the orchestrator, a pipeline
// without a jenkinsPath.
const FIXTURE = [
  { path: './c/build-manifest.json', content: { pipeline: 'elohim-conductor', jenkinsPath: 'c/Jenkinsfile', steps: { 'build-conductor-image': { inputs: { sources: ['pin'] } } } } },
  { path: './e/build-manifest.json', content: { pipeline: 'elohim-edge', jenkinsPath: 'e/Jenkinsfile', steps: {
    'cargo-build-storage': { inputs: { sources: ['sdk/**', 'e/build-manifest.json'] }, depends: ['elohim-conductor:build-conductor-image', 'elohim-ghost:x'] },
    'build-edge-image': { inputs: { sources: [] }, depends: ['cargo-build-storage', 'elohim-rakia:rakia-ci'] },
    'dataplane-validation': { inputs: { sources: ['steps/**'] } },
  } } },
  { path: './a/build-manifest.json', content: { pipeline: 'elohim', jenkinsPath: 'Jenkinsfile', steps: { 'build-app': { inputs: { sources: ['app/**', 'a/build-manifest.json'] } } } } },
  { path: './r/build-manifest.json', content: { pipeline: 'elohim-rakia', manualOnly: true, steps: { 'rakia-ci': { inputs: { sources: ['rk', 'rk/**'] } } } } },
  { path: './o/build-manifest.json', content: { pipeline: 'elohim-orchestrator', jenkinsPath: 'o/Jenkinsfile', steps: { lint: { inputs: { sources: ['**/Jenkinsfile*'] } } } } },
  { path: './l/build-manifest.json', content: { pipeline: 'local-only', steps: { s: { inputs: { sources: ['local/**'] } } } } },
];
// Every real manifest step declares `depends` (the path walker's topoSort reads it).
for (const { content } of FIXTURE) for (const step of Object.values(content.steps)) step.depends ??= [];
const fwalk = changed => walkGraph(FIXTURE, changed, { ci: true });
const onDev = (changed, tip = 'feat: x', over = {}) => evaluateRef({ target: 'dev', walked: fwalk(changed), tip, mode: 'refuse', ...over });
const only = rows => { assert.equal(rows.length, 1, JSON.stringify(rows)); return rows[0]; };
const BASELINE = /the next run's diff starts past this push; a later change that selects the same pipeline still builds these bytes; a failed or aborted run keeps the old baseline/;

// ── CI's graph, as an option on the walker ──────────────────────────────────

test('the default walker is unchanged: no propagation, picomatch dotfile rules', () => {
  assert.deepEqual(walkGraph(FIXTURE, ['pin']).pipelines.map(p => p.name), ['elohim-conductor']);
  assert.deepEqual(walkGraph(FIXTURE, ['sdk/.gitignore']).pipelines, []);
});

test('ci option: CI glob matches dotfiles, keeps segment wildcards, and skips .epr-meta', () => {
  for (const [file, pattern, hit] of [
    ['sdk/.gitignore', 'sdk/**', true], ['a/b.c', 'a/*.c', true], ['a/x/b.c', 'a/*.c', false],
    ['a/b', 'a/**/b', true], ['a/x/y/b', 'a/**/b', true], ['ab', 'a?', true], ['aXc', 'a.c', false],
  ]) assert.equal(ciGlobMatch(file, pattern), hit, `${file} ~ ${pattern}`);
  assert.deepEqual(fwalk(['sdk/.gitignore']).pipelines.map(p => p.name), ['elohim-edge']);
  assert.deepEqual(fwalk(['sdk/.epr-meta/rule.md']).pipelines, []);
});

test('ci option: staleness propagates over depends to a fixed point, with CI\'s reasons', () => {
  const edge = fwalk(['pin']).pipelines.find(p => p.name === 'elohim-edge');
  assert.deepEqual(edge.steps.map(s => [s.name, s.reason, s.cause]), [
    ['cargo-build-storage', 'depends: elohim-conductor:build-conductor-image', 'pin'],
    ['build-edge-image', 'depends: elohim-edge:cargo-build-storage', 'pin'],
  ]);
  assert.equal(fwalk(['sdk/x.json']).pipelines.find(p => p.name === 'elohim-edge').steps[0].reason, 'source: sdk/x.json matches sdk/**');
});

test('ci option: a manual-only step propagates but is not dispatched', () => {
  const walked = fwalk(['rk']);
  assert.deepEqual(walked.pipelines.map(p => p.name), ['elohim-edge']);
  assert.deepEqual(walked.pipelines[0].steps.map(s => s.name), ['build-edge-image']);
});

test('ci option: the manifest-only cap needs every trigger a manifest and no stale dependency', () => {
  assert.equal(fwalk(['a/build-manifest.json']).pipelines.find(p => p.name === 'elohim').runClass, 'verify');
  assert.equal(fwalk(['a/build-manifest.json', 'app/x.ts']).pipelines.find(p => p.name === 'elohim').runClass, 'build');
  // A step stale only by propagation is not "by source", so Edge's own manifest edit stays build in CI.
  assert.equal(fwalk(['e/build-manifest.json']).pipelines.find(p => p.name === 'elohim-edge').runClass, 'build');
});

// ── Parity cases (fixtures) ─────────────────────────────────────────────────

test('parity: propagation turns a push the path walker reads as validation only into a build', () => {
  const plain = walkGraph(FIXTURE, ['steps/a.ts', 'pin']).pipelines.find(p => p.name === 'elohim-edge');
  assert.deepEqual(plain.steps.map(s => s.name), ['dataplane-validation'], 'the trap: no propagation');
  const r = only(onDev(['steps/a.ts', 'pin']));
  assert.equal(r.level, 'REFUSED');
  assert.match(r.why, /^CI would select an Edge build and its deploy step for this push to dev: elohim-edge stale steps cargo-build-storage, build-edge-image, from pin, through elohim-conductor:build-conductor-image/);
});

test('parity: [deploy-only] over validation-only stays validate-only; over nothing, or over build inputs, it deploys', () => {
  const dv = only(onDev(['steps/a.ts'], 'ops: [deploy-only]'));
  assert.equal(dv.level, 'ok');
  assert.match(dv.why, /validates only/);
  const bare = only(onDev(['docs/a.md'], 'ops: [deploy-only]'));
  assert.equal(bare.level, 'REFUSED');
  assert.match(bare.why, /^the tip's \[deploy-only\] makes CI select the Edge deploy step for this push to dev \(its build stages are skipped\)/);
  assert.match(only(onDev(['sdk/x.json'], 'ops: [deploy-only]')).why, /^CI would select the Edge deploy step for this push to dev; the tip's \[deploy-only\] skips its build stages/);
});

test('parity: [build:all] on a docs tip is asked to be declared, saying CI forces nothing with it today', () => {
  const r = only(onDev(['docs/a.md'], 'docs: refresh [build:all]'));
  assert.equal(r.level, 'REFUSED');
  assert.match(r.why, /^the tip's \[build:all\] asks for every pipeline, Edge included, on this push to dev: CI forces nothing with it today/);
  assert.match(r.cure, /drop \[build:all\] from the tip commit/);
  assert.equal(only(onDev(['docs/a.md'], 'docs: [build:all]', { declaredBy: 'a'.repeat(40) })).level, 'ok');
});

test('parity: an explicit edge token forces the selection; a space inside the tag breaks both grammars', () => {
  for (const tip of ['ci: [build:edge]', 'ci: [BUILD:dna,Edge]']) {
    const r = only(onDev(['docs/a.md'], tip));
    assert.equal(r.level, 'REFUSED', tip);
    assert.match(r.why, /^the tip's \[build:[^\]]+\] makes CI select an Edge build and its deploy step/i, tip);
    assert.match(r.cure, /^drop \[build:[^\]]+\] from the tip commit/i, tip);
  }
  assert.deepEqual(onDev(['docs/a.md'], 'ci: [build:dna, edge]'), []);
});

// ── Outcomes ────────────────────────────────────────────────────────────────

test('an undeclared selection on dev is red, with cures that are true', () => {
  const r = only(onDev(['sdk/x.json']));
  assert.equal(r.level, 'REFUSED');
  assert.match(r.why, /no pushed commit carries \[edge:roll\]\. Read from this ref's own range and its tip's tags; an Edge step left stale by an earlier landing is not seen$/);
  assert.doesNotMatch(r.why, /\broll(s|ed)? the\b|rebuild and roll/);
  assert.match(r.cure, /take the Edge build inputs out of this push, or land them with the next deliberate roll/);
  assert.match(r.cure, /add \[edge:roll\] and the reason to a pushed commit message/);
  assert.doesNotMatch(r.cure, /own push|documentation only/);
  assert.equal(only(evaluateRef({ target: 'main', walked: fwalk(['sdk/x.json']), mode: 'refuse' })).level, 'REFUSED');
});

test('a declared selection is ok and names the commit', () => {
  const r = only(onDev(['sdk/x.json'], 'feat: x', { declaredBy: 'abcdef0123456789' }));
  assert.equal(r.level, 'ok');
  assert.match(r.why, /declared by \[edge:roll\] in abcdef012345$/);
});

test('validation-only selection is ok; no Edge selection is silent', () => {
  assert.match(only(onDev(['steps/a.ts'])).why, /validates only \(dataplane-validation\); CI selects no Edge build or deploy/);
  assert.deepEqual(onDev(['app/x.ts']), []);
});

test('mode warn gives the red text as WARN; off gives nothing; a default mode says why', () => {
  const red = only(onDev(['sdk/x.json']));
  const warn = only(onDev(['sdk/x.json'], 'feat: x', { mode: 'warn' }));
  assert.equal(warn.level, 'WARN');
  assert.equal(warn.why, red.why);
  assert.deepEqual(onDev(['sdk/x.json'], 'feat: x', { mode: 'off' }), []);
  assert.match(only(onDev(['sdk/x.json'], 'feat: x', { policyWhy: 'pool-policy.json is not valid JSON' })).why,
    /\(policy: mode refuse, the default, because pool-policy.json is not valid JSON\)$/);
});

test('off a deploying branch, or outside a push, a selection warns and never goes red', () => {
  const self = only(evaluateRef({ target: 'self/x', walked: fwalk(['sdk/x.json']), mode: 'refuse' }));
  assert.equal(self.level, 'WARN');
  assert.match(self.why, /for this change, were it pushed to dev \(target self\/x does not deploy\)/);
  assert.match(only(evaluateRef({ target: null, walked: fwalk(['sdk/x.json']), mode: 'refuse' })).why, /were it pushed to dev \(not a push\)/);
});

test('a skip tag over Edge build inputs warns twice with the true baseline words', () => {
  const rows = onDev(['sdk/x.json', 'app/y.ts', 'local/z', 'o/Jenkinsfile'], 'docs: notes [skip ci]');
  assert.deepEqual(rows.map(r => [r.check, r.level]), [['fleet-roll', 'WARN'], ['skip-ci', 'WARN']]);
  for (const r of rows) {
    assert.match(r.why, BASELINE);
    assert.doesNotMatch(r.why, /no later diff|only with a later forced/);
  }
  assert.match(rows[1].why, /^the tip's \[skip ci\] dispatches none of elohim-edge, elohim, which this ref's paths select/);
  assert.match(rows[1].cure, /a tip whose paths select no pipeline/);
});

test('skip-ci: silent over paths that select nothing, and with no skip tag', () => {
  assert.deepEqual(onDev(['docs/a.md'], 'docs [skip ci]'), []);
  assert.deepEqual(onDev(['app/x.ts'], 'feat: x'), []);
  assert.deepEqual(only(onDev(['app/x.ts'], 'docs [no ci]')).check, 'skip-ci');
});

test('[edge:validate-only] warns over Edge build inputs alone and over other pipelines; over validation alone it is ok', () => {
  const alone = only(onDev(['sdk/x.json'], 'measure [edge:validate-only] [build:edge]'));
  assert.equal(alone.level, 'WARN');
  assert.match(alone.why, /makes elohim-edge validate only for this push to dev: its Edge build inputs \(elohim-edge stale steps cargo-build-storage, build-edge-image, from sdk\/x.json\) are not built by this run; once the run ends green, the next run's diff/);
  assert.match(only(onDev(['app/x.ts', 'local/z'], 'measure [edge:validate-only]')).why, /it drops elohim from the run/);
  assert.equal(only(onDev(['steps/a.ts'], 'measure [edge:validate-only]')).level, 'ok');
});

test('[run:verify] keeps Edge out unless forced; with [deploy-only] Edge still deploys', () => {
  assert.match(only(onDev(['sdk/x.json'], '[run:verify]')).why, /^the tip's \[run:verify\] keeps elohim-edge out of the dispatch/);
  assert.equal(only(onDev(['sdk/x.json'], '[run:verify] [build:edge]')).level, 'REFUSED');
  assert.equal(only(onDev(['docs/a.md'], '[run:verify] [deploy-only]')).level, 'REFUSED');
});

test('predict reads tags from the tip only', () => {
  assert.equal(predict(fwalk(['docs/a.md']), 'feat: tip').selects, false);
});

// ── Each protected ref on its own (real git) ────────────────────────────────

const cleanEnv = Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith('GIT_') || k.startsWith('GIT_CONFIG_')));
function repo(t) {
  const dir = mkdtempSync(join(tmpdir(), 'fleet-roll-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const git = (...args) => {
    const r = spawnSync('git', ['-C', dir, '-c', 'user.name=t', '-c', 'user.email=t@t', '-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=/dev/null', ...args],
      { encoding: 'utf8', env: cleanEnv, maxBuffer: 64 * 1024 * 1024 });
    assert.equal(r.status, 0, r.stderr);
    return r.stdout.trim();
  };
  const commit = (files, message) => {
    for (const [path, text] of Object.entries(files)) { mkdirSync(join(dir, path, '..'), { recursive: true }); writeFileSync(join(dir, path), text); }
    git('add', '-A');
    const msg = join(dir, '..', `${dir.split('/').at(-1)}.msg`);
    writeFileSync(msg, message);
    t.after(() => rmSync(msg, { force: true }));
    git('commit', '--quiet', '--allow-empty', '-F', msg);
    return git('rev-parse', 'HEAD');
  };
  git('init', '--quiet', '-b', 'dev');
  return { dir, git, commit };
}
const spawnIn = (cmd, args, opts) => spawnSync(cmd, args, { ...opts, env: cleanEnv });
const check = (dir, refs, extra = {}) => checkFleetRoll({
  root: dir, walk: fwalk, spawn: spawnIn, policy: { mode: 'refuse', why: null },
  env: { GATE_PUSH_TARGETS: refs.map(r => r[0]).join(' '), GATE_PUSH_REFS: refs.map(r => r.join(' ')).join('\n'), ...extra },
});

test('per ref: a docs-only dev tip pushed with an Edge-changing self/x is not red', (t) => {
  const { git, commit, dir } = repo(t);
  const base = commit({ 'README.md': 'x' }, 'base');
  const selfTip = commit({ 'sdk/x.json': '1' }, 'feat: edge input');
  git('checkout', '--quiet', '-b', 'side', base);
  const devTip = commit({ 'docs/a.md': 'y' }, 'docs: a');
  git('checkout', '--quiet', 'dev');
  git('reset', '--quiet', '--hard', devTip);
  const rows = check(dir, [['refs/heads/dev', devTip, base], ['refs/heads/self/x', selfTip, base]]);
  assert.deepEqual(rows, []);
});

test('per ref: dev and main pushed together are judged on their own ranges', (t) => {
  const { git, commit, dir } = repo(t);
  const base = commit({ 'README.md': 'x' }, 'base');
  const edgeChange = commit({ 'sdk/x.json': '1' }, 'feat: edge input');
  const tip = commit({ 'docs/a.md': 'y' }, 'docs: a');
  const rows = check(dir, [['refs/heads/dev', tip, edgeChange], ['refs/heads/main', tip, base]]);
  assert.equal(git('rev-parse', 'HEAD'), tip);
  assert.equal(only(rows).level, 'REFUSED');
  assert.match(rows[0].why, /for this push to main:/);
});

test('per ref: a declaration anywhere in a range over 1 MiB of messages is found; a 0x1e byte in the tip is harmless', (t) => {
  const { commit, dir } = repo(t);
  const base = commit({ 'README.md': 'x' }, 'base');
  const filler = 'filler line of a long commit body\n'.repeat(18000);
  commit({ 'sdk/x.json': '1' }, `feat: schema field\n\nRolls alpha on purpose. [Edge:Roll]\n${filler}`);
  commit({}, `chore: middle\n\n${filler}`);
  const tip = commit({}, `fix: tip \x1e with a separator byte\n\n${filler}`);
  assert.ok(filler.length * 3 > 1024 * 1024);
  const r = only(check(dir, [['refs/heads/dev', tip, base]]));
  assert.equal(r.level, 'ok', r.why);
  assert.match(r.why, /declared by \[edge:roll\] in [0-9a-f]{12}$/);
});

test('per ref: an unknown destination tip, or a push that creates dev, cannot be decided and is red', (t) => {
  const { commit, dir } = repo(t);
  const tip = commit({ 'docs/a.md': 'x' }, 'docs');
  const unknown = only(check(dir, [['refs/heads/dev', tip, '1'.repeat(40)]]));
  assert.equal(unknown.level, 'REFUSED');
  assert.match(unknown.why, /the destination tip 111111111111 is not in this clone/);
  assert.equal(unknown.cure, 'git fetch origin dev, then push again');
  assert.match(only(check(dir, [['refs/heads/dev', tip, '0'.repeat(40)]])).why, /this push creates dev/);
});

test('per ref: a pushed tip other than HEAD, or a build manifest that differs from it, is red', (t) => {
  const { git, commit, dir } = repo(t);
  const base = commit({ 'README.md': 'x', 'e/build-manifest.json': '{}' }, 'base');
  const tip = commit({ 'docs/a.md': 'y' }, 'docs');
  git('checkout', '--quiet', base);
  assert.match(only(check(dir, [['refs/heads/dev', tip, base]])).why, /is not this checkout's HEAD/);
  git('checkout', '--quiet', 'dev');
  writeFileSync(join(dir, 'e/build-manifest.json'), '{"changed":true}');
  const drift = only(check(dir, [['refs/heads/dev', tip, base]]));
  assert.equal(drift.level, 'REFUSED');
  assert.match(drift.why, /build-manifest.json differs between this checkout and the pushed tip \(e\/build-manifest.json\)/);
  assert.equal(drift.cure, 'push from a checkout at the pushed commit, with its build-manifest.json files committed');
  git('checkout', '--quiet', '--', 'e/build-manifest.json');
  mkdirSync(join(dir, 'n'), { recursive: true });
  writeFileSync(join(dir, 'n/build-manifest.json'), '{}');
  assert.match(only(check(dir, [['refs/heads/dev', tip, base]])).why, /\(n\/build-manifest.json\)/);
});

test('per ref: a new branch that does not deploy stays advisory even when its base cannot be read', (t) => {
  const { commit, dir } = repo(t);
  const tip = commit({ 'sdk/x.json': '1' }, 'feat');
  const r = only(check(dir, [['refs/heads/self/new', tip, '0'.repeat(40)]]));
  assert.equal(r.level, 'WARN');
});

// ── Ambient inputs and failures ─────────────────────────────────────────────

test('ambient: an empty or stale ref list that disagrees with the targets is red on a protected push', (t) => {
  const { commit, dir } = repo(t);
  const tip = commit({ 'docs/a.md': 'x' }, 'docs');
  const policy = { mode: 'refuse', why: null };
  const empty = only(checkFleetRoll({ root: dir, walk: fwalk, spawn: spawnIn, policy, env: { GATE_PUSH_TARGETS: 'refs/heads/dev', GATE_PUSH_REFS: '' } }));
  assert.equal(empty.level, 'REFUSED');
  assert.match(empty.why, /does not match — the check cannot decide/);
  const stale = only(checkFleetRoll({ root: dir, walk: fwalk, spawn: spawnIn, policy,
    env: { GATE_PUSH_TARGETS: 'refs/heads/dev', GATE_PUSH_REFS: `refs/heads/self/old ${tip} ${tip}` } }));
  assert.equal(stale.level, 'REFUSED');
  const selfOnly = only(checkFleetRoll({ root: dir, walk: fwalk, spawn: spawnIn, policy, env: { GATE_PUSH_TARGETS: 'refs/heads/self/a', GATE_PUSH_REFS: '' } }));
  assert.equal(selfOnly.level, 'WARN');
});

test('an evaluator failure is red on a protected ref under refuse, a WARN otherwise, and never throws', (t) => {
  const { commit, dir } = repo(t);
  const base = commit({ 'README.md': 'x' }, 'base');
  const tip = commit({ 'sdk/x.json': '1' }, 'feat');
  const boom = () => { throw new SyntaxError('Unexpected token } in JSON at position 12'); };
  const run = (mode, target) => only(checkFleetRoll({ root: dir, walk: boom, spawn: spawnIn, policy: { mode, why: null },
    env: { GATE_PUSH_TARGETS: target, GATE_PUSH_REFS: `${target} ${tip} ${base}` } }));
  const red = run('refuse', 'refs/heads/dev');
  assert.equal(red.level, 'REFUSED');
  assert.match(red.why, /^the fleet-roll check could not decide: Unexpected token \} in JSON at position 12$/);
  assert.match(red.cure, /build-graph.groovy:815-827/);
  assert.equal(run('warn', 'refs/heads/dev').level, 'WARN');
  assert.equal(run('refuse', 'refs/heads/self/a').level, 'WARN');
  assert.deepEqual(failureRows({ error: new Error('x'), deploying: true, mode: 'off' }), []);
});

test('a build-manifest.json that does not parse is named in the row', (t) => {
  const dir = mkdtempSync(join(tmpdir(), 'fleet-roll-bad-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  mkdirSync(join(dir, 'x'));
  writeFileSync(join(dir, 'x/build-manifest.json'), '{ "pipeline": ');
  const r = only(checkFleetRoll({ root: dir, env: {}, changed: ['a.txt'] }));
  assert.equal(r.level, 'WARN');
  assert.match(r.why, /^the fleet-roll check could not decide: \.\/x\/build-manifest\.json: /);
  assert.match(r.why, /\(policy: mode refuse, the default, because pool-policy.json could not be read \(ENOENT\)\)$/);
});

test('older hook (no ref list): silent with no Edge build; red with one, naming the launcher check first', () => {
  const env = { GATE_PUSH_TARGETS: 'refs/heads/dev' };
  const policy = { mode: 'refuse', why: null };
  assert.deepEqual(checkFleetRoll({ env, changed: ['steps/a.ts', 'app/x.ts'], walk: fwalk, policy }), []);
  const r = only(checkFleetRoll({ env, changed: ['pin'], walk: fwalk, policy }));
  assert.equal(r.level, 'REFUSED');
  assert.match(r.why, /hands over no ref list \(GATE_PUSH_REFS\), so neither this ref's own range nor a declaration can be read$/);
  assert.ok(r.cure.indexOf('test -x <worktree>/.husky/_/pre-push') < r.cure.indexOf('-c core.hooksPath=<worktree>/.husky/_'), r.cure);
  assert.equal(only(checkFleetRoll({ env, changed: null, walk: fwalk, policy })).level, 'REFUSED');
  assert.equal(only(checkFleetRoll({ env: { GATE_PUSH_TARGETS: 'refs/heads/self/a' }, changed: ['pin'], walk: fwalk, policy })).level, 'WARN');
});

test('the done flag counts only for this process or an ancestor', () => {
  assert.equal(doneFlagHonoured({ GATE_FLEET_ROLL_DONE: String(process.pid) }), true);
  assert.equal(doneFlagHonoured({ GATE_FLEET_ROLL_DONE: String(process.ppid) }), true);
  assert.equal(doneFlagHonoured({ GATE_FLEET_ROLL_DONE: '1' }), false);
  assert.equal(doneFlagHonoured({ GATE_FLEET_ROLL_DONE: 'yes' }), false);
  assert.equal(doneFlagHonoured({}), false);
  const chain = { [process.pid]: `${process.pid} (node (x) y) S 4242 0`, 4242: '4242 (bash) S 1 0' };
  assert.equal(doneFlagHonoured({ GATE_FLEET_ROLL_DONE: '4242' }, pid => chain[pid]), true);
  assert.equal(doneFlagHonoured({ GATE_FLEET_ROLL_DONE: '777' }, pid => chain[pid]), false);
});

test('policy fixtures: refuse, warn, off; and refuse with its reason for missing key, unknown mode, malformed JSON, unreadable file', (t) => {
  const dir = mkdtempSync(join(tmpdir(), 'fleet-roll-policy-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  assert.match(fleetRollMode(dir).why, /could not be read \(ENOENT\)/);
  mkdirSync(join(dir, 'genesis/agentic'), { recursive: true });
  const put = text => writeFileSync(join(dir, 'genesis/agentic/pool-policy.json'), text);
  for (const mode of ['refuse', 'warn', 'off']) { put(JSON.stringify({ fleet_roll_guard: { mode } })); assert.deepEqual(fleetRollMode(dir), { mode, why: null }); }
  put(JSON.stringify({ gate_on_red: { mode: 'continue' } }));
  assert.deepEqual(fleetRollMode(dir), { mode: 'refuse', why: 'pool-policy.json declares no fleet_roll_guard.mode' });
  put(JSON.stringify({ fleet_roll_guard: { mode: 'sometimes' } }));
  assert.match(fleetRollMode(dir).why, /"sometimes" is not refuse, warn or off/);
  put('{ not json');
  assert.equal(fleetRollMode(dir).why, 'pool-policy.json is not valid JSON');
  rmSync(join(dir, 'genesis/agentic/pool-policy.json'));
  mkdirSync(join(dir, 'genesis/agentic/pool-policy.json'));
  assert.match(fleetRollMode(dir).why, /could not be read \(EISDIR\)/);
});

test('GATE_PUSH_REFS lines parse into ref, pushed sha, destination sha and branch', () => {
  assert.deepEqual(parseRefs('refs/heads/dev abc def\nrefs/tags/v1 123 000\n'), [
    { ref: 'refs/heads/dev', local: 'abc', remote: 'def', branch: 'dev' },
    { ref: 'refs/tags/v1', local: '123', remote: '000', branch: null },
  ]);
});

test('an ambient GATE_ROOT does not make the CLI judge another checkout', (t) => {
  const dir = mkdtempSync(join(tmpdir(), 'fleet-roll-root-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  writeFileSync(join(dir, 'changed.txt'), 'elohim/sdk/schemas/v1/gate-live-event.schema.json\n');
  const env = { ...cleanEnv, GATE_ROOT: dir, GATE_CHANGED_FILES: join(dir, 'changed.txt'), GATE_LIVE_RUN: '' };
  delete env.GATE_PUSH_TARGETS; delete env.GATE_PUSH_REFS;
  const r = spawnSync('node', [join(ROOT, 'genesis/orchestrator/gate-preflight.mjs'), '--fleet-roll'], { cwd: ROOT, env, encoding: 'utf8' });
  assert.equal(r.status, 0, r.stderr);
  assert.match(r.stdout, /WARN fleet-roll: .*cargo-build-storage/);
});

// ── Real manifests: the incident witnesses and the steward's named paths ────

const real = ciWalker(ROOT);
const realOnDev = changed => evaluateRef({ target: 'dev', walked: real(changed), tip: 'feat: x', mode: 'refuse' });
const redFor = changed => realOnDev(changed).find(r => r.check === 'fleet-roll');

test('real manifests: the conductor pin alone selects Edge through the conductor image', () => {
  const p = predict(real(['elohim/holochain-conductor']), 'feat: pin');
  assert.equal(p.selects, true);
  for (const step of ['cargo-build-storage', 'build-edge-image', 'deploy-manifests']) assert.ok(p.buildSteps.some(s => s.name === step), step);
  const r = redFor(['elohim/holochain-conductor']);
  assert.equal(r.level, 'REFUSED');
  assert.ok(r.why.includes('elohim-conductor:build-conductor-image'), r.why);
});

test('real manifests: the tracked elohim/sdk/schemas/.gitignore alone selects cargo-build-storage', () => {
  assert.ok(predict(real(['elohim/sdk/schemas/.gitignore'])).buildSteps.some(s => s.name === 'cargo-build-storage'));
  assert.equal(redFor(['elohim/sdk/schemas/.gitignore']).level, 'REFUSED');
});

test('real manifests: the 2026-11-19 schema edit and the steward\'s named paths each select an Edge build', () => {
  for (const path of ['elohim/sdk/schemas/v1/gate-live-event.schema.json', 'elohim/eprfs/device-runtime/src/lib.rs', 'elohim/Cargo.lock']) {
    const p = predict(real([path]));
    assert.equal(p.selects, true, path);
    assert.equal(redFor([path]).level, 'REFUSED', path);
  }
  assert.ok(predict(real(['elohim/sdk/schemas/v1/gate-live-event.schema.json'])).buildSteps.some(s => s.name === 'cargo-build-storage'));
});
