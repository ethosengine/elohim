import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { loadGateRegistry } from './pipeline-registry.mjs';
import { gateChildEnv, projectsForChanges, selectGateProjects } from './gate-runner.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

describe('manifest-driven local gate registry', () => {
  const registry = loadGateRegistry(ROOT);

  test('every project has typed execution metadata', () => {
    assert.ok(registry.size >= 32);
    for (const project of registry.values()) {
      assert.ok(['just', 'root-just'].includes(project.run.kind), project.name);
      assert.match(project.run.recipe, /^_?[a-z][a-z0-9-]*$/);
    }
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
