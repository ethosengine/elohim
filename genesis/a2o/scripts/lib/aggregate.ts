import { fingerprint, normalizeMessage } from './fingerprint.js';
import { pillarFromFeature } from './pillar-from-feature.js';
import type { ScenarioResult } from './load-cucumber.js';
import type { ConsoleArtifact } from './load-console.js';
import type { GapFinding } from './load-coverage-gap.js';

export type FindingSource =
  | 'console-error'
  | 'page-error'
  | 'failed-request'
  | 'scenario-failure'
  | 'pending-step'
  | 'coverage-gap';

export interface FindingScenario { name: string; feature: string; human?: string }

export interface Finding {
  fingerprint: string;
  source: FindingSource;
  pillar: string;
  severity: 'error' | 'warning' | 'info';
  message: string;
  firstSeenUrl?: string;
  occurrences: number;
  scenarios: FindingScenario[];
  suggestedObjective: string;
}

export interface SprintReport {
  generatedAt: string;
  runId: string;
  profile: string;
  doorway?: string;
  summary: {
    scenarios: { total: number; passed: number; failed: number; skipped: number; pending: number };
    findings:  { total: number; bySource: Record<string, number>; byPillar: Record<string, number> };
  };
  findings: Finding[];
}

interface AggregateInput {
  scenarios: ScenarioResult[];
  consoleArtifacts: ConsoleArtifact[];
  gaps: GapFinding[];
  runId: string;
  profile: string;
  doorway?: string;
}

interface RawFinding {
  source: FindingSource;
  pillar: string;
  severity: 'error' | 'warning' | 'info';
  rawMessage: string;
  url?: string;
  scenario: FindingScenario;
}

function suggestObjective(source: FindingSource, message: string): string {
  const head = message.slice(0, 120);
  switch (source) {
    case 'console-error':     return `Fix browser console error: ${head}`;
    case 'page-error':        return `Fix unhandled page exception: ${head}`;
    case 'failed-request':    return `Fix failing network request: ${head}`;
    case 'scenario-failure':  return `Fix scenario failure: ${head}`;
    case 'pending-step':      return `Implement pending step definition: ${head}`;
    case 'coverage-gap':      return `Author missing scenario: ${head}`;
  }
}

export function aggregate(input: AggregateInput): SprintReport {
  const raws: RawFinding[] = [];

  // Scenario-level
  for (const s of input.scenarios) {
    if (s.status === 'failed' && s.failureMessage) {
      raws.push({
        source: 'scenario-failure',
        pillar: pillarFromFeature(s.feature),
        severity: 'error',
        rawMessage: s.failureMessage.split('\n')[0],
        scenario: { name: s.name, feature: s.feature },
      });
    } else if (s.status === 'pending') {
      raws.push({
        source: 'pending-step',
        pillar: pillarFromFeature(s.feature),
        severity: 'warning',
        rawMessage: `Pending scenario: ${s.name}`,
        scenario: { name: s.name, feature: s.feature },
      });
    }
  }

  // Console / page / network
  for (const art of input.consoleArtifacts) {
    for (const e of art.consoleErrors) {
      raws.push({
        source: 'console-error',
        pillar: 'browser',
        severity: e.level === 'warning' ? 'warning' : 'error',
        rawMessage: e.text,
        url: e.url,
        scenario: { name: art.scenario, feature: 'browser', human: art.human },
      });
    }
    for (const p of art.pageErrors) {
      raws.push({
        source: 'page-error',
        pillar: 'browser',
        severity: 'error',
        rawMessage: p.message,
        url: p.url,
        scenario: { name: art.scenario, feature: 'browser', human: art.human },
      });
    }
  }

  // Coverage gaps
  for (const g of input.gaps) {
    raws.push({
      source: 'coverage-gap',
      pillar: pillarFromFeature(g.feature),
      severity: g.severity === 'high' ? 'error' : 'warning',
      rawMessage: g.missing,
      scenario: { name: g.missing, feature: g.feature },
    });
  }

  // Group by (source + fingerprint) — keep sources separate so the same normalized
  // text from two different sources doesn't collapse unexpectedly.
  const groups = new Map<string, Finding>();
  for (const r of raws) {
    const fp = fingerprint(r.rawMessage);
    const key = `${r.source}::${fp}`;
    const scenarioKey = (s: FindingScenario) => `${s.feature}::${s.name}::${s.human ?? ''}`;
    const existing = groups.get(key);
    if (existing) {
      existing.occurrences += 1;
      if (!existing.scenarios.some(s => scenarioKey(s) === scenarioKey(r.scenario))) {
        existing.scenarios.push(r.scenario);
      }
    } else {
      const message = normalizeMessage(r.rawMessage);
      groups.set(key, {
        fingerprint: fp,
        source: r.source,
        pillar: r.pillar,
        severity: r.severity,
        message,
        firstSeenUrl: r.url,
        occurrences: 1,
        scenarios: [r.scenario],
        suggestedObjective: suggestObjective(r.source, message),
      });
    }
  }

  const findings = [...groups.values()].sort((a, b) => {
    if (b.occurrences !== a.occurrences) return b.occurrences - a.occurrences;
    return a.fingerprint.localeCompare(b.fingerprint);
  });

  const scenarioCounts = { total: 0, passed: 0, failed: 0, skipped: 0, pending: 0 };
  for (const s of input.scenarios) {
    scenarioCounts.total += 1;
    const status = (s.status === 'undefined' ? 'pending' : s.status) as keyof typeof scenarioCounts;
    if (status !== 'total') scenarioCounts[status] += 1;
  }

  const bySource: Record<string, number> = {};
  const byPillar: Record<string, number> = {};
  for (const f of findings) {
    bySource[f.source] = (bySource[f.source] ?? 0) + 1;
    byPillar[f.pillar] = (byPillar[f.pillar] ?? 0) + 1;
  }

  return {
    generatedAt: new Date().toISOString(),
    runId: input.runId,
    profile: input.profile,
    doorway: input.doorway,
    summary: {
      scenarios: scenarioCounts,
      findings: { total: findings.length, bySource, byPillar },
    },
    findings,
  };
}
