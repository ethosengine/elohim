/**
 * Recipe-closure digest, shadow mode: the part of the root justfile a root-just gate's recipe can
 * depend on, hashed from a commit's bytes, and the one line that says whether a justfile edit
 * would have moved the gate's key through the recipe file. Shadow changes no key, no reuse
 * decision, no record and no recorded duration; its subprocesses are bounded.
 *
 * Run:
 *   node --test genesis/orchestrator/gate-recipe-key.test.mjs
 */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chmodSync, mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import {
  clearRecipeKeyCache, recipeClosureAt, recipeClosureFromText, recipeKeyMode, recipeKeyShadow,
} from './gate-recipe-key.mjs';

const which = spawnSync('bash', ['-c', 'command -v just'], { encoding: 'utf8' });
const JUST = which.status === 0 ? which.stdout.trim() : null;
const skip = JUST ? false : 'just is not installed';
const GIT = spawnSync('bash', ['-c', 'command -v git'], { encoding: 'utf8' }).stdout.trim();
// PATH holding just and git; a test that wants just missing passes a PATH without it.
const PATH = [...new Set([JUST && dirname(JUST), '/usr/local/bin', '/usr/bin', '/bin'].filter(Boolean))].join(':');
const env = { PATH };

const BASE = `# The root justfile of a fixture.
set dotenv-load := false

root := justfile_directory()

# Recipe A: depends on dep, runs helper.
_gate-a: dep
    echo a {{ root }}
    just _helper build

_gate-b:
    echo b

dep:
    echo dep

_helper mode:
    echo helper {{ mode }}

_unrelated:
    echo unrelated
`;

const digest = (text, recipe, options = {}) => recipeClosureFromText(text, recipe, { env, ...options });

test('editing recipe A\'s body changes A\'s digest and not B\'s', { skip }, () => {
  clearRecipeKeyCache();
  const edited = BASE.replace('echo a {{ root }}', 'echo A {{ root }}');
  assert.ok(digest(BASE, '_gate-a').digest);
  assert.notEqual(digest(edited, '_gate-a').digest, digest(BASE, '_gate-a').digest);
  assert.equal(digest(edited, '_gate-b').digest, digest(BASE, '_gate-b').digest);
});

test('editing a recipe A depends on, or one it runs, changes A\'s digest; an unrelated one does not', { skip }, () => {
  const a = digest(BASE, '_gate-a');
  assert.deepEqual(Object.keys(a.members).sort(), ['_gate-a', '_helper', 'dep']);
  assert.notEqual(digest(BASE.replace('echo dep', 'echo DEP'), '_gate-a').digest, a.digest);
  assert.notEqual(digest(BASE.replace('echo helper', 'echo HELPER'), '_gate-a').digest, a.digest);
  assert.notEqual(digest(BASE.replace('_helper mode:', '_helper mode="x":'), '_gate-a').digest, a.digest,
    'a parameter of a member is in the closure');
  assert.equal(digest(BASE.replace('echo unrelated', 'echo UNRELATED'), '_gate-a').digest, a.digest);
});

const VERBS = `${BASE}
test target="changed" scope="":
    echo test {{ target }} {{ scope }}

mesh *args:
    echo mesh {{ args }}
`;
const withBody = body => VERBS.replace('_gate-b:\n    echo b', `name := "dep"\n_gate-b:\n${body}`);
const membersOf = (body, options) => {
  const result = digest(withBody(body), '_gate-b', options);
  assert.ok(result.members, `${body}: ${result.why}`);
  return Object.keys(result.members).sort();
};
const refusalOf = body => {
  const result = digest(withBody(body), '_gate-b');
  assert.equal(result.digest, undefined, body);
  return result.why;
};

