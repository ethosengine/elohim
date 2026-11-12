import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { createRecognizer } from './gate-failure-lines.mjs';
const script = fileURLToPath(new URL('./gate-steps.sh', import.meta.url));

test('runs all steps, preserves argv and returns the first failure', () => {
  const r = spawnSync(
    'bash',
    [
      script,
      'fmt',
      '--',
      'bash',
      '-c',
      'exit 7',
      ':::',
      'lint',
      '--',
      'bash',
      '-c',
      'exit 9',
      ':::',
      'test',
      '--',
      'printf',
      '%s\n',
      'literal $value with spaces',
    ],
    { encoding: 'utf8' },
  );
  assert.equal(r.status, 7);
  assert.match(r.stdout, /literal \$value with spaces/);
  const events = [];
  const rec = createRecognizer({
    project: 'proj',
    emit: (e) => events.push(e),
  });
  r.stderr.split('\n').forEach((l) => rec.push(l));
  rec.push('error: Recipe `gate` failed on line 9 with exit code 7');
  rec.end();
  assert.deepEqual(
    events.map((e) => [e.kind, e.step, e.exit]),
    [
      ['step-failed', 'fmt', 7],
      ['step-failed', 'lint', 9],
      ['step-failed', 'gate', 7],
    ],
  );
});
test('validates all steps before execution and passes a green plan', () => {
  for (const args of [
    [],
    ['fmt', '--', 'echo', 'ran', ':::'],
    ['fmt', '--', 'echo', 'ran', ':::', 'bad'],
  ]) {
    const r = spawnSync('bash', [script, ...args], { encoding: 'utf8' });
    assert.equal(r.status, 2);
    assert.equal(r.stdout, '');
  }
  assert.equal(spawnSync('bash', [script, 'test', '--', 'true']).status, 0);
});

test('an invoking shell with errexit still runs later checks', () => {
  const r = spawnSync(
    'bash',
    ['-e', script, 'lint', '--', 'false', ':::', 'test', '--', 'echo', 'tests ran'],
    { encoding: 'utf8' },
  );
  assert.equal(r.status, 1);
  assert.match(r.stdout, /tests ran/);
});
