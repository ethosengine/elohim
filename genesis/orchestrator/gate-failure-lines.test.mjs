import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import {
  createRecognizer, fingerprint, normalize, stripAnsi, MAX_FAILURES, MAX_EXCERPT_BYTES,
} from './gate-failure-lines.mjs';
import { classifyFailure } from './gate-cycle.mjs';

function run(lines, opts = {}) {
  const events = [];
  const rec = createRecognizer({ project: 'proj', emit: e => events.push(e), ...opts });
  for (const line of lines) rec.push(line);
  rec.end();
  return { events, rec };
}

const failures = events => events.filter(e => e.kind === 'failure');

describe('cargo test', () => {
  const CARGO = [
    'running 3 tests',
    'test ok ... ok',
    'test a ... FAILED',
    'test b ... FAILED',
    '',
    'failures:',
    '',
    '---- a stdout ----',
    "thread 'a' panicked at src/lib.rs:10:5:",
    'assertion `left == right` failed',
    '',
    '---- b stdout ----',
    "thread 'b' panicked at src/other.rs:22:9:",
    'boom',
    '',
    'failures:',
    '    a',
    '    b',
    '',
    'test result: FAILED. 1 passed; 2 failed; 0 ignored',
  ];

  test('raises one failure per FAILED line before any detail, then details carry the panic locus', () => {
    const { events } = run(CARGO);
    assert.deepEqual(events.map(e => e.kind), ['failure', 'failure', 'detail', 'detail']);
    const [fa, fb, da, db] = events;
    assert.equal(fa.failure.class, 'test');
    assert.deepEqual(fa.failure.locus, { test: 'a' });
    assert.deepEqual(fb.failure.locus, { test: 'b' });
    assert.deepEqual(da.failure.locus, { test: 'a', file: 'src/lib.rs', line: 10 });
    assert.deepEqual(db.failure.locus, { test: 'b', file: 'src/other.rs', line: 22 });
    assert.equal(da.failure.fingerprint, fa.failure.fingerprint);
    assert.equal(db.failure.fingerprint, fb.failure.fingerprint);
    assert.match(da.failure.excerpt, /panicked at src\/lib\.rs:10:5/);
  });

  test('a detail block without an earlier FAILED line still raises the failure first', () => {
    const { events } = run([
      '---- lone stdout ----',
      "thread 'lone' panicked at src/x.rs:1:1:",
      'nope',
      'test result: FAILED. 0 passed; 1 failed',
    ]);
    assert.deepEqual(events.map(e => e.kind), ['failure', 'detail']);
  });

  test('a failing test after `running N tests` classifies as code at exit 101', () => {
    const { rec } = run(['running 2 tests', 'test a ... FAILED']);
    assert.equal(classifyFailure(101, rec.signals()), 'code');
  });
});

describe('rustc, clippy, rustfmt', () => {
  test('rustc error takes its locus from the --> line and ends at the blank line', () => {
    const { events } = run([
      'error[E0308]: mismatched types',
      '  --> src/lib.rs:3:5',
      '   |',
      '3  |     let x: u32 = "s";',
      '   |                  ^^^ expected `u32`, found `&str`',
      '',
    ]);
    const f = failures(events);
    assert.equal(f.length, 1);
    assert.equal(f[0].failure.class, 'compile');
    assert.deepEqual(f[0].failure.locus, { file: 'src/lib.rs', line: 3 });
  });

  test('aborting and could-not-compile summaries raise nothing', () => {
    const { events, rec } = run([
      'error: aborting due to 1 previous error',
      '',
      'error: could not compile `x` (lib) due to 1 previous error',
      '',
      'error: test failed, to rerun pass `-p x --lib`',
      '',
    ]);
    assert.equal(events.length, 0);
    assert.equal(rec.summary().failures, 0);
  });

  test('clippy denied warning is class lint', () => {
    const { events } = run([
      'error: unused variable: `x`',
      '  --> src/a.rs:5:9',
      '   |',
      '5  |     let x = 1;',
      '   |         ^ help: if this is intentional, prefix it with an underscore: `_x`',
      '   |',
      '   = note: `-D unused-variables` implied by `-D warnings`',
      '',
    ]);
    const f = failures(events);
    assert.equal(f.length, 1);
    assert.equal(f[0].failure.class, 'lint');
    assert.deepEqual(f[0].failure.locus, { file: 'src/a.rs', line: 5 });
  });

  test('a diagnostic still open at end() is raised', () => {
    const { events } = run(['error[E0425]: cannot find value `y`', '  --> src/b.rs:9:1']);
    assert.equal(failures(events).length, 1);
    assert.equal(failures(events)[0].failure.locus.line, 9);
  });

  test('rustfmt diff is class format', () => {
    const { events } = run(['Diff in /x/src/lib.rs:12:']);
    const f = failures(events);
    assert.equal(f.length, 1);
    assert.equal(f[0].failure.class, 'format');
    assert.deepEqual(f[0].failure.locus, { file: '/x/src/lib.rs', line: 12 });
  });
});