test('a word that only mentions a recipe is no member: `cargo test`, prose, a comment', { skip }, () => {
  for (const body of [
    '    cargo test --workspace -- --skip live_corpus_',
    '    echo "run the mesh test, then _unrelated"',
    '    # just test mesh, some day',
  ]) {
    assert.deepEqual(membersOf(body), ['_gate-b'], body);
  }
  // `echo just mesh` is an invocation by the parse (a just word, then `mesh`): over-inclusion.
  assert.deepEqual(membersOf('    echo just mesh'), ['_gate-b', 'mesh']);
  // The analysis-only broader rule still counts a mention.
  assert.ok(membersOf('    echo see _unrelated', { mentions: true }).includes('_unrelated'));
});

test('every invocation form of just makes its recipe tokens members', { skip }, () => {
  for (const [body, expected] of [
    // `mesh` is test's argument here, but a token that equals a recipe name counts (over-inclusion).
    ['    just test mesh', ['mesh', 'test']],
    ['    @just _helper x', ['_helper']],
    ['    {{ just_executable() }} _helper x', ['_helper']],
    ['    "{{just_executable()}}" --justfile "{{justfile()}}" _helper x', ['_helper']],
    ['    env A=1 just _helper x', ['_helper']],
    ['    exec just _helper x', ['_helper']],
    ['    timeout 9 nice -n 5 just _helper x', ['_helper']],
    ['    just --justfile other.just _helper x', ['_helper']],
    ['    just --justfile "$f" _helper x', ['_helper']],
    ['    cd elsewhere && just -f ../justfile --set k v _helper x', ['_helper']],
    ['    out="$(just _helper x)"; echo "$out"', ['_helper']],
    ['    bash -c "just _helper x"', ['_helper']],
    ['    just mesh dep _unrelated', ['_unrelated', 'dep', 'mesh']],
    ['    if just _helper x; then echo ok; fi', ['_helper']],
  ]) {
    assert.deepEqual(membersOf(body).filter(n => n !== '_gate-b'), [...expected].sort(), body);
  }
});

test('a word built at run time anywhere in a just invocation\'s arguments yields no digest', { skip }, () => {
  for (const body of [
    '    just {{ name }}',
    '    just $x',
    "    args='x _unrelated'; just _helper $args",
    '    just _helper "$mode"',
    '    just test "$target" "$scope"',
    '    just mesh "$@"',
    '    for r in a b; do just $r; done',
    '    just --justfile {{ justfile() }} _helper x "$next"',
    '    {{ just_executable() }} $1',
    '    just --justfile $f _helper x',
    '    just a2o_dir=$d _helper x',
  ]) {
    assert.match(refusalOf(body), /built at run time|unquoted expansion/, body);
  }
});

test('a shape that builds the command word, or hides what runs, yields no digest', { skip }, () => {
  for (const [body, why] of [
    ['    J=just; $J _helper x', /variable holds the just command/],
    ['    J="$(command -v just)"; "$J" x', /variable holds the just command/],
    ['    eval "$cmd"', /runs eval/],
    ['    sh -c "$cmd"', /command string built at run time/],
    ['    ls | xargs just', /xargs supplies/],
    ['    source lib.sh; run x', /sources a file/],
    ['    . ./lib.sh', /sources a file/],
    ['    just --list', /prints the justfile/],
    ['    #!/usr/bin/env python3\n    import subprocess\n    subprocess.run(["just", "_helper", "x"])', /python3 recipe holds the word just/],
    ['    #!/usr/bin/env -S node --no-warnings\n    // just a comment', /node recipe holds the word just/],
  ]) {
    assert.match(refusalOf(body), why, body);
  }
  const script = VERBS.replace('_gate-b:\n    echo b', 'set unstable\n[script("python3")]\n_gate-b:\n    print("just")');
  assert.match(digest(script, '_gate-b').why, /python3 recipe holds the word just/);
  assert.ok(digest(VERBS.replace('_gate-b:\n    echo b', '_gate-b:\n    #!/usr/bin/env python3\n    print("b")'), '_gate-b').digest);
});

