import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { loadGateRegistry } from './pipeline-registry.mjs';
import { loadManifests } from './manifest-utils.mjs';
import { gateChildEnv, oracleMode, projectsForChanges, selectGateProjects, worktreeTargetDir } from './gate-runner.mjs';
import { resolveRakiaBin } from './gate-oracle.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

describe('manifest-driven local gate registry', () => {
  const registry = loadGateRegistry(ROOT);

  test('every project has typed execution metadata', () => {
    assert.ok(registry.size >= 35);
    for (const project of registry.values()) {
      assert.ok(['just', 'root-just', 'attested'].includes(project.run.kind), project.name);
      if (project.run.kind === 'attested') {
        assert.equal(project.run.recipe, undefined, `${project.name}: attested projects run no recipe`);
        assert.equal(project.run.attestation.provider, 'github-checks', project.name);
        assert.match(project.run.attestation.repo, /^[\w.-]+\/[\w.-]+$/, project.name);
        assert.ok(project.run.attestation.check.length > 0, project.name);
      } else {
        assert.match(project.run.recipe, /^_?[a-z][a-z0-9-]*$/);
      }
    }
    for (const name of ['brit', 'rakia', 'sophia']) {
      assert.equal(registry.get(name).run.kind, 'attested', `${name} is an attested component`);
    }
  });

  test('a pin move is a direct input of its component step, in both matchers', () => {
    for (const [component, gitlink] of [['brit', 'elohim/brit'], ['rakia', 'elohim/rakia'], ['sophia', 'sophia']]) {
      const project = registry.get(component);
      const manifest = loadManifests(ROOT).find(m => m.content.pipeline === project.pipeline);
      const step = manifest.content.steps[project.steps[0]];
      assert.ok(step.inputs.sources.includes(gitlink), `${component}: ${gitlink} listed verbatim, not only as a glob`);
    }
  });

  test('a rakia pin move fires the two schema gates directly', () => {
    const selected = projectsForChanges(ROOT, ['elohim/rakia'], { oracle: 'path', log: () => {} }).map(p => p.name);
    assert.ok(selected.includes('rakia-validate'));
    assert.ok(selected.includes('rakia-codegen'));
    assert.ok(selected.includes('rakia'));
  });

  test('resolves an explicit project or the most specific owning path', () => {
    assert.deepStrictEqual(selectGateProjects(registry, 'epr-ts').map(p => p.name), ['epr-ts']);
    assert.deepStrictEqual(
      selectGateProjects(registry, 'elohim/sdk/epr-ts/src/index.ts').map(p => p.name),
      ['epr-ts']
    );
  });

  test('representative diffs select manifest and gate-only projects', () => {
    const epr = projectsForChanges(ROOT, ['elohim/sdk/epr-ts/src/index.ts']).map(p => p.name);
    assert.ok(epr.includes('epr-ts'));
    assert.ok(!epr.includes('elements-codegen'), 'gate-only inputs must not default to every pipeline step');

    const app = projectsForChanges(ROOT, ['app/elohim-library/projects/foo.ts']).map(p => p.name);
    assert.ok(app.includes('elohim-library'));
    assert.ok(app.includes('elohim-storybook'));

    const seam = projectsForChanges(ROOT, ['crates/seam-contracts/src/lib.rs']).map(p => p.name);
    assert.ok(seam.includes('seam-contracts'));
    assert.equal(registry.get('seam-contracts').run.cargo.workspace, 'crates/seam-contracts');
    assert.equal(registry.get('elohim-compute').run.cargo.workspace, 'elohim');
  });

  test('former shell-only checks are all selected by manifest inputs', () => {
    const cases = [
      ['epr-storage', 'elohim/elohim-storage/src/api/epr.rs'],
      ['epr-storage', 'elohim/elohim-storage/src/api/epr/routes.rs'],
      ['reach-drift', 'genesis/data/lamad/content/example.json'],
      ['domain-types', 'elohim/sdk/domains/lamad/types/src/lib.rs'],
      ['rakia-codegen', 'elohim/rakia/schemas/v1/example.json'],
      ['rakia-validate', 'steward/device/build-manifest.json'],
      ['cargo-coverage', 'steward/node/Cargo.toml'],
      ['elements-codegen', 'app/elohim-elements/elohim-core/src/card.ts'],
      ['pipeline-list-fresh', 'app/elohim-app/build-manifest.json'],
    ];

    for (const [expected, changed] of cases) {
      const selected = projectsForChanges(ROOT, [changed]).map(project => project.name);
      assert.ok(selected.includes(expected), `${changed} must select ${expected}`);
    }
  });


  test('names-only CLI returns an empty list for a documentation-only change', () => {
    const command = [resolve(ROOT, 'genesis/orchestrator/gate-runner.mjs'), '--changed-file-list'];
    const input = 'genesis/data/timeline/backlog/CLUSTERS.md\n';
    const names = spawnSync(process.execPath, [...command, '--names'], {
      cwd: ROOT, encoding: 'utf8', input,
    });
    assert.equal(names.status, 0, names.stderr);
    assert.equal(names.stdout, '', 'pre-push must not interpret status prose as gate names');

    const human = spawnSync(process.execPath, command, { cwd: ROOT, encoding: 'utf8', input });
    assert.equal(human.status, 0, human.stderr);
    assert.equal(human.stdout, '[gate] no manifest-declared projects selected\n');
  });

  test('names-only CLI keeps selected names and rejects an unknown target', () => {
    const command = [resolve(ROOT, 'genesis/orchestrator/gate-runner.mjs'), '--names', '--target'];
    const selected = spawnSync(process.execPath, [...command, 'doorway'], {
      cwd: ROOT, encoding: 'utf8',
    });
    assert.equal(selected.status, 0, selected.stderr);
    assert.equal(selected.stdout, 'doorway\n');

    const unknown = spawnSync(process.execPath, [...command, 'not-a-project'], {
      cwd: ROOT, encoding: 'utf8',
    });
    assert.notEqual(unknown.status, 0);
    assert.match(unknown.stderr, /Unknown gate project or path/);
  });

  test('an empty rustflags declaration CLEARS RUSTFLAGS instead of inheriting', () => {
    // Regression: run-local-gate.sh used ${9:-__inherit__}, so `rustflags: ""`
    // — how every native crate drops the ambient WASM getrandom cfg — collapsed
    // back to inherit and each native gate died at link time with
    // `undefined symbol: __getrandom_v03_custom`.
    const probe = (rustflags) => spawnSync(
      'bash',
      [resolve(ROOT, 'genesis/orchestrator/run-local-gate.sh'),
        ROOT, 'selftest', '.', 'root-just', '_gate-selftest-env', '', '', 'dev', rustflags],
      { encoding: 'utf8', env: { ...process.env, RUSTFLAGS: '--cfg getrandom_backend="custom"' } }
    );

    assert.match(probe('').stdout, /RUSTFLAGS=\[\]/);
    assert.match(probe('__inherit__').stdout, /RUSTFLAGS=\[--cfg getrandom_backend="custom"\]/);

    // Every manifest that declares an empty rustflags depends on the above.
    const clearing = [...registry.values()]
      .filter(project => project.run.cargo && project.run.cargo.rustflags === '');
    assert.ok(clearing.length >= 8, `expected native gates that clear RUSTFLAGS, got ${clearing.length}`);
  });

  test('a manifest cargo.env reaches the gate as environment, never as argv', () => {
    // The positional contract is fixed at four cargo args; a fifth would break every
    // caller of run-local-gate.sh. `run.cargo.env` therefore travels as ONE serialized
    // variable that the script parses and exports before `just`.
    const declared = { run: { cargo: { workspace: 'x', env: { CARGO_BUILD_JOBS: '2', RUST_TEST_THREADS: '2' } } } };
    const bare = { run: { cargo: { workspace: 'x' } } };
    const none = { run: {} };

    assert.equal(
      gateChildEnv(declared, {}).GATE_CARGO_ENV,
      '{"CARGO_BUILD_JOBS":"2","RUST_TEST_THREADS":"2"}'
    );
    // A project without the key must be unchanged — and must not inherit a stale
    // GATE_CARGO_ENV from the parent environment.
    assert.equal(Object.hasOwn(gateChildEnv(bare, { GATE_CARGO_ENV: '{"X":"1"}' }), 'GATE_CARGO_ENV'), false);
    assert.equal(Object.hasOwn(gateChildEnv(none, {}), 'GATE_CARGO_ENV'), false);
    assert.equal(Object.hasOwn(gateChildEnv(declared, { cargoEnv: undefined, PATH: '/bin' }), 'PATH'), true);

    const probe = (env) => spawnSync(
      'bash',
      [resolve(ROOT, 'genesis/orchestrator/run-local-gate.sh'),
        ROOT, 'selftest', '.', 'root-just', '_gate-selftest-env', '', '', 'dev', ''],
      { encoding: 'utf8', env: { ...process.env, ...env } }
    );

    const capped = probe({ GATE_CARGO_ENV: '{"CARGO_BUILD_JOBS":"2","RUST_TEST_THREADS":"2"}' });
    assert.equal(capped.status, 0, capped.stderr);
    assert.match(capped.stdout, /CARGO_BUILD_JOBS=\[2\]/);
    assert.match(capped.stdout, /RUST_TEST_THREADS=\[2\]/);
    // The carrier itself is consumed, so a nested cargo run cannot re-apply it.
    assert.match(capped.stdout, /GATE_CARGO_ENV=\[<unset>\]/);

    const uncapped = probe({ GATE_CARGO_ENV: undefined, CARGO_BUILD_JOBS: undefined, RUST_TEST_THREADS: undefined });
    assert.equal(uncapped.status, 0, uncapped.stderr);
    assert.match(uncapped.stdout, /CARGO_BUILD_JOBS=\[<unset>\]/);
    assert.match(uncapped.stdout, /RUST_TEST_THREADS=\[<unset>\]/);
  });

  test('cargo env merges the manifest with pool-policy overrides, manifest winning', () => {
    // The rakia schema accepts run.cargo.env since 2b2cedb, and the storage cap now
    // declares on its own manifest project. pool-policy's cargo_env_overrides remain
    // for projects that have not moved their cap yet (eprfs, memory-ceremony). Merge
    // rule: the manifest's own declaration always wins; the policy override only fills
    // a gap the manifest leaves undeclared.
    const noManifestEnv = { name: 'eprfs', run: { cargo: { targetDir: '/tmp/x' } } };
    const filled = gateChildEnv(noManifestEnv, {});
    assert.ok(Object.hasOwn(filled, 'GATE_CARGO_ENV'), 'pool-policy override applies when the manifest declares no cargo.env');
    assert.deepStrictEqual(JSON.parse(filled.GATE_CARGO_ENV), { CARGO_BUILD_JOBS: '1', RUST_TEST_THREADS: '1' });

    const withManifestEnv = {
      name: 'eprfs',
      run: { cargo: { targetDir: '/tmp/x', env: { CARGO_BUILD_JOBS: '3' } } },
    };
    const won = gateChildEnv(withManifestEnv, {});
    assert.deepStrictEqual(
      JSON.parse(won.GATE_CARGO_ENV),
      { CARGO_BUILD_JOBS: '3', RUST_TEST_THREADS: '1' },
      'a manifest-declared cargo.env value wins over the pool-policy override; undeclared keys are still filled'
    );

    const storage = registry.get('elohim-storage');
    assert.deepStrictEqual(JSON.parse(gateChildEnv(storage, {}).GATE_CARGO_ENV), { CARGO_BUILD_JOBS: '1' },
      'the storage cap is declared on its manifest, no longer in pool-policy');

    const neither = { name: 'not-a-real-project-xyz', run: { cargo: { workspace: 'x' } } };
    assert.equal(Object.hasOwn(gateChildEnv(neither, {}), 'GATE_CARGO_ENV'), false);
  });

  test('unknown targets fail instead of silently running the wrong gate', () => {
    assert.throws(() => selectGateProjects(registry, 'not-a-project'), /Unknown gate project/);
  });

  test('content validation cannot fall through to the writing seeder', () => {
    const scripts = JSON.parse(readFileSync(resolve(ROOT, 'genesis/seeder/package.json'), 'utf8')).scripts;
    assert.equal(Object.hasOwn(scripts, 'seed:validate'), false);
    assert.equal(Object.hasOwn(scripts, 'seed:dry-run'), false);

    const justfile = readFileSync(resolve(ROOT, 'justfile'), 'utf8');
    const seedRecipe = justfile.slice(justfile.indexOf('seed action='), justfile.indexOf('# Render a page'));
    assert.ok(seedRecipe.includes('src/schema-validation.ts'));
    assert.doesNotMatch(seedRecipe, /--validate-only|--dry-run/);
  });
});

