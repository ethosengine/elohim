import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'path';
import { fileURLToPath } from 'url';
import { affectedPackages, loadMetadata } from './cargo-affected.mjs';
import { runProject } from './gate-runner.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

function fixture(root = '/repo') {
  const packages = ['core', 'mid', 'leaf', 'tool', 'ext'].map(name => ({
    id: `path:${name}`, name,
    manifest_path: resolve(root, name === 'ext' ? 'ext' : `ws/${name}`, 'Cargo.toml'),
  }));
  packages.push({ id: 'registry:serde', name: 'serde', manifest_path: '/registry/serde/Cargo.toml' });
  const deps = { core: ['ext'], mid: ['core'], leaf: ['mid'], tool: ['core'], ext: [] };
  return {
    packages,
    workspace_members: ['core', 'mid', 'leaf', 'tool'].map(name => `path:${name}`),
    resolve: { nodes: Object.entries(deps).map(([name, names]) => ({
      id: `path:${name}`, deps: names.map(dep => ({ pkg: `path:${dep}` })),
    })) },
  };
}

const affected = (files, options = {}) => affectedPackages({
  root: '/repo', workspaceDir: 'ws', files, metadata: fixture(), ...options,
});

describe('Cargo reverse dependencies', () => {
  test('a leaf file selects only leaf', () => {
    assert.deepEqual(affected(['ws/leaf/src/lib.rs']).packages, ['leaf']);
  });

  test('a core file affects every member', () => {
    assert.deepEqual(affected(['ws/core/src/lib.rs']), { packages: null, why: 'every member affected' });
  });

  test('a mid file selects mid and its transitive dependent leaf', () => {
    assert.deepEqual(affected(['ws/mid/src/lib.rs']).packages, ['leaf', 'mid']);
  });

  test('a path dependency outside the workspace reaches every dependent member', () => {
    assert.deepEqual(affected(['ext/src/lib.rs']), { packages: null, why: 'every member affected' });
  });

  test('the longest package prefix wins', () => {
    const metadata = fixture();
    metadata.packages.find(pkg => pkg.name === 'leaf').manifest_path = '/repo/ws/mid/leaf/Cargo.toml';
    assert.deepEqual(affected(['ws/mid/leaf/src/lib.rs'], { metadata }).packages, ['leaf']);
  });

  test('Cargo.lock changes require the full workspace wherever they live', () => {
    for (const file of ['Cargo.lock', 'ws/Cargo.lock', 'ext/Cargo.lock', 'unrelated/Cargo.lock']) {
      const result = affected([file]);
      assert.equal(result.packages, null);
      assert.match(result.why, /Cargo.lock changed/);
    }
  });

  test('a non-package Cargo.toml requires the full workspace', () => {
    for (const file of ['ws/Cargo.toml', 'other/Cargo.toml', 'ws/mid/new/Cargo.toml']) {
      const result = affected([file]);
      assert.equal(result.packages, null);
      assert.match(result.why, /non-package Cargo.toml/);
    }
    assert.deepEqual(affected(['ws/leaf/Cargo.toml']).packages, ['leaf']);
  });

  test('an unmapped file inside the workspace requires the full workspace', () => {
    assert.deepEqual(affected(['ws/shared.txt']), { packages: null, why: 'unmapped workspace file: ws/shared.txt' });
  });

  test('unmapped docs outside the workspace and registry crates are ignored', () => {
    assert.deepEqual(affected(['docs/readme.md']).packages, []);
    assert.deepEqual(affected(['docs/readme.md', 'ws/leaf/src/lib.rs']).packages, ['leaf']);
    assert.deepEqual(affected(['registry/serde/src/lib.rs']).packages, []);
    assert.deepEqual(affected(['ws-other/file.rs']).packages, []);
  });

  test('shared compiler inputs require the full workspace', () => {
    for (const file of ['.cargo/config.toml', 'ws/.cargo/config.toml', 'rust-toolchain',
      'ws/rust-toolchain.toml', 'genesis/orchestrator/rust-style/checker.rs']) {
      assert.equal(affected([file]).packages, null, file);
    }
  });

  test('a tool read of fixtures/x.json makes that change affect tool', () => {
    const readsOf = dir => dir === 'ws/tool' ? ['fixtures/x.json'] : [];
    assert.deepEqual(affected(['fixtures/x.json'], { readsOf }).packages, ['tool']);
    assert.deepEqual(affected(['fixtures/x.json.extra'], { readsOf }).packages, []);
  });

  test('directory reads cover descendants, and readers propagate to dependents', () => {
    const readsOf = dir => dir === 'ws/mid' ? ['fixtures/'] : [];
    assert.deepEqual(affected(['fixtures/nested/x.json'], { readsOf }).packages, ['leaf', 'mid']);
    assert.deepEqual(affected(['fixtures-other/x.json'], { readsOf }).packages, []);
  });

  test('null metadata requires the full workspace', () => {
    assert.deepEqual(affected(['ws/leaf/src/lib.rs'], { metadata: null }), { packages: null, why: 'metadata unavailable' });
  });

  test('null and empty file lists require the full workspace', () => {
    for (const files of [null, []]) assert.deepEqual(affected(files), { packages: null, why: 'no changed files' });
  });

  test('missing resolution data cannot certify a smaller package set', () => {
    assert.equal(affected(['ws/leaf/src/lib.rs'], { metadata: { ...fixture(), resolve: null } }).packages, null);
  });
});