test('a bare `just` brings in the default recipe and `first`', { skip }, () => {
  const bare = `first:\n    echo first\n${BASE.replace('    echo b', '    just')}`;
  const result = digest(bare, '_gate-b');
  assert.ok(result.members.first, 'the default recipe is a member');
  assert.notEqual(digest(bare.replace('echo first', 'echo FIRST'), '_gate-b').digest, result.digest);
});

test('a dependency, an attribute or a quiet marker of the recipe itself moves its digest', { skip }, () => {
  const b = digest(BASE, '_gate-b').digest;
  assert.notEqual(digest(BASE.replace('_gate-b:', '_gate-b: dep'), '_gate-b').digest, b);
  assert.notEqual(digest(BASE.replace('_gate-b:', '[no-cd]\n_gate-b:'), '_gate-b').digest, b);
  assert.notEqual(digest(BASE.replace('    echo b', '    @echo b'), '_gate-b').digest, b);
});

test('editing an assignment or a setting changes every digest', { skip }, () => {
  for (const edited of [
    BASE.replace('root := justfile_directory()', 'root := justfile_directory() / "x"'),
    BASE.replace('set dotenv-load := false', 'set dotenv-load := true'),
    BASE.replace('set dotenv-load := false', 'set dotenv-load := false\nset export'),
    BASE.replace('root := justfile_directory()', 'root := justfile_directory()\nunused := "x"'),
  ]) {
    for (const recipe of ['_gate-a', '_gate-b']) {
      assert.notEqual(digest(edited, recipe).digest, digest(BASE, recipe).digest, `${recipe} after: ${edited.split('\n').slice(0, 5).join(' | ')}`);
    }
  }
});

test('reordering recipes, and comments, doc comments and blank lines outside bodies, change no digest', { skip }, () => {
  const reordered = `set dotenv-load := false


# a different file comment
_unrelated:
    echo unrelated
# a new doc comment on _gate-b
_gate-b:
    echo b
_helper mode:
    echo helper {{ mode }}

root := justfile_directory()
dep:
    echo dep
# Recipe A, reworded.
_gate-a: dep
    echo a {{ root }}
    just _helper build
`;
  for (const recipe of ['_gate-a', '_gate-b']) {
    assert.ok(digest(BASE, recipe).digest);
    assert.equal(digest(reordered, recipe).digest, digest(BASE, recipe).digest, recipe);
  }
});

test('a file with import or mod yields no digest, whatever the dump says', () => {
  for (const line of ['import? "extra.just"', 'mod? extra', 'import "extra.just"', 'mod extra']) {
    const result = digest(`${line}\n${BASE}`, '_gate-a');
    assert.equal(result.digest, undefined, line);
    assert.match(result.why, /import or module/, line);
  }
});

test('a missing just, an unparseable file, an absent recipe or a throwing spawn yields no digest and never throws', () => {
  clearRecipeKeyCache();
  assert.match(recipeClosureFromText(BASE, '_gate-a', { env: { PATH: '' } }).why, /just not on PATH/);
  const thrower = () => { throw new Error('spawn exploded'); };
  assert.match(recipeClosureFromText(BASE, '_gate-a', { env, justBin: '/x/just', spawn: thrower }).why, /spawn exploded/);
  if (JUST) {
    assert.match(digest('_gate-a:\n\techo a\n    echo mixed\n', '_gate-a').why, /dump failed/);
    assert.match(digest(BASE, '_gate-nope').why, /recipe _gate-nope not found/);
  }
});

test('every subprocess gets a SIGKILL deadline', () => {
  clearRecipeKeyCache();
  const seen = [];
  const spawn = (bin, args, opts) => { seen.push(opts); return { status: null, stdout: '', stderr: '', error: { code: 'ETIMEDOUT' } }; };
  assert.match(recipeClosureFromText(BASE, '_gate-a', { env, justBin: '/x/just', spawn }).why, /timed out and was killed/);
  assert.ok(seen.length > 0 && seen.every(opts => opts.killSignal === 'SIGKILL' && opts.timeout > 0 && opts.timeout <= 2000));
});

