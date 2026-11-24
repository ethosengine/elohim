import { gateEvidence, recordEvidence } from './gate-evidence.mjs';
import { after, describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { chmodSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { loadGateRegistry } from './pipeline-registry.mjs';
import { discoverManifests, loadManifests } from './manifest-utils.mjs';
import {
  appendGateReadings,
  costLineAndDefer,
  emitGateReading,
  gateChildEnv,
  gateReadingRow,
  oracleMode,
  projectsForChanges,
  runProject,
  selectGateProjects,
  worktreeTargetDir,
} from './gate-runner.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

describe('gate step reading collector', () => {
  const base = {
    runId: '20261124T120000Z-deadbeef-123abc',
    step: 'fixture',
    key: 'reason says \"quoted\" and \\ escaped',
    keyChanged: 'no',
    status: 'green',
    seconds: 4.25,
    waitedSeconds: 1.5,
    leaseWaitedSeconds: 0.75,
    synthesized: false,
    startedAt: '2026-11-24T12:00:00.000Z',
    endedAt: '2026-11-24T12:00:06.000Z',
  };

  for (const [outcome, seconds, status] of [['ran', 4.25, 'green'], ['reused', 0.125, 'green'], ['skipped', 0.05, 'none']]) {
    test(`builds the native ${outcome} JSON row`, () => {
      const row = gateReadingRow({ ...base, outcome, seconds, status, at: base.endedAt });
      assert.deepEqual(row, {
        measure: 'gate-step-reading@1',
        value: seconds,
        unit: 'seconds',
        run_id: base.runId,
        step: 'fixture',
        key: base.key,
        key_changed: 'no',
        status,
        outcome,
        waited_seconds: 1.5,
        lease_waited_seconds: 0.75,
        synthesized: false,
        started_at: base.startedAt,
        ended_at: base.endedAt,
        at: base.endedAt,
      });
    });
  }

  test('collects in memory and appends every row with one bounded native call', () => {
    const collector = [];
    emitGateReading({ ...base, outcome: 'ran', at: base.endedAt }, { collector, env: {} });
    emitGateReading({ ...base, step: 'second', outcome: 'skipped', at: base.endedAt }, { collector, env: {} });
    let call;
    appendGateReadings(collector, {
      root: ROOT,
      env: { EPR_BIN: 'fixture-epr' },
      spawn: (command, args, options) => {
        call = { command, args, options, rows: readFileSync(args[args.indexOf('--from') + 1], 'utf8') };
        return { status: 0 };
      },
    });
    assert.equal(call.command, 'timeout');
    assert.deepEqual(call.args.slice(0, 7), ['-k', '1', '5', 'fixture-epr', 'flow', 'reading', 'append']);
    const rows = call.rows.trim().split('\n').map(line => JSON.parse(line));
    assert.equal(rows.length, 2);
    assert.equal(rows[0].key, 'reason says "quoted" and \\ escaped');
  });

  test('a failed native append is one warning and never throws', () => {
    const warnings = [];
    assert.doesNotThrow(() => appendGateReadings([gateReadingRow({ ...base, outcome: 'ran' })], {
      root: ROOT,
      env: { EPR_BIN: 'fixture-epr' },
      spawn: () => ({ status: 9 }),
      warn: line => warnings.push(line),
    }));
    assert.deepEqual(warnings, ['[gate-reading] run readings dropped (exit 9)\n']);
  });

  test('a contended native append is one drop line carrying its reason', () => {
    const warnings = [];
    appendGateReadings([gateReadingRow({ ...base, outcome: 'ran' })], {
      root: ROOT,
      env: { EPR_BIN: 'fixture-epr' },
      spawn: () => ({
        status: 3,
        stderr: 'reading append dropped 1 row(s): the device-local reading lock stayed busy\n',
      }),
      warn: line => warnings.push(line),
    });
    assert.deepEqual(warnings, [
      '[gate-reading] run readings dropped (exit 3: reading append dropped 1 row(s): the device-local reading lock stayed busy)\n',
    ]);
  });

  test('under a push with no scratch file, runners collect nothing and never append', () => {
    const collector = [];
    const env = { GATE_READING_PUSH: '1', GATE_READING_RUN_ID: 'push' };
    emitGateReading({ ...base, outcome: 'ran', at: base.endedAt }, { collector, env });
    assert.deepEqual(collector, []);
    let spawned = 0;
    const warnings = [];
    appendGateReadings([gateReadingRow({ ...base, outcome: 'ran' })], {
      root: ROOT,
      env,
      spawn: () => { spawned += 1; return { status: 0 }; },
      warn: line => warnings.push(line),
    });
    assert.equal(spawned, 0, 'the hook owns the one append under a push');
    assert.deepEqual(warnings, [], 'the hook, not each runner, reports the drop');
  });

  test('unwritable and unremovable telemetry scratch never throws', () => {
    const row = gateReadingRow({ ...base, outcome: 'ran' });
    for (const overrides of [
      { makeScratch: () => { throw new Error('cannot create'); } },
      { writeRows: () => { throw new Error('cannot write'); } },
      { removeScratch: () => { throw new Error('cannot remove'); } },
    ]) {
      assert.doesNotThrow(() => appendGateReadings([row], {
        root: ROOT,
        env: { EPR_BIN: 'fixture-epr' },
        spawn: () => ({ status: 0 }),
        warn: () => {},
        ...overrides,
      }));
    }
  });

  test('an unwritable pre-push scratch file cannot change the project verdict', () => {
    const project = { name: 'fixture', dir: 'fixture', steps: ['build'], run: { kind: 'just', recipe: 'gate' } };
    const status = runProject(project, false, false, {
      root: ROOT,
      env: { GATE_READING_ROWS: '/unwritable/rows.jsonl', GATE_READING_RUN_ID: 'push' },
      spawn: () => ({ status: 23, stdout: '', stderr: '' }),
      write: () => {},
    });
    assert.equal(status, 23);
  });

  test('GATE_READING=off suppresses collection and the native call', () => {
    let spawned = false;
    const warnings = [];
    const collector = [];
    emitGateReading({ ...base, outcome: 'ran' }, {
      collector,
      env: { GATE_READING: 'off' },
    });
    appendGateReadings(collector, {
      root: ROOT,
      env: { GATE_READING: 'off' },
      spawn: () => { spawned = true; },
      warn: line => warnings.push(line),
    });
    assert.deepEqual(collector, []);
    assert.equal(spawned, false);
    assert.deepEqual(warnings, []);
  });
});

// projectsForChanges re-walks the whole tree for build-manifest.json on every call (~1 s here,
// dozens of calls). A selection that asks no oracle of the tree (path mode, or an injected rakia)
// reads nothing but those manifests, so such calls read MANIFESTS: this repository's manifests
// copied once, byte for byte, at their own paths, and held equal to the real walk below. Calls
// that may ask the real rakia keep ROOT.
const MANIFESTS = mkdtempSync(resolve(tmpdir(), 'gate-runner-manifests-'));
for (const path of discoverManifests(ROOT)) {
  mkdirSync(dirname(resolve(MANIFESTS, path)), { recursive: true });
  writeFileSync(resolve(MANIFESTS, path), readFileSync(resolve(ROOT, path)));
}
after(() => rmSync(MANIFESTS, { recursive: true, force: true }));

test("the manifest mirror is this repository's build graph", () => {
  assert.deepStrictEqual(loadManifests(MANIFESTS), loadManifests(ROOT));
});

describe('gate expectations and feature-branch CI deferral', () => {
  const project = { name: 'fixture', dir: 'fixture', steps: ['build'], run: { kind: 'just', recipe: 'gate' } };
  const rows = Array.from({ length: 10 }, (_, i) => ({
    project: project.name, subject: 'fixture/justfile', value: 660,
    kind: 'full-gate', exit: 'ok', occurredAt: new Date(Date.UTC(2026, 9, 1, 0, i)).toISOString(),
  }));
  const pushEnv = {
    GATE_RUN_KIND: 'pre-push',
    GATE_PUSH_TARGETS: ' refs/heads/feat-cost\trefs/heads/feat-other ',
    GATE_PUSH_BRANCHES: ' feat-cost\nfeat-other ',
  };
  const expected = '[gate] fixture: expect ~11 min (p90 11 min, max 11 min; 10 runs) · failed locally 0 of last 10';

  function fixture(t, { samples = rows, projects = [project], eprStatus = 0 } = {}) {
    const root = mkdtempSync(resolve(tmpdir(), 'gate-cost-runner-'));
    t.after(() => rmSync(root, { recursive: true, force: true }));
    for (const p of projects) {
      mkdirSync(resolve(root, p.dir), { recursive: true });
      writeFileSync(resolve(root, p.dir, 'justfile'), 'gate:\n');
    }
    writeFileSync(resolve(root, 'build-manifest.json'), JSON.stringify({
      pipeline: 'fixture', steps: { build: {} },
      gate: { projects: Object.fromEntries(projects.map(p => [p.name, p])) },
    }));
    mkdirSync(resolve(root, '.eprfs/status'), { recursive: true });
    const flowsPath = resolve(root, '.eprfs/status/flows.jsonl');
    writeFileSync(flowsPath, samples.map(row => JSON.stringify({ cid: row.cid, record: {
      occurredAt: row.occurredAt,
      classifiedAs: ['run:observation', row.subject, 'measure:gate-cycle-seconds@1',
        `value:${row.value}`, `env:run-kind=${row.kind}`, `env:exit=${row.exit}`,
        ...(row.project === undefined ? [] : [`env:project=${row.project}`])],
    } })).join('\n'));
    return {
      root,
      flowsPath,
      run(gate = project, env = pushEnv, printOnly = false, namesOnly = false) {
        const calls = [];
        const readings = [];
        let output = '';
        const status = runProject(gate, printOnly, namesOnly, {
          root, env: { PATH: '', EPR_BIN: 'fixture-epr', ...env },
          readingCollector: readings,
          write: text => { output += text; },
          spawn: (command, args) => {
            calls.push({ command, args });
            return { status: command === 'fixture-epr' ? eprStatus : 7, stdout: '', stderr: '' };
          },
        });
        return { status, output, calls, readings };
      },
    };
  }

  test('the pure decision formats fixture costs and splits push refs on whitespace', () => {
    const decision = costLineAndDefer(project, { rows, env: pushEnv });
    assert.equal(decision.line, expected);
    assert.equal(decision.defer, true);
  });

  test('a qualifying feat push writes the expectation and returns zero without spawning the recipe or evidence', t => {
    const result = fixture(t).run();
    assert.equal(result.status, 0);
    assert.ok(result.output.startsWith(`\n[gate] fixture (fixture)\n${expected}\n`));
    assert.match(result.output, /\[gate\] fixture deferred to CI: feature-branch CI covers this push/);
    assert.doesNotMatch(result.output, /evidence:/);
    assert.equal(result.calls.length, 1, 'only the cycle observation is spawned');
    assert.equal(result.calls[0].command, 'fixture-epr');
    assert.deepEqual(result.calls[0].args.slice(0, 4), ['flow', 'note', '--kind', 'observation']);
    assert.ok(result.calls[0].args.includes('run-kind=deferred'));
    assert.ok(result.calls[0].args.includes('exit=ok'));
    assert.ok(result.calls[0].args.includes('project=fixture'));
    assert.ok(!result.calls[0].args.some(arg => /^(cpu_s|throttled_s|rss_peak_bytes|memory_high_events|io_full_pct_max)=/.test(arg)),
      'a deferred run has no measured cost');
  });

  test('collecting a reading cannot change a skipped project status', t => {
    const result = fixture(t, { eprStatus: 9 }).run(project, {
      ...pushEnv,
      GATE_READING_RUN_ID: 'fixture-run',
    });
    assert.equal(result.status, 0);
    assert.equal(result.readings.length, 1);
    assert.equal(result.readings[0].run_id, 'fixture-run');
    assert.equal(result.readings[0].outcome, 'skipped');
    assert.equal(result.calls.filter(call => call.command === 'fixture-epr').length, 1,
      'only the existing cycle observation is spawned by runProject');
  });

  test('readings on and off leave selection, reuse decision, verdict and exit identical', t => {
    const fx = fixture(t, { samples: [] });
    const selected = selectGateProjects(loadGateRegistry(fx.root), 'fixture');
    const run = reading => fx.run(selected[0], {
      ...pushEnv,
      GATE_DEFER: 'off',
      GATE_EVIDENCE: 'reuse',
      GATE_READING: reading,
      GATE_READING_RUN_ID: 'equivalence',
    });
    const on = run('on');
    const off = run('off');
    assert.deepEqual(selected.map(project => project.name), ['fixture']);
    assert.equal(on.status, off.status);
    assert.equal(on.output, off.output);
    assert.deepEqual(
      on.calls.map(call => [call.command, call.args]),
      off.calls.map(call => [call.command, call.args]),
    );
    assert.equal(on.readings.length, 1);
    assert.equal(off.readings.length, 0);
    assert.match(on.output, /not reused: project not enabled for reuse/);
  });

  for (const [name, gate, env, reason] of [
    ['a dev target', project, { ...pushEnv, GATE_PUSH_TARGETS: 'refs/heads/feat-cost refs/heads/dev', GATE_PUSH_BRANCHES: 'feat-cost dev' }, /refs\/heads\/dev requires local gates/],
    ['a main target', project, { ...pushEnv, GATE_PUSH_TARGETS: 'refs/heads/main', GATE_PUSH_BRANCHES: 'main' }, /refs\/heads\/main requires local gates/],
    ['empty pipeline steps', { ...project, steps: [] }, pushEnv, /no CI pipeline steps/],
    ['missing pipeline steps', { ...project, steps: undefined }, pushEnv, /no CI pipeline steps/],
    ['a full-gate run', project, { ...pushEnv, GATE_RUN_KIND: 'full-gate' }, /only pre-push runs defer/],
    ['an undeclared run kind', project, { ...pushEnv, GATE_RUN_KIND: undefined }, /only pre-push runs defer/],
    ['a tag push', project, { ...pushEnv, GATE_PUSH_TARGETS: 'refs/tags/v1', GATE_PUSH_BRANCHES: '' }, /no CI branch supplied/],
  ]) {
    test(`${name} never defers and prints no refusal noise`, t => {
      const decision = costLineAndDefer(gate, { rows, env });
      assert.equal(decision.defer, false);
      assert.match(decision.why, reason);
      const result = fixture(t).run(gate, env);
      assert.equal(result.status, 7, 'the recipe result is returned');
      assert.ok(result.output.includes(expected));
      assert.doesNotMatch(result.output, /deferred to CI/);
      assert.ok(!result.output.includes(decision.why));
      assert.ok(result.calls.some(call => call.command === 'bash' && call.args[0].endsWith('/run-local-gate.sh')));
      assert.ok(result.calls.some(call => call.command === 'fixture-epr' && call.args.includes('exit=fail')));
    });
  }

  test('cycle rows are read once for multiple projects in the same process', t => {
    const other = { ...project, name: 'other' };
    const f = fixture(t, { projects: [project, other], samples: [...rows, ...rows.map(row => ({ ...row, project: 'other' }))] });
    assert.equal(f.run().status, 0);
    rmSync(f.flowsPath);
    const result = f.run(other);
    assert.equal(result.status, 0);
    assert.ok(result.output.includes(expected.replace('fixture:', 'other:')));
  });

  test('main, sibling and current ledgers deduplicate rows and retain append order despite older author dates', t => {
    const main = fixture(t, { samples: Array.from({ length: 25 }, (_, i) => ({
      ...rows[0], cid: `main-${i}`, value: i < 15 ? 9000 : 660, exit: i < 5 ? 'fail' : 'ok',
      occurredAt: new Date(Date.UTC(2026, 9, 25, 0, 25 - i)).toISOString(),
    })) });
    const current = fixture(t, { samples: [
      { ...rows[0], value: 1200 },
      { ...rows[0], exit: 'fail' },
    ] });
    const commonDir = resolve(main.root, '.git');
    const gitDir = resolve(commonDir, 'worktrees/fixture');
    for (const dir of ['objects', 'refs', 'worktrees/fixture']) {
      mkdirSync(resolve(commonDir, dir), { recursive: true });
    }
    writeFileSync(resolve(commonDir, 'HEAD'), 'ref: refs/heads/main\n');
    writeFileSync(resolve(gitDir, 'HEAD'), 'ref: refs/heads/fixture\n');
    writeFileSync(resolve(gitDir, 'commondir'), '../..\n');
    writeFileSync(resolve(gitDir, 'gitdir'), `${resolve(current.root, '.git')}\n`);
    writeFileSync(resolve(current.root, '.git'), `gitdir: ${gitDir}\n`);

    const mainRows = readFileSync(main.flowsPath, 'utf8');
    const currentRows = readFileSync(current.flowsPath, 'utf8');
    const siblingPath = resolve(main.root, '.worktrees/sibling/.eprfs/status/flows.jsonl');
    mkdirSync(dirname(siblingPath), { recursive: true });
    const siblingRow = JSON.stringify({ cid: 'sibling-run', record: {
      occurredAt: '2026-09-30T00:00:00Z',
      classifiedAs: ['run:observation', 'fixture/justfile', 'measure:gate-cycle-seconds@1',
        'value:1800', 'env:run-kind=full-gate', 'env:exit=ok', 'env:project=fixture'],
    } });
    writeFileSync(siblingPath, `${mainRows}\n${siblingRow}\n${siblingRow}`);
    writeFileSync(current.flowsPath, `${mainRows}\n${siblingRow}\n${currentRows}\n${currentRows}`);

    const result = current.run();
    assert.ok(result.output.includes('[gate] fixture: expect ~14 min (p90 20 min, max 30 min; 10 runs) · failed locally 1 of last 20'), result.output);
    assert.equal(result.status, 7, 'the current checkout failure keeps the recipe local');
    assert.doesNotMatch(result.output, /deferred to CI/);
  });

  test('an unreadable sidecar stays silent and keeps the local recipe, without retrying', t => {
    const f = fixture(t);
    rmSync(f.flowsPath);
    const first = f.run();
    writeFileSync(f.flowsPath, '');
    for (const result of [first, f.run()]) {
      assert.equal(result.status, 7);
      assert.doesNotMatch(result.output, /expect ~|no recorded runs|deferred to CI/);
      assert.ok(result.calls.some(call => call.command === 'bash'));
    }
  });

  test('a readable empty sidecar prints no recorded runs and keeps the recipe', t => {
    const result = fixture(t, { samples: [] }).run();
    assert.equal(result.status, 7);
    assert.match(result.output, /\[gate\] fixture: no recorded runs yet/);
  });

  test('an empty sibling sidecar is readable history even when the invoking ledger is absent', t => {
    const f = fixture(t);
    rmSync(f.flowsPath);
    const siblingPath = resolve(f.root, '.worktrees/sibling/.eprfs/status/flows.jsonl');
    mkdirSync(dirname(siblingPath), { recursive: true });
    writeFileSync(siblingPath, '');
    const result = f.run();
    assert.equal(result.status, 7);
    assert.match(result.output, /\[gate\] fixture: no recorded runs yet/);
  });

  test('legacy subjects count only when no other registered project records that subject', t => {
    for (const dir of ['fixture', '.']) {
      const gate = { ...project, dir };
      const samples = rows.map(row => ({ ...row, project: undefined, subject: dir === '.' ? '.' : 'fixture/justfile' }));
      const unique = fixture(t, { samples, projects: [gate] }).run(gate);
      assert.equal(unique.status, 0);
      assert.ok(unique.output.includes(expected));
      const shared = fixture(t, { samples, projects: [gate, { ...gate, name: 'other' }] }).run(gate);
      assert.equal(shared.status, 7);
      assert.match(shared.output, /no recorded runs yet/);
    }
  });

  test('print and names modes stay free of expectation and deferral output', t => {
    const f = fixture(t);
    for (const [printOnly, namesOnly] of [[true, false], [false, true]]) {
      const result = f.run(project, pushEnv, printOnly, namesOnly);
      assert.equal(result.status, 0);
      assert.deepEqual(result.calls, []);
      assert.doesNotMatch(result.output, /expect ~|no recorded runs|deferred to CI/);
      if (namesOnly) assert.equal(result.output, 'fixture\n');
      else assert.equal(JSON.parse(result.output).name, 'fixture');
    }
  });
});

test('Rust style selects new crates and inherited lint configuration', () => {
  for (const changed of [
    'new-crate/src/lib.rs', '.cargo/config.toml', 'some/crate/clippy.toml',
    '.epr-meta/rust-style-policy.json', '.epr-meta/rust-style/objects/policy.json',
    'genesis/docs/rust-style-guide.md', 'genesis/orchestrator/rust-style/source-distribution.py',
    'some/crate/.epr-meta/manifest.md',
  ]) {
    for (const oracle of ['path', 'rakia']) {
      const selected = projectsForChanges(MANIFESTS, [changed], { oracle, ask: () => new Map() }).map(p => p.name);
      assert.ok(selected.includes('rust-style'), `${changed} must select the style floor (${oracle})`);
    }
  }
});

// S6: the checker's own legs are a separate project. A checker change selects both; a Rust change
// elsewhere selects only the always-run scan, whose baseline moves with origin/dev.
test('a checker change selects rust-style-checker; a Rust change elsewhere selects only the scan', () => {
  const select = (changed, oracle) => projectsForChanges(MANIFESTS, [changed], { oracle, ask: () => new Map() }).map(p => p.name);
  for (const oracle of ['path', 'rakia']) {
    for (const changed of [
      'genesis/orchestrator/rust-style/src/lib.rs', 'genesis/orchestrator/rust-style/tests/ratchet.rs',
      'genesis/orchestrator/rust-style/source-distribution.py', 'genesis/orchestrator/rust-style.sh',
      'genesis/orchestrator/rust-style-build-env.sh', '.cargo/config.toml',
    ]) {
      const selected = select(changed, oracle);
      assert.ok(selected.includes('rust-style-checker'), `${changed} must select rust-style-checker (${oracle})`);
      assert.ok(selected.includes('rust-style'), `${changed} must still select the scan (${oracle})`);
    }
    for (const changed of ['elohim/eprfs/epr-cli/src/main.rs', 'some/crate/src/lib.rs', 'genesis/docs/rust-style-guide.md']) {
      const selected = select(changed, oracle);
      assert.ok(selected.includes('rust-style'), `${changed} must select the scan (${oracle})`);
      assert.ok(!selected.includes('rust-style-checker'), `${changed} must not select rust-style-checker (${oracle})`);
    }
  }
});

test('the rust-style-checker step adds no CI dispatch: lint-jenkinsfiles already watches every source', () => {
  const manifest = JSON.parse(readFileSync(resolve(ROOT, 'genesis/orchestrator/build-manifest.json'), 'utf8'));
  const step = manifest.steps['rust-style-checker'];
  const lint = manifest.steps['lint-jenkinsfiles'];
  assert.ok(lint.inputs.sources.includes('genesis/orchestrator/**'));
  for (const source of [...step.inputs.sources, ...step.inputs.buildProcess]) {
    assert.ok(source.startsWith('genesis/orchestrator/'), `${source} would widen elohim-orchestrator's dispatch`);
  }
  // Same run class as the step it shadows, and nothing downstream depends on it.
  assert.equal(step.class, lint.class);
  for (const { content } of loadManifests(ROOT)) {
    for (const [name, other] of Object.entries(content.steps || {})) {
      assert.ok(!(other.depends || []).some(d => d === 'rust-style-checker' || d.endsWith(':rust-style-checker')),
        `${content.pipeline}:${name} depends on the local-only checker step`);
    }
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
    const selected = projectsForChanges(MANIFESTS, ['elohim/rakia'], { oracle: 'path', log: () => {} }).map(p => p.name);
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

  // A test's result must not depend on which brit the caller's shell names (BRIT_BIN or PATH): these
  // ask the manifests alone, through the path walk, with no brit at all.
  const noBrit = { env: { BRIT_BIN: '/nonexistent/brit', PATH: '' }, oracle: 'path', log: () => {}, inputsLog: () => {} };

  test('representative diffs select manifest and gate-only projects', () => {
    const epr = projectsForChanges(ROOT, ['elohim/sdk/epr-ts/src/index.ts'], noBrit).map(p => p.name);
    assert.ok(epr.includes('epr-ts'));
    assert.ok(!epr.includes('elements-codegen'), 'gate-only inputs must not default to every pipeline step');

    const app = projectsForChanges(ROOT, ['app/elohim-library/projects/foo.ts'], noBrit).map(p => p.name);
    assert.ok(app.includes('elohim-library'));
    assert.ok(app.includes('elohim-storybook'));

    const seam = projectsForChanges(ROOT, ['crates/seam-contracts/src/lib.rs'], noBrit).map(p => p.name);
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
      const selected = projectsForChanges(ROOT, [changed], noBrit).map(project => project.name);
      assert.ok(selected.includes(expected), `${changed} must select ${expected}`);
    }
  });


  test('names-only CLI returns an empty list for a documentation-only change', () => {
    const command = [resolve(ROOT, 'genesis/orchestrator/gate-runner.mjs'), '--changed-file-list'];
    const input = 'genesis/data/timeline/backlog/CLUSTERS.md\n';
    const names = spawnSync(process.execPath, [...command, '--names'], {
      cwd: ROOT, encoding: 'utf8', input, env: { ...process.env, BRIT_BIN: '/nonexistent/brit' },
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
      '{"RUSTUP_TOOLCHAIN":"1.100.0","CARGO_BUILD_JOBS":"2","RUST_TEST_THREADS":"2"}'
    );
    // A project with no declared env gets only the pool-policy toolchain pin, and must not inherit a stale
    // GATE_CARGO_ENV from the parent environment.
    assert.equal(gateChildEnv(bare, { GATE_CARGO_ENV: '{"X":"1"}' }).GATE_CARGO_ENV, '{"RUSTUP_TOOLCHAIN":"1.100.0"}');
    assert.equal(Object.hasOwn(gateChildEnv(none, {}), 'GATE_CARGO_ENV'), false);
    assert.equal(Object.hasOwn(gateChildEnv(declared, { cargoEnv: undefined, PATH: '/bin' }), 'PATH'), true);
    // Push-scoped git config (`git -c …`) never reaches the temp repositories a gate creates.
    const scrubbed = gateChildEnv(none, {
      PATH: '/bin', GIT_CONFIG_PARAMETERS: "'core.hookspath'='/x/.husky/_'",
      GIT_CONFIG_COUNT: '1', GIT_CONFIG_KEY_0: 'core.hooksPath', GIT_CONFIG_VALUE_0: '/x', GIT_CONFIG_GLOBAL: '/g',
    });
    for (const k of ['GIT_CONFIG_PARAMETERS', 'GIT_CONFIG_COUNT', 'GIT_CONFIG_KEY_0', 'GIT_CONFIG_VALUE_0']) {
      assert.equal(Object.hasOwn(scrubbed, k), false, k);
    }
    assert.equal(scrubbed.GIT_CONFIG_GLOBAL, '/g', 'a global config file location is not push-scoped');

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
    // The rakia schema accepts run.cargo.env since 2b2cedb, and every capped project now
    // declares its cap on its own manifest project (storage first; eprfs and memory-ceremony
    // on 2026-11-22, at four test threads). pool-policy's cargo_env_overrides remain as the
    // place a cap can be filled for a project that has not declared one, so the merge rule is
    // proved on a fixture policy: the manifest's own declaration always wins; the policy
    // override only fills a gap the manifest leaves undeclared.
    const fixture = mkdtempSync(resolve(tmpdir(), 'gate-cargo-env-'));
    mkdirSync(resolve(fixture, 'genesis/agentic'), { recursive: true });
    writeFileSync(resolve(fixture, 'genesis/agentic/pool-policy.json'), JSON.stringify({
      cargo_env_overrides: {
        '*': { RUSTUP_TOOLCHAIN: '1.100.0' },
        'still-in-policy': { CARGO_BUILD_JOBS: '1', RUST_TEST_THREADS: '1' },
      },
    }));
    const noManifestEnv = { name: 'still-in-policy', run: { cargo: { targetDir: '/tmp/x' } } };
    const filled = gateChildEnv(noManifestEnv, {}, fixture);
    assert.ok(Object.hasOwn(filled, 'GATE_CARGO_ENV'), 'pool-policy override applies when the manifest declares no cargo.env');
    assert.deepStrictEqual(JSON.parse(filled.GATE_CARGO_ENV), { CARGO_BUILD_JOBS: '1', RUST_TEST_THREADS: '1', RUSTUP_TOOLCHAIN: '1.100.0' });

    const withManifestEnv = {
      name: 'still-in-policy',
      run: { cargo: { targetDir: '/tmp/x', env: { CARGO_BUILD_JOBS: '3' } } },
    };
    const won = gateChildEnv(withManifestEnv, {}, fixture);
    assert.deepStrictEqual(
      JSON.parse(won.GATE_CARGO_ENV),
      { CARGO_BUILD_JOBS: '3', RUST_TEST_THREADS: '1', RUSTUP_TOOLCHAIN: '1.100.0' },
      'a manifest-declared cargo.env value wins over the pool-policy override; undeclared keys are still filled'
    );

    // This tree: the two gates that run the eprfs workspace's tests declare the same caps on their
    // own manifests, and the policy carries no per-project cap that could shadow or disagree.
    for (const name of ['eprfs', 'memory-ceremony']) {
      const env = JSON.parse(gateChildEnv(registry.get(name), {}).GATE_CARGO_ENV);
      assert.deepStrictEqual(
        { jobs: env.CARGO_BUILD_JOBS, threads: env.RUST_TEST_THREADS }, { jobs: '6', threads: '4' },
        `${name} declares its build jobs and test threads on its manifest`);
    }
    const realOverrides = JSON.parse(readFileSync(resolve(ROOT, 'genesis/agentic/pool-policy.json'), 'utf8')).cargo_env_overrides;
    assert.deepStrictEqual(Object.keys(realOverrides), ['*'],
      'no per-project cap is left in pool-policy; a cap lives on the gate project it caps');

    const storage = registry.get('elohim-storage');
    assert.deepStrictEqual(JSON.parse(gateChildEnv(storage, {}).GATE_CARGO_ENV), { CARGO_BUILD_JOBS: '1', RUSTUP_TOOLCHAIN: '1.100.0' },
      'the storage cap is declared on its manifest, no longer in pool-policy');

    const neither = { name: 'not-a-real-project-xyz', run: { cargo: { workspace: 'x' } } };
    assert.deepStrictEqual(JSON.parse(gateChildEnv(neither, {}).GATE_CARGO_ENV), { RUSTUP_TOOLCHAIN: '1.100.0' },
      'the "*" pool-policy entry pins the local gate toolchain for every cargo project');
    assert.equal(Object.hasOwn(gateChildEnv({ name: 'not-a-real-project-xyz', run: {} }, {}), 'GATE_CARGO_ENV'), false,
      'a project without run.cargo gets no toolchain pin');
    assert.equal(
      JSON.parse(gateChildEnv({ name: 'x', run: { cargo: { workspace: 'x', env: { RUSTUP_TOOLCHAIN: '1.94.0' } } } }, {}).GATE_CARGO_ENV).RUSTUP_TOOLCHAIN,
      '1.94.0', 'a manifest may override the pinned toolchain per project');
    assert.equal(
      JSON.parse(gateChildEnv(neither, { RUSTUP_TOOLCHAIN: 'nightly' }).GATE_CARGO_ENV).RUSTUP_TOOLCHAIN,
      '1.100.0', 'the declared pin is exported over the caller environment, like any other cargo env key');
  });

  // Row 29.2 (backlog steward-device-arrival-open-items.md row 29): the gate builds C at the
  // Rust target's declared baseline. RHEL 10 gcc defaults to -march=x86-64-v3 and cc 1.2.65
  // passes no -march, so ring's curve25519.c carried AVX2 and a v2 sibling trapped (SIGILL).
  const CFLAGS_KEY = 'CFLAGS_x86_64_unknown_linux_gnu';
  const CXXFLAGS_KEY = 'CXXFLAGS_x86_64_unknown_linux_gnu';
  const declaredTargetsRoot = (extra = {}) => {
    const root = mkdtempSync(resolve(tmpdir(), 'gate-declared-targets-'));
    mkdirSync(resolve(root, 'genesis/agentic'), { recursive: true });
    writeFileSync(resolve(root, 'genesis/agentic/pool-policy.json'), JSON.stringify({
      cargo_env_overrides: { '*': { RUSTUP_TOOLCHAIN: '1.100.0' }, ...(extra.overrides || {}) },
      declared_targets: {
        _comment: 'fixture',
        'x86_64-unknown-linux-gnu': {
          isa: 'x86-64', cflags: '-march=x86-64', env: { CC_ENABLE_DEBUG_OUTPUT: '1' }, workspaces: ['elohim/eprfs'],
        },
      },
    }));
    return root;
  };

  // E12 L2a rework, ruling 1: the record's `env` is exported beside the two flags, so cc logs every
  // compile command for put to read; projects off the listed workspaces get none of it.
  test('a declared target exports its env beside the C flags, on its listed workspaces only', () => {
    const root = declaredTargetsRoot();
    try {
      const eprfs = { name: 'fixture-eprfs', run: { cargo: { workspace: 'elohim/eprfs' } } };
      const env = JSON.parse(gateChildEnv(eprfs, {}, root).GATE_CARGO_ENV);
      assert.equal(env.CC_ENABLE_DEBUG_OUTPUT, '1');
      const storage = { name: 'fixture-storage', run: { cargo: { workspace: 'elohim/elohim-storage' } } };
      assert.equal(Object.hasOwn(JSON.parse(gateChildEnv(storage, {}, root).GATE_CARGO_ENV), 'CC_ENABLE_DEBUG_OUTPUT'), false);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test('a declared target puts its C baseline on projects of its listed workspaces only', () => {
    const root = declaredTargetsRoot();
    try {
      const eprfs = { name: 'fixture-eprfs', run: { cargo: { workspace: 'elohim/eprfs' } } };
      const env = JSON.parse(gateChildEnv(eprfs, {}, root).GATE_CARGO_ENV);
      assert.equal(env[CFLAGS_KEY], '-march=x86-64');
      assert.equal(env[CXXFLAGS_KEY], '-march=x86-64');
      assert.equal(env.RUSTUP_TOOLCHAIN, '1.100.0', 'the record adds to the merged cargo env, it replaces nothing');

      const storage = { name: 'fixture-storage', run: { cargo: { workspace: 'elohim/elohim-storage', env: { CARGO_BUILD_JOBS: '1' } } } };
      const stored = JSON.parse(gateChildEnv(storage, {}, root).GATE_CARGO_ENV);
      assert.deepStrictEqual(stored, { RUSTUP_TOOLCHAIN: '1.100.0', CARGO_BUILD_JOBS: '1' },
        'a project off the listed workspaces gets nothing new (cc 1.11.1 already pushes -march there)');
      assert.equal(Object.hasOwn(gateChildEnv({ name: 'js', run: {} }, {}, root), 'GATE_CARGO_ENV'), false,
        'a project without run.cargo gets no C flags');
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test('a manifest or override that sets a declared-target key does not silently win', () => {
    // Precedence: the record's value is kept (the baseline is a property of the binary every
    // sibling runs, not a per-project tuning knob) and the conflict is printed, never dropped.
    const root = declaredTargetsRoot({ overrides: { 'fixture-override': { [CXXFLAGS_KEY]: '-march=x86-64-v3' } } });
    try {
      const warnings = [];
      const warn = line => warnings.push(line);
      const manifest = { name: 'fixture-manifest', run: { cargo: { workspace: 'elohim/eprfs', env: { [CFLAGS_KEY]: '-march=native' } } } };
      const env = JSON.parse(gateChildEnv(manifest, {}, root, { warn }).GATE_CARGO_ENV);
      assert.equal(env[CFLAGS_KEY], '-march=x86-64', 'the declared target value is kept over the manifest');
      assert.equal(warnings.length, 1, warnings.join('\n'));
      assert.match(warnings[0], /fixture-manifest/);
      assert.match(warnings[0], /CFLAGS_x86_64_unknown_linux_gnu/);
      assert.match(warnings[0], /-march=native/);
      assert.match(warnings[0], /declared_targets/);

      warnings.length = 0;
      const override = { name: 'fixture-override', run: { cargo: { workspace: 'elohim/eprfs' } } };
      const env2 = JSON.parse(gateChildEnv(override, {}, root, { warn }).GATE_CARGO_ENV);
      assert.equal(env2[CXXFLAGS_KEY], '-march=x86-64', 'the declared target value is kept over a pool-policy override');
      assert.equal(warnings.length, 1, warnings.join('\n'));
      assert.match(warnings[0], /-march=x86-64-v3/);

      warnings.length = 0;
      const agreeing = { name: 'fixture-agree', run: { cargo: { workspace: 'elohim/eprfs', env: { [CFLAGS_KEY]: '-march=x86-64' } } } };
      gateChildEnv(agreeing, {}, root, { warn });
      assert.deepStrictEqual(warnings, [], 'an agreeing value is not a conflict');
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test('pool-policy declares the x86_64 C baseline record in its decided shape', () => {
    const policy = JSON.parse(readFileSync(resolve(ROOT, 'genesis/agentic/pool-policy.json'), 'utf8'));
    const { _comment, ...targets } = policy.declared_targets || {};
    assert.equal(typeof _comment, 'string');
    assert.match(_comment, /29\.2/, 'the comment names the decision row');
    assert.deepStrictEqual(targets, {
      'x86_64-unknown-linux-gnu': {
        isa: 'x86-64', cflags: '-march=x86-64', env: { CC_ENABLE_DEBUG_OUTPUT: '1' }, workspaces: ['elohim/eprfs'],
      },
    });
    assert.equal(Object.hasOwn(policy.cargo_env_overrides, 'declared_targets'), false,
      'the record is a top-level key, never inside cargo_env_overrides (hc-mesh.sh reads that map)');
  });

  test('every real gate project on the eprfs workspace gets the C baseline, and only those', () => {
    const onEprfs = [...registry.values()].filter(p => p.run.cargo && p.run.cargo.workspace === 'elohim/eprfs');
    assert.deepStrictEqual(onEprfs.map(p => p.name).sort(), ['eprfs', 'eprfs-live-corpus', 'memory-ceremony', 'orchestrator-tree']);
    for (const project of onEprfs) {
      const env = JSON.parse(gateChildEnv(project, {}).GATE_CARGO_ENV);
      assert.equal(env[CFLAGS_KEY], '-march=x86-64', project.name);
      assert.equal(env[CXXFLAGS_KEY], '-march=x86-64', project.name);
      assert.equal(env.CC_ENABLE_DEBUG_OUTPUT, '1', project.name);
    }
    for (const project of registry.values()) {
      if (onEprfs.includes(project)) continue;
      const carried = gateChildEnv(project, {}).GATE_CARGO_ENV;
      const env = carried ? JSON.parse(carried) : {};
      assert.equal(Object.hasOwn(env, CFLAGS_KEY), false, project.name);
      assert.equal(Object.hasOwn(env, CXXFLAGS_KEY), false, project.name);
      assert.equal(Object.hasOwn(env, 'CC_ENABLE_DEBUG_OUTPUT'), false, project.name);
    }
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
    const names = projectsForChanges(MANIFESTS, ['sophia'], { oracle: 'path', ask: withOracle, log: () => {} }).map(p => p.name);
    assert.ok(names.includes('sophia'));
    assert.ok(!names.includes('elohim-app'), 'path-only never propagates');
  });

  test('shadow mode selects by path and prints one oracle-diff line when the sets differ', () => {
    const lines = [];
    const names = projectsForChanges(MANIFESTS, ['sophia'], { oracle: 'shadow', ask: withOracle, log: l => lines.push(l) }).map(p => p.name);
    assert.ok(!names.includes('elohim-app'), 'shadow mode does not change selection');
    assert.equal(lines.length, 1);
    assert.match(lines[0], /^\[gate\] oracle-diff: \+elohim-app/);
  });

  test('shadow mode is silent when the sets agree', () => {
    const lines = [];
    // A documentation-only change selects nothing by path (proven below by the names-only CLI
    // case); an oracle that also finds nothing stale must not print a diff.
    const selected = projectsForChanges(MANIFESTS, ['genesis/data/timeline/backlog/CLUSTERS.md'], { oracle: 'shadow', ask: () => new Map(), log: l => lines.push(l) });
    assert.deepEqual(selected, []);
    assert.deepEqual(lines, []);
  });

  test('rakia mode selects the direct component and its one-hop consumer with the upstream reason', () => {
    const projects = projectsForChanges(MANIFESTS, ['sophia'], { oracle: 'rakia', ask: withOracle, log: () => {} });
    const app = projects.find(p => p.name === 'elohim-app');
    assert.ok(app, 'elohim-app is a depth-one consumer of the sophia pin');
    assert.deepEqual(app.reasons, ['upstream: elohim-sophia:build-sophia-umd']);
    assert.ok(projects.some(p => p.name === 'sophia'));
  });

  test('rakia mode falls back to path selection and says so when the oracle is unavailable', () => {
    const lines = [];
    const names = projectsForChanges(MANIFESTS, ['sophia'], { oracle: 'rakia', ask: noOracle, log: l => lines.push(l) }).map(p => p.name);
    assert.ok(names.includes('sophia'));
    assert.ok(!names.includes('elohim-app'));
    assert.deepEqual(lines, ['[gate] brit build affected unavailable (no reason reported) — path-only selection']);
  });

  test('a gitlink path and files beneath it select the component once, reasons merged', () => {
    const projects = projectsForChanges(MANIFESTS, ['elohim/brit', 'elohim/brit/Cargo.toml'], { oracle: 'path', log: () => {} });
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
  function fixture(t, checkRuns) {
    const root = mkdtempSync(resolve(tmpdir(), 'gate-attested-refused-'));
    t.after(() => rmSync(root, { recursive: true, force: true }));
    const gh = resolve(root, 'gh');
    writeFileSync(gh, '#!/bin/sh\nexit 1\n');
    chmodSync(gh, 0o755);
    const project = {
      name: 'fixture-attested',
      dir: 'component',
      run: { kind: 'attested', attestation: { provider: 'github-checks', repo: 'example/component', check: 'Tests pass' } },
    };
    const sha = 'a'.repeat(40);
    const lines = [];
    const calls = [];
    const spawn = (command, args) => {
      calls.push({ command, args });
      if (command === 'git' && args.includes('ls-tree')) {
        return { status: 0, stdout: `160000 commit ${sha}\tcomponent\n`, stderr: '' };
      }
      if (command === 'git' && args.includes('status')) return { status: 0, stdout: '', stderr: '' };
      if (command === gh) return checkRuns();
      if (command === 'fixture-epr') return { status: 0, stdout: '', stderr: '' };
      return { status: 7, stdout: '', stderr: `unexpected ${command}` };
    };
    const run = () => runProject(project, false, false, {
      root,
      env: { GH_BIN: gh, EPR_BIN: 'fixture-epr' },
      spawn,
      write: text => lines.push(text),
    });
    return { calls, lines, run };
  }

  test('claimed is admitted with debt, named claimed, and never recorded as green evidence', t => {
    const f = fixture(t, () => ({ status: 1, stdout: '', stderr: 'offline' }));
    assert.equal(f.run(), 0);
    const output = f.lines.join('');
    assert.match(output, /attested, no local recipe/);
    assert.match(output, /attested: claimed — offline/);
    assert.match(output, /^\[gate\] fixture-attested: claimed \(admitted with debt; no green evidence\)$/m);
    assert.doesNotMatch(output, /\[gate\] fixture-attested (?:evidence: recorded|reused: green)/);
    assert.equal(f.calls.some(({ command, args }) => command.includes('brit') || args.includes('attest')), false,
      'a claim must not write reusable green evidence');
  });

  test('a witnessed refusal still makes the run non-zero', t => {
    const f = fixture(t, () => ({ status: 0, stdout: JSON.stringify({ check_runs: [] }), stderr: '' }));
    const status = f.run();
    const output = f.lines.join('');
    assert.notEqual(status, 0);
    assert.match(output, /attested: REFUSED/);
    assert.doesNotMatch(output, /^\[gate\] fixture-attested: claimed/m);
    assert.equal(f.calls.some(({ command, args }) => command.includes('brit') || args.includes('attest')), false,
      'a refusal must not write reusable green evidence either');
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
  // The shadow assertion needs an oracle that answers. It is a brit double, so the result never
  // depends on which brit (if any) the caller's shell names: it reports the sophia step changed and
  // the app build one hop upstream, and its `--help` has no `--inputs`, so GATE_INPUTS stays quiet.
  test('shadow mode: --names stdout is only names; the oracle-diff line goes to stderr', t => {
    const bins = mkdtempSync(resolve(tmpdir(), 'gate-runner-brit-'));
    t.after(() => rmSync(bins, { recursive: true, force: true }));
    const affected = JSON.stringify({ changed_paths: ['sophia'], affected: [
      { qualified_name: 'elohim-sophia:build-sophia-umd', affected_by: [{ kind: 'changedFile', path: 'sophia' }] },
      { qualified_name: 'elohim:build-angular', affected_by: [{ kind: 'upstreamNode', upstream: 'elohim-sophia:build-sophia-umd' }] },
    ] });
    writeFileSync(resolve(bins, 'brit'), `#!/bin/sh\ncase "$*" in\n  *--help*) echo 'Usage: brit build affected [OPTIONS]' ;;\n  'build affected '*) echo '${affected}' ;;\n  *) exit 1 ;;\nesac\n`);
    chmodSync(resolve(bins, 'brit'), 0o755);
    const out = spawnSync(process.execPath, [cli, '--changed-file-list', '--names'], {
      cwd: ROOT, encoding: 'utf8', input: 'sophia\n', env: { ...process.env, GATE_ORACLE: 'shadow', BRIT_BIN: resolve(bins, 'brit') },
    });
    assert.equal(out.status, 0, out.stderr);
    assert.equal(out.stdout.trim(), 'sophia');
    assert.match(out.stderr, /\[gate\] oracle-diff: \+elohim-app/);
  });
  test('rakia mode without a brit binary: --names stdout is only names; the fallback line goes to stderr', () => {
    const out = spawnSync(process.execPath, [cli, '--changed-file-list', '--names'], {
      cwd: ROOT, encoding: 'utf8', input: 'sophia\n', env: { ...process.env, GATE_ORACLE: 'rakia', BRIT_BIN: '/nonexistent/brit' },
    });
    assert.equal(out.status, 0, out.stderr);
    assert.equal(out.stdout.trim(), 'sophia');
    assert.match(out.stderr, /\[gate\] brit build affected unavailable \(no brit executable \(BRIT_BIN or PATH\)\) — path-only selection/);
  });
  test('shadow mode without a binary also says so, so the flip evidence cannot silently never arrive', () => {
    const lines = [];
    projectsForChanges(MANIFESTS, ['sophia'], { oracle: 'shadow', ask: () => null, log: l => lines.push(l) });
    assert.deepEqual(lines, ['[gate] brit build affected unavailable (no reason reported) — path-only selection']);
  });
  test('the real resolution path prints the oracle reason in the path-only line', () => {
    const lines = [];
    projectsForChanges(MANIFESTS, ['sophia'], { oracle: 'rakia', env: { BRIT_BIN: '/nonexistent/brit', PATH: '' }, log: l => lines.push(l) });
    assert.deepEqual(lines, ['[gate] brit build affected unavailable (no brit executable (BRIT_BIN or PATH)) — path-only selection']);
  });
  test('a failing brit build affected prints its stderr in the path-only line', () => {
    const lines = [];
    const spawn = () => ({ status: 3, stdout: '', stderr: 'bad repo\n' });
    projectsForChanges(MANIFESTS, ['sophia'], { oracle: 'rakia', env: { BRIT_BIN: process.execPath }, spawn, log: l => lines.push(l) });
    assert.deepEqual(lines, ['[gate] brit build affected unavailable (brit build affected exited 3: bad repo) — path-only selection']);
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


// A scratch eprfs-shaped project: its gate evidence carries the native source closure.
function receiptFixture(t) {
  const root = mkdtempSync(resolve(tmpdir(), 'gate-receipt-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const dir = 'elohim/eprfs';
  mkdirSync(resolve(root, dir), { recursive: true });
  writeFileSync(resolve(root, dir, 'build-manifest.json'),
    '{"steps":{"build":{"inputs":{"sources":["elohim/eprfs/**","crates/external/**"]}}}}\n');
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
  // A launcher older than the verb would fail every closure assertion below as if the code were wrong.
  const verb = spawnSync(env.EPR_BIN || 'epr', ['bin', 'source-state', '--help'], {env, encoding:'utf8'});
  assert.equal(verb.status, 0, `environment: ${env.EPR_BIN || 'epr on PATH'} has no \`epr bin source-state\`; ` +
    'set EPR_BIN to a bin-store build from dev (`epr bin --tool epr resolve --current` names one when the device head is current)');
  env.EPR_STORE = resolve(root, 'store');
  env.EPR_REPO_ROOT = root;
  env.EPR_BIN_TOOLS = resolve(root, 'genesis/agentic/bin-tools.json');
  // Slice M embeds this committed base in epr's source closure, even in a
  // minimal Cargo fixture. A missing blob must still make the evaluator refuse.
  const base = 'genesis/orchestrator/manifests/runtime/workspace-device.manifest.json';
  mkdirSync(dirname(resolve(root, base)), { recursive: true });
  writeFileSync(resolve(root, base), readFileSync(resolve(ROOT, base)));
  const metadata = spawnSync('cargo', ['metadata', '--offline', '--format-version', '1', '--manifest-path', resolve(root, dir, 'Cargo.toml')], {env, encoding:'utf8'});
  assert.equal(metadata.status, 0, metadata.stderr);
  // Closure key v3 reads the recipe's justfile and the gate runner at the commit.
  writeFileSync(resolve(root, 'justfile'), '_gate-fixture:\n    true\n');
  mkdirSync(resolve(root, 'genesis/orchestrator'), {recursive:true});
  writeFileSync(resolve(root, 'genesis/orchestrator/run-local-gate.sh'), 'exit 0\n');
  git(['init', '-q']); git(['add', dir, 'crates/external', 'genesis/agentic', 'genesis/orchestrator', 'justfile']); git(['commit', '-qm', 'fixture']);
  const project = { name:'fixture', dir, run:{kind:'root-just', recipe:'_gate-fixture', cargo:{env:{RUSTUP_TOOLCHAIN:'1.96.1'}}} };
  return {root, env, project, git};
}

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

test('a red run keeps its non-zero status when its same-device green note is kept', t => {
  const root = mkdtempSync(resolve(tmpdir(), 'gate-kept-green-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const bin = resolve(root, 'brit');
  writeFileSync(bin, '', { mode: 0o755 });
  const project = { name: 'fixture', dir: 'fixture', steps: ['build'],
    manifestPath: 'build-manifest.json', run: { kind: 'just', recipe: 'gate' } };
  const agentId = 'ab'.repeat(32);
  let key, puts = 0, output = '';
  const spawn = (command, args) => {
    const ok = stdout => ({ status: 0, stdout, stderr: '' });
    if (args[0] === 'rev-parse') return ok('a'.repeat(40));
    if (args[0] === 'show') return ok(JSON.stringify({ steps: { build: { inputs: { sources: ['fixture/**'] } } } }));
    if (args[0] === 'status' || args[0] === 'ls-tree') return ok('');
    if (args[1] === 'fingerprint') return ok(JSON.stringify({ fingerprints: [{ step: 'build', fingerprint: 'fp' }] }));
    if (args[3] === 'find') { key = args[args.indexOf('--inputs-hash') + 1]; return ok('[]'); }
    if (args[3] === 'get') return ok(JSON.stringify({ inputsHash: key, success: true, agentId, stepName: 'gate:fixture' }));
    if (args[2] === 'signer') return ok(JSON.stringify({ agentId }));
    if (args[3] === 'put') { puts++; return ok('written'); }
    return { status: 7, stdout: '', stderr: '' };
  };
  const status = runProject(project, false, false, { root, spawn,
    env: { PATH: '', BRIT_BIN: bin, BRIT_BUILD_REF_BIN: bin, GATE_TASK: 'off',
      GATE_EVIDENCE: 'off', GATE_INPUTS: 'off', GATE_RECIPE_KEY: 'off' },
    write: line => { output += line; }, warn: line => { output += line; } });
  assert.equal(status, 7);
  assert.equal(puts, 0);
  assert.match(output, /kept green for closure .*this run was red/);
  assert.doesNotMatch(output, /evidence: recorded/);
});

test('ark CLI and workspace lock are eprfs gate-owned inputs, not pipeline dispatch sources', () => {
  const manifest = JSON.parse(readFileSync(resolve(ROOT, 'elohim/eprfs/build-manifest.json'), 'utf8'));
  for (const path of ['elohim/ark/cli/**', 'elohim/Cargo.lock']) {
    assert.ok(manifest.gate.projects.eprfs.inputs.sources.includes(path));
    assert.ok(!manifest.steps['rust-build-test'].inputs.sources.includes(path));
  }
});
