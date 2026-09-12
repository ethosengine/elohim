/**
 * Shared honest-parity contract for `/api/v1/diagnostics/inventory-parity`.
 *
 * "Parity" across a household mesh is each peer's gossip view agreeing with
 * its own filesystem, and a churned/paused-then-returned peer holding no
 * fewer blobs than it held before the perturbation — NOT every peer holding
 * the same blob count. Household peers legitimately hold DIFFERENT blob sets
 * (the seeding peer carries the corpus, the others hold what custody
 * commitments placed on them), so an equal-counts assertion is red before
 * any drill even starts. See:
 *   - genesis/a2o/features/federation/peer-loss-failover.feature
 *     ("A returning peer re-syncs without operator help")
 *   - genesis/a2o/features/resilience/chaos-peer-churn.feature
 *     ("A flapping peer never corrupts what the mesh believes")
 *
 * Originally implemented once, inline, in
 * genesis/a2o/steps/federation-failover.steps.ts and duplicated (with a
 * WRONG equal-counts assertion) in genesis/a2o/steps/mesh/household-chaos.steps.ts
 * — extracted here so both step files share one definition of "healthy".
 */

import { strict as assert } from 'node:assert';

export interface InventoryParityReport {
  gossipedButMissing: string[];
  localButNotGossiped: string[];
  filesystemCount: number;
  gossipedCount: number;
}

/** Loose shape of the wire body — the endpoint has shipped both snake_case and camelCase eras. */
export type InventoryParityWire = Record<string, unknown>;

/**
 * Normalize the wire body into a typed report. `peerLabel`, when given,
 * prefixes the "omitted a count" failure messages — purely cosmetic, never
 * changes pass/fail.
 */
export function normalizeInventoryParity(
  wire: InventoryParityWire,
  peerLabel?: string
): InventoryParityReport {
  const list = (snake: string, camel: string): string[] => {
    const value = wire[snake] ?? wire[camel];
    return Array.isArray(value) ? (value as string[]) : [];
  };
  const count = (snake: string, camel: string): number => {
    const value = wire[snake] ?? wire[camel];
    return typeof value === 'number' ? value : -1;
  };
  const report: InventoryParityReport = {
    gossipedButMissing: list('gossiped_but_missing', 'gossipedButMissing'),
    localButNotGossiped: list('local_but_not_gossiped', 'localButNotGossiped'),
    filesystemCount: count('filesystem_count', 'filesystemCount'),
    gossipedCount: count('gossiped_count', 'gossipedCount'),
  };
  const prefix = peerLabel ? `${peerLabel} ` : '';
  assert.ok(report.filesystemCount >= 0, `${prefix}inventory parity omitted filesystem count`);
  assert.ok(report.gossipedCount >= 0, `${prefix}inventory parity omitted gossiped count`);
  return report;
}

/**
 * Assert one peer's inventory-parity report is honestly healthy against a
 * pre-perturbation baseline: no phantom gossip, no un-gossiped local blobs,
 * filesystem/gossip counts agree, and the peer lost nothing it held before.
 *
 * `baselineContext` only changes the wording of the last message (e.g.
 * "before the outage" vs "before the churn") — never the comparison itself.
 */
export function assertPeerInventoryParityHealthy(
  peerLabel: string,
  report: InventoryParityReport,
  baselineFilesystemCount: number,
  baselineContext = 'before the outage'
): void {
  assert.deepEqual(
    report.gossipedButMissing,
    [],
    `${peerLabel} gossips blobs it does not hold: ${report.gossipedButMissing.join(', ')}`
  );
  assert.deepEqual(
    report.localButNotGossiped,
    [],
    `${peerLabel} holds blobs it has not gossiped: ${report.localButNotGossiped.join(', ')}`
  );
  assert.equal(
    report.filesystemCount,
    report.gossipedCount,
    `${peerLabel} filesystem/gossip counts differ`
  );
  assert.ok(
    report.filesystemCount >= baselineFilesystemCount,
    `${peerLabel} holds ${report.filesystemCount} blobs, fewer than the ${baselineFilesystemCount} ` +
      `it held ${baselineContext}`
  );
}