// ---------------------------------------------------------------- a fixture repository

const DATE = { GIT_AUTHOR_DATE: '2026-11-21T00:00:00Z', GIT_COMMITTER_DATE: '2026-11-21T00:00:00Z' };
function git(repo, args) {
  const r = spawnSync(GIT, ['-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', ...args], {
    cwd: repo, encoding: 'utf8',
    env: { ...process.env, ...DATE, GIT_AUTHOR_NAME: 'f', GIT_AUTHOR_EMAIL: 'f@x', GIT_COMMITTER_NAME: 'f', GIT_COMMITTER_EMAIL: 'f@x' },
  });
  assert.equal(r.status, 0, `git ${args.join(' ')}: ${r.stderr}`);
  return r.stdout.trim();
}
function commitFiles(repo, files, message) {
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(dirname(join(repo, path)), { recursive: true });
    writeFileSync(join(repo, path), text);
  }
  git(repo, ['add', '-A']);
  git(repo, ['commit', '-q', '--no-verify', '-m', message]);
  return git(repo, ['rev-parse', 'HEAD']);
}

const fixtureProject = {
  name: 'fixture-gate', dir: 'p', pipeline: 'fixture', steps: ['s'], manifestPath: 'p/build-manifest.json',
  run: { kind: 'root-just', recipe: '_gate-a' },
};
const fixtureManifest = JSON.stringify({
  pipeline: 'fixture',
  steps: { s: { inputs: { sources: ['p/**'], buildProcess: [] } } },
  gate: { projects: { 'fixture-gate': { dir: 'p', steps: ['s'], run: { kind: 'root-just', recipe: '_gate-a' } } } },
});
const pathsMaterial = JSON.stringify({ reads: { scope: 'paths', oids: [] } });

/** `branch` at a base commit; HEAD on a feature branch whose justfile is `next`. */
function fixtureRepo(t, next, { extra = {}, branch = 'dev' } = {}) {
  const repo = mkdtempSync(join(tmpdir(), 'gate-recipe-key-'));
  t.after(() => rmSync(repo, { recursive: true, force: true }));
  git(repo, ['init', '-q', '-b', branch]);
  const files = {
    '.gitignore': '.claude/\n',
    justfile: BASE,
    'p/build-manifest.json': fixtureManifest,
    'p/x.txt': 'x\n',
    'genesis/orchestrator/run-local-gate.sh': '#!/bin/sh\n',
    ...extra,
  };
  const base = commitFiles(repo, files, 'base');
  git(repo, ['switch', '-q', '-c', 'feature']);
  const head = next === BASE ? base : commitFiles(repo, { justfile: next }, 'edit');
  return { repo, base, head };
}

/** A spawn that runs real git (and just), counting every call. */
function counting() {
  const calls = [];
  const spawn = (bin, args, opts) => { calls.push([bin, ...args].join(' ')); return spawnSync(bin === 'git' ? GIT : bin, args, opts); };
  return { spawn, calls };
}

test('the shadow line says the recipe file would not have moved the key when only another recipe changed', { skip }, t => {
  clearRecipeKeyCache();
  const { repo, base, head } = fixtureRepo(t, BASE.replace('echo b', 'echo B'));
  const line = recipeKeyShadow(fixtureProject, { root: repo, commit: head, env, material: pathsMaterial });
  assert.equal(line, `justfile changed since ${base.slice(0, 9)}, recipe closure unchanged — the recipe file would not have moved this key`);
});

