/** Feed validated evidence to EPR before rendering; never fulfil or publish here. */
import { spawnSync } from 'node:child_process';

import { resolveBinary } from './household-attestation.js';

import type { SprintReport } from './aggregate.js';

export type FeedbackRecorder = (report: SprintReport, root: string) => boolean;

const record: FeedbackRecorder = (report, root) => {
  const binary = resolveBinary('epr', process.env.EPR_BIN, process.env.PATH);
  if (!binary) return false;
  const result = spawnSync(binary, ['feedback', 'ingest-genesis', '--root', root, '--json'], {
    input: JSON.stringify(report),
    encoding: 'utf8',
    timeout: 2000,
    maxBuffer: 1024 * 1024,
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  return !result.error && result.status === 0;
};

export function captureFeedback(
  report: SprintReport,
  root: string,
  recorder: FeedbackRecorder = record,
  warn: (message: string) => void = console.warn
): boolean {
  try {
    if (recorder(report, root)) return true;
  } catch {
    // Recording is subordinate to the operation: preserve its result on every capture failure.
  }
  warn('Feedback capture gap: evidence was not fully retained; report verdict unchanged.');
  return false;
}