describe('typescript and JS tooling', () => {
  test('tsc in both output shapes is a compile failure with a locus', () => {
    const paren = run(["src/a.ts(4,7): error TS2322: Type 'string' is not assignable to type 'number'."]);
    const pretty = run(["src/a.ts:4:7 - error TS2322: Type 'string' is not assignable to type 'number'."]);
    for (const { events } of [paren, pretty]) {
      const f = failures(events);
      assert.equal(f.length, 1);
      assert.equal(f[0].failure.class, 'compile');
      assert.deepEqual(f[0].failure.locus, { file: 'src/a.ts', line: 4 });
    }
  });

  test('vitest x line and FAIL line for the same test are ONE failure; the assertion only sets a signal', () => {
    const { events, rec } = run([
      ' RUN  v2.1.0 /repo',
      ' × suite > adds 3ms',
      ' FAIL  src/a.test.ts > suite > adds',
      'AssertionError: expected 2 to equal 3',
    ]);
    const f = failures(events);
    assert.equal(f.length, 1);
    assert.equal(f[0].failure.class, 'test');
    assert.equal(f[0].failure.locus.test, 'suite > adds');
    assert.equal(rec.summary().failures, 1);
    assert.equal(rec.signals().assertionFailed, true);
  });

  test('an AssertionError alone raises no failure but sets assertionFailed', () => {
    const { events, rec } = run(['AssertionError: expected 2 to equal 3']);
    assert.equal(events.length, 0);
    assert.equal(rec.signals().assertionFailed, true);
  });

  test('eslint takes the file from the path header and the line from the row', () => {
    const { events } = run([
      '/abs/src/a.ts',
      "  12:5  error  'x' is defined but never used  no-unused-vars",
      '',
      '✖ 1 problem (1 error, 0 warnings)',
    ]);
    const f = failures(events);
    assert.equal(f.length, 1);
    assert.equal(f[0].failure.class, 'lint');
    assert.deepEqual(f[0].failure.locus, { file: '/abs/src/a.ts', line: 12 });
  });

  test('the eslint problem summary alone raises nothing', () => {
    const { events } = run(['✖ 1 problem (1 error, 0 warnings)', '✖ 3 problems (3 errors, 0 warnings)']);
    assert.equal(events.length, 0);
  });

  test('node --test: a failing test is a failure; the "failing tests:" header is not', () => {
    const { events } = run(['✖ failing tests:', '✖ adds (1.2ms)']);
    const f = failures(events);
    assert.equal(f.length, 1);
    assert.equal(f[0].failure.locus.test, 'adds');
  });
});

