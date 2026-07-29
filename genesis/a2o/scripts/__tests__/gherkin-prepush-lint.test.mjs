import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { parseFeature } from '../gherkin-prepush-lint.mjs';

describe('gherkin-prepush-lint', () => {
  it('accepts route paths containing slashes', () => {
    assert.doesNotThrow(() =>
      parseFeature(
        [
          'Feature: diagnostics',
          '  Scenario: transport probe',
          '    Given the operator requests "/db/p2p/conductor-diagnostics"',
        ].join('\n')
      )
    );
  });

  it('rejects a bare continuation after a step', () => {
    assert.throws(
      () =>
        parseFeature(
          [
            'Feature: diagnostics',
            '  Scenario: transport probe',
            '    Given the operator requests diagnostics',
            '      then reads its transport state',
          ].join('\n')
        ),
      /expected:/
    );
  });
});
