import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { currentChanges, mainCheckout, prep, submoduleCommand } from './worktree-prep.mjs';
import { ATTESTED_SUBMODULES, runPreflight, WORKSPACE_INSTALL } from './gate-preflight.mjs';

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
  assert.deepEqual(clones, [...ATTESTED_SUBMODULES, 'elohim/holochain-conductor'], 'the attested three as before, then the absent declared one');
  assert.match(f.logs.join('\n'), /elohim\/holochain-conductor: pin aaaaaaaaaaaa is in no object store here/);
});
