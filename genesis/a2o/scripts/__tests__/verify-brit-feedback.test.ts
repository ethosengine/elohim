import assert from 'node:assert/strict';
import { existsSync, mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import { verifyBritFeedback } from '../verify-brit-feedback.js';

void test('missing installed executables are unmeasured, never green', async () => {
  const result = await verifyBritFeedback({ env: { PATH: '/missing-feedback-probe' } });
  assert.equal(result.outcome, 'skipped');
  assert.match(result.summary, /unmeasured/);
});

void test('incompatible explicit pair fails instead of claiming capture', async () => {
  const result = await verifyBritFeedback({ epr: process.execPath, brit: process.execPath });
  assert.equal(result.outcome, 'failed');
  assert.match(result.summary, /required native feedback interface/);
});

void test('candidate selection requires both executables', async () => {
  const result = await verifyBritFeedback({ epr: process.execPath });
  assert.equal(result.outcome, 'failed');
  assert.match(result.summary, /both/);
});

void test('native absence is unmeasured while an incompatible resolver still fails', async () => {
  const root = mkdtempSync(join(tmpdir(), 'feedback-absence-test-'));
  try {
    const resolver = join(root, 'epr');
    for (const [diagnostic, expected] of [
      ['REFUSED head-absent: no committed current head', 'skipped'],
      ['REFUSED arguments: unknown bin verb', 'failed'],
    ] as const) {
      writeFileSync(resolver, `#!/bin/sh\nprintf '%s\\n' '${diagnostic}' >&2\nexit 2\n`, {
        mode: 0o700,
      });
      const result = await verifyBritFeedback({ resolver, env: {} });
      assert.equal(result.outcome, expected);
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

void test('default resolution ignores inherited candidates and requests installed current', async () => {
  const root = mkdtempSync(join(tmpdir(), 'feedback-resolver-test-'));
  try {
    mkdirSync(join(root, 'bin'));
    // This fake only witnesses resolution: it cannot fabricate successful feedback capture.
    writeFileSync(
      join(root, 'bin/epr'),
      '#!/bin/sh\n[ -z "${EPR_BIN:-}${BRIT_BIN:-}${EPR_PIN_CID:-}${BRIT_PIN_CID:-}" ] || exit 17\n[ "$1 $2 $3 $4 $5 $6" = "bin resolve --tool epr --current --json" ] || exit 18\nprintf \'{"source":"candidate","path":"/untrusted","cid":"untrusted"}\\n\'\n',
      { mode: 0o700 }
    );
    const result = await verifyBritFeedback({
      env: {
        PATH: join(root, 'bin'),
        EPR_BIN: join(root, 'bin/epr'),
        BRIT_BIN: '/inherited-brit',
        EPR_PIN_CID: 'old-epr',
        BRIT_PIN_CID: 'old-brit',
      },
    });
    assert.equal(result.outcome, 'failed');
    assert.match(result.summary, /did not identify an installed current release/);
    assert.deepEqual((result.observed as { ignoredOverrides: string[] }).ignoredOverrides, [
      'EPR_BIN',
      'BRIT_BIN',
      'EPR_PIN_CID',
      'BRIT_PIN_CID',
    ]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

void test('an older launcher is unmeasured without entering its session-pinning alias', async () => {
  const root = mkdtempSync(join(tmpdir(), 'feedback-launcher-test-'));
  try {
    const launcher = join(root, 'bin-launch');
    const marker = join(root, 'pin-written');
    writeFileSync(
      launcher,
      '#!/bin/sh\n[ "$1 $2 $3 $4 $5" = "resolve --tool epr --current --json" ] || printf pin > "$PROBE_PIN_MARKER"\nprintf "REFUSED launcher-name-invalid: bin-launch resolves to 0 tools\\n" >&2\nexit 127\n',
      {
        mode: 0o700,
      }
    );
    symlinkSync(launcher, join(root, 'epr'));
    const result = await verifyBritFeedback({ env: { PATH: root, PROBE_PIN_MARKER: marker } });
    assert.equal(result.outcome, 'skipped');
    assert.match(result.summary, /direct EPR executable/);
    assert.equal(existsSync(marker), false);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

void test('current launcher resolves both tools through its own read-only command', async () => {
  const root = mkdtempSync(join(tmpdir(), 'feedback-launcher-current-test-'));
  try {
    const launcher = join(root, 'bin-launch');
    const marker = join(root, 'pin-written');
    writeFileSync(
      launcher,
      '#!/bin/sh\n[ "$1 $2 $4 $5" = "resolve --tool --current --json" ] || printf pin > "$PROBE_PIN_MARKER"\nprintf \'{"source":"current","path":"%s","cid":"fixture-%s"}\\n\' "$PROBE_CURRENT" "$3"\n',
      { mode: 0o700 }
    );
    symlinkSync(launcher, join(root, 'epr'));
    const result = await verifyBritFeedback({
      env: { PATH: root, PROBE_CURRENT: process.execPath, PROBE_PIN_MARKER: marker },
    });
    assert.equal(result.outcome, 'failed');
    assert.match(result.summary, /required native feedback interface/);
    const observed = result.observed as { pair: { epr: { cid: string }; brit: { cid: string } } };
    assert.equal(observed.pair.epr.cid, 'fixture-epr');
    assert.equal(observed.pair.brit.cid, 'fixture-brit');
    assert.equal(existsSync(marker), false);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

void test('adapter refuses recorder-only and inconsistent qualification witnesses', async () => {
  const root = mkdtempSync(join(tmpdir(), 'feedback-witness-test-'));
  try {
    const epr = join(root, 'epr');
    for (const [mode, qualified, code] of [
      ['recorder-only', false, 0],
      ['pair', false, 0],
      ['pair', true, 2],
    ] as const) {
      const witness = {
        checkId: 'private-feedback-loop',
        outcome: 'passed',
        summary: 'fixture',
        observed: { mode, pairQualified: qualified, fixtureOnly: true },
      };
      writeFileSync(epr, `#!/bin/sh\nprintf '%s\\n' '${JSON.stringify(witness)}'\nexit ${code}\n`, {
        mode: 0o700,
      });
      const result = await verifyBritFeedback({ epr, brit: process.execPath });
      assert.equal(result.outcome, 'failed');
      assert.match(result.summary, /inconsistent witness/);
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

void test('adapter preserves measured failure and unavailable native prerequisites', async () => {
  const root = mkdtempSync(join(tmpdir(), 'feedback-native-result-test-'));
  try {
    const epr = join(root, 'epr');
    for (const outcome of ['failed', 'skipped'] as const) {
      const witness = {
        checkId: 'private-feedback-loop',
        outcome,
        summary: 'native prerequisite',
        observed: { mode: 'pair', pairQualified: false, fixtureOnly: true },
      };
      writeFileSync(epr, `#!/bin/sh\nprintf '%s\\n' '${JSON.stringify(witness)}'\nexit 2\n`, {
        mode: 0o700,
      });
      const result = await verifyBritFeedback({ epr, brit: process.execPath });
      assert.equal(result.outcome, outcome);
      assert.equal(result.summary, 'native prerequisite');
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

void test(
  'real candidate pair carries outcomes into a native proposal in isolation',
  {
    skip: !process.env.FEEDBACK_TEST_EPR || !process.env.FEEDBACK_TEST_BRIT,
  },
  async () => {
    const result = await verifyBritFeedback({
      epr: process.env.FEEDBACK_TEST_EPR,
      brit: process.env.FEEDBACK_TEST_BRIT,
    });
    assert.equal(result.outcome, 'passed', result.summary);
    const observed = result.observed as {
      fixtureOnly: boolean;
      admitted: boolean;
      recorderFailurePreservesExit: boolean;
      pair: { epr: { source: string }; brit: { source: string } };
      occurrences: string[];
    };
    assert.equal(observed.fixtureOnly, true);
    assert.equal(observed.admitted, false);
    assert.equal(observed.recorderFailurePreservesExit, true);
    assert.equal(observed.pair.epr.source, 'explicit-candidate');
    assert.equal(observed.pair.brit.source, 'explicit-candidate');
    assert.equal(observed.occurrences.length, 2);
  }
);
