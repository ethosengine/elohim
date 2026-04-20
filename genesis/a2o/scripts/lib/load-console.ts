import { readdirSync, readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

export interface ConsoleLogEntry {
  level: string;
  text: string;
  url: string;
}
export interface PageErrorEntry {
  message: string;
  url: string;
}

export interface ConsoleArtifact {
  scenario: string;
  human: string;
  consoleErrors: ConsoleLogEntry[];
  pageErrors: PageErrorEntry[];
}

const FILENAME_RE = /^(.+)-([^-]+)-errors\.json$/;

export function parseScenarioHumanFromFilename(
  filename: string
): { scenario: string; human: string } | null {
  const m = filename.match(FILENAME_RE);
  if (!m) return null;
  return { scenario: m[1], human: m[2] };
}

export function loadConsoleArtifacts(dir: string): ConsoleArtifact[] {
  if (!existsSync(dir)) return [];
  const entries = readdirSync(dir);
  const artifacts: ConsoleArtifact[] = [];
  for (const name of entries) {
    const parts = parseScenarioHumanFromFilename(name);
    if (!parts) continue;
    const body = JSON.parse(readFileSync(join(dir, name), 'utf8')) as {
      consoleErrors?: ConsoleLogEntry[];
      pageErrors?: PageErrorEntry[];
    };
    artifacts.push({
      scenario: parts.scenario,
      human: parts.human,
      consoleErrors: body.consoleErrors ?? [],
      pageErrors: body.pageErrors ?? [],
    });
  }
  return artifacts;
}
