import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { After, Given, Then, When } from '@cucumber/cucumber';

import { verifyBritFeedback } from '../../scripts/verify-brit-feedback.js';

import type { CheckWitness } from '../../scripts/lib/performance-report.js';

const FIXTURE_PARTICIPANT = 'human:fixture';

interface FeedbackWorld {
  feedbackRoot?: string;
  feedbackRows?: { occurrence: { outcome: string; count: number; message: string } }[];
  feedbackPair?: { epr: string; brit: string };
  feedbackProbe?: CheckWitness;
  feedbackOccurrence?: string;
  feedbackContext?: {
    reconciliation: { assertions: { issues: string[]; acceptance: string }[] };
  };
}

Given(
  'an explicitly selected Brit and EPR pair for a private fixture',
  function (this: FeedbackWorld) {
    assert.ok(process.env.FEEDBACK_TEST_EPR, 'Select FEEDBACK_TEST_EPR explicitly.');
    assert.ok(process.env.FEEDBACK_TEST_BRIT, 'Select FEEDBACK_TEST_BRIT explicitly.');
    this.feedbackPair = {
      epr: process.env.FEEDBACK_TEST_EPR,
      brit: process.env.FEEDBACK_TEST_BRIT,
    };
  }
);
When(
  'the developer checks feedback from a successful command and a refused command',
  async function (this: FeedbackWorld) {
    this.feedbackProbe = await verifyBritFeedback(this.feedbackPair);
  }
);
Then(
  'both experiences survive reopening without retaining the private argument',
  function (this: FeedbackWorld) {
    assert.equal(this.feedbackProbe?.outcome, 'passed', this.feedbackProbe?.summary);
  }
);
Then(
  'the refused experience produces a native proposal without admitting repair',
  function (this: FeedbackWorld) {
    const observed = this.feedbackProbe?.observed as { admitted: boolean; proposal: string };
    assert.equal(observed.admitted, false);
    assert.ok(observed.proposal);
  }
);
Then('a recorder failure preserves the command exits', function (this: FeedbackWorld) {
  assert.equal(
    (this.feedbackProbe?.observed as { recorderFailurePreservesExit: boolean })
      .recorderFailurePreservesExit,
    true
  );
});
Then(
  'the check identifies candidate selection without claiming installed delivery',
  function (this: FeedbackWorld) {
    const observed = this.feedbackProbe?.observed as {
      fixtureOnly: boolean;
      pair: { epr: { source: string }; brit: { source: string } };
    };
    assert.equal(observed.fixtureOnly, true);
    assert.equal(observed.pair.epr.source, 'explicit-candidate');
    assert.equal(observed.pair.brit.source, 'explicit-candidate');
  }
);
function invoke(world: FeedbackWorld, command: string[], input?: unknown): Record<string, unknown> {
  const result = spawnSync(
    process.env.EPR_BIN ?? 'epr',
    ['feedback', ...command, '--root', world.feedbackRoot!, '--json'],
    {
      encoding: 'utf8',
      input: input === undefined ? undefined : JSON.stringify(input),
      timeout: 10000,
    }
  );
  assert.equal(result.status, 0, result.stderr);
  return JSON.parse(result.stdout) as Record<string, unknown>;
}
Given('a private feedback workspace with no Git repository', function (this: FeedbackWorld) {
  this.feedbackRoot = mkdtempSync(join(tmpdir(), 'feedback-story-'));
  mkdirSync(join(this.feedbackRoot, '.epr-meta'));
  writeFileSync(
    join(this.feedbackRoot, '.epr-meta/feedback-policy.json'),
    JSON.stringify({
      id: 'story@1',
      authority: FIXTURE_PARTICIPANT,
      participant: FIXTURE_PARTICIPANT,
      producers: ['human', 'genesis'],
      automaticCapture: true,
      deliberateEvidence: true,
      maxBytes: 1000000,
      maxRecordBytes: 65536,
      response: 'repair',
      sharing: 'separately-authorized',
    })
  );
});
Given(
  'the workspace has no repair commitment or execution admission',
  function (this: FeedbackWorld) {
    assert.ok(!existsSync(join(this.feedbackRoot!, '.eprfs/status/flows.jsonl')));
  }
);
When('a participant records a need for bounded history', function (this: FeedbackWorld) {
  const record = invoke(this, ['record'], {
    kind: 'occurrence',
    source: { producer: 'human', run: 'one', item: 'need' },
    observedAt: '2026-11-08T15:00:00.000Z',
    participant: FIXTURE_PARTICIPANT,
    subject: 'history',
    outcome: 'reported',
    count: 1,
    durationMs: null,
    impact: null,
    message: 'I need bounded history',
    detail: null,
  });
  this.feedbackOccurrence = record.cid as string;
});
When('the participant proposes work from that retained need', function (this: FeedbackWorld) {
  invoke(this, [
    'propose',
    '--id',
    this.feedbackOccurrence!,
    '--as',
    FIXTURE_PARTICIPANT,
    '--objective',
    'Make bounded history usable',
  ]);
  const result = spawnSync(
    process.env.EPR_BIN ?? 'epr',
    ['flow', 'context', this.feedbackOccurrence!, '--root', this.feedbackRoot!, '--json'],
    { encoding: 'utf8', timeout: 10000 }
  );
  assert.equal(result.status, 0, result.stderr);
  this.feedbackContext = JSON.parse(result.stdout) as FeedbackWorld['feedbackContext'];
});
Then(
  'EPR shows the original need as the available source for the proposal',
  function (this: FeedbackWorld) {
    const assertions = this.feedbackContext?.reconciliation.assertions;
    assert.equal(assertions?.length, 1);
    assert.ok(!assertions[0].issues.some(issue => issue.includes('source unestablished')));
    assert.ok(!existsSync(join(this.feedbackRoot!, '.eprfs/status/labels.json')));
  }
);
Then('recognizing the evidence grants no repair acceptance', function (this: FeedbackWorld) {
  assert.notEqual(this.feedbackContext?.reconciliation.assertions[0].acceptance, 'accepted');
  assert.ok(!existsSync(join(this.feedbackRoot!, '.eprfs/status/acceptance.jsonl')));
});
When('the participant opens feedback again', function (this: FeedbackWorld) {
  this.feedbackRows = invoke(this, ['list']).experiences as FeedbackWorld['feedbackRows'];
});
Then('the original need remains readable', function (this: FeedbackWorld) {
  assert.equal(this.feedbackRows?.length, 1);
  assert.equal(this.feedbackRows[0].occurrence.message, 'I need bounded history');
});
Then(
  'feedback repair reports missing commitment and execution admission without starting work',
  function (this: FeedbackWorld) {
    const result = invoke(this, ['triage', '--response', 'repair']);
    const rows = result.rows as { admitted: boolean; waiting: string[] }[];
    assert.equal(rows[0].admitted, false);
    assert.ok(rows[0].waiting.some(reason => reason.includes('commitment')));
    assert.ok(rows[0].waiting.some(reason => reason.includes('execution')));
  }
);
When(
  'a Genesis report with one finding reported nine times, private diagnostics and a URL, one skipped concern, and one absent declared concern is ingested twice',
  function (this: FeedbackWorld) {
    const report = {
      runId: 'one',
      generatedAt: '2026-11-08T15:00:00Z',
      findings: [
        {
          fingerprint: 'finding',
          source: 'console-error',
          severity: 'error',
          occurrences: 9,
          message: 'PRIVATE_DIAGNOSTIC',
          firstSeenUrl: 'https://private.invalid',
        },
      ],
      summary: { byConcern: { skipped: { passed: 0, failed: 0, pending: 1, skipped: 1 } } },
      declared: { concerns: ['skipped', 'absent'] },
    };
    invoke(this, ['ingest-genesis'], report);
    invoke(this, ['ingest-genesis'], { ...report, generatedAt: '2026-11-09T15:00:00Z' });
    this.feedbackRows = invoke(this, ['list']).experiences as FeedbackWorld['feedbackRows'];
  }
);
Then(
  'its finding remains one aggregate with nine reported occurrences',
  function (this: FeedbackWorld) {
    assert.equal(this.feedbackRows?.length, 4);
    assert.equal(this.feedbackRows[0].occurrence.count, 9);
  }
);
Then(
  'the feedback list shows skipped coverage distinct from unmeasured coverage',
  function (this: FeedbackWorld) {
    assert.ok(this.feedbackRows?.some(row => row.occurrence.outcome === 'skipped'));
    assert.ok(this.feedbackRows?.some(row => row.occurrence.outcome === 'unmeasured'));
  }
);
Then(
  'private diagnostics and URLs are absent from feedback retention',
  function (this: FeedbackWorld) {
    const bytes = readFileSync(join(this.feedbackRoot!, '.eprfs/status/feedback.jsonl'), 'utf8');
    assert.ok(!bytes.includes('PRIVATE_DIAGNOSTIC'));
    assert.ok(!bytes.includes('private.invalid'));
  }
);
After({ tags: '@concern:private-feedback-loop' }, function (this: FeedbackWorld) {
  if (this.feedbackRoot) rmSync(this.feedbackRoot, { recursive: true, force: true });
});

