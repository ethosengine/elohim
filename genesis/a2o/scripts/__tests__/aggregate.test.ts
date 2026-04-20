import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { aggregate } from '../lib/aggregate.js';
import type { ScenarioResult } from '../lib/load-cucumber.js';
import type { ConsoleArtifact } from '../lib/load-console.js';
import type { GapFinding } from '../lib/load-coverage-gap.js';

function input() {
  const scenarios: ScenarioResult[] = [
    { name: 'Timothy completes path', feature: 'features/lamad/learning-journey.feature', status: 'passed' },
    { name: 'Mary fails on assessment', feature: 'features/lamad/learning-journey.feature', status: 'failed', failureMessage: 'AssertionError: expected 500 to be 200' },
    { name: 'Stub not implemented', feature: 'features/auth/fixture-humans.feature', status: 'pending' },
  ];
  const console: ConsoleArtifact[] = [
    {
      scenario: 'learning-journey', human: 'timothy',
      consoleErrors: [
        { level: 'error', text: 'ReferenceError: Sophia is not defined', url: 'https://doorway-alpha.elohim.host/a.js' },
      ],
      pageErrors: [],
    },
    {
      scenario: 'learning-journey', human: 'mary',
      consoleErrors: [
        { level: 'error', text: 'ReferenceError: Sophia is not defined', url: 'https://doorway-alpha.elohim.host/a.js' },
      ],
      pageErrors: [
        { message: "TypeError: Cannot read properties of null (reading 'token')", url: 'https://doorway-alpha.elohim.host/login' },
      ],
    },
  ];
  const gaps: GapFinding[] = [
    { feature: 'features/elohim/presence.feature', missing: 'presence claim expires', severity: 'medium' },
  ];
  return { scenarios, console, gaps };
}

describe('aggregate', () => {
  it('counts scenarios in summary', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({ scenarios, consoleArtifacts: console, gaps, runId: 'r1', profile: 'alpha', doorway: 'https://d.alpha' });
    assert.equal(r.summary.scenarios.total, 3);
    assert.equal(r.summary.scenarios.passed, 1);
    assert.equal(r.summary.scenarios.failed, 1);
    assert.equal(r.summary.scenarios.pending, 1);
  });

  it('dedupes identical console errors into one finding with occurrences=2', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({ scenarios, consoleArtifacts: console, gaps, runId: 'r1', profile: 'alpha' });
    const sophia = r.findings.find(f => f.message.includes('Sophia is not defined'))!;
    assert.ok(sophia);
    assert.equal(sophia.occurrences, 2);
    assert.equal(sophia.source, 'console-error');
    assert.equal(sophia.scenarios.length, 2);
  });

  it('includes scenario-failure findings', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({ scenarios, consoleArtifacts: console, gaps, runId: 'r1', profile: 'alpha' });
    const failure = r.findings.find(f => f.source === 'scenario-failure')!;
    assert.ok(failure);
    assert.match(failure.message, /AssertionError/);
    assert.equal(failure.pillar, 'lamad');
  });

  it('includes pending-step findings', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({ scenarios, consoleArtifacts: console, gaps, runId: 'r1', profile: 'alpha' });
    const pending = r.findings.find(f => f.source === 'pending-step');
    assert.ok(pending);
    assert.equal(pending!.pillar, 'imagodei');
  });

  it('includes coverage-gap findings', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({ scenarios, consoleArtifacts: console, gaps, runId: 'r1', profile: 'alpha' });
    const gap = r.findings.find(f => f.source === 'coverage-gap')!;
    assert.ok(gap);
    assert.equal(gap.pillar, 'elohim');
  });

  it('sorts findings by occurrences desc', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({ scenarios, consoleArtifacts: console, gaps, runId: 'r1', profile: 'alpha' });
    for (let i = 1; i < r.findings.length; i++) {
      assert.ok(r.findings[i - 1].occurrences >= r.findings[i].occurrences);
    }
  });

  it('emits suggested objective headlines', () => {
    const { scenarios, console, gaps } = input();
    const r = aggregate({ scenarios, consoleArtifacts: console, gaps, runId: 'r1', profile: 'alpha' });
    for (const f of r.findings) {
      assert.ok(f.suggestedObjective && f.suggestedObjective.length > 0);
    }
  });
});