test('the shadow line names the changed closure member', { skip }, t => {
  clearRecipeKeyCache();
  const { repo, base, head } = fixtureRepo(t, BASE.replace('echo helper', 'echo HELPER'));
  assert.equal(recipeKeyShadow(fixtureProject, { root: repo, commit: head, env, material: pathsMaterial }),
    `justfile changed since ${base.slice(0, 9)}, recipe closure changed (_helper)`);
  const setting = fixtureRepo(t, BASE.replace('set dotenv-load := false', 'set dotenv-load := true'));
  assert.match(recipeKeyShadow(fixtureProject, { root: setting.repo, commit: setting.head, env, material: pathsMaterial }),
    /recipe closure changed \(settings\)$/);
});

test('an unchanged closure says which other piece holds the justfile, or that it cannot tell, or makes no claim when dirty', { skip }, t => {
  clearRecipeKeyCache();
  const { repo, head } = fixtureRepo(t, BASE.replace('echo b', 'echo B'));
  const shadow = (material, project = fixtureProject, dirty = false) =>
    recipeKeyShadow(project, { root: repo, commit: head, env, material: JSON.stringify(material), dirty });
  assert.match(shadow({ reads: { scope: 'tree', oid: 't' } }), /recipe closure unchanged, but the whole-tree read scope also holds the justfile — the justfile still moves this key$/);
  assert.match(shadow({ reads: { scope: 'paths', oids: [{ path: 'justfile', oid: 'o' }] } }), /but the read set \(justfile\) also holds/);
  assert.match(shadow({ reads: { scope: 'paths', oids: [] } }, { ...fixtureProject, dir: '.' }), /but the root directory tree also holds/);
  assert.match(shadow({ reads: { scope: 'paths', oids: [] } }, { ...fixtureProject, manifestPath: 'p/missing.json' }),
    /recipe closure unchanged; whether another piece of the key holds the justfile cannot be told \(manifest p\/missing.json unreadable at [0-9a-f]{9}\)$/);
  assert.match(shadow({ reads: { scope: 'paths', oids: [] } }, { ...fixtureProject, steps: ['nope'] }), /cannot be told \(step nope has no readable inputs\)/);
  assert.match(shadow({}), /cannot be told \(the material names no read scope\)/);
  assert.match(shadow({ reads: { scope: 'paths', oids: [] } }, fixtureProject, true), /working tree is dirty or unknown, so no claim about the key is made$/);
  assert.match(shadow({ reads: { scope: 'paths', oids: [] } }, fixtureProject, null), /no claim about the key is made$/);
});

test('an unchanged justfile costs only the memoised base and blob reads, once per process', { skip }, t => {
  clearRecipeKeyCache();
  const same = fixtureRepo(t, BASE);
  const head = commitFiles(same.repo, { 'p/x.txt': 'y\n' }, 'not the justfile');
  const { spawn, calls } = counting();
  assert.equal(recipeKeyShadow(fixtureProject, { root: same.repo, commit: head, env, material: pathsMaterial, spawn }), null);
  assert.deepEqual(calls.map(call => call.split(' ').slice(0, 2).join(' ')), ['git merge-base', 'git merge-base', 'git rev-parse', 'git rev-parse'],
    'origin/dev (absent), dev, then the two justfile blob ids');
  const before = calls.length;
  assert.equal(recipeKeyShadow({ ...fixtureProject, name: 'second', run: { kind: 'root-just', recipe: '_gate-b' } },
    { root: same.repo, commit: head, env, material: pathsMaterial, spawn }), null);
  assert.equal(calls.length, before, 'a second project runs no subprocess at all');
});

