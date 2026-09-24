/**
 * Unit cover for the pure readers behind features/dataplane/app-delivery-refuses-fast.feature:
 * the fleet write-readiness probe's NOT-READY lines, the doorway/host match, and the timing
 * lines stage-spa-blob.sh prints while it waits. The probe itself is exercised against small
 * stand-in scripts that honour the same exit-status contract (0 ready · 3 not ready · 2 usage),
 * so these tests run without a household mesh.
 */
import { strict as assert } from 'node:assert';
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { after, describe, it } from 'node:test';

import {
  askFleetWriteReadiness,
  namesDoorway,
  parseNotReadyLines,
  parseTimingLines,
  timingViolations,
} from '../app-delivery-refuses-fast.helpers.js';

const DOORWAY_A = 'http://localhost:8888';
const CATCHING_UP = 'catching-up';

const scratch = mkdtempSync(join(tmpdir(), 'refuses-fast-'));
after(() => {
  rmSync(scratch, { recursive: true, force: true });
});

function standIn(name: string, body: string): string {
  const path = join(scratch, name);
  writeFileSync(path, `#!/bin/bash\n${body}\n`);
  chmodSync(path, 0o755);
  return path;
}

void describe('parseNotReadyLines', () => {
  void it('reads every FLEET-NOT-READY line and nothing else', () => {
    const lines = parseNotReadyLines(
      [
        'probing 2 doorways',
        'FLEET-NOT-READY localhost:8888 face=catching-up retryAfter=118',
        'FLEET-NOT-READY http://localhost:8889 face=cell-not-running retryAfter=30',
        'done',
      ].join('\n')
    );
    assert.deepEqual(
      lines.map(({ host, face, retryAfter }) => ({ host, face, retryAfter })),
      [
        { host: 'localhost:8888', face: CATCHING_UP, retryAfter: 118 },
        { host: 'http://localhost:8889', face: 'cell-not-running', retryAfter: 30 },
      ]
    );
  });

  void it('keeps a line whose retry-after is not a number, with the value absent', () => {
    const [line] = parseNotReadyLines(
      'FLEET-NOT-READY localhost:8888 face=catching-up retryAfter='
    );
    assert.equal(line?.face, CATCHING_UP);
    assert.equal(line?.retryAfter, null);
  });
});

void describe('namesDoorway', () => {
  void it('matches host:port, origin and full URL forms of the same doorway', () => {
    for (const host of ['localhost:8888', 'http://localhost:8888', 'http://localhost:8888/']) {
      assert.ok(namesDoorway(host, DOORWAY_A), host);
    }
  });

  void it('never matches a bare hostname two doorways on one machine share, or a sibling port', () => {
    assert.equal(namesDoorway('localhost', DOORWAY_A), false);
    assert.equal(namesDoorway('localhost:8889', DOORWAY_A), false);
  });
});

// The four line shapes below are copied from the echo templates in
// scripts/ci/stage-spa-blob.sh (cell_ready_wait, cell_ready_exhausted, the
// "took the write after" line, and the transport ladder's retry line).
const WAIT =
  '  … [s] still not ready on http://localhost:8888 (face=catching-up, advertised retryAfter=31s) — 12s waited, 48s left of 60s; re-offering the same content-addressed browser declaration in 5s';
const EXHAUSTED =
  '  ✗ [s] browser declaration via http://localhost:8888 — readiness deadline reached; the holder was still not ready after 21s (last face=catching-up, budget 20s, first not-ready answer of this run 12:00:00Z; last answer HTTP 503: {}); an attempt in flight may have finished past it.';
const CLEARED =
  "  ✓ [s] http://localhost:8888 took the write after 33s of not-ready answers (last face=catching-up) — the same content-addressed offer went through, no intervention needed; 27s of this run's readiness deadline left for a later window";
const TRANSPORT =
  '  ⚠ [s] attempt 2/60 against http://localhost:8888 failed — retrying in 4s (elapsed 9s, 111s left of 120s budget)';

