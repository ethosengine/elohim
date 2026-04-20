export type ScenarioStatus = 'passed' | 'failed' | 'skipped' | 'pending' | 'undefined';

export interface ScenarioResult {
  name: string;
  feature: string;
  status: ScenarioStatus;
  failureMessage?: string;
}

interface CucumberStep {
  name: string;
  result?: { status: string; duration?: number; error_message?: string };
}

interface CucumberElement {
  name: string;
  type: string;
  steps?: CucumberStep[];
}

interface CucumberFeature {
  uri: string;
  name: string;
  elements?: CucumberElement[];
}

const STATUS_PRIORITY: ScenarioStatus[] = ['failed', 'undefined', 'pending', 'skipped', 'passed'];

function aggregateStatus(steps: CucumberStep[]): ScenarioStatus {
  const seen = new Set(steps.map(s => (s.result?.status ?? 'undefined') as ScenarioStatus));
  for (const s of STATUS_PRIORITY) if (seen.has(s)) return s;
  return 'passed';
}

export function loadCucumber(json: string): ScenarioResult[] {
  const features = JSON.parse(json) as CucumberFeature[];
  const results: ScenarioResult[] = [];
  for (const feature of features) {
    for (const el of feature.elements ?? []) {
      if (el.type !== 'scenario') continue;
      const steps = el.steps ?? [];
      const status = aggregateStatus(steps);
      const failed = steps.find(s => s.result?.status === 'failed');
      results.push({
        name: el.name,
        feature: feature.uri,
        status,
        failureMessage: failed?.result?.error_message,
      });
    }
  }
  return results;
}