describe('steps, preflight, environment', () => {
  test('just recipe failure is a step-failed event', () => {
    const { events } = run(['error: Recipe `gate` failed on line 3 with exit code 101']);
    assert.deepEqual(events, [{ kind: 'step-failed', project: 'proj', step: 'gate', exit: 101 }]);
  });

  test('a recipe failure does not raise a compile failure', () => {
    const { rec } = run(['error: Recipe `gate` failed on line 3 with exit code 101']);
    assert.equal(rec.summary().failures, 0);
  });

  test('a refused preflight classifies as environment', () => {
    const { rec } = run(["gate x: pinned Rust toolchain '1.96.1' is not installed; run: rustup toolchain install 1.96.1"]);
    const signals = rec.signals();
    assert.equal(signals.preflight, 'refused');
    assert.equal(classifyFailure(1, signals), 'environment');
  });

  test('a withheld producer offer is a notice, never a failure: no red count, no first red, no signal', () => {
    const text = 'epr producer put withheld (CFLAGS_x86_64_unknown_linux_gnu is not set; gate remains green)';
    const { events, rec } = run(['test result: ok. 3 passed; 0 failed', `[gate] ${text}`, `[gate] ${text}`]);
    assert.deepEqual(failures(events), []);
    assert.deepEqual(events, [{ kind: 'notice', project: 'proj', notice: { class: 'offer-withheld', text } }], 'heard once');
    // The notice contract is the notice schema's class vocabulary (orchestrator-local, E12 L1).
    const schema = JSON.parse(readFileSync(new URL('./gate-live-notice.schema.json', import.meta.url), 'utf8'));
    assert.ok(schema.properties.notice.properties.class.enum.includes(events[0].notice.class));
    assert.deepEqual(rec.summary(), { failures: 0, overflow: 0, firstRedSeconds: null });
    const signals = rec.signals();
    assert.equal(signals.assertionFailed, false);
    assert.equal(signals.compilerError, false);
    assert.equal(classifyFailure(0, signals), undefined);
    // Were the run red for another reason, the offer would not make it a code failure.
    assert.equal(classifyFailure(1, signals), 'unknown');
  });

  test('a skipped producer put is routine and raises nothing', () => {
    const { events } = run(["[gate] epr producer put skipped (epr's source closure was not clean across the build; gate remains green)"]);
    assert.deepEqual(events, []);
  });

  test('a withheld offer does not stand in for an unrecognized red step', () => {
    const lines = [
      '[gate] epr producer put withheld (x; gate remains green)',
      // A finding that quotes the env rule's words is still the advisory offer line.
      '[gate] epr producer put withheld (cc said: Cannot allocate memory; gate remains green)',
    ];
    const events = [];
    const rec = createRecognizer({ project: 'proj', emit: e => events.push(e) });
    for (const line of lines) rec.push(line);
    rec.push('error: Recipe `_gate-x` failed on line 3 with exit code 1');
    rec.end(1);
    assert.deepEqual(failures(events).map(e => e.failure.class), ['step']);
    assert.deepEqual(events.filter(e => e.kind === 'notice').map(e => e.notice.class), ['offer-withheld', 'offer-withheld']);
    assert.equal(rec.summary().failures, 1, 'only the stand-in counts');
  });

  test('out-of-space output is class env', () => {
    const { events } = run(['error writing: No space left on device (os error 28)']);
    assert.equal(failures(events)[0].failure.class, 'env');
  });
});