void describe('parseTimingLines', () => {
  void it('reads each kind of timing line the deploy prints', () => {
    const lines = parseTimingLines([WAIT, EXHAUSTED, CLEARED, TRANSPORT, 'noise'].join('\n'));
    assert.deepEqual(
      lines.map(({ kind, elapsedSecs, budgetSecs, face }) => ({
        kind,
        elapsedSecs,
        budgetSecs,
        face,
      })),
      [
        { kind: 'readiness-wait', elapsedSecs: 12, budgetSecs: 60, face: CATCHING_UP },
        { kind: 'readiness-exhausted', elapsedSecs: 21, budgetSecs: 20, face: CATCHING_UP },
        { kind: 'readiness-cleared', elapsedSecs: 33, budgetSecs: null, face: CATCHING_UP },
        { kind: 'transport-retry', elapsedSecs: 9, budgetSecs: 120, face: null },
      ]
    );
  });
});

void describe('timingViolations', () => {
  const told = { readinessSecs: 60, transportSecs: 120, inFlightSlackSecs: 30 };

  void it('finds nothing wrong in a deploy that obeyed its budgets', () => {
    assert.deepEqual(
      timingViolations(parseTimingLines([WAIT, CLEARED, TRANSPORT].join('\n')), told),
      []
    );
  });

  void it('flags a re-offer scheduled at or past the readiness deadline', () => {
    const late = WAIT.replace('12s waited, 48s left of 60s', '60s waited, 0s left of 60s');
    assert.equal(timingViolations(parseTimingLines(late), told).length, 1);
  });

  void it('flags a deploy that read a different budget from the one it was told', () => {
    const drifted = WAIT.replace('48s left of 60s', '7188s left of 7200s');
    const found = timingViolations(parseTimingLines(drifted), told);
    assert.ok(
      found.some(v => v.includes('7200')),
      found.join('\n')
    );
  });

  void it('flags a stop reported later than the deadline plus one in-flight attempt', () => {
    const tooLate = EXHAUSTED.replace('after 21s', 'after 51s');
    const found = timingViolations(parseTimingLines(tooLate), { ...told, readinessSecs: 20 });
    assert.equal(found.length, 1, found.join('\n'));
  });

  void it('flags a transport retry past its budget', () => {
    const over = TRANSPORT.replace(
      'elapsed 9s, 111s left of 120s',
      'elapsed 125s, -5s left of 120s'
    );
    assert.equal(timingViolations(parseTimingLines(over), told).length, 1);
  });
});

void describe('askFleetWriteReadiness', () => {
  void it('reports a not-ready answer with its lines and how long it took', async () => {
    const probe = standIn(
      'not-ready.sh',
      'echo "FLEET-NOT-READY ${1#http://} face=catching-up retryAfter=90"; exit 3'
    );
    const answer = await askFleetWriteReadiness([DOORWAY_A], 10_000, probe);
    assert.equal(answer.code, 3);
    assert.equal(answer.timedOut, false);
    assert.deepEqual(
      answer.notReady.map(line => line.host),
      ['localhost:8888']
    );
    assert.ok(answer.elapsedMs < 10_000);
  });

  void it('reports a ready answer as exit status 0 with no lines', async () => {
    const probe = standIn('ready.sh', 'exit 0');
    const answer = await askFleetWriteReadiness([DOORWAY_A], 10_000, probe);
    assert.equal(answer.code, 0);
    assert.deepEqual(answer.notReady, []);
  });

  void it('stops a probe that waits past its bound and says so, instead of waiting with it', async () => {
    const probe = standIn('sleeps.sh', 'sleep 30; exit 0');
    const started = Date.now();
    const answer = await askFleetWriteReadiness([DOORWAY_A], 500, probe);
    assert.equal(answer.timedOut, true);
    assert.equal(answer.code, null);
    assert.ok(Date.now() - started < 10_000, 'the caller itself waited on a hung probe');
  });
});
