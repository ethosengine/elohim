import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { currentChangeList, currentChanges, mainCheckout, neededSubmodule, prep, submoduleCommand } from './worktree-prep.mjs';
import { ATTESTED_SUBMODULES, attestedNeed, checkSubmodules, runPreflight, WORKSPACE_INSTALL } from './gate-preflight.mjs';

const ok = stdout => ({ status: 0, stdout: stdout || '' });
const put = (root, file, value = '') => {
  mkdirSync(join(root, file, '..'), { recursive: true });
  writeFileSync(join(root, file), value);
};

function fixture(t, { hook = true, fail, projects, ineffectiveBuild = false, git } = {}) {
  const root = mkdtempSync(join(tmpdir(), 'worktree-prep-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const calls = [], logs = [];
  const initialized = new Set();
  const selected = projects || [
    { name: 'elohim-app', dir: 'app/elohim-app', run: { kind: 'root-just' } },
    { name: 'genesis-a2o', dir: 'genesis/a2o', run: { kind: 'just' } },
  ];
  put(root, 'app/elohim-app/package.json', JSON.stringify({ dependencies: { 'elohim-core': '*', '@elohim/epr': '*' } }));
  for (const name of ['elohim-core', 'unused']) {
    put(root, `app/elohim-elements/${name}/package.json`, JSON.stringify({ exports: { './register': { import: './dist/register.js' } } }));
  }
  const spawn = (command, args, options) => {
    calls.push([command, ...args]);
    if (fail?.(command, args)) return { status: 7, stderr: 'fixture failure' };
    if (command === 'test') return spawnSync(command, args, options);
    if (command === 'bash') return { status: 1 }; // cargo-deny unavailable; never invoke cargo.
    if (command === 'git') {
      const answer = git?.(args, initialized);
      if (answer) return answer;
      if (args[0] === '-c') { initialized.add(args.at(-1)); return ok(); }
      if (args.includes('status')) return ok(ATTESTED_SUBMODULES.map(m => `${initialized.has(m) ? ' ' : '-'}abc ${m}`).join('\n'));
      if (args.includes('--git-common-dir')) return ok('/main checkout/.git\n');
      if (args.includes('--git-dir')) return ok('/main checkout/.git/worktrees/new\n');
      if (args[0] === 'merge-base') return ok('base\n');
      if (args[0] === 'rev-parse') return ok('head\n');
      return ok('app/elohim-app/src/main.ts\0');
    }
    assert.equal(command, 'pnpm', 'unexpected external command (including cargo)');
    if (args[0] === 'install') {
      put(root, 'node_modules/.modules.yaml');
      if (hook) { put(root, '.husky/_/pre-push', '#!/bin/sh\n'); chmodSync(join(root, '.husky/_/pre-push'), 0o755); }
    } else if (!ineffectiveBuild) {
      const outputs = {
        'elohim-core': 'app/elohim-elements/elohim-core/dist/register.js',
        '@elohim/storage-client...': 'elohim/sdk/storage-client-ts/dist/index.js',
        '@elohim/epr': 'elohim/sdk/epr-ts/dist/index.js',
      };
      assert.ok(outputs[args[1]], `unexpected build ${args}`);
      put(root, outputs[args[1]]);
    }
    return ok();
  };
  const options = {
    root, spawn, exists: () => true, log: line => logs.push(line),
    select: (selectedRoot, changed) => {
      assert.equal(selectedRoot, root);
      assert.deepEqual(changed, ['app/elohim-app/src/main.ts']);
      assert.equal(initialized.size, 3, 'load manifest registry after submodule checkout');
      return selected;
    },
    // This sandbox has /tmp/.git. Test the prep contract independently of that real refusal.
    preflight: options => runPreflight({ ...options, registry: new Map(), env: { TMPDIR: '/prep-fixture-tmp' } }),
  };
  return { root, options, calls, logs };
}

test('fresh linked worktree: local submodules, frozen install, only refused builds, full preflight, executable hook', t => {
  const f = fixture(t);
  assert.equal(prep(f.options), 0, f.logs.join('\n'));
  const effects = f.calls.filter(([cmd, arg]) => cmd === 'pnpm' || (cmd === 'git' && arg === '-c'));
  assert.deepEqual(effects.slice(0, 3).map(c => c.at(-1)), ATTESTED_SUBMODULES);
  assert.deepEqual(effects[3], WORKSPACE_INSTALL);
  assert.deepEqual(effects.slice(4), [
    ['pnpm', '--filter', 'elohim-core', 'run', 'build'],
    ['pnpm', '--filter', '@elohim/storage-client...', 'run', 'build'],
    ['pnpm', '--filter', '@elohim/epr', 'run', 'build'],
  ]);
  assert.deepEqual(f.calls.at(-1), ['test', '-x', '.husky/_/pre-push']);
  assert.match(f.logs.join('\n'), /WARN sophia-umd/);
  assert.match(f.logs.at(-2), /preflight clear/);
  assert.match(f.logs.at(-1), /is executable/);
  f.calls.length = 0;
  assert.equal(prep(f.options), 0);
  assert.deepEqual(f.calls.filter(([cmd]) => cmd === 'pnpm'), [WORKSPACE_INSTALL], 'rerun builds nothing');
  assert.ok(!f.calls.some(([cmd, arg]) => cmd === 'git' && arg === '-c'), 'initialized modules remain untouched');
});

test('no selected projects still installs hooks, builds no dists', t => {
  const f = fixture(t, { projects: [] });
  assert.equal(prep(f.options), 0);
  assert.deepEqual(f.calls.filter(([cmd]) => cmd === 'pnpm'), [WORKSPACE_INSTALL]);
});

test('missing hook refuses even when preflight is clear (Landing 21)', t => {
  const f = fixture(t, { hook: false });
  assert.equal(prep(f.options), 1);
  assert.match(f.logs.at(-1), /REFUSED.*pre-push is missing or not executable/);
});

test('non-executable hook refuses too', t => {
  const f = fixture(t, { hook: false });
  put(f.root, '.husky/_/pre-push', '#!/bin/sh\n');
  assert.equal(prep(f.options), 1);
});

test('successful command that leaves dists missing is caught by final preflight', t => {
  const f = fixture(t, { ineffectiveBuild: true });
  assert.equal(prep(f.options), 1);
  assert.match(f.logs.at(-2), /preflight REFUSED/);
});

for (const stage of ['submodule', 'install', 'build', 'selection']) {
  test(`failed ${stage} stops subsequent effects and still asserts the hook`, t => {
    const f = fixture(t, { fail: (cmd, args) => (
      stage === 'submodule' ? cmd === 'git' && args[0] === '-c' :
      stage === 'install' ? cmd === 'pnpm' && args[0] === 'install' :
      stage === 'selection' ? cmd === 'git' && args[0] === 'merge-base' :
      cmd === 'pnpm' && args.includes('build')
    ) });
    assert.equal(prep(f.options), 1);
    assert.deepEqual(f.calls.at(-1), ['test', '-x', '.husky/_/pre-push']);
    assert.ok(!f.logs.some(line => line.includes('preflight clear')));
    if (stage === 'submodule') assert.ok(!f.calls.some(([cmd]) => cmd === 'pnpm'));
  });
}

test('main checkout discovery handles relative common dir and ordinary checkout', () => {
  const spawn = (_cmd, args) => ok(args.includes('--git-common-dir') ? '../../.git\n' : '../../.git/worktrees/w\n');
  assert.equal(mainCheckout({ root: '/repo/.worktrees/w', spawn }), '/repo');
  assert.equal(mainCheckout({ root: '/repo', spawn: () => ok('.git\n') }), null);
  assert.throws(() => mainCheckout({ root: '/repo', spawn: () => ({ status: 1 }) }), /failed/);
});

test('submodule cloning uses a local reference and source; network protocols and fetching disabled', () => {
  const command = submoduleCommand('elohim/brit', { main: '/main checkout', exists: () => true });
  assert.deepEqual(command, ['git', '-c', 'protocol.allow=never', '-c', 'protocol.file.allow=always',
    '-c', 'submodule.elohim/brit.url=/main checkout/elohim/brit',
    'submodule', 'update', '--init', '--no-fetch', '--reference', '/main checkout/elohim/brit', '--', 'elohim/brit']);
  assert.throws(() => submoduleCommand('sophia', { main: '/main', exists: () => false }), /local submodule objects unavailable/);
  assert.ok(!submoduleCommand('sophia', { main: null }).includes('--reference'));
});

test('change selection includes committed, staged, unstaged and untracked paths without splitting spaces', () => {
  const spawn = (_cmd, args) => {
    if (args[0] === 'rev-parse') return ok('head');
    if (args[0] === 'merge-base') return ok('base\n');
    if (args.includes('base')) return ok('branch.js\0');
    if (args.includes('--cached')) return ok('staged.js\0');
    if (args[0] === 'ls-files') return ok('new file.js\0');
    return ok(' leading space.js\0staged.js\0');
  };
  assert.deepEqual(currentChanges({ root: '/repo', spawn }), [' leading space.js', 'branch.js', 'new file.js', 'staged.js']);
});

test('missing local base uses HEAD changes; unreadable range fails closed', () => {
  const calls = [];
  const spawn = (_cmd, args) => {
    calls.push(args);
    if (args[0] === 'rev-parse') return { status: 1 };
    return ok(args[0] === 'diff-tree' ? 'committed.js\0' : '');
  };
  assert.deepEqual(currentChanges({ root: '/repo', spawn }), ['committed.js']);
  assert.ok(calls.some(args => args.includes('--root')));
  assert.throws(() => currentChanges({ root: '/repo', spawn: () => ({ status: 1, stderr: 'bad range' }) }), /bad range/);
});

test('root dev prep dispatches the prep entrypoint', () => {
  const justfile = readFileSync(new URL('../../justfile', import.meta.url), 'utf8');
  assert.match(justfile, /prep\) exec node "\{\{ root \}\}\/genesis\/orchestrator\/worktree-prep\.mjs" ;;/);
});

const PIN = 'f9787c9e'.padEnd(40, '0');
const ls = module => ok(`160000 commit ${PIN}\t${module}\n`);

test('row 11 (a): a pin the main checkout lacks is taken from the sibling worktree that holds it, and named', t => {
  const f = fixture(t, {
    git: (args, initialized) => {
      // The main checkout's rakia lacks the pin: its clone fails ("unable to read tree").
      if (args[0] === '-c' && args.includes('submodule.elohim/rakia.url=/main checkout/elohim/rakia')) {
        return { status: 128, stderr: 'fatal: unable to read tree f9787c9e' };
      }
      if (args[0] === '-c' && args.includes('submodule.elohim/rakia.url=/wt/nt-e1/elohim/rakia')) {
        initialized.add('elohim/rakia'); return ok();
      }
      if (args.includes('ls-tree')) return ls(args.at(-1));
      if (args.includes('worktree')) return ok(`worktree /main checkout\nHEAD x\n\nworktree /wt/stale\n\nworktree /wt/nt-e1\n\nworktree ${f.root}\n`);
      if (args.includes('cat-file')) return args[1] === '/wt/nt-e1/elohim/rakia' ? ok() : { status: 1 };
      return undefined;
    },
  });
  f.options.exists = path => !path.startsWith('/wt/stale');
  assert.equal(prep(f.options), 0, f.logs.join('\n'));
  const log = f.logs.join('\n');
  assert.match(log, /elohim\/rakia: the main checkout lacks pin f9787c9e0000; using \/wt\/nt-e1/);
  const clones = f.calls.filter(([cmd, arg, ...rest]) => cmd === 'git' && arg === '-c' && rest.at(-1) === 'elohim/rakia');
  assert.equal(clones.length, 2, 'the main checkout first, then the sibling');
  assert.ok(clones[1].includes('--reference') && clones[1].includes('/wt/nt-e1/elohim/rakia'), clones[1].join(' '));
  assert.ok(clones[1].includes('protocol.allow=never'), 'still local-only');
  assert.ok(!f.calls.some(call => call.includes('cat-file') && call.includes(`${f.root}/elohim/rakia`)), 'never its own checkout');
  // The first attempt's leftover repository is given the pin before the retry (live, 2026-11-18).
  const fetch = f.calls.find(call => call.includes('fetch') && call.includes('/wt/nt-e1/elohim/rakia'));
  assert.deepEqual(fetch, ['git', '-C', join(f.root, 'elohim/rakia'), '-c', 'protocol.allow=never',
    '-c', 'protocol.file.allow=always', 'fetch', '--no-tags', '/wt/nt-e1/elohim/rakia', PIN]);
  assert.ok(f.calls.indexOf(fetch) < f.calls.indexOf(clones[1]), 'fetched before the retry');
});

test('row 11 (a): with no local holder the refusal names every checkout tried and the remote cure', t => {
  const f = fixture(t, {
    git: args => {
      if (args[0] === '-c' && args.at(-1) === 'elohim/rakia') return { status: 128, stderr: 'fatal: unable to read tree' };
      if (args.includes('ls-tree')) return ls(args.at(-1));
      if (args.includes('worktree')) return ok(`worktree /main checkout\n\nworktree /wt/a\n`);
      if (args.includes('cat-file')) return { status: 1 };
      return undefined;
    },
  });
  assert.equal(prep(f.options), 1);
  const refused = f.logs.find(line => line.startsWith('[prep] REFUSED:'));
  assert.match(refused, /no local checkout holds the pinned commit f9787c9e0000 \(tried \/main checkout, \/wt\/a\)/);
  assert.match(refused, /git submodule update --init -- elohim\/rakia/);
  assert.ok(!f.calls.some(([cmd]) => cmd === 'pnpm'), 'nothing installs after a refused submodule');
});

function movedFixture(t, { dirty = false, aborted = false, indexEntries = '', held = true, sibling = true } = {}) {
  const status = ATTESTED_SUBMODULES.map(m => `${m === 'elohim/brit' ? '+' : ' '}abc ${m}`).join('\n');
  const f = fixture(t, {
    git: (args, initialized) => {
      ATTESTED_SUBMODULES.forEach(m => initialized.add(m));
      if (args.includes('--porcelain') && args.includes('status')) {
        return ok(aborted ? 'D  Cargo.toml\nD  src/lib.rs\n' : dirty ? ' M src/lib.rs\n' : '');
      }
      if (args.includes('status')) return ok(status);
      if (args.includes('ls-tree')) return ls(args.at(-1));
      if (args[0] === '-C' && args.includes('ls-files') && !args.includes('--others')) return ok(indexEntries);
      if (args.includes('worktree')) return ok('worktree /main checkout\n\nworktree /wt/nt-e2\n');
      if (args.includes('cat-file')) {
        if (args[1].endsWith(`${f.root}/elohim/brit`) || args[1] === join(f.root, 'elohim/brit')) return held ? ok() : { status: 1 };
        return sibling && args[1] === '/wt/nt-e2/elohim/brit' ? ok() : { status: 1 };
      }
      if (args.includes('fetch') || (args[0] === '-c' && args.includes('--no-fetch') && !args.includes('--init'))) return ok();
      return undefined;
    },
  });
  return f;
}

test('row 11 (b): a clean submodule at another commit is moved to its pin, not warned', t => {
  const f = movedFixture(t);
  assert.equal(prep(f.options), 0, f.logs.join('\n'));
  assert.match(f.logs.join('\n'), /elohim\/brit: moving the clean checkout to its pin f9787c9e0000/);
  const update = f.calls.find(call => call[0] === 'git' && call.includes('--no-fetch') && call.at(-1) === 'elohim/brit');
  assert.deepEqual(update, ['git', '-c', 'protocol.allow=never', '-c', 'protocol.file.allow=always',
    'submodule', 'update', '--no-fetch', '--', 'elohim/brit']);
  assert.ok(!f.calls.some(call => call.includes('fetch') && !call.includes('--no-fetch')), 'its own checkout holds the pin');
});

test('row 11 (b): a pin the moved submodule lacks is fetched from a sibling first, locally', t => {
  const f = movedFixture(t, { held: false });
  assert.equal(prep(f.options), 0, f.logs.join('\n'));
  assert.match(f.logs.join('\n'), /elohim\/brit: pin f9787c9e0000 fetched from \/wt\/nt-e2/);
  const fetch = f.calls.find(call => call.includes('fetch') && !call.includes('--no-fetch'));
  assert.deepEqual(fetch, ['git', '-C', join(f.root, 'elohim/brit'), '-c', 'protocol.allow=never',
    '-c', 'protocol.file.allow=always', 'fetch', '--no-tags', '/wt/nt-e2/elohim/brit', PIN]);
});

test('row 11 (b): a dirty moved submodule is left alone with the warning', t => {
  const f = movedFixture(t, { dirty: true });
  assert.equal(prep(f.options), 0, f.logs.join('\n'));
  assert.match(f.logs.join('\n'), /WARN elohim\/brit: differs from its pin and has local changes/);
  assert.ok(!f.calls.some(call => call.includes('--no-fetch') && call.at(-1) === 'elohim/brit' && !call.includes('--init')),
    'no update over local work');
});

test('row 11 (b): a moved submodule whose pin no checkout holds refuses with the cure', t => {
  const f = movedFixture(t, { held: false, sibling: false });
  assert.equal(prep(f.options), 1);
  assert.match(f.logs.find(line => line.startsWith('[prep] REFUSED:')), /no local checkout holds the pinned commit/);
});

test('E8 #2: every tracked file staged as deleted is refused, never forced, and names the command', t => {
  // An aborted checkout and a deliberate deletion of every file both leave an empty index.
  const f = movedFixture(t, { aborted: true, held: false });
  assert.equal(prep(f.options), 1, f.logs.join('\n'));
  const line = f.logs.find(l => l.startsWith('[prep] REFUSED elohim/brit:'));
  assert.ok(line, f.logs.join('\n'));
  assert.match(line, /every tracked file \(2\) is staged as deleted/);
  assert.match(line, /git submodule update --force -- elohim\/brit$/);
  assert.ok(!f.calls.some(call => call.includes('--force')), 'nothing is forced');
  assert.ok(!f.calls.some(call => call.includes('--no-fetch') && call.at(-1) === 'elohim/brit' && !call.includes('--init')),
    'no update of that submodule at all');
  assert.ok(f.calls.some(call => call.includes('install')), 'the rest of prep still runs');
});

test('E6 #1: one intentionally staged deletion in a moved submodule is work, never forced away', t => {
  // `git status` lists only changed paths, so a single staged deletion beside untouched files also
  // has every status line starting "D ". The index still holds the untouched files: not aborted.
  const f = movedFixture(t, { aborted: true, indexEntries: 'README.md\nsrc/main.rs\n' });
  assert.equal(prep(f.options), 0, f.logs.join('\n'));
  assert.match(f.logs.join('\n'), /WARN elohim\/brit: differs from its pin and has local changes/);
  assert.ok(!f.calls.some(call => call.includes('--force')), 'no forced update over staged work');
  assert.ok(!f.calls.some(call => call.includes('--no-fetch') && call.at(-1) === 'elohim/brit' && !call.includes('--init')),
    'no update at all');
});


test('declared submodules: prep initializes one whose pin is in no store and skips one that is present', t => {
  const f = fixture(t);
  f.options.select = () => [];
  const pin = 'a'.repeat(40);
  f.options.declaredPins = () => [
    { path: 'elohim/holochain-conductor', oid: pin, store: null },
    { path: 'che-devworkspaces', oid: pin, store: '/store/che-devworkspaces' },
    { path: 'elohim/brit', oid: pin, store: null }, // attested: prep's existing loop owns it
  ];
  assert.equal(prep(f.options), 0, f.logs.join('\n'));
  const clones = f.calls.filter(([cmd, arg]) => cmd === 'git' && arg === '-c').map(call => call.at(-1));
  assert.deepEqual(clones, ['elohim/holochain-conductor'], 'the absent declared one; no selected project needs an attested one');
  assert.match(f.logs.join('\n'), /elohim\/holochain-conductor: pin aaaaaaaaaaaa is in no object store here/);
});

// Needed vs not needed: a declared pin whose objects no local checkout holds.
const CHE = 'che-devworkspaces', COND = 'elohim/holochain-conductor';
function neededFixture(t, { selected, heldBy = {}, failModules = [] }) {
  const f = fixture(t);
  f.options.select = () => selected;
  f.options.declaredPins = () => [CHE, COND].map(path => ({ path, oid: 'b'.repeat(40), store: null }));
  const inner = f.options.spawn;
  f.options.spawn = (command, args, options) => {
    if (command === 'git' && args.includes('cat-file')) {
      // `git -C <checkout>/<module> cat-file -e <pin>^{commit}`: held only where the fixture says.
      const dir = args[args.indexOf('-C') + 1];
      return { status: Object.keys(heldBy).some(module => dir.endsWith(`/${module}`) && heldBy[module]) ? 0 : 1 };
    }
    if (command === 'git' && args.includes('ls-tree')) return ok(`160000 commit ${'b'.repeat(40)}\t${args.at(-1)}\n`);
    if (command === 'git' && args[0] === '-c' && failModules.includes(args.at(-1))) return { status: 1, stderr: 'unable to read tree' };
    return inner(command, args, options);
  };
  return f;
}
const cloned = f => f.calls.filter(([cmd, arg]) => cmd === 'git' && arg === '-c').map(call => call.at(-1));

test('not needed and held by no local checkout: one line, no clone, no fetch, install and hook check still run', t => {
  const f = neededFixture(t, { selected: [] });
  assert.equal(prep(f.options), 0, f.logs.join('\n'));
  const out = f.logs.join('\n');
  assert.match(out, new RegExp(`\\[prep\\] ${CHE}: not needed for this worktree and not present locally; left uninitialised`));
  assert.match(out, new RegExp(`\\[prep\\] ${COND}: not needed for this worktree and not present locally; left uninitialised`));
  assert.deepEqual(cloned(f), [], 'and no attested submodule either: nothing selected needs one');
  assert.ok(f.calls.some(([cmd, arg]) => cmd === 'pnpm' && arg === 'install'), 'pnpm install ran');
  assert.ok(f.calls.some(([cmd]) => cmd === 'test'), 'hook check ran');
  assert.ok(!f.calls.some(call => call.includes('fetch')), 'nothing fetched');
});

test('needed (a selected project names it) and held by no local checkout: REFUSED as before', t => {
  const f = neededFixture(t, { selected: [{ name: 'orchestrator', dir: 'genesis/orchestrator' }], failModules: [CHE] });
  assert.equal(prep(f.options), 1);
  assert.match(f.logs.join('\n'), new RegExp(`REFUSED: ${CHE}: no local checkout holds the pinned commit bbbbbbbbbbbb`));
  assert.doesNotMatch(f.logs.join('\n'), /not needed for this worktree/);
});

test('not needed but available locally is initialised as today', t => {
  const f = neededFixture(t, { selected: [], heldBy: { [CHE]: true, [COND]: true } });
  assert.equal(prep(f.options), 0, f.logs.join('\n'));
  assert.deepEqual(cloned(f), [CHE, COND]);
  assert.doesNotMatch(f.logs.join('\n'), /left uninitialised/);
});

test('neededSubmodule: attested always; a manifest step source names it; otherwise not', () => {
  const read = () => JSON.stringify({ steps: { build: { inputs: { sources: [COND] } } } });
  assert.equal(neededSubmodule('sophia', [], { read }), true);
  assert.equal(neededSubmodule(COND, [{ name: 'conductor-image', manifestPath: 'elohim/conductor-image/build-manifest.json' }], { read }), true);
  assert.equal(neededSubmodule(CHE, [{ name: 'conductor-image', manifestPath: 'elohim/conductor-image/build-manifest.json' }], { read }), false);
  assert.equal(neededSubmodule(CHE, [], { read }), false);
  assert.equal(neededSubmodule(CHE, null, { read }), true, 'selection unavailable: every submodule stays needed');
  const unreadable = () => { throw new Error('ENOENT'); };
  assert.equal(neededSubmodule(CHE, [{ name: 'x', manifestPath: 'x/build-manifest.json' }], { read: unreadable }), true, 'unreadable manifest: needed');
});

test('the pre-push hook and prep read one attested list file', () => {
  const hook = readFileSync(new URL('../../.husky/pre-push.bash', import.meta.url), 'utf8');
  assert.match(hook, /attested-list\.mjs/);
  assert.doesNotMatch(hook, /submodule status -- elohim\/brit/);
  assert.deepEqual(ATTESTED_SUBMODULES, ['elohim/brit', 'elohim/rakia', 'sophia']);
});

// Thin-device station 1 (backlog row 106): prep asks which attested submodules this work needs.
const SKIPPED = /\[preflight\] ok submodules \(not checked out and not required: elohim\/brit elohim\/rakia sophia/;

test('station 1: no selected project needs an attested submodule: none is cloned, the skip is named, prep is clear', t => {
  const f = fixture(t, { projects: [] });
  f.options.select = () => [];
  assert.equal(prep(f.options), 0, f.logs.join('\n'));
  assert.deepEqual(cloned(f), []);
  assert.match(f.logs[0], SKIPPED, 'said before anything else');
  assert.ok(f.logs.filter(line => SKIPPED.test(line)).length >= 2, 'and again by the final preflight, which no longer refuses');
  assert.match(f.logs.at(-2), /preflight clear/);
});

test('station 1: a selected project that is not the orchestrator and reads no submodule skips them too', t => {
  const f = fixture(t);
  put(f.root, 'm/build-manifest.json', JSON.stringify({ pipeline: 'p', steps: { s: { inputs: { sources: ['genesis/docs/**'] } } } }));
  f.options.select = () => [{ name: 'eprfs-live-corpus', dir: '.', manifestPath: 'm/build-manifest.json', run: { kind: 'root-just' } }];
  f.options.preflight = () => [];
  assert.equal(prep(f.options), 0, f.logs.join('\n'));
  assert.deepEqual(cloned(f), []);
});

test('station 1: the orchestrator selected requires all three, as before', t => {
  const f = fixture(t);
  f.options.select = () => [{ name: 'orchestrator', dir: 'genesis/orchestrator', manifestPath: 'genesis/orchestrator/build-manifest.json', run: { kind: 'just' } }];
  f.options.preflight = () => [];
  assert.equal(prep(f.options), 0, f.logs.join('\n'));
  assert.deepEqual(cloned(f), [...ATTESTED_SUBMODULES]);
  assert.match(f.logs[0], /REFUSED submodules: not checked out: elohim\/brit elohim\/rakia sophia/);
});

test('station 1: a selection that cannot be computed requires all three', t => {
  const f = fixture(t);
  let asked = 0;
  f.options.select = () => { asked += 1; if (asked === 1) throw new Error('registry unreadable'); return []; };
  f.options.preflight = () => [];
  assert.equal(prep(f.options), 0, f.logs.join('\n'));
  assert.deepEqual(cloned(f), [...ATTESTED_SUBMODULES]);
  assert.equal(asked, 2, 'read again once the submodule manifests exist');
});

test('station 1: a moved pin requires that submodule and no other', t => {
  const f = fixture(t, { git: args => (args[0] === 'diff' || args[0] === 'ls-files' ? ok('sophia\0genesis/docs/a.md\0') : null) });
  f.options.select = () => [];
  f.options.preflight = () => [];
  assert.equal(prep(f.options), 0, f.logs.join('\n'));
  assert.deepEqual(cloned(f), ['sophia']);
  assert.match(f.logs[0], /REFUSED submodules: not checked out: sophia .*sophia: the change touches sophia/);
});

test('station 1: attestedNeed gives a reason whenever the answer cannot be computed, and null only for irrelevant work', () => {
  const manifest = { pipeline: 'elohim', dependsOn: ['elohim-sophia'], steps: { build: { depends: ['elohim-sophia:build-sophia-umd'], inputs: { sources: ['app/**'] } } } };
  const read = () => JSON.stringify(manifest);
  const none = () => new Map([['elohim', {}]]);
  const both = () => new Map([['elohim', {}], ['elohim-sophia', {}]]);
  const app = { name: 'elohim-app', dir: 'app/elohim-app', manifestPath: 'app/elohim-app/build-manifest.json', run: { kind: 'just' } };
  const ask = (module, projects, changed, pipelines = both, reader = read) => attestedNeed(module, { root: '/x', projects, changed, read: reader, pipelines });
  assert.equal(ask('sophia', [], ['genesis/docs/a.md']), null);
  assert.equal(ask('sophia', [app], ['app/elohim-app/src/main.ts']), null, 'its dependencies are all declared by manifests that are here');
  assert.match(ask('sophia', [app], ['app/elohim-app/src/main.ts'], none), /depends on elohim-sophia, which no checked-out manifest declares/);
  assert.match(ask('elohim/brit', [app], ['app/x.ts'], none), /depends on elohim-sophia/, 'which absent submodule declares it is unknown: each is needed');
  assert.match(ask('sophia', null, ['a.md']), /selection is unavailable/);
  assert.match(ask('sophia', [], null), /no change list/);
  for (const path of ['sophia', 'sophia/package.json', '.gitmodules', 'genesis/orchestrator/attested-submodules.txt']) {
    assert.match(ask('sophia', [], [path]), /the change touches/, path);
  }
  assert.equal(ask('elohim/brit', [], ['sophia']), null, 'another submodule\'s pin does not need this one');
  assert.match(ask('sophia', [{ ...app, name: 'orchestrator' }], ['x']), /orchestrator is selected/);
  assert.match(ask('sophia', [{ ...app, name: 'orchestrator-tree' }], ['x']), /orchestrator-tree is selected/);
  assert.match(ask('sophia', [{ ...app, run: { kind: 'attested' } }], ['x']), /attested project/);
  assert.match(ask('sophia', [{ ...app, dir: 'sophia/packages/x' }], ['x']), /lives in it/);
  assert.match(ask('sophia', [app], ['x'], both, () => { throw new Error('ENOENT'); }), /cannot be read/);
  assert.match(ask('sophia', [app], ['x'], () => { throw new Error('bad manifest'); }), /build manifests could not be read/);
  assert.match(ask('elohim/rakia', [app], ['x'], both, () => JSON.stringify({ steps: { v: { inputs: { sources: ['a'], buildProcess: ['elohim/rakia/schemas/scripts/validate.mjs'] } } } })), /manifest reads it/);
});

test('station 1: without a selection and a change list (just gate) every absent attested submodule still refuses', () => {
  const spawn = () => ({ status: 0, stdout: '-abc elohim/brit\n-abc elohim/rakia\n-abc sophia\n' });
  const all = checkSubmodules({ root: '/x', spawn });
  assert.equal(all.level, 'REFUSED');
  assert.deepEqual(all.missing, ['elohim/brit', 'elohim/rakia', 'sophia']);
  assert.equal(checkSubmodules({ root: '/x', spawn, projects: [], changed: null }).level, 'REFUSED');
  assert.equal(checkSubmodules({ root: '/x', spawn, projects: null, changed: ['a.md'] }).level, 'REFUSED');
  const thin = checkSubmodules({ root: '/x', spawn, projects: [], changed: ['a.md'], need: () => null });
  assert.equal(thin.level, 'ok');
  assert.deepEqual(thin.skipped, ['elohim/brit', 'elohim/rakia', 'sophia']);
});

// Review of 42faf6aaa, blocking: without origin/dev the committed part of the list is the tip commit
// alone. A two-commit branch whose FIRST commit moved the brit pin and a Cargo.toml shows neither.
test('a change list read without its base is not exact: every attested submodule is required and both cargo checks run', t => {
  const tipOnly = args => {
    if (args[0] === 'rev-parse' && args.includes('--verify')) return { status: 1, stdout: '', stderr: 'fatal: Needed a single revision' };
    if (args[0] === 'diff-tree') return ok('genesis/docs/a.md\0'); // HEAD alone; HEAD~1 moved elohim/brit and ws/Cargo.toml
    if (args[0] === 'diff' || args[0] === 'ls-files') return ok('');
    return null;
  };
  assert.deepEqual(currentChangeList({ root: '/x', spawn: (cmd, args) => tipOnly(args) || ok('') }), { paths: ['genesis/docs/a.md'], exact: false });
  assert.equal(currentChangeList({ root: '/x', spawn: () => ok('a.md\0') }).exact, true);

  const f = fixture(t, { git: tipOnly });
  put(f.root, '.gitmodules', '[submodule "elohim/brit"]\n\tpath = elohim/brit\n');
  put(f.root, 'ws/Cargo.toml', '[workspace]\nmembers = ["a"]\n');
  put(f.root, 'ws/Cargo.lock', '');
  put(f.root, 'ws/a/Cargo.toml', '[package]\nname = "a"\n');
  put(f.root, 'm/build-manifest.json', JSON.stringify({ pipeline: 'p', steps: { s: { inputs: { sources: ['genesis/docs/**'] } } } }));
  f.options.select = () => [{ name: 'a', dir: 'ws/a', manifestPath: 'm/build-manifest.json', run: { kind: 'just', cargo: {} } }];
  const inner = f.options.spawn;
  const cargo = [];
  f.options.spawn = (command, args, options) => (command === 'cargo' ? (cargo.push(args[0]), { status: 0, stderr: '' }) : inner(command, args, options));
  assert.equal(prep(f.options), 0, f.logs.join('\n'));
  const out = f.logs.join('\n');
  assert.deepEqual(cloned(f), [...ATTESTED_SUBMODULES], 'the pin move in the first commit cannot be ruled out');
  assert.match(f.logs[0], /REFUSED submodules: not checked out: elohim\/brit elohim\/rakia sophia/);
  assert.match(out, /\[prep\] origin\/dev does not resolve here, so only the tip commit's paths are known: no check is skipped by the change list/);
  assert.deepEqual(cargo, ['metadata'], 'cargo-lock ran');
  assert.match(out, /\[preflight\] ok cargo-lock:ws/);
  assert.match(out, /\[preflight\] WARN cargo-deny-bans: cargo-deny is not installed/, 'cargo-deny was asked for, not skipped');
  assert.doesNotMatch(out, /\(not run/);
});