describe('metadata discovery without building', () => {
  test('uses the rustc host, locked resolution and an explicit manifest', () => {
    const calls = [];
    const metadata = fixture();
    const result = loadMetadata({ root: '/repo', workspaceDir: 'ws', spawn: (command, args, options) => {
      calls.push({ command, args });
      assert.equal(Object.hasOwn(options.env, 'CARGO_TARGET_DIR'), false);
      assert.equal(options.cwd, '/repo');
      return { status: 0, stdout: command === 'rustc' ? 'rustc 1.96.1\nhost: x86_64-unknown-linux-gnu\n' : JSON.stringify(metadata) };
    } });
    assert.deepEqual(result, metadata);
    assert.deepEqual(calls, [
      { command: 'rustc', args: ['-vV'] },
      { command: 'cargo', args: ['metadata', '--format-version', '1', '--locked', '--filter-platform',
        'x86_64-unknown-linux-gnu', '--manifest-path', '/repo/ws/Cargo.toml'] },
    ]);
  });

  test('all discovery failures return null without an offline retry', () => {
    const host = { status: 0, stdout: 'host: x86_64-unknown-linux-gnu\n' };
    for (const replies of [
      [{ status: 1, stderr: 'rustc failed' }],
      [{ status: 0, stdout: 'no host line' }],
      [host, { status: 1, stderr: 'registry unavailable' }],
      [host, { status: 0, stdout: 'not JSON' }],
      [host, { status: null, error: new Error('spawn denied') }],
    ]) {
      const result = loadMetadata({ root: '/repo', workspaceDir: 'ws', spawn: () => {
        assert.ok(replies.length, 'must not retry');
        return replies.shift();
      } });
      assert.equal(result, null);
    }
    assert.equal(loadMetadata({ root: '/repo', workspaceDir: 'ws', spawn: () => { throw new Error('denied'); } }), null);
  });
});