describe('robustness', () => {
  test('ANSI-coloured lines are recognized identically', () => {
    const plain = run(['test a ... FAILED']).events;
    const colour = run(['\x1b[31mtest a ... FAILED\x1b[0m']).events;
    assert.deepEqual(colour, plain);
    assert.equal(colour.length, 1);
  });

  test('CRLF line endings are tolerated', () => {
    assert.equal(failures(run(['test a ... FAILED\r']).events).length, 1);
  });

  test('identical failure lines twice raise one failure', () => {
    const { events, rec } = run(['test a ... FAILED', 'test a ... FAILED']);
    assert.equal(failures(events).length, 1);
    assert.equal(rec.summary().failures, 1);
  });

  test('past MAX_FAILURES distinct failures are counted as overflow, not emitted', () => {
    const lines = Array.from({ length: MAX_FAILURES + 5 }, (_, i) => `test t${i} ... FAILED`);
    const { events, rec } = run(lines);
    assert.equal(failures(events).length, MAX_FAILURES);
    assert.deepEqual(
      { failures: rec.summary().failures, overflow: rec.summary().overflow },
      { failures: MAX_FAILURES, overflow: 5 },
    );
  });

  test('overflow deduplication stays bounded and never counts repeats', () => {
    const lines = Array.from({ length: MAX_FAILURES * 6 }, (_, i) => `test t${i} ... FAILED`);
    const { events, rec } = run([...lines, ...lines]);
    assert.equal(failures(events).length, MAX_FAILURES);
    assert.equal(rec.summary().overflow, MAX_FAILURES * 4);
  });

  test('details are emitted only for failures admitted under the cap', () => {
    const lines = Array.from({ length: MAX_FAILURES + 1 }, (_, i) => `test t${i} ... FAILED`);
    const { events, rec } = run([
      ...lines,
      '---- t0 stdout ----', 'retained detail',
      `---- t${MAX_FAILURES} stdout ----`, 'overflow detail',
      '---- lone stdout ----', 'unannounced overflow detail',
      `---- t${MAX_FAILURES} stdout ----`, 'repeated overflow detail',
    ]);
    const details = events.filter(e => e.kind === 'detail');
    assert.deepEqual(details.map(e => e.failure.locus.test), ['t0']);
    assert.equal(details[0].failure.fingerprint, failures(events)[0].failure.fingerprint);
    assert.equal(rec.summary().overflow, 2);
  });

  test('excerpts obey the UTF-8 byte budget without splitting characters', () => {
    const kept = ['error: unicode diagnostic', 'é'.repeat(300), '😀'.repeat(300)];
    const { events } = run([...kept, '界'.repeat(100)]);
    const excerpt = failures(events)[0].failure.excerpt;
    assert.ok(Buffer.byteLength(excerpt) <= MAX_EXCERPT_BYTES);
    assert.equal(excerpt, kept.join('\n'));
    assert.equal(excerpt.includes('\ufffd'), false);
  });

  test('firstRedSeconds reads the injected clock', () => {
    const ticks = [1000, 4500, 9000];
    const { rec } = run(['test a ... FAILED', 'test b ... FAILED'], { now: () => ticks.shift() });
    assert.equal(rec.summary().firstRedSeconds, 3.5);
  });

  test('firstRedSeconds is null with no failure', () => {
    const { rec } = run(['running 1 test']);
    assert.equal(rec.summary().firstRedSeconds, null);
  });

  test('unrelated output classifies unknown', () => {
    const { rec } = run(['some banner', 'another line']);
    assert.equal(classifyFailure(1, rec.signals()), 'unknown');
  });

  test('ordinary progress lines raise nothing', () => {
    const { events, rec } = run([
      '   Compiling foo v0.1.0 (/repo/foo)',
      'test ok ... ok',
      'warning: unused import: `std::fmt`',
      '  --> src/a.rs:1:5',
      '',
      '    Finished `test` profile [unoptimized] target(s) in 3.21s',
      'test result: ok. 3 passed; 0 failed; 0 ignored',
    ]);
    assert.equal(events.length, 0);
    assert.equal(rec.summary().failures, 0);
  });
});

describe('normalize and fingerprint', () => {
  test('normalize strips ANSI, hashes, durations and line:col', () => {
    assert.equal(
      normalize('\x1b[31merror at src/a.rs:10:5 deadbeef1 took 12ms\x1b[0m'),
      'error at src/a.rs:# # took #',
    );
  });

  test('normalize collapses whitespace and strips timestamps and IPs', () => {
    assert.equal(normalize('  a   b  10.0.0.1  2026-10-26T10:11:12Z '), 'a b # #');
  });

  test('stripAnsi removes colour and OSC sequences', () => {
    assert.equal(stripAnsi('\x1b[1;31mred\x1b[0m\x1b]0;title\x07!'), 'red!');
  });

  test('fingerprint is stable, 12 hex chars, and sensitive to each part', () => {
    const fp = fingerprint('p', 'test', 'a');
    assert.match(fp, /^[0-9a-f]{12}$/);
    assert.equal(fp, fingerprint('p', 'test', 'a'));
    assert.notEqual(fp, fingerprint('q', 'test', 'a'));
    assert.notEqual(fp, fingerprint('p', 'lint', 'a'));
    assert.notEqual(fp, fingerprint('p', 'test', 'b'));
  });
});

