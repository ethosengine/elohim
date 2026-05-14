import { strict as assert } from 'node:assert';
import { mkdtempSync, writeFileSync, cpSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';

import { loadConsoleArtifacts, parseScenarioHumanFromFilename } from '../lib/load-console.js';

const fixturesDir = fileURLToPath(new URL('./fixtures/', import.meta.url));

function makeReportsDir(): string {
  const tmp = mkdtempSync(join(tmpdir(), 'a2o-console-'));
  cpSync(
    join(fixturesDir, 'console-terrance-errors.json'),
    join(tmp, 'learning-journey-terrance-errors.json')
  );
  cpSync(
    join(fixturesDir, 'console-mary-errors.json'),
    join(tmp, 'learning-journey-mary-errors.json')
  );
  writeFileSync(join(tmp, 'not-an-artifact.txt'), 'ignored');
  return tmp;
}

void describe('parseScenarioHumanFromFilename', () => {
  void it('splits scenario and human on last hyphen before "errors"', () => {
    assert.deepEqual(parseScenarioHumanFromFilename('learning-journey-terrance-errors.json'), {
      scenario: 'learning-journey',
      human: 'terrance',
    });
  });

  void it('returns null on unrecognized filename', () => {
    assert.equal(parseScenarioHumanFromFilename('random.json'), null);
  });
});

void describe('loadConsoleArtifacts', () => {
  void it('reads every *-errors.json in the directory', () => {
    const dir = makeReportsDir();
    const arts = loadConsoleArtifacts(dir);
    assert.equal(arts.length, 2);
  });

  void it('returns empty array when directory does not exist', () => {
    assert.deepEqual(loadConsoleArtifacts('/no/such/path'), []);
  });

  void it('includes console + page errors with scenario/human tagging', () => {
    const dir = makeReportsDir();
    const arts = loadConsoleArtifacts(dir);
    const mary = arts.find(a => a.human === 'mary')!;
    assert.ok(mary);
    assert.equal(mary.scenario, 'learning-journey');
    assert.equal(mary.consoleErrors.length, 1);
    assert.equal(mary.pageErrors.length, 1);
  });

  void it('ignores non-artifact files silently', () => {
    const dir = makeReportsDir();
    const arts = loadConsoleArtifacts(dir);
    assert.ok(arts.every(a => a.scenario && a.human));
  });
});
