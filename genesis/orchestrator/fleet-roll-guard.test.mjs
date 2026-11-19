// fleet-roll-guard.mjs: a pushed dev or main ref for which CI would select an Edge build and its
// deploy step is red unless a pushed commit carries `[edge:roll]`. Pure cases run CI's graph
// (graph-walker.mjs walkGraphCi) over fixture manifests; push cases use real git repositories whose
// manifests and policy are read from git objects, never the working tree. Habit
// `careless-reds-cost-seconds`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ciGlobMatch, walkGraph } from './graph-walker.mjs';
import { checkFleetRoll, evaluatePaths, evaluateRef, fleetRollMarkHonoured, fleetRollMode, parseRefs, predict } from './fleet-roll-guard.mjs';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));

// ── Pure: CI's graph over fixture manifests ─────────────────────────────────

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
const fwalk = changed => walkGraph(FIXTURE, changed, { ci: true });
const onDev = (changed, tip = 'feat: x', over = {}) => evaluateRef({ target: 'dev', walked: fwalk(changed), tip, mode: 'refuse', ...over });
const only = rows => { assert.equal(rows.length, 1, JSON.stringify(rows)); return rows[0]; };
const BASELINE = /once the run ends green, the next run's diff starts past this push; a later change that selects the same pipeline still builds these bytes; a failed or aborted run keeps the old baseline/;

test('the default walker is unchanged: no propagation, picomatch dotfile rules', () => {
  const withDepends = FIXTURE.map(m => ({ ...m, content: { ...m.content, steps: Object.fromEntries(Object.entries(m.content.steps).map(([n, s]) => [n, { depends: [], ...s }])) } }));
  assert.deepEqual(walkGraph(withDepends, ['pin']).pipelines.map(p => p.name), ['elohim-conductor']);
  assert.deepEqual(walkGraph(withDepends, ['sdk/.gitignore']).pipelines, []);
});

test('ci: an omitted depends is [] throughout, with no call into the path walker\'s sorter', () => {
  assert.deepEqual(fwalk(['pin']).pipelines.map(p => p.name), ['elohim-conductor', 'elohim-edge']);
});

test('ci: CI glob matches dotfiles, keeps segment wildcards, and skips .epr-meta', () => {
  for (const [file, pattern, hit] of [
    ['sdk/.gitignore', 'sdk/**', true], ['a/b.c', 'a/*.c', true], ['a/x/b.c', 'a/*.c', false],
    ['a/b', 'a/**/b', true], ['a/x/y/b', 'a/**/b', true], ['ab', 'a?', true], ['aXc', 'a.c', false],
  ]) assert.equal(ciGlobMatch(file, pattern), hit, `${file} ~ ${pattern}`);
  assert.deepEqual(fwalk(['sdk/.gitignore']).pipelines.map(p => p.name), ['elohim-edge']);
  assert.deepEqual(fwalk(['sdk/.epr-meta/rule.md']).pipelines, []);
});

test('ci: staleness propagates to a fixed point even when manifests list dependents first', () => {
  const reversed = [
    { path: './z/build-manifest.json', content: { pipeline: 'z', jenkinsPath: 'z', steps: { last: { depends: ['y:middle'] } } } },
    { path: './y/build-manifest.json', content: { pipeline: 'y', jenkinsPath: 'y', steps: { middle: { depends: ['x:first'] } } } },
    { path: './x/build-manifest.json', content: { pipeline: 'x', jenkinsPath: 'x', steps: { first: { inputs: { sources: ['src/**'] } } } } },
  ];
  const walked = walkGraph(reversed, ['src/a'], { ci: true });
  assert.deepEqual(walked.pipelines.map(p => [p.name, p.steps[0].reason, p.steps[0].cause]).sort(), [
    ['x', 'source: src/a matches src/**', 'src/a'], ['y', 'depends: x:first', 'src/a'], ['z', 'depends: y:middle', 'src/a'],
  ]);
});

test('ci: a manual-only step propagates but is not dispatched', () => {
  const walked = fwalk(['rk']);
  assert.deepEqual(walked.pipelines.map(p => p.name), ['elohim-edge']);
  assert.deepEqual(walked.pipelines[0].steps.map(s => s.name), ['build-edge-image']);
});

