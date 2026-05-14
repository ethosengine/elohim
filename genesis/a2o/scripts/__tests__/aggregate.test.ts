import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { aggregate } from '../lib/aggregate.js';

import type { ConsoleArtifact } from '../lib/load-console.js';
import type { GapFinding } from '../lib/load-coverage-gap.js';
import type { ScenarioResult } from '../lib/load-cucumber.js';

function input() {
  const scenarios: ScenarioResult[] = [
    {
      name: 'Terrance completes path',
      feature: 'features/lamad/learning-journey.feature',
      status: 'passed',
      tags: [],
    },
    {
      name: 'Mary fails on assessment',
      feature: 'features/lamad/learning-journey.feature',
      status: 'failed',
      failureMessage: 'AssertionError: expected 500 to be 200',
      tags: [],
    },
    {
      name: 'Stub not implemented',
      feature: 'features/auth/fixture-humans.feature',
      status: 'pending',
      tags: [],
    },
  ];
  const console: ConsoleArtifact[] = [
    {
      scenario: 'learning-journey',
      human: 'terrance',
      consoleErrors: [
        {
          level: 'error',
          text: 'ReferenceError: Sophia is not defined',
          url: 'https://doorway-alpha.elohim.host/a.js',
        },
      ],
      pageErrors: [],
    },
    {
      scenario: 'learning-journey',
      human: 'mary',
      consoleErrors: [
        {
          level: 'error',
          text: 'ReferenceError: Sophia is not defined',
          url: 'https://doorway-alpha.elohim.host/a.js',
        },
      ],
      pageErrors: [
        {
          message: "TypeError: Cannot read properties of null (reading 'token')",
          url: 'https://doorway-alpha.elohim.host/login',
        },
      ],
    },
  ];
  const gaps: GapFinding[] = [
    {
      feature: 'features/elohim/presence.feature',
      missing: 'presence claim expires',
      severity: 'medium',
    },
  ];
  return { scenarios, console, gaps };
}

const SCENARIO_FAILURE = 'scenario-failure' as const;
const VISUAL_REGRESSION = 'visual-regression' as const;

