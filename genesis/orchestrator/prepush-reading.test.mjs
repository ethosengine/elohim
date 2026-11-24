import { afterEach, test } from 'node:test';
import { strict as assert } from 'node:assert';
import {
  chmodSync,
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(fileURLToPath(new URL('../..', import.meta.url)));
const HOOK = readFileSync(join(ROOT, '.husky/pre-push.bash'), 'utf8');
const START = '# ── Gate step reading helper ────────────────────────────────────';
const END = '# ── Gate step reading helper end ────────────────────────────────';
const HELPER = HOOK.slice(HOOK.indexOf(START), HOOK.indexOf(END) + END.length);
const scratch = [];

afterEach(() => {
  while (scratch.length > 0) rmSync(scratch.pop(), { recursive: true, force: true });
});

function fixture(exit = 0) {
  const dir = mkdtempSync(join(tmpdir(), 'prepush-reading-'));
  scratch.push(dir);
  const capture = join(dir, 'argv');
  const epr = join(dir, 'epr');
  writeFileSync(epr, `#!/usr/bin/env bash\nprintf '%s\\n' "$@" >>"$CAPTURE"\nexit ${exit}\n`);
  chmodSync(epr, 0o755);
  return { dir, capture, epr };
}

function run(fx, body, env = {}) {
  return spawnSync('bash', ['-c', `${HELPER}\n${body}`], {
    cwd: ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      EPR_BIN: fx.epr,
      CAPTURE: fx.capture,
      GATE_READING_RUN_ID: 'fixture-run',
      ...env,
    },
  });
}

test('pre-push leg helper builds the native observation argv', () => {
  const fx = fixture();
  const result = run(fx, `
GATE_READING_SEEN=""
gate_step_reading 0 fixture ran "none: reads device state" unknown 4 \
  2026-11-24T12:00:00.000Z 2026-11-24T12:00:04.000Z
printf 'rc=%s\\n' "$?"
`);

  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, 'rc=0\n');
  assert.deepEqual(readFileSync(fx.capture, 'utf8').trim().split('\n'), [
    'flow', 'sense', 'observation',
    '--measure', 'gate-step-reading@1',
    '--subject', '.',
    '--value', '4',
    '--unit', 'seconds',
    '--env', 'run_id=fixture-run',
    '--env', 'step=leg:fixture',
    '--env', 'key=none: reads device state',
    '--env', 'key_changed=unknown',
    '--env', 'outcome=ran',
    '--env', 'waited_seconds=0',
    '--env', 'started_at=2026-11-24T12:00:00.000Z',
    '--env', 'ended_at=2026-11-24T12:00:04.000Z',
  ]);
});

test('pre-push leg helper is silent when GATE_READING=off', () => {
  const fx = fixture();
  const result = run(fx, `
GATE_READING_SEEN=""
gate_step_reading 23 fixture skipped "none: not selected" unknown 0 \
  2026-11-24T12:00:00.000Z 2026-11-24T12:00:00.000Z
printf 'rc=%s\\n' "$?"
`, { GATE_READING: 'off' });

  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, 'rc=23\n');
  assert.equal(result.stderr, '');
  assert.equal(existsSync(fx.capture), false);
});

test('pre-push leg helper preserves status and warns once when observation fails', () => {
  const fx = fixture(9);
  const result = run(fx, `
GATE_READING_SEEN=""
gate_step_reading 17 first ran "none: first" unknown 1 \
  2026-11-24T12:00:00.000Z 2026-11-24T12:00:01.000Z
printf 'first=%s\\n' "$?"
gate_step_reading 19 second ran "none: second" unknown 1 \
  2026-11-24T12:00:01.000Z 2026-11-24T12:00:02.000Z
printf 'second=%s\\n' "$?"
`);

  assert.equal(result.status, 0);
  assert.equal(result.stdout, 'first=17\nsecond=19\n');
  assert.equal(result.stderr, '[gate-reading] pre-push leg observations unavailable (exit 9)\n');
});

test('pre-push leg helper fills each unreached leg exactly once as skipped', () => {
  const fx = fixture();
  const result = run(fx, `
GATE_READING_SEEN=""
gate_step_reading 0 submodules ran "none: fixture" unknown 1 \
  2026-11-24T12:00:00.000Z 2026-11-24T12:00:01.000Z
gate_step_reading
`);

  assert.equal(result.status, 0, result.stderr);
  const argv = readFileSync(fx.capture, 'utf8').trim().split('\n');
  const steps = argv.filter((arg, index) => argv[index - 1] === '--env' && arg.startsWith('step='));
  const outcomes = argv.filter((arg, index) => argv[index - 1] === '--env' && arg.startsWith('outcome='));
  assert.equal(steps.length, 28);
  assert.equal(new Set(steps).size, 28);
  assert.equal(steps.filter(step => step === 'step=leg:submodules').length, 1);
  assert.equal(outcomes.filter(outcome => outcome === 'outcome=ran').length, 1);
  assert.equal(outcomes.filter(outcome => outcome === 'outcome=skipped').length, 27);
});
