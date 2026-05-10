// Drift detection for build-artifacts.json.
//
// Every documented key MUST exist with a non-empty string filename
// because consumers across Groovy + TypeScript + JavaScript all index
// into the same shape. A typo or missing key here is a silent break
// across pipelines — this test catches it before merge.

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

import { ARTIFACTS, ORCHESTRATOR, GENESIS } from './build-artifacts.mjs';

describe('build-artifacts manifest — schema integrity', () => {
  it('declares schemaVersion', () => {
    assert.equal(typeof ARTIFACTS.schemaVersion, 'string');
    assert.match(ARTIFACTS.schemaVersion, /^\d+$/, 'schemaVersion must be a digit string');
  });

  it('exposes orchestrator + genesis namespaces', () => {
    assert.equal(typeof ARTIFACTS.orchestrator, 'object');
    assert.equal(typeof ARTIFACTS.genesis, 'object');
  });

  it('orchestrator has all required keys', () => {
    const required = ['predictedBuildGraph', 'actualBuildGraph', 'buildGraphReconciliation'];
    for (const key of required) {
      assert.equal(typeof ORCHESTRATOR[key], 'string', `orchestrator.${key} must be a string`);
      assert.match(ORCHESTRATOR[key], /\.json$/, `orchestrator.${key} must end in .json`);
      assert.ok(ORCHESTRATOR[key].length > 5, `orchestrator.${key} must not be empty`);
    }
  });

  it('genesis has all required keys', () => {
    const required = ['conductorReadiness', 'seedResultsConductorIdentities', 'seedResultsAgentBindings'];
    for (const key of required) {
      assert.equal(typeof GENESIS[key], 'string', `genesis.${key} must be a string`);
      assert.match(GENESIS[key], /\.json$/, `genesis.${key} must end in .json`);
      assert.ok(GENESIS[key].length > 5, `genesis.${key} must not be empty`);
    }
  });

  it('all filenames are unique across both namespaces', () => {
    const all = [
      ...Object.values(ORCHESTRATOR),
      ...Object.values(GENESIS),
    ];
    const seen = new Set();
    for (const name of all) {
      assert.ok(!seen.has(name), `duplicate artifact filename: ${name}`);
      seen.add(name);
    }
  });

  it('filenames use kebab-case (no spaces, slashes, or uppercase)', () => {
    const all = [
      ...Object.values(ORCHESTRATOR),
      ...Object.values(GENESIS),
    ];
    for (const name of all) {
      assert.match(name, /^[a-z0-9-]+\.json$/, `${name} must be lowercase kebab-case .json`);
    }
  });
});
