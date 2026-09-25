/**
 * Unit cover for the pure readers behind features/dataplane/app-delivery-refuses-fast.feature:
 * the fleet write-readiness probe's NOT-READY lines, the doorway/host match, and the timing
 * lines stage-spa-blob.sh prints while it waits. The probe itself is exercised against small
 * stand-in scripts that honour the same exit-status contract (0 ready · 3 not ready · 2 usage),
 * so these tests run without a household mesh.
 */
import { strict as assert } from 'node:assert';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { after, describe, it } from 'node:test';

import {
  askFleetWriteReadiness,
  CONDUCTOR_RESTART_FACES,
  FLEET_WRITE_READINESS,
  isReadinessFace,
  lastFace,
  namesDoorway,
  parseNotReadyLines,
  parseTimingLines,
  PLAN_FACES,
  pollReadiness,
  READINESS_FACES,
  READINESS_FACES_FILE,
  timingViolations,
  withEnv,
  type PollClock,
  type ReadinessAnswer,
} from '../app-delivery-refuses-fast.helpers.js';
import { REPO_ROOT } from '../epr-app-deliverability.helpers.js';

const DOORWAY_A = 'http://localhost:8888';
const DOORWAY_B = 'http://localhost:8889';
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

void describe('pollReadiness', () => {
  // A clock the test drives: sleeping advances it, and each ask costs `askMs`, so the loop's
  // arithmetic is exercised without waiting in real time.
  function fakeClock(askMs = 1_000): { clock: PollClock; tick: () => void; slept: number[] } {
    let now = 0;
    const slept: number[] = [];
    return {
      clock: {
        now: () => now,
        sleep: async ms => {
          slept.push(ms);
          now += ms;
          return Promise.resolve();
        },
      },
      tick: () => {
        now += askMs;
      },
      slept,
    };
  }

  function answer(code: number | null, output = ''): ReadinessAnswer {
    return {
      code,
      timedOut: code === null,
      output,
      startedAt: 0,
      elapsedMs: 1_000,
      notReady: parseNotReadyLines(output),
    };
  }

  const NOT_READY = answer(
    3,
    'FLEET-NOT-READY http://localhost:8889 face=storage-refused retryAfter=20'
  );

  void it('keeps asking every interval until ready, and measures the time it took', async () => {
    const { clock, tick, slept } = fakeClock();
    const script = [NOT_READY, NOT_READY, answer(0)];
    const poll = await pollReadiness(
      async () => {
        tick();
        return Promise.resolve(script.shift() ?? answer(0));
      },
      120_000,
      5_000,
      clock
    );
    assert.equal(poll.ready, true);
    assert.equal(poll.answers.length, 3);
    // three asks of 1s each and two 5s pauses between them
    assert.equal(poll.timeToReadyMs, 13_000);
    assert.deepEqual(slept, [5_000, 5_000]);
    assert.equal(lastFace(poll.last), 'ready');
  });

  void it('answers at once, without sleeping, when the first ask is ready', async () => {
    const { clock, tick, slept } = fakeClock();
    const poll = await pollReadiness(
      async () => {
        tick();
        return Promise.resolve(answer(0));
      },
      30_000,
      5_000,
      clock
    );
    assert.equal(poll.ready, true);
    assert.equal(poll.answers.length, 1);
    assert.deepEqual(slept, []);
  });

  void it('stops at the deadline with the last not-ready answer, asking once on the deadline itself', async () => {
    const { clock, tick, slept } = fakeClock();
    const poll = await pollReadiness(
      async () => {
        tick();
        return Promise.resolve(NOT_READY);
      },
      12_000,
      5_000,
      clock
    );
    assert.equal(poll.ready, false);
    assert.equal(poll.timeToReadyMs, null);
    // 1s ask, 5s, 1s ask, 5s (t=12s), last ask lands past the deadline
    assert.deepEqual(slept, [5_000, 5_000]);
    assert.ok(poll.elapsedMs >= 12_000);
    assert.equal(poll.last, NOT_READY);
    assert.equal(lastFace(poll.last), 'storage-refused');
  });

  void it('shortens the last pause so an ask lands on the deadline, not past it', async () => {
    const { clock, tick, slept } = fakeClock();
    await pollReadiness(
      async () => {
        tick();
        return Promise.resolve(NOT_READY);
      },
      8_000,
      5_000,
      clock
    );
    assert.deepEqual(slept, [5_000, 1_000]);
  });

  void it('keeps asking after an answer that hung or could not be judged', async () => {
    const { clock, tick } = fakeClock();
    const script = [answer(null), answer(2), answer(0)];
    const poll = await pollReadiness(
      async () => {
        tick();
        return Promise.resolve(script.shift() ?? answer(0));
      },
      60_000,
      5_000,
      clock
    );
    assert.equal(poll.ready, true);
    assert.deepEqual(
      poll.answers.map(a => lastFace(a)),
      ['no-answer', 'exit-2', 'ready']
    );
  });
});

const sorted = (values: Iterable<string>): string[] =>
  [...values].sort((a, b) => a.localeCompare(b));

