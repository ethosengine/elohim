import { fingerprint, normalizeMessage } from './fingerprint.js';
import { pillarFromFeature } from './pillar-from-feature.js';

import type { DeclaredConcernSet } from './declared-concerns.js';
import type { ConsoleArtifact } from './load-console.js';
import type { GapFinding } from './load-coverage-gap.js';
import type { ScenarioResult } from './load-cucumber.js';
import type { RunEnv } from './run-env.js';

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

export interface ConcernScenario {
  name: string;
  /**
   * `skipped` is carried as ITSELF, not collapsed into `pending` (guidestar
   * 2026-08-22 §S3 supersedes that collapse by name). Ran-and-skipped is a
   * different fact from step-not-implemented, and NOT MEASURED can only be
   * rendered against the right one.
   */
  status: 'passed' | 'failed' | 'pending' | 'skipped';
  surface: string;
}

export interface ConcernRollup {
  passed: number;
  failed: number;
  /**
   * NO VERDICT — the union of pending + undefined + skipped.
   *
   * Deliberately kept as the union rather than narrowed to "pending only":
   * two shipped gates read it as "this concern did not fully answer" —
   * `fulfill.rs:250` (`all_green = failed==0 && pending==0 && passed>0`,
   * which is what keeps a skipped chapter from discharging a commitment) and
   * `saga-status.py:288` (`pending>0 -> pending-env`). Narrowing it would
   * silently un-gate both: a concern with 3 passed + 1 skipped would read
   * all-green. The skipped SUBSET is named separately below.
   */
  pending: number;
  /** How many of `pending` ran and were skipped, as opposed to pending/undefined. */
  skipped: number;
  scenarios: ConcernScenario[];
}

/**
 * Law I: a concern was MEASURED only if the run produced a real verdict for it
 * — at least one pass or one fail. An all-zero rollup is an apparatus that ran
 * nothing, never a pass; that conflation is exactly what let edge #1376's
 * 76-of-80 skips read as green on three habits.
 *
 * This is the same predicate `.claude/scripts/habits-status.py::observe_concern`
 * applies to the same rollup. The register and the report must never answer
 * "was this measured?" differently about one run.
 */
export function producedVerdict(rollup: ConcernRollup): boolean {
  return rollup.passed + rollup.failed > 0;
}

/**
 * Law I (guidestar 2026-08-22 §3) — the denominator and what the run did with it.
 * `concerns` arrives from OUTSIDE the run (habits.yaml); only `exercised` and
 * `notMeasured` are derived here.
 */
export interface DeclaredRollup {
  source: string;
  /**
   * Present ONLY when the denominator could not be read. A zero denominator
   * must never render as clean: every count-based reader sees `concerns: 0`
   * and `notMeasured: 0`, which is indistinguishable from perfect coverage
   * unless the failure has a field of its own.
   */
  unreadable?: string;
  concerns: string[];
  exercised: string[];
  notMeasured: string[];
  /**
   * The subset of `notMeasured` whose scenarios RAN and were skipped (as
   * opposed to never appearing in this run at all). Both are NOT MEASURED;
   * they have different first moves.
   */
  notMeasuredRanAndSkipped: string[];
}

export interface SprintReport {
  generatedAt: string;
  runId: string;
  profile: string;
  /** Storage Track-2 backend measured by this run. */
  transport?: string;
  /** The commit this run measured; omitted when neither GIT_COMMIT nor git answered. */
  gitCommit?: string;
  doorway?: string;
  /** The environment component of the derivation key, including `sut`. */
  env?: RunEnv;
  /** The externally-anchored denominator, and the concerns this run did not measure. */
  declared?: DeclaredRollup;
  summary: {
    scenarios: { total: number; passed: number; failed: number; skipped: number; pending: number };
    findings: { total: number; bySource: Record<string, number>; byPillar: Record<string, number> };
    byConcern: Record<string, ConcernRollup>;
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
  transport?: string;
  doorway?: string;
  gitCommit?: string;
  env?: RunEnv;
  /**
   * The denominator, read from habits.yaml by the caller. Optional so an
   * in-process caller can aggregate without a repo; build-sprint-report.ts
   * always supplies it, because a report with no denominator cannot say what
   * it failed to measure.
   */
  declaredConcerns?: DeclaredConcernSet;
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
    byConcern: {} as Record<string, ConcernRollup>,
  };
}

