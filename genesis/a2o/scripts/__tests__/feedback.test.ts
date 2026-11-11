import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import { aggregate } from '../lib/aggregate.js';
import { captureFeedback } from '../lib/feedback.js';

import type { SprintReport } from '../lib/aggregate.js';

void test('capture passes the validated report unchanged and performs no publication', () => {
  const report = { runId: 'run-1', findings: [{ occurrences: 7 }] } as unknown as SprintReport;
  const before = JSON.stringify(report);
  let calls = 0;
  assert.equal(
    captureFeedback(report, '/fixture', (actual, root) => {
      calls++;
      assert.equal(actual, report);
      assert.equal(root, '/fixture');
      return true;
    }),
    true
  );
  assert.equal(calls, 1);
  assert.equal(JSON.stringify(report), before);
});

void test('recorder refusal and exception expose one bounded gap without changing verdict', () => {
  const report = { summary: { scenarios: { failed: 1 } } } as unknown as SprintReport;
  for (const recorder of [
    () => false,
    () => {
      throw new Error('private diagnostic');
    },
  ]) {
    const warnings: string[] = [];
    assert.equal(
      captureFeedback(report, '/fixture', recorder, message => warnings.push(message)),
      false
    );
    assert.equal(warnings.length, 1);
    assert.ok(!warnings[0].includes('private diagnostic'));
    assert.equal(report.summary.scenarios.failed, 1);
  }
});

void test(
  'the real Genesis aggregate crosses the native feedback boundary',
  { skip: !process.env.EPR_BIN },
  () => {
    const root = mkdtempSync(join(tmpdir(), 'feedback-genesis-'));
    try {
      mkdirSync(join(root, '.epr-meta'));
      writeFileSync(
        join(root, '.epr-meta/feedback-policy.json'),
        JSON.stringify({
          id: 'test@1',
          authority: 'human:fixture',
          participant: null,
          producers: ['genesis'],
          automaticCapture: true,
          deliberateEvidence: true,
          maxBytes: 1000000,
          maxRecordBytes: 65536,
          response: 'report',
          sharing: 'separately-authorized',
        })
      );
      const report = aggregate({
        scenarios: [
          {
            name: 'failed story',
            feature: 'features/private.feature',
            status: 'failed',
            tags: ['@concern:example'],
            failureMessage: 'PRIVATE_DIAGNOSTIC https://secret.invalid',
          },
        ],
        consoleArtifacts: [],
        gaps: [],
        runId: 'production-shape',
        profile: 'local',
      });
      const before = JSON.stringify(report);
      assert.equal(captureFeedback(report, root), true);
      assert.equal(JSON.stringify(report), before);
      const read = spawnSync(process.env.EPR_BIN!, ['feedback', 'list', '--root', root, '--json'], {
        encoding: 'utf8',
      });
      assert.equal(read.status, 0, read.stderr);
      const view = JSON.parse(read.stdout) as {
        experiences: { occurrence: { outcome: string; detail: { adapter: string } } }[];
      };
      assert.ok(
        view.experiences.some(
          row =>
            row.occurrence.detail.adapter === 'genesis-concern@1' &&
            row.occurrence.outcome === 'failed'
        )
      );
      const retained = readFileSync(join(root, '.eprfs/status/feedback.jsonl'), 'utf8');
      assert.ok(!retained.includes('PRIVATE_DIAGNOSTIC'));
      assert.ok(!retained.includes('secret.invalid'));
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  }
);
