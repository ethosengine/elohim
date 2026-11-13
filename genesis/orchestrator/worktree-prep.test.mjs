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

function fixture(t, { hook = true, fail, projects, ineffectiveBuild = false } = {}) {
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