describe('selection oracle — shadow, rakia, path', () => {
  const stale = new Map([
    ['elohim-sophia:build-sophia-umd', ['source: sophia']],
    ['elohim:build-angular', ['upstream: elohim-sophia:build-sophia-umd']],
  ]);
  const withOracle = () => stale;
  const noOracle = () => null;

  test('path mode ignores the oracle entirely', () => {
    const names = projectsForChanges(ROOT, ['sophia'], { oracle: 'path', rakia: withOracle, log: () => {} }).map(p => p.name);
    assert.ok(names.includes('sophia'));
    assert.ok(!names.includes('elohim-app'), 'path-only never propagates');
  });

  test('shadow mode selects by path and prints one oracle-diff line when the sets differ', () => {
    const lines = [];
    const names = projectsForChanges(ROOT, ['sophia'], { oracle: 'shadow', rakia: withOracle, log: l => lines.push(l) }).map(p => p.name);
    assert.ok(!names.includes('elohim-app'), 'shadow mode does not change selection');
    assert.equal(lines.length, 1);
    assert.match(lines[0], /^\[gate\] oracle-diff: \+elohim-app/);
  });

  test('shadow mode is silent when the sets agree', () => {
    const lines = [];
    // A documentation-only change selects nothing by path (proven below by the names-only CLI
    // case); an oracle that also finds nothing stale must not print a diff.
    const selected = projectsForChanges(ROOT, ['genesis/data/timeline/backlog/CLUSTERS.md'], { oracle: 'shadow', rakia: () => new Map(), log: l => lines.push(l) });
    assert.deepEqual(selected, []);
    assert.deepEqual(lines, []);
  });

  test('rakia mode selects the direct component and its one-hop consumer with the upstream reason', () => {
    const projects = projectsForChanges(ROOT, ['sophia'], { oracle: 'rakia', rakia: withOracle, log: () => {} });
    const app = projects.find(p => p.name === 'elohim-app');
    assert.ok(app, 'elohim-app is a depth-one consumer of the sophia pin');
    assert.deepEqual(app.reasons, ['upstream: elohim-sophia:build-sophia-umd']);
    assert.ok(projects.some(p => p.name === 'sophia'));
  });

  test('rakia mode falls back to path selection and says so when the oracle is unavailable', () => {
    const lines = [];
    const names = projectsForChanges(ROOT, ['sophia'], { oracle: 'rakia', rakia: noOracle, log: l => lines.push(l) }).map(p => p.name);
    assert.ok(names.includes('sophia'));
    assert.ok(!names.includes('elohim-app'));
    assert.deepEqual(lines, ['[gate] rakia unavailable — path-only selection']);
  });

  test('a gitlink path and files beneath it select the component once, reasons merged', () => {
    const projects = projectsForChanges(ROOT, ['elohim/brit', 'elohim/brit/Cargo.toml'], { oracle: 'path', log: () => {} });
    assert.equal(projects.filter(p => p.name === 'brit').length, 1);
    const brit = projects.find(p => p.name === 'brit');
    assert.ok(brit.reasons.some(r => r === 'source: elohim/brit'));
  });

  test('GATE_ROOT points the CLI at another repository', () => {
    const out = spawnSync(process.execPath, [resolve(ROOT, 'genesis/orchestrator/gate-runner.mjs'), '--list'], {
      cwd: ROOT, encoding: 'utf8', env: { ...process.env, GATE_ROOT: resolve(ROOT, 'genesis/a2o') },
    });
    assert.equal(out.status, 0);
    assert.equal(out.stdout.trim(), '', 'a2o has no build-manifest.json, so the registry is empty');
  });
});