describe('gate-runner package environment', () => {
  test('shadow clears inherited packages; on passes the selected packages to the probe recipe', () => {
    const root = mkdtempSync(resolve(tmpdir(), 'cargo-affected-'));
    mkdirSync(resolve(root, 'genesis/agentic'), { recursive: true });
    writeFileSync(resolve(root, 'genesis/agentic/pool-policy.json'), JSON.stringify({ gate_narrow: { projects: ['eprfs'] } }));
    const changed = resolve(root, 'changed');
    writeFileSync(changed, 'ws/mid/src/lib.rs\n');
    // Same env-probe idiom as gate-runner.test.mjs; this fixture adds the new
    // carrier without modifying the root justfile's out-of-scope probe recipe.
    writeFileSync(resolve(root, 'justfile'), '_gate-selftest-env:\n    @echo "GATE_CARGO_PACKAGES=[${GATE_CARGO_PACKAGES-<unset>}]"\n');
    const project = { name: 'eprfs', dir: 'ws', run: { kind: 'root-just', recipe: '_gate-selftest-env' } };
    try {
      for (const mode of ['shadow', 'on', 'off', undefined, 'unknown']) {
        let output = '';
        let probe;
        let childEnv;
        const status = runProject(project, false, false, {
          root, readsOf: () => [],
          env: { ...process.env, GATE_NARROW: mode, GATE_NARROW_BASE: 'ok', GATE_CHANGED_FILES: changed,
            GATE_CARGO_PACKAGES: 'stale', GATE_EVIDENCE: 'off', BRIT_BIN: '/nonexistent/brit', EPR_BIN: '/nonexistent/epr' },
          write: text => { output += text; },
          spawn: (command, args, options) => {
            if (command === 'rustc') return { status: 0, stdout: 'host: x86_64-unknown-linux-gnu\n' };
            if (command === 'cargo') {
              assert.equal(args[0], 'metadata');
              return { status: 0, stdout: JSON.stringify(fixture(root)) };
            }
            if (command === 'bash' && args[0].endsWith('/run-local-gate.sh')) {
              childEnv = options.env;
              probe = spawnSync(command, [resolve(ROOT, 'genesis/orchestrator/run-local-gate.sh'), ...args.slice(1)],
                { ...options, stdio: 'pipe', encoding: 'utf8' });
              return probe;
            }
            return { status: 1, stdout: '', stderr: 'fixture has no evidence writer' };
          },
        });
        assert.equal(status, 0, probe?.stderr || output);
        if (mode === 'on') {
          assert.equal(childEnv.GATE_CARGO_PACKAGES, 'leaf mid');
          assert.match(probe.stdout, /GATE_CARGO_PACKAGES=\[leaf mid\]/);
        } else {
          assert.equal(Object.hasOwn(childEnv, 'GATE_CARGO_PACKAGES'), false);
          assert.match(probe.stdout, /GATE_CARGO_PACKAGES=\[<unset>\]/);
        }
        assert.match(output, mode === 'off' ? /narrow \(off\): full workspace — narrowing disabled/
          : /narrow \((?:shadow|on)\): leaf mid \(2 of 4 members\)/);
      }
      for (const inputs of [
        {}, { GATE_NARROW_BASE: 'none', GATE_CHANGED_FILES: changed },
        { GATE_NARROW_BASE: 'ok' }, { GATE_NARROW_BASE: 'ok', GATE_CHANGED_FILES: resolve(root, 'missing') },
      ]) {
        let output = '';
        assert.equal(runProject(project, false, false, {
          root, env: { ...inputs, GATE_NARROW: 'on', GATE_CARGO_PACKAGES: 'stale' },
          write: text => { output += text; },
          spawn: (command, args, options) => {
            assert.notEqual(command, 'cargo', 'no metadata without a real base');
            if (command === 'bash') assert.equal(Object.hasOwn(options.env, 'GATE_CARGO_PACKAGES'), false);
            return { status: 0, stdout: '' };
          },
        }), 0);
        assert.match(output, /narrow \(on\): full workspace — no push base/);
      }
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});

test('pre-push preserves unfiltered files across refs and removes the file through its exit handler', () => {
  const hook = readFileSync(resolve(ROOT, '.husky/pre-push.bash'), 'utf8');
  const handlers = hook.slice(hook.indexOf('HOOK_ON_EXIT=""'), hook.indexOf('BERTH_BIN='));
  const detection = hook.slice(hook.indexOf('CHANGED=""'), hook.indexOf('# Gate evidence and household landing'));
  for (const remote of ['base', '0000000000000000000000000000000000000000', 'unreadable']) {
    const result = spawnSync('bash', ['-c', `
      ${handlers}
      ZERO_SHA=0000000000000000000000000000000000000000
      PUSH_LINES=("refs/heads/one local refs/heads/one base" "refs/heads/two local refs/heads/two $1")
      git() {
        case "$1" in
          rev-parse) echo /fixture ;;
          diff)
            [ "$3" != unreadable ] || return 1
            printf '%s\\n' .claude/test-fixture.json ws/leaf/src/lib.rs ;;
        esac
      }
      bash() { return 0; } # The style gate is outside this input-handoff test.
      ${detection}
      printf '%s\\n' "$GATE_NARROW_BASE" "$GATE_CHANGED_FILES"
      cat "$GATE_CHANGED_FILES"
    `, 'pre-push-fixture', remote], { encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
    const [base, file, ...files] = result.stdout.split('\n');
    assert.equal(base, remote === 'base' ? 'ok' : 'none');
    assert.ok(files.includes('.claude/test-fixture.json'), '.ci-ignore must not strip package test inputs');
    assert.ok(files.includes('ws/leaf/src/lib.rs'));
    assert.equal(existsSync(file), false, 'the existing EXIT trap removed the list');
  }
});
