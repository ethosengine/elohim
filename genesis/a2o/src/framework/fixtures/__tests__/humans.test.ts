/**
 * Tests for the deployment-topology guard in humans.ts.
 *
 * Run: tsx --test src/framework/fixtures/__tests__/humans.test.ts
 *
 * The guard reads genesis/orchestrator/data/deployments.json and lets a2o
 * step definitions auto-skip scenarios for suspended humans without
 * touching feature files. We use ELOHIM_DEPLOYMENTS_PATH_OVERRIDE +
 * _resetDeploymentsCacheForTests() to inject fixture data.
 *
 * The remote-compute suite also covers the cluster-state derivation: when
 * ELOHIM_REMOTE_COMPUTE_STATUS is unset, the signal is DERIVED from
 * genesis/manifests/cluster-state.yaml (injected via
 * ELOHIM_CLUSTER_STATE_PATH_OVERRIDE) so the runtime can't disagree with the
 * durable home. See scope-reconcile.py's derive_remote_compute_status().
 */

import { strict as assert } from 'node:assert';
import { mkdtempSync, writeFileSync, rmSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, dirname } from 'node:path';
import { afterEach, beforeEach, describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';

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

/** Write a minimal cluster-state.yaml fixture exercising the line-based parser
 * (role + multi-line note around the `available:` line, like the real file). */
function writeClusterState(dir: string, shemAvailable: boolean | 'degraded'): string {
  const path = join(dir, 'cluster-state.yaml');
  writeFileSync(
    path,
    [
      'schema_version: 1',
      'resources:',
      '  household-nodes:',
      '    available: true',
      '  shem:',
      '    role: multi-tenant live P2P canvas',
      `    available: ${shemAvailable}`,
      '    note: fixture for the derivation test',
      '          continuation line, deeper indent',
      '',
    ].join('\n')
  );
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
    delete process.env.ELOHIM_CLUSTER_STATE_PATH_OVERRIDE;
    _resetDeploymentsCacheForTests();
    rmSync(workDir, { recursive: true, force: true });
  });

  void it('derives shem-available from cluster-state when env unset', () => {
    process.env.ELOHIM_CLUSTER_STATE_PATH_OVERRIDE = writeClusterState(workDir, true);
    assert.equal(isRemoteComputeAvailable(), true);
  });

  void it('derives shem-unavailable from cluster-state when env unset', () => {
    process.env.ELOHIM_CLUSTER_STATE_PATH_OVERRIDE = writeClusterState(workDir, false);
    assert.equal(isRemoteComputeAvailable(), false);
  });

  void it('treats a degraded shem as unavailable (conservative, mirrors scope-reconcile)', () => {
    process.env.ELOHIM_CLUSTER_STATE_PATH_OVERRIDE = writeClusterState(workDir, 'degraded');
    assert.equal(isRemoteComputeAvailable(), false);
  });

  void it('explicit ELOHIM_REMOTE_COMPUTE_STATUS=available overrides cluster-state (CI override)', () => {
    // CI's probe wins even when the durable home declares shem down.
    process.env.ELOHIM_CLUSTER_STATE_PATH_OVERRIDE = writeClusterState(workDir, false);
    process.env.ELOHIM_REMOTE_COMPUTE_STATUS = 'available';
    assert.equal(isRemoteComputeAvailable(), true);
  });

  void it('isRemoteComputeAvailable() returns false when ELOHIM_REMOTE_COMPUTE_STATUS=unavailable', () => {
    process.env.ELOHIM_REMOTE_COMPUTE_STATUS = 'unavailable';
    assert.equal(isRemoteComputeAvailable(), false);
  });

  void it('unrecognized env value falls through to cluster-state derivation', () => {
    // A typo'd env var no longer silently claims shem-up; it consults the
    // durable home exactly like an unset value does.
    process.env.ELOHIM_REMOTE_COMPUTE_STATUS = 'garbage';
    process.env.ELOHIM_CLUSTER_STATE_PATH_OVERRIDE = writeClusterState(workDir, false);
    assert.equal(isRemoteComputeAvailable(), false);
  });

  void it("explicit 'unknown' derives from cluster-state (means: probe could not determine)", () => {
    process.env.ELOHIM_REMOTE_COMPUTE_STATUS = 'unknown';
    process.env.ELOHIM_CLUSTER_STATE_PATH_OVERRIDE = writeClusterState(workDir, true);
    assert.equal(isRemoteComputeAvailable(), true);
  });

  void it('fails open (available) when env unset AND cluster-state is unreadable', () => {
    // No durable home to consult (e.g. a published consumer without the genesis
    // tree) → preserve the legacy fail-open so tests still execute.
    process.env.ELOHIM_CLUSTER_STATE_PATH_OVERRIDE = join(workDir, 'no-such-cluster-state.yaml');
    assert.equal(isRemoteComputeAvailable(), true);
  });

  void it('default path (no override) resolves to the real cluster-state.yaml', () => {
    // Independently compute the real path (5 levels up from this __tests__ dir)
    // and confirm humans.ts's own default resolution (4 levels up from fixtures/)
    // lands on the same file — catches a wrong relative depth. Robust to shem
    // being flipped because both sides read the same file.
    const realPath = resolve(
      dirname(fileURLToPath(import.meta.url)),
      '../../../../../manifests/cluster-state.yaml'
    );
    process.env.ELOHIM_CLUSTER_STATE_PATH_OVERRIDE = realPath;
    const viaOverride = isRemoteComputeAvailable();
    // Sanity: the real file must be readable for this assertion to mean anything.
    assert.ok(readFileSync(realPath, 'utf-8').includes('shem:'));
    delete process.env.ELOHIM_CLUSTER_STATE_PATH_OVERRIDE;
    const viaDefault = isRemoteComputeAvailable();
    assert.equal(viaDefault, viaOverride);
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
