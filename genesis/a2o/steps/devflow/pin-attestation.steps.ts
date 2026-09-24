/* eslint-disable sonarjs/no-os-command-from-path -- these steps deliberately drive `git` and the
   gate-runner as real processes over a scratch repository; the same posture as
   steps/conductor-spin.steps.ts and steps/compute-allocation.steps.ts */
/**
 * Drives genesis/orchestrator/gate-runner.mjs against a scratch superproject. One gitlink
 * (`comp`), two manifests, a fake `gh` whose answer is FAKE_GH_CONCLUSION, a fake `epr` that
 * appends its argv to a file so the observation can be asserted. Every scenario owns its
 * fixture and removes it afterwards.
 *
 * The component's manifest is committed INSIDE the sub-repository (the way sophia's lives in
 * sophia), so the superproject only ever commits the gitlink — exactly the real tree's shape.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

import { After, Given, Then, When } from '@cucumber/cucumber';

const REPO_ROOT = resolve(__dirname, '../../../..');
const GATE_RUNNER = join(REPO_ROOT, 'genesis/orchestrator/gate-runner.mjs');
const scratchBase = process.env.A2O_TMPDIR ?? tmpdir();

interface Fixture {
  base: string;
  root: string;
  sub: string;
  gitlink: string;
  bin: string;
  eprLog: string;
  conclusion: string;
  status?: number;
  stdout?: string;
}

let fx: Fixture | undefined;

function git(cwd: string, args: string[]): string {
  const r = spawnSync(
    'git',
    [
      '-c',
      'protocol.file.allow=always',
      '-c',
      'user.email=a2o@example.test',
      '-c',
      'user.name=a2o',
      '-c',
      'commit.gpgsign=false',
      ...args,
    ],
    { cwd, encoding: 'utf8' }
  );
  assert.equal(r.status, 0, `git ${args.join(' ')} in ${cwd}: ${r.stderr}`);
  return r.stdout.trim();
}

function writeJson(path: string, value: unknown): void {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function step(sources: string[], depends: string[] = []) {
  return {
    description: 's',
    inputs: { sources, buildProcess: [] },
    outputs: { artifacts: [], verify: null },
    depends,
    executor: { stage: 'S', function: null },
  };
}

Given(
  'a scratch superproject with a component pinned as the gitlink {string}',
  function (gitlink: string) {
    const base = mkdtempSync(join(scratchBase, 'pin-attestation-'));
    const sub = join(base, 'sub');
    mkdirSync(sub);
    git(sub, ['init', '-q', '-b', 'main']);
    writeFileSync(join(sub, 'README.md'), 'component\n');
    git(sub, ['add', '.']);
    git(sub, ['commit', '-q', '-m', 'component']);

    const root = join(base, 'super');
    mkdirSync(root);
    git(root, ['init', '-q', '-b', 'main']);
    git(root, ['submodule', 'add', '-q', sub, gitlink]);

    const bin = join(base, 'bin');
    mkdirSync(bin);
    const eprLog = join(base, 'epr.log');
    writeFileSync(
      join(bin, 'gh'),
      [
        '#!/usr/bin/env bash',
        'case "${FAKE_GH_CONCLUSION:-success}" in',
        '  unreachable) echo "gh: could not resolve host" >&2; exit 1 ;;',
        '  absent) echo \'{"check_runs":[]}\' ;;',
        '  pending) echo \'{"check_runs":[{"name":"ci","status":"in_progress","conclusion":null,"started_at":"2026-09-23T00:00:00Z"}]}\' ;;',
        '  unpushed) echo "gh: Not Found (HTTP 404)" >&2; exit 1 ;;',
        '  *) printf \'{"check_runs":[{"name":"ci","status":"completed","conclusion":"%s","started_at":"2026-09-23T00:00:00Z"}]}\' "$FAKE_GH_CONCLUSION" ;;',
        'esac',
        '',
      ].join('\n')
    );
    chmodSync(join(bin, 'gh'), 0o755);
    writeFileSync(join(bin, 'epr'), `#!/usr/bin/env bash\nprintf '%s\\n' "$*" >> '${eprLog}'\n`);
    chmodSync(join(bin, 'epr'), 0o755);

    fx = { base, root, sub, gitlink, bin, eprLog, conclusion: 'success' };
  }
);

Given(
  'the component declares an attested gate on {string} check {string}',
  function (repo: string, check: string) {
    assert.ok(fx);
    const inside = join(fx.root, fx.gitlink);
    writeJson(join(inside, 'build-manifest.json'), {
      manifestVersion: '1.0',
      pipeline: 'comp',
      manualOnly: true,
      description: 'component',
      steps: { ci: step([fx.gitlink, `${fx.gitlink}/**`]) },
      gate: {
        projects: {
          comp: {
            dir: fx.gitlink,
            steps: ['ci'],
            run: { kind: 'attested', attestation: { provider: 'github-checks', repo, check } },
          },
        },
      },
    });
    git(inside, ['add', 'build-manifest.json']);
    git(inside, ['commit', '-q', '-m', 'declare the component']);
    git(fx.root, ['add', fx.gitlink]);
  }
);

Given(
  'a consumer step {string} depends on {string} and a deeper step {string} depends on {string}',
  function (build: string, on: string, deep: string, onBuild: string) {
    assert.ok(fx);
    const [pipeline, buildStep] = build.split(':');
    const [, deepStep] = deep.split(':');
    const [, onBuildStep] = onBuild.split(':');
    writeJson(join(fx.root, 'app/build-manifest.json'), {
      manifestVersion: '1.0',
      pipeline,
      description: 'consumer',
      steps: {
        [buildStep]: step(['app/**'], [on]),
        [deepStep]: step(['app/deep/**'], [onBuildStep]),
      },
      gate: {
        projects: {
          app: { dir: 'app', steps: [buildStep], run: { kind: 'just', recipe: 'gate' } },
          deep: { dir: 'app', steps: [deepStep], run: { kind: 'just', recipe: 'gate' } },
        },
      },
    });
    git(fx.root, ['add', '.']);
    git(fx.root, ['commit', '-q', '-m', 'superproject']);
  }
);

Given('the upstream check at the pinned commit concluded {string}', function (conclusion: string) {
  assert.ok(fx);
  fx.conclusion = conclusion;
});
Given('the upstream has no run of the check at the pinned commit', function () {
  assert.ok(fx);
  fx.conclusion = 'absent';
});
Given('the upstream check at the pinned commit is still running', function () {
  assert.ok(fx);
  fx.conclusion = 'pending';
});
Given('the upstream has never seen the pinned commit', function () {
  assert.ok(fx);
  fx.conclusion = 'unpushed';
});
Given('the upstream cannot be read', function () {
  assert.ok(fx);
  fx.conclusion = 'unreachable';
});

function runGate(args: string[], extraEnv: NodeJS.ProcessEnv = {}, input?: string): void {
  assert.ok(fx);
  const r = spawnSync(process.execPath, [GATE_RUNNER, ...args], {
    cwd: fx.root,
    encoding: 'utf8',
    input,
    env: {
      ...process.env,
      GATE_ROOT: fx.root,
      GH_BIN: join(fx.bin, 'gh'),
      EPR_BIN: join(fx.bin, 'epr'),
      FAKE_GH_CONCLUSION: fx.conclusion,
      ...extraEnv,
    },
  });
  fx.status = r.status ?? -1;
  fx.stdout = `${r.stdout}${r.stderr}`;
}

When('the gate runs for project {string}', function (project: string) {
  runGate(['--target', project]);
});

When(
  'selection runs for the changed path {string} with the rakia oracle',
  function (changed: string) {
    runGate(['--changed-file-list', '--print'], { GATE_ORACLE: 'rakia' }, `${changed}\n`);
  }
);

Then('the gate exits {int}', function (code: number) {
  assert.ok(fx);
  assert.equal(fx.status, code, fx.stdout);
});

Then('the gate printed {string}', function (text: string) {
  assert.ok(fx);
  assert.ok(fx.stdout?.includes(text), `expected ${JSON.stringify(text)} in:\n${fx.stdout}`);
});

Then('the gate printed {string} followed by {string}', function (a: string, b: string) {
  assert.ok(fx);
  const line = (fx.stdout ?? '').split('\n').find(l => l.includes(a));
  assert.ok(
    line?.includes(b),
    `expected a line with ${JSON.stringify(a)} and ${JSON.stringify(b)} in:\n${fx.stdout}`
  );
});

Then(
  'one pin-attestation observation was recorded with tier {string} and conclusion {string}',
  function (tier: string, conclusion: string) {
    assert.ok(fx);
    assert.ok(existsSync(fx.eprLog), 'the fake epr was never invoked');
    const lines = readFileSync(fx.eprLog, 'utf8').trim().split('\n');
    assert.equal(lines.length, 1, lines.join('\n'));
    assert.match(
      lines[0],
      /^flow note --kind observation --measure pin-attestation@1 --subject comp --value 1 --unit reads /
    );
    assert.ok(
      lines[0].includes(`--env tier=${tier}`) &&
        lines[0].includes(`--env conclusion=${conclusion}`),
      lines[0]
    );
  }
);

function printedProjects(): { name: string; reasons: string[] }[] {
  assert.ok(fx);
  return (fx.stdout ?? '')
    .split('\n')
    .filter(l => l.startsWith('{'))
    .map(l => JSON.parse(l) as { name: string; reasons: string[] });
}

Then('the selected projects are {string}', function (expected: string) {
  assert.ok(fx);
  assert.deepEqual(
    printedProjects().map(p => p.name),
    expected.split(',').map(s => s.trim()),
    fx.stdout
  );
});

Then('project {string} was selected because of {string}', function (name: string, reason: string) {
  assert.ok(fx);
  const row = printedProjects().find(r => r.name === name);
  assert.ok(row, `${name} not selected:\n${fx.stdout}`);
  assert.ok(row.reasons.includes(reason), JSON.stringify(row.reasons));
});

After(function () {
  if (fx) rmSync(fx.base, { recursive: true, force: true });
  fx = undefined;
});