void describe('one face vocabulary', () => {
  const listed = (
    JSON.parse(readFileSync(READINESS_FACES_FILE, 'utf8')) as {
      faces: { face: string; plan: boolean }[];
    }
  ).faces;

  void it('is the list in scripts/ci/lib/readiness-faces.json, in its order', () => {
    assert.deepEqual(
      [...READINESS_FACES],
      listed.map(entry => entry.face)
    );
    assert.deepEqual(
      [...PLAN_FACES],
      listed.filter(entry => entry.plan).map(entry => entry.face)
    );
  });

  void it('names the plan faces by the three names the feature and the deploy use', () => {
    assert.deepEqual(sorted(PLAN_FACES), [
      'catching-up',
      'cell-not-running',
      'storage-forward-timeout',
    ]);
  });

  void it('is exactly the set of faces the probe script can print', () => {
    const source = readFileSync(FLEET_WRITE_READINESS, 'utf8');
    const printable = new Set([...source.matchAll(/"NOT ([a-z0-9-]+)"/g)].map(match => match[1]));
    assert.deepEqual(sorted(printable), sorted(READINESS_FACES));
  });

  void it('holds every face the feature file names', () => {
    const feature = readFileSync(
      join(REPO_ROOT, 'genesis/a2o/features/dataplane/app-delivery-refuses-fast.feature'),
      'utf8'
    );
    const named = [...feature.matchAll(/face ("[^"]+"(?:(?:, | or )"[^"]+")*)/g)].flatMap(match =>
      [...match[1].matchAll(/"([^"]+)"/g)].map(quoted => quoted[1])
    );
    assert.ok(named.length >= 3, `found only ${named.length} named faces in the feature`);
    for (const face of named) assert.ok(isReadinessFace(face), face);
  });

  void it('lists the faces a conductor restart can wear, each one in the vocabulary', () => {
    assert.deepEqual(sorted(CONDUCTOR_RESTART_FACES), [
      'catching-up',
      'cell-not-running',
      'storage-refused',
    ]);
    for (const face of CONDUCTOR_RESTART_FACES) assert.ok(isReadinessFace(face), face);
  });

  void it('refuses a name that is not in the vocabulary, including the retired conductor-blind', () => {
    assert.equal(isReadinessFace('conductor-blind'), false);
    assert.equal(isReadinessFace('catching up'), false);
  });
});

void describe('the probe script, as the household asks it', () => {
  // A stand-in curl on PATH: both doorways are healthy and "do not hold" the probe
  // blob, but doorway B sheds the content route the way the household's declared shed
  // does. The REAL probe script runs, so the line the helper reads is the line it prints.
  const bin = join(scratch, 'fake-bin');
  mkdirSync(bin, { recursive: true });
  const curl = join(bin, 'curl');
  writeFileSync(
    curl,
    [
      '#!/bin/bash',
      'hdr="" out="" url=""',
      'while [ "$#" -gt 0 ]; do case "$1" in',
      '  -D) hdr="$2"; shift 2 ;; -o) out="$2"; shift 2 ;; -w|-X|-H|--max-time|--data-binary) shift 2 ;;',
      '  -*) shift ;; *) url="$1"; shift ;; esac; done',
      'case "$url" in',
      '  */health/serving) st=200; ra=""; body=\'{"shedding":false,"degrading":false,"rolesDiscovered":3,"storageServing":{"status":"serving"}}\' ;;',
      '  *:8889/db/content/*) st=503; ra=90; body=\'{"status":"catching-up","retryAfter":90,"cause":"dev-fixture"}\' ;;',
      '  */db/content/*) st=404; ra=""; body=\'{"error":"not found"}\' ;;',
      '  */admin/seed/blob) st=409; ra=""; body=\'{"success":false,"forwarded_to_storage":false}\' ;;',
      '  *) exit 7 ;;',
      'esac',
      String.raw`{ printf "HTTP/1.1 %s\r\n" "$st"; [ -n "$ra" ] && printf "Retry-After: %s\r\n" "$ra"; printf "\r\n"; } > "$hdr"`,
      'printf "%s" "$body" > "$out"',
      'printf "%s" "$st"',
    ].join('\n')
  );
  chmodSync(curl, 0o755);

  void it('names the shedding doorway by its origin with a listed face, and only that doorway', async () => {
    const answer = await withEnv({ PATH: `${bin}:${process.env['PATH'] ?? ''}` }, async () =>
      askFleetWriteReadiness([`${DOORWAY_A}/`, DOORWAY_B], 20_000)
    );
    assert.equal(answer.code, 3, answer.output);
    assert.deepEqual(
      answer.notReady.map(({ host, face, retryAfter }) => ({ host, face, retryAfter })),
      [{ host: DOORWAY_B, face: CATCHING_UP, retryAfter: 90 }],
      answer.output
    );
    const [line] = answer.notReady;
    assert.ok(line && isReadinessFace(line.face));
    assert.ok(line && namesDoorway(line.host, DOORWAY_B));
    assert.ok(line && !namesDoorway(line.host, DOORWAY_A));
    assert.match(answer.output, /^FLEET-READY http:\/\/localhost:8888$/m);
  });
});
