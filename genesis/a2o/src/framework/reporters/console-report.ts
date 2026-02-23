/**
 * Browser artifact report — structured JSON output of console errors,
 * page errors, and failed network requests captured during Playwright scenarios.
 *
 * Written after each scenario (on failure) and as a summary at the end of a run.
 */

import { writeFileSync } from 'node:fs';

/** A single scenario's browser artifacts from one human's device. */
export interface BrowserArtifact {
  scenario: string;
  human: string;
  consoleErrors: { level: string; text: string; url: string }[];
  pageErrors: { message: string; url: string }[];
  failedRequests: { url: string; method: string; failure?: string }[];
  screenshot?: string;
  trace?: string;
}

/** Write a JSON report of browser artifacts. */
export function writeBrowserReport(artifacts: BrowserArtifact[], outputPath: string): void {
  const summary = {
    totalScenarios: artifacts.length,
    totalConsoleErrors: artifacts.reduce((n, a) => n + a.consoleErrors.length, 0),
    totalPageErrors: artifacts.reduce((n, a) => n + a.pageErrors.length, 0),
    totalFailedRequests: artifacts.reduce((n, a) => n + a.failedRequests.length, 0),
    artifacts,
  };

  writeFileSync(outputPath, JSON.stringify(summary, null, 2));
}
