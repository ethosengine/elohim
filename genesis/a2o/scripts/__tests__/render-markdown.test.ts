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

  void it('renders the Visual Validation section when summary.visualValidation is present', () => {
    const reportWithVisual: SprintReport = {
      ...report,
      profile: 'browser',
      summary: {
        ...report.summary,
        visualValidation: {
          validatedPassing: 12,
          validatedRegressed: 3,
          pendingPassing: 47,
          pendingFailing: 18,
        },
      },
    };
    const md = renderMarkdown(reportWithVisual);
    assert.match(md, /## Visual Validation/);
    assert.match(md, /\*\*12\*\* validatedPassing/);
    assert.match(md, /\*\*3\*\* validatedRegressed/);
    assert.match(md, /\*\*47\*\* pendingPassing/);
    assert.match(md, /\*\*18\*\* pendingFailing/);
  });

  void it('omits the Visual Validation section when summary.visualValidation is absent', () => {
    const md = renderMarkdown(report);
    assert.doesNotMatch(md, /## Visual Validation/);
  });

  void it('renders screenshotPath as a Screenshot bullet on findings that have one', () => {
    const reportWithRegression: SprintReport = {
      ...report,
      findings: [
        {
          fingerprint: 'reg123',
          source: 'visual-regression',
          pillar: 'lamad',
          severity: 'error',
          message: 'Validated visual regressed: Starting a Journey',
          screenshotPath:
            'reports/screenshots/lamad-learning-journey/starting-a-journey--Matthew.png',
          occurrences: 1,
          scenarios: [
            {
              name: 'Starting a Journey',
              feature: 'features/lamad/learning-journey.feature',
              human: 'Matthew',
            },
          ],
          suggestedObjective: 'Restore visual delivery of: Starting a Journey',
        },
      ],
    };
    const md = renderMarkdown(reportWithRegression);
    assert.match(md, /Screenshot.*lamad-learning-journey\/starting-a-journey--Matthew\.png/);
  });
});
