/**
 * Tests for the deployment-topology guard in humans.ts.
 *
 * Run: tsx --test src/framework/fixtures/__tests__/humans.test.ts
 *
 * The guard reads genesis/orchestrator/data/deployments.json and lets a2o
 * step definitions auto-skip scenarios for suspended humans without
 * touching feature files. We use ELOHIM_DEPLOYMENTS_PATH_OVERRIDE +
 * _resetDeploymentsCacheForTests() to inject fixture data.
 */

import { strict as assert } from 'node:assert';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, it } from 'node:test';

import { _resetDeploymentsCacheForTests, isHumanDeployed } from '../humans.js';

interface FixtureHuman {
  name: string;
  suspended?: boolean;
}

function writeFixture(dir: string, humans: FixtureHuman[]): string {
  const path = join(dir, 'deployments.json');
  writeFileSync(path, JSON.stringify({ humans }, null, 2));
  return path;
}

void describe('isHumanDeployed', () => {
  let workDir: string;

  beforeEach(() => {
    workDir = mkdtempSync(join(tmpdir(), 'elohim-deployments-'));
    _resetDeploymentsCacheForTests();
  });

  afterEach(() => {
    delete process.env.ELOHIM_DEPLOYMENTS_PATH_OVERRIDE;
    _resetDeploymentsCacheForTests();
    rmSync(workDir, { recursive: true, force: true });
  });

  void it('returns true for a deployed (not suspended) human', () => {
    process.env.ELOHIM_DEPLOYMENTS_PATH_OVERRIDE = writeFixture(workDir, [
      { name: 'matthew' },
      { name: 'jessica' },
    ]);
    assert.equal(isHumanDeployed('Matthew'), true);
    assert.equal(isHumanDeployed('Jessica'), true);
  });

  void it('returns false for a suspended human', () => {
    process.env.ELOHIM_DEPLOYMENTS_PATH_OVERRIDE = writeFixture(workDir, [
      { name: 'matthew' },
      { name: 'pete', suspended: true },
      { name: 'frank', suspended: true },
    ]);
    assert.equal(isHumanDeployed('Pete'), false);
    assert.equal(isHumanDeployed('Frank'), false);
    assert.equal(isHumanDeployed('Matthew'), true);
  });

  void it('returns true for a doorway-only persona not in deployments.json', () => {
    // Susan, Tommy, Georgina, etc. are seeded humans without k8s pods —
    // they should NOT be gated by the deployment topology.
    process.env.ELOHIM_DEPLOYMENTS_PATH_OVERRIDE = writeFixture(workDir, [
      { name: 'matthew' },
      { name: 'pete', suspended: true },
    ]);
    assert.equal(isHumanDeployed('Susan'), true);
    assert.equal(isHumanDeployed('Tommy'), true);
    assert.equal(isHumanDeployed('Georgina'), true);
  });

  void it('fails open when deployments.json is missing or unreadable', () => {
    // Point at a path that does not exist; loader should swallow the error
    // and return an empty cache, which means "assume all humans deployed."
    process.env.ELOHIM_DEPLOYMENTS_PATH_OVERRIDE = join(workDir, 'does-not-exist.json');
    assert.equal(isHumanDeployed('Pete'), true);
    assert.equal(isHumanDeployed('Anyone'), true);
  });

  void it('compares names case-insensitively', () => {
    // deployments.json uses lowercase names ("pete"); Cucumber steps use
    // displayName ("Pete"). Lookups must be case-insensitive.
    process.env.ELOHIM_DEPLOYMENTS_PATH_OVERRIDE = writeFixture(workDir, [
      { name: 'pete', suspended: true },
    ]);
    assert.equal(isHumanDeployed('PETE'), false);
    assert.equal(isHumanDeployed('Pete'), false);
    assert.equal(isHumanDeployed('pete'), false);
    assert.equal(isHumanDeployed('PeTe'), false);
  });
});