/**
 * 'undefined' → pending; 'skipped' stays SKIPPED (guidestar §S3 supersedes the
 * old skipped→pending collapse by name).
 */
function concernStatusOf(status: ScenarioResult['status']): ConcernScenario['status'] {
  if (status === 'passed' || status === 'failed' || status === 'skipped') return status;
  return 'pending';
}

/**
 * `pending` stays the NO-VERDICT UNION (pending + undefined + skipped) while
 * `skipped` names its ran-and-skipped subset — so fulfill.rs's `pending == 0`
 * and saga-status.py's `pending > 0` keep their exact meaning.
 */
function tally(rollup: ConcernRollup, status: ConcernScenario['status']): void {
  if (status === 'passed') rollup.passed += 1;
  else if (status === 'failed') rollup.failed += 1;
  else {
    rollup.pending += 1;
    if (status === 'skipped') rollup.skipped += 1;
  }
}

function computeByConcern(scenarios: ScenarioResult[]): Record<string, ConcernRollup> {
  const result: Record<string, ConcernRollup> = {};
  for (const s of scenarios) {
    const concernTag = s.tags.find(t => t.startsWith('@concern:'));
    if (!concernTag) continue;
    const concern = concernTag.slice('@concern:'.length);
    result[concern] ??= { passed: 0, failed: 0, pending: 0, skipped: 0, scenarios: [] };
    const rollup = result[concern];
    const status = concernStatusOf(s.status);
    tally(rollup, status);
    rollup.scenarios.push({ name: s.name, status, surface: s.feature });
  }
  return result;
}

/**
 * Law I: `notMeasured` is DERIVED against the declared set, never reported by
 * the runner. A tag-scoped run exercises fewer concerns and therefore reports
 * MORE not-measured — a re-scope must not be able to make missing coverage
 * vanish.
 *
 * A concern counts as EXERCISED only when the run produced a real verdict for
 * it (`producedVerdict`: at least one pass or one fail). Mere presence in
 * byConcern is not enough: the guidestar's status table maps `Skipped` →
 * **NOT MEASURED** (§S3, restated in §6), and on the build this slice exists
 * for — edge #1376, 80 scenarios, 3 passed / 1 failed / 76 skipped — presence
 * would render `not measured 1` against the worked example's `declared 11 ·
 * exercised 0 · … not measured 11`. Skipping is how the coverage went missing,
 * not evidence that it did not.
 *
 * `notMeasuredRanAndSkipped` splits the absence into its two causes so a
 * reader can act: an apparatus that ran and skipped is a fixture/gate problem,
 * a concern absent from byConcern entirely is a scope/registry problem.
 */
function computeDeclared(
  declaredConcerns: DeclaredConcernSet,
  byConcern: Record<string, ConcernRollup>
): DeclaredRollup {
  const exercised: string[] = [];
  const notMeasured: string[] = [];
  const notMeasuredRanAndSkipped: string[] = [];
  for (const concern of declaredConcerns.concerns) {
    const rollup = byConcern[concern];
    if (rollup && producedVerdict(rollup)) {
      exercised.push(concern);
      continue;
    }
    notMeasured.push(concern);
    if (rollup) notMeasuredRanAndSkipped.push(concern);
  }
  return {
    source: declaredConcerns.source,
    unreadable: declaredConcerns.unreadable,
    concerns: [...declaredConcerns.concerns],
    exercised,
    notMeasured,
    notMeasuredRanAndSkipped,
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

  summary.byConcern = computeByConcern(input.scenarios);

  if (emitVisual) {
    summary.visualValidation = computeVisualValidation(input.scenarios);
  }

  return {
    generatedAt: new Date().toISOString(),
    runId: input.runId,
    profile: input.profile,
    transport: input.transport,
    gitCommit: input.gitCommit,
    doorway: input.doorway,
    env: input.env,
    declared: input.declaredConcerns
      ? computeDeclared(input.declaredConcerns, summary.byConcern)
      : undefined,
    summary,
    findings,
  };
}