Given(
  'a version-pinned local retention policy and a migrated feedback store',
  function (this: FeedbackWorld) {
    const registry = new URL('../../../../.claude/epr-meta/policies.yaml', import.meta.url);
    mkdirSync(join(this.feedbackRoot!, '.claude/epr-meta'), { recursive: true });
    writeFileSync(
      join(this.feedbackRoot!, '.claude/epr-meta/policies.yaml'),
      readFileSync(registry)
    );
    writeFileSync(
      join(this.feedbackRoot!, '.epr-meta/feedback-capacity.json'),
      JSON.stringify({
        policy: 'feedback-local-retention@1',
        params: {
          totalBytes: 240000,
          activeBytes: 80000,
          replacementBytes: 80000,
          controlBytes: 80000,
          recordBytes: 20000,
        },
      })
    );
    const path = join(this.feedbackRoot!, '.epr-meta/feedback-policy.json');
    const policy = JSON.parse(readFileSync(path, 'utf8')) as { producers: string[] };
    policy.producers.push('brit');
    writeFileSync(path, JSON.stringify(policy));
    invoke(this, ['migrate', '--legacy-writers-quiesced']);
  }
);
When(
  'old automatic successes reach the warning threshold beside a protected need',
  function (this: FeedbackWorld) {
    const at = new Date(Date.now() - 10 * 86400000).toISOString();
    const occurrence = (run: string, producer: string, outcome: string) => ({
      kind: 'occurrence',
      source: { producer, run, item: 'status' },
      observedAt: at,
      participant: FIXTURE_PARTICIPANT,
      subject: 'history',
      outcome,
      count: 1,
      durationMs: null,
      impact: null,
      message: null,
      detail: null,
    });
    const protectedRecord = invoke(
      this,
      ['record', '--protect'],
      occurrence('need', 'human', 'reported')
    );
    writeFileSync(join(this.feedbackRoot!, 'protected-cid.txt'), String(protectedRecord.cid));
    for (let i = 0; i < 100; i++) {
      const old = invoke(this, ['record'], occurrence(`old-${i}`, 'brit', 'passed'));
      if (Number(invoke(this, ['capacity']).retainedBytes) >= 68000) return;
      // Explicit author classification closes the provenance loop of carried command evidence.
      invoke(this, [
        'evidence',
        '--id',
        String(old.cid),
        '--evidence',
        String(old.cid),
        '--relation',
        'routine-success',
        '--as',
        FIXTURE_PARTICIPANT,
        '--outcome',
        'passed',
      ]);
      if (Number(invoke(this, ['capacity']).retainedBytes) >= 68000) return;
    }
    assert.fail('fixture did not cross the warning threshold');
  }
);
Then(
  'a dry run shows eligible bytes without changing retained evidence',
  function (this: FeedbackWorld) {
    const state = join(this.feedbackRoot!, '.eprfs/status/feedback/state.json');
    const before = readFileSync(state, 'utf8');
    const report = invoke(this, ['reclaim', '--dry-run']);
    assert.ok(Number(report.selectedBytes) > 0);
    assert.equal(readFileSync(state, 'utf8'), before);
  }
);
When('the participant executes feedback reclamation', function (this: FeedbackWorld) {
  invoke(this, ['reclaim']);
});
Then(
  'the protected need keeps its address and capacity returns below reset',
  function (this: FeedbackWorld) {
    const id = readFileSync(join(this.feedbackRoot!, 'protected-cid.txt'), 'utf8');
    invoke(this, ['show', '--id', id]);
    const report = invoke(this, ['capacity']);
    assert.ok(Number(report.retainedBytes) <= 56000);
    assert.ok(Number(report.totalManagedFileBytes) <= 240000);
  }
);
Then(
  'the capacity report explains the bounded history window and policy identity',
  function (this: FeedbackWorld) {
    const report = invoke(this, ['capacity']);
    assert.match(String(report.history), /retained records only/);
    const policy = report.policy as { policy: string; policyCid: string };
    assert.equal(policy.policy, 'feedback-local-retention@1');
    assert.match(policy.policyCid, /^bafy/);
  }
);
