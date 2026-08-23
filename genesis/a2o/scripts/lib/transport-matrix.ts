import { dirname, relative } from 'node:path';

import type { SprintReport } from './aggregate.js';

const NOT_MEASURED = 'not measured';

export interface TransportMatrixInput {
  path: string;
  report: SprintReport;
}

interface TransportCell {
  transport: string;
  reports: TransportMatrixInput[];
}

function percentile(values: number[], pct: number): number | null {
  if (values.length === 0) return null;
  const ordered = [...values].sort((a, b) => a - b);
  const index = Math.max(0, Math.ceil((pct / 100) * ordered.length) - 1);
  return ordered[index];
}

function formatMs(value: number | null): string {
  if (value === null) return NOT_MEASURED;
  if (value >= 1_000) return `${(value / 1_000).toFixed(2)} s`;
  return `${value.toFixed(1)} ms`;
}

function verdict(report: SprintReport): 'passed' | 'failed' | typeof NOT_MEASURED {
  const scenarios = report.summary.scenarios;
  if (scenarios.failed > 0) return 'failed';
  if (scenarios.pending > 0 || scenarios.skipped > 0 || scenarios.total === 0) {
    return NOT_MEASURED;
  }
  return 'passed';
}

function outcomeDetail(report: SprintReport): string {
  const state = verdict(report);
  if (state === 'passed') return 'contract satisfied';
  if (state === NOT_MEASURED) return NOT_MEASURED;
  const finding = report.findings.find(item => item.source === 'scenario-failure');
  const message = finding?.message ?? 'scenario failed; see witness';
  const oneLine = message
    .replaceAll('|', String.raw`\|`)
    .replace(/\s+/g, ' ')
    .trim();
  return oneLine.length > 180 ? `${oneLine.slice(0, 177)}...` : oneLine;
}

function cellsOf(inputs: TransportMatrixInput[]): TransportCell[] {
  const groups = new Map<string, TransportMatrixInput[]>();
  for (const input of inputs) {
    const transport = input.report.transport ?? 'unknown';
    const reports = groups.get(transport) ?? [];
    reports.push(input);
    groups.set(transport, reports);
  }
  return ['libp2p', 'iroh', 'dual', 'unknown']
    .filter(transport => groups.has(transport))
    .map(transport => ({ transport, reports: groups.get(transport)! }));
}

/**
 * Render an Ephemeral (Category C) query over sprint-report witnesses.
 * Delete it and rerun the matrix: the source reports reconstruct every cell.
 */
export function renderTransportMatrix(
  inputs: TransportMatrixInput[],
  outputPath: string,
  generatedAt: string
): string {
  const cells = cellsOf(inputs);
  const peerCounts = [...new Set(inputs.map(input => input.report.env?.peers ?? 0))]
    .sort((left, right) => left - right)
    .join(', ');
  const stages = [
    ...new Set(inputs.map(input => input.report.env?.networkStage ?? 'unknown')),
  ].join(', ');
  const lines = [
    '# Household transport comparison matrix',
    '',
    `Generated: ${generatedAt}`,
    '',
    '## ValueFlows process view',
    '',
    '- ProcessSpecification: `transport-household-contract@1`',
    '- inScopeOf: `habit:dataplane-convergence`',
    '- inputs: the transport-parity scenario, live household, and each linked sprint-report witness',
    '- outputs: this reconstructible comparison view; the sprint reports remain the evidence bodies',
    '- economic events: none — transport timing is operational evidence, not recognition, reputation, or payout attribution',
    '- evidence strength: `claimed` by one household host until an independent household recomputes the same derivation',
    '',
    'Cucumber duration is end-to-end scenario time accounted by step results. Matrix percentiles',
    'include successful runs only, so a fast failure cannot masquerade as transport performance.',
    'The timing is useful for same-host backend comparison, but it is not a transport-only',
    'microbenchmark and must not be read as LAN, relay, NAT, or constrained-device performance.',
    '',
    'A passing libp2p or iroh row is a causal transport-capability proof because the run enables',
    'only that plane. A passing dual row proves convergence while both planes perform sync work;',
    'it does not attribute the document delivery to either racing plane. Failed rows prove only',
    'the named boundary failure recorded in their witness.',
    '',
    '## Matrix',
    '',
    '| Transport | Runs | Peers | Passed / failed / not measured | Scenario p50 | p95 | p99 | SUTs |',
    '|---|---:|---:|---|---:|---:|---:|---:|',
  ];

  for (const cell of cells) {
    const durations = cell.reports
      .filter(input => verdict(input.report) === 'passed')
      .map(input => input.report.summary.scenarios.durationMs)
      .filter((value): value is number => typeof value === 'number');
    const verdicts = cell.reports.map(input => verdict(input.report));
    const count = (target: string) => verdicts.filter(value => value === target).length;
    const peers = [...new Set(cell.reports.map(input => input.report.env?.peers ?? 0))].join(', ');
    const suts = new Set(cell.reports.map(input => input.report.env?.sut).filter(Boolean)).size;
    lines.push(
      `| ${cell.transport} | ${cell.reports.length} | ${peers} | ${count('passed')} / ${count('failed')} / ${count(NOT_MEASURED)} | ${formatMs(percentile(durations, 50))} | ${formatMs(percentile(durations, 95))} | ${formatMs(percentile(durations, 99))} | ${suts} |`
    );
  }
  lines.push(
    '| mixed per-peer | 0 | not supported | 0 / 0 / not measured | not measured | not measured | not measured | 0 |',
    '',
    'Mixed per-peer mode is explicit absence: the current mesh selects one backend for the whole',
    'household. This row becomes measurable only after per-peer launch configuration exists.',
    '',
    '## Coverage boundaries',
    '',
    '| Dimension | Observed in these witnesses | Coverage |',
    '|---|---|---|',
    `| Peer count | ${peerCounts || NOT_MEASURED} | measured only at the listed counts |`,
    `| Network stage | ${stages || 'unknown'} | same-host mesh; not a LAN/NAT/relay comparison |`,
    '| Per-peer backend mix | launcher selects one household-wide mode | not supported / not measured |',
    '| Device diversity | one development host | constrained and heterogeneous devices not measured |',
    '',
    '## Run witnesses',
    '',
    '| Transport | Run | Verdict | Outcome | Peers | Duration | SUT | Evidence |',
    '|---|---|---|---|---:|---:|---|---|'
  );

  for (const input of inputs) {
    const report = input.report;
    const rel = relative(dirname(outputPath), input.path).replaceAll('\\', '/');
    lines.push(
      `| ${report.transport ?? 'unknown'} | [${report.runId}](${rel}) | ${verdict(report)} | ${outcomeDetail(report)} | ${report.env?.peers ?? 0} | ${formatMs(report.summary.scenarios.durationMs ?? null)} | ${report.env?.sut ?? 'unknown'} | claimed |`
    );
  }
  lines.push('');
  return lines.join('\n');
}
