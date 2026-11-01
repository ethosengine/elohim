export type ScenarioStatus = 'passed' | 'failed' | 'skipped' | 'pending' | 'undefined';

export interface ScenarioResult {
  name: string;
  feature: string;
  status: ScenarioStatus;
  /** Sum of cucumber-json step durations. Cucumber serializes these in nanoseconds. */
  durationMs?: number;
  failureMessage?: string;
  tags: string[];
}

interface CucumberStep {
  name: string;
  hidden?: boolean;
  before?: CucumberStep[];
  after?: CucumberStep[];
  result?: { status: string; duration?: number; error_message?: string };
}

interface CucumberTag {
  name: string;
}

interface CucumberElement {
  name: string;
  type: string;
  steps?: CucumberStep[];
  before?: CucumberStep[];
  after?: CucumberStep[];
  tags?: CucumberTag[];
}

interface CucumberFeature {
  uri: string;
  name: string;
  elements?: CucumberElement[];
}

const STATUS_PRIORITY: ScenarioStatus[] = ['failed', 'undefined', 'pending', 'skipped', 'passed'];

function withHooks(steps: CucumberStep[]): CucumberStep[] {
  return steps.flatMap(step => [
    ...withHooks(step.before ?? []),
    step,
    ...withHooks(step.after ?? []),
  ]);
}

function aggregateStatus(steps: CucumberStep[]): ScenarioStatus {
  const seen = new Set(
    steps.map(s => {
      const status = s.result?.status;
      // Ambiguous bindings are a Cucumber execution failure, not a passing step.
      if (status === 'ambiguous') return 'failed';
      return STATUS_PRIORITY.includes(status as ScenarioStatus) ? status : 'undefined';
    })
  );
  for (const s of STATUS_PRIORITY) if (seen.has(s)) return s;
  return 'undefined';
}

export function loadCucumber(json: string): ScenarioResult[] {
  if (json.trim().length === 0) return [];
  const features = JSON.parse(json) as CucumberFeature[];
  const results: ScenarioResult[] = [];
  for (const feature of features) {
    for (const el of feature.elements ?? []) {
      if (el.type !== 'scenario') continue;
      const steps = el.steps ?? [];
      const outcomes = withHooks([...(el.before ?? []), ...steps, ...(el.after ?? [])]);
      // Hooks can fail even when every Gherkin step passes. Conversely, passing
      // hooks cannot establish that an empty scenario exercised anything.
      const status = aggregateStatus(
        steps.some(step => !step.hidden) ? outcomes : [...outcomes, { name: 'missing steps' }]
      );
      const failed = outcomes.find(s => ['failed', 'ambiguous'].includes(s.result?.status ?? ''));
      const tags = (el.tags ?? []).map(t => t.name);
      results.push({
        name: el.name,
        feature: feature.uri,
        status,
        durationMs:
          steps.reduce((total, step) => total + (step.result?.duration ?? 0), 0) / 1_000_000,
        failureMessage: failed?.result?.error_message,
        tags,
      });
    }
  }
  return results;
}