void describe('aggregate', () => {
  void it('counts scenarios in summary', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({
      scenarios,
      consoleArtifacts: console,
      gaps,
      runId: 'r1',
      profile: 'alpha',
      doorway: 'https://d.alpha',
    });
    assert.equal(r.summary.scenarios.total, 3);
    assert.equal(r.summary.scenarios.passed, 1);
    assert.equal(r.summary.scenarios.failed, 1);
    assert.equal(r.summary.scenarios.pending, 1);
  });

  void it('dedupes identical console errors into one finding with occurrences=2', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({
      scenarios,
      consoleArtifacts: console,
      gaps,
      runId: 'r1',
      profile: 'alpha',
    });
    const sophia = r.findings.find(f => f.message.includes('Sophia is not defined'))!;
    assert.ok(sophia);
    assert.equal(sophia.occurrences, 2);
    assert.equal(sophia.source, 'console-error');
    assert.equal(sophia.scenarios.length, 2);
  });

  void it('includes scenario-failure findings', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({
      scenarios,
      consoleArtifacts: console,
      gaps,
      runId: 'r1',
      profile: 'alpha',
    });
    const failure = r.findings.find(f => f.source === SCENARIO_FAILURE)!;
    assert.ok(failure);
    assert.match(failure.message, /AssertionError/);
    assert.equal(failure.pillar, 'lamad');
  });

  void it('includes pending-step findings', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({
      scenarios,
      consoleArtifacts: console,
      gaps,
      runId: 'r1',
      profile: 'alpha',
    });
    const pending = r.findings.find(f => f.source === 'pending-step');
    assert.ok(pending);
    assert.equal(pending.pillar, 'imagodei');
  });

  void it('includes coverage-gap findings', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({
      scenarios,
      consoleArtifacts: console,
      gaps,
      runId: 'r1',
      profile: 'alpha',
    });
    const gap = r.findings.find(f => f.source === 'coverage-gap')!;
    assert.ok(gap);
    assert.equal(gap.pillar, 'elohim');
  });

  void it('sorts findings by occurrences desc', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({
      scenarios,
      consoleArtifacts: console,
      gaps,
      runId: 'r1',
      profile: 'alpha',
    });
    for (let i = 1; i < r.findings.length; i++) {
      assert.ok(r.findings[i - 1].occurrences >= r.findings[i].occurrences);
    }
  });

  void it('emits suggested objective headlines', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({
      scenarios,
      consoleArtifacts: console,
      gaps,
      runId: 'r1',
      profile: 'alpha',
    });
    for (const f of r.findings) {
      assert.ok(f.suggestedObjective && f.suggestedObjective.length > 0);
    }
  });

  function visualScenarios(): ScenarioResult[] {
    return [
      // validated-passing
      {
        name: 'Validated and passing',
        feature: 'features/lamad/a.feature',
        status: 'passed',
        tags: ['@e2e', '@elohim-visually-validated'],
      },
      // validated-regressed
      {
        name: 'Validated but failed',
        feature: 'features/lamad/b.feature',
        status: 'failed',
        failureMessage: 'AssertionError: visual element missing',
        tags: ['@e2e', '@elohim-visually-validated'],
      },
      // pending-passing
      {
        name: 'Untagged passing',
        feature: 'features/lamad/c.feature',
        status: 'passed',
        tags: ['@e2e'],
      },
      // pending-failing
      {
        name: 'Untagged failed',
        feature: 'features/lamad/d.feature',
        status: 'failed',
        failureMessage: 'AssertionError: nope',
        tags: ['@e2e'],
      },
    ];
  }

  void it('emits summary.visualValidation when profile is browser', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'browser',
    });
    assert.ok(r.summary.visualValidation, 'visualValidation should be present');
    assert.equal(r.summary.visualValidation.validatedPassing, 1);
    assert.equal(r.summary.visualValidation.validatedRegressed, 1);
    assert.equal(r.summary.visualValidation.pendingPassing, 1);
    assert.equal(r.summary.visualValidation.pendingFailing, 1);
  });

  void it('emits summary.visualValidation when profile is delivery-browser', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'delivery-browser',
    });
    assert.ok(r.summary.visualValidation);
  });

  void it('omits summary.visualValidation when profile is alpha', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'alpha',
    });
    assert.equal(r.summary.visualValidation, undefined);
  });

  void it('omits summary.visualValidation when profile is local', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'local',
    });
    assert.equal(r.summary.visualValidation, undefined);
  });

  void it('emits a visual-regression finding for tagged-and-failed scenarios in playwright mode', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'browser',
    });
    const regression = r.findings.find(f => f.source === VISUAL_REGRESSION);
    assert.ok(regression, 'expected a visual-regression finding');
    assert.equal(regression.scenarios.length, 1);
    assert.equal(regression.scenarios[0].name, 'Validated but failed');
    assert.equal(regression.severity, 'error');
    assert.match(regression.message, /^Visually-validated scenario regressed:/);
    assert.match(regression.suggestedObjective, /Restore visual delivery/);
  });

  void it('does not emit visual-regression for tagged-and-failed scenarios outside playwright mode', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'alpha',
    });
    const regression = r.findings.find(f => f.source === VISUAL_REGRESSION);
    assert.equal(regression, undefined);
  });

  void it('populates screenshotPath on visual-regression findings', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'browser',
    });
    const regression = r.findings.find(f => f.source === VISUAL_REGRESSION)!;
    assert.ok(regression.screenshotPath);
    assert.match(regression.screenshotPath, /^reports\/screenshots\/lamad-b\//);
    assert.match(regression.screenshotPath, /\.png$/);
    assert.match(regression.screenshotPath, /--\*\.png$/);
  });

  void it('populates screenshotPath on scenario-failure findings in playwright mode', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'browser',
    });
    const failure = r.findings.find(
      f => f.source === SCENARIO_FAILURE && f.message.includes('nope')
    );
    assert.ok(failure);
    assert.ok(failure.screenshotPath);
    assert.match(failure.screenshotPath, /^reports\/screenshots\/lamad-d\//);
    assert.match(failure.screenshotPath, /--\*\.png$/);
  });

  void it('omits screenshotPath on scenario-failure findings outside playwright mode', () => {
    const r = aggregate({
      scenarios: visualScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r1',
      profile: 'alpha',
    });
    const failure = r.findings.find(f => f.source === SCENARIO_FAILURE)!;
    assert.equal(failure.screenshotPath, undefined);
  });
});
