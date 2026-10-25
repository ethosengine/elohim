import { gateEvidence, recordEvidence } from './gate-evidence.mjs';
import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { readFileSync, mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { loadGateRegistry } from './pipeline-registry.mjs';
import { loadManifests } from './manifest-utils.mjs';
import { receiptKey, matchingReceipt, findReceipt, writeReceipt, gateChildEnv, oracleMode, projectsForChanges, selectGateProjects, worktreeTargetDir } from './gate-runner.mjs';
import { resolveRakiaBin } from './gate-oracle.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

test('Rust style selects new crates and inherited lint configuration', () => {
  for (const changed of ['new-crate/src/lib.rs', '.cargo/config.toml', 'some/crate/clippy.toml']) {
    const selected = projectsForChanges(ROOT, [changed], { oracle: 'path' }).map(p => p.name);
    assert.ok(selected.includes('rust-style'), `${changed} must select the style floor`);
  }
});

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
      // Row 35: the path-dependency closure rail (build-inputs-closure.test.mjs) runs
      // in the orchestrator project, so any Cargo manifest/lock or build-manifest
      // change anywhere selects it — a new crate under an existing one included.
      ['orchestrator', 'crates/x/Cargo.toml'],
      ['orchestrator', 'elohim/epr/Cargo.lock'],
      ['orchestrator', 'Cargo.toml'],
      ['orchestrator', 'elohim/eprfs/build-manifest.json'],
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
      '{"RUSTUP_TOOLCHAIN":"1.96.1","CARGO_BUILD_JOBS":"2","RUST_TEST_THREADS":"2"}'
    );
    // A project with no declared env gets only the pool-policy toolchain pin, and must not inherit a stale
    // GATE_CARGO_ENV from the parent environment.
    assert.equal(gateChildEnv(bare, { GATE_CARGO_ENV: '{"X":"1"}' }).GATE_CARGO_ENV, '{"RUSTUP_TOOLCHAIN":"1.96.1"}');
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
    assert.deepStrictEqual(JSON.parse(filled.GATE_CARGO_ENV), { CARGO_BUILD_JOBS: '1', RUST_TEST_THREADS: '1', RUSTUP_TOOLCHAIN: '1.96.1' });

    const withManifestEnv = {
      name: 'eprfs',
      run: { cargo: { targetDir: '/tmp/x', env: { CARGO_BUILD_JOBS: '3' } } },
    };
    const won = gateChildEnv(withManifestEnv, {});
    assert.deepStrictEqual(
      JSON.parse(won.GATE_CARGO_ENV),
      { CARGO_BUILD_JOBS: '3', RUST_TEST_THREADS: '1', RUSTUP_TOOLCHAIN: '1.96.1' },
      'a manifest-declared cargo.env value wins over the pool-policy override; undeclared keys are still filled'
    );

    const storage = registry.get('elohim-storage');
    assert.deepStrictEqual(JSON.parse(gateChildEnv(storage, {}).GATE_CARGO_ENV), { CARGO_BUILD_JOBS: '1', RUSTUP_TOOLCHAIN: '1.96.1' },
      'the storage cap is declared on its manifest, no longer in pool-policy');

    const neither = { name: 'not-a-real-project-xyz', run: { cargo: { workspace: 'x' } } };
    assert.deepStrictEqual(JSON.parse(gateChildEnv(neither, {}).GATE_CARGO_ENV), { RUSTUP_TOOLCHAIN: '1.96.1' },
      'the "*" pool-policy entry pins the local gate toolchain for every cargo project');
    assert.equal(Object.hasOwn(gateChildEnv({ name: 'not-a-real-project-xyz', run: {} }, {}), 'GATE_CARGO_ENV'), false,
      'a project without run.cargo gets no toolchain pin');
    assert.equal(
      JSON.parse(gateChildEnv({ name: 'x', run: { cargo: { workspace: 'x', env: { RUSTUP_TOOLCHAIN: '1.94.0' } } } }, {}).GATE_CARGO_ENV).RUSTUP_TOOLCHAIN,
      '1.94.0', 'a manifest may override the pinned toolchain per project');
    assert.equal(
      JSON.parse(gateChildEnv(neither, { RUSTUP_TOOLCHAIN: 'nightly' }).GATE_CARGO_ENV).RUSTUP_TOOLCHAIN,
      '1.96.1', 'the declared pin is exported over the caller environment, like any other cargo env key');
  });

  test('a pinned toolchain that is not installed refuses with the fix named', () => {
    const r = spawnSync(
      'bash',
      [resolve(ROOT, 'genesis/orchestrator/run-local-gate.sh'),
        ROOT, 'selftest', '.', 'root-just', '_gate-selftest-env', '', '', 'dev', ''],
      { encoding: 'utf8', env: { ...process.env, GATE_CARGO_ENV: '{"RUSTUP_TOOLCHAIN":"0.0.1-not-installed"}' } }
    );
    assert.equal(r.status, 2);
    assert.match(r.stderr, /pinned Rust toolchain '0\.0\.1-not-installed' is not installed; run: rustup toolchain install 0\.0\.1-not-installed/);
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

describe('a shared cargo slot never serves another checkout its compiled crates', () => {
  test('a slot built by a different checkout forgets only this workspace\'s own crates', async () => {
    const { mkdtempSync, mkdirSync, writeFileSync, existsSync, readFileSync: read, rmSync } = await import('node:fs');
    const { tmpdir } = await import('node:os');
    const target = mkdtempSync(resolve(tmpdir(), 'gate-slot-'));
    const fp = resolve(target, 'debug/.fingerprint');
    mkdirSync(resolve(fp, 'elohim-epr-0123abcd'), { recursive: true });
    mkdirSync(resolve(fp, 'serde-89ef4567'), { recursive: true });
    writeFileSync(resolve(target, '.gate-owner'), '/projects/elohim/.claude/worktrees/some-other-checkout');
    const run = () => spawnSync(
      'bash',
      [resolve(ROOT, 'genesis/orchestrator/run-local-gate.sh'),
        ROOT, 'selftest', 'elohim/epr', 'root-just', '_gate-selftest-env', '', target, 'dev', ''],
      { encoding: 'utf8', env: { ...process.env, GATE_CARGO_ENV: undefined } }
    );
    try {
      const first = run();
      assert.equal(first.status, 0, first.stderr);
      assert.match(first.stdout, /slot last built by .*some-other-checkout; forgot 1 local-crate fingerprint/);
      assert.equal(existsSync(resolve(fp, 'elohim-epr-0123abcd')), false, 'the local crate must rebuild');
      assert.equal(existsSync(resolve(fp, 'serde-89ef4567')), true, 'third-party deps stay cached');
      assert.equal(read(resolve(target, '.gate-owner'), 'utf8'), resolve(ROOT));
      // The same checkout again: nothing is forgotten.
      mkdirSync(resolve(fp, 'elohim-epr-0123abcd'), { recursive: true });
      const again = run();
      assert.equal(again.status, 0, again.stderr);
      assert.doesNotMatch(again.stdout, /forgot/);
      assert.equal(existsSync(resolve(fp, 'elohim-epr-0123abcd')), true);
      // A slot from before the guard (no marker) has an unknown owner: treated as foreign once.
      rmSync(resolve(target, '.gate-owner'));
      const unmarked = run();
      assert.equal(unmarked.status, 0, unmarked.stderr);
      assert.match(unmarked.stdout, /slot last built by an unknown checkout; forgot 1/);
    } finally {
      rmSync(target, { recursive: true, force: true });
    }
  });
});


function receiptFixture(t) {
  const root = mkdtempSync(resolve(tmpdir(), 'gate-receipt-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const dir = 'elohim/eprfs';
  mkdirSync(resolve(root, dir), { recursive: true });
  writeFileSync(resolve(root, dir, 'build-manifest.json'), '{"fixture":1}\n');
  const env = { ...process.env };
  for (const key of ['GIT_DIR', 'GIT_WORK_TREE', 'GIT_INDEX_FILE', 'GIT_PREFIX', 'GATE_COMMIT']) delete env[key];
  const git = args => {
    const r = spawnSync('git', ['-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.test', ...args], {cwd:root, env, encoding:'utf8'});
    assert.equal(r.status, 0, r.stderr);
    return r.stdout.trim();
  };
  mkdirSync(resolve(root, 'crates/external'), {recursive:true});
  writeFileSync(resolve(root, dir, 'Cargo.toml'), '[workspace]\nmembers=["."]\n[package]\nname="closure-fixture"\nversion="0.1.0"\nedition="2021"\n[lib]\npath="lib.rs"\n[dependencies]\nexternal={path="../../crates/external"}\n');
  writeFileSync(resolve(root, dir, 'lib.rs'), 'pub fn fixture() {}');
  writeFileSync(resolve(root, 'crates/external/Cargo.toml'), '[package]\nname="external"\nversion="0.1.0"\nedition="2021"\n[lib]\npath="lib.rs"\n');
  writeFileSync(resolve(root, 'crates/external/lib.rs'), 'pub fn external() {}');
  mkdirSync(resolve(root, 'genesis/agentic'), {recursive:true});
  writeFileSync(resolve(root, 'genesis/agentic/bin-tools.json'), JSON.stringify({version:1,tools:{epr:{bins:['epr'],source:{kind:'repository'}}}}));
  // EPR_STORE below names the store under test. The `epr` on PATH is the device launcher, which
  // would resolve ITSELF from that empty store and refuse, so the CLI under test is the device's
  // landed build unless the caller names a candidate.
  if (!env.EPR_BIN) {
    const current = spawnSync('epr', ['bin', '--tool', 'epr', 'resolve', '--current'], {env, encoding:'utf8'});
    if (current.status === 0 && current.stdout.trim()) env.EPR_BIN = current.stdout.trim();
  }
  env.EPR_STORE = resolve(root, 'store');
  env.EPR_REPO_ROOT = root;
  env.EPR_BIN_TOOLS = resolve(root, 'genesis/agentic/bin-tools.json');
  const metadata = spawnSync('cargo', ['metadata', '--offline', '--format-version', '1', '--manifest-path', resolve(root, dir, 'Cargo.toml')], {env, encoding:'utf8'});
  assert.equal(metadata.status, 0, metadata.stderr);
  git(['init', '-q']); git(['add', dir, 'crates/external', 'genesis/agentic']); git(['commit', '-qm', 'fixture']);
  const project = { name:'fixture', dir, run:{cargo:{env:{EPR_GATE_RECEIPT:'1', RUSTUP_TOOLCHAIN:'1.96.1'}}} };
  return {root, env, project, git};
}

test('receipt key uses clean native closure, resolved toolchain and manifest CID', function receipt_key_uses_clean_project_tree(t) {
  const fx = receiptFixture(t);
  const key = receiptKey(fx.project, fx);
  assert.match(key.closureCid, /^bafkrei/);
  assert.equal(key.toolchain, '1.96.1');
  assert.match(key.manifest, /^bafkrei/);
  writeFileSync(resolve(fx.root, 'unrelated.txt'), 'outside project');
  assert.deepEqual(receiptKey(fx.project, fx), key);
  writeFileSync(resolve(fx.root, fx.project.dir, 'untracked.rs'), 'unfinished');
  assert.equal(receiptKey(fx.project, fx), null);
  fx.git(['add', fx.project.dir]); fx.git(['commit', '-qm', 'changed tree']);
  assert.notEqual(receiptKey(fx.project, fx).closureCid, key.closureCid);
  fx.env.GATE_COMMIT = 'HEAD~1';
  assert.equal(receiptKey(fx.project, fx), null, 'never certify a tree other than the checked-out one');
});

test('only exact successful native receipt slots attest the gate', function receipt_requires_exact_successful_native_slots() {
  const key = {closureCid:'abc', toolchain:'1.96.1', manifest:'bafkreimanifest'};
  const entry = {cid:'bafkreireceipt', kind:'observation', extra:['measure:gate-receipt@1', 'value:1',
    ...Object.entries(key).map(([k,v]) => `env:${k}=${v}`)]};
  assert.equal(matchingReceipt(key, {entries:[entry]}), entry.cid);
  for (const name of Object.keys(key)) {
    assert.equal(matchingReceipt({...key, [name]:'different'}, {entries:[entry]}), null, name);
  }
  for (const changed of [ {kind:'ruling'}, {via:'other-commitment'}, {cid:'sha256:abc'},
    {extra:entry.extra.filter(s => s !== 'value:1')}, {extra:[...entry.extra, 'env:closureCid=ambiguous']} ]) {
    assert.equal(matchingReceipt(key, {entries:[{...entry,...changed}]}), null);
  }
  assert.equal(findReceipt({}, key, {run:() => ({status:1})}), null);
  assert.equal(findReceipt({}, key, {run:() => ({status:0, stdout:'bad-json'})}), null);
});

test('receipt writes through epr and refuses changed or dirty finishing trees', function receipt_refuses_changed_finishing_tree(t) {
  const fx = receiptFixture(t);
  const key = receiptKey(fx.project, fx);
  const calls = [];
  const run = (bin, args, options) => {
    if (bin === 'git' || args[0] === 'bin') return spawnSync(bin, args, options);
    calls.push(args);
    return {status:0, stdout:JSON.stringify({record_cid:'bafkreireceipt'})};
  };
  assert.equal(writeReceipt(fx.project, key, {...fx, run}), 'bafkreireceipt');
  assert.deepEqual(calls[0].slice(0, 7), ['flow','note','--kind','observation','--measure','gate-receipt@1','--subject']);
  assert.ok(calls[0].includes(`closureCid=${key.closureCid}`));
  assert.ok(calls[0].includes(`manifest=${key.manifest}`));
  writeFileSync(resolve(fx.root, fx.project.dir, 'build-manifest.json'), '{}');
  assert.equal(writeReceipt(fx.project, key, {...fx, run}), null);
  assert.equal(calls.length, 1);
});

test('native flow note round-trips a receipt through the ledger reader', t => {
  if (!process.env.EPR_BIN) return t.skip('set EPR_BIN to the built native CLI');
  const fx = receiptFixture(t);
  mkdirSync(resolve(fx.root, '.claude/epr-meta'), {recursive:true});
  writeFileSync(resolve(fx.root, '.claude/epr-meta/measures.yaml'),
    'measures:\n  - id: gate-receipt\n    version: 1\n    unit: passes\n');
  mkdirSync(resolve(fx.root, '.epr-meta'), {recursive:true});
  writeFileSync(resolve(fx.root, '.epr-meta/repository.yaml'), 'version: 1\nagent: repo:ethosengine/elohim\n');
  fx.env.EPR_BIN = process.env.EPR_BIN;
  const key = receiptKey(fx.project, fx);
  let diagnostic = '';
  const run = (bin, args, options) => {
    const result = spawnSync(bin, args, options);
    if (result.status !== 0) diagnostic += result.stderr;
    return result;
  };
  const written = writeReceipt(fx.project, key, {...fx, run});
  assert.match(written ?? '', /^baf/, diagnostic);
  assert.equal(findReceipt(fx.project, key, fx), written);
  assert.equal(findReceipt(fx.project, {...key, toolchain:'another-toolchain'}, fx), null);
});

test('pre-push skips the recipe only for a matching receipt and reruns mismatches', t => {
  if (!process.env.EPR_BIN) return t.skip('set EPR_BIN to the built native CLI');
  const fx = receiptFixture(t);
  fx.project.run.kind = 'root-just';
  fx.project.run.recipe = 'fixture-gate';
  writeFileSync(resolve(fx.root, fx.project.dir, 'build-manifest.json'), JSON.stringify({
    pipeline:'fixture', steps:{}, gate:{projects:{fixture:fx.project}},
  }));
  mkdirSync(resolve(fx.root, 'genesis/orchestrator'), {recursive:true});
  writeFileSync(resolve(fx.root, 'genesis/orchestrator/rust-style.sh'), 'exit 0\n');
  writeFileSync(resolve(fx.root, 'genesis/orchestrator/run-local-gate.sh'), 'echo RECIPE-RAN\nexit 0\n');
  mkdirSync(resolve(fx.root, '.claude/epr-meta'), {recursive:true});
  writeFileSync(resolve(fx.root, '.claude/epr-meta/measures.yaml'),
    'measures:\n  - id: gate-receipt\n    version: 1\n    unit: passes\n');
  mkdirSync(resolve(fx.root, '.epr-meta'), {recursive:true});
  writeFileSync(resolve(fx.root, '.epr-meta/repository.yaml'), 'version: 1\nagent: repo:ethosengine/elohim\n');
  fx.git(['add', fx.project.dir]); fx.git(['commit','-qm','gate manifest']);
  fx.env.EPR_BIN = process.env.EPR_BIN;
  const cli = () => spawnSync(process.execPath, [resolve(ROOT, 'genesis/orchestrator/gate-runner.mjs'), '--target', 'fixture'], {
    cwd:fx.root, env:{...fx.env, GATE_ROOT:fx.root, GATE_RUN_KIND:'pre-push'}, encoding:'utf8', timeout:15000,
  });
  const first = cli();
  assert.equal(first.status, 0, first.stderr);
  assert.match(first.stdout, /RECIPE-RAN/);
  assert.match(first.stdout, /receipt: baf/);
  const second = cli();
  assert.equal(second.status, 0, second.stderr);
  assert.match(second.stdout, /attested: receipt baf/);
  assert.doesNotMatch(second.stdout, /RECIPE-RAN/);
  writeFileSync(resolve(fx.root, fx.project.dir, 'unfinished.rs'), 'dirty');
  const dirty = cli();
  assert.equal(dirty.status, 0, dirty.stderr);
  assert.match(dirty.stdout, /RECIPE-RAN/);
  assert.doesNotMatch(dirty.stdout, /attested: receipt/);
  assert.match(dirty.stdout, /receipt: not written/);
});

test('receipt rejects a dirty path dependency outside the project directory', t => {
  const fx = receiptFixture(t);
  mkdirSync(resolve(fx.root, 'crates/external'), {recursive:true});
  writeFileSync(resolve(fx.root, 'crates/external/lib.rs'), 'dependency');
  fx.git(['add', 'crates/external']); fx.git(['commit', '-qm', 'dependency']);
  assert.ok(receiptKey(fx.project, fx));
  writeFileSync(resolve(fx.root, 'crates/external/lib.rs'), 'changed dependency');
  assert.equal(receiptKey(fx.project, fx), null);
});


test('gate evidence rejects dirty external dependency and changes key when it is committed', t => {
  const fx = receiptFixture(t);
  const project = {...fx.project, manifestPath: `${fx.project.dir}/build-manifest.json`, steps:['build']};
  const env = {...fx.env, BRIT_BIN:process.execPath, BRIT_BUILD_REF_BIN:process.execPath};
  const cargoEnv = {RUSTUP_TOOLCHAIN:'1.96.1'};
  let puts = 0;
  const spawn = (bin, args, options) => {
    if (args[0] === 'build' && args[1] === 'fingerprint') return {status:0, stdout:JSON.stringify({fingerprints:[{step:'build',fingerprint:'unchanged-directory'}]})};
    if (args[2] === 'build' && args[3] === 'find') return {status:0, stdout:'[]'};
    if (args[2] === 'build' && args[3] === 'put') { puts++; return {status:0,stdout:'stored'}; }
    return spawnSync(bin,args,options);
  };
  const options = {...fx, env, cargoEnv, spawn, commit:fx.git(['rev-parse','HEAD'])};
  const before = gateEvidence(project, options);
  assert.ok(before.key, before.line);
  writeFileSync(resolve(fx.root,'crates/external/lib.rs'), 'pub fn changed() {}');
  const dirty = gateEvidence(project, options);
  assert.equal(dirty.key, undefined, dirty.line);
  assert.equal(recordEvidence(project,before.key,{...options,...before,success:true,seconds:1}).status,1);
  assert.equal(puts,0);
  fx.git(['add','crates/external/lib.rs']); fx.git(['commit','-qm','dependency change']);
  options.commit = fx.git(['rev-parse','HEAD']);
  const after = gateEvidence(project, options);
  assert.ok(after.key, after.line);
  assert.notEqual(after.key,before.key);
  assert.equal(recordEvidence(project,after.key,{...options,...after,success:true,seconds:1}).status,0);
  assert.equal(puts,1);
});

test('--recipe runs another recipe of one project in its gate slot, never through a receipt', t => {
  const root = mkdtempSync(resolve(tmpdir(), 'gate-recipe-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const project = { dir:'tool', steps:[],
    run:{ kind:'root-just', recipe:'_gate-tool', cargo:{ targetDir:'/tmp/tool-gate-target', rustflags:'' } } };
  mkdirSync(resolve(root, 'tool'), { recursive:true });
  writeFileSync(resolve(root, 'tool/build-manifest.json'), JSON.stringify({ pipeline:'tool', steps:{}, gate:{ projects:{ tool:project } } }));
  mkdirSync(resolve(root, 'genesis/orchestrator'), { recursive:true });
  writeFileSync(resolve(root, 'genesis/orchestrator/run-local-gate.sh'), 'echo "RAN recipe=$5 target=$7"\n');
  const env = { ...process.env, GATE_ROOT:root, GATE_RUN_KIND:'pre-push' };
  for (const key of ['GIT_DIR', 'GIT_WORK_TREE', 'GIT_INDEX_FILE', 'GIT_PREFIX']) delete env[key];
  const cli = (...args) => spawnSync(process.execPath, [resolve(ROOT, 'genesis/orchestrator/gate-runner.mjs'), '--target', 'tool', ...args],
    { cwd:root, env, encoding:'utf8', timeout:15000 });

  const run = cli('--recipe', '_tool-deliver');
  assert.equal(run.status, 0, run.stderr);
  assert.match(run.stdout, /\[gate\] tool \(tool\) — recipe _tool-deliver/);
  assert.match(run.stdout, /RAN recipe=_tool-deliver target=\/tmp\/tool-gate-target/);
  assert.doesNotMatch(run.stdout, /receipt|evidence/);

  const printed = JSON.parse(cli('--print').stdout.trim().split('\n')[0]);
  assert.equal(printed.resolvedTargetDir, '/tmp/tool-gate-target');

  for (const bad of [['--recipe'], ['--recipe', 'x; rm -rf /']]) {
    const refused = cli(...bad);
    assert.equal(refused.status, 2, `${bad.join(' ')} must refuse`);
    assert.doesNotMatch(refused.stdout, /RAN/);
  }
});