test('no shadow line when off, for a project with its own justfile, or with no key computed', { skip }, t => {
  clearRecipeKeyCache();
  const edited = fixtureRepo(t, BASE.replace('echo b', 'echo B'));
  const { spawn, calls } = counting();
  const ask = (project, extra) => recipeKeyShadow(project, { root: edited.repo, commit: edited.head, env, material: pathsMaterial, spawn, ...extra });
  assert.equal(ask(fixtureProject, { env: { ...env, GATE_RECIPE_KEY: 'off' } }), null);
  assert.equal(ask({ ...fixtureProject, run: { kind: 'just', recipe: 'gate' } }), null);
  assert.equal(ask(fixtureProject, { material: undefined }), null, 'no key was computed, so there is no key to compare');
  assert.equal(calls.length, 0, 'none of these starts a subprocess');
  assert.equal(recipeKeyMode({}), 'shadow');
  assert.equal(recipeKeyMode({ GATE_RECIPE_KEY: 'on' }), 'shadow', 'there is no `on` yet');
  assert.equal(recipeKeyMode({ GATE_RECIPE_KEY: 'off' }), 'off');
});

test('with neither origin/dev nor dev, one line per process says so, then nothing', t => {
  clearRecipeKeyCache();
  const { repo, head } = fixtureRepo(t, BASE.replace('echo b', 'echo B'), { branch: 'trunk' });
  const ask = name => recipeKeyShadow({ ...fixtureProject, name }, { root: repo, commit: head, env, material: pathsMaterial, spawn: counting().spawn });
  assert.match(ask('one'), /neither origin\/dev nor dev resolves here/);
  assert.equal(ask('two'), null);
});

test('the dump runs once per justfile per process, however many projects ask', { skip }, t => {
  clearRecipeKeyCache();
  const { repo, head, base } = fixtureRepo(t, BASE.replace('echo b', 'echo B'));
  const { spawn, calls } = counting();
  for (const recipe of ['_gate-a', '_gate-b', '_gate-a']) {
    assert.ok(recipeClosureAt(repo, head, recipe, { env, spawn }).digest);
    assert.ok(recipeClosureAt(repo, base, recipe, { env, spawn }).digest);
  }
  assert.equal(calls.filter(call => call.includes('--dump')).length, 2);
  assert.equal(calls.filter(call => call.startsWith('git cat-file')).length, 2);
});

test('a missing just degrades the shadow to one line, never a throw (no just needed)', t => {
  clearRecipeKeyCache();
  const { repo, head, base } = fixtureRepo(t, BASE.replace('echo b', 'echo B'));
  const line = recipeKeyShadow(fixtureProject, {
    root: repo, commit: head, env: { PATH: '/nonexistent' }, material: pathsMaterial, spawn: counting().spawn,
  });
  assert.equal(line, `justfile changed since ${base.slice(0, 9)}; no recipe closure (just not on PATH) — the whole-file key stands`);
});

