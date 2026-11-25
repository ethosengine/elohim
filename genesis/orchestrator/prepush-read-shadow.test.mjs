import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(fileURLToPath(new URL('../..', import.meta.url)));
const HOOK = readFileSync(join(ROOT, '.husky/pre-push.bash'), 'utf8');
const START = '# ── Read-shadow push ledger ─────────────────────────────────────';
const END = '# ── Read-shadow push ledger end ─────────────────────────────────';
const LEDGER = HOOK.slice(HOOK.indexOf(START), HOOK.indexOf(END) + END.length);
const GATE_CALL = HOOK.match(
  /GATE_RUN_KIND=pre-push node genesis\/orchestrator\/gate-runner\.mjs --target "\$PROJECT"/,
)?.[0];

test('the pre-push gate loop exports one read-shadow ledger to every runner and cleans it', t => {
  assert.ok(LEDGER.startsWith(START) && LEDGER.endsWith(END));
  assert.ok(GATE_CALL, 'the actual inline gate-runner invocation is present');
  const dir = mkdtempSync(join(tmpdir(), 'prepush-read-shadow-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const bin = join(dir, 'bin');
  const capture = join(dir, 'runner-env.tsv');
  mkdirSync(bin);
  writeFileSync(join(bin, 'node'), `#!/usr/bin/env bash
if [ -f "$GATE_READ_SHADOW_LEDGER" ]; then exists=yes; else exists=no; fi
printf '%s\t%s\t%s\n' "$3" "$GATE_READ_SHADOW_LEDGER" "$exists" >>"$CAPTURE"
`);
  chmodSync(join(bin, 'node'), 0o755);

  const result = spawnSync('bash', ['-c', `
HOOK_ON_EXIT=""
HOOK_WORK_TREE=${JSON.stringify(dir)}
${LEDGER}
for PROJECT in storage app; do
  ${GATE_CALL}
done
printf 'ledger=%s\n' "$GATE_READ_SHADOW_LEDGER"
gate_remove_read_shadow_ledger
if [ -e "$GATE_READ_SHADOW_LEDGER" ]; then echo cleaned=no; else echo cleaned=yes; fi
`], {
    cwd: ROOT,
    encoding: 'utf8',
    env: { ...process.env, PATH: `${bin}:${process.env.PATH}`, CAPTURE: capture },
  });

  assert.equal(result.status, 0, result.stderr);
  const rows = readFileSync(capture, 'utf8').trim().split('\n').map(row => row.split('\t'));
  assert.deepEqual(rows.map(([project]) => project), ['storage', 'app']);
  assert.ok(rows.every(([, ledger, exists]) => ledger === rows[0][1] && exists === 'yes'));
  assert.equal(result.stdout, `ledger=${rows[0][1]}\ncleaned=yes\n`);
});

test('the pre-push hook exports no read-shadow ledger when the knob is off', t => {
  const dir = mkdtempSync(join(tmpdir(), 'prepush-read-shadow-off-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const result = spawnSync('bash', ['-c', `
HOOK_ON_EXIT=""
HOOK_WORK_TREE=${JSON.stringify(dir)}
GATE_READ_SHADOW=off
unset GATE_READ_SHADOW_LEDGER GATE_READ_SHADOW_SCRATCH
${LEDGER}
if [ "\${GATE_READ_SHADOW_LEDGER+x}" = x ]; then echo ledger=exported; else echo ledger=absent; fi
printf 'handlers=%s\n' "$HOOK_ON_EXIT"
`], { cwd: ROOT, encoding: 'utf8', env: process.env });

  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, 'ledger=absent\nhandlers=\n');
});