describe('attested dispatch', () => {
  test('an attested project never reaches run-local-gate.sh', () => {
    const out = spawnSync(process.execPath, [resolve(ROOT, 'genesis/orchestrator/gate-runner.mjs'), '--target', 'brit'], {
      cwd: ROOT, encoding: 'utf8', env: { ...process.env, GH_BIN: '/nonexistent/gh', EPR_BIN: '/nonexistent/epr' },
    });
    assert.equal(out.status, 0, out.stdout + out.stderr);
    assert.match(out.stdout, /attested, no local recipe/);
    assert.match(out.stdout, /attested: claimed — gh not found/);
    assert.doesNotMatch(out.stdout, /cargo target:/);
  });
});

describe('the oracle flip', () => {
  test('rakia is the default oracle once the shadow run has been read', () => {
    assert.equal(oracleMode({}), 'rakia');
    assert.equal(oracleMode({ GATE_ORACLE: 'shadow' }), 'shadow');
    assert.equal(oracleMode({ GATE_ORACLE: 'path' }), 'path');
  });
});

describe('the hook parses stdout as project names — diagnostics never ride on it', () => {
  const cli = resolve(ROOT, 'genesis/orchestrator/gate-runner.mjs');
  // The shadow assertion needs the REAL oracle: without a `rakia` binary the runner
  // honestly prints "rakia unavailable — path-only selection" (covered by the next test),
  // and the oracle-diff line cannot exist. The pinned elohim/rakia builds no CLI, so a
  // checkout without one skips this case instead of failing every other developer's gate.
  const rakiaBin = resolveRakiaBin(process.env);
  test('shadow mode: --names stdout is only names; the oracle-diff line goes to stderr',
    { skip: rakiaBin ? false : 'no rakia binary on PATH or RAKIA_BIN — the shadow oracle-diff needs the real oracle' }, () => {
    const out = spawnSync(process.execPath, [cli, '--changed-file-list', '--names'], {
      cwd: ROOT, encoding: 'utf8', input: 'sophia\n', env: { ...process.env, GATE_ORACLE: 'shadow' },
    });
    assert.equal(out.status, 0, out.stderr);
    assert.equal(out.stdout.trim(), 'sophia');
    assert.match(out.stderr, /\[gate\] oracle-diff: \+elohim-app/);
  });
  test('rakia mode without a binary: --names stdout is only names; the fallback line goes to stderr', () => {
    const out = spawnSync(process.execPath, [cli, '--changed-file-list', '--names'], {
      cwd: ROOT, encoding: 'utf8', input: 'sophia\n', env: { ...process.env, GATE_ORACLE: 'rakia', RAKIA_BIN: '/nonexistent/rakia' },
    });
    assert.equal(out.status, 0, out.stderr);
    assert.equal(out.stdout.trim(), 'sophia');
    assert.match(out.stderr, /\[gate\] rakia unavailable — path-only selection/);
  });
  test('shadow mode without a binary also says so, so the flip evidence cannot silently never arrive', () => {
    const lines = [];
    projectsForChanges(ROOT, ['sophia'], { oracle: 'shadow', rakia: () => null, log: l => lines.push(l) });
    assert.deepEqual(lines, ['[gate] rakia unavailable — path-only selection']);
  });
  test('--list names the attestation for attested projects instead of an undefined recipe', () => {
    const out = spawnSync(process.execPath, [cli, '--list'], { cwd: ROOT, encoding: 'utf8' });
    assert.match(out.stdout, /^brit\telohim\/brit\tattested:ethosengine\/brit#Tests pass$/m);
    assert.doesNotMatch(out.stdout, /undefined/);
  });
});

describe('worktree-scoped cargo target', () => {
  const main = () => ({ gitDir: '/r/.git', commonDir: '/r/.git' });
  const linked = () => ({ gitDir: '/r/.git/worktrees/wt-a', commonDir: '/r/.git' });

  test('the main checkout keeps the manifest path the hooks read', () => {
    assert.equal(worktreeTargetDir('/tmp/eprfs-gate-target', '/r', main), '/tmp/eprfs-gate-target');
  });

  test('a linked worktree builds into its own suffixed dir', () => {
    assert.equal(
      worktreeTargetDir('/tmp/eprfs-gate-target', '/r/.claude/worktrees/wt-a', linked),
      '/tmp/eprfs-gate-target-wt-wt-a',
    );
  });

  test('an undeclared target stays undeclared, and an unreadable repo changes nothing', () => {
    assert.equal(worktreeTargetDir('', '/r/.claude/worktrees/wt-a', linked), '');
    assert.equal(worktreeTargetDir('/tmp/x', '/r', () => null), '/tmp/x');
  });
});
