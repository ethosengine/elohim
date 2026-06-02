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

import {
  _resetDeploymentsCacheForTests,
  autoSkippedHumans,
  isAnyNodePoolAvailable,
  isHumanDeployed,
  isRemoteComputeAvailable,
} from '../humans.js';

interface FixtureHuman {
  name: string;
  suspended?: boolean;
  nodeTypes?: ('remote' | 'edge' | 'performance' | 'operations')[];
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
    delete process.env.ELOHIM_REMOTE_COMPUTE_STATUS;
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

void describe('remote-compute availability gating', () => {
  let workDir: string;

  beforeEach(() => {
    workDir = mkdtempSync(join(tmpdir(), 'elohim-deployments-shem-'));
    _resetDeploymentsCacheForTests();
  });

  afterEach(() => {
    delete process.env.ELOHIM_DEPLOYMENTS_PATH_OVERRIDE;
    delete process.env.ELOHIM_REMOTE_COMPUTE_STATUS;
    _resetDeploymentsCacheForTests();
    rmSync(workDir, { recursive: true, force: true });
  });

  void it('isRemoteComputeAvailable() defaults to true when env unset (fail-open)', () => {
    assert.equal(isRemoteComputeAvailable(), true);
  });

  void it('isRemoteComputeAvailable() returns false when ELOHIM_REMOTE_COMPUTE_STATUS=unavailable', () => {
    process.env.ELOHIM_REMOTE_COMPUTE_STATUS = 'unavailable';
    assert.equal(isRemoteComputeAvailable(), false);
  });

  void it('isRemoteComputeAvailable() treats unknown values as available (fail-open)', () => {
    process.env.ELOHIM_REMOTE_COMPUTE_STATUS = 'garbage';
    assert.equal(isRemoteComputeAvailable(), true);
  });

  void it('isAnyNodePoolAvailable: dev-cluster pools always available', () => {
    process.env.ELOHIM_REMOTE_COMPUTE_STATUS = 'unavailable';
    assert.equal(isAnyNodePoolAvailable(['performance']), true);
    assert.equal(isAnyNodePoolAvailable(['edge']), true);
    assert.equal(isAnyNodePoolAvailable(['operations']), true);
  });

  void it('isAnyNodePoolAvailable: remote-only fails when shem down', () => {
    process.env.ELOHIM_REMOTE_COMPUTE_STATUS = 'unavailable';
    assert.equal(isAnyNodePoolAvailable(['remote']), false);
  });

  void it('isAnyNodePoolAvailable: remote-with-fallback passes when shem down', () => {
    // ["remote", "performance"] still has the performance fallback;
    // pool-level gating sees performance and returns true.
    process.env.ELOHIM_REMOTE_COMPUTE_STATUS = 'unavailable';
    assert.equal(isAnyNodePoolAvailable(['remote', 'performance']), true);
  });

  void it('auto-skips a remote-only human when shem unavailable', () => {
    process.env.ELOHIM_REMOTE_COMPUTE_STATUS = 'unavailable';
    process.env.ELOHIM_DEPLOYMENTS_PATH_OVERRIDE = writeFixture(workDir, [
      { name: 'matthew', nodeTypes: ['performance'] },
      { name: 'shemonly', nodeTypes: ['remote'] },
    ]);
    assert.equal(isHumanDeployed('matthew'), true);
    assert.equal(isHumanDeployed('shemonly'), false);
    assert.deepEqual(autoSkippedHumans(), ['shemonly']);
  });

  void it('keeps remote-with-fallback humans deployed when shem unavailable', () => {
    // Most of our shem-resident humans declare ["remote", "performance"],
    // meaning "prefer shem but fall back to performance node". When shem
    // is down, the scheduler lands them on performance — they are still
    // deployable, just on the dev cluster. Pool-level gating respects that.
    process.env.ELOHIM_REMOTE_COMPUTE_STATUS = 'unavailable';
    process.env.ELOHIM_DEPLOYMENTS_PATH_OVERRIDE = writeFixture(workDir, [
      { name: 'gertrude', nodeTypes: ['remote', 'performance'] },
      { name: 'daniel', nodeTypes: ['remote', 'performance'] },
    ]);
    assert.equal(isHumanDeployed('gertrude'), true);
    assert.equal(isHumanDeployed('daniel'), true);
    assert.deepEqual(autoSkippedHumans(), []);
  });

  void it('still respects hard "suspended" flag even when shem available', () => {
    process.env.ELOHIM_REMOTE_COMPUTE_STATUS = 'available';
    process.env.ELOHIM_DEPLOYMENTS_PATH_OVERRIDE = writeFixture(workDir, [
      { name: 'pete', suspended: true, nodeTypes: ['remote', 'performance'] },
    ]);
    assert.equal(isHumanDeployed('pete'), false);
    // Hard-suspended is NOT counted as auto-skipped — that bucket is only
    // for the soft per-run scale-down.
    assert.deepEqual(autoSkippedHumans(), []);
  });
});