test('gate-steps reports each red step without inventing a second diagnostic', () => {
  const { events } = run([
    'gate-steps: running fmt',
    'gate-steps: fmt failed with exit code 1',
    'gate-steps: running lint',
    'gate-steps: lint failed with exit code 101',
    'gate-steps: running test',
    'test result: ok. 3 passed; 0 failed',
    'error: Recipe `gate` failed on line 7 with exit code 1',
  ]);
  assert.deepEqual(events.map(e => [e.kind, e.step, e.exit]), [
    ['step-failed', 'fmt', 1], ['step-failed', 'lint', 101], ['step-failed', 'gate', 1],
  ]);
});

test('lowercase just recipe summaries remain step events', () => {
  const { events } = run(['error: recipe `gate` failed on line 11 with exit code 1']);
  assert.deepEqual(events.map(e => [e.kind, e.step, e.exit]), [['step-failed', 'gate', 1]]);
});

describe('E6 #2: an unrecognized red step stands in only once the child is known to have failed', () => {
  const RECIPE = name => `error: Recipe \`${name}\` failed on line 3 with exit code 1`;
  const end = (lines, exitCode) => {
    const events = [];
    const rec = createRecognizer({ project: 'proj', emit: e => events.push(e) });
    for (const line of lines) rec.push(line);
    rec.end(exitCode);
    return { events, rec };
  };

  test('a child that exits 0 never sends a red, whatever it printed', () => {
    const { events, rec } = end(['(6:1): expected: #EOF', RECIPE('_gate-x')], 0);
    assert.deepEqual(failures(events), []);
    assert.equal(rec.summary().failures, 0);
  });

  test('a printed exit code 0 marker is no failed step', () => {
    const { events } = end(['error: Recipe `_gate-x` failed on line 3 with exit code 0'], 1);
    assert.deepEqual(events.filter(e => e.kind === 'step-failed'), []);
    assert.deepEqual(failures(events), []);
  });

  test('each distinct opaque failed step stands in once, counted in the summary', () => {
    const { events, rec } = end([RECIPE('_gate-a'), RECIPE('_gate-b'), RECIPE('_gate-a')], 1);
    const stand = failures(events);
    assert.deepEqual(stand.map(e => [e.step, e.failure.class, e.failure.nth]), [['_gate-a', 'step', 1], ['_gate-b', 'step', 2]]);
    assert.match(stand[0].failure.fingerprint, /^[0-9a-f]{12}$/);
    assert.deepEqual(stand[0].failure.locus, {});
    assert.equal(typeof stand[0].failure.excerpt, 'string');
    assert.equal(rec.summary().failures, 2, 'the fd-3 summary and the final agree');
  });

  test('a recognized failure anywhere in the run means no stand-in, before or after the marker', () => {
    const real = 'error[E0425]: cannot find value `x` in this scope';
    for (const lines of [[RECIPE('_gate-a'), real, '  --> src/lib.rs:1:1', ''], [real, '  --> src/lib.rs:1:1', '', RECIPE('_gate-a')]]) {
      const { events } = end(lines, 1);
      assert.ok(failures(events).every(e => e.failure.class !== 'step'), JSON.stringify(failures(events)));
      assert.ok(failures(events).length >= 1);
    }
  });
});

describe('the orchestrator suite named half', () => {
  const one = line => failures(run([line]).events);
  test('an undeclared skip is a test red naming its file and test', () => {
    const [f] = one('[suite] named: skipped and not in allowedSkips: gate-cost.test.mjs › e11 proof: a skip (proof only)');
    assert.equal(f.failure.class, 'test');
    assert.deepEqual(f.failure.locus, { file: 'gate-cost.test.mjs', test: 'e11 proof: a skip' });
  });
  test('a missing group skip report is a test red', () => {
    const [f] = one('[suite] named: group 3 wrote no skip report (/tmp/x/skips/group-3.jsonl)');
    assert.equal(f.failure.class, 'test');
    assert.equal(f.failure.locus.test, 'named group 3 skip report');
  });
  test('a failed group is a step red', () => {
    const [f] = one('[suite] named group (node) failed with 1: 42 file(s)');
    assert.equal(f.failure.class, 'step');
    assert.equal(f.failure.locus.test, 'named group (node)');
  });
  test('a named half that did not report is a step red', () => {
    const [f] = one('[gate] orchestrator evidence: not recorded (the named half did not report (no result))');
    assert.equal(f.failure.class, 'step');
  });
});
