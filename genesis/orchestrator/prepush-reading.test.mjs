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
  const rows = join(dir, 'rows.jsonl');
  const epr = join(dir, 'epr');
  writeFileSync(epr, `#!/usr/bin/env bash
printf '%s\\n' "$@" >>"$CAPTURE"
while [ "$#" -gt 0 ]; do
  if [ "$1" = --from ]; then cp "$2" "$ROWS_CAPTURE"; break; fi
  shift
done
exit ${exit}
`);
  chmodSync(epr, 0o755);
  return { dir, capture, rows, epr };
}

function run(fx, body, env = {}) {
  return spawnSync('bash', ['-c', `${HELPER}\n${body}`], {
    cwd: ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      EPR_BIN: fx.epr,
      CAPTURE: fx.capture,
      ROWS_CAPTURE: fx.rows,
      HOOK_WORK_TREE: ROOT,
      GATE_READING_RUN_ID: 'fixture-run',
      ...env,
    },
  });
}

test('pre-push leg helper safely collects quotes and backslashes then appends once', () => {
  const fx = fixture();
  const result = run(fx, `
GATE_READING_SEEN=""
GATE_READING_LEGS=$'fixture\\tnone\\treason says "quoted" and \\\\ escaped'
gate_step_reading 0 fixture ran 4 \
  2026-11-24T12:00:00.000Z 2026-11-24T12:00:04.000Z
printf 'rc=%s\\n' "$?"
gate_step_reading
`);

  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, 'rc=0\n');
  const argv = readFileSync(fx.capture, 'utf8').trim().split('\n');
  assert.deepEqual(argv.slice(0, 4), ['flow', 'reading', 'append', '--from']);
  assert.match(argv[4], /\/rows\.jsonl$/);
  assert.deepEqual(argv.slice(5), ['--root', ROOT]);
  const rows = readFileSync(fx.rows, 'utf8').trim().split('\n').map(line => JSON.parse(line));
  assert.equal(rows.length, 1);
  assert.equal(rows[0].key, 'none: reason says "quoted" and \\ escaped');
  assert.equal(rows[0].value, 4);
});

test('pre-push leg helper is silent when GATE_READING=off', () => {
  const fx = fixture();
  const result = run(fx, `
GATE_READING_SEEN=""
gate_step_reading 23 fixture skipped 0 \
  2026-11-24T12:00:00.000Z 2026-11-24T12:00:00.000Z
printf 'rc=%s\\n' "$?"
gate_step_reading
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
GATE_READING_LEGS=$'first\\tnone\\tfirst reason\\nsecond\\tnone\\tsecond reason'
gate_step_reading 17 first ran 1 \
  2026-11-24T12:00:00.000Z 2026-11-24T12:00:01.000Z
printf 'first=%s\\n' "$?"
gate_step_reading 19 second ran 1 \
  2026-11-24T12:00:01.000Z 2026-11-24T12:00:02.000Z
printf 'second=%s\\n' "$?"
gate_step_reading
`);

  assert.equal(result.status, 0);
  assert.equal(result.stdout, 'first=17\nsecond=19\n');
  assert.equal(result.stderr, '[gate-reading] pre-push readings unavailable (exit 9)\n');
});

test('pre-push leg helper fills each unreached leg exactly once as skipped', () => {
  const fx = fixture();
  const result = run(fx, `
GATE_READING_SEEN=""
gate_step_reading 0 submodules ran 1 \
  2026-11-24T12:00:00.000Z 2026-11-24T12:00:01.000Z
gate_step_reading
`);

  assert.equal(result.status, 0, result.stderr);
  const argv = readFileSync(fx.capture, 'utf8').trim().split('\n');
  assert.equal(argv.filter(arg => arg === 'flow').length, 1);
  const rows = readFileSync(fx.rows, 'utf8').trim().split('\n').map(line => JSON.parse(line));
  const steps = rows.map(row => row.step);
  const outcomes = rows.map(row => row.outcome);
  assert.equal(steps.length, 28);
  assert.equal(new Set(steps).size, 28);
  assert.equal(steps.filter(step => step === 'leg:submodules').length, 1);
  assert.equal(outcomes.filter(outcome => outcome === 'ran').length, 1);
  assert.equal(outcomes.filter(outcome => outcome === 'skipped').length, 27);
});
