import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { loadCucumber } from './lib/load-cucumber.js';

const LANES = ['api', 'browser', 'peer-stage'] as const;

/** A presentation of this build's foreign Cucumber results, never a new verdict. */
export function featureSummary(read: (lane: string) => string | undefined): string {
  return LANES.map(lane => {
    try {
      const bytes = read(lane);
      if (bytes === undefined) return `${lane}: not measured (no report)`;
      const scenarios = loadCucumber(bytes);
      if (!scenarios.length) return `${lane}: not measured (empty report)`;
      const count = (status: string) =>
        scenarios.filter(scenario => scenario.status === status).length;
      return `${lane}: ${count('passed')}/${scenarios.length} scenarios passed, ${count('failed')} failed, ${count('skipped')} skipped, ${count('pending') + count('undefined')} pending/undefined`;
    } catch {
      return `${lane}: not measured (unreadable report)`;
    }
  }).join(' | ');
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const reports = resolve(process.argv[2] ?? 'genesis/a2o/reports');
  console.log(
    featureSummary(lane => {
      try {
        return readFileSync(resolve(reports, `cucumber-report-${lane}.json`), 'utf8');
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code === 'ENOENT') return undefined;
        throw error;
      }
    })
  );
}