test('ci: the manifest-only cap fails for a source-stale step that also has a stale dependency', () => {
  assert.equal(fwalk(['a/build-manifest.json']).pipelines.find(p => p.name === 'elohim').runClass, 'verify');
  // cargo-build-storage is stale by its own manifest (a source) AND has a stale dependency.
  const edge = fwalk(['e/build-manifest.json', 'pin']).pipelines.find(p => p.name === 'elohim-edge');
  assert.equal(edge.steps.find(s => s.name === 'cargo-build-storage').reason, 'source: e/build-manifest.json matches e/build-manifest.json');
  assert.equal(edge.runClass, 'build');
});

test('ci: a build-process reference CI cannot read, or a function it cannot find, makes the step stale', () => {
  const m = [{ path: './p/build-manifest.json', content: { pipeline: 'p', jenkinsPath: 'p', steps: {
    a: { inputs: { sources: [], buildProcess: ['scripts/missing.sh'] } },
    b: { inputs: { sources: [], buildProcess: ['ci/Jenkinsfile@deployAll'] } },
    c: { inputs: { sources: [], buildProcess: ['ci/Jenkinsfile@buildAll'] } },
  } } }];
  const files = { 'ci/Jenkinsfile': 'def buildAll(x) {\n  sh "make"\n}\n' };
  const walked = walkGraph(m, ['README.md'], { ci: true, readFile: path => files[path] ?? null });
  assert.deepEqual(walked.pipelines[0].steps.map(s => [s.name, s.reason]), [
    ['a', 'buildProcess: cannot read scripts/missing.sh'],
    ['b', 'buildProcess: function deployAll not found in ci/Jenkinsfile'],
  ]);
});

test('ci: structural errors are returned as CI reports them, and no result is invented', () => {
  const errorsOf = manifests => walkGraph(manifests, ['src/a'], { ci: true }).errors;
  const step = (depends = []) => ({ inputs: { sources: ['src/**'] }, depends });
  assert.match(errorsOf([{ path: './a/b.json', content: { pipeline: 'p', steps: { s: step() } } }, { path: './b/b.json', content: { pipeline: 'p', steps: { t: step() } } }])[0],
    /^Duplicate pipeline name 'p' in \.\/b\/b\.json and \.\/a\/b\.json$/);
  assert.match(errorsOf([{ path: './a/b.json', content: { pipeline: 'p', steps: { s: step(['missing']) } } }])[0], /^Step 'p:s' depends on 'p:missing' which does not exist\./);
  assert.equal(errorsOf([{ path: './a/b.json', content: { pipeline: 'p', steps: { s: step(['s']) } } }])[0], 'Dependency cycle detected: p:s -> p:s');
  assert.equal(errorsOf([{ path: './a/b.json', content: { pipeline: 'p', steps: { s: step(['t']), t: step(['s']) } } }])[0], 'Dependency cycle detected: p:s -> p:t -> p:s');
  assert.deepEqual(errorsOf(FIXTURE), []);
});

test('invalid graph: red on a protected ref under refuse, a WARN otherwise', () => {
  const walked = { pipelines: [], meta: {}, errors: ['Dependency cycle detected: p:s -> p:s'] };
  const red = only(evaluateRef({ target: 'dev', walked, mode: 'refuse' }));
  assert.equal(red.level, 'REFUSED');
  assert.equal(red.why, 'the build graph at the pushed commit is not valid: Dependency cycle detected: p:s -> p:s; CI stops there too (build-graph.groovy:815-827), so no selection is predicted');
  assert.equal(only(evaluateRef({ target: 'dev', walked, mode: 'warn' })).level, 'WARN');
  assert.equal(only(evaluateRef({ target: 'self/x', walked, mode: 'refuse' })).level, 'WARN');
});

// ── Pure: outcomes and tags ─────────────────────────────────────────────────

test('parity: propagation turns a push the path walker reads as validation only into a build', () => {
  const r = only(onDev(['steps/a.ts', 'pin']));
  assert.equal(r.level, 'REFUSED');
  assert.match(r.why, /^CI would select an Edge build and its deploy step for this push to dev: elohim-edge stale steps cargo-build-storage, build-edge-image, from pin, through elohim-conductor:build-conductor-image/);
});

