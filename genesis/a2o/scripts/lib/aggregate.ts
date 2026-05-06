import { fingerprint, normalizeMessage } from './fingerprint.js';
import { pillarFromFeature } from './pillar-from-feature.js';

import type { ConsoleArtifact } from './load-console.js';
import type { GapFinding } from './load-coverage-gap.js';
import type { ScenarioResult } from './load-cucumber.js';

const VISUAL_VALIDATION_TAG = '@elohim-visually-validated';

const PLAYWRIGHT_PROFILES = new Set(['browser', 'delivery-browser']);

function isPlaywrightProfile(profile: string): boolean {
  return PLAYWRIGHT_PROFILES.has(profile);
}

function screenshotPathFor(scenario: ScenarioResult, human?: string): string {
  const featureSlug = scenario.feature
    .replace(/^.*features\//, '')
    .replace(/\.feature$/, '')
    .replace(/\//g, '-');
  const scenarioSlug = scenario.name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
  const suffix = human ? `--${human}` : '--*';
  return `reports/screenshots/${featureSlug}/${scenarioSlug}${suffix}.png`;
}

export type FindingSource =
  | 'console-error'
  | 'page-error'
  | 'failed-request'
  | 'scenario-failure'
  | 'pending-step'
  | 'coverage-gap'
  | 'visual-regression';

export interface FindingScenario {
  name: string;
  feature: string;
  human?: string;
}

export interface Finding {
  fingerprint: string;
  source: FindingSource;
  pillar: string;
  severity: 'error' | 'warning' | 'info';
  message: string;
  firstSeenUrl?: string;
  screenshotPath?: string;
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
    findings: { total: number; bySource: Record<string, number>; byPillar: Record<string, number> };
    visualValidation?: {
      validatedPassing: number;
      validatedRegressed: number;
      pendingPassing: number;
      pendingFailing: number;
    };
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
  screenshotPath?: string;
  scenario: FindingScenario;
}

function suggestObjective(source: FindingSource, message: string): string {
  const head = message.slice(0, 120);
  switch (source) {
    case 'console-error':
      return `Fix browser console error: ${head}`;
    case 'page-error':
      return `Fix unhandled page exception: ${head}`;
    case 'failed-request':
      return `Fix failing network request: ${head}`;
    case 'scenario-failure':
      return `Fix scenario failure: ${head}`;
    case 'pending-step':
      return `Implement pending step definition: ${head}`;
    case 'coverage-gap':
      return `Author missing scenario: ${head}`;
    case 'visual-regression':
      return `Restore visual delivery of: ${head}`;
  }
}

function buildScenarioRaws(scenarios: ScenarioResult[], emitVisual: boolean): RawFinding[] {
  const raws: RawFinding[] = [];
  for (const s of scenarios) {
    const validated = s.tags.includes(VISUAL_VALIDATION_TAG);
    if (s.status === 'failed' && s.failureMessage) {
      raws.push({
        source: 'scenario-failure',
        pillar: pillarFromFeature(s.feature),
        severity: 'error',
        rawMessage: s.failureMessage.split('\n')[0],
        screenshotPath: emitVisual ? screenshotPathFor(s) : undefined,
        scenario: { name: s.name, feature: s.feature },
      });
      if (emitVisual && validated) {
        raws.push({
          source: 'visual-regression',
          pillar: pillarFromFeature(s.feature),
          severity: 'error',
          rawMessage: `Visually-validated scenario regressed: ${s.name}`,
          screenshotPath: screenshotPathFor(s),
          scenario: { name: s.name, feature: s.feature },
        });
      }
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
  return raws;
}

function buildConsoleRaws(artifacts: ConsoleArtifact[]): RawFinding[] {
  const raws: RawFinding[] = [];
  for (const art of artifacts) {
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
  return raws;
}

function buildGapRaws(gaps: GapFinding[]): RawFinding[] {
  return gaps.map(g => ({
    source: 'coverage-gap' as FindingSource,
    pillar: pillarFromFeature(g.feature),
    severity: g.severity === 'high' ? ('error' as const) : ('warning' as const),
    rawMessage: g.missing,
    scenario: { name: g.missing, feature: g.feature },
  }));
}

function groupIntoFindings(raws: RawFinding[]): Finding[] {
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
      // First non-empty screenshotPath wins; downstream renderers link to one image per finding.
      if (!existing.screenshotPath && r.screenshotPath) {
        existing.screenshotPath = r.screenshotPath;
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
        screenshotPath: r.screenshotPath,
        occurrences: 1,
        scenarios: [r.scenario],
        suggestedObjective: suggestObjective(r.source, message),
      });
    }
  }
  return [...groups.values()].sort((a, b) => {
    if (b.occurrences !== a.occurrences) return b.occurrences - a.occurrences;
    return a.fingerprint.localeCompare(b.fingerprint);
  });
}

function computeSummary(scenarios: ScenarioResult[], findings: Finding[]): SprintReport['summary'] {
  const scenarioCounts = { total: 0, passed: 0, failed: 0, skipped: 0, pending: 0 };
  for (const s of scenarios) {
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
    scenarios: scenarioCounts,
    findings: { total: findings.length, bySource, byPillar },
  };
}

function computeVisualValidation(
  scenarios: ScenarioResult[]
): NonNullable<SprintReport['summary']['visualValidation']> {
  const counts = {
    validatedPassing: 0,
    validatedRegressed: 0,
    pendingPassing: 0,
    pendingFailing: 0,
  };
  for (const s of scenarios) {
    const validated = s.tags.includes(VISUAL_VALIDATION_TAG);
    if (validated && s.status === 'passed') counts.validatedPassing += 1;
    else if (validated && s.status === 'failed') counts.validatedRegressed += 1;
    else if (!validated && s.status === 'passed') counts.pendingPassing += 1;
    else if (!validated && s.status === 'failed') counts.pendingFailing += 1;
  }
  return counts;
}

export function aggregate(input: AggregateInput): SprintReport {
  const emitVisual = isPlaywrightProfile(input.profile);
  const raws = [
    ...buildScenarioRaws(input.scenarios, emitVisual),
    ...buildConsoleRaws(input.consoleArtifacts),
    ...buildGapRaws(input.gaps),
  ];

  const findings = groupIntoFindings(raws);
  const summary = computeSummary(input.scenarios, findings);

  if (emitVisual) {
    summary.visualValidation = computeVisualValidation(input.scenarios);
  }

  return {
    generatedAt: new Date().toISOString(),
    runId: input.runId,
    profile: input.profile,
    doorway: input.doorway,
    summary,
    findings,
  };
}
