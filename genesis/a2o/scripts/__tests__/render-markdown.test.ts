import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { renderMarkdown } from '../lib/render-markdown.js';

import type { SprintReport } from '../lib/aggregate.js';

const report: SprintReport = {
  generatedAt: '2026-04-19T10:00:00Z',
  runId: 'build-123',
  profile: 'alpha',
  doorway: 'https://doorway-alpha.elohim.host',
  summary: {
    scenarios: { total: 3, passed: 1, failed: 1, skipped: 0, pending: 1 },
    findings: {
      total: 3,
      bySource: { 'console-error': 1, 'scenario-failure': 1, 'pending-step': 1 },
      byPillar: { browser: 1, lamad: 1, imagodei: 1 },
    },
  },
  findings: [
    {
      fingerprint: 'abc123def456',
      source: 'console-error',
      pillar: 'browser',
      severity: 'error',
      message: 'ReferenceError: Sophia is not defined',
      firstSeenUrl: 'https://doorway-alpha.elohim.host/a.js',
      occurrences: 2,
      scenarios: [
        { name: 'learning-journey', feature: 'browser', human: 'timothy' },
        { name: 'learning-journey', feature: 'browser', human: 'mary' },
      ],
      suggestedObjective: 'Fix browser console error: ReferenceError: Sophia is not defined',
    },
  ],
};

void describe('renderMarkdown', () => {
  void it('includes the run id and profile in the header', () => {
    const md = renderMarkdown(report);
    assert.match(md, /A2O Sprint Report/);
    assert.match(md, /build-123/);
    assert.match(md, /alpha/);
  });

  void it('renders summary counts', () => {
    const md = renderMarkdown(report);
    assert.match(md, /passed\D*1/i);
    assert.match(md, /failed\D*1/i);
  });

  void it('groups findings by pillar header', () => {
    const md = renderMarkdown(report);
    assert.match(md, /## .*browser/i);
  });

  void it('includes fingerprint, occurrences, and suggested objective', () => {
    const md = renderMarkdown(report);
    assert.match(md, /abc123def456/);
    assert.match(md, /occurrences.*2/i);
    assert.match(md, /Fix browser console error/);
  });

  void it('lists each scenario that triggered the finding', () => {
    const md = renderMarkdown(report);
    assert.match(md, /timothy/);
    assert.match(md, /mary/);
  });
});
