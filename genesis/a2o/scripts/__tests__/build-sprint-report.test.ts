import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { it } from 'node:test';
import { fileURLToPath } from 'node:url';

import type { SprintReport } from '../lib/aggregate.js';
import type { TestContext } from 'node:test';

function fixture(t: TestContext) {
  const dir = mkdtempSync(join(tmpdir(), 'sprint-selection-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const cucumber = join(dir, 'cucumber.json');
  const json = join(dir, 'sprint-report.json');
  const markdown = join(dir, 'sprint-report.md');
  const feedback = join(dir, 'feedback');
  const epr = join(dir, 'epr');
  writeFileSync(epr, '#!/bin/sh\ntouch "$REPORT_TEST_FEEDBACK"\n', { mode: 0o755 });
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    EPR_BIN: epr,
    REPORT_TEST_FEEDBACK: feedback,
    A2O_POST_REPORT: '0',
  };
  // The child is a CLI, not a node:test worker with serialized console output.
  delete env.NODE_TEST_CONTEXT;
  const run = () =>
    spawnSync(
      process.execPath,
      [
        '--import',
        'tsx',
        'scripts/build-sprint-report.ts',
        '--cucumber',
        cucumber,
        '--out-json',
        json,
        '--out-md',
        markdown,
        '--console-dir',
        join(dir, 'console'),
        '--coverage-gap',
        join(dir, 'coverage-gap.json'),
        '--scope',
        '@concern:fixture',
      ],
      {
        cwd: fileURLToPath(new URL('../../', import.meta.url)),
        encoding: 'utf8',
        env,
      }
    );
  return { cucumber, json, markdown, feedback, run };
}

for (const input of [undefined, '', '[]', '[{"uri":"features/empty.feature","elements":[]}]']) {
  void it(`scoped selection refuses ${String(input)} before any report effects`, t => {
    const f = fixture(t);
    if (input !== undefined) writeFileSync(f.cucumber, input);
    const result = f.run();
    assert.equal(result.status, 3, result.stderr);
    assert.match(result.stderr, /EMPTY SELECTION.*0 scenarios/);
    assert.equal(existsSync(f.json), false);
    assert.equal(existsSync(f.markdown), false);
    assert.equal(existsSync(f.feedback), false);
  });
}

for (const status of ['passed', 'failed']) {
  void it(`a scoped ${status} scenario still writes its report`, t => {
    const f = fixture(t);
    writeFileSync(
      f.cucumber,
      JSON.stringify([
        {
          uri: 'features/fixture.feature',
          elements: [
            {
              type: 'scenario',
              name: 'selected scenario',
              tags: [{ name: '@concern:fixture' }],
              steps: [{ name: 'measured step', result: { status, duration: 1_000_000 } }],
            },
          ],
        },
      ])
    );
    const result = f.run();
    assert.equal(result.status, 0, result.stderr);
    const report = JSON.parse(readFileSync(f.json, 'utf8')) as SprintReport;
    assert.equal(report.summary.scenarios.total, 1);
    assert.equal(report.summary.byConcern.fixture.scenarios[0].status, status);
    assert.equal(existsSync(f.markdown), true);
  });
}