/** A `just` that ignores SIGTERM and sleeps: only SIGKILL ends it. */
function stubbornJust(t, seconds) {
  const dir = mkdtempSync(join(tmpdir(), 'gate-recipe-key-bin-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const path = join(dir, 'just');
  writeFileSync(path, `#!/bin/sh\ntrap '' TERM\nexec sleep ${seconds}\n`);
  chmodSync(path, 0o755);
  return path;
}
const dumpDirs = () => readdirSync(tmpdir()).filter(name => name.startsWith('gate-recipe-dump-')).sort();

test('a hung just is killed at its deadline and leaves no temporary directory', t => {
  clearRecipeKeyCache({ totalMs: 4000, callMs: 300 });
  const { repo, head } = fixtureRepo(t, BASE.replace('echo b', 'echo B'));
  const before = dumpDirs();
  const started = Date.now();
  const line = recipeKeyShadow(fixtureProject, { root: repo, commit: head, env, material: pathsMaterial, justBin: stubbornJust(t, 30), spawn: counting().spawn });
  assert.ok(Date.now() - started < 2500, `returned in ${Date.now() - started} ms`);
  assert.match(line, /no recipe closure \(just --dump timed out and was killed\)/);
  assert.deepEqual(dumpDirs(), before);
  clearRecipeKeyCache();
});

test('the shadow stops once its budget is spent, says so once, and is silent after', t => {
  clearRecipeKeyCache({ totalMs: 600, callMs: 5000 });
  const { repo, head } = fixtureRepo(t, BASE.replace('echo b', 'echo B'));
  const justBin = stubbornJust(t, 30);
  const started = Date.now();
  const ask = name => recipeKeyShadow({ ...fixtureProject, name }, { root: repo, commit: head, env, material: pathsMaterial, justBin, spawn: counting().spawn });
  assert.match(ask('one'), /^stopped: the shadow's 0.6 s budget for this run is spent; no further recipe-key lines$/);
  const { spawn, calls } = counting();
  assert.equal(recipeKeyShadow(fixtureProject, { root: repo, commit: head, env, material: pathsMaterial, justBin, spawn }), null);
  assert.equal(calls.length, 0, 'a spent budget starts nothing');
  assert.ok(Date.now() - started < 2500, `the budget held: ${Date.now() - started} ms`);
  clearRecipeKeyCache();
});

// ---------------------------------------------------------------- shadow changes no verdict

const SEED = '9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60';
const PUBLIC_KEY = 'd75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a';
const OUTPUT_CID = [0x12, 0x20, ...Array(32).fill(7)];

/**
 * runProject over a real fixture repository: git and just are real, brit, the recipe and epr are
 * doubles, and any other command fails the test. The clock advances one second per subprocess, so
 * shadow subprocesses would show in every recorded duration unless their span is taken out.
 */
async function runFixture(t, mode, { reuse }) {
  const { runProject } = await import('./gate-runner.mjs');
  const policy = JSON.stringify({ gate_reuse: { projects: reuse ? ['fixture-gate'] : [] } });
  const { repo, head } = fixtureRepo(t, BASE.replace('echo b', 'echo B'), { extra: { 'genesis/agentic/pool-policy.json': policy } });
  mkdirSync(join(repo, '.git/brit'), { recursive: true });
  writeFileSync(join(repo, '.git/brit/agent-key'), Buffer.from(SEED, 'hex'));
  const bins = mkdtempSync(join(tmpdir(), 'gate-recipe-key-brit-'));
  t.after(() => rmSync(bins, { recursive: true, force: true }));
  for (const name of ['brit', 'brit-build-ref']) {
    writeFileSync(join(bins, name), '#!/bin/sh\nexit 1\n');
    chmodSync(join(bins, name), 0o755);
  }
  const green = { commit: head, attestation: { success: true, agentId: PUBLIC_KEY, builtAt: '2026-11-21T00:00:00Z', outputCid: OUTPUT_CID } };
  let now = 0n;
  const recorded = [];
  const unexpected = [];
  const spawn = (bin, args, opts) => {
    now += 1_000_000_000n;
    const call = [bin, ...args];
    if (bin === 'git' || bin === JUST) return spawnSync(bin === 'git' ? GIT : bin, args, opts);
    // A brit without --inputs: GATE_INPUTS (unset, so shadow) prints its unavailable line in both
    // runs and decides as off, so the two switches are proven together.
    if (bin.endsWith('/brit') && args[2] === '--help') return { status: 0, stdout: 'Usage: brit build fingerprint [OPTIONS] <MANIFEST>\n', stderr: '' };
    if (bin.endsWith('/brit') && args[0] === 'build' && args[1] === 'fingerprint') {
      return { status: 0, stdout: JSON.stringify({ fingerprints: [{ step: 's', fingerprint: 'bafy-fixture' }] }), stderr: '' };
    }
    if (bin.endsWith('/brit-build-ref')) {
      const key = args[args.indexOf('--inputs-hash') + 1];
      if (args.includes('find')) { recorded.push(['find', key]); return { status: 0, stdout: JSON.stringify(reuse ? [green] : []), stderr: '' }; }
      if (args.includes('put')) {
        recorded.push(['put', key, args[args.indexOf('--output-json') + 1], args[args.indexOf('--duration-ms') + 1]]);
        return { status: 0, stdout: 'bafyrecorded\n', stderr: '' };
      }
      if (args.includes('signer')) return { status: 0, stdout: `{"agentId":"${PUBLIC_KEY}"}\n`, stderr: '' };
      if (args.includes('get')) return { status: 0, stdout: '', stderr: `no build ref found for step=x commit=${head}\n` };
    }
    if (bin === 'rustc') return { status: 1, stdout: '', stderr: '' };
    if (bin === 'bash' && args[0].endsWith('run-local-gate.sh')) return { status: 0, stdout: '', stderr: '' };
    if (bin === 'epr' && args[0] === 'peer' && args[1] === 'verdicts') return { status: 0, stdout: '[]', stderr: '' };
    if (bin === 'epr' && args[0] === 'peer' && args[1] === 'leases') return { status: 0, stdout: '{"standing":"none"}', stderr: '' };
    if (bin === 'epr' && args[0] === 'flow' && args[1] === 'note') {
      recorded.push(['note', ...args.map(arg => arg.split(repo).join('<repo>'))]);
      return { status: 0, stdout: '', stderr: '' };
    }
    unexpected.push(call.join(' '));
    return { status: 127, stdout: '', stderr: 'unexpected command' };
  };
  const lines = [];
  const status = runProject(fixtureProject, false, false, {
    root: repo, spawn, tap: false, write: text => lines.push(text), warn: () => {}, clock: () => now,
    env: {
      PATH, BRIT_BIN: join(bins, 'brit'), BRIT_BUILD_REF_BIN: join(bins, 'brit-build-ref'),
      GATE_TASK: 'off', GATE_EVIDENCE: 'reuse', GATE_RUN_KIND: 'full-gate', GATE_RECIPE_KEY: mode,
    },
  });
  const text = lines.join('');
  return {
    status,
    unexpected,
    shadow: text.split('\n').filter(line => line.includes('recipe-key (shadow)')),
    lines: text.split('\n').filter(line => !line.includes('recipe-key (shadow)')).map(line => line.replace(/\d+[smhd] ago/, 'AGE')),
    recorded,
  };
}

for (const reuse of [false, true]) {
  test(`GATE_RECIPE_KEY=shadow and off decide, record and time identically (${reuse ? 'a reused green' : 'a run that records'})`, { skip }, async t => {
    clearRecipeKeyCache();
    const shadow = await runFixture(t, 'shadow', { reuse });
    clearRecipeKeyCache();
    const off = await runFixture(t, 'off', { reuse });
    assert.deepEqual(shadow.unexpected, []);
    assert.deepEqual(off.unexpected, []);
    assert.equal(shadow.shadow.length, 1, 'the shadow run printed its line');
    // A gate with no audited reads keys on the whole tree, so the justfile still moves its key.
    assert.match(shadow.shadow[0], /\[gate\] fixture-gate recipe-key \(shadow\): justfile changed since [0-9a-f]{9}, recipe closure unchanged, but the whole-tree read scope also holds the justfile — the justfile still moves this key$/);
    assert.equal(off.shadow.length, 0);
    assert.equal(shadow.status, off.status);
    // Every key looked up and recorded, the verdict JSON, --duration-ms and the cycle note args.
    assert.deepEqual(shadow.recorded, off.recorded);
    assert.deepEqual(shadow.lines, off.lines);
    assert.ok(off.lines.includes('[gate] inputs: --inputs unavailable; selection and keys use legacy all'), 'GATE_INPUTS shadow ran, and said why it compares nothing');
    assert.ok(shadow.recorded.some(([kind]) => kind === 'find'));
    if (reuse) assert.match(shadow.lines.join('\n'), /fixture-gate reused: green/);
    else {
      const put = shadow.recorded.find(([kind]) => kind === 'put');
      assert.ok(put, 'the run recorded its verdict');
      assert.ok(Number(put[3]) > 0, 'the recorded duration is the clock\'s, nonzero, and equal in both modes');
    }
  });
}