test('[deploy-only] on the tip always asks for the declaration, even over a validation-only Edge', () => {
  for (const changed of [['steps/a.ts'], ['docs/a.md'], ['sdk/x.json']]) {
    const r = only(onDev(changed, 'ops: redeploy [deploy-only]'));
    assert.equal(r.level, 'REFUSED', changed[0]);
    assert.match(r.why, /^the tip's \[deploy-only\] asks CI for the Edge deploy step on this push to dev/, changed[0]);
    assert.match(r.why, /already-built filter/, changed[0]);
  }
});

test('[build:edge] with [deploy-only] says what CI does: it forwards deploy-only and Edge skips its build stages', () => {
  const r = only(onDev(['docs/a.md'], 'ops: [build:edge] [deploy-only]'));
  assert.match(r.why, /^the tip's \[build:edge\] and \[deploy-only\] make CI dispatch elohim-edge with DEPLOY_ONLY: Edge skips its build stages and runs its deploy step on this push to dev/);
});

test('a forcing tag over path-selected Edge inputs: the cure names both', () => {
  const r = only(onDev(['sdk/x.json'], 'ci: [build:edge]'));
  assert.match(r.cure, /^drop \[build:edge\] from the tip commit and take the Edge build inputs out of this push \(dropping the tag alone leaves the selection by path\)/);
  assert.match(only(onDev(['docs/a.md'], 'ci: [build:edge]')).cure, /^drop \[build:edge\] from the tip commit; or declare/);
});

test('[build:all] on a docs tip is asked to be declared, saying CI forces nothing with it today', () => {
  const r = only(onDev(['docs/a.md'], 'docs: refresh [build:all]'));
  assert.equal(r.level, 'REFUSED');
  assert.match(r.why, /^the tip's \[build:all\] asks for every pipeline, Edge included, on this push to dev: CI forces nothing with it today/);
});

test('an explicit edge token forces the selection; a space inside the tag breaks both grammars', () => {
  for (const tip of ['ci: [build:edge]', 'ci: [BUILD:dna,Edge]']) assert.equal(only(onDev(['docs/a.md'], tip)).level, 'REFUSED', tip);
  assert.deepEqual(onDev(['docs/a.md'], 'ci: [build:dna, edge]'), []);
});

test('an Edge without a jenkinsPath is not dispatched, as in CI', () => {
  const blank = FIXTURE.map(m => (m.content.pipeline === 'elohim-edge' ? { ...m, content: { ...m.content, jenkinsPath: ' ' } } : m));
  assert.deepEqual(evaluateRef({ target: 'dev', walked: walkGraph(blank, ['sdk/x.json'], { ci: true }), tip: 'ci: [build:edge]', mode: 'refuse' }), []);
});

test('an undeclared selection on dev is red, with cures that are true; the declaration is a tag, its reason unchecked', () => {
  const r = only(onDev(['sdk/x.json']));
  assert.equal(r.level, 'REFUSED');
  assert.match(r.why, /no pushed commit carries \[edge:roll\]/);
  assert.doesNotMatch(r.why, /rebuild and roll/);
  assert.match(r.cure, /take the Edge build inputs out of this push, or land them with the next deliberate roll; or declare the selection: add \[edge:roll\] to a pushed commit message, with the reason \(asked for, not checked\)/);
  assert.match(r.cure, /once the run ends green the next run's diff starts past this push/);
  assert.doesNotMatch(r.cure, /own push|documentation only|moves the baseline/);
  assert.equal(only(onDev(['sdk/x.json'], 'feat: x', { declaredBy: 'abcdef0123456789' })).level, 'ok');
});

test('validation-only selection is ok; no Edge selection is silent', () => {
  assert.match(only(onDev(['steps/a.ts'])).why, /validates only/);
  assert.deepEqual(onDev(['app/x.ts']), []);
});

test('mode warn gives the red text as WARN and says so; off gives nothing; a default mode says why', () => {
  const red = only(onDev(['sdk/x.json']));
  const warn = only(onDev(['sdk/x.json'], 'feat: x', { mode: 'warn', policySource: "dev's genesis/agentic/pool-policy.json" }));
  assert.equal(warn.level, 'WARN');
  assert.equal(warn.why, `${red.why} (policy: mode warn, declared in dev's genesis/agentic/pool-policy.json)`);
  assert.deepEqual(onDev(['sdk/x.json'], 'feat: x', { mode: 'off' }), []);
  assert.match(only(onDev(['sdk/x.json'], 'feat: x', { policyWhy: 'dev has no genesis/agentic/pool-policy.json' })).why,
    /\(policy: mode refuse, the default, because dev has no genesis\/agentic\/pool-policy.json\)$/);
});

test('off a deploying branch, or outside a push, a selection warns and never goes red', () => {
  assert.match(only(evaluateRef({ target: 'self/x', walked: fwalk(['sdk/x.json']), mode: 'refuse' })).why, /were it pushed to dev \(target self\/x does not deploy\)/);
  assert.equal(only(evaluateRef({ target: null, walked: fwalk(['sdk/x.json']), mode: 'refuse' })).level, 'WARN');
});

test('a skip tag over Edge build inputs warns twice with the true baseline words', () => {
  const rows = onDev(['sdk/x.json', 'app/y.ts', 'local/z', 'o/Jenkinsfile'], 'docs: notes [skip ci]');
  assert.deepEqual(rows.map(r => [r.check, r.level]), [['fleet-roll', 'WARN'], ['skip-ci', 'WARN']]);
  for (const r of rows) assert.match(r.why, BASELINE);
  assert.match(rows[1].why, /^the tip's \[skip ci\] dispatches none of elohim-edge, elohim, which this ref's paths select/);
  assert.deepEqual(onDev(['docs/a.md'], 'docs [skip ci]'), []);
});

test('[edge:validate-only] warns over Edge build inputs alone and over other pipelines; over validation alone it is ok', () => {
  assert.equal(only(onDev(['sdk/x.json'], 'measure [edge:validate-only]')).level, 'WARN');
  assert.match(only(onDev(['app/x.ts'], 'measure [edge:validate-only]')).why, /it drops elohim from the run/);
  assert.equal(only(onDev(['steps/a.ts'], 'measure [edge:validate-only]')).level, 'ok');
});

test('[run:verify] keeps Edge out unless forced', () => {
  assert.match(only(onDev(['sdk/x.json'], '[run:verify]')).why, /keeps elohim-edge out of the dispatch/);
  assert.equal(only(onDev(['sdk/x.json'], '[run:verify] [build:edge]')).level, 'REFUSED');
  assert.equal(predict(fwalk(['docs/a.md']), 'feat: tip').selects, false);
});

// ── Pushes, read from git objects ───────────────────────────────────────────

const cleanEnv = Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith('GIT_') || k.startsWith('GIT_CONFIG_')));
const spawnIn = (cmd, args, opts) => spawnSync(cmd, args, { ...opts, env: cleanEnv });
const json = value => `${JSON.stringify(value, null, 2)}\n`;
const MANIFESTS = {
  'genesis/agentic/pool-policy.json': json({ fleet_roll_guard: { mode: 'refuse' } }),
  'c/build-manifest.json': json({ pipeline: 'elohim-conductor', jenkinsPath: 'c/Jenkinsfile', steps: { 'build-conductor-image': { inputs: { sources: ['pin'] }, depends: [] } } }),
  'e/build-manifest.json': json({ pipeline: 'elohim-edge', jenkinsPath: 'e/Jenkinsfile', steps: {
    'cargo-build-storage': { inputs: { sources: ['sdk/**'] }, depends: ['elohim-conductor:build-conductor-image', 'elohim-rakia:rakia-ci'] },
    'build-edge-image': { inputs: { sources: [] }, depends: ['cargo-build-storage'] },
    'dataplane-validation': { inputs: { sources: ['steps/**'] }, depends: [] },
  } }),
  'a/build-manifest.json': json({ pipeline: 'elohim', jenkinsPath: 'Jenkinsfile', steps: { 'build-app': { inputs: { sources: ['app/**'] }, depends: [] } } }),
  'README.md': 'x\n',
};

function repo(t) {
  const dir = mkdtempSync(join(tmpdir(), 'fleet-roll-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const git = (...args) => {
    const r = spawnSync('git', ['-C', dir, '-c', 'user.name=t', '-c', 'user.email=t@t', '-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=/dev/null', ...args],
      { encoding: 'utf8', env: cleanEnv, maxBuffer: 64 * 1024 * 1024 });
    assert.equal(r.status, 0, `git ${args.join(' ')}: ${r.stderr}`);
    return r.stdout.trim();
  };
  const write = files => {
    for (const [path, text] of Object.entries(files)) {
      if (text === null) { rmSync(join(dir, path), { force: true }); continue; }
      mkdirSync(join(dir, path, '..'), { recursive: true });
      writeFileSync(join(dir, path), text);
    }
  };
  const commit = (files, message, { gitlinks = {} } = {}) => {
    write(files);
    git('add', '-A');
    for (const [path, sha] of Object.entries(gitlinks)) git('update-index', '--add', '--cacheinfo', `160000,${sha},${path}`);
    const msg = `${dir}.msg`;
    writeFileSync(msg, message);
    t.after(() => rmSync(msg, { force: true }));
    git('commit', '--quiet', '--allow-empty', '-F', msg);
    return git('rev-parse', 'HEAD');
  };
  git('init', '--quiet', '-b', 'dev');
  return { dir, git, commit, write };
}
const push = (dir, refs, extra = {}) => checkFleetRoll({
  root: dir, spawn: spawnIn,
  env: { GATE_PUSH_TARGETS: refs.map(r => r[0]).join(' '), GATE_PUSH_REFS: refs.map(r => r.join(' ')).join('\n'), ...extra },
});

test('seen failing first: a manifest-only edit that adds an unreadable Edge build-process reference selects Edge', (t) => {
  const { dir, commit } = repo(t);
  const base = commit(MANIFESTS, 'base');
  const edge = JSON.parse(MANIFESTS['e/build-manifest.json']);
  edge.steps['cargo-build-storage'].inputs.buildProcess = ['scripts/missing.sh'];
  const tip = commit({ 'e/build-manifest.json': json(edge) }, 'ci: storage gate script');
  const r = only(push(dir, [['refs/heads/dev', tip, base]]));
  assert.equal(r.level, 'REFUSED', r.why);
  assert.match(r.why, /cargo-build-storage/);
});

test('seen failing first: a pin move gives the same verdict with and without the submodule checked out', (t) => {
  const { dir, commit, write } = repo(t);
  const base = commit(MANIFESTS, 'base', { gitlinks: { 'elohim/rakia': '1'.repeat(40) } });
  const tip = commit({}, 'chore(rakia): bump pin', { gitlinks: { 'elohim/rakia': '2'.repeat(40) } });
  const without = push(dir, [['refs/heads/dev', tip, base]]);
  write({ 'elohim/rakia/build-manifest.json': json({ pipeline: 'elohim-rakia', manualOnly: true, steps: { 'rakia-ci': { inputs: { sources: ['elohim/rakia', 'elohim/rakia/**'] }, depends: [] } } }) });
  const withCheckout = push(dir, [['refs/heads/dev', tip, base]]);
  assert.deepEqual(withCheckout.map(r => [r.check, r.level]), without.map(r => [r.check, r.level]));
  assert.equal(without.some(r => r.level === 'REFUSED'), false, JSON.stringify(without));
  const warn = only(without);
  assert.equal(warn.level, 'WARN');
  assert.match(warn.why, /^this push moves the elohim\/rakia pin, whose submodule carries pipeline elohim-rakia \(by its name\); elohim-edge's build steps depend on it, but CI's checkout initialises no submodule/);
  assert.match(warn.why, /if CI ever checked the submodule out, this push would select cargo-build-storage, build-edge-image$/);
  assert.match(only(withCheckout).why, /carries pipeline elohim-rakia \(read from its checkout\)/);
});

test('seen failing first: two protected refs at different tips are each judged from their own objects', (t) => {
  const { dir, git, commit } = repo(t);
  const base = commit(MANIFESTS, 'base');
  const devTip = commit({ 'docs/a.md': 'a\n' }, 'docs: a');
  git('checkout', '--quiet', '-b', 'm', base);
  const mainTip = commit({ 'docs/b.md': 'b\n' }, 'docs: b');
  git('checkout', '--quiet', 'dev');
  assert.deepEqual(push(dir, [['refs/heads/dev', devTip, base], ['refs/heads/main', mainTip, base]]), []);
  git('checkout', '--quiet', base);
  assert.deepEqual(push(dir, [['refs/heads/dev', devTip, base]]), [], 'HEAD~1:dev style pushes are judged too');
});

test('a poisoned working-tree manifest or an uncommitted policy does not move the verdict', (t) => {
  const { dir, commit, write } = repo(t);
  const base = commit(MANIFESTS, 'base');
  const tip = commit({ 'sdk/x.json': '1\n' }, 'feat: edge input');
  const clean = only(push(dir, [['refs/heads/dev', tip, base]]));
  write({ 'e/build-manifest.json': json({ pipeline: 'elohim-edge', steps: {} }), 'genesis/agentic/pool-policy.json': json({ fleet_roll_guard: { mode: 'off' } }) });
  const poisoned = only(push(dir, [['refs/heads/dev', tip, base]]));
  assert.equal(poisoned.level, 'REFUSED');
  assert.equal(poisoned.why, clean.why);
});

test('the policy in force is the destination tip\'s; a push cannot switch off its own check', (t) => {
  const { dir, commit } = repo(t);
  const base = commit({ ...MANIFESTS, 'genesis/agentic/pool-policy.json': json({ fleet_roll_guard: { mode: 'warn' } }) }, 'base');
  const tip = commit({ 'sdk/x.json': '1\n', 'genesis/agentic/pool-policy.json': json({ fleet_roll_guard: { mode: 'off' } }) }, 'feat: and switch off');
  const r = only(push(dir, [['refs/heads/dev', tip, base]]));
  assert.equal(r.level, 'WARN');
  assert.match(r.why, /\(policy: mode warn, declared in dev's genesis\/agentic\/pool-policy\.json at [0-9a-f]{12}\)$/);
  const { dir: d2, commit: c2 } = repo(t);
  const { 'genesis/agentic/pool-policy.json': _, ...noPolicy } = MANIFESTS;
  const b2 = c2(noPolicy, 'base');
  const t2 = c2({ 'sdk/x.json': '1\n' }, 'feat');
  assert.match(only(push(d2, [['refs/heads/dev', t2, b2]])).why, /\(policy: mode refuse, the default, because dev's tip [0-9a-f]{12} has no genesis\/agentic\/pool-policy\.json\)$/);
});

test('a symlink named build-manifest.json in the pushed tree cannot be decided', (t) => {
  const { dir, commit } = repo(t);
  const base = commit(MANIFESTS, 'base');
  symlinkSync('../e/build-manifest.json', join(dir, 'link-build-manifest.json'));
  mkdirSync(join(dir, 'x'));
  symlinkSync('../e/build-manifest.json', join(dir, 'x/build-manifest.json'));
  const tip = commit({ 'docs/a.md': 'a\n' }, 'docs');
  const r = only(push(dir, [['refs/heads/dev', tip, base]]));
  assert.equal(r.level, 'REFUSED');
  assert.match(r.why, /x\/build-manifest\.json is a symlink in the pushed tree/);
});

test('a declaration anywhere in a range over 1 MiB of messages is found; a 0x1e byte in the tip is harmless', (t) => {
  const { dir, commit } = repo(t);
  const base = commit(MANIFESTS, 'base');
  const filler = 'filler line of a long commit body\n'.repeat(18000);
  commit({ 'sdk/x.json': '1' }, `feat: schema field\n\nRolls alpha on purpose. [Edge:Roll]\n${filler}`);
  commit({}, `chore: middle\n\n${filler}`);
  const tip = commit({}, `fix: tip \x1e with a separator byte\n\n${filler}`);
  assert.ok(filler.length * 3 > 1024 * 1024);
  assert.match(only(push(dir, [['refs/heads/dev', tip, base]])).why, /declared by \[edge:roll\] in [0-9a-f]{12}$/);
});

test('an unknown destination tip names the hook\'s remote; a new protected branch cannot be decided', (t) => {
  const { dir, commit } = repo(t);
  const tip = commit(MANIFESTS, 'base');
  const unknown = only(push(dir, [['refs/heads/dev', tip, '1'.repeat(40)]], { GATE_PUSH_REMOTE: 'forge' }));
  assert.equal(unknown.level, 'REFUSED');
  assert.equal(unknown.cure, 'git fetch forge dev, then push again');
  assert.match(only(push(dir, [['refs/heads/dev', tip, '0'.repeat(40)]])).why, /this push creates dev/);
});

test('a docs-only dev tip pushed with an Edge-changing self/x is not red; a new feature branch stays advisory', (t) => {
  const { dir, git, commit } = repo(t);
  const base = commit(MANIFESTS, 'base');
  const selfTip = commit({ 'sdk/x.json': '1' }, 'feat: edge input');
  git('checkout', '--quiet', '-b', 'side', base);
  const devTip = commit({ 'docs/a.md': 'y' }, 'docs: a');
  assert.deepEqual(push(dir, [['refs/heads/dev', devTip, base], ['refs/heads/self/x', selfTip, base]]), []);
  assert.equal(only(push(dir, [['refs/heads/self/new', selfTip, '0'.repeat(40)]])).level, 'WARN');
});

test('ambient: an empty or stale ref list that disagrees with the targets is red on a protected push', (t) => {
  const { dir, commit } = repo(t);
  const tip = commit(MANIFESTS, 'base');
  const run = env => only(checkFleetRoll({ root: dir, spawn: spawnIn, env }));
  assert.equal(run({ GATE_PUSH_TARGETS: 'refs/heads/dev', GATE_PUSH_REFS: '' }).level, 'REFUSED');
  assert.equal(run({ GATE_PUSH_TARGETS: 'refs/heads/dev', GATE_PUSH_REFS: `refs/heads/self/old ${tip} ${tip}` }).level, 'REFUSED');
  assert.equal(run({ GATE_PUSH_TARGETS: 'refs/heads/self/a', GATE_PUSH_REFS: '' }).level, 'WARN');
});

test('a build manifest that does not parse at the pushed commit is an invalid graph, named', (t) => {
  const { dir, commit } = repo(t);
  const base = commit(MANIFESTS, 'base');
  const tip = commit({ 'x/build-manifest.json': '{ "pipeline": ' }, 'broken');
  const r = only(push(dir, [['refs/heads/dev', tip, base]]));
  assert.equal(r.level, 'REFUSED');
  assert.match(r.why, /^the build graph at the pushed commit is not valid: \.\/x\/build-manifest\.json: /);
});

test('seen failing first: older hook (no ref list) reads HEAD\'s message, so a forcing tag on a docs tip is red', (t) => {
  const { dir, commit } = repo(t);
  commit(MANIFESTS, 'base');
  commit({ 'docs/a.md': 'a\n' }, 'docs: a [build:edge]');
  const r = only(checkFleetRoll({ root: dir, spawn: spawnIn, env: { GATE_PUSH_TARGETS: 'refs/heads/dev' }, changed: ['docs/a.md'] }));
  assert.equal(r.level, 'REFUSED');
  assert.match(r.why, /\[build:edge\]/);
  assert.ok(r.cure.indexOf('test -x <worktree>/.husky/_/pre-push') < r.cure.indexOf('-c core.hooksPath=<worktree>/.husky/_'), r.cure);
});

test('older hook: a path-selected Edge build is red; no Edge build is silent; no changed list is red', (t) => {
  const { dir, commit } = repo(t);
  commit(MANIFESTS, 'base');
  commit({ 'docs/a.md': 'a\n' }, 'docs: a');
  const run = (changed, targets = 'refs/heads/dev') => checkFleetRoll({ root: dir, spawn: spawnIn, env: { GATE_PUSH_TARGETS: targets }, changed });
  assert.equal(only(run(['sdk/x.json'])).level, 'REFUSED');
  assert.deepEqual(run(['docs/a.md', 'steps/a.ts']), []);
  assert.equal(only(run(null)).level, 'REFUSED');
  assert.equal(only(run(['sdk/x.json'], 'refs/heads/self/a')).level, 'WARN');
});

test('the done mark counts only when it names exactly the refs this hook judged', (t) => {
  const dir = mkdtempSync(join(tmpdir(), 'fleet-roll-mark-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const mark = join(dir, 'mark');
  const refs = `refs/heads/dev ${'b'.repeat(40)} ${'a'.repeat(40)}`;
  assert.equal(fleetRollMarkHonoured({ GATE_FLEET_ROLL_MARK: mark, GATE_PUSH_REFS: refs }), false, 'no file: not run');
  writeFileSync(mark, refs);
  assert.equal(fleetRollMarkHonoured({ GATE_FLEET_ROLL_MARK: mark, GATE_PUSH_REFS: refs }), true);
  assert.equal(fleetRollMarkHonoured({ GATE_FLEET_ROLL_MARK: mark, GATE_PUSH_REFS: refs.replace('b', 'c') }), false);
  assert.equal(fleetRollMarkHonoured({ GATE_FLEET_ROLL_MARK: mark, GATE_PUSH_REFS: '' }), false);
  assert.equal(fleetRollMarkHonoured({ GATE_FLEET_ROLL_DONE: String(process.ppid), GATE_PUSH_REFS: refs }), false, 'a pid proves nothing');
});

test('policy fixtures outside a push: refuse, warn, off; refuse with its reason for missing key, unknown mode, malformed JSON, unreadable file', (t) => {
  const dir = mkdtempSync(join(tmpdir(), 'fleet-roll-policy-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  assert.match(fleetRollMode(dir).why, /could not be read \(ENOENT\)/);
  mkdirSync(join(dir, 'genesis/agentic'), { recursive: true });
  const put = text => writeFileSync(join(dir, 'genesis/agentic/pool-policy.json'), text);
  for (const mode of ['refuse', 'warn', 'off']) { put(json({ fleet_roll_guard: { mode } })); assert.equal(fleetRollMode(dir).mode, mode); }
  put(json({ gate_on_red: { mode: 'continue' } }));
  assert.match(fleetRollMode(dir).why, /declares no fleet_roll_guard.mode/);
  put(json({ fleet_roll_guard: { mode: 'sometimes' } }));
  assert.match(fleetRollMode(dir).why, /"sometimes" is not refuse, warn or off/);
  put('{ not json');
  assert.match(fleetRollMode(dir).why, /is not valid JSON/);
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

// ── Real manifests from this repository's HEAD tree ─────────────────────────

const probe = path => evaluatePaths({ root: ROOT, sha: 'HEAD', changed: [path], target: 'dev', tip: 'feat: x', spawn: spawnIn });
const verdict = rows => (rows.find(r => r.level === 'REFUSED') ? 'REFUSED' : rows.find(r => r.level === 'WARN') ? 'WARN' : 'no row');

test('real manifests: the incident witnesses select an Edge build', () => {
  for (const path of ['elohim/holochain-conductor', 'elohim/sdk/schemas/.gitignore', 'elohim/sdk/schemas/v1/gate-live-event.schema.json', 'elohim/eprfs/device-runtime/src/lib.rs', 'elohim/Cargo.lock']) {
    const rows = probe(path);
    assert.equal(verdict(rows), 'REFUSED', path);
    assert.ok(rows[0].why.includes('cargo-build-storage') || rows[0].why.includes('cargo-build-doorway'), path);
  }
  assert.ok(probe('elohim/holochain-conductor')[0].why.includes('elohim-conductor:build-conductor-image'));
});

test('real manifests: a brit or rakia pin alone is a WARN about CI\'s checkout, the same whether or not the submodule is checked out', () => {
  for (const path of ['elohim/brit', 'elohim/rakia']) {
    const here = probe(path);
    const absent = evaluatePaths({ root: ROOT, sha: 'HEAD', changed: [path], target: 'dev', tip: 'feat: x', spawn: spawnIn, readCheckout: () => null });
    assert.equal(verdict(here), 'WARN', path);
    assert.deepEqual(absent.map(r => r.level), here.map(r => r.level), path);
    assert.match(here[0].why, /CI's checkout initialises no submodule/, path);
  }
});
