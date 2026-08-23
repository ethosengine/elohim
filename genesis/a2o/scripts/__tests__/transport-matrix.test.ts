import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { renderTransportMatrix } from '../lib/transport-matrix.js';

import type { SprintReport } from '../lib/aggregate.js';

function report(transport: string, durationMs: number, failed = 0): SprintReport {
  return {
    generatedAt: '2026-08-23T00:00:00.000Z',
    runId: `${transport}-${durationMs}`,
    profile: 'mesh',
    transport,
    env: {
      lane: 'household',
      peers: 3,
      processControl: true,
      networkStage: 'simulacra',
      sut: 'sha256:0123456789abcdef',
      sutParts: {},
      unknown: [],
    },
    summary: {
      scenarios: {
        total: 2,
        passed: 2 - failed,
        failed,
        skipped: 0,
        pending: 0,
        durationMs,
      },
      findings: { total: 0, bySource: {}, byPillar: {} },
      byConcern: {},
    },
    findings: [],
  };
}

void describe('renderTransportMatrix', () => {
  void it('renders backend percentiles, witnesses, and honest mixed-mode absence', () => {
    const markdown = renderTransportMatrix(
      [
        { path: '/repo/reports/lib-1.json', report: report('libp2p', 100) },
        { path: '/repo/reports/lib-2.json', report: report('libp2p', 200) },
        { path: '/repo/reports/lib-3.json', report: report('libp2p', 300, 1) },
        { path: '/repo/reports/iroh.json', report: report('iroh', 50) },
      ],
      '/repo/reports/matrix.md',
      '2026-08-23T01:00:00.000Z'
    );
    assert.match(markdown, /\| libp2p \| 3 \| 3 \| 2 \/ 1 \/ 0 \| 100\.0 ms \| 200\.0 ms/);
    assert.match(markdown, /\| iroh \| 1 \| 3 \| 1 \/ 0 \/ 0 \| 50\.0 ms/);
    assert.match(markdown, /mixed per-peer \| 0 \| not supported/);
    assert.match(markdown, /economic events: none/);
    assert.match(markdown, /successful runs only/);
    assert.match(
      markdown,
      /passing dual row proves convergence while both planes perform sync work/
    );
    assert.match(markdown, /Failed rows prove only\s+the named boundary failure/);
    assert.match(markdown, /Peer count \| 3 \| measured only at the listed counts/);
    assert.match(markdown, /LAN\/NAT\/relay comparison/);
    assert.match(markdown, /\[libp2p-100\]\(lib-1\.json\)/);
  });
});
