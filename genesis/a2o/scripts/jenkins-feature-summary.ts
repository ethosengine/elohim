import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { loadCucumber } from './lib/load-cucumber.js';

const PEER_STAGE = 'peer-stage';
const LANES = ['api', 'browser', PEER_STAGE] as const;

interface PeerStageVerdict {
  verdict: 'pass' | 'fail' | 'skip';
  scenarios?: unknown[];
  held?: number;
  summary?: string;
}

/** A presentation of this build's foreign Cucumber results, never a new verdict. */
export function featureSummary(
  read: (lane: string) => string | undefined,
  readPeerVerdict: () => string | undefined = () => undefined
): string {
  return LANES.map(lane => {
    try {
      if (lane === PEER_STAGE) {
        const verdictBytes = readPeerVerdict();
        if (verdictBytes !== undefined) {
          const verdict = JSON.parse(verdictBytes) as PeerStageVerdict;
          if (verdict.summary) return `${lane}: ${verdict.summary}`;
          const measured = verdict.scenarios?.length ?? 0;
          const held = verdict.held ?? 0;
          return (
            `${lane}: ${verdict.verdict} (${measured} measured, ${held} held)` +
            (verdict.verdict === 'skip' ? ' — held, not measured' : '')
          );
        }
      }
      const bytes = read(lane);
      if (bytes === undefined) return `${lane}: not measured (no report)`;
      const scenarios = loadCucumber(bytes);
      if (!scenarios.length) return `${lane}: not measured (empty report)`;
      const count = (status: string) =>
        scenarios.filter(scenario => scenario.status === status).length;
      if (lane === PEER_STAGE) {
        const held = count('skipped');
        const measured = scenarios.length - held;
        if (measured === 0) return `${lane}: held, not measured (${held} held)`;
        return (
          `${lane}: ${count('passed')}/${measured} measured scenarios passed, ` +
          `${count('failed')} failed, ${held} held, ` +
          `${count('pending') + count('undefined')} pending/undefined`
        );
      }
      return `${lane}: ${count('passed')}/${scenarios.length} scenarios passed, ${count('failed')} failed, ${count('skipped')} skipped, ${count('pending') + count('undefined')} pending/undefined`;
    } catch {
      return `${lane}: not measured (unreadable report)`;
    }
  }).join(' | ');
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const reports = resolve(process.argv[2] ?? 'genesis/a2o/reports');
  console.log(
    featureSummary(
      lane => {
        try {
          return readFileSync(resolve(reports, `cucumber-report-${lane}.json`), 'utf8');
        } catch (error) {
          if ((error as NodeJS.ErrnoException).code === 'ENOENT') return undefined;
          throw error;
        }
      },
      () => {
        try {
          return readFileSync(resolve(reports, 'peer-stage-ci/verdict.json'), 'utf8');
        } catch (error) {
          if ((error as NodeJS.ErrnoException).code === 'ENOENT') return undefined;
          throw error;
        }
      }
    )
  );
}
